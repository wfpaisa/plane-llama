// Historial configurable con orden, filtro, selección y edición persistida.
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, icons};
use crate::{
    bridge::Action,
    store::{DEFAULT_COLUMNS, Store, TAB_BENCH},
};
use egui::{RichText, Sense, emath::GuiRounding};
use egui_extras::{Column, TableBuilder};
use plane_llama_core::{
    client::{format::*, store_logic},
    *,
};

pub const COLUMNS: &[(&str, &str, &str)] = &[
    ("actions", "Acciones", ""),
    ("date", "Fecha", "timestamp"),
    ("fav", "♥", "favorite"),
    ("rating", "★ Calificación", "rating"),
    ("model", "Modelo", ""),
    ("backend", "Backend", ""),
    ("ctx", "ctx", "config.ctxSize"),
    ("batch", "batch", ""),
    ("cache", "cache", ""),
    ("tsplit", "tsplit", ""),
    ("genTokens", "Generated tokens", ""),
    ("generationTime", "Generation time", "generationTimeMs"),
    ("genTps", "Generation speed", "generationTokensPerSecond"),
    ("promptTokens", "Prompt tokens", ""),
    ("promptTime", "Prompt processing time", ""),
    ("promptTps", "Prompt speed", "promptTokensPerSecond"),
    ("draftAcc", "draft acc", "draftAcceptance"),
    ("genDr", "gen dr", ""),
    ("accDr", "acc dr", ""),
    ("loadTime", "load s", "loadTimeSeconds"),
    ("vram", "VRAM", ""),
    ("totalVram", "Total VRAM", "totalVramMiB"),
    ("ram", "RAM", "ramUsedMiB"),
];
/// Grupos de métricas; cada uno tiñe sus columnas para leer la tabla por bloques.
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Write,
    Read,
    Speculative,
}
const GROUPS: [(Group, &str, &str); 3] = [
    (Group::Write, icons::PENCIL_SIMPLE, "Escritura"),
    (Group::Read, icons::EYE, "Lectura"),
    (Group::Speculative, icons::MAGIC_WAND, "Especulativo"),
];
fn group(key: &str) -> Option<Group> {
    match key {
        "genTokens" | "generationTime" | "genTps" => Some(Group::Write),
        "promptTokens" | "promptTime" | "promptTps" => Some(Group::Read),
        "draftAcc" | "genDr" | "accDr" => Some(Group::Speculative),
        _ => None,
    }
}
/// Tinte del grupo: más intenso en el encabezado que en las filas.
fn tint(ui: &egui::Ui, group: Group, header: bool) -> egui::Color32 {
    let t = Tokens::get(ui.ctx());
    let base = t.groups[group as usize];
    let alpha = match (ui.visuals().dark_mode, header) {
        (true, true) => 0.18,
        (true, false) => 0.06,
        (false, true) => 0.13,
        (false, false) => 0.05,
    };
    base.gamma_multiply(alpha)
}
/// Fondo del grupo y líneas de la cuadrícula de una celda.
fn paint_cell(ui: &egui::Ui, key: Option<&str>, header: bool) {
    let t = Tokens::get(ui.ctx());
    let rect = ui
        .max_rect()
        .expand2(ui.spacing().item_spacing * 0.5)
        .round_ui();
    let painter = ui.painter();
    if let Some(group) = key.and_then(group) {
        painter.rect_filled(rect, 0., tint(ui, group, header));
    }
    let stroke = egui::Stroke::new(Tokens::BORDER, t.border);
    painter.vline(rect.right(), rect.y_range(), stroke);
    painter.hline(rect.x_range(), rect.bottom(), stroke);
}
/// Separa la cifra de su unidad para pintar la unidad más tenue.
fn value_unit(r: &BenchmarkResult, key: &str) -> (String, &'static str) {
    match key {
        "generationTime" => (fmt_sec(r.generation_time_ms), "s"),
        "genTps" => (fmt(r.generation_tokens_per_second, 1), "t/s"),
        "promptTime" => (fmt_sec(r.prompt_eval_time_ms), "s"),
        "promptTps" => (fmt(r.prompt_tokens_per_second, 1), "t/s"),
        "loadTime" => (fmt(r.load_time_seconds, 2), "s"),
        "totalVram" => (total_device_vram_txt(r), "GB"),
        "ram" => (fmt_gb(r.ram_used_mi_b, 2), "GB"),
        _ => (cell(r, key), ""),
    }
}
pub fn total_vram(r: &BenchmarkResult) -> Option<f64> {
    let n = if r.device_vram.is_empty() {
        r.gpus.iter().filter_map(|g| g.mem_used_mi_b).sum()
    } else {
        r.device_vram.iter().filter_map(|d| d.used_mi_b).sum()
    };
    (n > 0.).then_some(n)
}
pub fn cell(r: &BenchmarkResult, key: &str) -> String {
    let c = &r.config;
    match key {
        "date" => date(&r.timestamp),
        "model" => short_model(c.model.as_deref()),
        "backend" => compute_backend_label(r.backend.as_deref(), Some(c.n_cpu_moe)),
        "ctx" => fmt(c.ctx_size, 0),
        "batch" => {
            let (b, u) = effective_batch(c.batch_size, c.ubatch_size);
            format!("{}/{}", fmt(Some(b), 0), fmt(Some(u), 0))
        }
        "cache" => {
            let (k, v) = effective_cache(c.cache_type_k.as_deref(), c.cache_type_v.as_deref());
            format!("{k}/{v}")
        }
        "tsplit" => c.tensor_split.clone().unwrap_or("—".into()),
        "device" => c.device.clone().unwrap_or("—".into()),
        "genTokens" => fmt(r.generation_token_count, 0),
        "generationTime" => format!("{} s", fmt_sec(r.generation_time_ms)),
        "genTps" => format!(
            "{} t/s{}",
            fmt(r.generation_tokens_per_second, 1),
            c.spec_draft_n_max
                .map(|n| format!(" · MTP: {n}"))
                .unwrap_or_default()
        ),
        "promptTokens" => fmt(r.prompt_token_count, 0),
        "promptTime" => format!("{} s", fmt_sec(r.prompt_eval_time_ms)),
        "promptTps" => format!("{} t/s", fmt(r.prompt_tokens_per_second, 1)),
        "draftAcc" => fmt(r.draft_acceptance, 3),
        "genDr" => fmt(r.gen_drafts, 0),
        "accDr" => fmt(r.acc_drafts, 0),
        "loadTime" => fmt(r.load_time_seconds, 2),
        "vram" => device_vram_line(r, true),
        "totalVram" => total_device_vram_txt(r),
        "ram" => fmt_gb(r.ram_used_mi_b, 2),
        _ => "—".into(),
    }
}

