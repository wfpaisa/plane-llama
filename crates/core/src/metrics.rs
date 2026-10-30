// Métricas de logs, con preferencia por las últimas mediciones.
use crate::LogEntry;
use serde::{Deserialize, Serialize};
pub const DEFAULT_PROMPT: &str = "Un agricultor tiene 17 ovejas. Todas menos 9 se escapan. ¿Cuántas ovejas le quedan? Explica tu razonamiento paso a paso.\n\nLuego resuelve esto sin calculadora: ¿cuántos números primos hay entre 20 y 40? Lista cada uno y verifica brevemente por qué es primo.";
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedMetrics {
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
}
static REGEXES: std::sync::LazyLock<std::collections::HashMap<&'static str, Option<regex::Regex>>> =
    std::sync::LazyLock::new(|| {
        [
            r"(?i)prompt eval time.*?(\d+(?:\.\d+)?)\s*tokens per second",
            r"(?i)prompt eval time.*?/\s*(\d+)\s*tokens",
            r"(?i)prompt eval time\s*=\s*(\d+(?:\.\d+)?)\s*ms",
            r"(?i)eval time.*?(\d+(?:\.\d+)?)\s*tokens per second",
            r"(?i)eval time.*?/\s*(\d+)\s*tokens",
            r"(?i)eval time\s*=\s*(\d+(?:\.\d+)?)\s*ms",
            r"(?i)draft acceptance\s*=?\s*([0-9.]+)",
            r"(?i)model loaded.*?([0-9.]+)\s*ms",
            r"(?i)draft acceptance\s*=\s*[0-9.]+\s*\(\s*(\d+)\s*accepted\s*/\s*(\d+)\s*generated",
            r"(?i)draft-mtp:.*?#gen drafts\s*=\s*(\d+).*?#acc drafts\s*=\s*(\d+)",
        ]
        .into_iter()
        .map(|p| (p, regex::Regex::new(p).ok()))
        .collect()
    });
fn capture(line: &str, pattern: &str, group: usize) -> Option<f64> {
    REGEXES
        .get(pattern)?
        .as_ref()?
        .captures(line)?
        .get(group)?
        .as_str()
        .parse()
        .ok()
}
pub fn parse_metrics_from_logs(lines: &[LogEntry]) -> ParsedMetrics {
    let mut m = ParsedMetrics::default();
    for entry in lines.iter().rev() {
        let l = &entry.msg;
        if m.prompt_tokens_per_second.is_none() {
            m.prompt_tokens_per_second = capture(
                l,
                r"(?i)prompt eval time.*?(\d+(?:\.\d+)?)\s*tokens per second",
                1,
            );
        }
        if m.prompt_token_count.is_none() {
            m.prompt_token_count = capture(l, r"(?i)prompt eval time.*?/\s*(\d+)\s*tokens", 1);
        }
        if m.prompt_eval_time_ms.is_none() {
            m.prompt_eval_time_ms =
                capture(l, r"(?i)prompt eval time\s*=\s*(\d+(?:\.\d+)?)\s*ms", 1);
        }
        if m.generation_tokens_per_second.is_none()
            && !l.to_lowercase().contains("prompt eval time")
        {
            m.generation_tokens_per_second =
                capture(l, r"(?i)eval time.*?(\d+(?:\.\d+)?)\s*tokens per second", 1);
        }
        if m.generation_token_count.is_none() && !l.to_lowercase().contains("prompt eval time") {
            m.generation_token_count = capture(l, r"(?i)eval time.*?/\s*(\d+)\s*tokens", 1);
        }
        if m.generation_time_ms.is_none() && !l.to_lowercase().contains("prompt eval time") {
            m.generation_time_ms = capture(l, r"(?i)eval time\s*=\s*(\d+(?:\.\d+)?)\s*ms", 1);
        }
        if m.draft_acceptance.is_none() {
            m.draft_acceptance = capture(l, r"(?i)draft acceptance\s*=?\s*([0-9.]+)", 1);
        }
        if m.load_time_seconds.is_none() {
            m.load_time_seconds =
                capture(l, r"(?i)model loaded.*?([0-9.]+)\s*ms", 1).map(|v| v / 1000.0);
        }

        if m.gen_drafts.is_none() {
            let p = r"(?i)draft acceptance\s*=\s*[0-9.]+\s*\(\s*(\d+)\s*accepted\s*/\s*(\d+)\s*generated";
            if let Some(v) = capture(l, p, 2) {
                m.gen_drafts = Some(v);
                m.acc_drafts = capture(l, p, 1);
            } else {
                let p = r"(?i)draft-mtp:.*?#gen drafts\s*=\s*(\d+).*?#acc drafts\s*=\s*(\d+)";
                m.gen_drafts = capture(l, p, 1);
                m.acc_drafts = capture(l, p, 2);
            }
        }
    }
    if m.load_time_seconds.is_none() {
        let first = lines
            .iter()
            .position(|l| l.msg.to_lowercase().contains("loading model"));
        let last = lines
            .iter()
            .rposition(|l| l.msg.to_lowercase().contains("model loaded"));
        if let Some((a, b)) = first.zip(last)
            && b > a
        {
            m.load_time_seconds = Some((lines[b].t - lines[a].t) / 1000.0);
        }
    }
    m
}
