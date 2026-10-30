// Capturas PNG de cada página con el historial real, para revisar el diseño a ojo.
// No es parte de `cargo test`: necesita GPU. Uso: `just screenshots [carpeta]`.
use egui_kittest::{Harness, kittest::Queryable};
use plane_llama_core::*;
use plane_llama_ui::{
    App,
    bridge::{Action, Message},
    library, optimizer_modal, store, theme,
};

fn history() -> Vec<BenchmarkResult> {
    let path = std::env::var_os("PLANE_LLAMA_HISTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Config::default().file("history.json"));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn estimate() -> EstimateResponse {
    EstimateResponse {
        devices: vec![LlamaDevice {
            id: "CUDA0".into(),
            name: "NVIDIA GeForce RTX 4090".into(),
            total_mi_b: 24564.,
            free_mi_b: 22800.,
            ..Default::default()
        }],
        model_meta: ModelMeta {
            base: "Qwen3-30B-A3B".into(),
            raw: "unsloth/Qwen3-30B-A3B-GGUF:Q4_K_M".into(),
            quant: Some("Q4_K_M".into()),
            params_b: Some(30.5),
            bytes_per_param: Some(0.56),
            layers: Some(48.),
            kv_heads: Some(4.),
            head_dim: Some(128.),
            weights_file_mi_b: Some(17700.),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "necesita GPU; se corre con `just screenshots`"]
async fn render_pages() {
    let out = std::path::PathBuf::from(
        std::env::var("SCREENSHOT_DIR").unwrap_or_else(|_| "target/screenshots".into()),
    );
    std::fs::create_dir_all(&out).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        data_dir: dir.path().into(),
    };
    cfg.ensure_data_dir().unwrap();
    let core = Core::new(cfg).unwrap();
    let runtime = tokio::runtime::Handle::current();
    let history = history();
    let script = std::fs::read_to_string(Config::default().file("script-default.txt"))
        .unwrap_or_else(|_| {
            "llama-server \\\n  -hf unsloth/Qwen3-30B-A3B-GGUF:Q4_K_M \\\n  --ctx-size 32768 \\\n  --n-gpu-layers 99 \\\n  --flash-attn on".into()
        });
    let c = core.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440., 900.))
        .wgpu()
        .build_ui_state(
            move |ui, app: &mut Option<App>| {
                let app = app.get_or_insert_with(|| {
                    App::headless(c.clone(), runtime.clone(), ui.ctx().clone())
                });
                app.frame(ui);
            },
            None,
        );
    h.run_steps(3);
    let seed = |h: &mut Harness<'_, Option<App>>| {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.history = history.clone();
        s.script = script.clone();
        s.last_result = history.first().cloned();
        s.show_response = s.last_result.is_some();
        for i in 1..=60 {
            s.append(LogEntry {
                seq: i as f64,
                t: i as f64 * 137.,
                stream: if i % 9 == 0 { "stderr" } else { "stdout" }.into(),
                msg: format!(
                    "slot update_slots: id  0 | task {i} | prompt processing progress, n_past = {}",
                    i * 512
                ),
                ..Default::default()
            });
        }
    };
    seed(&mut h);
    let shot = |h: &mut Harness<'_, Option<App>>, name: &str| {
        h.run_steps(4);
        h.render()
            .unwrap()
            .save(out.join(format!("{name}.png")))
            .unwrap();
    };
    // Espera a datos que llegan por el puente (caché local, Hugging Face).
    let wait = |h: &mut Harness<'_, Option<App>>, what: &dyn Fn(&store::Store) -> bool| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !what(&h.state().as_ref().unwrap().store) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
            h.run_steps(1);
        }
    };
    // Vista simple: biblioteca sembrada con el historial real, en un DATA_DIR temporal.
    wait(&mut h, &|s| s.library.loaded && s.hub.local_loaded);
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.library.list = plane_llama_core::scripts::seed(&history, Some(&script));
        if let Some(first) = s.library.list.first_mut() {
            first.favorite = true;
        }
        s.refresh_library();
        s.simple = true;
        s.tab = store::TAB_SCRIPTS;
        s.result_banner = s.last_result.is_some();
    }
    // Ventana compacta de la vista simple, a su tamaño real.
    h.set_size(egui::Vec2::from(theme::Tokens::COMPACT_SIZE));
    shot(&mut h, "00-compacta");
    h.ctx.set_theme(egui::Theme::Light);
    shot(&mut h, "00-compacta-claro");
    h.ctx.set_theme(egui::Theme::Dark);
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        let first = s.library.list.first().cloned();
        s.library.draft = first.map(|x| library::Draft {
            id: Some(x.id),
            name: x.name,
            script: x.script,
        });
    }
    shot(&mut h, "00-compacta-editar");
    h.state_mut().as_mut().unwrap().store.library.draft = None;
    h.set_size(egui::vec2(1440., 900.));
    h.state_mut().as_mut().unwrap().store.simple = false;
    shot(&mut h, "00-scripts");
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        let ids: Vec<String> = s
            .library
            .list
            .iter()
            .take(2)
            .map(|x| x.id.clone())
            .collect();
        s.library.selected.extend(ids);
    }
    shot(&mut h, "00-scripts-seleccion");
    h.state_mut()
        .as_mut()
        .unwrap()
        .store
        .library
        .selected
        .clear();
    h.set_size(egui::vec2(960., 760.));
    shot(&mut h, "00-scripts-estrecha");
    h.set_size(egui::vec2(1440., 900.));
    h.state_mut().as_mut().unwrap().store.tab = store::TAB_MODELS;
    shot(&mut h, "00-modelos");
    h.state_mut().as_mut().unwrap().store.hub.view = library::ModelsView::Hub;
    h.run_steps(2);
    wait(&mut h, &|s| {
        s.hub.details.is_some() || s.hub.hub_error.is_some()
    });
    shot(&mut h, "00-hugging-face");
    h.set_size(egui::vec2(960., 760.));
    shot(&mut h, "00-hugging-face-estrecha");
    h.set_size(egui::vec2(1440., 900.));
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.tab = store::TAB_SCRIPTS;
        let first = s.library.list.first().cloned();
        s.library.draft = first.map(|x| library::Draft {
            id: Some(x.id),
            name: x.name,
            script: x.script,
        });
    }
    shot(&mut h, "00-editar-script");
    h.state_mut().as_mut().unwrap().store.library.draft = None;
    theme::set_accent(&h.ctx, theme::Accent::Preset(3));
    h.ctx.set_theme(egui::Theme::Light);
    shot(&mut h, "00-scripts-claro-esmeralda");
    theme::set_accent(&h.ctx, theme::Accent::default());
    h.ctx.set_theme(egui::Theme::Dark);
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.tab = store::TAB_BENCH;
    }
    shot(&mut h, "01-benchmark");
    h.state_mut().as_mut().unwrap().store.tab = store::TAB_HISTORY;
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        for r in history.iter().take(2) {
            s.selected.insert(r.id.clone());
        }
    }
    shot(&mut h, "02-historial");
    // Todas las columnas, para ver los grupos teñidos (escritura, lectura, especulativo).
    let columns = h.state().as_ref().unwrap().store.columns.clone();
    h.state_mut().as_mut().unwrap().store.columns = [
        "model",
        "genTokens",
        "genTps",
        "promptTokens",
        "promptTps",
        "draftAcc",
        "accDr",
        "vram",
        "totalVram",
    ]
    .map(String::from)
    .to_vec();
    shot(&mut h, "02-historial-grupos");
    h.state_mut().as_mut().unwrap().store.columns = columns;
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.compare = true;
    }
    shot(&mut h, "03-comparar");
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.compare = false;
        s.chart_order = s.selected.iter().cloned().collect();
        s.chart = true;
    }
    shot(&mut h, "04-grafico");
    // Con el puntero sobre una barra, sus datos salen en un tooltip completo.
    h.event(egui::Event::PointerMoved(egui::pos2(1020., 520.)));
    shot(&mut h, "04-grafico-tooltip");
    h.event(egui::Event::PointerGone);
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.chart = false;
        optimizer_modal::open(s);
        s.actions.clear();
        let id = s.optimizer_serial;
        s.receive(Message::Estimate(id, Box::new(estimate())));
    }
    shot(&mut h, "05-optimizador");
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.optimizer = None;
        s.tab = store::TAB_LOGS;
    }
    shot(&mut h, "06-logs");
    {
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.tab = store::TAB_BENCH;
        s.flags_open = true;
    }
    shot(&mut h, "07-flags");
    h.state_mut().as_mut().unwrap().store.flags_open = false;
    h.ctx.set_theme(egui::Theme::Light);
    shot(&mut h, "08-benchmark-claro");
    h.state_mut().as_mut().unwrap().store.tab = store::TAB_HISTORY;
    shot(&mut h, "09-historial-claro");
    h.state_mut().as_mut().unwrap().open_settings();
    shot(&mut h, "10-ajustes-claro");
    h.ctx.set_theme(egui::Theme::Dark);
    h.state_mut().as_mut().unwrap().store.tab = store::TAB_BENCH;
    h.state_mut().as_mut().unwrap().store.last_result = None;
    h.state_mut().as_mut().unwrap().store.show_response = false;
    h.run_steps(2);
    h.get_by_label("Cancelar").click();
    h.set_size(egui::vec2(900., 700.));
    h.run_steps(2);
    h.get_by_label("Contraer menú").click();
    shot(&mut h, "11-estrecha-sin-resultados");
    // Script con un modelo que no está en la caché: pregunta y, al aceptar, la barra.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let hf = "unsloth/Qwen3.5-4B-GGUF:Q8_0".to_owned();
    let start = Action::Start(format!("llama-server -hf {hf}"));
    h.state_mut().as_mut().unwrap().store.ask_fetch = Some((hf.clone(), true, start.clone()));
    shot(&mut h, "12-modelo-no-descargado");
    {
        // Como `accept_fetch`, sin pedir la descarga real.
        let s = &mut h.state_mut().as_mut().unwrap().store;
        s.ask_fetch = None;
        s.hub.downloads.insert(
            hf.clone(),
            library::Download {
                repo: "unsloth/Qwen3.5-4B-GGUF".into(),
                quant: "Q8_0".into(),
                done: 1_700_000_000,
                total: 4_600_000_000,
                started: std::time::Instant::now() - std::time::Duration::from_secs(60),
            },
        );
        s.fetching = Some((hf, start));
        s.tab = store::TAB_SCRIPTS;
    }
    shot(&mut h, "13-descargando-modelo");
    core.shutdown().await;
}