/// Encabezado ordenable; el nombre accesible incluye la flecha del orden actual.
fn sort_header(ui: &mut egui::Ui, s: &mut Store, label: &str, field: &str) {
    let t = Tokens::get(ui.ctx());
    let active = s.sort.col == field;
    let (indicator, arrow) = match (active, s.sort.dir.as_str()) {
        (true, "asc") => (" ↑", icons::ARROW_UP),
        (true, _) => (" ↓", icons::ARROW_DOWN),
        _ => ("", ""),
    };
    let painter = ui.painter().clone();
    let text = painter.layout_no_wrap(
        label.to_uppercase(),
        theme::font_semibold(Tokens::FONT_XS),
        Tokens::INK,
    );
    let icon = painter.layout_no_wrap(
        arrow.to_owned(),
        theme::icon_font(Tokens::FONT_SM),
        Tokens::INK,
    );
    let size = egui::vec2(
        text.size().x + icon.size().x + Tokens::S1,
        Tokens::CONTROL - Tokens::S2,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{label}{indicator}"),
        )
    });
    let hovered = response.hovered();
    let y = rect.center().y - text.size().y / 2.;
    let width = text.size().x;
    painter.galley(
        egui::pos2(rect.left(), y),
        text,
        if active || hovered {
            t.text
        } else {
            t.text_dim
        },
    );
    painter.galley(
        egui::pos2(rect.left() + width + Tokens::S1, y),
        icon,
        t.accent_ink,
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        s.sort_by(field);
    }
}
/// Celda con margen a los lados, para que el contenido no quede pegado al de la columna
/// vecina.
fn padded(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let rect = ui.max_rect().shrink2(egui::vec2(Tokens::CELL_PAD, 0.));
    let layout = *ui.layout();
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect).layout(layout), add);
}
/// Guarda el ancho actual de la columna para restaurarlo al reabrir la app.
fn remember_width(ui: &egui::Ui, s: &mut Store, key: &str) {
    if ui.is_sizing_pass() {
        return;
    }
    let width = ui.max_rect().width();
    let known = s.column_widths.get(key).copied();
    if known.is_none_or(|w| (w - width).abs() > 0.5) {
        s.column_widths.insert(key.to_owned(), width);
    }
}
fn header_text(ui: &mut egui::Ui, label: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(
        RichText::new(label.to_uppercase())
            .font(theme::font_semibold(Tokens::FONT_XS))
            .color(t.text_dim),
    );
}
fn clickable(ui: &mut egui::Ui, text: RichText, hover: &str) -> egui::Response {
    let response = ui
        .add(egui::Label::new(text).sense(Sense::click()))
        .on_hover_text(hover);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn toolbar(ui: &mut egui::Ui, s: &mut Store, total: usize) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        let models = if s.models.is_empty() {
            "Todos los modelos".to_owned()
        } else {
            format!("{} modelos", s.models.len())
        };
        let response = widgets::button(ui, Kind::Secondary, Some(icons::FUNNEL), &models);
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(Tokens::SEARCH_WIDTH);
                if widgets::button(ui, Kind::Ghost, Some(icons::X), "Todos").clicked() {
                    s.models.clear();
                    s.page = 0;
                }
                for model in store_logic::model_options(&s.history) {
                    let mut checked = s.models.contains(&model);
                    if ui.checkbox(&mut checked, &model).changed() {
                        if checked {
                            s.models.push(model);
                        } else {
                            s.models.retain(|m| m != &model);
                        }
                        s.page = 0;
                    }
                }
            });
        if widgets::search(
            ui,
            &mut s.history_filter,
            "Filtrar resultados",
            Tokens::SEARCH_NARROW,
        )
        .changed()
        {
            s.page = 0;
        }
        let label = format!("{} columnas", s.columns.len());
        let response = widgets::button(ui, Kind::Secondary, Some(icons::COLUMNS), &label);
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(Tokens::SEARCH_WIDTH);
                if widgets::button(
                    ui,
                    Kind::Ghost,
                    Some(icons::ARROWS_CLOCKWISE),
                    "Restablecer",
                )
                .clicked()
                {
                    s.columns = DEFAULT_COLUMNS.iter().map(|s| (*s).into()).collect();
                }
                for (key, label, _) in COLUMNS {
                    let mut checked = s.columns.iter().any(|s| s == key);
                    if ui.checkbox(&mut checked, *label).changed() {
                        if checked {
                            s.columns.push((*key).into());
                        } else {
                            s.columns.retain(|s| s != key);
                        }
                    }
                }
            });
        ui.add_space(Tokens::S2);
        for (group, icon, label) in GROUPS {
            legend(ui, s, group, icon, label);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let clicked = widgets::enabled(ui, !s.history.is_empty(), |ui| {
                widgets::button(ui, Kind::Ghost, Some(icons::TRASH), "Limpiar todo").clicked()
            });
            if clicked {
                s.confirm("¿Eliminar todo el historial?", Action::ClearHistory);
            }

            ui.label(
                RichText::new(format!("{total} de {} resultados", s.history.len()))
                    .font(theme::font(Tokens::FONT_SM))
                    .color(t.text_dim),
            );
        });
    });
}

