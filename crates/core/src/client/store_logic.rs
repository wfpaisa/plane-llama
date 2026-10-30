// Derivados del estado del cliente y orden estable para tablas y gráficos.
use crate::*;
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bests {
    pub p: f64,
    pub g: f64,
    pub d: f64,
    pub l: f64,
    pub gt: f64,
}
// Intl.Collator usa en-US por defecto en el referente de este proyecto.
static COLLATOR: std::sync::LazyLock<Option<icu_collator::CollatorBorrowed<'static>>> =
    std::sync::LazyLock::new(|| {
        icu_collator::Collator::try_new(icu_locale::locale!("en-US").into(), Default::default())
            .ok()
    });
pub fn compare_text(a: &str, b: &str) -> std::cmp::Ordering {
    COLLATOR
        .as_ref()
        .map(|c| c.compare(a, b))
        .unwrap_or_else(|| a.cmp(b))
}
pub fn compute_bests(history: &[BenchmarkResult]) -> Bests {
    let max = |get: fn(&BenchmarkResult) -> Option<f64>| {
        history
            .iter()
            .filter_map(get)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let min = |get: fn(&BenchmarkResult) -> Option<f64>| {
        history.iter().filter_map(get).fold(f64::INFINITY, f64::min)
    };
    Bests {
        p: max(|r| r.prompt_tokens_per_second),
        g: max(|r| r.generation_tokens_per_second),
        d: max(|r| r.draft_acceptance),
        l: min(|r| r.load_time_seconds),
        gt: min(|r| r.generation_time_ms),
    }
}
pub fn running(status: &StatusResponse) -> bool {
    matches!(status.status.as_str(), "running" | "starting")
}
pub fn status_label(status: &str) -> &str {
    match status {
        "stopped" => "detenido",
        "starting" => "iniciando…",
        "running" => "corriendo",
        "error" => "error",
        s => s,
    }
}
pub fn status_meta(s: &StatusResponse) -> String {
    let mut text = String::new();
    if let Some(pid) = s.pid.filter(|n| *n != 0.) {
        text += &format!("pid {pid} · ");
    }
    if let Some(url) = &s.url {
        text += &format!("{url} · ");
    }
    if let Some(error) = &s.error {
        text += error;
    }
    text
}
pub fn model_options(history: &[BenchmarkResult]) -> Vec<String> {
    let mut options: Vec<_> = history
        .iter()
        .filter_map(|r| super::format::model_base(r.config.model.as_deref()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    options.sort_by(|a, b| compare_text(a, b));
    options
}
pub fn selected_results<'a>(
    history: &'a [BenchmarkResult],
    ids: &BTreeSet<String>,
) -> Vec<&'a BenchmarkResult> {
    history.iter().filter(|r| ids.contains(&r.id)).collect()
}
pub fn chart_results<'a>(
    history: &'a [BenchmarkResult],
    order: &[String],
) -> Vec<&'a BenchmarkResult> {
    order
        .iter()
        .filter_map(|id| history.iter().find(|r| &r.id == id))
        .collect()
}
pub fn filter_history<'a>(
    history: &'a [BenchmarkResult],
    models: &[String],
) -> Vec<&'a BenchmarkResult> {
    history
        .iter()
        .filter(|r| {
            models.is_empty()
                || super::format::model_base(r.config.model.as_deref())
                    .is_some_and(|m| models.contains(&m))
        })
        .collect()
}
pub fn sort_history<'a>(
    history: &'a [BenchmarkResult],
    column: &str,
    descending: bool,
) -> Vec<&'a BenchmarkResult> {
    let mut rows: Vec<_> = history.iter().collect();
    if column.is_empty() {
        return rows;
    }
    rows.sort_by(|a, b| {
        let row = |r: &BenchmarkResult| {
            let mut value = serde_json::to_value(r).unwrap_or_default();
            value["modelBase"] = serde_json::json!(
                super::format::model_base(r.config.model.as_deref()).unwrap_or_default()
            );
            value["totalVramMiB"] = serde_json::json!(if r.device_vram.is_empty() {
                r.gpus.iter().filter_map(|g| g.mem_used_mi_b).sum::<f64>()
            } else {
                r.device_vram
                    .iter()
                    .filter_map(|d| d.used_mi_b)
                    .sum::<f64>()
            });
            value
        };
        let av = row(a);
        let bv = row(b);
        let get = |v: &serde_json::Value| {
            let mut v = v;
            for key in column.split('.') {
                v = &v[key];
            }
            v.clone()
        };
        let (av, bv) = (get(&av), get(&bv));
        let cmp = match (av.as_f64(), bv.as_f64()) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ if av.is_string() && bv.is_string() => {
                compare_text(av.as_str().unwrap_or(""), bv.as_str().unwrap_or(""))
            }
            _ => {
                if av.is_null() && bv.is_null() {
                    std::cmp::Ordering::Equal
                } else if av.is_null() {
                    std::cmp::Ordering::Less
                } else if bv.is_null() {
                    std::cmp::Ordering::Greater
                } else {
                    av.to_string().cmp(&bv.to_string())
                }
            }
        };
        if descending { cmp.reverse() } else { cmp }
    });
    rows
}
