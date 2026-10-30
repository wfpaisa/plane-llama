// Vista simple: ventana pequeña con la lista de scripts, con las mismas filas que la
// página Scripts. Solo tiene cerrar y pasar a la vista ampliada; el resto vive en esa vista.
use crate::{
    desktop::APP_NAME,
    library::ScriptFilter,
    scripts_page, sidebar,
    store::Store,
    theme::{self, Tokens},
    titlebar,
    widgets::{self, Kind, icons},
};
use egui::{Align, Layout, RichText, Sense, Ui};
use plane_llama_core::scripts::SavedScript;

/// Barra superior: logo y nombre (arrastran la ventana), vista ampliada y cerrar.
/// Devuelve `true` si se pidió la vista ampliada.
pub fn titlebar(root: &mut Ui) -> bool {
    let t = Tokens::get(root.ctx());
    let mut expand = false;
    egui::Panel::top("compact-bar")
        .exact_size(Tokens::COMPACT_BAR)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin {
            left: Tokens::S4 as i8,
            right: Tokens::S2 as i8,
            top: 0,
            bottom: 0,
        }))
        .show(root, |ui| {
            widgets::no_select(ui);
            ui.horizontal_centered(|ui| {
                sidebar::logo(ui, Tokens::ICON_LG + 2.);
                ui.label(
                    RichText::new(APP_NAME)
                        .font(theme::font_semibold(Tokens::FONT_BODY))
                        .color(t.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = Tokens::S1 / 2.;
                    if titlebar::caption(ui, icons::X, "Cerrar", true).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    expand =
                        titlebar::caption(ui, icons::ARROWS_OUT_SIMPLE, "Vista ampliada", false)
                            .clicked();
                    let rest = ui.available_rect_before_wrap();
                    if ui
                        .allocate_rect(rest, Sense::click_and_drag())
                        .drag_started()
                    {
                        crate::window::start_drag(ui.ctx());
                    }
                });
            });
        });
    expand
}

/// Scripts visibles en una sola lista (favoritos primero), sin grupos, buscador ni filtros.
pub fn list(ui: &mut Ui, s: &mut Store) {
    activity(ui, s);
    if !s.library.loaded {
        ui.centered_and_justified(|ui| ui.spinner());
        return;
    }
    // Siempre los visibles y sin búsqueda; el orden es el elegido en la vista ampliada.
    let (search, filter) = (
        std::mem::take(&mut s.library.search),
        std::mem::replace(&mut s.library.filter, ScriptFilter::Visible),
    );
    let rows: Vec<SavedScript> = s.library.sorted().into_iter().cloned().collect();
    (s.library.search, s.library.filter) = (search, filter);
    if rows.is_empty() {
        widgets::empty_state(
            ui,
            icons::ROCKET_LAUNCH,
            "No hay scripts visibles",
            "Crea uno o elige cuáles mostrar en la vista ampliada.",
        );
        ui.vertical_centered(|ui| {
            if widgets::button(ui, Kind::Primary, Some(icons::PLUS), "Crear script").clicked() {
                scripts_page::new_script(s);
            }
        });
        return;
    }
    let running = s.running();
    let running_id = s.library.running_id(running).map(str::to_owned);
    let blocked = running || s.busy() || s.manual_pending;
    let mut action = None;
    egui::ScrollArea::vertical()
        .id_salt("compact-scripts")
        .auto_shrink(false)
        .show(ui, |ui| {
            widgets::card(ui, |ui| {
                for script in &rows {
                    let is_running = running_id.as_deref() == Some(script.id.as_str());
                    if let Some(a) = scripts_page::row(
                        ui,
                        s,
                        script,
                        is_running,
                        blocked,
                        scripts_page::Place::Compact,
                    ) {
                        action = Some(a);
                    }
                }
            });
        });
    if let Some(a) = action {
        scripts_page::apply(s, a);
    }
}

/// Benchmark en curso, con su botón para detenerlo.
fn activity(ui: &mut Ui, s: &mut Store) {
    crate::models_page::fetch_progress(ui, s);
    let Some(started) = s.bench_started else {
        return;
    };
    ui.horizontal(|ui| {
        ui.spinner();
        widgets::dim(
            ui,
            format!("Benchmark en curso · {} s", started.elapsed().as_secs()),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button(ui, Kind::Danger, Some(icons::STOP), "Detener").clicked() {
                s.cancel_benchmark();
            }
        });
    });
    ui.add_space(Tokens::S2);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(200));
}
