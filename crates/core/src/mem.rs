// Lectura y parseo de RAM de Linux.
use crate::RamInfo;
pub fn parse_meminfo(raw: &str) -> RamInfo {
    let value = |key: &str| {
        raw.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k == key)
                .then(|| {
                    v.split_whitespace()
                        .next()?
                        .parse::<f64>()
                        .ok()
                        .map(|n| n / 1024.0)
                })
                .flatten()
        })
    };
    let total = value("MemTotal");
    let available = value("MemAvailable");
    RamInfo {
        mem_total_mi_b: total,
        mem_available_mi_b: available,
        mem_used_mi_b: total.zip(available).map(|(t, a)| (t - a).max(0.0)),
        ..Default::default()
    }
}
pub async fn read_ram_stats() -> RamInfo {
    parse_meminfo(
        &tokio::fs::read_to_string("/proc/meminfo")
            .await
            .unwrap_or_default(),
    )
}
pub fn subtract_ram_baseline(final_: &RamInfo, baseline: &RamInfo) -> Option<f64> {
    final_
        .mem_used_mi_b
        .zip(baseline.mem_used_mi_b)
        .map(|(f, b)| (f - b).max(0.0))
}
