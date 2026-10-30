// Comparación transpuesta de parámetros y métricas con resaltado del mejor valor.
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind};
use crate::{history_table::cell, store::Store};
use egui::RichText;
use plane_llama_core::{
    BenchmarkResult,
    client::{format::*, store_logic::selected_results},
};
fn number(r: &BenchmarkResult, key: &str) -> Option<f64> {
    match key {
        "promptTps" => r.prompt_tokens_per_second,
        "genTps" => r.generation_tokens_per_second,
        "draftAcc" => r.draft_acceptance,
        "loadTime" => r.load_time_seconds,
        "generationTime" => r.generation_time_ms,
        "latency" => r.request_latency_ms,
        _ => None,
    }
}
const ROWS: &[(&str, &str, bool)] = &[
    ("promptTps", "Prompt t/s", false),
    ("genTps", "Generación t/s", false),
    ("draftAcc", "Aceptación draft", false),
    ("genDr", "Drafts generados", false),
    ("accDr", "Drafts aceptados", false),
    ("loadTime", "Carga (s)", true),
    ("generationTime", "Tiempo de generación", true),
    ("latency", "Latencia (ms)", true),
    ("vram", "VRAM (GB)", false),
    ("ram", "RAM (GB)", false),
    ("ctx", "ctx", false),
    ("batch", "batch/ubatch", false),
    ("cache", "cache K/V", false),
    ("device", "device", false),
    ("backend", "backend", false),
    ("tsplit", "tensor-split", false),
];

pub fn show(ctx: &egui::Context, s: &mut Store) {
    if !s.compare {
        return;
    }
    let t = Tokens::get(ctx);
    let items = selected_results(&s.history, &s.selected);
    let dialog = widgets::dialog(
        ctx,
        "compare-modal",
        "Comparación",
        Tokens::DIALOG_LG,
        |ui| {
            egui::ScrollArea::both()
                .max_height(
                    (ctx.content_rect().height() - Tokens::DIALOG_RESERVED).max(Tokens::DIALOG_MIN),
                )
                .show(ui, |ui| {
                    egui::Grid::new("compare")
                        .min_col_width(Tokens::COMPARE_COLUMN)
                        .spacing([Tokens::S3, Tokens::S2 + 2.])
                        .show(ui, |ui| {
                            ui.allocate_space(egui::vec2(Tokens::COMPARE_LABEL, 0.));
                            for r in &items {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.;
                                    ui.label(
                                        RichText::new(short_model(r.config.model.as_deref()))
                                            .font(theme::font_semibold(Tokens::FONT_SM + 0.5))
                                            .color(t.text),
                                    );
                                    ui.label(
                                        RichText::new(crate::history_table::date(&r.timestamp))
                                            .font(theme::font(Tokens::FONT_XS))
                                            .color(t.text_faint),
                                    );
                                });
                            }
                            ui.end_row();
                            for (key, label, lower) in ROWS {
                                ui.label(
                                    RichText::new(*label)
                                        .font(theme::font(Tokens::FONT_SM + 0.5))
                                        .color(t.text_dim),
                                );
                                let values: Vec<_> =
                                    items.iter().filter_map(|r| number(r, key)).collect();
                                let best = if values.len() >= 2 {
                                    values.into_iter().reduce(if *lower {
                                        f64::min
                                    } else {
                                        f64::max
                                    })
                                } else {
                                    None
                                };
                                for r in &items {
                                    let text = match *key {
                                        "latency" => fmt(r.request_latency_ms, 0),
                                        "promptTps" => fmt(r.prompt_tokens_per_second, 1),
                                        "genTps" => fmt(r.generation_tokens_per_second, 1),
                                        "generationTime" => fmt_ms(r.generation_time_ms),
                                        "backend" => backend_label(r.backend.as_deref()),
                                        _ => cell(r, key),
                                    };
                                    if best.is_some() && number(r, key) == best {
                                        widgets::best(ui, &text);
                                    } else {
                                        ui.label(
                                            RichText::new(text)
                                                .font(egui::FontId::monospace(Tokens::FONT_SM))
                                                .color(t.text),
                                        );
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
            widgets::dialog_footer(ui, |ui| {
                if widgets::button(ui, Kind::Secondary, None, "Cerrar").clicked() {
                    s.compare = false;
                }
                widgets::dim(ui, "En verde, el mejor valor de cada fila.");
            });
        },
    );
    if dialog.close {
        s.compare = false;
    }
}
