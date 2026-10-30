// Scripts: la biblioteca en una sola lista, lista para arrancar un servidor o medirlo con
// un clic. Es la página principal de la vista simple.
use crate::{
    bridge::Action,
    library::{Draft, ScriptFilter, ScriptMeta, ScriptSort},
    models_page::{self, bytes, hover_row, slot},
    script_editor,
    store::{Store, TAB_BENCH, TAB_MODELS},
    theme::{self, Tokens},
    widgets::{self, Kind, icons},
};
use egui::{Align, Layout, RichText, Ui};
use plane_llama_core::{
    client::format::{fmt, fmt_gb},
    scripts::SavedScript,
};

const SORTS: [(ScriptSort, &str); 2] = [
    (ScriptSort::Recent, "Recientes"),
    (ScriptSort::Manual, "Manual"),
];

/// Qué hacer con un script tras pintar la lista (la lista toma prestado el store).
pub(crate) enum RowAction {
    Start(String),
    Stop,
    Bench(String),
    Edit(String),
    Open(String),
    Duplicate(String),
    Favorite(String),
    Hide(String, bool),
    Delete(String),
    /// Orden manual: el primero se suelta antes (o después, `true`) del segundo.
    Move(String, String, bool),
    Select(String, bool),
    /// Marca (`true`) o desmarca todos los de la lista.
    SelectAll(Vec<String>, bool),
    DeleteSelected,
}

pub fn show(ui: &mut Ui, s: &mut Store) {
    toolbar(ui, s);
    ui.add_space(Tokens::S4);
    activity(ui, s);
    if !s.library.loaded {
        ui.spinner();
        return;
    }
    if s.library.list.is_empty() {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::ROCKET_LAUNCH,
                "Todavía no hay scripts",
                "Crea uno a partir de un modelo descargado o escribe el comando de llama-server.",
            );
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    if widgets::button(
                        ui,
                        Kind::Primary,
                        Some(icons::PLUS),
                        "Crear el primer script",
                    )
                    .clicked()
                    {
                        new_script(s);
                    }
                    if widgets::button(ui, Kind::Secondary, Some(icons::CUBE), "Ver modelos")
                        .clicked()
                    {
                        s.tab = TAB_MODELS;
                    }
                });
            });
        });
        return;
    }
    let running = s.running();
    let running_id = s.library.running_id(running).map(str::to_owned);
    let blocked = s.busy() || s.manual_pending;
    let rows: Vec<SavedScript> = s.library.sorted().into_iter().cloned().collect();
    let mut action = None;
    // Lo eliminado (o ya no listado tras recargar) deja de estar marcado.
    let listed: Vec<String> = rows.iter().map(|x| x.id.clone()).collect();
    s.library
        .selected
        .retain(|id| s.library.list.iter().any(|x| &x.id == id));
    if !s.library.selected.is_empty() {
        action = selection_bar(ui, s, listed);
        ui.add_space(Tokens::S2);
    }
    egui::ScrollArea::vertical()
        .id_salt("scripts")
        .auto_shrink(false)
        .show(ui, |ui| {
            widgets::card(ui, |ui| {
                if rows.is_empty() {
                    widgets::dim(
                        ui,
                        match s.library.filter {
                            ScriptFilter::Hidden => "No hay scripts ocultos.",
                            _ => "Ningún script coincide con la búsqueda.",
                        },
                    );
                }
                for script in &rows {
                    let is_running = running_id.as_deref() == Some(script.id.as_str());
                    if let Some(a) = row(ui, s, script, is_running, running || blocked, Place::Page)
                    {
                        action = Some(a);
                    }
                }
            });
        });
    if let Some(a) = action {
        apply(s, a);
    }
}

