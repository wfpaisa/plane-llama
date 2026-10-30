// Formatos es-CO, nombres de modelos y etiquetas para las vistas.
use crate::*;
use std::sync::LazyLock;
pub fn fmt(n: Option<f64>, decimals: usize) -> String {
    let Some(n) = n else {
        return "—".into();
    };
    let text = format!("{n:.decimals$}");
    let (integer, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let sign = if integer.starts_with('-') { "-" } else { "" };
    let digits = integer.trim_start_matches('-');
    let mut result = sign.to_string();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push('.');
        }
        result.push(c);
    }
    if decimals > 0 {
        result.push(',');
        result.push_str(fraction);
    }
    result
}
pub fn fmt_gb(n: Option<f64>, d: usize) -> String {
    fmt(n.map(|n| n / 1000.), d)
}
pub fn fmt_sec(n: Option<f64>) -> String {
    fmt(n.map(|n| n / 1000.), 2)
}
pub fn fmt_ms(n: Option<f64>) -> String {
    n.map(|n| {
        let s = (n / 1000.).round() as i64;
        format!("{:02}:{:02}", s / 60, s % 60)
    })
    .unwrap_or("—".into())
}
pub fn effective_batch(batch: Option<f64>, ubatch: Option<f64>) -> (f64, f64) {
    (batch.unwrap_or(2048.), ubatch.unwrap_or(512.))
}
pub fn effective_cache<'a>(k: Option<&'a str>, v: Option<&'a str>) -> (&'a str, &'a str) {
    (k.unwrap_or("f16"), v.unwrap_or("f16"))
}
pub fn model_base(m: Option<&str>) -> Option<String> {
    m.filter(|s| !s.is_empty()).map(|s| {
        s.split(':')
            .next()
            .unwrap_or(s)
            .rsplit('/')
            .next()
            .unwrap_or(s)
            .to_string()
    })
}
pub fn short_model(m: Option<&str>) -> String {
    model_base(m)
        .map(|s| s.chars().take(22).collect())
        .unwrap_or("—".into())
}
pub fn alert_cls(p: f64) -> &'static str {
    if p > 90. {
        "red"
    } else if p > 70. {
        "yellow"
    } else {
        "green"
    }
}
pub fn backend_label(b: Option<&str>) -> String {
    b.filter(|b| *b != "unknown")
        .map(|b| {
            let mut c = b.chars();
            c.next()
                .map(|v| v.to_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}
pub fn compute_backend_label(b: Option<&str>, n_cpu_moe: Option<f64>) -> String {
    let main = backend_label(b);
    if n_cpu_moe.unwrap_or(0.) > 0. {
        if main.is_empty() {
            "CPU".into()
        } else {
            format!("{main} + CPU")
        }
    } else {
        main
    }
}
pub fn backend_severity(b: Option<&str>) -> &'static str {
    match b {
        Some("cuda") => "success",
        Some("vulkan") => "danger",
        Some("sycl") => "info",
        Some("metal") => "warn",
        _ => "secondary",
    }
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedModel {
    pub base: String,
    pub size: Option<String>,
    pub quant: Option<String>,
    pub mtp: bool,
    pub tags: Vec<String>,
    pub org: Option<String>,
}
const TAGS: &[&str] = &[
    "it", "sft", "dpo", "orpo", "kto", "qat", "awq", "gguf", "lora", "rlhf", "rm", "chat",
    "instruct", "pretrain", "finetune", "ft", "flash",
];
static SIZE: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^(\d+B(?:-A\d+B)?|MoE|A\d+B)$").ok());
static QUANT: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^(UD-)?(I?Q\d[_A-Z0-9]*|F16|F32|BF16|FP16|FP8|TQ\d[_A-Z0-9]*)$").ok()
});
static MOE: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)A\d+B|MoE|Mixtral|\d+x\d+B|GR\d+|OLMoE|SmolLM2?-MoE").ok()
});
pub fn is_model_moe(base: Option<&str>) -> bool {
    base.is_some_and(|s| MOE.as_ref().is_some_and(|r| r.is_match(s)))
}
pub fn parse_model(m: Option<&str>) -> Option<ParsedModel> {
    let m = m.filter(|s| !s.is_empty())?;
    let body = model_base(Some(m))?;
    let parts: Vec<_> = body.split('-').filter(|s| !s.is_empty()).collect();
    let is_size = |s: &str| SIZE.as_ref().is_some_and(|r| r.is_match(s));
    let is_quant = |s: &str| QUANT.as_ref().is_some_and(|r| r.is_match(s));
    let start = parts.iter().position(|s| is_size(s));
    let end = start.map(|i| {
        if parts
            .get(i + 1)
            .is_some_and(|s| s.to_uppercase().starts_with('A') && is_size(s))
        {
            i + 1
        } else {
            i
        }
    });
    let base_end = start.unwrap_or_else(|| {
        parts
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, s)| TAGS.contains(&s.to_lowercase().as_str()) || is_quant(s))
            .map(|(i, _)| i)
            .unwrap_or(parts.len())
    });
    let base = parts[..base_end].join("-");
    let quant = m
        .split_once(':')
        .map(|(_, q)| q.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            parts
                .iter()
                .enumerate()
                .skip(end.map(|e| e + 1).unwrap_or(0))
                .find(|(_, s)| is_quant(s))
                .map(|(i, _)| parts[i..].join("-"))
        });
    Some(ParsedModel {
        base: if base.is_empty() { body.clone() } else { base },
        size: start.zip(end).map(|(s, e)| parts[s..=e].join("-")),
        quant,
        mtp: body.to_uppercase().contains("MTP"),
        tags: parts
            .iter()
            .skip(end.map(|e| e + 1).unwrap_or(base_end))
            .filter(|s| TAGS.contains(&s.to_lowercase().as_str()))
            .map(|s| s.to_uppercase())
            .collect(),
        org: m.split_once('/').map(|(org, _)| org.to_string()),
    })
}
pub fn model_display_name(m: Option<&str>) -> Option<String> {
    parse_model(m).map(|p| p.base).or_else(|| model_base(m))
}
pub fn format_script(s: &str) -> String {
    let clean = s.replace("\\\n", "").replace('\\', "");
    let tokens: Vec<_> = clean.split_whitespace().collect();
    let mut parts = vec![tokens.first().copied().unwrap_or("").to_string()];
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i].starts_with('-') {
            let mut flag = tokens[i].to_string();
            if tokens.get(i + 1).is_some_and(|s| !s.starts_with('-')) {
                i += 1;
                flag.push(' ');
                flag.push_str(tokens[i]);
            }
            parts.push(flag);
        }
        i += 1;
    }
    parts.join(" \\\n")
}
/// Nombre corto de una tarjeta: «NVIDIA GeForce RTX 5070 Ti» → «RTX 5070 Ti».
pub fn gpu_short_name(name: &str) -> String {
    let mut n = name.trim();
    for prefix in ["NVIDIA ", "GeForce ", "AMD ", "Intel(R) ", "Intel "] {
        n = n.strip_prefix(prefix).unwrap_or(n);
    }
    n.trim().to_owned()
}
/// Nombre de la tarjeta si se conoce; si no, su identificador.
pub fn gpu_title(g: &GpuInfo) -> String {
    g.name
        .as_deref()
        .map(gpu_short_name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| gpu_label(g))
}
pub fn gpu_label(g: &GpuInfo) -> String {
    if !g.index.is_empty() {
        g.index.trim().into()
    } else if !g.vendor.is_empty() {
        g.vendor.trim().into()
    } else {
        "gpu".into()
    }
}
pub fn device_vram_line(r: &BenchmarkResult, compact: bool) -> String {
    let values: Vec<_> = if r.device_vram.is_empty() {
        r.gpus
            .iter()
            .map(|g| (gpu_label(g), g.mem_used_mi_b))
            .collect()
    } else {
        r.device_vram
            .iter()
            .map(|d| (d.device.id.clone(), d.used_mi_b))
            .collect()
    };
    if values.is_empty() {
        return "—".into();
    }
    values
        .iter()
        .map(|(id, n)| {
            let gb = n.map(|n| fmt_gb(Some(n), 1)).unwrap_or("?".into());
            if compact {
                format!("{id}:{gb}")
            } else {
                format!("{id}: {gb} GB")
            }
        })
        .collect::<Vec<_>>()
        .join(if compact { ", " } else { " · " })
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVramRow {
    pub vendor: String,
    pub index: String,
    pub gb: String,
    pub detail: String,
}
pub fn device_vram_rows(r: &BenchmarkResult) -> Vec<DeviceVramRow> {
    r.device_vram
        .iter()
        .map(|d| {
            let dev = &d.device;
            let index = dev
                .id
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            DeviceVramRow {
                vendor: if dev.vendor.is_empty() || dev.vendor == "unknown" {
                    "—".into()
                } else {
                    dev.vendor.to_uppercase()
                },
                index: if index.is_empty() {
                    dev.id.clone()
                } else {
                    index
                },
                gb: d
                    .used_mi_b
                    .map(|n| fmt_gb(Some(n), 1))
                    .unwrap_or("?".into()),
                detail: format!(
                    "{}: {} ({} GB, {} GB libres)",
                    dev.id,
                    dev.name,
                    fmt_gb(Some(dev.total_mi_b), 1),
                    fmt_gb(Some(dev.free_mi_b), 1)
                ),
            }
        })
        .collect()
}
pub fn total_device_vram_txt(r: &BenchmarkResult) -> String {
    let total: f64 = if r.device_vram.is_empty() {
        r.gpus.iter().filter_map(|g| g.mem_used_mi_b).sum()
    } else {
        r.device_vram.iter().filter_map(|d| d.used_mi_b).sum()
    };
    if total > 0. {
        fmt_gb(Some(total), 1)
    } else {
        "—".into()
    }
}
pub use super::store_logic::compute_bests;
