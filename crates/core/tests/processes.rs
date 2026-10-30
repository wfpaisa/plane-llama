// Pruebas reales de procesos contra el fake, registradas por su paquete Cargo.
use plane_llama_core::{script_parser::parse_script, server_manager::*, *};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tempfile::TempDir;
fn core() -> (TempDir, Core) {
    let dir = TempDir::new().unwrap();
    let core = Core::new(Config {
        data_dir: dir.path().into(),
    })
    .unwrap();
    (dir, core)
}
fn script() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    format!(
        "{} --port {port} -hf org/Qwen3-8B:Q4_K_S",
        env!("CARGO_BIN_EXE_fake-llama-server")
    )
}
fn request(script: String) -> BenchmarkRequest {
    BenchmarkRequest {
        script,
        prompt: Some("Prueba".into()),
        max_tokens: Some(10.),
    }
}
async fn starting(core: &Core) -> u32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(pid) = core.status().pid {
                return pid as u32;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
fn gone(pid: u32) {
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "proceso huérfano {pid}"
    );
}
#[tokio::test]
async fn start_ready_stop_and_events() {
    let state = Arc::new(state::AppState::default());
    let mut events = state.logs.events.subscribe();
    let parsed = parse_script(&script()).unwrap();
    let managed = start_server(&parsed, state.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), managed.ready())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.status.read().status, "running");
    assert!(events.try_recv().is_ok());
    assert!(
        reqwest::get(format!("{}/health", url_for(&parsed)))
            .await
            .unwrap()
            .status()
            .is_success()
    );
    stop_server(&managed).await;
    gone(managed.pid);
    assert_eq!(state.status.read().status, "stopped");
}
#[tokio::test]
async fn benchmark_persists_metrics_and_releases_reservation() {
    let (dir, core) = core();
    let result = core.benchmark(request(script())).await.unwrap();
    assert!(result.errors.is_empty());
    assert_eq!(result.generation_tokens_per_second, Some(50.));
    assert_eq!(result.prompt_tokens_per_second, Some(200.));
    assert_eq!(result.generation_token_count, Some(10.));
    assert_eq!(result.draft_acceptance, Some(0.75));
    assert!(result.load_time_seconds.is_some());
    assert!(result.request_latency_ms.is_some());
    assert!(!result.response.is_empty());
    assert_eq!(core.history().await.unwrap().len(), 1);
    assert_eq!(
        Core::new(Config {
            data_dir: dir.path().into()
        })
        .unwrap()
        .history()
        .await
        .unwrap()[0]
            .id,
        result.id
    );
    assert_eq!(core.status().status, "stopped");
    let patched = core
        .patch_result(
            &result.id,
            ResultPatch {
                rating: Some(Some(9.)),
                favorite: Some(true),
            },
        )
        .await
        .unwrap();
    assert_eq!(patched.favorite, Some(true));
    core.benchmark(request(script())).await.unwrap();
    assert_eq!(core.history().await.unwrap().len(), 2);
    core.delete_results(&[result.id]).await.unwrap();
    assert_eq!(core.history().await.unwrap().len(), 1);
    core.clear_history().await.unwrap();
    assert!(core.history().await.unwrap().is_empty());
}
#[tokio::test]
async fn cancel_conflict_and_no_orphan() {
    let (_dir, core) = core();
    let other = core.clone();
    let task = tokio::spawn(async move { other.benchmark(request(script())).await });
    let pid = starting(&core).await;
    assert!(matches!(
        core.benchmark(request(script())).await,
        Err(CoreError::Conflict(_))
    ));
    assert!(matches!(
        core.start(&script()).await,
        Err(CoreError::Conflict(_))
    ));
    assert!(matches!(
        core.dryfit(DryfitRequest { script: script() }).await,
        Err(CoreError::Conflict(_))
    ));
    core.stop_benchmark();
    assert!(matches!(task.await.unwrap(), Err(CoreError::Cancelled)));
    gone(pid);
    assert!(core.history().await.unwrap().is_empty());
}
#[tokio::test]
async fn manual_conflict_shutdown_and_abandoned_future() {
    let (_dir, core) = core();
    let status = core.start(&script()).await.unwrap();
    let pid = status.pid.unwrap() as u32;
    assert!(matches!(
        core.benchmark(request(script())).await,
        Err(CoreError::Conflict(_))
    ));
    core.shutdown().await;
    gone(pid);
    let other = core.clone();
    let task = tokio::spawn(async move { other.benchmark(request(script())).await });
    let pid = starting(&core).await;
    task.abort();
    let _ = task.await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::path::Path::new(&format!("/proc/{pid}")).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    gone(pid);
}
#[tokio::test]
async fn defaults_devices_estimate_and_dryfit() {
    let (_dir, core) = core();
    assert!(core.script_default().await.is_err());
    assert_eq!(
        core.prompt_default().await.unwrap(),
        metrics::DEFAULT_PROMPT
    );
    core.save_script_default("llama-server").await.unwrap();
    assert_eq!(core.script_default().await.unwrap(), "llama-server");
    core.save_prompt_default("hola").await.unwrap();
    assert_eq!(core.prompt_default().await.unwrap(), "hola");
    core.save_flags_favorites(&["--port".into()]).await.unwrap();
    assert_eq!(core.flags_favorites().await.unwrap(), vec!["--port"]);
    let devices = devices::list_devices(env!("CARGO_BIN_EXE_fake-llama-server")).await;
    assert_eq!(devices.len(), 1);
    let estimate = core
        .estimate(EstimateRequest {
            script: script(),
            params: None,
            priority: None,
        })
        .await
        .unwrap();
    assert_eq!(estimate.backend, "cuda");
    assert!(estimate.heuristic.total_mi_b > 0.);
    let response = core
        .dryfit(DryfitRequest { script: script() })
        .await
        .unwrap();
    assert!(response.error.is_none());
    assert!(response.fits);
    assert_eq!(response.total_mi_b, Some(0.));
    assert!(response.load_time_seconds.is_some());
    assert_eq!(core.status().status, "stopped");
    let response = core
        .dryfit(DryfitRequest { script: "".into() })
        .await
        .unwrap();
    assert!(!response.fits);
    assert!(response.error.is_some());
}
#[tokio::test]
async fn crash_fails_fast_with_exit_code() {
    // Un wrapper por prueba evita modificar el entorno global del proceso de tests.
    use std::os::unix::fs::PermissionsExt;
    let (dir, core) = core();
    let wrapper = dir.path().join("crash-server");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport FAKE_CRASH=1\nexec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_fake-llama-server")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    let start = Instant::now();
    let error = core
        .benchmark(request(format!("{} --port 0", wrapper.display())))
        .await
        .unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(error.to_string().contains("código 1"));
    assert!(core.history().await.unwrap().is_empty());
}