pub(crate) fn apply(s: &mut Store, a: RowAction) {
    match a {
        RowAction::Start(id) => s.start_script(&id),
        RowAction::Stop => s.stop_server(),
        RowAction::Bench(id) => s.bench_script(&id),
        RowAction::Edit(id) => {
            if let Some(x) = s.library.get(&id) {
                s.library.draft = Some(Draft {
                    id: Some(x.id.clone()),
                    name: x.name.clone(),
                    script: x.script.clone(),
                });
            }
        }
        RowAction::Open(id) => {
            if let Some(x) = s.library.get(&id).map(|x| x.script.clone()) {
                s.open_in_editor(x);
            }
        }
        RowAction::Duplicate(id) => {
            s.library.duplicate(&id);
            s.save_library();
        }
        RowAction::Favorite(id) => {
            s.library.toggle_favorite(&id);
            s.save_library();
        }
        RowAction::Hide(id, hidden) => {
            s.library.set_hidden(&id, hidden);
            s.save_library();
            if hidden {
                s.notice("Script oculto. Míralo en «Ocultos».");
            }
        }
        RowAction::Move(id, target, after) => {
            s.library.move_to(&id, &target, after);
            s.save_library();
        }
        RowAction::Select(id, on) => {
            if on {
                s.library.selected.insert(id);
            } else {
                s.library.selected.remove(&id);
            }
        }
        RowAction::SelectAll(ids, on) => {
            if on {
                s.library.selected.extend(ids);
            } else {
                s.library.selected.clear();
            }
        }
        RowAction::DeleteSelected => {
            let n = s.library.selected.len();
            if n == 0 {
                return;
            }
            let rest: Vec<SavedScript> = s
                .library
                .list
                .iter()
                .filter(|x| !s.library.selected.contains(&x.id))
                .cloned()
                .collect();
            let what = if n == 1 {
                "el script seleccionado".to_owned()
            } else {
                format!("los {n} scripts seleccionados")
            };
            s.confirm(
                format!("¿Eliminar {what}? El historial de benchmarks se conserva."),
                Action::DeleteScript(rest),
            );
        }
        RowAction::Delete(id) => {
            if let Some(x) = s.library.get(&id) {
                let name = x.name.clone();
                let rest: Vec<SavedScript> = s
                    .library
                    .list
                    .iter()
                    .filter(|x| x.id != id)
                    .cloned()
                    .collect();
                s.confirm(
                    format!(
                        "¿Eliminar el script «{name}»? El historial de benchmarks se conserva."
                    ),
                    Action::DeleteScript(rest),
                );
            }
        }
    }
}

pub(crate) fn new_script(s: &mut Store) {
    s.library.draft = Some(Draft {
        id: None,
        name: String::new(),
        script: plane_llama_core::models::SCRIPT_TEMPLATE.into(),
    });
}

fn toolbar(ui: &mut Ui, s: &mut Store) {
    // En anchos estrechos el buscador encoge y los botones quedan solo con icono.
    let narrow = ui.available_width() < Tokens::TOOLBAR_FULL;
    ui.horizontal(|ui| {
        widgets::search(
            ui,
            &mut s.library.search,
            "Buscar scripts",
            if narrow {
                Tokens::SEARCH_NARROW
            } else {
                Tokens::SEARCH_WIDTH
            },
        );
        let mut filter = s.library.filter;
        if widgets::segmented(
            ui,
            &mut filter,
            &[
                (ScriptFilter::Visible, "Visibles"),
                (ScriptFilter::Hidden, "Ocultos"),
                (ScriptFilter::All, "Todos"),
            ],
        ) {
            s.library.filter = filter;
        }
        let label = SORTS
            .iter()
            .find(|(v, _)| *v == s.library.sort)
            .map_or("Recientes", |(_, l)| l);
        let sort = if narrow {
            widgets::icon_button(ui, icons::SORT_ASCENDING, "Ordenar")
        } else {
            widgets::button(ui, Kind::Secondary, Some(icons::SORT_ASCENDING), label)
                .on_hover_text("Ordenar")
        };
        egui::Popup::menu(&sort)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.set_min_width(Tokens::COLUMN_WIDTH + Tokens::S8);
                for (value, text) in SORTS {
                    if ui.selectable_label(s.library.sort == value, text).clicked() {
                        s.library.sort = value;
                    }
                }
            });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if narrow { "Nuevo" } else { "Nuevo script" };
            if widgets::button(ui, Kind::Primary, Some(icons::PLUS), label)
                .on_hover_text("Nuevo script")
                .clicked()
            {
                new_script(s);
            }
        });
    });
}