/// Interruptor de la leyenda: muestra u oculta todas las columnas del grupo.
fn legend(ui: &mut egui::Ui, s: &mut Store, group: Group, icon: &str, label: &str) {
    let t = Tokens::get(ui.ctx());
    let keys: Vec<_> = COLUMNS
        .iter()
        .map(|(k, _, _)| *k)
        .filter(|k| self::group(k) == Some(group))
        .collect();
    let shown = keys.iter().any(|k| s.columns.iter().any(|c| c == k));
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Tokens::S1;
            let (swatch, _) =
                ui.allocate_exact_size(egui::vec2(Tokens::ICON, Tokens::ICON), Sense::hover());
            let fill = if shown {
                t.groups[group as usize]
            } else {
                t.transparent
            };
            ui.painter().rect(
                swatch.shrink(1.),
                3.,
                fill,
                egui::Stroke::new(Tokens::BORDER, t.border_strong),
                egui::StrokeKind::Inside,
            );
            let ink = if shown { t.text_dim } else { t.text_faint };
            ui.label(
                RichText::new(icon)
                    .font(theme::icon_font(Tokens::FONT_SM))
                    .color(ink),
            );
            ui.label(
                RichText::new(label)
                    .font(theme::font(Tokens::FONT_SM))
                    .color(ink),
            );
        })
        .response
        .interact(Sense::click())
        .on_hover_text(if shown {
            format!("Ocultar columnas de {}", label.to_lowercase())
        } else {
            format!("Mostrar columnas de {}", label.to_lowercase())
        });
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        if shown {
            s.columns.retain(|c| !keys.contains(&c.as_str()));
        } else {
            s.columns.extend(keys.iter().map(|k| (*k).to_owned()));
        }
    }
    ui.add_space(Tokens::S2);
}

