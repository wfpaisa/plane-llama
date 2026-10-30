// Interacciones de componentes, persistencia y recepción de datos sin ventana nativa.
use eframe::Storage;
use egui_kittest::{Harness, kittest::Queryable};
use plane_llama_core::*;
use plane_llama_ui::{
    benchmark_panel,
    bridge::{Action, Message},
    history_table, optimizer_modal, status_bar,
    store::{self, KEYS, Store},
};
use std::collections::BTreeMap;

#[test]
fn status_controls_dispatch_without_blocking() {
    let mut h = Harness::new_ui_state(status_bar::show, Store::default());
    h.get_by_label("Iniciar servidor").click();
    h.run();
    assert!(matches!(h.state().actions.last(), Some(Action::Start(_))));
    assert!(h.state().manual_pending);
    h.state_mut().receive(Message::Event(CoreEvent::Status {
        status: StatusResponse {
            status: "running".into(),
            pid: Some(12.),
            ..Default::default()
        },
    }));
    h.run();
    h.get_by_label("Detener servidor").click();
    h.run();
    assert!(matches!(h.state().actions.last(), Some(Action::Stop)));
    assert_eq!(h.state().status.pid, Some(12.));
}
#[test]
fn benchmark_limits_request_and_cancellation() {
    let s = Store {
        script: "llama-server --port 8099".into(),
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000., 650.))
        .build_ui_state(benchmark_panel::show, s);
    h.get_by_label("Limitar").click();
    h.run();
    assert!(!h.state().max_tokens_enabled);
    h.get_by_label("Ejecutar benchmark").click();
    h.run_steps(2);
    let Some(Action::Benchmark(req)) = h.state().actions.last().cloned() else {
        panic!("Falta solicitud")
    };
    assert_eq!(req.max_tokens, None);
    assert_eq!(req.prompt.as_deref(), Some(metrics::DEFAULT_PROMPT));
    assert!(h.state().bench_started.is_some());
    h.get_by_label("Detener").click();
    h.run_steps(2);
    assert!(matches!(h.state().actions.last(), Some(Action::Cancel)));
    h.state_mut().receive(Message::Failed(
        Action::Benchmark(req.clone()),
        "Benchmark cancelado por el usuario.".into(),
    ));
    assert!(h.state().bench_started.is_none());
}
fn result(id: &str, speed: f64) -> BenchmarkResult {
    BenchmarkResult {
        id: id.into(),
        generation_tokens_per_second: Some(speed),
        config: ParsedScript {
            model: Some(format!("org/{id}:Q4_K_M")),
            script: format!("llama-server -hf {id}"),
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn history_sort_selection_compare_chart_and_delete() {
    let s = Store {
        history: vec![result("uno", 10.), result("dos", 20.)],
        columns: vec!["model".into(), "genTps".into()],
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400., 800.))
        .build_ui_state(history_table::show, s);
    h.get_by_label("Generation speed").click();
    h.run();
    assert_eq!(h.state().rows()[0].id, "uno");
    h.get_by_label("Generation speed ↑").click();
    h.run();
    assert_eq!(h.state().rows()[0].id, "dos");
    h.get_by_label("Generation speed ↓").click();
    h.run();
    assert!(h.state().sort.col.is_empty());
    // Los checkboxes de filas conservan la selección a través del filtro.
    let checkboxes = h
        .get_all_by_role(egui::accesskit::Role::CheckBox)
        .collect::<Vec<_>>();
    assert_eq!(checkboxes.len(), 3);
    checkboxes[0].click();
    h.run();
    assert_eq!(h.state().selected.len(), 2);
    h.get_by_label("Comparar").click();
    h.run();
    assert!(h.state().compare);
    h.get_by_label("Gráfico").click();
    h.run();
    assert_eq!(h.state().chart_order, ["uno", "dos"]);
    h.get_by_label("Eliminar seleccionados").click();
    h.run();
    assert!(
        matches!(h.state().confirmation.as_ref().map(|c| &c.action), Some(Action::Delete(ids)) if ids.len() == 2)
    );
    h.state_mut()
        .receive(Message::History(vec![result("dos", 20.)]));
    assert_eq!(h.state().selected.len(), 1);
}
#[derive(Default)]
struct MemoryStorage(BTreeMap<String, String>);
impl eframe::Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.into(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}
#[test]
fn persistence_restores_all_keys_and_tolerates_invalid_data() {
    let mut storage = MemoryStorage::default();
    let mut s = Store {
        script: "llama-server".into(),
        prompt: "Prueba".into(),
        max_tokens: 777.,
        max_tokens_enabled: false,
        columns: vec!["model".into()],
        ..Default::default()
    };
    s.sort_by("generationTokensPerSecond");
    s.save(&mut storage);
    assert!(KEYS.iter().all(|k| storage.0.contains_key(*k)));
    let restored = Store::load(Some(&storage));
    assert_eq!(restored.script, s.script);
    assert_eq!(restored.prompt, "Prueba");
    assert_eq!(restored.max_tokens, 777.);
    assert!(!restored.max_tokens_enabled);
    assert_eq!(restored.columns, ["model"]);
    assert_eq!(restored.sort.col, s.sort.col);
    storage.set_string(KEYS[3], "-1".into());
    storage.set_string(KEYS[5], "null".into());
    storage.set_string(KEYS[2], "invalid".into());
    assert_eq!(Store::load(Some(&storage)).max_tokens, 2048.);
    let mut s = restored;
    s.receive(Message::InitialScript("otro".into()));
    s.receive(Message::InitialPrompt("otro".into()));
    assert_eq!(s.script, "llama-server");
    assert_eq!(s.prompt, "Prueba");
}
#[test]
fn optimizer_applies_only_on_accept_and_bars_are_local() {
    let mut s = Store {
        script: "llama-server --ctx-size 4096".into(),
        ..Default::default()
    };
    optimizer_modal::open(&mut s);
    assert_eq!(s.optimizer.as_ref().unwrap().params.ctx_size, 4096.);
    s.receive(Message::Estimate(
        s.optimizer_serial,
        Box::new(EstimateResponse {
            devices: vec![LlamaDevice {
                id: "CUDA0".into(),
                total_mi_b: 16384.,
                free_mi_b: 12000.,
                ..Default::default()
            }],
            model_meta: ModelMeta {
                params_b: Some(7.),
                bytes_per_param: Some(0.5),
                layers: Some(32.),
                kv_heads: Some(8.),
                head_dim: Some(128.),
                ..Default::default()
            },
            ..Default::default()
        }),
    ));
    let o = s.optimizer.as_mut().unwrap();
    let before = o.breakdown().unwrap().total_mi_b;
    o.params.ctx_size *= 2.;
    assert!(o.breakdown().unwrap().total_mi_b > before);
    assert_eq!(s.script, "llama-server --ctx-size 4096");
}
#[test]
fn logs_deduplicate_cap_and_keep_monotonic_cursor() {
    let mut s = Store::default();
    for i in 1..=5100 {
        s.append(LogEntry {
            seq: i as f64,
            ..Default::default()
        });
    }
    assert_eq!(s.logs.len(), 5000);
    s.append(LogEntry {
        seq: 5100.,
        ..Default::default()
    });
    assert_eq!(s.logs.len(), 5000);
    s.logs.clear();
    s.append(LogEntry {
        seq: 100.,
        ..Default::default()
    });
    assert!(s.logs.is_empty());
    assert_eq!(s.cursor, 5100.);
}

#[test]
fn optimizer_modal_defaults_cancel_apply_and_ignore_old_responses() {
    let script = "llama-server --ctx-size 4096";
    let mut s = Store {
        script: script.into(),
        ..Default::default()
    };
    optimizer_modal::open(&mut s);
    let old_id = s.optimizer_serial;
    optimizer_modal::open(&mut s);
    s.receive(Message::Estimate(old_id, Box::default()));
    assert!(s.optimizer.as_ref().unwrap().loading);
    s.receive(Message::Failed(
        Action::Estimate(
            old_id,
            EstimateRequest {
                script: script.into(),
                params: None,
                priority: None,
            },
        ),
        "viejo".into(),
    ));
    assert!(s.optimizer.as_ref().unwrap().loading);
    s.optimizer.as_mut().unwrap().loading = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440., 1000.))
        .build_ui_state(|ui, s| optimizer_modal::show(&ui.ctx().clone(), s), s);
    h.get_by_label("Restablecer").click();
    h.run();
    assert_eq!(h.state().optimizer.as_ref().unwrap().params.ctx_size, 2048.);
    h.get_by_label("Cancelar").click();
    h.run();
    assert!(h.state().optimizer.is_none());
    assert_eq!(h.state().script, script);
    optimizer_modal::open(h.state_mut());
    h.state_mut().optimizer.as_mut().unwrap().loading = false;
    h.run();
    h.get_by_label("Restablecer").click();
    h.run();
    h.get_by_label("Aplicar al script").click();
    h.run();
    assert!(h.state().optimizer.is_none());
    assert!(h.state().script.contains("--ctx-size 2048"));
}
#[test]
fn history_favorites_and_ten_star_rating_dispatch_nullable_patch() {
    let s = Store {
        history: vec![result("uno", 10.)],
        columns: vec!["fav".into(), "rating".into()],
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400., 800.))
        .build_ui_state(history_table::show, s);
    h.get_by_label("♡").click();
    h.run();
    assert!(matches!(
        h.state().actions.last(),
        Some(Action::Patch(
            _,
            ResultPatch {
                favorite: Some(true),
                ..
            }
        ))
    ));
    h.get_all_by_label("☆").nth(4).unwrap().click();
    h.run();
    assert!(matches!(
        h.state().actions.last(),
        Some(Action::Patch(
            _,
            ResultPatch {
                rating: Some(Some(5.)),
                ..
            }
        ))
    ));
    h.state_mut().history[0].rating = Some(Some(5.));
    h.run();
    h.get_all_by_label("★").nth(4).unwrap().click();
    h.run();
    assert!(matches!(
        h.state().actions.last(),
        Some(Action::Patch(
            _,
            ResultPatch {
                rating: Some(None),
                ..
            }
        ))
    ));
}

