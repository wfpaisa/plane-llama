// Diálogo del optimizador: ajustes temporales y estimación local de VRAM, con
// calibración real por modelo. Nada se escribe en el script hasta «Aplicar al script».
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, icons};
use crate::{
    bridge::Action,
    store::{Calibration, NoticeKind, Store, TAB_BENCH},
};
use egui::RichText;
use plane_llama_core::client::format::gpu_short_name;
use plane_llama_core::{
    client::{flag_writer::*, format::*, vram_estimate::*},
    *,
};

pub struct Optimizer {
    pub id: u64,
    pub params: TunedParams,
    pub enabled: TunedFlagsEnabled,
    pub estimate: Option<EstimateResponse>,
    pub loading: bool,
    pub error: Option<String>,
    pub calibrating: bool,
    pub calibration: Option<Calibration>,
    pub calibrating_heuristic: Option<VramBreakdown>,
    pub count_used: bool,
}
impl Optimizer {
    fn from_script(script: &str) -> Self {
        Self {
            id: 0,
            params: parse_params_from_script(script),
            enabled: parse_enabled_from_script(script),
            estimate: None,
            loading: true,
            error: None,
            calibrating: false,
            calibration: None,
            calibrating_heuristic: None,
            count_used: true,
        }
    }
    pub fn breakdown(&self) -> Option<VramBreakdown> {
        self.estimate
            .as_ref()
            .map(|e| build_breakdown(&e.model_meta, &self.params, &e.devices))
    }
    fn reset(&mut self) {
        self.params = TunedParams {
            ctx_size: 2048.,
            ngl: 0.,
            cache_type_k: "f16".into(),
            cache_type_v: "f16".into(),
            batch_size: 2048.,
            ubatch_size: 512.,
            flash_attn: true,
            cache_ram: 8192.,
            ..Default::default()
        };
        for key in ["cacheTypeKDraft", "cacheTypeVDraft"] {
            self.params
                .extra
                .insert(key.into(), serde_json::json!("f16"));
        }
        self.enabled.clear();
        for key in [
            "ctxSize",
            "ngl",
            "batchSize",
            "ubatchSize",
            "cacheRam",
            "cacheTypeK",
            "cacheTypeV",
        ] {
            self.enabled.insert(key.into(), true);
        }
    }
}
pub fn open(s: &mut Store) {
    if s.dryfit_running {
        return;
    }
    s.optimizer_serial += 1;
    let mut o = Optimizer::from_script(&s.script);
    o.id = s.optimizer_serial;
    s.actions.push(Action::Estimate(
        o.id,
        EstimateRequest {
            script: s.script.clone(),
            params: serde_json::to_value(&o.params).ok(),
            priority: Some("ctx".into()),
        },
    ));
    s.optimizer = Some(o);
}

