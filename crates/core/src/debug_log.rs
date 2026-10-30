// Volcado de diagnóstico del último benchmark; no interrumpe la ejecución.
use crate::{Config, LogEntry, metrics::ParsedMetrics};
pub async fn dump_debug_log(
    cfg: &Config,
    lines: &[LogEntry],
    metrics: &ParsedMetrics,
    extra: &[String],
) {
    let body = format!(
        "# log_debug.txt — volcado del último benchmark\n# Generado: {}\n## Métricas parseadas\n{}\n{}\n## Slice de logs (cola)\n{}\n",
        chrono::Utc::now().to_rfc3339(),
        serde_json::to_string_pretty(metrics).unwrap_or_default(),
        extra.join("\n"),
        lines
            .iter()
            .skip(lines.len().saturating_sub(1000))
            .map(|l| format!(
                "{}{}",
                if l.stream == "system" {
                    "[system] "
                } else {
                    ""
                },
                l.msg
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    if let Err(e) = tokio::fs::write(cfg.file("log_debug.txt"), body).await {
        log::warn!("No se pudo guardar diagnóstico: {e}");
    }
}