#[test]
fn desktop_settings_pick_saved_launch_script() {
    let saved = vec![plane_llama_core::scripts::SavedScript::new(
        "Mi script",
        "llama-server --port 8111",
    )];
    let id = saved[0].id.clone();
    let mut h = Harness::new_ui_state(
        move |ui, settings| plane_llama_ui::settings::controls(ui, settings, &saved),
        plane_llama_core::desktop::AppSettings::default(),
    );
    h.get_by_label("Al cerrar, mantener en la bandeja").click();
    h.run();
    assert!(!h.state().close_to_tray);
    h.get_by_label("Iniciar con la sesión").click();
    h.run();
    assert!(h.state().autostart);
    h.get_by_label("Al iniciar con la sesión, empezar oculta")
        .click();
    h.run();
    assert!(!h.state().autostart_hidden);
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    h.get_by_label("Mi script").click();
    h.run();
    assert_eq!(h.state().launch_script_id.as_deref(), Some(id.as_str()));
    assert!(
        h.state()
            .launch_script
            .as_deref()
            .is_some_and(|s| s.contains("--port 8111"))
    );
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    h.get_by_label("Ninguno").click();
    h.run();
    assert!(h.state().launch_script.is_none());
    assert!(h.state().launch_script_id.is_none());
}

/// Scripts: el ojo de cada fila oculta el script y, en orden manual, arrastrar una fila
/// por su asa la pone antes de otra.
#[test]
fn scripts_rows_hide_and_reorder_by_drag() {
    use plane_llama_ui::{library::ScriptSort, scripts_page, widgets::icons};
    let mut s = Store::default();
    s.library.list = vec![
        scripts::SavedScript::new("Alfa", "llama-server -hf unsloth/A-GGUF:Q8_0"),
        scripts::SavedScript::new("Beta", "llama-server -hf bartowski/B-GGUF:Q4_K_M"),
    ];
    s.library.loaded = true;
    s.library.sort = ScriptSort::Manual;
    s.refresh_library();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000., 500.))
        .build_ui_state(scripts_page::show, s);
    h.run();
    // Arrastra Beta (segunda asa) sobre la fila de Alfa.
    let handles = h
        .get_all_by_label(icons::DOTS_SIX_VERTICAL)
        .collect::<Vec<_>>();
    let from = handles[1].rect().center();
    let to = h.get_by_label("Alfa").rect().center();
    drop(handles);
    let ev = |h: &mut Harness<'_, Store>, e: egui::Event| {
        h.event(e);
        h.run_steps(1);
    };
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    ev(&mut h, egui::Event::PointerMoved(from));
    ev(&mut h, button(from, true));
    ev(
        &mut h,
        egui::Event::PointerMoved(from + egui::vec2(0., -10.)),
    );
    ev(&mut h, egui::Event::PointerMoved(to));
    ev(&mut h, button(to, false));
    h.run();
    let names: Vec<_> = h
        .state()
        .library
        .list
        .iter()
        .map(|x| x.name.clone())
        .collect();
    assert_eq!(names, ["Beta", "Alfa"]);
    // El ojo de la primera fila (Beta) la oculta.
    h.get_all_by_label("Visible: ocultar de la lista")
        .next()
        .unwrap()
        .click();
    h.run();
    assert!(h.state().library.list[0].hidden);
    assert!(!h.state().library.list[1].hidden);
}

