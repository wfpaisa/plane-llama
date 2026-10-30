// Contratos JSON compartidos con llama-bench.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedScript {
    pub script: String,
    pub binary: String,
    pub argv: Vec<String>,
    pub model: Option<String>,
    pub host: String,
    pub port: f64,
    pub ctx_size: Option<f64>,
    pub batch_size: Option<f64>,
    pub ubatch_size: Option<f64>,
    pub cache_type_k: Option<String>,
    pub cache_type_v: Option<String>,
    pub device: Option<String>,
    pub tensor_split: Option<String>,
    pub temp: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<f64>,
    pub ngl: Option<f64>,
    pub flash_attn: bool,
    pub threads: Option<f64>,
    pub min_p: Option<f64>,
    pub repeat_penalty: Option<f64>,
    pub model_file: Option<String>,
    pub n_cpu_moe: f64,
    pub cache_reuse: f64,
    pub no_mmproj: bool,
    pub spec_draft_n_max: Option<f64>,
    pub cache_ram: Option<f64>,
    pub extra: BTreeMap<String, serde_json::Value>,
    pub missing_fields: std::collections::BTreeSet<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    #[serde(rename = "status")]
    pub status: ServerStatus,
    #[serde(rename = "pid")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub pid: Option<f64>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "url")]
    pub url: Option<String>,
    #[serde(rename = "error")]
    pub error: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    #[serde(rename = "seq")]
    #[serde(serialize_with = "serialize_number")]
    pub seq: f64,
    #[serde(rename = "t")]
    #[serde(serialize_with = "serialize_number")]
    pub t: f64,
    #[serde(rename = "stream")]
    pub stream: String,
    #[serde(rename = "msg")]
    pub msg: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsResponse {
    #[serde(rename = "entries")]
    pub entries: Vec<LogEntry>,
    #[serde(rename = "cursor")]
    #[serde(serialize_with = "serialize_number")]
    pub cursor: f64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    #[serde(rename = "index")]
    pub index: String,
    #[serde(rename = "vendor")]
    pub vendor: String,
    #[serde(rename = "memUsedMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mem_used_mi_b: Option<f64>,
    #[serde(rename = "memTotalMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mem_total_mi_b: Option<f64>,
    #[serde(rename = "gpuUtilPct")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub gpu_util_pct: Option<f64>,
    /// Nombre comercial (p. ej. «NVIDIA GeForce RTX 5070 Ti») para distinguir tarjetas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaDevice {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "vendor")]
    pub vendor: String,
    #[serde(rename = "totalMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub total_mi_b: f64,
    #[serde(rename = "freeMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub free_mi_b: f64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVram {
    #[serde(rename = "device")]
    pub device: LlamaDevice,
    #[serde(rename = "usedMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub used_mi_b: Option<f64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RamInfo {
    #[serde(rename = "memTotalMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mem_total_mi_b: Option<f64>,
    #[serde(rename = "memUsedMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mem_used_mi_b: Option<f64>,
    #[serde(rename = "memAvailableMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mem_available_mi_b: Option<f64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BenchmarkResult {
    pub id: String,
    pub timestamp: String,
    pub config: ParsedScript,
    pub prompt_tokens_per_second: Option<f64>,
    pub prompt_token_count: Option<f64>,
    pub prompt_eval_time_ms: Option<f64>,
    pub generation_tokens_per_second: Option<f64>,
    pub generation_token_count: Option<f64>,
    pub draft_acceptance: Option<f64>,
    pub gen_drafts: Option<f64>,
    pub acc_drafts: Option<f64>,
    pub load_time_seconds: Option<f64>,
    pub generation_time_ms: Option<f64>,
    pub request_latency_ms: Option<f64>,
    pub prompt: String,
    pub response: String,
    pub gpus: Vec<GpuInfo>,
    pub backend: Option<GpuBackend>,
    pub device_vram: Vec<DeviceVram>,
    pub ram_used_mi_b: Option<f64>,
    pub rating: Option<Option<f64>>,
    pub favorite: Option<bool>,
    pub errors: Vec<String>,
    pub extra: BTreeMap<String, serde_json::Value>,
    pub missing_fields: std::collections::BTreeSet<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunedParams {
    #[serde(rename = "ctxSize")]
    #[serde(serialize_with = "serialize_number")]
    pub ctx_size: f64,
    #[serde(rename = "ngl")]
    #[serde(serialize_with = "serialize_number")]
    pub ngl: f64,
    #[serde(rename = "cacheTypeK")]
    pub cache_type_k: String,
    #[serde(rename = "cacheTypeV")]
    pub cache_type_v: String,
    #[serde(rename = "batchSize")]
    #[serde(serialize_with = "serialize_number")]
    pub batch_size: f64,
    #[serde(rename = "ubatchSize")]
    #[serde(serialize_with = "serialize_number")]
    pub ubatch_size: f64,
    #[serde(rename = "flashAttn")]
    pub flash_attn: bool,
    #[serde(rename = "device")]
    pub device: Vec<String>,
    #[serde(rename = "tensorSplit")]
    pub tensor_split: Option<Vec<f64>>,
    #[serde(rename = "nCpuMoe")]
    #[serde(serialize_with = "serialize_number")]
    pub n_cpu_moe: f64,
    #[serde(rename = "cacheReuse")]
    #[serde(serialize_with = "serialize_number")]
    pub cache_reuse: f64,
    #[serde(rename = "noMmproj")]
    pub no_mmproj: bool,
    #[serde(rename = "specDraftMax")]
    #[serde(serialize_with = "serialize_number")]
    pub spec_draft_max: f64,
    #[serde(rename = "cacheRam")]
    #[serde(serialize_with = "serialize_number")]
    pub cache_ram: f64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VramBreakdown {
    #[serde(rename = "perDeviceMiB")]
    pub per_device_mi_b: Vec<f64>,
    #[serde(rename = "totalMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub total_mi_b: f64,
    #[serde(rename = "weightsMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub weights_mi_b: f64,
    #[serde(rename = "kvMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub kv_mi_b: f64,
    #[serde(rename = "overheadMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub overhead_mi_b: f64,
    #[serde(rename = "fits")]
    pub fits: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateResponse {
    #[serde(rename = "devices")]
    pub devices: Vec<LlamaDevice>,
    #[serde(rename = "totalFreeMiB")]
    #[serde(serialize_with = "serialize_number")]
    pub total_free_mi_b: f64,
    #[serde(rename = "backend")]
    pub backend: GpuBackend,
    #[serde(rename = "modelMeta")]
    pub model_meta: ModelMeta,
    #[serde(rename = "heuristic")]
    pub heuristic: VramBreakdown,
    #[serde(rename = "recommendation")]
    pub recommendation: TunedParams,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryfitResponse {
    #[serde(rename = "perDevice")]
    pub per_device: Vec<DeviceVram>,
    #[serde(rename = "totalMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub total_mi_b: Option<f64>,
    #[serde(rename = "loadTimeSeconds")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub load_time_seconds: Option<f64>,
    #[serde(rename = "fits")]
    pub fits: bool,
    #[serde(rename = "error")]
    pub error: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMeta {
    #[serde(rename = "raw")]
    pub raw: String,
    #[serde(rename = "base")]
    pub base: String,
    #[serde(rename = "quant")]
    pub quant: Option<String>,
    #[serde(rename = "bytesPerParam")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub bytes_per_param: Option<f64>,
    #[serde(rename = "paramsB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub params_b: Option<f64>,
    #[serde(rename = "layers")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub layers: Option<f64>,
    #[serde(rename = "attentionLayers")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub attention_layers: Option<f64>,
    #[serde(rename = "kvHeads")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub kv_heads: Option<f64>,
    #[serde(rename = "headDim")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub head_dim: Option<f64>,
    #[serde(rename = "weightsFileMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub weights_file_mi_b: Option<f64>,
    #[serde(rename = "weightsFile")]
    pub weights_file: Option<String>,
    #[serde(rename = "mmprojSizeMiB")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub mmproj_size_mi_b: Option<f64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
pub type ServerStatus = String;
pub type GpuBackend = String;

fn present_nullable<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Option<f64>>, D::Error> {
    Option::<f64>::deserialize(d).map(Some)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CoreEvent {
    Log { entry: LogEntry },
    Status { status: StatusResponse },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultPatch {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_nullable"
    )]
    #[serde(serialize_with = "serialize_present_number")]
    pub rating: Option<Option<f64>>,
    pub favorite: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkRequest {
    pub script: String,
    pub prompt: Option<String>,
    #[serde(rename = "max_tokens", default = "default_max_tokens")]
    #[serde(serialize_with = "serialize_optional_number")]
    pub max_tokens: Option<f64>,
}
fn default_max_tokens() -> Option<f64> {
    Some(2048.0)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryfitRequest {
    pub script: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateRequest {
    pub script: String,
    pub params: Option<serde_json::Value>,
    pub priority: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuResponse {
    pub gpus: Vec<GpuInfo>,
    pub ram: RamInfo,
}

fn serialize_number<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.is_finite() && v.fract() == 0.0 && *v >= i64::MIN as f64 && *v < i64::MAX as f64 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}
fn serialize_optional_number<S: serde::Serializer>(
    v: &Option<f64>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => serialize_number(v, s),
        None => s.serialize_none(),
    }
}
fn serialize_present_number<S: serde::Serializer>(
    v: &Option<Option<f64>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    serialize_optional_number(&v.flatten(), s)
}

impl<'de> Deserialize<'de> for ParsedScript {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut map = BTreeMap::<String, serde_json::Value>::deserialize(d)?;
        let mut result = Self::default();
        if let Some(v) = map.remove("script") {
            result.script = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("script".into());
        }
        if let Some(v) = map.remove("binary") {
            result.binary = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("binary".into());
        }
        if let Some(v) = map.remove("argv") {
            result.argv = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("argv".into());
        }
        if let Some(v) = map.remove("model") {
            result.model = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("model".into());
        }
        if let Some(v) = map.remove("host") {
            result.host = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("host".into());
        }
        if let Some(v) = map.remove("port") {
            result.port = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("port".into());
        }
        if let Some(v) = map.remove("ctxSize") {
            result.ctx_size = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("ctxSize".into());
        }
        if let Some(v) = map.remove("batchSize") {
            result.batch_size = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("batchSize".into());
        }
        if let Some(v) = map.remove("ubatchSize") {
            result.ubatch_size = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("ubatchSize".into());
        }
        if let Some(v) = map.remove("cacheTypeK") {
            result.cache_type_k = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("cacheTypeK".into());
        }
        if let Some(v) = map.remove("cacheTypeV") {
            result.cache_type_v = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("cacheTypeV".into());
        }
        if let Some(v) = map.remove("device") {
            result.device = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("device".into());
        }
        if let Some(v) = map.remove("tensorSplit") {
            result.tensor_split = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("tensorSplit".into());
        }
        if let Some(v) = map.remove("temp") {
            result.temp = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("temp".into());
        }
        if let Some(v) = map.remove("topP") {
            result.top_p = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("topP".into());
        }
        if let Some(v) = map.remove("topK") {
            result.top_k = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("topK".into());
        }
        if let Some(v) = map.remove("ngl") {
            result.ngl = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("ngl".into());
        }
        if let Some(v) = map.remove("flashAttn") {
            result.flash_attn = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("flashAttn".into());
        }
        if let Some(v) = map.remove("threads") {
            result.threads = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("threads".into());
        }
        if let Some(v) = map.remove("minP") {
            result.min_p = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("minP".into());
        }
        if let Some(v) = map.remove("repeatPenalty") {
            result.repeat_penalty = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("repeatPenalty".into());
        }
        if let Some(v) = map.remove("modelFile") {
            result.model_file = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("modelFile".into());
        }
        if let Some(v) = map.remove("nCpuMoe") {
            result.n_cpu_moe = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("nCpuMoe".into());
        }
        if let Some(v) = map.remove("cacheReuse") {
            result.cache_reuse = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("cacheReuse".into());
        }
        if let Some(v) = map.remove("noMmproj") {
            result.no_mmproj = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("noMmproj".into());
        }
        if let Some(v) = map.remove("specDraftNMax") {
            result.spec_draft_n_max =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("specDraftNMax".into());
        }
        if let Some(v) = map.remove("cacheRam") {
            result.cache_ram = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("cacheRam".into());
        }
        result.extra = map;
        Ok(result)
    }
}
impl Serialize for ParsedScript {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = self.extra.clone();
        if !self.missing_fields.contains("script") {
            map.insert(
                "script".into(),
                serde_json::to_value(&self.script).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("binary") {
            map.insert(
                "binary".into(),
                serde_json::to_value(&self.binary).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("argv") {
            map.insert(
                "argv".into(),
                serde_json::to_value(&self.argv).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("model") {
            map.insert(
                "model".into(),
                serde_json::to_value(&self.model).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("host") {
            map.insert(
                "host".into(),
                serde_json::to_value(&self.host).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("port") {
            map.insert("port".into(), number_value(Some(self.port)));
        }
        if !self.missing_fields.contains("ctxSize") {
            map.insert("ctxSize".into(), number_value(self.ctx_size));
        }
        if !self.missing_fields.contains("batchSize") {
            map.insert("batchSize".into(), number_value(self.batch_size));
        }
        if !self.missing_fields.contains("ubatchSize") {
            map.insert("ubatchSize".into(), number_value(self.ubatch_size));
        }
        if !self.missing_fields.contains("cacheTypeK") {
            map.insert(
                "cacheTypeK".into(),
                serde_json::to_value(&self.cache_type_k).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("cacheTypeV") {
            map.insert(
                "cacheTypeV".into(),
                serde_json::to_value(&self.cache_type_v).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("device") {
            map.insert(
                "device".into(),
                serde_json::to_value(&self.device).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("tensorSplit") {
            map.insert(
                "tensorSplit".into(),
                serde_json::to_value(&self.tensor_split).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("temp") {
            map.insert("temp".into(), number_value(self.temp));
        }
        if !self.missing_fields.contains("topP") {
            map.insert("topP".into(), number_value(self.top_p));
        }
        if !self.missing_fields.contains("topK") {
            map.insert("topK".into(), number_value(self.top_k));
        }
        if !self.missing_fields.contains("ngl") {
            map.insert("ngl".into(), number_value(self.ngl));
        }
        if !self.missing_fields.contains("flashAttn") {
            map.insert(
                "flashAttn".into(),
                serde_json::to_value(self.flash_attn).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("threads") {
            map.insert("threads".into(), number_value(self.threads));
        }
        if !self.missing_fields.contains("minP") {
            map.insert("minP".into(), number_value(self.min_p));
        }
        if !self.missing_fields.contains("repeatPenalty") {
            map.insert("repeatPenalty".into(), number_value(self.repeat_penalty));
        }
        if !self.missing_fields.contains("modelFile") {
            map.insert(
                "modelFile".into(),
                serde_json::to_value(&self.model_file).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("nCpuMoe") {
            map.insert("nCpuMoe".into(), number_value(Some(self.n_cpu_moe)));
        }
        if !self.missing_fields.contains("cacheReuse") {
            map.insert("cacheReuse".into(), number_value(Some(self.cache_reuse)));
        }
        if !self.missing_fields.contains("noMmproj") {
            map.insert(
                "noMmproj".into(),
                serde_json::to_value(self.no_mmproj).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("specDraftNMax") {
            map.insert("specDraftNMax".into(), number_value(self.spec_draft_n_max));
        }
        if !self.missing_fields.contains("cacheRam") {
            map.insert("cacheRam".into(), number_value(self.cache_ram));
        }
        map.serialize(s)
    }
}

impl<'de> Deserialize<'de> for BenchmarkResult {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut map = BTreeMap::<String, serde_json::Value>::deserialize(d)?;
        let mut result = Self::default();
        if let Some(v) = map.remove("id") {
            result.id = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("id".into());
        }
        if let Some(v) = map.remove("timestamp") {
            result.timestamp = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("timestamp".into());
        }
        if let Some(v) = map.remove("config") {
            result.config = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("config".into());
        }
        if let Some(v) = map.remove("promptTokensPerSecond") {
            result.prompt_tokens_per_second =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("promptTokensPerSecond".into());
        }
        if let Some(v) = map.remove("promptTokenCount") {
            result.prompt_token_count =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("promptTokenCount".into());
        }
        if let Some(v) = map.remove("promptEvalTimeMs") {
            result.prompt_eval_time_ms =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("promptEvalTimeMs".into());
        }
        if let Some(v) = map.remove("generationTokensPerSecond") {
            result.generation_tokens_per_second =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result
                .missing_fields
                .insert("generationTokensPerSecond".into());
        }
        if let Some(v) = map.remove("generationTokenCount") {
            result.generation_token_count =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("generationTokenCount".into());
        }
        if let Some(v) = map.remove("draftAcceptance") {
            result.draft_acceptance =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("draftAcceptance".into());
        }
        if let Some(v) = map.remove("genDrafts") {
            result.gen_drafts = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("genDrafts".into());
        }
        if let Some(v) = map.remove("accDrafts") {
            result.acc_drafts = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("accDrafts".into());
        }
        if let Some(v) = map.remove("loadTimeSeconds") {
            result.load_time_seconds =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("loadTimeSeconds".into());
        }
        if let Some(v) = map.remove("generationTimeMs") {
            result.generation_time_ms =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("generationTimeMs".into());
        }
        if let Some(v) = map.remove("requestLatencyMs") {
            result.request_latency_ms =
                serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("requestLatencyMs".into());
        }
        if let Some(v) = map.remove("prompt") {
            result.prompt = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("prompt".into());
        }
        if let Some(v) = map.remove("response") {
            result.response = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("response".into());
        }
        if let Some(v) = map.remove("gpus") {
            result.gpus = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("gpus".into());
        }
        if let Some(v) = map.remove("backend") {
            result.backend = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("backend".into());
        }
        if let Some(v) = map.remove("deviceVram") {
            result.device_vram = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("deviceVram".into());
        }
        if let Some(v) = map.remove("ramUsedMiB") {
            result.ram_used_mi_b = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("ramUsedMiB".into());
        }
        if let Some(v) = map.remove("rating") {
            result.rating = Some(serde_json::from_value(v).map_err(serde::de::Error::custom)?);
        } else {
            result.missing_fields.insert("rating".into());
        }
        if let Some(v) = map.remove("favorite") {
            result.favorite = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("favorite".into());
        }
        if let Some(v) = map.remove("errors") {
            result.errors = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
        } else {
            result.missing_fields.insert("errors".into());
        }
        result.extra = map;
        Ok(result)
    }
}
impl Serialize for BenchmarkResult {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = self.extra.clone();
        if !self.missing_fields.contains("id") {
            map.insert(
                "id".into(),
                serde_json::to_value(&self.id).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("timestamp") {
            map.insert(
                "timestamp".into(),
                serde_json::to_value(&self.timestamp).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("config") {
            map.insert(
                "config".into(),
                serde_json::to_value(&self.config).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("promptTokensPerSecond") {
            map.insert(
                "promptTokensPerSecond".into(),
                number_value(self.prompt_tokens_per_second),
            );
        }
        if !self.missing_fields.contains("promptTokenCount") {
            map.insert(
                "promptTokenCount".into(),
                number_value(self.prompt_token_count),
            );
        }
        if !self.missing_fields.contains("promptEvalTimeMs") {
            map.insert(
                "promptEvalTimeMs".into(),
                number_value(self.prompt_eval_time_ms),
            );
        }
        if !self.missing_fields.contains("generationTokensPerSecond") {
            map.insert(
                "generationTokensPerSecond".into(),
                number_value(self.generation_tokens_per_second),
            );
        }
        if !self.missing_fields.contains("generationTokenCount") {
            map.insert(
                "generationTokenCount".into(),
                number_value(self.generation_token_count),
            );
        }
        if !self.missing_fields.contains("draftAcceptance") {
            map.insert(
                "draftAcceptance".into(),
                number_value(self.draft_acceptance),
            );
        }
        if !self.missing_fields.contains("genDrafts") {
            map.insert("genDrafts".into(), number_value(self.gen_drafts));
        }
        if !self.missing_fields.contains("accDrafts") {
            map.insert("accDrafts".into(), number_value(self.acc_drafts));
        }
        if !self.missing_fields.contains("loadTimeSeconds") {
            map.insert(
                "loadTimeSeconds".into(),
                number_value(self.load_time_seconds),
            );
        }
        if !self.missing_fields.contains("generationTimeMs") {
            map.insert(
                "generationTimeMs".into(),
                number_value(self.generation_time_ms),
            );
        }
        if !self.missing_fields.contains("requestLatencyMs") {
            map.insert(
                "requestLatencyMs".into(),
                number_value(self.request_latency_ms),
            );
        }
        if !self.missing_fields.contains("prompt") {
            map.insert(
                "prompt".into(),
                serde_json::to_value(&self.prompt).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("response") {
            map.insert(
                "response".into(),
                serde_json::to_value(&self.response).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("gpus") {
            map.insert(
                "gpus".into(),
                serde_json::to_value(&self.gpus).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("backend") {
            map.insert(
                "backend".into(),
                serde_json::to_value(&self.backend).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("deviceVram") {
            map.insert(
                "deviceVram".into(),
                serde_json::to_value(&self.device_vram).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("ramUsedMiB") {
            map.insert("ramUsedMiB".into(), number_value(self.ram_used_mi_b));
        }
        if !self.missing_fields.contains("rating") && self.rating.is_some() {
            map.insert("rating".into(), number_value(self.rating.flatten()));
        }
        if !self.missing_fields.contains("favorite") && self.favorite.is_some() {
            map.insert(
                "favorite".into(),
                serde_json::to_value(self.favorite).map_err(serde::ser::Error::custom)?,
            );
        }
        if !self.missing_fields.contains("errors") {
            map.insert(
                "errors".into(),
                serde_json::to_value(&self.errors).map_err(serde::ser::Error::custom)?,
            );
        }
        map.serialize(s)
    }
}

fn number_value(v: Option<f64>) -> serde_json::Value {
    match v {
        Some(v)
            if v.is_finite() && v.fract() == 0.0 && v >= i64::MIN as f64 && v < i64::MAX as f64 =>
        {
            serde_json::Value::from(v as i64)
        }
        Some(v) => serde_json::Value::from(v),
        None => serde_json::Value::Null,
    }
}