/// Barra que aparece al seleccionar filas, con las acciones sobre la selección.
fn selection_bar(ui: &mut egui::Ui, s: &mut Store) {
    if s.selected.is_empty() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.accent_soft)
        .corner_radius(Tokens::RADIUS_MD + 2)
        .inner_margin(egui::Margin::symmetric(
            Tokens::S3 as i8,
            Tokens::S1 as i8 + 2,
        ))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} seleccionados", s.selected.len()))
                        .font(theme::font_semibold(Tokens::FONT_BODY))
                        .color(t.accent_ink),
                );
                ui.add_space(Tokens::S2);
                let compare = widgets::enabled(ui, s.selected.len() >= 2, |ui| {
                    widgets::button(ui, Kind::Secondary, Some(icons::COLUMNS), "Comparar").clicked()
                });
                if compare {
                    s.compare = true;
                }
                if widgets::button(ui, Kind::Secondary, Some(icons::CHART_BAR), "Gráfico").clicked()
                {
                    s.chart_order = s
                        .rows()
                        .iter()
                        .filter(|r| s.selected.contains(&r.id))
                        .map(|r| r.id.clone())
                        .collect();
                    s.chart = true;
                }
                if widgets::button(
                    ui,
                    Kind::Danger,
                    Some(icons::TRASH),
                    "Eliminar seleccionados",
                )
                .clicked()
                {
                    s.confirm(
                        format!("¿Eliminar {} resultados seleccionados?", s.selected.len()),
                        Action::Delete(s.selected.iter().cloned().collect()),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, icons::X, "Quitar selección").clicked() {
                        s.selected.clear();
                    }
                });
            });
        });
    ui.add_space(Tokens::S2);
}

fn model_cell(ui: &mut egui::Ui, r: &BenchmarkResult) {
    let t = Tokens::get(ui.ctx());
    let Some(p) = parse_model(r.config.model.as_deref()) else {
        ui.label("—");
        return;
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = Tokens::S1;
        if let Some(org) = p.org {
            ui.label(
                RichText::new(format!("{org}/"))
                    .font(theme::font(Tokens::FONT_SM))
                    .color(t.text_faint),
            );
        }
        ui.label(
            RichText::new(p.base)
                .font(theme::font_medium(Tokens::FONT_BODY))
                .color(t.text),
        );
        for badge in p.size.into_iter().chain(p.tags).chain(p.quant) {
            widgets::chip(ui, &badge, t.surface_alt, t.text_dim);
        }
        if p.mtp {
            widgets::chip(ui, "MTP", t.accent_soft, t.accent_ink);
        }
    })
    .response
    .on_hover_text(r.config.model.as_deref().unwrap_or("—"));
}

