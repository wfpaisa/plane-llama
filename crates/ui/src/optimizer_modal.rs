// Página del optimizador: ajustes temporales y estimación local de VRAM, con
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

/// Fila de parámetro: interruptor que incluye el flag, nombre, flag y control debajo.
fn param_row(
    ui: &mut egui::Ui,
    enabled: &mut TunedFlagsEnabled,
    key: &str,
    label: &str,
    flag: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    let on = enabled.entry(key.into()).or_default();
    ui.horizontal(|ui| {
        widgets::toggle(ui, on, label);
        ui.label(
            RichText::new(flag)
                .font(egui::FontId::monospace(Tokens::FONT_XS))
                .color(Tokens::get(ui.ctx()).text_faint),
        );
    });
    let on = *on;
    ui.add_enabled_ui(on, control);
    ui.add_space(Tokens::S1);
}
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut egui::Ui,
    enabled: &mut TunedFlagsEnabled,
    key: &str,
    label: &str,
    flag: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
) {
    param_row(ui, enabled, key, label, flag, |ui| {
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
    value: &mut String,
) {
    param_row(ui, enabled, key, label, flag, |ui| {
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
fn group(ui: &mut egui::Ui, icon: &str, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    widgets::card(ui, |ui| {
        widgets::card_header(ui, icon, title, None, |_| {});
        body(ui);
    });
    ui.add_space(Tokens::S3);
}

fn model_header(ui: &mut egui::Ui, o: &Optimizer, reload: &mut bool) {
    let t = Tokens::get(ui.ctx());
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, icons::CUBE, t.accent_soft, t.accent_ink);
            ui.vertical(|ui| {
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
                            for chip in [
                                m.quant.clone().unwrap_or("—".into()),
                                format!("{} capas", fmt(m.layers, 0)),
                                format!("{}B params", fmt(m.params_b, 1)),
                                format!("pesos {} GB", fmt_gb(m.weights_file_mi_b, 2)),
                                format!("mmproj {} GB", fmt_gb(m.mmproj_size_mi_b, 2)),
                            ] {
                                widgets::chip(ui, &chip, t.surface_alt, t.text_dim);
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
            });
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
    group(ui, icons::STACK, "Contexto y capas", |ui| {
        slider(
            ui,
            &mut o.enabled,
            "ctxSize",
            "Contexto",
            "--ctx-size",
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
            &mut o.params.cache_reuse,
            0.0..=8192.0,
            64.,
        );
    });
    group(ui, icons::SQUARES_FOUR, "Batch", |ui| {
        slider(
            ui,
            &mut o.enabled,
            "batchSize",
            "Batch",
            "--batch-size",
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
            &mut o.params.ubatch_size,
            32.0..=1024.0,
            32.,
        );
    });
    group(ui, icons::DATABASE, "KV cache", |ui| {
        kv(
            ui,
            &mut o.enabled,
            "cacheTypeK",
            "Tipo K",
            "--cache-type-k",
            &mut o.params.cache_type_k,
        );
        kv(
            ui,
            &mut o.enabled,
            "cacheTypeV",
            "Tipo V",
            "--cache-type-v",
            &mut o.params.cache_type_v,
        );
        for (key, label, flag) in [
            ("cacheTypeKDraft", "Tipo K draft", "--cache-type-k-draft"),
            ("cacheTypeVDraft", "Tipo V draft", "--cache-type-v-draft"),
        ] {
            let mut value = o
                .params
                .extra
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("q8_0")
                .to_string();
            kv(ui, &mut o.enabled, key, label, flag, &mut value);
            o.params.extra.insert(key.into(), serde_json::json!(value));
        }
        widgets::toggle(
            ui,
            &mut o.params.flash_attn,
            "Flash attention (--flash-attn)",
        );
    });
    group(ui, icons::MEMORY, "Memoria y especulativo", |ui| {
        slider(
            ui,
            &mut o.enabled,
            "cacheRam",
            "Caché en RAM",
            "--cache-ram",
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
            &mut o.params.spec_draft_max,
            0.0..=8.0,
            1.,
        );
        let mut vision = !o.params.no_mmproj;
        if widgets::toggle(ui, &mut vision, "Cargar visión (sin --no-mmproj)").changed() {
            o.params.no_mmproj = !vision;
        }
    });
    if let Some(e) = &o.estimate {
        let devices: Vec<_> = e.devices.clone();
        group(ui, icons::CPU, "Dispositivos", |ui| {
            widgets::toggle(
                ui,
                o.enabled.entry("device".into()).or_default(),
                "Fijar dispositivos (--device)",
            );
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
            let chosen = selected_devices(&devices, &o.params.device);
            let mut auto = o.params.tensor_split.is_none();
            if widgets::toggle(ui, &mut auto, "Tensor split automático").changed() {
                o.params.tensor_split = if auto {
                    None
                } else {
                    Some(vec![1.; chosen.len()])
                };
            }
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
        if total > capacity && capacity > 0. {
            ui.add_space(Tokens::S1);
            widgets::banner(
                ui,
                NoticeKind::Error,
                "La configuración actual excede la VRAM disponible.",
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
            Some("Arranca llama-server un momento para medir el consumo real"),
            |_| {},
        );
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

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    let Some(mut o) = s.optimizer.take() else {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::LIGHTNING,
                "Optimizador",
                "Analiza el script actual para estimar cuánta VRAM necesita.",
            );
            ui.vertical_centered(|ui| {
                if widgets::button(
                    ui,
                    Kind::Primary,
                    Some(icons::LIGHTNING),
                    "Analizar script actual",
                )
                .clicked()
                {
                    open(s);
                }
            });
            ui.add_space(Tokens::S4);
        });
        return;
    };
    let t = Tokens::get(ui.ctx());
    let mut apply = false;
    let mut reload = false;
    let mut cancelled = false;
    // Pie fijo con las acciones; el resto de la página se desplaza por encima.
    egui::Panel::bottom("optimizer-actions")
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin {
            top: Tokens::S3 as i8,
            ..Default::default()
        }))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if widgets::enabled(ui, !o.calibrating, |ui| {
                    widgets::button(ui, Kind::Ghost, Some(icons::ARROWS_CLOCKWISE), "Default")
                        .on_hover_text("Valores por defecto de llama-server")
                        .clicked()
                }) {
                    o.reset();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::enabled(ui, !o.loading && !o.calibrating, |ui| {
                        widgets::button(ui, Kind::Primary, Some(icons::CHECK), "Aplicar al script")
                            .clicked()
                    }) {
                        apply = true;
                    }
                    if widgets::button(ui, Kind::Secondary, None, "Cancelar").clicked() {
                        cancelled = true;
                    }
                });
            });
        });
    egui::ScrollArea::vertical()
        .id_salt("optimizer-page")
        .auto_shrink(false)
        .show(ui, |ui| {
            model_header(ui, &o, &mut reload);
            ui.add_space(Tokens::S4);
            let ram = s.ram.mem_total_mi_b.unwrap_or(32768.);
            let width = ui.available_width();
            {
                if width >= Tokens::TWO_COLUMNS {
                    let gap = Tokens::S4;
                    let right = (width - gap) * Tokens::SIDE_COLUMN;
                    let left = width - gap - right;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        ui.allocate_ui_with_layout(
                            egui::vec2(left, 0.),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(left);
                                params(ui, &mut o, ram);
                            },
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(right, 0.),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(right);
                                usage(ui, s, &mut o);
                            },
                        );
                    });
                } else {
                    usage(ui, s, &mut o);
                    ui.add_space(Tokens::S3);
                    params(ui, &mut o, ram);
                }
            }
        });
    if apply {
        s.script = apply_tuned_params(&s.script, &o.params, &o.enabled);
        s.notice("Parámetros aplicados al script");
        s.tab = TAB_BENCH;
    } else if cancelled {
        if o.calibrating {
            s.actions.push(Action::Cancel);
        }
        s.tab = TAB_BENCH;
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
