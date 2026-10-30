// Aplicación egui y composición de las secciones de plane-llama.
#[cfg(target_os = "linux")]
pub mod activate;
pub mod benchmark_panel;
#[cfg(target_os = "linux")]
pub mod blur;
pub mod bridge;
#[cfg(target_os = "linux")]
pub mod center;
pub mod chart_modal;
pub mod compact;
pub mod compare_modal;
pub mod desktop;
pub mod gpu_grid;
pub mod history_table;
pub mod last_result;
pub mod library;
pub mod logs_viewer;
pub mod models_page;
pub mod optimizer_modal;
pub mod response_card;
pub mod script_editor;
pub mod scripts_page;
pub mod settings;
pub mod sidebar;
pub mod status_bar;
pub mod store;
pub mod theme;
pub mod titlebar;
pub mod widgets;
pub mod window;
use bridge::Bridge;
use plane_llama_core::Core;
use store::Store;
use theme::Tokens;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Appearance {
    theme: egui::ThemePreference,
    /// Barra lateral expandida (con etiquetas) o contraída a iconos.
    sidebar: bool,
    accent: theme::Accent,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: egui::ThemePreference::Dark,
            sidebar: true,
            accent: theme::Accent::default(),
        }
    }
}

pub struct App {
    appearance: Appearance,
    pub store: Store,
    bridge: Bridge,
    desktop: std::sync::Arc<desktop::Desktop>,
    cfg: plane_llama_core::Config,
    settings_open: bool,
    settings_draft: plane_llama_core::desktop::AppSettings,
    was_visible: bool,
    /// Frames desde que se creó la ventana; al segundo ya está colocada.
    frames: u32,
    /// La ventana tuvo el foco alguna vez (para `center::placed`).
    had_focus: bool,
    /// Frame desde el que el tamaño de la ventana ya es el de la vista actual.
    settle: u32,
    /// Desenfoque del compositor sobre esta ventana; `None` si no hay Wayland o protocolo.
    #[cfg(target_os = "linux")]
    blur: Option<blur::Blur>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}
impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        core: Core,
        runtime: tokio::runtime::Handle,
        shutdown: tokio::sync::oneshot::Sender<()>,
    ) -> Self {
        let mut app = Self::with_desktop(
            cc,
            core,
            runtime,
            plane_llama_core::Config::default(),
            desktop::Desktop::new(Default::default(), true),
        );
        app.shutdown = Some(shutdown);
        app
    }
    /// App de una ventana de escritorio. La ventana se crea y se destruye cada vez que se
    /// abre o va a la bandeja, así que el cierre del core lo hace quien llama a `run_native`.
    pub fn with_desktop(
        cc: &eframe::CreationContext<'_>,
        core: Core,
        runtime: tokio::runtime::Handle,
        cfg: plane_llama_core::Config,
        desktop: std::sync::Arc<desktop::Desktop>,
    ) -> Self {
        let appearance: Appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        cc.egui_ctx.set_theme(appearance.theme);
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        theme::set_accent(&cc.egui_ctx, appearance.accent);
        let mut store = Store::load(cc.storage);
        store.library.started = desktop
            .started
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        desktop
            .simple
            .store(store.simple, std::sync::atomic::Ordering::Release);
        Self {
            #[cfg(target_os = "linux")]
            blur: blur::Blur::new(cc),
            appearance,
            store,
            bridge: Bridge::with_visibility(
                core,
                runtime,
                cc.egui_ctx.clone(),
                desktop.visible.clone(),
            ),
            shutdown: None,
            desktop,
            cfg,
            settings_open: false,
            settings_draft: Default::default(),
            was_visible: false,
            frames: 0,
            had_focus: false,
            settle: SETTLE_FRAMES,
        }
    }
    pub fn headless(core: Core, runtime: tokio::runtime::Handle, ctx: egui::Context) -> Self {
        let desktop = desktop::Desktop::new(Default::default(), true);
        ctx.set_theme(egui::Theme::Dark);
        theme::install_fonts(&ctx);
        theme::apply(&ctx);
        Self {
            appearance: Appearance::default(),
            store: Store::default(),
            bridge: Bridge::with_visibility(core, runtime, ctx, desktop.visible.clone()),
            shutdown: None,
            desktop,
            cfg: Default::default(),
            settings_open: false,
            settings_draft: Default::default(),
            was_visible: true,
            frames: 0,
            had_focus: false,
            settle: SETTLE_FRAMES,
            #[cfg(target_os = "linux")]
            blur: None,
        }
    }
    fn drain_updates(&mut self, ctx: &egui::Context) {
        // Un aluvión de logs no puede impedir que se procese la entrada del usuario.
        for _ in 0..1024 {
            let Ok(message) = self.bridge.receiver.try_recv() else {
                return;
            };
            self.store.receive(message);
        }
        ctx.request_repaint();
    }
    /// Abre el diálogo de ajustes con una copia editable de los ajustes guardados.
    pub fn open_settings(&mut self) {
        if let Ok(settings) = self.desktop.settings.lock() {
            self.settings_draft = settings.clone();
        }
        self.settings_open = true;
    }
    fn page_title(&self) -> (&'static str, String) {
        let s = &self.store;
        match s.tab {
            store::TAB_SCRIPTS => (
                "Scripts",
                format!(
                    "{} guardados · arranca un modelo con un clic",
                    s.library.list.len()
                ),
            ),
            store::TAB_MODELS => (
                "Modelos",
                format!(
                    "{} descargados · busca y baja GGUF de Hugging Face",
                    s.hub.local.len()
                ),
            ),
            store::TAB_HISTORY => (
                "Historial",
                format!("{} resultados guardados", s.history.len()),
            ),
            store::TAB_LOGS => ("Logs", "Salida en tiempo real de llama-server".into()),
            _ => (
                "Editor",
                "Configura el servidor, lanza una prueba y revisa el resultado".into(),
            ),
        }
    }
    pub fn frame(&mut self, root: &mut egui::Ui) {
        if !self
            .desktop
            .visible
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        let ctx = root.ctx().clone();
        self.drain_updates(&ctx);
        let rect = window::begin(root);
        root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            if self.store.simple {
                self.compact(ui);
            } else {
                self.extended(ui);
            }
        });
        window::end(root, rect);
        self.overlays(&ctx);
    }
    /// Cambia de vista. Con ventana propia la vuelve a crear para que el compositor la
    /// centre con el tamaño guardado de esa vista (y la trae al frente con un token de
    /// activación); con descargas en curso (o sin ventana) la redimensiona.
    fn set_simple(&mut self, ctx: &egui::Context, simple: bool) {
        use std::sync::atomic::Ordering;
        if simple && !self.store.simple {
            self.store.extended_tab = self.store.tab;
        }
        self.store.simple = simple;
        if !simple {
            self.store.tab = self.store.extended_tab;
        }
        if simple && !matches!(self.store.tab, store::TAB_SCRIPTS | store::TAB_MODELS) {
            self.store.tab = store::TAB_SCRIPTS;
        }
        self.desktop.simple.store(simple, Ordering::Release);
        if self.store.hub.downloads.is_empty() && self.desktop.has_window() {
            #[cfg(target_os = "linux")]
            activate::prepare();
            self.desktop.reopen();
            return;
        }
        // La ventana tarda unos frames en tomar el tamaño nuevo: hasta entonces no se
        // guarda como el de esta vista.
        self.settle = self.frames.saturating_add(SETTLE_FRAMES);
        let (size, min) = self.window_size(simple);
        let shadow = 2. * Tokens::WINDOW_SHADOW;
        let (size, min) = (
            [size[0] + shadow, size[1] + shadow],
            [min[0] + shadow, min[1] + shadow],
        );
        let maximized = self
            .desktop
            .window
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .maximized(simple);
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
        if maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
    }
    fn compact(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        if compact::titlebar(root) {
            self.set_simple(&ctx, false);
        }
        let s = &mut self.store;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                left: Tokens::S3 as i8,
                right: Tokens::S3 as i8,
                top: 0,
                bottom: Tokens::S3 as i8,
            }))
            .show(root, |ui| compact::list(ui, s));
    }
    fn extended(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        sidebar::show(root, &mut self.store, &mut self.appearance.sidebar);
        let (title, subtitle) = self.page_title();
        let clicks = titlebar::show(root, title, &subtitle);
        if clicks.settings {
            self.open_settings();
        }
        if let Some(mode) = clicks.simple {
            self.set_simple(&ctx, mode);
        }
        if clicks.theme {
            let next = if ctx.theme() == egui::Theme::Dark {
                egui::ThemePreference::Light
            } else {
                egui::ThemePreference::Dark
            };
            self.appearance.theme = next;
            ctx.set_theme(next);
        }
        let s = &mut self.store;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(Tokens::S6 as i8, Tokens::S5 as i8)),
            )
            .show(root, |ui| {
                // La página Modelos ya lista todas las descargas.
                if s.tab != store::TAB_MODELS {
                    models_page::fetch_progress(ui, s);
                }
                page(ui, s)
            });
    }
    /// Diálogos, avisos y zonas de redimensionado, comunes a las dos vistas.
    fn overlays(&mut self, ctx: &egui::Context) {
        let ctx = ctx.clone();
        let t = Tokens::get(&ctx);
        let s = &mut self.store;
        compare_modal::show(&ctx, s);
        chart_modal::show(&ctx, s);
        optimizer_modal::show(&ctx, s);
        script_editor::catalog(&ctx, s);
        scripts_page::editor_dialog(&ctx, s);
        models_page::fetch_dialog(&ctx, s);
        if let Some(confirmation) = s.confirmation.take() {
            let mut keep = true;
            let dialog = widgets::dialog(
                &ctx,
                "confirmation",
                "Confirmar acción",
                Tokens::DIALOG_SM,
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&confirmation.text)
                                .font(theme::font(Tokens::FONT_BODY))
                                .color(t.text),
                        )
                        .wrap(),
                    );
                    widgets::dialog_footer(ui, |ui| {
                        if widgets::button(ui, widgets::Kind::Primary, None, "Aceptar").clicked() {
                            s.actions.push(confirmation.action.clone());
                            keep = false;
                        }
                        if widgets::button(ui, widgets::Kind::Secondary, None, "Cancelar").clicked()
                        {
                            keep = false;
                        }
                    });
                },
            );
            if keep && !dialog.close {
                s.confirmation = Some(confirmation);
            }
        }
        s.notices
            .retain(|(time, _, _)| time.elapsed().as_secs() < 6);
        widgets::toasts(&ctx, &s.notices);
        if let Some((time, _, _)) = s.notices.front() {
            ctx.request_repaint_after(
                std::time::Duration::from_secs(6).saturating_sub(time.elapsed()),
            );
        }
        if self.settings_open {
            let dialog = widgets::dialog(&ctx, "settings", "Ajustes", Tokens::DIALOG_MD, |ui| {
                widgets::section_label(ui, "Apariencia");
                let mut theme = self.appearance.theme;
                if widgets::segmented(
                    ui,
                    &mut theme,
                    &[
                        (egui::ThemePreference::Dark, "Oscuro"),
                        (egui::ThemePreference::Light, "Claro"),
                        (egui::ThemePreference::System, "Sistema"),
                    ],
                ) {
                    self.appearance.theme = theme;
                    ctx.set_theme(theme);
                }
                ui.add_space(Tokens::S3);
                widgets::section_label(ui, "Color de acento");
                if let Some(accent) = settings::accent_picker(ui, self.appearance.accent) {
                    self.appearance.accent = accent;
                    theme::set_accent(&ctx, accent);
                }
                ui.add_space(Tokens::S3);
                #[cfg(target_os = "linux")]
                let blur = self.blur.as_mut().is_some_and(|b| b.supported());
                #[cfg(not(target_os = "linux"))]
                let blur = false;
                widgets::enabled(ui, blur, |ui| {
                    widgets::toggle(ui, &mut self.settings_draft.blur, "Desenfocar el fondo")
                });
                widgets::dim(
                    ui,
                    if blur {
                        "Fondo translúcido con el desenfoque del escritorio."
                    } else {
                        "El escritorio no ofrece desenfoque (GNOME 51 o posterior)."
                    },
                );
                ui.add_space(Tokens::S3);
                settings::controls(ui, &mut self.settings_draft, &s.library.list);
                widgets::dialog_footer(ui, |ui| {
                    if widgets::button(ui, widgets::Kind::Primary, None, "Guardar").clicked() {
                        let result =
                            std::env::current_exe()
                                .map_err(|e| e.to_string())
                                .and_then(|exe| {
                                    plane_llama_core::desktop::save_settings(
                                        &self.cfg,
                                        desktop::APP_ID,
                                        &exe,
                                        &self.settings_draft,
                                    )
                                    .map_err(|e| e.to_string())
                                });
                        match result {
                            Ok(()) => {
                                if let Ok(mut settings) = self.desktop.settings.lock() {
                                    *settings = self.settings_draft.clone();
                                }
                                self.settings_open = false;
                                s.notice("Ajustes guardados");
                            }
                            Err(e) => s.notice_with_kind(e, store::NoticeKind::Error),
                        }
                    }
                    if widgets::button(ui, widgets::Kind::Secondary, None, "Cancelar").clicked() {
                        self.settings_open = false;
                    }
                });
            });
            if dialog.close {
                self.settings_open = false;
            }
        }
        titlebar::resize_zones(&ctx);
        for action in s.actions.drain(..) {
            self.bridge.dispatch(action);
        }
    }
}