fn rating_cell(ui: &mut egui::Ui, s: &mut Store, r: &BenchmarkResult) {
    let t = Tokens::get(ui.ctx());
    let current = r.rating.flatten().unwrap_or(0.) as usize;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.;
        for n in 1..=10 {
            let filled = n <= current;
            if clickable(
                ui,
                RichText::new(if filled { "★" } else { "☆" })
                    .font(theme::font(Tokens::FONT_SM))
                    .color(if filled { t.warning } else { t.text_faint }),
                &format!("Calificación {n} de 10"),
            )
            .clicked()
            {
                s.actions.push(Action::Patch(
                    r.id.clone(),
                    ResultPatch {
                        rating: Some((n != current).then_some(n as f64)),
                        ..Default::default()
                    },
                ));
            }
        }
    });
}

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    let rows: Vec<_> = s.rows().into_iter().cloned().collect();
    toolbar(ui, s, rows.len());
    ui.add_space(Tokens::S2);
    selection_bar(ui, s);
    if rows.is_empty() && !s.history.is_empty() {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::FUNNEL,
                "Sin coincidencias",
                "Ningún resultado coincide con los filtros actuales.",
            );
            ui.add_space(Tokens::S4);
        });
        return;
    }
    if rows.is_empty() {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::CLOCK_COUNTER_CLOCKWISE,
                "Sin resultados todavía",
                "Cada benchmark que ejecutes se guarda aquí para compararlo.",
            );
            ui.vertical_centered(|ui| {
                if widgets::button(ui, Kind::Primary, Some(icons::PLAY), "Ir a Benchmark").clicked()
                {
                    s.tab = TAB_BENCH;
                }
            });
            ui.add_space(Tokens::S4);
        });
        return;
    }
    let last_page = rows.len().saturating_sub(1) / s.page_size;
    s.page = s.page.min(last_page);
    let best = store_logic::compute_bests(&s.history);
    let columns: Vec<_> = COLUMNS
        .iter()
        .filter(|(k, _, _)| s.columns.iter().any(|c| c == k))
        .copied()
        .collect();
    let footer = Tokens::CONTROL + Tokens::S4 * 2.;
    egui::Frame::new()
        .fill(t.surface)
        .stroke(egui::Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_LG)
        .inner_margin(egui::Margin::symmetric(Tokens::S2 as i8, Tokens::S2 as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let table_height = (ui.available_height() - footer).max(Tokens::ROW * 4.);
            egui::ScrollArea::horizontal()
                .id_salt("history-horizontal")
                .show(ui, |ui| {
                    let mut table = TableBuilder::new(ui)
                        .sense(Sense::click())
                        .id_salt("history-table")
                        .striped(false)
                        .resizable(true)
                        .auto_shrink([false, true])
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .max_scroll_height(table_height)
                        .column(Column::exact(Tokens::CHECK_WIDTH))
                        .column(Column::exact(Tokens::INDEX_WIDTH));
                    // Los anchos de contenido más el margen de cada lado de la celda.
                    let pad = 2. * Tokens::CELL_PAD;
                    for (key, _, _) in &columns {
                        let initial = s.column_widths.get(*key).copied().unwrap_or(match *key {
                            "model" => Tokens::MODEL_WIDTH + pad,
                            "rating" => Tokens::RATING_WIDTH + pad,
                            "actions" => Tokens::HISTORY_ACTIONS + pad,
                            "fav" => Tokens::ACTIONS_WIDTH + pad,
                            _ => Tokens::COLUMN_WIDTH,
                        });
                        table =
                            table.column(Column::initial(initial).at_least(if *key == "actions" {
                                Tokens::HISTORY_ACTIONS + pad
                            } else {
                                Tokens::COLUMN_MIN
                            }));
                    }
                    table
                        .header(Tokens::TABLE_HEADER, |mut header| {
                            header.col(|ui| {
                                paint_cell(ui, None, true);
                                let mut all = rows.iter().all(|r| s.selected.contains(&r.id));
                                if ui
                                    .checkbox(&mut all, "")
                                    .on_hover_text("Seleccionar todo")
                                    .changed()
                                {
                                    for r in &rows {
                                        if all {
                                            s.selected.insert(r.id.clone());
                                        } else {
                                            s.selected.remove(&r.id);
                                        }
                                    }
                                }
                            });
                            header.col(|ui| {
                                paint_cell(ui, None, true);
                                header_text(ui, "#");
                            });
                            for (key, label, field) in &columns {
                                header.col(|ui| {
                                    remember_width(ui, s, key);
                                    paint_cell(ui, Some(key), true);
                                    padded(ui, |ui| {
                                        // La columna de acciones solo lleva iconos; no necesita
                                        // título.
                                        match (*label, field.is_empty()) {
                                            ("Acciones", _) => {}
                                            (_, true) => header_text(ui, label),
                                            _ => sort_header(ui, s, label, field),
                                        }
                                    });
                                });
                            }
                        })
                        .body(|body| {
                            let start = s.page * s.page_size;
                            let end = (start + s.page_size).min(rows.len());
                            body.rows(Tokens::ROW, end - start, |mut row| {
                                let index = start + row.index();
                                let r = &rows[index];
                                row.set_selected(s.selected.contains(&r.id));
                                row.col(|ui| {
                                    paint_cell(ui, None, false);
                                    let mut selected = s.selected.contains(&r.id);
                                    if ui
                                        .checkbox(&mut selected, "")
                                        .on_hover_text(format!("Seleccionar {}", r.id))
                                        .changed()
                                    {
                                        if selected {
                                            s.selected.insert(r.id.clone());
                                        } else {
                                            s.selected.remove(&r.id);
                                        }
                                    }
                                });
                                row.col(|ui| {
                                    paint_cell(ui, None, false);
                                    ui.label(
                                        RichText::new((index + 1).to_string())
                                            .monospace()
                                            .color(t.text_faint),
                                    );
                                });
                                for (key, _, _) in &columns {
                                    row.col(|ui| {
                                        paint_cell(ui, Some(key), false);
                                        padded(ui, |ui| {
                                            ui.push_id((&r.id, *key), |ui| {
                                                body_cell(ui, s, r, key, &best)
                                            });
                                        });
                                    });
                                }
                            });
                        });
                });
            pagination(ui, s, rows.len(), last_page);
        });
}

