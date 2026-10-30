// Logs virtualizados con filtro, número de línea y autoscroll que permite leer hacia arriba.
use crate::theme::Tokens;
use crate::widgets::{self, Kind, icons};
use crate::{bridge::Action, store::Store};
use egui::RichText;

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        widgets::search(ui, &mut s.log_filter, "Filtrar logs", Tokens::SEARCH_WIDTH);
        ui.add_space(Tokens::S2);
        widgets::toggle(ui, &mut s.autoscroll, "Seguir al final");
        widgets::toggle(ui, &mut s.timestamps, "Marca de tiempo");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::button(ui, Kind::Ghost, Some(icons::TRASH), "Limpiar").clicked() {
                s.logs.clear();
                s.actions.push(Action::ClearLogs);
            }
            widgets::dim(ui, format!("{} líneas", s.logs.len()));
        });
    });
    ui.add_space(Tokens::S2);
    let filter = s.log_filter.to_lowercase();
    let rows: Vec<usize> = if filter.is_empty() {
        (0..s.logs.len()).collect()
    } else {
        s.logs
            .iter()
            .enumerate()
            .filter(|(_, e)| e.msg.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect()
    };
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(egui::Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_LG)
        .inner_margin(Tokens::S3)
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            if rows.is_empty() {
                widgets::empty_state(
                    ui,
                    icons::TERMINAL_WINDOW,
                    if s.logs.is_empty() {
                        "Sin logs todavía"
                    } else {
                        "Ninguna línea coincide"
                    },
                    "Inicia el servidor o ejecuta un benchmark para ver su salida.",
                );
                return;
            }
            egui::ScrollArea::both()
                .id_salt("logs")
                .auto_shrink(false)
                .stick_to_bottom(s.autoscroll)
                .show_rows(ui, Tokens::LOG_ROW, rows.len(), |ui, range| {
                    // Cada fila mide exactamente LOG_ROW para que la virtualización cuadre.
                    ui.spacing_mut().item_spacing.y = 0.;
                    ui.spacing_mut().interact_size.y = Tokens::LOG_ROW;
                    for i in range {
                        let e = &s.logs[rows[i]];
                        let color = match e.stream.as_str() {
                            "stderr" => t.error,
                            "command" => t.accent_ink,
                            "system" => t.text_faint,
                            _ => t.text,
                        };
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [Tokens::LOG_GUTTER, Tokens::LOG_ROW],
                                egui::Label::new(
                                    RichText::new(if s.timestamps {
                                        format!("+{:.1}s", e.t / 1000.)
                                    } else {
                                        (rows[i] + 1).to_string()
                                    })
                                    .monospace()
                                    .color(t.text_faint),
                                ),
                            );
                            ui.add(
                                egui::Label::new(RichText::new(&e.msg).monospace().color(color))
                                    .extend(),
                            );
                        });
                    }
                });
        });
}
