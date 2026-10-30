// Ciclo de inferencia con cancelación, métricas y persistencia.
use crate::{devices::*, gpu::*, mem::*, metrics::*, server_manager::*, *};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub(crate) async fn cancellable<T>(
    token: &CancellationToken,
    future: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! { biased; _ = token.cancelled() => Err(CoreError::Cancelled), r = future => r }
}
/// Espera a que `/health` responda. Si el proceso muere mientras carga el modelo falla en
/// el acto, en vez de agotar el plazo.
pub async fn wait_for_server(
    base: &str,
    server: Option<&ManagedServer>,
    token: &CancellationToken,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    cancellable(token, async {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(300) {
            if server.is_some_and(|s| s.is_exited()) {
                return Err(CoreError::Invalid(
                    "llama-server terminó mientras cargaba el modelo; revisa los logs.".into(),
                ));
            }
            if let Ok(response) = client.get(format!("{base}/health")).send().await {
                if response.status().is_success() {
                    return Ok(());
                }
                if response.status() == reqwest::StatusCode::NOT_FOUND
                    && client
                        .get(base)
                        .send()
                        .await
                        .is_ok_and(|r| r.status().is_success())
                {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(CoreError::Invalid(
            "Servidor no respondió tras 300 s".into(),
        ))
    })
    .await
}
pub async fn poll_metrics_until_ready(
    core: &Core,
    since: u64,
    token: &CancellationToken,
) -> Result<ParsedMetrics> {
    cancellable(token, async {
        let start = Instant::now();
        loop {
            let metrics = parse_metrics_from_logs(&core.logs(Some(since)).entries);
            if metrics.generation_tokens_per_second.is_some()
                || start.elapsed() >= Duration::from_secs(5)
            {
                return Ok(metrics);
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    })
    .await
}
fn empty_result(config: ParsedScript, prompt: String, errors: Vec<String>) -> BenchmarkResult {
    BenchmarkResult {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        config,
        prompt,
        errors,
        ..Default::default()
    }
}
pub(crate) async fn run_benchmark(
    core: &Core,
    req: BenchmarkRequest,
    token: &CancellationToken,
) -> Result<BenchmarkResult> {
    let prompt = req
        .prompt
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROMPT.into());
    let parsed = match script_parser::parse_script(&req.script) {
        Ok(p) => p,
        Err(e) => {
            return Ok(empty_result(
                script_parser::empty_parsed_script(),
                prompt,
                vec![format!("Script inválido: {e}")],
            ));
        }
    };
    if let Err(e) = assert_binary_exists(&parsed.binary) {
        return Ok(empty_result(parsed, prompt, vec![e.to_string()]));
    }
    let since = core.logs(None).cursor as u64;
    core.inner
        .state
        .logs
        .system("benchmark: iniciando llama-server…");
    let (gpu_base, ram_base, devices_base) = tokio::join!(
        read_gpu_stats(),
        read_ram_stats(),
        list_devices(&parsed.binary)
    );
    cancellable(token, async {
        let mut slot = core.inner.server.lock().await;
        let managed = match start_server(&parsed, core.inner.state.clone()).await {
            Ok(s) => s, Err(e) => return Ok(empty_result(parsed.clone(), prompt.clone(), vec![format!("No se pudo iniciar el servidor: {e}")])),
        };
        *slot = Some(managed.clone());
        drop(slot);
        cancellable(token, managed.ready()).await.map_err(|e| if token.is_cancelled() { CoreError::Cancelled } else { CoreError::Invalid(format!("El servidor no arrancó: {e}. Revisá el script, su formato y los flags (binario, modelo, rutas, comillas, continuaciones \\).")) })?;
        let base = url_for(&parsed);
        wait_for_server(&base, Some(&managed), token).await?;
        core.inner.state.logs.system("benchmark: servidor responde, ejecutando request…");
        let mut body = json!({"messages":[{"role":"user","content":prompt}],"max_tokens":req.max_tokens.map(|n| if n > 0.0 { n } else { 2048.0 }).unwrap_or(-1.0),"stream":false});
        if let Some(m) = &parsed.model { body["model"] = json!(m.split(':').next().unwrap_or(m)); }
        for (key, value) in [("temperature", parsed.temp), ("top_p", parsed.top_p), ("top_k", parsed.top_k)] { if let Some(v) = value { body[key] = json!(v); } }
        let start = Instant::now();
        let mut errors = vec![];
        let mut response = String::new();
        let request = async {
            let resp = reqwest::Client::new().post(format!("{base}/v1/chat/completions")).json(&body).send().await?;
            if !resp.status().is_success() { errors.push(format!("HTTP {} en /v1/chat/completions", resp.status().as_u16())); }
            else {
                let data: serde_json::Value = resp.json().await?;
                let message = &data["choices"][0]["message"];
                response = message["content"].as_str().filter(|s| !s.is_empty()).or_else(|| message["reasoning_content"].as_str()).unwrap_or("").into();
            }
            Ok(())
        };
        if let Err(e) = cancellable(token, request).await { if token.is_cancelled() { return Err(CoreError::Cancelled); } errors.push(format!("Fallo en request: {e}")); }
        let latency = start.elapsed().as_secs_f64() * 1000.0;
        let m = poll_metrics_until_ready(core, since, token).await?;
        let (gpu_final, ram_final, devices_final) = tokio::join!(read_gpu_stats(), read_ram_stats(), list_devices(&parsed.binary));
        let backend = detect_backend(&devices_base);
        let mut result = empty_result(parsed.clone(), prompt, errors);
        result.response = response;
        result.request_latency_ms = Some(latency);
        result.prompt_tokens_per_second = m.prompt_tokens_per_second;
        result.prompt_token_count = m.prompt_token_count;
        result.prompt_eval_time_ms = m.prompt_eval_time_ms;
        result.generation_tokens_per_second = m.generation_tokens_per_second;
        result.generation_token_count = m.generation_token_count;
        result.generation_time_ms = m.generation_time_ms;
        result.load_time_seconds = m.load_time_seconds;
        result.draft_acceptance = m.draft_acceptance;
        result.gen_drafts = m.gen_drafts;
        result.acc_drafts = m.acc_drafts;
        result.gpus = subtract_gpu_baseline(&gpu_final, &gpu_base);
        result.ram_used_mi_b = subtract_ram_baseline(&ram_final, &ram_base);
        result.backend = (backend != "unknown").then_some(backend);
        result.device_vram = compute_device_vram(&devices_base, &devices_final, parsed.device.as_deref());
        debug_log::dump_debug_log(&core.inner.cfg, &core.logs(Some(since)).entries, &m, &[format!("requestLatencyMs: {latency}")]).await;
        core.inner.history.lock().save(result.clone())?;
        core.inner.state.logs.system("benchmark: finalizado y guardado.");
        Ok(result)
    }).await
}