fn body_cell(
    ui: &mut egui::Ui,
    s: &mut Store,
    r: &BenchmarkResult,
    key: &str,
    best: &store_logic::Bests,
) {
    let t = Tokens::get(ui.ctx());
    match key {
        "model" => model_cell(ui, r),
        "actions" => {
            let has_script = !r.config.script.is_empty();
            let (load, save) = widgets::enabled(ui, has_script, |ui| {
                (
                    widgets::icon_button(ui, icons::ARROW_SQUARE_OUT, "Cargar script en editor")
                        .clicked(),
                    widgets::icon_button(ui, icons::FILE_PLUS, "Guardar como script").clicked(),
                )
            });
            if load {
                s.script = r.config.script.clone();
                s.editor_source = None;
                s.tab = TAB_BENCH;
                s.notice("Script cargado en el editor");
            }
            // Abre el diálogo de nuevo script con el comando de esta corrida.
            if save {
                s.library.draft = Some(crate::library::Draft {
                    to_editor: false,
                    id: None,
                    name: String::new(),
                    script: r.config.script.clone(),
                });
            }
        }
        "fav" => {
            let favorite = r.favorite.unwrap_or(false);
            if clickable(
                ui,
                RichText::new(if favorite { "♥" } else { "♡" })
                    .font(theme::font(Tokens::ICON))
                    .color(if favorite { t.error } else { t.text_faint }),
                "Favorito",
            )
            .clicked()
            {
                s.actions.push(Action::Patch(
                    r.id.clone(),
                    ResultPatch {
                        favorite: Some(!favorite),
                        ..Default::default()
                    },
                ));
            }
        }
        "rating" => rating_cell(ui, s, r),
        "backend" => {
            let text = cell(r, key);
            widgets::chip(ui, &text, t.ok_soft, t.ok);
        }
        "vram" => {
            let lines = device_vram_line(r, false);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                for line in lines.split(" · ") {
                    ui.label(
                        RichText::new(line)
                            .font(egui::FontId::monospace(Tokens::FONT_XS - 1.))
                            .color(t.text_dim),
                    );
                }
            })
            .response
            .on_hover_text(device_vram_line(r, false));
        }
        _ => {
            let is_best = match key {
                "genTps" => r.generation_tokens_per_second == Some(best.g),
                "promptTps" => r.prompt_tokens_per_second == Some(best.p),
                "draftAcc" => r.draft_acceptance == Some(best.d),
                "loadTime" => r.load_time_seconds == Some(best.l),
                "generationTime" => r.generation_time_ms == Some(best.gt),
                _ => false,
            };
            let numeric = !matches!(key, "date" | "cache" | "tsplit");
            let (value, unit) = value_unit(r, key);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = Tokens::S1;
                let font = if numeric {
                    egui::FontId::monospace(Tokens::FONT_SM)
                } else {
                    theme::font(Tokens::FONT_SM + 0.5)
                };
                if is_best {
                    widgets::best(ui, &value);
                } else {
                    ui.label(RichText::new(&value).font(font).color(t.text));
                }
                if !unit.is_empty() && value != "—" {
                    ui.label(
                        RichText::new(unit)
                            .font(theme::font(Tokens::FONT_XS))
                            .color(t.text_faint),
                    );
                }
                if let Some(n) = r.config.spec_draft_n_max.filter(|_| key == "genTps") {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        widgets::chip(ui, &format!("MTP: {n}"), t.warning_soft, t.warning);
                    });
                }
            })
            .response
            .on_hover_text(format!(
                "{}\n{}",
                r.config.model.as_deref().unwrap_or(""),
                r.config.script
            ));
        }
    }
}