/// Las casillas de las filas marcan scripts y «Eliminar» los borra juntos tras confirmar.
#[test]
fn scripts_select_several_and_delete_them_together() {
    use plane_llama_ui::scripts_page;
    let mut s = Store::default();
    s.library.list = vec![
        scripts::SavedScript::new("Alfa", "llama-server -hf unsloth/A-GGUF:Q8_0"),
        scripts::SavedScript::new("Beta", "llama-server -hf bartowski/B-GGUF:Q4_K_M"),
        scripts::SavedScript::new("Gama", "llama-server -hf unsloth/C-GGUF:Q4_K_M"),
    ];
    s.library.loaded = true;
    s.refresh_library();
    let gama = s.library.list[2].id.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000., 600.))
        .build_ui_state(scripts_page::show, s);
    h.run();
    assert!(h.query_by_label("Eliminar seleccionados").is_none());
    h.get_by_label("Seleccionar Alfa").click();
    h.run();
    h.get_by_label("Seleccionar Beta").click();
    h.run();
    assert_eq!(h.state().library.selected.len(), 2);
    h.get_by_label("2 seleccionados");
    h.get_by_label("Eliminar seleccionados").click();
    h.run();
    let confirmation = h.state().confirmation.as_ref().unwrap();
    assert!(confirmation.text.contains("los 2 scripts seleccionados"));
    match &confirmation.action {
        Action::DeleteScript(rest) => {
            assert_eq!(rest.len(), 1);
            assert_eq!(rest[0].id, gama);
        }
        other => panic!("acción inesperada: {other:?}"),
    }
    // «Marcar todos» y «Quitar selección».
    h.get_by_label("Marcar todos").click();
    h.run();
    assert_eq!(h.state().library.selected.len(), 3);
    h.get_by_label("Quitar selección").click();
    h.run();
    assert!(h.state().library.selected.is_empty());
}

