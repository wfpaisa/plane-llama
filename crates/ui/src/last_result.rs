// Indicadores principales y detalle completo del último benchmark.
use crate::store::{NoticeKind, Store};
use crate::theme::Tokens;
use crate::widgets::{self, icons};
use plane_llama_core::client::format::*;

/// Fila de cuatro indicadores grandes; muestran «—» hasta el primer resultado.
pub fn kpis(ui: &mut egui::Ui, s: &Store) {
    let r = s.last_result.as_ref();
    let vram = r.and_then(crate::history_table::total_vram);
    let items = [
        (
            icons::LIGHTNING,
            "Generación",
            fmt(r.and_then(|r| r.generation_tokens_per_second), 1),
            "t/s",
            true,
        ),
        (
            icons::ARROW_FAT_LINES_RIGHT,
            "Procesado del prompt",
            fmt(r.and_then(|r| r.prompt_tokens_per_second), 1),
            "t/s",
            false,
        ),
        (icons::CPU, "VRAM usada", fmt_gb(vram, 2), "GB", false),
        (
            icons::TIMER,
            "Carga del modelo",
            fmt(r.and_then(|r| r.load_time_seconds), 2),
            "s",
            false,
        ),
    ];
    widgets::grid(
        ui,
        &items,
        Tokens::STAT_MIN,
        |ui, (icon, label, value, unit, hl)| {
            widgets::stat(ui, icon, label, value, unit, *hl && r.is_some());
        },
    );
}

pub fn show(ui: &mut egui::Ui, s: &Store) {
    widgets::card(ui, |ui| {
        let Some(r) = &s.last_result else {
            widgets::empty_state(
                ui,
                icons::CHART_LINE_UP,
                "Aún no hay resultados",
                "Ejecuta un benchmark para ver aquí sus métricas.",
            );
            return;
        };
        let subtitle = format!(
            "{} · {}",
            short_model(r.config.model.as_deref()),
            crate::history_table::date(&r.timestamp)
        );
        widgets::card_header(
            ui,
            icons::LIST_BULLETS,
            "Detalle del último resultado",
            Some(&subtitle),
            |_| {},
        );
        for e in &r.errors {
            widgets::banner(ui, NoticeKind::Error, e);
        }
        let items = [
            ("Prompt t/s", fmt(r.prompt_tokens_per_second, 1)),
            ("Generación t/s", fmt(r.generation_tokens_per_second, 1)),
            ("Aceptación draft", fmt(r.draft_acceptance, 3)),
            ("Drafts generados", fmt(r.gen_drafts, 0)),
            ("Drafts aceptados", fmt(r.acc_drafts, 0)),
            ("Tokens del prompt", fmt(r.prompt_token_count, 0)),
            ("Tokens generados", fmt(r.generation_token_count, 0)),
            ("Carga (s)", fmt(r.load_time_seconds, 2)),
            ("Tiempo de generación", fmt_ms(r.generation_time_ms)),
            ("Latencia (ms)", fmt(r.request_latency_ms, 0)),
            ("VRAM", device_vram_line(r, true)),
            ("RAM (GB)", fmt_gb(r.ram_used_mi_b, 2)),
        ];
        widgets::grid(ui, &items, Tokens::STAT_MIN, |ui, (label, value)| {
            widgets::metric(ui, label, value);
        });
    });
}