fn pagination(ui: &mut egui::Ui, s: &mut Store, total: usize, last_page: usize) {
    let t = Tokens::get(ui.ctx());
    widgets::separator(ui);
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(Tokens::S2 as i8, Tokens::S1 as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let start = s.page * s.page_size + 1;
                let end = ((s.page + 1) * s.page_size).min(total);
                ui.label(
                    RichText::new(format!("Mostrando {start}–{end} de {total}"))
                        .font(theme::font(Tokens::FONT_SM))
                        .color(t.text_dim),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("page-size")
                        .selected_text(format!("{} por página", s.page_size))
                        .show_ui(ui, |ui| {
                            for n in [25, 50, 100] {
                                ui.selectable_value(&mut s.page_size, n, n.to_string());
                            }
                        });
                    ui.add_space(Tokens::S2);
                    if widgets::enabled(ui, s.page < last_page, |ui| {
                        widgets::icon_button(ui, icons::CARET_RIGHT, "Siguiente").clicked()
                    }) {
                        s.page += 1;
                    }
                    ui.label(
                        RichText::new(format!("{} / {}", s.page + 1, last_page + 1))
                            .font(egui::FontId::monospace(Tokens::FONT_SM))
                            .color(t.text),
                    );
                    if widgets::enabled(ui, s.page > 0, |ui| {
                        widgets::icon_button(ui, icons::CARET_LEFT, "Anterior").clicked()
                    }) {
                        s.page -= 1;
                    }
                });
            });
        });
}

pub fn date(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%d/%m/%Y, %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| timestamp.into())
}
