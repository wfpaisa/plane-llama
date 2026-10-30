// Coordina la visibilidad y la bandeja del escritorio.
use crate::theme::Tokens;
use ksni::TrayMethods;
use plane_llama_core::{Core, CoreEvent, desktop::AppSettings};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

pub const APP_ID: &str = "plane-llama-egui";
/// Nombre visible de la app: ventana, bandeja, menú y barra lateral.
pub const APP_NAME: &str = "Plane llama";

/// Estado compartido entre la ventana, la bandeja y la instancia única.
///
/// En Wayland winit no puede ocultar una ventana (`Visible(false)` no hace nada y deja
/// una superficie vacía), así que ir a la bandeja cierra la ventana de verdad y "Abrir"
/// la vuelve a crear. Sin ventana tampoco queda contexto OpenGL ocupando VRAM.
pub struct Desktop {
    pub visible: Arc<AtomicBool>,
    pub quitting: AtomicBool,
    pub settings: Mutex<AppSettings>,
    /// Vista simple (ventana compacta); decide el tamaño de la próxima ventana.
    pub simple: AtomicBool,
    /// Tamaño de cada vista; se conserva entre ventanas y en `ui.ron`.
    pub window: Mutex<WindowSizes>,
    /// La ventana se cierra para volver a crearse con otro tamaño, no para ir a la bandeja.
    pub reopening: AtomicBool,
    /// Script con el que se arrancó el servidor desde la app; sobrevive a la ventana
    /// (cambiar de vista la vuelve a crear) para seguir marcando su fila y poder detenerlo.
    pub started: Mutex<Option<String>>,
    /// Contexto de la ventana abierta; `None` mientras la app vive solo en la bandeja.
    ctx: Mutex<Option<egui::Context>>,
    /// Hay una petición de abrir la ventana pendiente.
    pending: Mutex<bool>,
    wake: Condvar,
}
impl Desktop {
    pub fn new(settings: AppSettings, visible: bool) -> Arc<Self> {
        Arc::new(Self {
            visible: Arc::new(AtomicBool::new(visible)),
            quitting: AtomicBool::new(false),
            settings: Mutex::new(settings),
            simple: AtomicBool::new(true),
            window: Mutex::new(WindowSizes::default()),
            reopening: AtomicBool::new(false),
            started: Mutex::new(None),
            ctx: Mutex::new(None),
            pending: Mutex::new(visible),
            wake: Condvar::new(),
        })
    }
    /// Registra el contexto de la ventana recién creada.
    pub fn attach(&self, ctx: egui::Context) {
        *self.ctx.lock().unwrap_or_else(|e| e.into_inner()) = Some(ctx);
    }
    /// La ventana se está cerrando: las peticiones siguientes crean una nueva.
    pub fn detach(&self) {
        self.ctx.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
    /// Muestra la ventana: la enfoca si existe o pide crearla.
    pub fn show(&self) {
        self.visible.store(true, Ordering::Release);
        if let Some(ctx) = self.ctx.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
            return;
        }
        *self.pending.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.wake.notify_all();
    }
    /// Hay una ventana abierta.
    pub fn has_window(&self) -> bool {
        self.ctx.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }
    /// Cierra la ventana y crea otra enseguida, para que el compositor la coloque de
    /// nuevo (centrada) con el tamaño de la vista actual. Sin ventana devuelve `false`.
    pub fn reopen(&self) -> bool {
        let Some(ctx) = self.ctx.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return false;
        };
        self.reopening.store(true, Ordering::Release);
        *self.pending.lock().unwrap_or_else(|e| e.into_inner()) = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.request_repaint();
        true
    }
    /// Sale de la app: cierra la ventana si existe y despierta al hilo principal.
    pub fn quit(&self) {
        self.quitting.store(true, Ordering::Release);
        if let Some(ctx) = self.ctx.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx.request_repaint();
        }
        self.wake.notify_all();
    }
    /// Bloquea hasta que haya que abrir la ventana (`true`) o salir (`false`).
    pub fn wait_for_window(&self) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        while !*pending && !self.quitting.load(Ordering::Acquire) {
            pending = self.wake.wait(pending).unwrap_or_else(|e| e.into_inner());
        }
        if self.quitting.load(Ordering::Acquire) {
            return false;
        }
        *pending = false;
        self.reopening.store(false, Ordering::Release);
        self.visible.store(true, Ordering::Release);
        true
    }
}
/// Tamaño de la ventana de cada vista, sin la sombra. `None` hasta que hay una ventana de
/// esa vista; entonces vale el tamaño por defecto o el que le dio el usuario.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WindowSizes {
    pub compact: Option<[f32; 2]>,
    pub extended: Option<[f32; 2]>,
    /// Último monitor visto, para no abrir la ventana más grande que él.
    pub monitor: Option<[f32; 2]>,
    /// La vista quedó maximizada: su ventana vuelve a abrirse así.
    #[serde(default)]
    pub compact_maximized: bool,
    #[serde(default)]
    pub extended_maximized: bool,
}
impl WindowSizes {
    /// Tamaño inicial y mínimo de la ventana de una vista.
    pub fn get(&self, simple: bool) -> ([f32; 2], [f32; 2]) {
        let (saved, default, min) = if simple {
            (self.compact, Tokens::COMPACT_SIZE, Tokens::COMPACT_MIN)
        } else {
            (self.extended, Tokens::WINDOW_SIZE, Tokens::WINDOW_MIN)
        };
        let mut size = saved.unwrap_or(default);
        if let Some(m) = self.monitor {
            for i in 0..2 {
                size[i] = size[i].min(m[i] - Tokens::MONITOR_MARGIN).max(min[i]);
            }
        }
        (size, min)
    }
    pub fn maximized(&self, simple: bool) -> bool {
        if simple {
            self.compact_maximized
        } else {
            self.extended_maximized
        }
    }
    pub fn set_maximized(&mut self, simple: bool, maximized: bool) {
        if simple {
            self.compact_maximized = maximized;
        } else {
            self.extended_maximized = maximized;
        }
    }
    /// Guarda el tamaño actual de una vista; `true` si cambió.
    pub fn set(&mut self, simple: bool, size: [f32; 2]) -> bool {
        let slot = if simple {
            &mut self.compact
        } else {
            &mut self.extended
        };
        let changed = slot.is_none_or(|s| (s[0] - size[0]).abs() + (s[1] - size[1]).abs() > 1.);
        if changed {
            *slot = Some(size);
        }
        changed
    }
}

