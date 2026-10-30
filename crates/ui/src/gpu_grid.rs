// Medidores de VRAM por GPU (con el nombre de cada tarjeta) y de RAM del sistema.
use crate::{
    store::Store,
    theme::{self, Tokens},
    widgets,
};
use egui::RichText;
use plane_llama_core::{
    GpuInfo,
    client::format::{fmt, fmt_gb, gpu_label, gpu_title},
};

fn usage(g: &GpuInfo) -> String {
    let util = g
        .gpu_util_pct
        .map(|u| format!(" · {}%", fmt(Some(u), 0)))
        .unwrap_or_default();
    format!(
        "{}/{} GB{util}",
        fmt_gb(g.mem_used_mi_b, 1),
        fmt_gb(g.mem_total_mi_b, 0)
    )
}
fn hint(g: &GpuInfo) -> String {
    format!("{} · {}", g.name.as_deref().unwrap_or("GPU"), gpu_label(g))
}

/// Columna de la barra lateral: nombre de la tarjeta, identificador y consumo.
pub fn show(ui: &mut egui::Ui, s: &Store) {
    let t = Tokens::get(ui.ctx());
    ui.spacing_mut().item_spacing.y = Tokens::S3;
    if s.gpus.is_empty() {
        widgets::dim(ui, "Sin GPU detectada");
    }
    for g in &s.gpus {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = Tokens::S1;
            ui.add(
                egui::Label::new(
                    RichText::new(gpu_title(g))
                        .font(theme::font_medium(Tokens::FONT_SM))
                        .color(t.text),
                )
                .truncate(),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(gpu_label(g))
                        .font(egui::FontId::monospace(Tokens::FONT_XS))
                        .color(t.text_faint),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(usage(g))
                            .font(egui::FontId::monospace(Tokens::FONT_XS))
                            .color(t.text_dim),
                    );
                });
            });
            widgets::meter(
                ui,
                g.mem_used_mi_b.unwrap_or(0.),
                g.mem_total_mi_b.unwrap_or(0.),
                Tokens::BAR,
            );
        })
        .response
        .on_hover_text(hint(g));
    }
    widgets::meter_row(
        ui,
        "RAM",
        &format!(
            "{}/{} GB",
            fmt_gb(s.ram.mem_used_mi_b, 1),
            fmt_gb(s.ram.mem_total_mi_b, 0)
        ),
        s.ram.mem_used_mi_b.unwrap_or(0.),
        s.ram.mem_total_mi_b.unwrap_or(0.),
    );
}

/// VRAM total de todas las tarjetas y RAM total, en MiB.
pub fn capacity(s: &Store) -> (f64, f64) {
    (
        s.gpus.iter().filter_map(|g| g.mem_total_mi_b).sum(),
        s.ram.mem_total_mi_b.unwrap_or(0.),
    )
}
