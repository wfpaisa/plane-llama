// Gráfico de barras con las once métricas del referente y el orden de la tabla.
use crate::theme::Tokens;
use crate::widgets::{self, Kind};
use crate::{history_table::total_vram, store::Store};
use plane_llama_core::{BenchmarkResult, client::format::*};
pub const METRICS: &[(&str, &str, bool)] = &[
    ("Generation speed", "t/s", false),
    ("Prompt speed", "t/s", false),
    ("Generation time", "s", true),
    ("Prompt processing time", "s", true),
    ("Load time", "s", true),
    ("Request latency", "s", true),
    ("Draft acceptance", "", false),
    ("Total VRAM", "GB", true),
    ("RAM", "GB", true),
    ("Context size", "", false),
    ("Generated tokens", "", false),
];
pub fn value(r: &BenchmarkResult, metric: usize) -> Option<f64> {
    match metric {
        0 => r.generation_tokens_per_second,
        1 => r.prompt_tokens_per_second,
        2 => r.generation_time_ms.map(|n| n / 1000.),
        3 => r.prompt_eval_time_ms.map(|n| n / 1000.),
        4 => r.load_time_seconds,
        5 => r.request_latency_ms.map(|n| n / 1000.),
        6 => r.draft_acceptance,
        7 => total_vram(r).map(|n| n / 1000.),
        8 => r.ram_used_mi_b.map(|n| n / 1000.),
        9 => r.config.ctx_size,
        10 => r.generation_token_count,
        _ => None,
    }
}
/// Ancho de cada barra, en unidades del eje X (una por resultado).
const BAR_WIDTH: f64 = 0.6;

pub fn show(ctx: &egui::Context, s: &mut Store) {
    if !s.chart {
        return;
    }
    let t = Tokens::get(ctx);
    let dialog = widgets::dialog(
        ctx,
        "chart-modal",
        "Gráfico comparativo",
        Tokens::DIALOG_LG,
        |ui| {
            ui.horizontal(|ui| {
                widgets::field_label(ui, "Métrica", None);
                egui::ComboBox::from_id_salt("chart-metric")
                    .width(Tokens::SEARCH_WIDTH)
                    .selected_text(METRICS[s.chart_metric].0)
                    .show_ui(ui, |ui| {
                        for (i, (label, _, _)) in METRICS.iter().enumerate() {
                            ui.selectable_value(&mut s.chart_metric, i, *label);
                        }
                    });
                let (_, _, lower) = METRICS[s.chart_metric];
                widgets::dim(
                    ui,
                    if lower {
                        "Menor es mejor"
                    } else {
                        "Mayor es mejor"
                    },
                );
            });
            ui.add_space(Tokens::S2);
            let items =
                plane_llama_core::client::store_logic::chart_results(&s.history, &s.chart_order);
            if items.is_empty() {
                widgets::empty_state(
                    ui,
                    widgets::icons::CHART_BAR,
                    "Nada que graficar",
                    "Selecciona uno o más resultados en el historial.",
                );
            }
            let values: Vec<_> = items
                .iter()
                .filter_map(|r| value(r, s.chart_metric))
                .collect();
            let best = values.iter().copied().reduce(if METRICS[s.chart_metric].2 {
                f64::min
            } else {
                f64::max
            });
            let best_index = items
                .iter()
                .position(|r| best.is_some() && value(r, s.chart_metric) == best);
            let tips: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, r)| tooltip(r, i))
                .collect();
            let bars = items
                .iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    let n = value(r, s.chart_metric)?;
                    Some(
                        egui_plot::Bar::new(i as f64 + 1., n)
                            .width(BAR_WIDTH)
                            .fill(if Some(i) == best_index {
                                t.ok
                            } else {
                                t.accent
                            })
                            .stroke(egui::Stroke::NONE),
                    )
                })
                .collect();
            let labels: Vec<_> = items
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let parsed = parse_model(r.config.model.as_deref());
                    format!(
                        "{}-{}\n{}",
                        i + 1,
                        model_base(r.config.model.as_deref()).unwrap_or("modelo".into()),
                        parsed.and_then(|p| p.quant).unwrap_or_default()
                    )
                })
                .collect();
            widgets::sunken(ui, |ui| {
                let plot = egui_plot::Plot::new("benchmark-chart")
                    .x_axis_formatter(move |mark, _| {
                        if mark.value.fract().abs() > 0.001 || mark.value < 1. {
                            return String::new();
                        }
                        labels
                            .get(mark.value as usize - 1)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .height(
                        (ctx.content_rect().height() - Tokens::DIALOG_RESERVED * 1.5)
                            .clamp(Tokens::DIALOG_MIN / 1.5, Tokens::CHART_HEIGHT),
                    )
                    .y_axis_label(format!(
                        "{} ({})",
                        METRICS[s.chart_metric].0, METRICS[s.chart_metric].1
                    ))
                    .show_grid([false, true])
                    .allow_scroll(false)
                    .include_y(0.)
                    .show(ui, |plot| {
                        // La etiqueta propia de egui_plot se pinta dentro del gráfico y
                        // queda recortada; los datos van en un tooltip de egui.
                        plot.bar_chart(
                            egui_plot::BarChart::new(METRICS[s.chart_metric].0, bars)
                                .allow_hover(false),
                        );
                    });
                let hovered = plot
                    .response
                    .hover_pos()
                    .map(|pos| plot.transform.value_from_position(pos).x)
                    .filter(|x| (x - x.round()).abs() <= BAR_WIDTH / 2.)
                    .and_then(|x| tips.get((x.round() as usize).checked_sub(1)?));
                if let Some(tip) = hovered {
                    plot.response.on_hover_ui_at_pointer(|ui| {
                        ui.label(tip);
                    });
                }
            });
            widgets::dialog_footer(ui, |ui| {
                if widgets::button(ui, Kind::Secondary, None, "Cerrar").clicked() {
                    s.chart = false;
                }
                widgets::dim(
                    ui,
                    "Pasa el cursor por una barra para ver sus datos · en verde, el mejor valor.",
                );
            });
        },
    );
    if dialog.close {
        s.chart = false;
    }
}

fn tooltip(r: &BenchmarkResult, index: usize) -> String {
    let c = &r.config;
    format!(
        "{}-{}\nPrompt speed: {} t/s\nGen speed: {} t/s\nGeneration time: {} s\nPrompt time: {} s\nLoad time: {} s\nDraft acc: {}\nctx: {} · batch: {}/{}\ncache: {}/{}\ndevice: {} · {}\nVRAM: {}\nRAM: {} GB",
        index + 1,
        short_model(c.model.as_deref()),
        fmt(r.prompt_tokens_per_second, 1),
        fmt(r.generation_tokens_per_second, 1),
        fmt_sec(r.generation_time_ms),
        fmt_sec(r.prompt_eval_time_ms),
        fmt(r.load_time_seconds, 2),
        fmt(r.draft_acceptance, 3),
        fmt(c.ctx_size, 0),
        fmt(c.batch_size, 0),
        fmt(c.ubatch_size, 0),
        c.cache_type_k.as_deref().unwrap_or("—"),
        c.cache_type_v.as_deref().unwrap_or("—"),
        c.device.as_deref().unwrap_or("—"),
        backend_label(r.backend.as_deref()),
        device_vram_line(r, true),
        fmt_gb(r.ram_used_mi_b, 2)
    )
}
