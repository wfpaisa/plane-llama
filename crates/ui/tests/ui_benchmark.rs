// Recorrido completo por la app y su puente contra un proceso fake real.
use egui_kittest::{Harness, kittest::Queryable};
use plane_llama_core::{Config, Core};
use plane_llama_ui::App;

/// Caché de Hugging Face con el modelo `-hf` de los scripts, para que la app no pregunte
/// si descargarlo. Compartida por los tests de este binario.
fn fake_hf_cache() {
    static CACHE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let hub = tempfile::tempdir().unwrap().keep();
        let snapshot = hub.join("models--fake--Modelo-7B/snapshots/rev");
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::write(snapshot.join("Modelo-7B-Q4_K_M.gguf"), b"gguf").unwrap();
        // SAFETY: se fija una sola vez, antes de crear el core y la app.
        unsafe { std::env::set_var("HF_HUB_CACHE", &hub) };
        hub
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn app_runs_benchmark_through_ui_and_persists_history() {
    fake_hf_cache();
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        data_dir: dir.path().into(),
    };
    cfg.ensure_data_dir().unwrap();
    let core = Core::new(cfg.clone()).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let script = format!(
        "{} --host 127.0.0.1 --port {} -hf fake/Modelo-7B:Q4_K_M",
        env!("CARGO_BIN_EXE_fake-llama-server"),
        port
    );
    let handle = tokio::runtime::Handle::current();
    let c = core.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440., 1100.))
        .build_ui_state(
            move |ui, app: &mut Option<App>| {
                let app = app.get_or_insert_with(|| {
                    App::headless(c.clone(), handle.clone(), ui.ctx().clone())
                });
                app.frame(ui);
            },
            None,
        );
    {
        let app = h.state_mut().as_mut().unwrap();
        app.store.script = script;
        app.store.simple = false;
        app.store.tab = 1;
        app.store.prompt = "Responde con una frase".into();
        app.store.max_tokens = 64.;
    }
    h.run_steps(2);
    h.get_all_by_label("Ejecutar benchmark")
        .last()
        .unwrap()
        .click();
    h.run_steps(2);
    assert!(h.state().as_ref().unwrap().store.bench_started.is_some());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while h.state().as_ref().unwrap().store.last_result.is_none()
        && std::time::Instant::now() < deadline
    {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        h.run_steps(1);
    }
    h.run_steps(2);
    let s = &h.state().as_ref().unwrap().store;
    let result = s.last_result.as_ref().expect("El benchmark debe terminar");
    assert!(result.generation_tokens_per_second.unwrap_or_default() > 0.);
    assert!(!result.response.is_empty());
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert!(s.bench_started.is_none());
    assert!(s.show_response);
    assert!(!s.logs.is_empty());
    let result_id = result.id.clone();
    h.state_mut().as_mut().unwrap().store.notices.clear();
    assert!(
        h.run() < 10,
        "La app debe estabilizarse sin repintar continuamente"
    );
    let disk = core.history().await.unwrap();
    assert_eq!(disk.len(), 1);
    assert_eq!(disk[0].id, result_id);
    assert!(cfg.file("history.json").exists());
    core.shutdown().await;
    assert_eq!(core.status().status, "stopped");
}

/// Vista simple (ventana compacta): un script guardado arranca y se detiene con el botón
/// de su fila, y la biblioteca queda en `scripts.json`.
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn simple_view_starts_and_stops_a_saved_script() {
    fake_hf_cache();
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        data_dir: dir.path().into(),
    };
    cfg.ensure_data_dir().unwrap();
    let core = Core::new(cfg.clone()).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let script = format!(
        "{} --host 127.0.0.1 --port {} -hf fake/Modelo-7B:Q4_K_M",
        env!("CARGO_BIN_EXE_fake-llama-server"),
        port
    );
    let handle = tokio::runtime::Handle::current();
    let c = core.clone();
    let mut h = Harness::builder()
        .with_size(egui::Vec2::from(
            plane_llama_ui::theme::Tokens::COMPACT_SIZE,
        ))
        .build_ui_state(
            move |ui, app: &mut Option<App>| {
                let app = app.get_or_insert_with(|| {
                    App::headless(c.clone(), handle.clone(), ui.ctx().clone())
                });
                app.frame(ui);
            },
            None,
        );
    let wait = |h: &mut Harness<'_, Option<App>>, what: &dyn Fn(&App) -> bool| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !what(h.state().as_ref().unwrap()) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(30));
            h.run_steps(1);
        }
        assert!(what(h.state().as_ref().unwrap()));
    };
    wait(&mut h, &|a| a.store.library.loaded);
    h.get_by_label("Crear script").click();
    h.run_steps(2);
    {
        let draft = h
            .state_mut()
            .as_mut()
            .unwrap()
            .store
            .library
            .draft
            .as_mut()
            .unwrap();
        draft.name = "Fake".into();
        draft.script = script;
    }
    h.run_steps(1);
    h.get_by_label("Guardar script").click();
    h.run_steps(2);
    assert_eq!(h.state().as_ref().unwrap().store.library.list.len(), 1);
    h.get_by_label("Iniciar").click();
    h.run_steps(2);
    wait(&mut h, &|a| a.store.running());
    let id = h.state().as_ref().unwrap().store.library.list[0].id.clone();
    assert_eq!(
        h.state().as_ref().unwrap().store.library.running_id(true),
        Some(id.as_str())
    );
    h.get_by_label("Detener").click();
    h.run_steps(2);
    wait(&mut h, &|a| !a.store.running() && !a.store.manual_pending);
    let saved = plane_llama_core::scripts::load(&cfg).unwrap().unwrap();
    assert_eq!(saved[0].name, "Fake");
    assert!(saved[0].last_run_at.is_some());
    core.shutdown().await;
}
