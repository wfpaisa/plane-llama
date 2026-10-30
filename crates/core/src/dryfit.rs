// Calibración de memoria real sin enviar inferencia.
use crate::{benchmark::cancellable, devices::*, gpu::*, server_manager::*, *};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
pub(crate) async fn run_dryfit(
    core: &Core,
    script: &str,
    token: &CancellationToken,
) -> Result<DryfitResponse> {
    let result = cancellable(token, async {
        let parsed = script_parser::parse_script(script)?;
        assert_binary_exists(&parsed.binary)?;
        let since = core.logs(None).cursor as u64;
        let (baseline, gpu_base) = tokio::join!(list_devices(&parsed.binary), read_gpu_stats());
        let mut slot = core.inner.server.lock().await;
        let server = start_server(&parsed, core.inner.state.clone()).await?;
        *slot = Some(server.clone());
        drop(slot);
        server.ready().await?;
        tokio::time::sleep(Duration::from_millis(4500)).await;
        let final_devices = list_devices(&parsed.binary).await;
        let per_device = compute_device_vram(&baseline, &final_devices, parsed.device.as_deref());
        let measured: Vec<_> = per_device.iter().filter_map(|d| d.used_mi_b).collect();
        let total = if !measured.is_empty() {
            Some(measured.iter().sum::<f64>())
        } else {
            let gpus = subtract_gpu_baseline(&read_gpu_stats().await, &gpu_base);
            let sum: f64 = gpus.iter().filter_map(|g| g.mem_used_mi_b).sum();
            (sum > 0.0).then_some(sum)
        };
        let capacity: f64 = per_device.iter().map(|d| d.device.total_mi_b).sum();
        Ok(DryfitResponse {
            fits: total.is_none_or(|t| capacity <= 0.0 || t <= capacity),
            total_mi_b: total,
            per_device,
            load_time_seconds: metrics::parse_metrics_from_logs(&core.logs(Some(since)).entries)
                .load_time_seconds,
            error: None,
            ..Default::default()
        })
    })
    .await;
    match result {
        Ok(r) => Ok(r),
        Err(e) => Ok(DryfitResponse {
            fits: false,
            error: Some(e.to_string()),
            ..Default::default()
        }),
    }
}
