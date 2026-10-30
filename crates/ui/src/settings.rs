// Ajustes de escritorio: segundo plano, arranque con la sesión y script al iniciar.
use crate::theme::Tokens;
use crate::theme::{ACCENTS, Accent};
use crate::widgets::{self, Kind, icons};
use plane_llama_core::desktop::AppSettings;

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

pub fn controls(ui: &mut egui::Ui, settings: &mut AppSettings, script: &str) {
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
    widgets::section_label(ui, "Script al iniciar");
    widgets::dim(
        ui,
        "Se ejecuta automáticamente cada vez que se abre la aplicación.",
    );
    widgets::sunken(ui, |ui| match &settings.launch_script {
        Some(script) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(script.split_whitespace().collect::<Vec<_>>().join(" "))
                        .monospace(),
                )
                .wrap(),
            );
        }
        None => {
            widgets::dim(ui, "Ninguno");
        }
    });
    ui.horizontal(|ui| {
        if widgets::button(
            ui,
            Kind::Secondary,
            Some(icons::ARROW_LINE_DOWN),
            "Usar el script actual del editor",
        )
        .clicked()
        {
            settings.launch_script = Some(script.to_owned());
        }
        let has_script = settings.launch_script.is_some();
        if widgets::enabled(ui, has_script, |ui| {
            widgets::button(ui, Kind::Ghost, Some(icons::TRASH), "Quitar").clicked()
        }) {
            settings.launch_script = None;
        }
    });
}