/// Frames tras abrir o redimensionar la ventana antes de guardar su tamaño.
const SETTLE_FRAMES: u32 = 10;
impl App {
    /// Tamaño inicial y mínimo de la ventana de una vista.
    fn window_size(&self, simple: bool) -> ([f32; 2], [f32; 2]) {
        self.desktop
            .window
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(simple)
    }
    /// Guarda el tamaño de la ventana de la vista actual cuando el usuario la cambia, y el
    /// del monitor. Maximizada o a pantalla completa no cuenta, ni la ventana que se cierra
    /// al cambiar de vista (ya tiene la vista nueva pero el tamaño de la anterior).
    fn track_window(&mut self, ctx: &egui::Context) {
        use std::sync::atomic::Ordering;
        if self.frames < self.settle || self.desktop.reopening.load(Ordering::Acquire) {
            return;
        }
        // En Wayland `inner_rect` siempre es `None` (winit no conoce la posición), así que
        // el tamaño sale del área de la ventana.
        let (inner, monitor, maximized, fullscreen) = ctx.input(|i| {
            let v = i.viewport();
            (
                Some(i.viewport_rect()).filter(|r| r.is_positive()),
                v.monitor_size,
                v.maximized == Some(true),
                v.fullscreen == Some(true),
            )
        });
        let mut window = self
            .desktop
            .window
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(m) = monitor {
            window.monitor = Some(m.into());
        }
        if !fullscreen {
            window.set_maximized(self.store.simple, maximized);
        }
        if let (Some(r), false) = (inner, maximized || fullscreen) {
            let shadow = 2. * Tokens::WINDOW_SHADOW;
            let size = [r.width() - shadow, r.height() - shadow];
            if window.set(self.store.simple, size) {
                log::debug!(
                    "tamaño de la vista (compacta: {}): {size:?}",
                    self.store.simple
                );
            }
        }
    }
    /// Aplica o quita el desenfoque según los ajustes y el tamaño actual del marco.
    #[cfg(target_os = "linux")]
    fn apply_blur(&mut self, ctx: &egui::Context) {
        let wanted = self.desktop.settings.lock().is_ok_and(|s| s.blur);
        let Some(blur) = self.blur.as_mut() else {
            return;
        };
        let on = wanted && blur.supported();
        theme::set_glass(ctx, on);
        let zoom = ctx.zoom_factor();
        let rect = window::inner(ctx);
        let rect = egui::Rect::from_min_max(
            (rect.min.to_vec2() * zoom).to_pos2(),
            (rect.max.to_vec2() * zoom).to_pos2(),
        );
        blur.set(on.then(|| (rect, window::radius(ctx) as f32 * zoom)));
    }
}