/// Fila de parámetro: interruptor (escribir o no el flag), flag, para qué sirve y, si
/// está activo, su control. Apagado, llama-server usa su valor por defecto.
fn param_row(
    ui: &mut egui::Ui,
    enabled: &mut TunedFlagsEnabled,
    key: &str,
    label: &str,
    flag: &str,
    hint: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    let on = enabled.entry(key.into()).or_default();
    ui.horizontal(|ui| {
        widgets::toggle(ui, on, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            flag_label(ui, flag);
        });
    });
    let on = *on;
    indented(ui, |ui| {
        hint_label(ui, hint);
        if on {
            control(ui);
        }
    });
    ui.add_space(Tokens::S3);
}
/// Interruptor de un valor sí/no, con su flag y explicación como las demás filas.
fn switch_row(ui: &mut egui::Ui, value: &mut bool, label: &str, flag: &str, hint: &str) {
    ui.horizontal(|ui| {
        widgets::toggle(ui, value, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            flag_label(ui, flag);
        });
    });
    indented(ui, |ui| hint_label(ui, hint));
    ui.add_space(Tokens::S3);
}
fn flag_label(ui: &mut egui::Ui, flag: &str) {
    ui.label(
        RichText::new(flag)
            .font(egui::FontId::monospace(Tokens::FONT_XS))
            .color(Tokens::get(ui.ctx()).text_faint),
    );
}
fn hint_label(ui: &mut egui::Ui, hint: &str) {
    ui.label(
        RichText::new(hint)
            .font(theme::font(Tokens::FONT_SM))
            .color(Tokens::get(ui.ctx()).text_dim),
    );
}
/// Alineado con el texto del interruptor, no con su pista.
fn indented<R>(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_space(Tokens::TOGGLE[0] + Tokens::S3);
        ui.vertical(body).inner
    })
    .inner
}
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut egui::Ui,
    enabled: &mut TunedFlagsEnabled,
    key: &str,
    label: &str,
    flag: &str,
    hint: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
) {
    param_row(ui, enabled, key, label, flag, hint, |ui| {
        slider_control(ui, value, range, step)
    });
}
fn slider_control(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
) {
    let t = Tokens::get(ui.ctx());
    ui.scope(|ui| {
        ui.visuals_mut().selection.bg_fill = t.accent;
        ui.visuals_mut().widgets.inactive.bg_fill = t.surface_alt;
        ui.spacing_mut().slider_width =
            ui.available_width() - Tokens::VALUE_WIDTH - ui.spacing().item_spacing.x;
        ui.add(
            egui::Slider::new(value, range)
                .step_by(step)
                .clamping(egui::SliderClamping::Edits),
        );
    });
}
fn kv(
    ui: &mut egui::Ui,
    enabled: &mut TunedFlagsEnabled,
    key: &str,
    label: &str,
    flag: &str,
    hint: &str,
    value: &mut String,
) {
    param_row(ui, enabled, key, label, flag, hint, |ui| {
        egui::ComboBox::from_id_salt(key)
            .width(ui.available_width())
            .selected_text(value.to_uppercase())
            .show_ui(ui, |ui| {
                for t in KV_TYPES {
                    ui.selectable_value(value, (*t).into(), t.to_uppercase());
                }
            });
    });
}
fn group(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    subtitle: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    widgets::card(ui, |ui| {
        widgets::card_header(ui, icon, title, Some(subtitle), |_| {});
        ui.add_space(Tokens::S2);
        body(ui);
    });
    ui.add_space(Tokens::S3);
}

/// Modelo analizado, cómo se usa el optimizador y avisos de la estimación.
fn model_header(ui: &mut egui::Ui, o: &Optimizer, reload: &mut bool) {
    let t = Tokens::get(ui.ctx());
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, icons::CUBE, t.accent_soft, t.accent_ink);
            // Ancho fijo: si no, las fichas se apilan una por línea.
            let width = (ui.available_width() - Tokens::RELOAD_WIDTH).max(Tokens::DIALOG_MIN / 2.);
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = Tokens::S1;
                    match &o.estimate {
                        Some(e) => {
                            let m = &e.model_meta;
                            ui.label(
                                RichText::new(&m.base)
                                    .font(theme::font_semibold(Tokens::FONT_MD))
                                    .color(t.text),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = Tokens::S1 + 2.;
                                let chips = [
                                    m.quant.clone(),
                                    m.layers.map(|_| format!("{} capas", fmt(m.layers, 0))),
                                    m.params_b
                                        .map(|_| format!("{}B params", fmt(m.params_b, 1))),
                                    m.weights_file_mi_b.map(|_| {
                                        format!("pesos {} GB", fmt_gb(m.weights_file_mi_b, 2))
                                    }),
                                    m.mmproj_size_mi_b.map(|_| {
                                        format!("visión {} GB", fmt_gb(m.mmproj_size_mi_b, 2))
                                    }),
                                ];
                                for chip in chips.iter().flatten() {
                                    widgets::chip(ui, chip, t.surface_alt, t.text_dim);
                                }
                            });
                        }
                        None => {
                            ui.label(
                                RichText::new("Analizando el modelo…")
                                    .font(theme::font_semibold(Tokens::FONT_MD))
                                    .color(t.text),
                            );
                            widgets::dim(ui, "Detectando dispositivos y leyendo el header GGUF.");
                        }
                    }
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clicked = widgets::enabled(ui, !o.loading && !o.calibrating, |ui| {
                    widgets::button(
                        ui,
                        Kind::Secondary,
                        Some(icons::ARROWS_CLOCKWISE),
                        "Recargar dispositivos",
                    )
                    .clicked()
                });
                if clicked {
                    *reload = true;
                }
            });
        });
        ui.add_space(Tokens::S3);
        hint_label(
            ui,
            "Activa los parámetros que quieras fijar en el script y mira a la derecha si cabe \
             en la VRAM. Los apagados se quedan con el valor por defecto de llama-server. \
             Nada cambia en el script hasta «Aplicar al script».",
        );
        if let Some(e) = &o.estimate
            && e.devices.is_empty()
        {
            ui.add_space(Tokens::S2);
            widgets::banner(
                ui,
                NoticeKind::Warning,
                "No se detectaron dispositivos. Revisa el binario en el script.",
            );
        }
        if let Some(error) = &o.error {
            ui.add_space(Tokens::S2);
            widgets::banner(ui, NoticeKind::Error, error);
        }
    });
}