/// «Guardar como script» abre el diálogo de nuevo script con el comando del resultado.
#[test]
fn history_row_opens_new_script_with_its_command() {
    let mut r = result("uno", 10.);
    r.config.script = "llama-server -hf unsloth/A-GGUF:Q8_0 -c 4096".into();
    let s = Store {
        history: vec![r],
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400., 800.))
        .build_ui_state(history_table::show, s);
    h.get_by_label("Guardar como script").click();
    h.run();
    let draft = h.state().library.draft.as_ref().unwrap();
    assert!(draft.id.is_none());
    assert_eq!(draft.script, "llama-server -hf unsloth/A-GGUF:Q8_0 -c 4096");
}

/// La vista compacta muestra todos los scripts visibles en una sola lista, favoritos
/// primero, sin cabeceras de grupo.
#[test]
fn compact_list_has_no_groups() {
    use plane_llama_ui::compact;
    let mut s = Store::default();
    s.library.list = vec![
        scripts::SavedScript::new("Alfa", "llama-server -hf unsloth/A-GGUF:Q8_0"),
        scripts::SavedScript::new("Beta", "llama-server -hf bartowski/B-GGUF:Q4_K_M"),
        scripts::SavedScript::new("Gama", "llama-server -hf unsloth/C-GGUF:Q4_K_M"),
    ];
    s.library.list[2].favorite = true;
    s.library.loaded = true;
    s.refresh_library();
    let mut h = Harness::builder()
        .with_size(egui::vec2(900., 600.))
        .build_ui_state(compact::list, s);
    h.run();
    assert!(h.query_by_label("Contraer grupo").is_none());
    let top = |h: &Harness<'_, Store>, name| h.get_by_label(name).rect().top();
    assert!(top(&h, "Gama") < top(&h, "Alfa"));
    assert!(top(&h, "Gama") < top(&h, "Beta"));
}