/// Barra con los scripts marcados, igual que la del historial: marcar todos, eliminarlos
/// y quitar la selección.
fn selection_bar(ui: &mut Ui, s: &Store, listed: Vec<String>) -> Option<RowAction> {
    let t = Tokens::get(ui.ctx());
    let mut action = None;
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
                    RichText::new(format!("{} seleccionados", s.library.selected.len()))
                        .font(theme::font_semibold(Tokens::FONT_BODY))
                        .color(t.accent_ink),
                );
                ui.add_space(Tokens::S2);
                let all = listed.iter().all(|id| s.library.selected.contains(id));
                if widgets::enabled(ui, !all, |ui| {
                    widgets::button(
                        ui,
                        Kind::Secondary,
                        Some(icons::CHECK_SQUARE),
                        "Marcar todos",
                    )
                })
                .clicked()
                {
                    action = Some(RowAction::SelectAll(listed.clone(), true));
                }
                if widgets::button(
                    ui,
                    Kind::Danger,
                    Some(icons::TRASH),
                    "Eliminar seleccionados",
                )
                .clicked()
                {
                    action = Some(RowAction::DeleteSelected);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(ui, icons::X, "Quitar selección").clicked() {
                        action = Some(RowAction::SelectAll(Vec::new(), false));
                    }
                });
            });
        });
    action
}

/// Benchmark en curso o último resultado, encima de la lista.
fn activity(ui: &mut Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    if let Some(started) = s.bench_started {
        widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                widgets::strong(
                    ui,
                    format!("Benchmark en curso · {} s", started.elapsed().as_secs()),
                );
                widgets::dim(ui, &s.bench_state);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::button(ui, Kind::Danger, Some(icons::STOP), "Detener benchmark")
                        .clicked()
                    {
                        s.cancel_benchmark();
                    }
                });
            });
        });
        ui.add_space(Tokens::S4);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
        return;
    }
    let Some(r) = s.last_result.as_ref().filter(|_| s.result_banner) else {
        return;
    };
    let mut close = false;
    let mut detail = false;
    let full = ui.available_width() >= Tokens::TOOLBAR_FULL;
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, icons::GAUGE, t.ok_soft, t.ok);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                widgets::strong(ui, "Último benchmark");
                widgets::dim(ui, plane_llama_core::models::script_title(&r.config.script));
            });
            ui.add_space(Tokens::S4);
            for (label, value) in [
                (
                    "Generación",
                    format!("{} tok/s", fmt(r.generation_tokens_per_second, 1)),
                ),
                (
                    "Prompt",
                    format!("{} tok/s", fmt(r.prompt_tokens_per_second, 0)),
                ),
                (
                    "VRAM",
                    plane_llama_core::client::format::total_device_vram_txt(r) + " GB",
                ),
                ("RAM", format!("{} GB", fmt_gb(r.ram_used_mi_b, 1))),
            ]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| full || i % 2 == 0)
            .map(|(_, m)| m)
            {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    widgets::dim(ui, label);
                    ui.label(
                        RichText::new(value)
                            .font(egui::FontId::monospace(Tokens::FONT_MD))
                            .color(t.text),
                    );
                });
                ui.add_space(Tokens::S3);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = widgets::icon_button(ui, icons::X, "Cerrar resultado").clicked();
                detail = widgets::button(ui, Kind::Ghost, None, "Ver detalle").clicked();
            });
        });
        if !r.errors.is_empty() {
            widgets::banner(ui, crate::store::NoticeKind::Warning, &r.errors.join("; "));
        }
    });
    ui.add_space(Tokens::S4);
    if close {
        s.result_banner = false;
    }
    if detail {
        s.result_banner = false;
        s.simple = false;
        s.tab = TAB_BENCH;
    }
}