fn params(ui: &mut egui::Ui, o: &mut Optimizer, ram_total: f64) {
    let calibrating = o.calibrating;
    widgets::enabled(ui, !calibrating, |ui| param_groups(ui, o, ram_total));
}
fn param_groups(ui: &mut egui::Ui, o: &mut Optimizer, ram_total: f64) {
    let moe = o
        .estimate
        .as_ref()
        .is_some_and(|e| is_model_moe(Some(&e.model_meta.base)));
    group(
        ui,
        icons::STACK,
        "Contexto y capas",
        "Lo que más cambia el uso de VRAM",
        |ui| {
            slider(
                ui,
                &mut o.enabled,
                "ctxSize",
                "Contexto",
                "--ctx-size",
                "Tokens que el modelo recuerda a la vez. Después de los pesos, es lo que más VRAM usa.",
                &mut o.params.ctx_size,
                512.0..=262144.0,
                2048.,
            );
            slider(
                ui,
                &mut o.enabled,
                "ngl",
                "Capas en GPU",
                "--n-gpu-layers",
                "Capas del modelo en la GPU (999 = todas). Menos capas liberan VRAM, pero va más lento.",
                &mut o.params.ngl,
                0.0..=999.0,
                1.,
            );
            if moe {
                slider(
                    ui,
                    &mut o.enabled,
                    "nCpuMoe",
                    "Expertos en CPU",
                    "--n-cpu-moe",
                    "Capas cuyos expertos se quedan en la CPU. En modelos MoE ahorra mucha VRAM.",
                    &mut o.params.n_cpu_moe,
                    0.0..=64.0,
                    1.,
                );
            }
            slider(
                ui,
                &mut o.enabled,
                "cacheReuse",
                "Reutilizar caché",
                "--cache-reuse",
                "Trozo mínimo (en tokens) para reaprovechar la caché entre peticiones parecidas. 0 = no.",
                &mut o.params.cache_reuse,
                0.0..=8192.0,
                64.,
            );
        },
    );
    group(
        ui,
        icons::DATABASE,
        "Caché KV",
        "Memoria del contexto; cuantizarla la reduce",
        |ui| {
            kv(
                ui,
                &mut o.enabled,
                "cacheTypeK",
                "Tipo K",
                "--cache-type-k",
                "Precisión de las claves. Q8_0 usa la mitad que F16 casi sin perder calidad.",
                &mut o.params.cache_type_k,
            );
            kv(
                ui,
                &mut o.enabled,
                "cacheTypeV",
                "Tipo V",
                "--cache-type-v",
                "Precisión de los valores. Cuantizada necesita flash attention.",
                &mut o.params.cache_type_v,
            );
            for (key, label, flag, hint) in [
                (
                    "cacheTypeKDraft",
                    "Tipo K draft",
                    "--cache-type-k-draft",
                    "Lo mismo para el modelo draft de la decodificación especulativa.",
                ),
                (
                    "cacheTypeVDraft",
                    "Tipo V draft",
                    "--cache-type-v-draft",
                    "Lo mismo para el modelo draft de la decodificación especulativa.",
                ),
            ] {
                let mut value = o
                    .params
                    .extra
                    .get(key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("q8_0")
                    .to_string();
                kv(ui, &mut o.enabled, key, label, flag, hint, &mut value);
                o.params.extra.insert(key.into(), serde_json::json!(value));
            }
            switch_row(
                ui,
                &mut o.params.flash_attn,
                "Flash attention",
                "--flash-attn",
                "Atención más rápida y con menos memoria. Recomendado.",
            );
        },
    );
    group(
        ui,
        icons::SQUARES_FOUR,
        "Batch",
        "Velocidad al procesar el prompt; más alto usa algo más de VRAM",
        |ui| {
            slider(
                ui,
                &mut o.enabled,
                "batchSize",
                "Batch",
                "--batch-size",
                "Tokens del prompt por paso lógico.",
                &mut o.params.batch_size,
                64.0..=2048.0,
                64.,
            );
            slider(
                ui,
                &mut o.enabled,
                "ubatchSize",
                "Micro-batch",
                "--ubatch-size",
                "Tokens por paso real en la GPU; no debe superar al batch.",
                &mut o.params.ubatch_size,
                32.0..=1024.0,
                32.,
            );
        },
    );
    group(
        ui,
        icons::MEMORY,
        "Memoria y especulativo",
        "Caché en RAM, decodificación especulativa y visión",
        |ui| {
            slider(
                ui,
                &mut o.enabled,
                "cacheRam",
                "Caché en RAM",
                "--cache-ram",
                "MiB de RAM para guardar prompts ya procesados y reutilizarlos. 0 = no.",
                &mut o.params.cache_ram,
                0.0..=ram_total,
                1024.,
            );
            slider(
                ui,
                &mut o.enabled,
                "specDraftMax",
                "Tokens draft",
                "--spec-draft-n-max",
                "Tokens que propone el modelo draft en cada paso.",
                &mut o.params.spec_draft_max,
                0.0..=8.0,
                1.,
            );
            let mut vision = !o.params.no_mmproj;
            ui.horizontal(|ui| {
                if widgets::toggle(ui, &mut vision, "Cargar visión").changed() {
                    o.params.no_mmproj = !vision;
                }
            });
            indented(ui, |ui| {
                hint_label(
                    ui,
                    "Carga el proyector de imágenes (mmproj). Apagado añade --no-mmproj y ahorra su VRAM.",
                )
            });
        },
    );
    if let Some(e) = &o.estimate {
        let devices: Vec<_> = e.devices.clone();
        group(
            ui,
            icons::CPU,
            "Dispositivos",
            "Qué GPUs usar y cómo repartir el modelo",
            |ui| {
                switch_row(
                    ui,
                    o.enabled.entry("device".into()).or_default(),
                    "Fijar dispositivos",
                    "--device",
                    "Usa solo las GPUs marcadas. Apagado, llama-server usa todas.",
                );
                indented(ui, |ui| {
                    for d in &devices {
                        let mut selected = o.params.device.contains(&d.id);
                        if ui
                            .checkbox(
                                &mut selected,
                                format!(
                                    "{} · {} ({} GB, {} libres)",
                                    d.id,
                                    gpu_short_name(&d.name),
                                    fmt_gb(Some(d.total_mi_b), 1),
                                    fmt_gb(Some(d.free_mi_b), 1)
                                ),
                            )
                            .changed()
                        {
                            if selected {
                                o.params.device.push(d.id.clone());
                            } else {
                                o.params.device.retain(|id| id != &d.id);
                            }
                            o.params.tensor_split = None;
                        }
                    }
                });
                ui.add_space(Tokens::S3);
                let chosen = selected_devices(&devices, &o.params.device);
                let mut auto = o.params.tensor_split.is_none();
                if widgets::toggle(ui, &mut auto, "Reparto automático").changed() {
                    o.params.tensor_split = if auto {
                        None
                    } else {
                        Some(vec![1.; chosen.len()])
                    };
                }
                indented(ui, |ui| {
                    hint_label(
                        ui,
                        "Apagado puedes fijar qué proporción del modelo va a cada GPU (--tensor-split).",
                    );
                    if let Some(split) = &mut o.params.tensor_split {
                        split.resize(chosen.len(), 1.);
                        for (d, value) in chosen.iter().zip(split.iter_mut()) {
                            widgets::field_label(ui, &d.id, Some(&gpu_short_name(&d.name)));
                            slider_control(ui, value, 0.0..=10.0, 0.1);
                        }
                        if split.iter().all(|v| *v == 0.) {
                            o.params.tensor_split = None;
                        }
                    }
                });
            },
        );
    }
}

/// Panel de consumo: total, barra apilada por concepto y medidores por dispositivo.
fn usage(ui: &mut egui::Ui, s: &mut Store, o: &mut Optimizer) {
    let t = Tokens::get(ui.ctx());
    widgets::card(ui, |ui| {
        let title = if o.calibration.is_some() {
            "Consumo medido"
        } else {
            "Consumo estimado"
        };
        widgets::card_header(
            ui,
            icons::CHART_PIE_SLICE,
            title,
            Some("Se recalcula al mover cualquier control"),
            |_| {},
        );
        let (Some(e), Some(h)) = (&o.estimate, o.breakdown()) else {
            widgets::dim(ui, "Esperando la estimación…");
            return;
        };
        let devices = selected_devices(&e.devices, &o.params.device);
        let mut total = 0.;
        let mut capacity = 0.;
        let mut base_total = 0.;
        let mut rows = vec![];
        for (i, d) in devices.iter().enumerate() {
            let heuristic = h.per_device_mi_b.get(i).copied().unwrap_or(0.);
            let used = o
                .calibration
                .as_ref()
                .and_then(|c| {
                    c.measured
                        .per_device
                        .iter()
                        .find(|v| v.device.id == d.id)
                        .and_then(|v| v.used_mi_b)
                        .map(|m| {
                            (m + heuristic
                                - c.heuristic.per_device_mi_b.get(i).copied().unwrap_or(0.))
                            .max(0.)
                        })
                })
                .unwrap_or(heuristic);
            let base = if o.count_used {
                (d.total_mi_b - d.free_mi_b).max(0.)
            } else {
                0.
            };
            total += used + base;
            base_total += base;
            capacity += d.total_mi_b;
            rows.push((
                format!("{} · {}", d.id, gpu_short_name(&d.name)),
                used + base,
                d.total_mi_b,
            ));
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Tokens::S1 + 2.;
            ui.label(
                RichText::new(fmt_gb(Some(total), 2))
                    .font(theme::font_semibold(Tokens::FONT_KPI))
                    .color(if total > capacity && capacity > 0. {
                        t.error
                    } else {
                        t.text
                    }),
            );
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
                ui.label(
                    RichText::new(format!("/ {} GB", fmt_gb(Some(capacity), 1)))
                        .font(theme::font_medium(Tokens::FONT_BODY))
                        .color(t.text_faint),
                );
            });
        });
        ui.add_space(Tokens::S1);
        let model = (total - base_total).max(0.);
        let scale = if h.total_mi_b > 0. {
            model / h.total_mi_b
        } else {
            1.
        };
        let segments = [
            (h.weights_mi_b * scale, t.series[0]),
            (h.kv_mi_b * scale, t.series[1]),
            (h.overhead_mi_b * scale, t.series[2]),
            (base_total, t.series[3]),
        ];
        widgets::stacked_bar(ui, &segments, capacity);
        ui.add_space(Tokens::S2);
        // Veredicto primero: lo que hay que mirar al mover un control.
        if capacity > 0. {
            if total > capacity {
                widgets::banner(
                    ui,
                    NoticeKind::Error,
                    &format!(
                        "No cabe: faltan {} GB. Baja el contexto, cuantiza la caché KV o deja \
                         capas o expertos en la CPU.",
                        fmt_gb(Some(total - capacity), 2)
                    ),
                );
            } else {
                widgets::banner(
                    ui,
                    NoticeKind::Success,
                    &format!(
                        "Cabe: quedan {} GB libres.",
                        fmt_gb(Some(capacity - total), 2)
                    ),
                );
            }
            ui.add_space(Tokens::S2);
        }
        for ((value, color), label) in
            segments
                .iter()
                .zip(["Pesos", "KV cache", "Overhead", "VRAM ya en uso"])
        {
            widgets::legend(
                ui,
                *color,
                label,
                &format!("{} GB", fmt_gb(Some(*value), 2)),
            );
        }
        ui.add_space(Tokens::S3);
        widgets::section_label(ui, "Por dispositivo");
        for (label, used, cap) in rows {
            widgets::meter_row(
                ui,
                &label,
                &format!("{} / {} GB", fmt_gb(Some(used), 2), fmt_gb(Some(cap), 1)),
                used,
                cap,
            );
        }
        ui.add_space(Tokens::S2);
        widgets::toggle(ui, &mut o.count_used, "Contar la VRAM ya en uso");
        indented(ui, |ui| {
            hint_label(ui, "Suma lo que ya ocupan otros programas en las GPUs.")
        });
        if let Some(c) = &o.calibration {
            widgets::banner(
                ui,
                NoticeKind::Success,
                &format!(
                    "Medido: {} GB · corrección sobre la heurística: {} GB",
                    fmt_gb(c.measured.total_mi_b, 2),
                    fmt_gb(c.measured.total_mi_b.map(|m| m - c.heuristic.total_mi_b), 2)
                ),
            );
        }
    });
    ui.add_space(Tokens::S3);
    widgets::card(ui, |ui| {
        widgets::card_header(
            ui,
            icons::RULER,
            "Calibrar",
            Some("Mide el consumo real en vez de estimarlo"),
            |_| {},
        );
        ui.add_space(Tokens::S2);
        hint_label(
            ui,
            "La estimación es aproximada. Calibrar arranca llama-server unos segundos con \
             estos ajustes y corrige la cifra con lo medido.",
        );
        ui.add_space(Tokens::S2);
        if o.calibrating {
            widgets::banner(ui, NoticeKind::Warning, "Midiendo…");
            if widgets::button_wide(
                ui,
                Kind::Danger,
                Some(icons::STOP),
                "Detener",
                Tokens::CONTROL,
            )
            .clicked()
            {
                s.actions.push(Action::Cancel);
            }
        } else {
            let can =
                !o.loading && o.estimate.is_some() && !s.running() && s.bench_started.is_none();
            let clicked = widgets::enabled(ui, can, |ui| {
                widgets::button_wide(
                    ui,
                    Kind::Secondary,
                    Some(icons::RULER),
                    "Calibrar",
                    Tokens::CONTROL,
                )
                .clicked()
            });
            if clicked {
                o.calibrating = true;
                s.dryfit_running = true;
                o.error = None;
                o.calibration = None;
                o.calibrating_heuristic = o.breakdown();
                s.actions.push(Action::Dryfit(apply_tuned_params(
                    &s.script, &o.params, &o.enabled,
                )));
            }
        }
    });
}

