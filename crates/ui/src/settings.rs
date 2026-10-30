// Ajustes de escritorio: segundo plano, arranque con la sesión y script al iniciar
// (uno de los guardados, con espera opcional).
use crate::theme::Tokens;
use crate::theme::{ACCENTS, Accent};
use crate::widgets;
use plane_llama_core::{desktop::AppSettings, scripts::SavedScript};

/// Muestras de los acentos y un selector para uno propio. Devuelve el nuevo si cambió.
pub fn accent_picker(ui: &mut egui::Ui, current: Accent) -> Option<Accent> {
    let t = Tokens::get(ui.ctx());
    let dark = ui.ctx().theme() == egui::Theme::Dark;
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = Tokens::S2;
        for (i, (name, ..)) in ACCENTS.iter().enumerate() {
            let accent = Accent::Preset(i);
            let selected = current == accent;
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::splat(Tokens::SWATCH), egui::Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, selected, *name)
            });
            let painter = ui.painter();
            if selected || response.hovered() {
                painter.circle_stroke(
                    rect.center(),
                    Tokens::SWATCH / 2.,
                    egui::Stroke::new(
                        Tokens::BORDER + 1.,
                        if selected { t.text } else { t.border_strong },
                    ),
                );
            }
            painter.circle_filled(rect.center(), Tokens::SWATCH / 2. - 4., accent.color(dark));
            if response.on_hover_text(*name).clicked() {
                chosen = Some(accent);
            }
        }
        ui.add_space(Tokens::S2);
        let mut rgb = current.rgb(true);
        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb)
            .on_hover_text("Color propio")
            .changed()
        {
            chosen = Some(Accent::Custom(rgb));
        }
        let custom = matches!(current, Accent::Custom(_));
        widgets::dim(
            ui,
            if custom {
                "Personalizado"
            } else {
                ACCENTS[match current {
                    Accent::Preset(i) => i.min(ACCENTS.len() - 1),
                    Accent::Custom(_) => 0,
                }]
                .0
            },
        );
    });
    chosen
}

pub fn controls(ui: &mut egui::Ui, settings: &mut AppSettings, saved: &[SavedScript]) {
    widgets::section_label(ui, "Segundo plano");
    widgets::toggle(
        ui,
        &mut settings.close_to_tray,
        "Al cerrar, mantener en la bandeja",
    );
    widgets::toggle(ui, &mut settings.autostart, "Iniciar con la sesión");
    widgets::enabled(ui, settings.autostart, |ui| {
        widgets::toggle(
            ui,
            &mut settings.autostart_hidden,
            "Al iniciar con la sesión, empezar oculta",
        );
    });
    ui.add_space(Tokens::S3);
    launch_controls(ui, settings, saved);
}

/// Script al iniciar: uno de los guardados y cuánto esperar antes de lanzarlo.
fn launch_controls(ui: &mut egui::Ui, settings: &mut AppSettings, saved: &[SavedScript]) {
    widgets::section_label(ui, "Script al iniciar");
    widgets::dim(
        ui,
        "Se ejecuta automáticamente cada vez que se abre la aplicación.",
    );
    ui.add_space(Tokens::S1);
    let chosen = settings
        .launch_script_id
        .as_deref()
        .and_then(|id| saved.iter().find(|s| s.id == id));
    // Un script pegado en versiones anteriores, o uno guardado que ya se borró.
    let selected_text = match (chosen, &settings.launch_script) {
        (Some(s), _) => s.name.clone(),
        (None, Some(_)) => "Script sin guardar".to_owned(),
        (None, None) => "Ninguno".to_owned(),
    };
    egui::ComboBox::from_id_salt("launch-script")
        .width(ui.available_width())
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(settings.launch_script.is_none(), "Ninguno")
                .clicked()
            {
                settings.launch_script = None;
                settings.launch_script_id = None;
            }
            for script in saved {
                let selected = chosen.is_some_and(|c| c.id == script.id);
                if ui.selectable_label(selected, &script.name).clicked() {
                    settings.launch_script_id = Some(script.id.clone());
                    settings.launch_script = Some(script.script.clone());
                }
            }
        });
    if saved.is_empty() {
        widgets::dim(ui, "No hay scripts guardados: crea uno en Scripts.");
    }
    ui.add_space(Tokens::S2);
    let any = chosen.is_some() || settings.launch_script.is_some();
    widgets::enabled(ui, any, |ui| {
        ui.horizontal(|ui| {
            ui.label("Esperar antes de iniciar");
            ui.add(
                egui::DragValue::new(&mut settings.launch_delay_secs)
                    .range(0..=600)
                    .suffix(" s"),
            );
        });
    });
    widgets::dim(
        ui,
        "Útil al iniciar con la sesión, para dar tiempo a que carguen los drivers de la GPU.",
    );
}