/// Lado en píxeles del icono de la bandeja: el glifo claro, sin el fondo de la app, para
/// que se lea en la barra superior como los demás indicadores.
const TRAY_ICON: i32 = 64;
struct Tray {
    desktop: Arc<Desktop>,
    core: Core,
    runtime: tokio::runtime::Handle,
    status: String,
    icon: Vec<u8>,
}
impl ksni::Tray for Tray {
    fn id(&self) -> String {
        APP_ID.into()
    }
    fn title(&self) -> String {
        APP_NAME.into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: TRAY_ICON,
            height: TRAY_ICON,
            data: self.icon.clone(),
        }]
    }
    fn activate(&mut self, _: i32, _: i32) {
        self.desktop.show();
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: format!("Abrir {APP_NAME}"),
                activate: Box::new(|t: &mut Self| t.desktop.show()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!("Servidor: {}", self.status),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Iniciar servidor".into(),
                activate: Box::new(|t: &mut Self| {
                    let settings = t.desktop.settings.lock().ok().map(|s| s.clone());
                    let core = t.core.clone();
                    t.runtime.spawn(async move {
                        let saved = core.scripts().await.unwrap_or_default();
                        let script = settings.and_then(|s| s.resolve_launch_script(&saved));
                        let script = match script {
                            Some(s) => Ok(s),
                            None => core.script_default().await,
                        };
                        if let Ok(s) = script
                            && let Err(e) = core.start(&s).await
                        {
                            eprintln!("No se pudo iniciar el servidor: {e}");
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Detener servidor".into(),
                activate: Box::new(|t: &mut Self| {
                    let core = t.core.clone();
                    t.runtime.spawn(async move {
                        if let Err(e) = core.stop().await {
                            eprintln!("No se pudo detener el servidor: {e}");
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Salir".into(),
                activate: Box::new(|t: &mut Self| t.desktop.quit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}
fn status_label(status: &str) -> String {
    match status {
        "running" => "corriendo",
        "starting" => "arrancando",
        "error" => "error",
        _ => "detenido",
    }
    .into()
}
pub fn spawn_tray(core: Core, runtime: &tokio::runtime::Handle, desktop: Arc<Desktop>) {
    let handle = runtime.clone();
    runtime.spawn(async move {
        let mut events = core.subscribe();
        let icon = match eframe::icon_data::from_png_bytes(include_bytes!(
            "../../../assets/plane-llama-tray.png"
        )) {
            Ok(image) => image
                .rgba
                .chunks_exact(4)
                .flat_map(|p| [p[3], p[0], p[1], p[2]])
                .collect(),
            Err(e) => {
                eprintln!("No se pudo leer el icono: {e}");
                Vec::new()
            }
        };
        let tray = Tray {
            status: status_label(&core.status().status),
            core: core.clone(),
            runtime: handle,
            desktop: desktop.clone(),
            icon,
        };
        let tray = match tray.spawn().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("No se pudo crear la bandeja: {e}");
                if let Ok(mut s) = desktop.settings.lock() {
                    s.close_to_tray = false;
                }
                desktop.show();
                return;
            }
        };
        loop {
            match events.recv().await {
                Ok(CoreEvent::Status { status }) => {
                    tray.update(|t| t.status = status_label(&status.status))
                        .await;
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    tray.update(|t| t.status = status_label(&core.status().status))
                        .await;
                }
                Err(_) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_sizes_default_saved_and_clamped_to_monitor() {
        let mut w = WindowSizes::default();
        assert_eq!(w.get(true).0, Tokens::COMPACT_SIZE);
        assert_eq!(w.get(false).0, Tokens::WINDOW_SIZE);
        // Cada vista guarda su tamaño por separado.
        assert!(w.set(true, [700., 450.]));
        assert!(!w.set(true, [700.4, 450.]));
        assert_eq!(w.get(true).0, [700., 450.]);
        assert_eq!(w.get(false).0, Tokens::WINDOW_SIZE);
        // La ampliada no pasa del monitor menos la barra y la sombra.
        w.monitor = Some([1366., 768.]);
        let m = Tokens::MONITOR_MARGIN;
        assert_eq!(w.get(false).0, [1366. - m, 768. - m]);
        assert_eq!(w.get(true).0, [700., 450.]);
        // Maximizada se recuerda por vista y no toca el tamaño guardado.
        w.set_maximized(false, true);
        assert!(w.maximized(false) && !w.maximized(true));
        assert_eq!(w.get(true).0, [700., 450.]);
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(serde_json::from_str::<WindowSizes>(&json).unwrap(), w);
        // Lo guardado antes de existir el campo se lee sin maximizar.
        let old: WindowSizes =
            serde_json::from_str(r#"{"compact":[700.0,450.0],"extended":null,"monitor":null}"#)
                .unwrap();
        assert!(!old.maximized(false));
    }
}