pub(crate) fn row(
    ui: &mut Ui,
    s: &Store,
    script: &SavedScript,
    is_running: bool,
    blocked: bool,
    place: Place,
) -> Option<RowAction> {
    let t = Tokens::get(ui.ctx());
    let meta = s.library.meta.get(&script.id).cloned().unwrap_or_default();
    let page = place == Place::Page;
    // En orden manual y sin búsqueda, las filas se arrastran por su asa.
    let reorder =
        page && s.library.sort == ScriptSort::Manual && s.library.search.trim().is_empty();
    let mut action = None;
    let (response, ()) = hover_row(ui, is_running, |ui| {
        if page {
            let mut marked = s.library.selected.contains(&script.id);
            let check = ui.checkbox(&mut marked, "");
            let name = format!("Seleccionar {}", script.name);
            check.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, marked, &name)
            });
            if check
                .on_hover_text("Marca varios scripts para eliminarlos juntos")
                .changed()
            {
                action = Some(RowAction::Select(script.id.clone(), marked));
            }
        }
        if reorder {
            let handle = egui::Id::new(("script-drag", &script.id));
            ui.dnd_drag_source(handle, script.id.clone(), |ui| {
                ui.label(
                    RichText::new(icons::DOTS_SIX_VERTICAL)
                        .font(theme::icon_font(Tokens::ICON))
                        .color(t.text_faint),
                );
            })
            .response
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text("Arrastra para cambiar el orden");
        }
        let (dot, hint) = if is_running {
            (t.ok, "Servidor corriendo con este script")
        } else if meta.downloaded {
            (t.accent, "Modelo descargado")
        } else {
            (
                t.text_faint,
                "Modelo sin descargar: llama-server lo baja al iniciar",
            )
        };
        widgets::status_dot(ui, dot);
        // Las celdas se colocan desde la derecha y el nombre ocupa lo que queda; en
        // ventanas estrechas se omiten las columnas menos importantes.
        let width = ui.available_width();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(Tokens::S1);
            if page {
                let menu = widgets::icon_button(ui, icons::DOTS_THREE, "Más acciones");
                egui::Popup::menu(&menu)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui| {
                        ui.set_min_width(Tokens::RATING_WIDTH + Tokens::S8 * 2.);
                        let id = script.id.clone();
                        let item = |ui: &mut Ui, icon, label| {
                            widgets::button(ui, Kind::Ghost, Some(icon), label).clicked()
                        };
                        if item(ui, icons::PENCIL_SIMPLE, "Editar") {
                            action = Some(RowAction::Edit(id.clone()));
                        }
                        if widgets::enabled(ui, !blocked, |ui| {
                            item(ui, icons::GAUGE, "Ejecutar benchmark")
                        }) {
                            action = Some(RowAction::Bench(id.clone()));
                        }
                        if item(ui, icons::CODE, "Abrir en el editor") {
                            action = Some(RowAction::Open(id.clone()));
                        }
                        if item(ui, icons::COPY, "Duplicar") {
                            action = Some(RowAction::Duplicate(id.clone()));
                        }
                        if item(
                            ui,
                            icons::STAR,
                            if script.favorite {
                                "Quitar de favoritos"
                            } else {
                                "Marcar como favorito"
                            },
                        ) {
                            action = Some(RowAction::Favorite(id.clone()));
                        }
                        widgets::separator(ui);
                        if widgets::button(ui, Kind::Danger, Some(icons::TRASH), "Eliminar")
                            .clicked()
                        {
                            action = Some(RowAction::Delete(id));
                        }
                    });
                // Visible u oculto en la vista compacta y en el filtro «Visibles».
                let (icon, hint) = if script.hidden {
                    (icons::EYE_SLASH, "Oculto: mostrar en la lista")
                } else {
                    (icons::EYE, "Visible: ocultar de la lista")
                };
                if widgets::icon_button(ui, icon, hint).clicked() {
                    action = Some(RowAction::Hide(script.id.clone(), !script.hidden));
                }
            }
            // En filas estrechas (la ventana compacta) el botón queda solo con su icono.
            let narrow = width < Tokens::ROW_NARROW;
            let act = |ui: &mut Ui, kind, icon, label| {
                if narrow {
                    widgets::icon_action(ui, kind, icon, label)
                } else {
                    widgets::button(ui, kind, Some(icon), label)
                }
            };
            let action_slot = if narrow {
                Tokens::CONTROL
            } else {
                Tokens::ACTION_SLOT
            };
            slot(ui, action_slot, |ui| {
                if is_running {
                    if widgets::enabled(ui, !s.manual_pending, |ui| {
                        act(ui, Kind::Danger, icons::STOP, "Detener")
                    })
                    .clicked()
                    {
                        action = Some(RowAction::Stop);
                    }
                } else if widgets::enabled(ui, !blocked, |ui| {
                    act(ui, Kind::Secondary, icons::PLAY, "Iniciar")
                })
                .on_disabled_hover_text("Detén el servidor o el benchmark en curso primero")
                .clicked()
                {
                    action = Some(RowAction::Start(script.id.clone()));
                }
            });
            if is_running
                && let Some(url) = s.status.url.clone()
                && widgets::icon_button(
                    ui,
                    icons::CHAT_CIRCLE_TEXT,
                    "Abrir el chat de llama-server",
                )
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
            if width > Tokens::ROW_WIDE {
                slot(ui, Tokens::SIZE_WIDTH, |ui| {
                    ui.label(
                        RichText::new(meta.size_bytes.map(bytes).unwrap_or_default())
                            .font(egui::FontId::monospace(Tokens::FONT_SM))
                            .color(t.text_dim),
                    );
                });
            }
            if width > Tokens::ROW_TPS {
                slot(ui, Tokens::TPS_SLOT, |ui| match meta.best_tps {
                    Some(tps) => {
                        widgets::best(ui, &format!("{} tok/s", fmt(Some(tps), 1)))
                            .on_hover_text(format!("Mejor generación en {} benchmarks", meta.runs));
                    }
                    None => {
                        widgets::chip(ui, "sin benchmark", t.surface_alt, t.text_faint);
                    }
                });
            }
            if width > Tokens::ROW_WIDER {
                slot(ui, Tokens::CHIP_SLOT, |ui| {
                    if let Some(ctx) = meta.ctx {
                        widgets::chip(
                            ui,
                            &format!("{}K ctx", fmt(Some(ctx / 1024.), 0)),
                            t.surface_alt,
                            t.text_dim,
                        );
                    }
                });
                slot(ui, Tokens::CHIP_SLOT, |ui| {
                    if let Some(p) = &meta.params {
                        widgets::chip(ui, p, t.surface_alt, t.text_dim);
                    }
                });
            }
            if width > Tokens::ROW_TPS {
                slot(ui, Tokens::ICON_SLOT, |ui| {
                    if meta.vision {
                        ui.label(
                            RichText::new(icons::IMAGE)
                                .font(theme::icon_font(Tokens::ICON))
                                .color(t.accent_ink),
                        )
                        .on_hover_text("Multimodal: carga el mmproj");
                    }
                });
            }
            if width > Tokens::ROW_QUANT {
                slot(ui, Tokens::QUANT_SLOT, |ui| {
                    if let Some(q) = &meta.quant {
                        widgets::chip(ui, q, t.surface_alt, t.text);
                    }
                });
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(&script.name)
                            .font(theme::font_medium(Tokens::FONT_BODY))
                            .color(if script.hidden { t.text_dim } else { t.text }),
                    )
                    .truncate()
                    .selectable(false),
                )
                .on_hover_text(name_hint(&meta, &script.name, hint, width));
            });
        });
    });
    if reorder {
        // Línea de inserción arriba o abajo de la fila, según la mitad donde esté el
        // puntero; así también se puede soltar al final de la lista.
        let r = response.rect;
        let after = ui
            .ctx()
            .pointer_interact_pos()
            .is_some_and(|p| p.y > r.center().y);
        if response.dnd_hover_payload::<String>().is_some() {
            ui.painter().hline(
                r.x_range(),
                if after { r.bottom() } else { r.top() },
                egui::Stroke::new(Tokens::BORDER * 2., t.accent),
            );
        }
        if let Some(id) = response.dnd_release_payload::<String>() {
            action = Some(RowAction::Move((*id).clone(), script.id.clone(), after));
        }
    }
    if response.double_clicked() && action.is_none() {
        action = Some(RowAction::Edit(script.id.clone()));
    }
    action
}

