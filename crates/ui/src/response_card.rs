// Respuesta del último modelo, con saltos de línea, selección y copia.
use crate::store::Store;
use crate::theme::{self, Tokens};
use crate::widgets::{self, icons};
use plane_llama_core::client::format::fmt;

pub fn show(ui: &mut egui::Ui, s: &Store) {
    let (true, Some(r)) = (s.show_response, &s.last_result) else {
        return;
    };
    let t = Tokens::get(ui.ctx());
    widgets::card(ui, |ui| {
        let subtitle = format!("{} tokens generados", fmt(r.generation_token_count, 0));
        widgets::card_header(
            ui,
            icons::CHAT_CIRCLE_TEXT,
            "Respuesta del modelo",
            Some(&subtitle),
            |ui| {
                if widgets::icon_button(ui, icons::COPY, "Copiar respuesta").clicked() {
                    ui.ctx().copy_text(r.response.clone());
                }
            },
        );
        widgets::sunken(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("response")
                .max_height(Tokens::RESPONSE_HEIGHT)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(if r.response.is_empty() {
                                "—"
                            } else {
                                &r.response
                            })
                            .font(theme::font(Tokens::FONT_BODY))
                            .color(t.text),
                        )
                        .selectable(true)
                        .wrap(),
                    );
                });
        });
    });
}
