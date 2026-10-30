// Metadatos GGUF, resolución local/HF y estimación del backend TS.
use crate::*;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};
const MIB: f64 = 1048576.0;
static SIZE: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*B\b").ok());
pub fn parse_model_meta(raw: Option<&str>) -> ModelMeta {
    let Some(raw) = raw.filter(|s| !s.is_empty()) else {
        return ModelMeta::default();
    };
    let mut split = raw.split(':');
    let base = split
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    let quant = split.next().map(str::to_string);
    let params_b = SIZE
        .as_ref()
        .and_then(|re| re.captures(&base))
        .and_then(|c| c[1].parse::<f64>().ok());
    let arch = params_b.map(|b| {
        [
            (1., 16., 4.),
            (4., 32., 4.),
            (8., 32., 8.),
            (14., 40., 8.),
            (32., 64., 8.),
            (70., 80., 8.),
            (110., 90., 8.),
            (f64::INFINITY, 94., 4.),
        ]
        .into_iter()
        .find(|r| b <= r.0)
        .unwrap_or((400., 94., 4.))
    });
    ModelMeta {
        raw: raw.into(),
        base,
        bytes_per_param: quant.as_deref().and_then(bytes_per_param_for),
        quant,
        params_b,
        layers: arch.map(|r| r.1),
        kv_heads: arch.map(|r| r.2),
        head_dim: Some(128.),
        ..Default::default()
    }
}
pub fn bytes_per_param_for(quant: &str) -> Option<f64> {
    let q: String = quant
        .to_uppercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let base = ["UD_", "DYN_", "DYNAMIC_"]
        .iter()
        .find_map(|p| q.strip_prefix(p))
        .unwrap_or(&q);
    let exact = |q: &str| match q {
        "F32" => Some(4.),
        "F16" | "BF16" => Some(2.),
        "Q8_0" => Some(0.85),
        "Q6_K" => Some(0.69),
        "Q5_K_M" | "Q5_1" => Some(0.59),
        "Q5_K_S" | "Q5_0" | "Q4_K_S" | "Q4_1" | "Q4_0" => Some(0.56),
        "Q4_K_M" => Some(0.69),
        "IQ4_NL" => Some(0.51),
        "IQ4_XS" | "Q3_K_M" => Some(0.45),
        "Q3_K_S" => Some(0.41),
        "IQ3_M" => Some(0.38),
        "IQ3_S" => Some(0.36),
        "IQ3_XXS" => Some(0.33),
        "Q2_K" => Some(0.35),
        "IQ2_M" => Some(0.3),
        "UD_Q6_K_XL" => Some(0.7),
        "UD_Q4_K_XL" => Some(0.57),
        _ => None,
    };
    exact(&q).or_else(|| exact(base)).or_else(|| {
        q.as_bytes().windows(2).find_map(|s| {
            if s[0] == b'Q' {
                match s[1] {
                    b'2' => Some(0.35),
                    b'3' => Some(0.45),
                    b'4' => Some(0.56),
                    b'5' => Some(0.57),
                    b'6' => Some(0.69),
                    b'8' => Some(0.85),
                    _ => None,
                }
            } else {
                None
            }
        })
    })
}
pub fn bytes_per_kv_element(t: &str) -> f64 {
    match t.to_lowercase().as_str() {
        "f32" => 4.,
        "q8_0" => 1.,
        "q4_0" | "q4_1" | "iq4_nl" | "iq4_xs" => 0.5,
        "q5_0" | "q5_1" => 0.625,
        _ => 2.,
    }
}
pub(crate) fn kv_mib(meta: &ModelMeta, p: &TunedParams, ctx: f64) -> f64 {
    2. * meta.attention_layers.or(meta.layers).unwrap_or(32.)
        * meta.kv_heads.unwrap_or(8.)
        * meta.head_dim.unwrap_or(128.)
        * ctx
        * (bytes_per_kv_element(&p.cache_type_k) + bytes_per_kv_element(&p.cache_type_v))
        / 2.
        / MIB
}
pub(crate) fn backend_estimate(
    meta: &ModelMeta,
    p: &TunedParams,
    ctx: f64,
) -> Option<(f64, f64, f64)> {
    Some((
        meta.params_b? * 1e9 * meta.bytes_per_param? / MIB,
        kv_mib(meta, p, ctx),
        256. + p.ubatch_size * 0.5,
    ))
}
pub fn recommend_params(
    meta: &ModelMeta,
    free: f64,
    priority: &str,
    current: &TunedParams,
) -> TunedParams {
    let mut p = current.clone();
    p.ngl = 999.;
    p.cache_type_v = if priority == "quality" {
        "q8_0".into()
    } else {
        p.cache_type_k.clone()
    };
    if meta.params_b.is_none() || meta.bytes_per_param.is_none() {
        return p;
    }
    let (mut lo, mut hi, mut best) = (512_i64, 1_000_000_i64, 8192_i64);
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if backend_estimate(meta, &p, mid as f64).is_some_and(|(w, k, o)| w + k + o <= free * 0.92)
        {
            best = mid;
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    p.ctx_size = ((best / 256) * 256).max(512) as f64;
    p
}
pub(crate) fn selected_devices<'a>(
    devices: &'a [LlamaDevice],
    ids: &[String],
) -> Vec<&'a LlamaDevice> {
    devices
        .iter()
        .filter(|d| ids.is_empty() || ids.contains(&d.id))
        .collect()
}
pub(crate) fn split_per_device(
    total: f64,
    devices: &[&LlamaDevice],
    split: Option<&[f64]>,
) -> Vec<f64> {
    if devices.len() == 1 {
        return vec![total];
    }
    let weights: Vec<f64> = split
        .filter(|s| s.len() == devices.len())
        .map(|s| s.to_vec())
        .unwrap_or_else(|| {
            devices
                .iter()
                .map(|d| if d.free_mi_b == 0. { 1. } else { d.free_mi_b })
                .collect()
        });
    let sum: f64 = weights.iter().sum();
    let sum = if sum == 0. { 1. } else { sum };
    weights.iter().map(|w| total * w / sum).collect()
}
pub fn build_estimate_response(
    meta: ModelMeta,
    devices: Vec<LlamaDevice>,
    params: TunedParams,
    priority: &str,
) -> EstimateResponse {
    let selected = selected_devices(&devices, &params.device);
    let free = selected.iter().map(|d| d.free_mi_b).sum();
    let (mut weights, kv, mut overhead) = backend_estimate(
        &meta,
        &params,
        (params.ctx_size - params.cache_reuse).max(0.),
    )
    .unwrap_or((0., 0., 0.));
    if let Some(size) = meta.weights_file_mi_b {
        let layers = meta.layers.unwrap_or(32.);
        weights = size
            * if params.ngl >= layers {
                1.
            } else {
                (params.ngl / layers).max(0.)
            };
    }
    if !params.no_mmproj {
        overhead += meta.mmproj_size_mi_b.unwrap_or(0.);
    }
    if params.spec_draft_max > 0. {
        overhead += weights * (0.08 + 0.035 * (params.spec_draft_max - 1.));
    }
    let total = weights + kv + overhead;
    let heuristic = VramBreakdown {
        per_device_mi_b: split_per_device(total, &selected, params.tensor_split.as_deref()),
        total_mi_b: total,
        weights_mi_b: weights,
        kv_mi_b: kv,
        overhead_mi_b: overhead,
        fits: total <= free,
        ..Default::default()
    };
    let recommendation = recommend_params(&meta, free, priority, &params);
    EstimateResponse {
        backend: devices::detect_backend(&devices),
        devices,
        total_free_mi_b: free,
        model_meta: meta,
        heuristic,
        recommendation,
        ..Default::default()
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GgufArch {
    pub layers: Option<f64>,
    pub kv_heads: Option<f64>,
    pub key_length: Option<f64>,
    pub value_length: Option<f64>,
    pub context_length: Option<f64>,
    pub attention_layers: Option<f64>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let n = usize::try_from(self.u64()?).ok()?;
        Some(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
    fn value(&mut self, t: u32, depth: usize) -> Option<Vec<f64>> {
        if depth > 16 {
            return None;
        }
        let number = match t {
            0 | 7 => self.take(1)?[0] as f64,
            1 => self.take(1)?[0] as i8 as f64,
            2 => u16::from_le_bytes(self.take(2)?.try_into().ok()?) as f64,
            3 => i16::from_le_bytes(self.take(2)?.try_into().ok()?) as f64,
            4 => self.u32()? as f64,
            5 => self.u32()? as i32 as f64,
            6 => f32::from_le_bytes(self.take(4)?.try_into().ok()?) as f64,
            10 => self.u64()? as f64,
            11 => self.u64()? as i64 as f64,
            12 => f64::from_le_bytes(self.take(8)?.try_into().ok()?),
            8 => {
                self.string()?;
                return Some(vec![]);
            }
            9 => {
                let elem = self.u32()?;
                let n = self.u64()?;
                if n > self.bytes.len() as u64 {
                    return None;
                }
                let mut values = vec![];
                for _ in 0..n {
                    values.extend(self.value(elem, depth + 1)?);
                }
                return Some(values);
            }
            _ => return None,
        };
        Some(vec![number])
    }
}
pub fn parse_gguf_header(bytes: &[u8]) -> GgufArch {
    let mut arch = GgufArch::default();
    let mut r = Reader { bytes, pos: 16 };
    if bytes.get(..4) != Some(b"GGUF") {
        return arch;
    }
    let Some(count) = r.u64() else {
        return arch;
    };
    let mut interval = None;
    for _ in 0..count {
        let Some(key) = r.string() else {
            break;
        };
        let Some(t) = r.u32() else {
            break;
        };
        let Some(value) = r.value(t, 0) else {
            break;
        };
        if t == 9 && key.ends_with(".attention.recurrent_layers") {
            arch.attention_layers = Some(value.iter().filter(|v| **v == 0.).count() as f64);
        } else if let Some(v) = value.first().copied() {
            if key.ends_with(".block_count") {
                arch.layers = Some(v);
            } else if key.ends_with(".attention.head_count_kv") {
                arch.kv_heads = Some(v);
            } else if key.ends_with(".attention.key_length") {
                arch.key_length = Some(v);
            } else if key.ends_with(".attention.value_length") {
                arch.value_length = Some(v);
            } else if key.ends_with(".context_length") {
                arch.context_length = Some(v);
            } else if key.ends_with(".full_attention_interval") && v > 0. {
                interval = Some(v);
            }
        }
    }
    if arch.attention_layers.is_none() {
        arch.attention_layers = arch.layers.zip(interval).map(|(l, i)| (l / i).floor());
    }
    arch
}
pub fn read_gguf_arch(file: impl AsRef<Path>) -> GgufArch {
    let mut bytes = vec![];
    if let Ok(f) = std::fs::File::open(file) {
        let _ = f.take(2 * 1024 * 1024).read_to_end(&mut bytes);
    }
    parse_gguf_header(&bytes)
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedModel {
    pub file: Option<String>,
    pub size_mi_b: Option<f64>,
    pub mmproj_file: Option<String>,
    pub mmproj_size_mi_b: Option<f64>,
    pub arch: Option<GgufArch>,
}
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    v.sort();
    v
}
fn gguf(p: &Path) -> bool {
    p.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("gguf"))
}
fn mmproj(p: &Path) -> bool {
    p.file_name()
        .is_some_and(|s| s.to_string_lossy().to_lowercase().contains("mmproj"))
}
pub fn resolve_model_file(hf: Option<&str>, model_file: Option<&str>) -> ResolvedModel {
    let size = |p: &Path| std::fs::metadata(p).ok().map(|m| m.len() as f64 / MIB);
    let explicit = model_file.and_then(|s| {
        let p = PathBuf::from(s);
        if p.is_file() && gguf(&p) {
            Some(p)
        } else if p.is_dir() {
            entries(&p).into_iter().find(|p| gguf(p) && !mmproj(p))
        } else {
            None
        }
    });
    if let Some(file) = explicit {
        let mmproj = file
            .parent()
            .and_then(|dir| entries(dir).into_iter().find(|p| gguf(p) && mmproj(p)));
        return ResolvedModel {
            size_mi_b: size(&file),
            arch: Some(read_gguf_arch(&file)),
            file: Some(file.to_string_lossy().into_owned()),
            mmproj_size_mi_b: mmproj.as_deref().and_then(size),
            mmproj_file: mmproj.map(|p| p.to_string_lossy().into_owned()),
        };
    }
    // `-hf org/modelo:QUANT` en la caché; los modelos divididos suman todas sus partes.
    let Some(model) = hf
        .filter(|h| h.split(':').next().is_some_and(|r| r.contains('/')))
        .and_then(|h| crate::models::find_local(&crate::models::hub_dir(), h))
    else {
        return ResolvedModel::default();
    };
    let Some(first) = model.files.first().map(PathBuf::from) else {
        return ResolvedModel::default();
    };
    let mmproj = model.mmproj.first().map(PathBuf::from);
    ResolvedModel {
        size_mi_b: Some(model.size_bytes as f64 / MIB),
        arch: Some(read_gguf_arch(&first)),
        file: Some(first.to_string_lossy().into_owned()),
        mmproj_size_mi_b: mmproj.as_deref().and_then(size),
        mmproj_file: mmproj.map(|p| p.to_string_lossy().into_owned()),
    }
}
pub async fn estimate(req: EstimateRequest) -> Result<EstimateResponse> {
    let parsed = script_parser::parse_script(&req.script)?;
    server_manager::assert_binary_exists(&parsed.binary)?;
    let devices = devices::list_devices(&parsed.binary).await;
    let mut meta = parse_model_meta(parsed.model.as_deref());
    let resolved = resolve_model_file(parsed.model.as_deref(), parsed.model_file.as_deref());
    meta.weights_file = resolved.file;
    meta.weights_file_mi_b = resolved.size_mi_b;
    meta.mmproj_size_mi_b = resolved.mmproj_size_mi_b;
    if let Some(a) = resolved.arch {
        meta.layers = a.layers.or(meta.layers);
        meta.kv_heads = a.kv_heads.or(meta.kv_heads);
        meta.head_dim = a.key_length.or(meta.head_dim);
        meta.attention_layers = a.attention_layers;
    }
    let mut params = client::flag_writer::params_from_parsed(&parsed);
    params.cache_type_k = parsed.cache_type_k.unwrap_or("f16".into());
    params.cache_type_v = parsed.cache_type_v.unwrap_or("f16".into());
    if let Some(overrides) = req.params {
        let mut value = serde_json::to_value(&params)?;
        if let Some(map) = overrides.as_object() {
            for (k, v) in map {
                if !v.is_null() {
                    value[k] = v.clone();
                }
            }
        }
        params = serde_json::from_value(value)?;
    }
    Ok(build_estimate_response(
        meta,
        devices,
        params,
        req.priority.as_deref().unwrap_or("ctx"),
    ))
}