fn wrapper(dir: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.path().join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}
async fn logged(core: &Core, needle: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !core
            .logs(None)
            .entries
            .iter()
            .any(|l| l.msg.contains(needle))
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn cancel_during_request_and_dryfit() {
    let (dir, core) = core();
    let bin = wrapper(
        &dir,
        "slow-chat",
        &format!(
            "export FAKE_CHAT_MS=800\nexec '{}' \"$@\"",
            env!("CARGO_BIN_EXE_fake-llama-server")
        ),
    );
    let other = core.clone();
    let script_text = script().replacen(
        env!("CARGO_BIN_EXE_fake-llama-server"),
        bin.to_str().unwrap(),
        1,
    );
    let task = tokio::spawn(async move { other.benchmark(request(script_text)).await });
    let pid = starting(&core).await;
    logged(&core, "fake request:").await;
    core.stop_benchmark();
    assert!(matches!(task.await.unwrap(), Err(CoreError::Cancelled)));
    gone(pid);
    core.clear_logs();
    let other = core.clone();
    let task = tokio::spawn(async move { other.dryfit(DryfitRequest { script: script() }).await });
    let pid = starting(&core).await;
    logged(&core, "server is listening").await;
    core.stop_benchmark();
    let result = task.await.unwrap().unwrap();
    assert!(!result.fits);
    assert!(result.error.unwrap().contains("cancelado"));
    gone(pid);
}
#[tokio::test]
async fn delayed_metrics_and_request_contract() {
    let (dir, core) = core();
    let bin = wrapper(
        &dir,
        "late-timings",
        &format!(
            "export FAKE_TIMING_DELAY_MS=700\nexec '{}' \"$@\"",
            env!("CARGO_BIN_EXE_fake-llama-server")
        ),
    );
    let script = format!(
        "{} --temp 0.2 --top-p 0.8 {}",
        script().replacen(
            env!("CARGO_BIN_EXE_fake-llama-server"),
            bin.to_str().unwrap(),
            1
        ),
        "--unknown ignored"
    );
    let mut req = request(script);
    req.max_tokens = None;
    let result = core.benchmark(req).await.unwrap();
    assert_eq!(result.generation_tokens_per_second, Some(50.));
    let body = core
        .logs(None)
        .entries
        .into_iter()
        .find_map(|l| l.msg.strip_prefix("fake request: ").map(str::to_string))
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["max_tokens"], -1.);
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["top_p"], 0.8);
    assert!(body.get("top_k").is_none());
    assert_eq!(body["model"], "org/Qwen3-8B");
}
#[tokio::test]
async fn stop_kills_process_group_and_escalates() {
    let dir = TempDir::new().unwrap();
    let child_file = dir.path().join("child.pid");
    let bin = wrapper(
        &dir,
        "with-child",
        &format!(
            "sleep 60 &\nchild=$!\necho $child > '{}'\ntrap 'wait \"$child\"; exit 0' TERM\necho 'server is listening'\nwait \"$child\"",
            child_file.display()
        ),
    );
    let state = Arc::new(state::AppState::default());
    let managed = start_server(&parse_script(bin.to_str().unwrap()).unwrap(), state.clone())
        .await
        .unwrap();
    managed.ready().await.unwrap();
    let child: u32 = std::fs::read_to_string(child_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(child as i32)))
            .unwrap()
            .as_raw(),
        managed.pid as i32
    );
    managed.stop(Duration::from_secs(1)).await;
    gone(managed.pid);
    gone(child);
    let bin = wrapper(
        &dir,
        "ignore-term",
        "trap '' TERM\necho 'server is listening'\nwhile :; do :; done",
    );
    let managed = start_server(&parse_script(bin.to_str().unwrap()).unwrap(), state)
        .await
        .unwrap();
    managed.ready().await.unwrap();
    let start = Instant::now();
    managed.stop(Duration::from_millis(80)).await;
    assert!(start.elapsed() < Duration::from_secs(1));
    gone(managed.pid);
}
