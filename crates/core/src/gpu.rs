// Lectura NVIDIA y AMD; parseo separado para pruebas sin hardware.
use crate::{GpuInfo, script_parser::to_num_or_null};
pub fn parse_nvidia_csv(raw: &str) -> Vec<GpuInfo> {
    raw.lines()
        .filter_map(|l| {
            let p: Vec<_> = l.split(',').map(str::trim).collect();
            // Con nombre: índice, nombre, uso, memoria usada y total; sin él, cuatro columnas.
            let (name, p) = match p.len() {
                n if n >= 5 => (Some(p[1].to_owned()), [p[0], p[2], p[3], p[4]]),
                4 => (None, [p[0], p[1], p[2], p[3]]),
                _ => return None,
            };
            Some(GpuInfo {
                index: format!("nvidia{}", p[0]),
                vendor: "nvidia".into(),
                name: name.filter(|n| !n.is_empty()),
                gpu_util_pct: to_num_or_null(Some(p[1])),
                mem_used_mi_b: to_num_or_null(Some(p[2])),
                mem_total_mi_b: to_num_or_null(Some(p[3])),
                ..Default::default()
            })
        })
        .collect()
}
pub async fn read_gpu_stats() -> Vec<GpuInfo> {
    let mut gpus = match tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await
    {
        Ok(out) => parse_nvidia_csv(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => vec![],
    };
    if let Ok(mut cards) = tokio::fs::read_dir("/sys/class/drm").await {
        while let Ok(Some(card)) = cards.next_entry().await {
            let name = card.file_name().to_string_lossy().into_owned();
            if !name
                .strip_prefix("card")
                .is_some_and(|s| s.parse::<u32>().is_ok())
            {
                continue;
            }
            let dev = card.path().join("device");
            let vendor = tokio::fs::read_to_string(dev.join("vendor"))
                .await
                .unwrap_or_default();
            if !vendor.contains("0x1002") && !vendor.to_lowercase().contains("amd") {
                continue;
            }
            async fn num(p: std::path::PathBuf) -> Option<f64> {
                tokio::fs::read_to_string(p).await.ok()?.trim().parse().ok()
            }
            let (used, total, util, product) = tokio::join!(
                num(dev.join("mem_info_vram_used")),
                num(dev.join("mem_info_vram_total")),
                num(dev.join("gpu_busy_percent")),
                tokio::fs::read_to_string(dev.join("product_name"))
            );
            gpus.push(GpuInfo {
                index: format!("amdgpu-{name}"),
                vendor: "amd".into(),
                name: product
                    .ok()
                    .map(|n| n.trim().to_owned())
                    .filter(|n| !n.is_empty()),
                mem_used_mi_b: used.map(|n| n / 1048576.0),
                mem_total_mi_b: total.map(|n| n / 1048576.0),
                gpu_util_pct: util,
                ..Default::default()
            });
        }
    }
    gpus
}
pub fn subtract_gpu_baseline(final_: &[GpuInfo], baseline: &[GpuInfo]) -> Vec<GpuInfo> {
    final_
        .iter()
        .map(|g| {
            let mut g = g.clone();
            if let Some(b) = baseline.iter().find(|b| b.index == g.index)
                && let Some((f, b)) = g.mem_used_mi_b.zip(b.mem_used_mi_b)
            {
                g.mem_used_mi_b = Some((f - b).max(0.0));
            }
            g
        })
        .collect()
}