/// Página Editor: el script arriba y, en su propia sección, el benchmark (indicadores,
/// prueba y detalle del último resultado lado a lado, y la respuesta).
/// Contenido de la página elegida en la vista ampliada.
fn page(ui: &mut egui::Ui, s: &mut Store) {
    match s.tab {
        store::TAB_SCRIPTS => scripts_page::show(ui, s),
        store::TAB_MODELS => models_page::show(ui, s),
        store::TAB_HISTORY => history_table::show(ui, s),
        store::TAB_LOGS => logs_viewer::show(ui, s),
        _ => {
            egui::ScrollArea::vertical()
                .id_salt("home")
                .auto_shrink(false)
                .show(ui, |ui| bench_page(ui, s));
        }
    }
}

fn bench_page(ui: &mut egui::Ui, s: &mut Store) {
    script_editor::show(ui, s);
    ui.add_space(Tokens::S6);
    widgets::section_title(ui, widgets::icons::GAUGE, "Benchmark");
    ui.add_space(Tokens::S3);
    last_result::kpis(ui, s);
    ui.add_space(Tokens::S1);
    let width = ui.available_width();
    if width >= Tokens::TWO_COLUMNS {
        let gap = Tokens::S4;
        let left = (width - gap) * Tokens::SIDE_COLUMN;
        let right = width - gap - left;
        // El espacio entre columnas no se hereda dentro de ellas.
        let spacing = ui.spacing().item_spacing.x;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(
                egui::vec2(left, 0.),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    ui.set_width(left);
                    benchmark_panel::show(ui, s);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(right, 0.),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    ui.set_width(right);
                    last_result::show(ui, s);
                },
            );
        });
    } else {
        benchmark_panel::show(ui, s);
        ui.add_space(Tokens::S4);
        last_result::show(ui, s);
    }
    if s.show_response {
        ui.add_space(Tokens::S4);
    }
    response_card::show(ui, s);
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use std::sync::atomic::Ordering;
        // Cerrar la ventana la destruye siempre; con "mantener en la bandeja" la app sigue
        // viva sin ventana y la bandeja o una segunda instancia la vuelven a crear.
        if ctx.input(|i| i.viewport().close_requested()) {
            #[cfg(target_os = "linux")]
            activate::forget();
            if self.desktop.reopening.load(Ordering::Acquire) {
                // Cambio de vista: la ventana se vuelve a crear enseguida.
            } else if !self.desktop.quitting.load(Ordering::Acquire)
                && self.desktop.settings.lock().is_ok_and(|s| s.close_to_tray)
            {
                self.desktop.visible.store(false, Ordering::Release);
            } else {
                self.desktop.quitting.store(true, Ordering::Release);
            }
            self.desktop.detach();
        }
        let visible = self.desktop.visible.load(Ordering::Acquire);
        if visible && !self.was_visible {
            self.bridge.refresh();
        }
        self.was_visible = visible;
        if !visible {
            return;
        }
        self.drain_updates(ctx);
        for action in self.store.actions.drain(..) {
            self.bridge.dispatch(action);
        }
        #[cfg(target_os = "linux")]
        {
            self.apply_blur(ctx);
            activate::poll();
            // Deja de ser diálogo al perder el foco por primera vez; hacerlo enfocada
            // la mandaba detrás de las demás ventanas.
            let focused = ctx.input(|i| i.viewport().focused == Some(true));
            if self.had_focus && !focused {
                center::placed();
            }
            self.had_focus |= focused;
        }
        self.track_window(ctx);
        self.frames = self.frames.saturating_add(1);
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        window::release_hook(ctx, raw_input);
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // La superficie es transparente: fuera del marco solo se ve la sombra.
        [0.; 4]
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        log::debug!(
            "guardando tamaños de ventana: {:?}",
            self.desktop.window.lock().map(|w| *w)
        );
        self.store.save(storage);
        eframe::set_value(storage, "appearance", &self.appearance);
        store::save_window(
            storage,
            &self
                .desktop
                .window
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        *self
            .desktop
            .started
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = self.store.library.started.clone();
        if self
            .desktop
            .quitting
            .load(std::sync::atomic::Ordering::Acquire)
        {
            self.bridge.cancel_downloads();
        }
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    #[test]
    fn appearance_roundtrip_and_defaults() {
        let appearance = Appearance {
            theme: egui::ThemePreference::System,
            sidebar: false,
            accent: theme::Accent::Custom([1, 2, 3]),
        };
        let json = serde_json::to_string(&appearance).unwrap();
        let restored: Appearance = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.theme, egui::ThemePreference::System);
        assert!(!restored.sidebar);
        assert_eq!(restored.accent, theme::Accent::Custom([1, 2, 3]));
        let defaults: Appearance = serde_json::from_str("{}").unwrap();
        assert_eq!(defaults.theme, egui::ThemePreference::Dark);
        assert!(defaults.sidebar);
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    use std::sync::atomic::Ordering;
    /// Un frame con la ventana de `size` (sombra incluida) y sin posición, como en Wayland.
    fn frame(app: &mut App, ctx: &egui::Context, size: egui::Vec2) {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };
        if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            v.inner_rect = None;
        }
        ctx.begin_pass(input);
        app.track_window(ctx);
        app.frames += 1;
        ctx.end_pass().textures_delta.clear();
    }
    #[tokio::test]
    async fn window_size_is_saved_per_view_without_inner_rect() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = plane_llama_core::Config {
            data_dir: dir.path().into(),
        };
        cfg.ensure_data_dir().unwrap();
        let core = Core::new(cfg).unwrap();
        let ctx = egui::Context::default();
        let mut app = App::headless(core.clone(), tokio::runtime::Handle::current(), ctx.clone());
        let shadow = 2. * Tokens::WINDOW_SHADOW;
        app.store.simple = true;
        for _ in 0..=SETTLE_FRAMES {
            frame(&mut app, &ctx, egui::vec2(700. + shadow, 500. + shadow));
        }
        let sizes = *app.desktop.window.lock().unwrap();
        assert_eq!(sizes.compact, Some([700., 500.]));
        assert_eq!(sizes.extended, None);
        // La ventana que se cierra al cambiar de vista no pisa el tamaño de la otra.
        app.store.simple = false;
        app.desktop.reopening.store(true, Ordering::Release);
        frame(&mut app, &ctx, egui::vec2(700. + shadow, 500. + shadow));
        assert_eq!(app.desktop.window.lock().unwrap().extended, None);
        core.shutdown().await;
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[tokio::test]
    async fn tabs_sidebar_and_theme_work_without_repaint_loop() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = plane_llama_core::Config {
            data_dir: dir.path().into(),
        };
        cfg.ensure_data_dir().unwrap();
        let core = Core::new(cfg).unwrap();
        let runtime = tokio::runtime::Handle::current();
        let mut h = Harness::builder()
            .with_size(egui::vec2(1440., 900.))
            .build_ui_state(
                |ui, app: &mut Option<App>| {
                    let app = app.get_or_insert_with(|| {
                        App::headless(core.clone(), runtime.clone(), ui.ctx().clone())
                    });
                    app.frame(ui);
                },
                None,
            );
        h.get_by_label("Vista ampliada").click();
        h.run_steps(2);
        h.get_by_label("Historial").click();
        h.run_steps(2);
        assert_eq!(h.state().as_ref().unwrap().store.tab, 2);
        h.get_by_label("Logs").click();
        h.run_steps(2);
        assert_eq!(h.state().as_ref().unwrap().store.tab, 4);
        h.get_by_label("Contraer menú").click();
        h.run_steps(2);
        assert!(!h.state().as_ref().unwrap().appearance.sidebar);
        h.get_by_label("Ajustes").click();
        h.run_steps(2);
        h.get_by_label("Claro").click();
        h.run_steps(2);
        assert_eq!(
            h.state().as_ref().unwrap().appearance.theme,
            egui::ThemePreference::Light
        );
        h.get_by_label("Cancelar").click();
        h.run_steps(2);
        assert!(!h.state().as_ref().unwrap().settings_open);
        assert!(h.run() < 10);
        core.shutdown().await;
    }
}
