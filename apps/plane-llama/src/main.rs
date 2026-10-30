// Ventana nativa (OpenGL o wgpu) y runtime separado con cierre ordenado del core.
mod instance;
use plane_llama_core::{Config, Core, desktop};
use plane_llama_ui::desktop::{APP_ID, APP_NAME, Desktop};

/// Renderer de la ventana según `PLANE_LLAMA_RENDERER`: `glow` (OpenGL, por defecto) o `wgpu` (Vulkan).
fn renderer_from_env() -> eframe::Renderer {
    match std::env::var("PLANE_LLAMA_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    }
}

/// Con `PLANE_LLAMA_BACKEND=x11` abre la ventana por XWayland: winit no implementa el
/// protocolo de tablet de Wayland y en GNOME el lápiz no llega como puntero. No es el
/// default porque por XWayland la ventana gasta ~4× más VRAM y CPU en reposo.
fn force_x11_if_requested() {
    if std::env::var("PLANE_LLAMA_BACKEND").as_deref() == Ok("x11")
        && std::env::var_os("DISPLAY").is_some()
    {
        // SAFETY: se llama al inicio de main, antes de crear cualquier hilo.
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
}

fn main() -> anyhow::Result<()> {
    force_x11_if_requested();
    env_logger::init();
    let socket = std::path::PathBuf::from(
        std::env::var_os("XDG_RUNTIME_DIR")
            .ok_or_else(|| anyhow::anyhow!("XDG_RUNTIME_DIR no está definido"))?,
    )
    .join("plane-llama-egui.sock");
    let Some(instance) = instance::acquire(&socket)? else {
        return Ok(());
    };
    let hidden = std::env::args().any(|a| a == desktop::HIDDEN_FLAG);
    let cfg = Config::default();
    cfg.ensure_data_dir()?;
    let persistence_path = cfg.file("ui.ron");
    let settings = desktop::load_settings(&cfg, APP_ID);
    let desktop = Desktop::new(settings.clone(), !hidden);
    desktop.simple.store(
        plane_llama_ui::store::saved_simple(&persistence_path),
        std::sync::atomic::Ordering::Release,
    );
    *desktop.window.lock().unwrap_or_else(|e| e.into_inner()) =
        plane_llama_ui::store::saved_window(&persistence_path);
    plane_llama_ui::store::forget_eframe_window(&persistence_path);
    let core = Core::new(cfg.clone())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("plane-llama-worker")
        .build()?;
    let handle = runtime.handle().clone();
    if let Some(script) = settings.launch_script {
        let c = core.clone();
        handle.spawn(async move {
            if let Err(e) = c.start(&script).await {
                eprintln!("No se pudo iniciar el servidor: {e}");
            }
        });
    }
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let cleanup = core.clone();
    // Señales del sistema (cierre de sesión, Ctrl+C, terminal cerrada): igual que shutdown.ts,
    // detienen el llama-server gestionado antes de salir para que no quede huérfano en la GPU.
    let (mut term, mut int, mut hup) = {
        use tokio::signal::unix::{SignalKind, signal};
        let _guard = runtime.enter();
        (
            signal(SignalKind::terminate())?,
            signal(SignalKind::interrupt())?,
            signal(SignalKind::hangup())?,
        )
    };
    let thread = std::thread::Builder::new()
        .name("plane-llama-runtime".into())
        .spawn(move || {
            runtime.block_on(async {
                let exit_code = tokio::select! {
                    _ = shutdown_rx => None,
                    _ = term.recv() => Some(143),
                    _ = int.recv() => Some(130),
                    _ = hup.recv() => Some(129),
                };
                cleanup.shutdown().await;
                if let Some(code) = exit_code {
                    std::process::exit(code);
                }
            });
            runtime.shutdown_timeout(std::time::Duration::from_secs(4));
        })?;
    let listener = instance.listener.try_clone()?;
    let d = desktop.clone();
    std::thread::Builder::new()
        .name("plane-llama-instance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        if instance::is_show(&mut stream) {
                            d.show();
                        }
                    }
                    Err(_) => break,
                }
            }
        })?;
    #[cfg(target_os = "linux")]
    plane_llama_ui::center::install();
    plane_llama_ui::desktop::spawn_tray(core.clone(), &handle, desktop.clone());
    // Cada vuelta crea la ventana, y `run_native` vuelve cuando se cierra. Con `--hidden`
    // la primera ventana no se crea hasta que la piden desde la bandeja. Cambiar de vista
    // también la vuelve a crear, con el tamaño guardado de esa vista, para que quede
    // centrada.
    let mut result = Ok(());
    while desktop.wait_for_window() {
        let (size, min, maximized) = {
            let window = desktop.window.lock().unwrap_or_else(|e| e.into_inner());
            let simple = desktop.simple.load(std::sync::atomic::Ordering::Acquire);
            let (size, min) = window.get(simple);
            (size, min, window.maximized(simple))
        };
        log::debug!(
            "abriendo la ventana (compacta: {}) con {size:?}",
            desktop.simple.load(std::sync::atomic::Ordering::Acquire)
        );
        // La sombra se pinta dentro de la superficie, alrededor de la ventana visible.
        let shadow = 2. * plane_llama_ui::theme::Tokens::WINDOW_SHADOW;
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_app_id(APP_ID)
                .with_title(APP_NAME)
                .with_decorations(false)
                .with_transparent(true)
                .with_inner_size([size[0] + shadow, size[1] + shadow])
                .with_min_inner_size([min[0] + shadow, min[1] + shadow])
                // Maximizada vuelve a abrir así; al restaurar toma el tamaño guardado.
                .with_maximized(maximized)
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!(
                        "../../../assets/plane-llama-egui.png"
                    ))
                    .unwrap_or_default(),
                ),
            renderer: renderer_from_env(),
            persistence_path: Some(persistence_path.clone()),
            // El tamaño lo decide la vista, no el de la última ventana.
            persist_window: false,
            ..Default::default()
        };
        let (core, handle, cfg, window_desktop) =
            (core.clone(), handle.clone(), cfg.clone(), desktop.clone());
        result = eframe::run_native(
            APP_NAME,
            options,
            Box::new(move |cc| {
                window_desktop.attach(cc.egui_ctx.clone());
                Ok(Box::new(plane_llama_ui::App::with_desktop(
                    cc,
                    core,
                    handle,
                    cfg,
                    window_desktop,
                )))
            }),
        );
        desktop.detach();
        if result.is_err() {
            break;
        }
    }
    let _ = shutdown_tx.send(());
    thread
        .join()
        .map_err(|_| anyhow::anyhow!("El runtime terminó inesperadamente"))?;
    drop(instance);
    result.map_err(|e| anyhow::anyhow!("No se pudo abrir {APP_NAME}: {e}"))
}