/// Dónde se pinta una fila: la página Scripts (con menú, visibilidad y arrastre) o la
/// vista compacta (solo arrancar y detener).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Page,
    Compact,
}

/// Tooltip del nombre: repo y estado, más la cuantización y los tok/s cuando la fila es
/// tan estrecha que no los muestra.
fn name_hint(meta: &ScriptMeta, name: &str, hint: &str, width: f32) -> String {
    let mut text = format!("{}\n{hint}", meta.repo.as_deref().unwrap_or(name));
    if width <= Tokens::ROW_QUANT
        && let Some(q) = &meta.quant
    {
        text.push_str(&format!("\n{q}"));
    }
    if width <= Tokens::ROW_TPS
        && let Some(tps) = meta.best_tps
    {
        text.push_str(&format!("\nMejor generación: {} tok/s", fmt(Some(tps), 1)));
    }
    text
}

/// Filas del editor del diálogo: menos en ventanas bajas, como la compacta.
fn editor_rows(ctx: &egui::Context) -> usize {
    let free = ctx.content_rect().height() - Tokens::DIALOG_RESERVED * 1.5;
    ((free / Tokens::LOG_ROW) as usize).clamp(4, Tokens::EDITOR_ROWS - 2)
}

/// Diálogo para crear o editar un script de la biblioteca.
pub fn editor_dialog(ctx: &egui::Context, s: &mut Store) {
    let Some(mut draft) = s.library.draft.take() else {
        return;
    };
    let t = Tokens::get(ctx);
    let title = if draft.id.is_some() {
        "Editar script"
    } else {
        "Nuevo script"
    };
    let mut keep = true;
    let mut save = false;
    let mut to_hub = false;
    let dialog = widgets::dialog(ctx, "script-draft", title, Tokens::DIALOG_XL, |ui| {
        // Los campos se desplazan y el pie queda siempre visible, también en la
        // ventana compacta.
        let fields = ctx.content_rect().height()
            - Tokens::DIALOG_CHROME
            - Tokens::S5 * 2.
            - Tokens::DIALOG_FOOTER;
        egui::ScrollArea::vertical()
            .id_salt("script-draft-fields")
            .max_height(fields.max(Tokens::ROW))
            .show(ui, |ui| {
                widgets::field_label(ui, "Nombre", Some("vacío = nombre del modelo"));
                ui.add(
                    egui::TextEdit::singleline(&mut draft.name)
                        .margin(egui::Margin::symmetric(Tokens::S3 as i8, Tokens::S2 as i8))
                        .hint_text(plane_llama_core::models::script_title(&draft.script))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(Tokens::S3);
                ui.horizontal(|ui| {
                    widgets::field_label(ui, "Script", None);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pick = widgets::button(
                            ui,
                            Kind::Secondary,
                            Some(icons::CUBE),
                            "Elegir modelo descargado",
                        );
                        match models_page::picker(ui, s, &pick) {
                            Some(models_page::Picked::Model(hf)) => {
                                draft.script =
                                    plane_llama_core::models::script_with_model(&draft.script, &hf);
                            }
                            Some(models_page::Picked::Hub) => to_hub = true,
                            None => {}
                        }
                    });
                });
                widgets::sunken(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut draft.script)
                            .code_editor()
                            .frame(egui::Frame::new())
                            .desired_rows(editor_rows(ui.ctx()))
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(Tokens::S1);
                script_editor::summary(ui, &draft.script);
                if draft.script.trim().is_empty() {
                    ui.label(
                        RichText::new("El script no puede estar vacío.")
                            .font(theme::font(Tokens::FONT_SM))
                            .color(t.error),
                    );
                }
            });
        widgets::dialog_footer(ui, |ui| {
            if widgets::enabled(ui, !draft.script.trim().is_empty(), |ui| {
                widgets::button(
                    ui,
                    Kind::Primary,
                    Some(icons::FLOPPY_DISK),
                    "Guardar script",
                )
            })
            .clicked()
            {
                save = true;
            }
            if widgets::button(ui, Kind::Secondary, None, "Cancelar").clicked() {
                keep = false;
            }
        });
    });
    if save {
        s.library.commit(draft);
        s.save_library();
        s.notice("Script guardado");
        return;
    }
    // «Buscar en Hugging Face» desde el selector cierra el diálogo y abre el Hub.
    if keep && !dialog.close && !to_hub {
        s.library.draft = Some(draft);
    }
}