/// La papelera de un modelo descargado pide confirmación y luego lo borra del disco.
#[test]
fn local_model_is_deleted_after_confirming() {
    use plane_llama_ui::models_page;
    let mut s = Store::default();
    s.hub.local = vec![models::LocalModel {
        repo: "unsloth/Qwen3.5-4B-GGUF".into(),
        quant: "Q8_0".into(),
        files: vec!["/cache/Qwen3.5-4B-Q8_0.gguf".into()],
        size_bytes: 4_000_000_000,
        ..Default::default()
    }];
    s.hub.local_loaded = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000., 600.))
        .build_ui_state(models_page::show, s);
    h.run();
    h.get_by_label("Eliminar del disco").click();
    h.run();
    let confirmation = h.state().confirmation.as_ref().unwrap();
    assert!(confirmation.text.contains("unsloth/Qwen3.5-4B-GGUF:Q8_0"));
    match &confirmation.action {
        Action::DeleteModel(repo, quant) => {
            assert_eq!(
                (repo.as_str(), quant.as_str()),
                ("unsloth/Qwen3.5-4B-GGUF", "Q8_0")
            );
        }
        other => panic!("acción inesperada: {other:?}"),
    }
}

/// La vista ampliada vuelve a la página que tenía abierta, también tras pasar por la
/// compacta (que vuelve a crear la ventana y carga el store de nuevo).
#[test]
fn extended_view_remembers_its_page() {
    let mut storage = MemoryStorage::default();
    let s = Store {
        simple: false,
        tab: store::TAB_HISTORY,
        ..Default::default()
    };
    s.save(&mut storage);
    assert_eq!(Store::load(Some(&storage)).tab, store::TAB_HISTORY);
    // En la compacta se guarda la página de la ampliada, no la de Scripts.
    let mut compact = Store::load(Some(&storage));
    compact.extended_tab = compact.tab;
    compact.simple = true;
    compact.tab = store::TAB_SCRIPTS;
    compact.save(&mut storage);
    assert_eq!(Store::load(Some(&storage)).tab, store::TAB_SCRIPTS);
    let mut back = Store::load(Some(&storage));
    assert_eq!(back.extended_tab, store::TAB_HISTORY);
    back.simple = false;
    back.tab = back.extended_tab;
    back.save(&mut storage);
    assert_eq!(Store::load(Some(&storage)).tab, store::TAB_HISTORY);
    // Un valor inválido cae en Benchmark.
    storage.set_string("plane-llama-extended-tab", "1".into());
    assert_eq!(Store::load(Some(&storage)).tab, store::TAB_BENCH);
}

/// Un script con un modelo `-hf` que no está en la caché pregunta si descargarlo; al
/// aceptar lo baja (con su barra de progreso) y arranca al terminar.
#[test]
fn missing_model_asks_and_downloads_before_starting() {
    use plane_llama_ui::models_page;
    let mut s = Store::default();
    s.hub.local_loaded = true;
    s.script = "llama-server -hf org/M-GGUF:Q8_0 --no-mmproj".into();
    s.manual_pending = true;
    s.launch(Action::Start(s.script.clone()));
    assert!(s.actions.is_empty());
    assert!(!s.manual_pending, "nada en curso mientras se pregunta");
    let mut h = Harness::builder()
        .with_size(egui::vec2(800., 400.))
        .build_ui_state(|ui, s| models_page::fetch_dialog(ui.ctx(), s), s);
    h.run();
    h.get_by_label_contains("org/M-GGUF:Q8_0 no está descargado");
    h.get_by_label("Descargar").click();
    h.run();
    let mut s = h.into_state();
    assert!(s.ask_fetch.is_none());
    assert!(s.manual_pending);
    match s.actions.as_slice() {
        [Action::Fetch(hf, mmproj)] => {
            assert_eq!((hf.as_str(), mmproj), ("org/M-GGUF:Q8_0", &false))
        }
        other => panic!("acciones inesperadas: {other:?}"),
    }
    s.actions.clear();
    s.receive(Message::DownloadProgress(
        "org/M-GGUF:Q8_0".into(),
        250_000_000,
        1_000_000_000,
    ));
    let mut h = Harness::builder()
        .with_size(egui::vec2(800., 200.))
        .build_ui_state(models_page::fetch_progress, s);
    h.step();
    h.get_by_label("DESCARGANDO EL MODELO DEL SCRIPT");
    h.get_by_label_contains("250 MB / 1.0 GB");
    h.state_mut().receive(Message::DownloadEnded(
        "org/M-GGUF:Q8_0".into(),
        Some(Ok("/cache/M-Q8_0.gguf".into())),
    ));
    assert!(matches!(h.state().actions.as_slice(), [Action::Start(_)]));
    assert!(h.state().fetching.is_none());
}

