// Tarjeta del servidor en la barra lateral: estado, detalle y arranque manual.
use crate::{
    bridge::Action,
    store::Store,
    theme::{self, Tokens},
    widgets::{self, Kind, icons},
};
use egui::RichText;
use plane_llama_core::client::{format::format_script, store_logic};

pub fn status_color(t: &Tokens, status: &str) -> egui::Color32 {
    match status {
        "running" => t.ok,
        "starting" => t.warning,
        "error" => t.error,
        _ => t.text_faint,
    }
}
fn toggle_server(s: &mut Store) {
    if s.running() {
        s.manual_pending = true;
        s.actions.push(Action::Stop);
    } else {
        s.script = format_script(&s.script);
        s.manual_pending = true;
        s.library.started = Some(s.script.clone());
        s.launch(Action::Start(s.script.clone()));
    }
}
fn can_toggle(s: &Store) -> bool {
    !s.manual_pending && (s.running() || !s.busy())
}
fn label(s: &Store) -> &'static str {
    if s.running() {
        "Detener servidor"
    } else {
        "Iniciar servidor"
    }
}

/// Tarjeta completa, para la barra lateral expandida.
pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.surface)
        .stroke(egui::Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_MD + 2)
        .inner_margin(Tokens::S3)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                widgets::status_dot(ui, status_color(&t, &s.status.status));
                ui.label(
                    RichText::new(format!(
                        "Servidor {}",
                        store_logic::status_label(&s.status.status)
                    ))
                    .font(theme::font_medium(Tokens::FONT_SM + 0.5))
                    .color(t.text),
                );
            });
            let meta = store_logic::status_meta(&s.status);
            let meta = meta.trim_end_matches(" · ");
            if !meta.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(meta)
                            .font(egui::FontId::monospace(Tokens::FONT_XS))
                            .color(t.text_dim),
                    )
                    .truncate(),
                );
            }
            ui.add_space(Tokens::S1);
            let running = s.running();
            let clicked = widgets::enabled(ui, can_toggle(s), |ui| {
                widgets::button_wide(
                    ui,
                    if running {
                        Kind::Danger
                    } else {
                        Kind::Secondary
                    },
                    Some(if running { icons::STOP } else { icons::PLAY }),
                    label(s),
                    Tokens::CONTROL,
                )
                .clicked()
            });
            if clicked {
                toggle_server(s);
            }
        });
}
/// Solo icono y punto de estado, para la barra lateral contraída.
pub fn compact(ui: &mut egui::Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    ui.vertical_centered(|ui| {
        widgets::status_dot(ui, status_color(&t, &s.status.status));
        let icon = if s.running() {
            icons::STOP
        } else {
            icons::PLAY
        };
        let clicked = widgets::enabled(ui, can_toggle(s), |ui| {
            widgets::icon_button(ui, icon, label(s))
        })
        .clicked();
        if clicked {
            toggle_server(s);
        }
    });
}
