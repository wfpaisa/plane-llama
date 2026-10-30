// Catálogo del referente incluido en el binario, sin depender de Bun en runtime.
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaFlag {
    pub name: String,
    pub long: String,
    pub short: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    pub default_value: Option<String>,
    pub description: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_description: Option<String>,
}
pub const CATALOG_JSON: &str = include_str!("../../../../assets/llama-flags.json");
pub static LLAMA_FLAGS: LazyLock<Vec<LlamaFlag>> =
    LazyLock::new(|| serde_json::from_str(CATALOG_JSON).unwrap_or_default());
pub fn flags_catalog() -> &'static [LlamaFlag] {
    &LLAMA_FLAGS
}
