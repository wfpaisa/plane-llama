// Tarjeta de la prueba: prompt, límite de tokens y ejecución o cancelación del benchmark.
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, icons};
use crate::{bridge::Action, store::Store};
use egui::RichText;

fn progress(ui: &mut egui::Ui, s: &mut Store, started: std::time::Instant) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.accent_soft)
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(Tokens::S3)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                // Pulso a 5 fps: la cuenta del reloj ya repinta a ese ritmo.
                let phase = (started.elapsed().as_millis() / 200 % 5) as f32 / 4.;
                widgets::status_dot(ui, t.accent.gamma_multiply(0.4 + phase * 0.6));
                ui.label(
                    RichText::new(&s.bench_state)
                        .font(theme::font_medium(Tokens::FONT_SM + 0.5))
                        .color(t.accent_ink),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let seconds = started.elapsed().as_secs();
                    ui.label(
                        RichText::new(format!("{}:{:02}", seconds / 60, seconds % 60))
                            .font(egui::FontId::monospace(Tokens::FONT_BODY))
                            .color(t.text),
                    );
                });
            });
        });
    ui.add_space(Tokens::S1);
    if widgets::button_wide(
        ui,
        Kind::Danger,
        Some(icons::STOP),
        "Detener",
        Tokens::CONTROL_LG,
    )
    .clicked()
    {
        s.bench_state = "deteniendo…".into();
        s.cancel_benchmark();
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(200));
}

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    widgets::card(ui, |ui| {
        widgets::card_header(
            ui,
            icons::CHAT_CENTERED_TEXT,
            "Prueba",
            Some("Prompt y límite de tokens"),
            |ui| {
                if widgets::icon_button(ui, icons::ARROW_U_UP_LEFT, "Restablecer").clicked() {
                    s.prompt = plane_llama_core::metrics::DEFAULT_PROMPT.into();
                    s.notice("Prompt restablecido al default");
                }
                if widgets::icon_button(
                    ui,
                    icons::ARROW_COUNTER_CLOCKWISE,
                    "Obtener prompt guardado",
                )
                .clicked()
                {
                    s.confirm(
                        "¿Restablecer el prompt al default guardado?",
                        Action::PromptDefault,
                    );
                }
                if widgets::icon_button(ui, icons::FLOPPY_DISK, "Guardar prompt").clicked() {
                    s.confirm(
                        "¿Guardar el prompt actual como default?",
                        Action::SavePrompt(if s.prompt.trim().is_empty() {
                            plane_llama_core::metrics::DEFAULT_PROMPT.into()
                        } else {
                            s.prompt.clone()
                        }),
                    );
                }
            },
        );
        let edited = widgets::sunken(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut s.prompt)
                    .frame(egui::Frame::new())
                    .hint_text("Escribe el prompt que se enviará al modelo")
                    .desired_rows(Tokens::PROMPT_ROWS)
                    .desired_width(f32::INFINITY),
            )
        });
        if edited.changed() {
            s.prompt_seeded = true;
        }
        ui.add_space(Tokens::S2);
        ui.horizontal(|ui| {
            widgets::toggle(ui, &mut s.max_tokens_enabled, "Limitar");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled(
                    s.max_tokens_enabled,
                    egui::DragValue::new(&mut s.max_tokens)
                        .range(1.0..=f64::MAX)
                        .speed(1.)
                        .suffix(" tokens"),
                );
                widgets::dim(ui, "Máximo");
            });
        });
        ui.add_space(Tokens::S3);
        if let Some(started) = s.bench_started {
            progress(ui, s, started);
        } else {
            let blocked = s.running() || s.busy() || s.manual_pending;
            let clicked = widgets::enabled(ui, !blocked, |ui| {
                widgets::button_wide(
                    ui,
                    Kind::Primary,
                    Some(icons::PLAY),
                    "Ejecutar benchmark",
                    Tokens::CONTROL_LG,
                )
                .clicked()
            });
            if clicked {
                s.run_benchmark();
            }
            if s.running() {
                ui.vertical_centered(|ui| {
                    widgets::dim(ui, "Detén el servidor manual para medir.");
                });
            }
        }
    });
}