/// Cancelar el modal no lanza nada; «Detener» durante la descarga del benchmark la cancela.
#[test]
fn missing_model_can_be_declined_or_cancelled() {
    let mut s = Store::default();
    s.hub.local_loaded = true;
    assert!(s.benchmark("llama-server -hf org/M-GGUF".into()));
    assert!(s.bench_started.is_none());
    s.decline_fetch();
    assert!(s.actions.is_empty() && s.ask_fetch.is_none());

    assert!(s.benchmark("llama-server -hf org/M-GGUF".into()));
    s.accept_fetch();
    assert!(s.bench_started.is_some());
    assert!(matches!(s.actions.as_slice(), [Action::Fetch(_, true)]));
    s.cancel_benchmark();
    assert!(matches!(s.actions.last(), Some(Action::CancelDownload(k)) if k == "org/M-GGUF"));
    s.receive(Message::DownloadEnded("org/M-GGUF".into(), None));
    assert!(s.bench_started.is_none());
    assert!(!s.actions.iter().any(|a| matches!(a, Action::Benchmark(_))));

    // Con el modelo ya en la caché arranca directamente.
    let mut s = Store::default();
    s.hub.local_loaded = true;
    s.hub.local = vec![models::LocalModel {
        repo: "org/M-GGUF".into(),
        quant: "Q8_0".into(),
        ..Default::default()
    }];
    s.launch(Action::Start("llama-server -hf org/M-GGUF:Q8_0".into()));
    assert!(matches!(s.actions.as_slice(), [Action::Start(_)]));
}

/// El texto de las celdas del historial se selecciona arrastrando y se copia con Ctrl+C.
#[test]
fn history_cells_text_can_be_copied() {
    let s = Store {
        history: vec![result("uno", 10.)],
        columns: vec!["model".into(), "genTps".into()],
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400., 800.))
        .build_ui_state(history_table::show, s);
    // Con el tema de la app, que decide qué textos se pueden seleccionar.
    plane_llama_ui::theme::apply(&h.ctx);
    h.run();
    let rect = h.get_all_by_value("uno").next().unwrap().rect();
    let (from, to) = (
        egui::pos2(rect.left() + 1., rect.center().y),
        egui::pos2(rect.right() + 4., rect.center().y),
    );
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(from));
    h.event(press(from, true));
    h.run();
    h.event(egui::Event::PointerMoved(to));
    h.run();
    h.event(press(to, false));
    h.run();
    h.event(egui::Event::Copy);
    h.step();
    let copied = h
        .output()
        .platform_output
        .commands
        .iter()
        .any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t.contains("uno")));
    assert!(copied, "{:?}", h.output().platform_output.commands);
}

/// Mientras el servidor carga el modelo, la fila del script arrancado ofrece «Detener»
/// y las demás quedan bloqueadas.
#[test]
fn scripts_loading_row_can_be_stopped() {
    use plane_llama_ui::{bridge::Action, scripts_page};
    let mut s = Store::default();
    s.library.list = vec![
        scripts::SavedScript::new("Alfa", "llama-server -hf unsloth/A-GGUF:Q8_0"),
        scripts::SavedScript::new("Beta", "llama-server -hf bartowski/B-GGUF:Q4_K_M"),
    ];
    s.library.loaded = true;
    s.refresh_library();
    s.library.started = Some(s.library.list[0].script.clone());
    s.status.status = "starting".into();
    s.status.started_at = Some(chrono::Utc::now().to_rfc3339());
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000., 500.))
        .build_ui_state(scripts_page::show, s);
    // La carga pide repintar sin parar: unos pasos bastan.
    h.run_steps(2);
    use egui_kittest::kittest::NodeT;
    assert!(h.get_by_label("Iniciar").accesskit_node().is_disabled());
    h.get_by_label("Detener").click();
    h.run_steps(2);
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Stop)));
}