/// Diálogo del optimizador sobre la página actual; cerrarlo equivale a «Cancelar».
pub fn show(ctx: &egui::Context, s: &mut Store) {
    let Some(mut o) = s.optimizer.take() else {
        return;
    };
    let mut apply = false;
    let mut reload = false;
    let mut cancelled = false;
    let dialog = widgets::dialog(ctx, "optimizer", "Optimizador", Tokens::DIALOG_LG, |ui| {
        // Los parámetros se desplazan y el pie con las acciones queda siempre visible.
        let body = ctx.content_rect().height()
            - Tokens::DIALOG_CHROME
            - Tokens::S5 * 2.
            - Tokens::DIALOG_FOOTER;
        let body = body.max(Tokens::ROW);
        let ram = s.ram.mem_total_mi_b.unwrap_or(32768.);
        let width = ui.available_width();
        if width >= Tokens::TWO_COLUMNS {
            // Ancha: el modelo arriba y el consumo siempre a la vista mientras los
            // parámetros se desplazan.
            let top = ui.cursor().top();
            model_header(ui, &o, &mut reload);
            ui.add_space(Tokens::S4);
            let columns = (body - (ui.cursor().top() - top)).max(Tokens::ROW);
            let gap = Tokens::S4;
            let right = (width - gap) * Tokens::SIDE_COLUMN;
            let left = width - gap - right;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(
                    egui::vec2(left, columns),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("optimizer-params")
                            .max_height(columns)
                            .auto_shrink([false, true])
                            .show(ui, |ui| params(ui, &mut o, ram));
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(right, columns),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("optimizer-usage")
                            .max_height(columns)
                            .auto_shrink([false, true])
                            .show(ui, |ui| usage(ui, s, &mut o));
                    },
                );
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("optimizer-body")
                .max_height(body)
                .show(ui, |ui| {
                    model_header(ui, &o, &mut reload);
                    ui.add_space(Tokens::S4);
                    usage(ui, s, &mut o);
                    ui.add_space(Tokens::S3);
                    params(ui, &mut o, ram);
                });
        }
        widgets::dialog_footer(ui, |ui| {
            if widgets::enabled(ui, !o.loading && !o.calibrating, |ui| {
                widgets::button(ui, Kind::Primary, Some(icons::CHECK), "Aplicar al script")
                    .clicked()
            }) {
                apply = true;
            }
            if widgets::button(ui, Kind::Secondary, None, "Cancelar").clicked() {
                cancelled = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if widgets::enabled(ui, !o.calibrating, |ui| {
                    widgets::button(
                        ui,
                        Kind::Ghost,
                        Some(icons::ARROWS_CLOCKWISE),
                        "Restablecer",
                    )
                    .on_hover_text("Vuelve a los valores por defecto de llama-server")
                    .clicked()
                }) {
                    o.reset();
                }
            });
        });
    });
    if apply {
        s.script = apply_tuned_params(&s.script, &o.params, &o.enabled);
        s.notice("Parámetros aplicados al script");
        s.tab = TAB_BENCH;
    } else if cancelled || dialog.close {
        if o.calibrating {
            s.actions.push(Action::Cancel);
        }
    } else {
        s.optimizer = Some(o);
        if reload {
            open(s);
        }
    }
}

fn selected_devices<'a>(devices: &'a [LlamaDevice], ids: &[String]) -> Vec<&'a LlamaDevice> {
    devices
        .iter()
        .filter(|d| ids.is_empty() || ids.contains(&d.id))
        .collect()
}
