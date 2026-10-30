// Parseo de devices del backend y deltas de VRAM libre.
use crate::{DeviceVram, LlamaDevice};
pub fn vendor_from_name(name: &str) -> String {
    let n = name.to_lowercase();
    for (vendor, words) in [
        (
            "nvidia",
            &["nvidia", "geforce", "rtx", "gtx", "quadro", "tesla"][..],
        ),
        ("amd", &["amd", "radeon", "radv", "instinct", "rx"][..]),
        ("intel", &["intel", "arc", "iris", "uhd"][..]),
    ] {
        if words.iter().any(|w| n.contains(w)) {
            return vendor.into();
        }
    }
    "unknown".into()
}
static DEVICE_RE: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\s*(\w+\d+)\s*:\s*(.+?)\s*\((\d+)\s*MiB\s*,\s*(\d+)\s*MiB\s*free\)\s*$")
        .ok()
});
pub fn parse_list_devices(raw: &str) -> Vec<LlamaDevice> {
    let Some(re) = DEVICE_RE.as_ref() else {
        return vec![];
    };
    raw.lines()
        .filter_map(|l| {
            let c = re.captures(l)?;
            Some(LlamaDevice {
                id: c[1].into(),
                name: c[2].trim().into(),
                vendor: vendor_from_name(c[2].trim()),
                total_mi_b: c[3].parse().ok()?,
                free_mi_b: c[4].parse().ok()?,
                ..Default::default()
            })
        })
        .collect()
}
pub fn detect_backend(devices: &[LlamaDevice]) -> String {
    let p = devices
        .first()
        .map(|d| {
            d.id.trim_end_matches(|c: char| c.is_ascii_digit())
                .to_lowercase()
        })
        .unwrap_or_default();
    if ["cuda", "vulkan", "sycl", "metal", "opencl", "cann", "cpu"].contains(&p.as_str()) {
        p
    } else {
        "unknown".into()
    }
}
pub fn compute_device_vram(
    baseline: &[LlamaDevice],
    final_: &[LlamaDevice],
    filter: Option<&str>,
) -> Vec<DeviceVram> {
    final_
        .iter()
        .filter(|d| filter.is_none_or(|f| f.is_empty() || f.split(',').any(|s| s.trim() == d.id)))
        .map(|d| DeviceVram {
            device: d.clone(),
            used_mi_b: baseline
                .iter()
                .find(|b| b.id == d.id)
                .map(|b| (b.free_mi_b - d.free_mi_b).max(0.0)),
            ..Default::default()
        })
        .collect()
}

pub async fn list_devices(binary: &str) -> Vec<LlamaDevice> {
    let Ok(mut cmd) = crate::server_manager::command(binary) else {
        return vec![];
    };
    cmd.arg("--list-devices").kill_on_drop(true);
    match tokio::time::timeout(std::time::Duration::from_secs(15), cmd.output()).await {
        Ok(Ok(output)) => parse_list_devices(&format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )),
        _ => vec![],
    }
}
