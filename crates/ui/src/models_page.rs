// Modelos: los GGUF descargados (para usarlos en scripts) y la búsqueda en Hugging Face
// con sus cuantizaciones, tamaños y descarga.
use crate::{
    bridge::Action,
    gpu_grid,
    library::{Download, Draft, ModelsView, download_key},
    store::{Store, TAB_MODELS},
    theme::{self, Tokens},
    widgets::{self, Kind, icons},
};
use egui::{Align, Layout, RichText, Sense, Ui, vec2};
use plane_llama_core::{
    client::format::{fmt, parse_model},
    hub::{HubModel, HubSort},
    models::{LocalModel, script_with_model},
};

const MIB: f64 = 1024. * 1024.;

pub fn bytes(n: u64) -> String {
    let gb = n as f64 / 1e9;
    if gb >= 10. {
        format!("{gb:.0} GB")
    } else if gb >= 1. {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", n as f64 / 1e6)
    }
}
fn count(n: u64) -> String {
    match n {
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1e6),
        n if n >= 1_000 => format!("{:.1}K", n as f64 / 1e3),
        n => n.to_string(),
    }
}
/// «hace 3 h», «hace 2 d»… a partir de una fecha ISO del Hub.
fn ago(iso: Option<&str>) -> String {
    let Some(t) = iso.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) else {
        return String::new();
    };
    let mins = (chrono::Utc::now() - t.with_timezone(&chrono::Utc))
        .num_minutes()
        .max(0);
    match mins {
        m if m < 60 => format!("hace {m} min"),
        m if m < 60 * 24 => format!("hace {} h", m / 60),
        m if m < 60 * 24 * 31 => format!("hace {} d", m / 60 / 24),
        m if m < 60 * 24 * 365 => format!("hace {} meses", m / 60 / 24 / 30),
        m => format!("hace {} años", m / 60 / 24 / 365),
    }
}

/// Si un modelo cabe en la VRAM total, necesita RAM o no cabe.
#[derive(Clone, Copy, PartialEq)]
pub enum Fit {
    Vram,
    Ram,
    No,
    Unknown,
}
pub fn fit(s: &Store, size: u64) -> Fit {
    let (vram, ram) = gpu_grid::capacity(s);
    let mib = size as f64 / MIB;
    if vram <= 0. {
        Fit::Unknown
    } else if mib <= vram * 0.9 {
        Fit::Vram
    } else if mib <= (vram + ram) * 0.8 {
        Fit::Ram
    } else {
        Fit::No
    }
}
fn fit_dot(ui: &mut Ui, f: Fit) {
    let t = Tokens::get(ui.ctx());
    let (color, text) = match f {
        Fit::Vram => (t.ok, "Cabe en la VRAM"),
        Fit::Ram => (t.warning, "Necesita parte de la RAM (más lento)"),
        Fit::No => (t.error, "No cabe en VRAM + RAM"),
        Fit::Unknown => (t.text_faint, "Sin datos de VRAM"),
    };
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(Tokens::S3), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), Tokens::DOT, color);
    response.on_hover_text(text);
}
fn mono_chip(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    widgets::chip(ui, text, t.surface_alt, t.text_dim);
}
fn group_header(ui: &mut Ui, text: &str, count: usize) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        widgets::section_label(ui, text);
        ui.label(
            RichText::new(count.to_string())
                .font(egui::FontId::monospace(Tokens::FONT_XS))
                .color(t.text_faint),
        );
    });
}

/// Celda de ancho fijo alineada a la derecha, para que las columnas de las filas cuadren.
pub fn slot(ui: &mut Ui, width: f32, body: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        vec2(width, Tokens::CONTROL),
        Layout::right_to_left(Align::Center),
        |ui| {
            ui.set_width(width);
            body(ui);
        },
    );
}
/// Fila de altura fija con fondo al pasar el ratón; el fondo se pinta detrás del contenido.
pub fn hover_row<R>(
    ui: &mut Ui,
    selected: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> (egui::Response, R) {
    row_with_height(ui, selected, Tokens::ROW_LG, body)
}
pub fn row_with_height<R>(
    ui: &mut Ui,
    selected: bool,
    height: f32,
    body: impl FnOnce(&mut Ui) -> R,
) -> (egui::Response, R) {
    let t = Tokens::get(ui.ctx());
    let bg = ui.painter().add(egui::Shape::Noop);
    let rect = egui::Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), height));
    let inner = ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center))
            .sense(Sense::click()),
        |ui| {
            ui.set_min_size(rect.size());
            widgets::no_select(ui);
            ui.add_space(Tokens::S2);
            body(ui)
        },
    );
    let response = inner.response;
    let fill = if selected {
        Some(t.accent_soft)
    } else if response.hovered() || response.contains_pointer() {
        Some(t.hover)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().set(
            bg,
            egui::epaint::RectShape::filled(response.rect, Tokens::RADIUS_MD, fill),
        );
    }
    (response, inner.inner)
}

const HUB_SORTS: [(HubSort, &str); 4] = [
    (HubSort::Recent, "Recientes"),
    (HubSort::Trending, "Tendencia"),
    (HubSort::Downloads, "Descargas"),
    (HubSort::Likes, "Likes"),
];

pub fn show(ui: &mut Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    // En anchos estrechos el orden pasa a un menú y se omite la capacidad.
    let narrow = ui.available_width() < Tokens::TOOLBAR_FULL + Tokens::HUB_LIST / 2.;
    ui.horizontal(|ui| {
        let local = format!("En el equipo · {}", s.hub.local.len());
        let mut view = s.hub.view;
        if widgets::segmented(
            ui,
            &mut view,
            &[
                (ModelsView::Local, local.as_str()),
                (ModelsView::Hub, "Hugging Face"),
            ],
        ) {
            s.hub.view = view;
        }
        ui.add_space(Tokens::S2);
        match s.hub.view {
            ModelsView::Local => {
                widgets::search(
                    ui,
                    &mut s.hub.search,
                    "Filtrar modelos",
                    Tokens::SEARCH_WIDTH,
                );
            }
            ModelsView::Hub => {
                let before = s.hub.hub_query.clone();
                let r = widgets::search(
                    ui,
                    &mut s.hub.hub_query,
                    if narrow {
                        "Buscar en Hugging Face"
                    } else {
                        "Buscar en Hugging Face (qwen3, bartowski/llama…)"
                    },
                    if narrow {
                        Tokens::SEARCH_WIDTH
                    } else {
                        Tokens::SEARCH_WIDTH + Tokens::S8 * 3.
                    },
                );
                if s.hub.hub_query != before {
                    s.hub.hub_edited = Some(std::time::Instant::now());
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    s.search_hub();
                }
                let mut sort = s.hub.hub_sort;
                if narrow {
                    let button = widgets::icon_button(ui, icons::SORT_ASCENDING, "Ordenar");
                    egui::Popup::menu(&button)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                        .show(|ui| {
                            ui.set_min_width(Tokens::COLUMN_WIDTH + Tokens::S8);
                            for (value, text) in HUB_SORTS {
                                if ui.selectable_label(sort == value, text).clicked() {
                                    sort = value;
                                }
                            }
                        });
                } else {
                    widgets::segmented(ui, &mut sort, &HUB_SORTS);
                }
                if sort != s.hub.hub_sort {
                    s.hub.hub_sort = sort;
                    s.search_hub();
                }
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::icon_button(ui, icons::ARROWS_CLOCKWISE, "Recargar modelos").clicked() {
                s.actions.push(Action::LocalModels);
                if s.hub.view == ModelsView::Hub {
                    s.hub.hub_sent = None;
                    s.search_hub();
                }
            }
            let (vram, ram) = gpu_grid::capacity(s);
            if vram > 0. && !narrow {
                ui.label(
                    RichText::new(format!(
                        "{} GB VRAM · {} GB RAM",
                        fmt(Some(vram / 1024.), 0),
                        fmt(Some(ram / 1024.), 0)
                    ))
                    .font(egui::FontId::monospace(Tokens::FONT_XS))
                    .color(t.text_dim),
                );
            }
        });
    });
    ui.add_space(Tokens::S4);
    downloads(ui, s);
    match s.hub.view {
        ModelsView::Local => local(ui, s),
        ModelsView::Hub => hub(ui, s),
    }
}

fn downloads(ui: &mut Ui, s: &mut Store) {
    if s.hub.downloads.is_empty() {
        return;
    }
    let mut cancel = None;
    widgets::card(ui, |ui| {
        widgets::section_label(ui, "Descargando");
        for (key, d) in &s.hub.downloads {
            if download_row(ui, d) {
                cancel = Some(key.clone());
            }
        }
    });
    ui.add_space(Tokens::S4);
    if let Some(key) = cancel {
        s.actions.push(Action::CancelDownload(key));
    }
}
/// Nombre, cuantización, avance y barra de una descarga. `true` si se pidió cancelarla.
fn download_row(ui: &mut Ui, d: &Download) -> bool {
    let mut cancel = false;
    ui.horizontal(|ui| {
        widgets::strong(ui, d.repo.rsplit('/').next().unwrap_or(&d.repo));
        if !d.quant.is_empty() {
            mono_chip(ui, &d.quant);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            cancel = widgets::icon_button(ui, icons::X, "Cancelar descarga").clicked();
            widgets::dim(ui, download_text(d));
        });
    });
    widgets::meter(ui, d.done as f64, d.total as f64, Tokens::BAR);
    cancel
}
fn download_text(d: &Download) -> String {
    if d.total == 0 {
        return "buscando en Hugging Face…".into();
    }
    let secs = d.started.elapsed().as_secs_f64().max(1.);
    let speed = d.done as f64 / secs;
    let left = if speed > 0. {
        format!(
            " · faltan {} min",
            ((d.total.saturating_sub(d.done)) as f64 / speed / 60.).ceil()
        )
    } else {
        String::new()
    };
    format!(
        "{} / {} · {}/s{left}",
        bytes(d.done),
        bytes(d.total),
        bytes(speed as u64)
    )
}
/// Descarga del modelo de un script antes de arrancarlo, con su barra y cancelación.
pub fn fetch_progress(ui: &mut Ui, s: &mut Store) {
    let Some(key) = s.fetching.as_ref().map(|(k, _)| k.clone()) else {
        return;
    };
    let Some(d) = s.hub.downloads.get(&key) else {
        return;
    };
    let mut cancel = false;
    widgets::card(ui, |ui| {
        widgets::section_label(ui, "Descargando el modelo del script");
        cancel = download_row(ui, d);
    });
    ui.add_space(Tokens::S4);
    if cancel {
        s.actions.push(Action::CancelDownload(key));
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(200));
}

/// Pregunta si descargar el modelo que falta antes de arrancar o medir un script.
pub fn fetch_dialog(ctx: &egui::Context, s: &mut Store) {
    let Some((hf, _, action)) = &s.ask_fetch else {
        return;
    };
    let t = Tokens::get(ctx);
    let then = if matches!(action, Action::Benchmark(_)) {
        "medirá el script"
    } else {
        "arrancará el servidor"
    };
    let text = format!(
        "{hf} no está descargado. ¿Descargarlo ahora de Hugging Face? Al terminar se {then}."
    );
    let mut answer = None;
    let dialog = widgets::dialog(
        ctx,
        "fetch-model",
        "Modelo no descargado",
        Tokens::DIALOG_SM,
        |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(theme::font(Tokens::FONT_BODY))
                        .color(t.text),
                )
                .wrap(),
            );
            widgets::dialog_footer(ui, |ui| {
                if widgets::button(ui, Kind::Primary, Some(icons::DOWNLOAD_SIMPLE), "Descargar")
                    .clicked()
                {
                    answer = Some(true);
                }
                if widgets::button(ui, Kind::Secondary, None, "Cancelar").clicked() {
                    answer = Some(false);
                }
            });
        },
    );
    match answer {
        Some(true) => s.accept_fetch(),
        Some(false) => s.decline_fetch(),
        None if dialog.close => s.decline_fetch(),
        None => {}
    }
}

/// Acciones sobre un modelo local: crear un script nuevo o usarlo en el del editor.
fn use_model(s: &mut Store, hf: &str, new_script: bool) {
    if new_script {
        let base = if s.script.trim().is_empty() {
            String::new()
        } else {
            s.script.clone()
        };
        s.library.draft = Some(Draft {
            id: None,
            name: String::new(),
            script: script_with_model(&base, hf),
        });
    } else {
        s.script = script_with_model(&s.script, hf);
        s.script_seeded = true;
        s.notice(format!("Modelo {hf} en el script del editor"));
    }
}

fn local(ui: &mut Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    if !s.hub.local_loaded {
        ui.spinner();
        return;
    }
    if s.hub.local.is_empty() {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::CUBE,
                "No hay modelos GGUF descargados",
                "Busca uno en Hugging Face y descárgalo; quedará en la caché que usa llama-server -hf.",
            );
            ui.vertical_centered(|ui| {
                if widgets::button(
                    ui,
                    Kind::Primary,
                    Some(icons::MAGNIFYING_GLASS),
                    "Buscar en Hugging Face",
                )
                .clicked()
                {
                    s.hub.view = ModelsView::Hub;
                }
            });
        });
        return;
    }
    let groups: Vec<(String, Vec<LocalModel>)> = s
        .hub
        .local_groups(&s.hub.search)
        .into_iter()
        .map(|(g, v)| (g, v.into_iter().cloned().collect()))
        .collect();
    let mut action = None;
    let mut delete = None;
    egui::ScrollArea::vertical()
        .id_salt("local-models")
        .auto_shrink(false)
        .show(ui, |ui| {
            widgets::card(ui, |ui| {
                if groups.is_empty() {
                    widgets::dim(ui, "Ningún modelo coincide con el filtro.");
                }
                for (i, (org, models)) in groups.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(Tokens::S2);
                        widgets::separator(ui);
                        ui.add_space(Tokens::S2);
                    }
                    group_header(ui, org, models.len());
                    for m in models {
                        let (_, ()) = hover_row(ui, false, |ui| {
                            fit_dot(ui, fit(s, m.size_bytes));
                            ui.label(
                                RichText::new(format!("{}/", m.org()))
                                    .font(theme::font(Tokens::FONT_BODY))
                                    .color(t.text_faint),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(m.name())
                                        .font(theme::font_medium(Tokens::FONT_BODY))
                                        .color(t.text),
                                )
                                .truncate(),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(Tokens::S1);
                                if widgets::button(
                                    ui,
                                    Kind::Secondary,
                                    Some(icons::PLUS),
                                    "Crear script",
                                )
                                .clicked()
                                {
                                    action = Some((m.hf_ref(), true));
                                }
                                if widgets::icon_button(
                                    ui,
                                    icons::ARROW_SQUARE_IN,
                                    "Usar en el script del editor",
                                )
                                .clicked()
                                {
                                    action = Some((m.hf_ref(), false));
                                }
                                if widgets::icon_button(ui, icons::COPY, "Copiar -hf").clicked() {
                                    ui.ctx().copy_text(m.hf_ref());
                                }
                                if widgets::icon_button(ui, icons::TRASH, "Eliminar del disco")
                                    .clicked()
                                {
                                    delete = Some(m.clone());
                                }
                                ui.allocate_ui(vec2(Tokens::SIZE_WIDTH, Tokens::CONTROL), |ui| {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(bytes(m.size_bytes))
                                                .font(egui::FontId::monospace(Tokens::FONT_SM))
                                                .color(t.text_dim),
                                        );
                                    });
                                });
                                if let Some(size) =
                                    parse_model(Some(&m.hf_ref())).and_then(|p| p.size)
                                {
                                    mono_chip(ui, &size);
                                }
                                if !m.mmproj.is_empty() {
                                    ui.label(
                                        RichText::new(icons::IMAGE)
                                            .font(theme::icon_font(Tokens::ICON))
                                            .color(t.accent_ink),
                                    )
                                    .on_hover_text("Multimodal: incluye mmproj");
                                }
                                if m.files.len() > 1 {
                                    mono_chip(ui, &format!("{} partes", m.files.len()));
                                }
                                widgets::chip(ui, &m.quant, t.surface_alt, t.text);
                            });
                        });
                    }
                }
            });
        });
    if let Some((hf, new_script)) = action {
        use_model(s, &hf, new_script);
    }
    if let Some(m) = delete {
        delete_model(s, &m);
    }
}

/// Pide confirmación para borrar el modelo del disco. Si el servidor lo está usando,
/// avisa en su lugar.
fn delete_model(s: &mut Store, m: &LocalModel) {
    let hf = m.hf_ref();
    let in_use = s.running()
        && s.library
            .started
            .as_deref()
            .is_some_and(|script| script.to_lowercase().contains(&hf.to_lowercase()));
    if in_use {
        s.notice(format!("Detén el servidor antes de eliminar {hf}."));
        return;
    }
    s.confirm(
        format!(
            "¿Eliminar {hf} del disco? Se borran {} de la caché de Hugging Face. Los scripts que lo usan se conservan; llama-server lo volverá a descargar al arrancarlos.",
            bytes(m.size_bytes)
        ),
        Action::DeleteModel(m.repo.clone(), m.quant.clone()),
    );
}

fn hub(ui: &mut Ui, s: &mut Store) {
    // Busca tras una pausa al escribir; la primera vez, los GGUF de cualquier autor en el
    // orden elegido.
    if let Some(edited) = s.hub.hub_edited {
        let wait = std::time::Duration::from_millis(450);
        if edited.elapsed() >= wait {
            s.search_hub();
        } else {
            ui.ctx().request_repaint_after(wait - edited.elapsed());
        }
    } else if s.hub.hub_sent.is_none() {
        s.search_hub();
    }
    let wide = ui.available_width() >= Tokens::TWO_COLUMNS;
    if wide {
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = Tokens::S4;
            ui.allocate_ui_with_layout(
                vec2(Tokens::HUB_LIST, height),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(Tokens::HUB_LIST);
                    results(ui, s, height);
                },
            );
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), height),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("hub-details")
                        .auto_shrink(false)
                        .show(ui, |ui| details(ui, s));
                },
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("hub-stack")
            .auto_shrink(false)
            .show(ui, |ui| {
                results(ui, s, Tokens::CHART_HEIGHT);
                ui.add_space(Tokens::S4);
                details(ui, s);
            });
    }
}

fn avatar(ui: &mut Ui, author: &str, size: f32) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), Sense::hover());
    ui.painter()
        .rect_filled(rect, Tokens::RADIUS_MD, t.accent_soft);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        author
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default(),
        theme::font_semibold(size * 0.45),
        t.accent_ink,
    );
}

fn results(ui: &mut Ui, s: &mut Store, height: f32) {
    let t = Tokens::get(ui.ctx());
    let title = if s.hub.hub_query.trim().is_empty() {
        "Modelos GGUF de Hugging Face"
    } else {
        "Resultados"
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme::font_semibold(Tokens::FONT_MD))
                .color(t.text),
        );
        if s.hub.hub_loading {
            ui.spinner();
        }
    });
    ui.add_space(Tokens::S2);
    if let Some(e) = &s.hub.hub_error {
        widgets::banner(
            ui,
            crate::store::NoticeKind::Error,
            &format!("No se pudo consultar Hugging Face: {e}"),
        );
        return;
    }
    if !s.hub.hub_loading && s.hub.hub_results.is_empty() {
        widgets::dim(ui, "Sin resultados con GGUF para esa búsqueda.");
        return;
    }
    let mut pick = None;
    egui::ScrollArea::vertical()
        .id_salt("hub-results")
        .max_height(height - Tokens::S8)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for m in &s.hub.hub_results {
                let selected = s.hub.selected.as_deref() == Some(m.id.as_str());
                let (response, ()) =
                    row_with_height(ui, selected, Tokens::ROW_HUB, |ui| result_row(ui, m));
                if response.clicked() {
                    pick = Some(m.id.clone());
                }
            }
        });
    if let Some(id) = pick {
        s.select_hub_model(id);
    }
}
fn result_row(ui: &mut Ui, m: &HubModel) {
    let t = Tokens::get(ui.ctx());
    avatar(ui, m.author(), Tokens::CONTROL);
    ui.add_space(Tokens::S1);
    widgets::middle(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.;
        ui.add(
            egui::Label::new(
                RichText::new(m.name())
                    .font(theme::font_medium(Tokens::FONT_BODY))
                    .color(t.text),
            )
            .truncate()
            .selectable(false),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Tokens::S1;
            // Los iconos van con su propia fuente: Inter ocupa los mismos códigos.
            for (icon, text) in [
                (None, m.author().to_owned()),
                (Some(icons::HEART), count(m.likes)),
                (Some(icons::DOWNLOAD_SIMPLE), count(m.downloads)),
                (None, ago(m.last_modified.as_deref())),
            ] {
                if let Some(icon) = icon {
                    ui.add_space(Tokens::S1);
                    ui.label(
                        RichText::new(icon)
                            .font(theme::icon_font(Tokens::FONT_XS))
                            .color(t.text_faint),
                    );
                }
                ui.label(
                    RichText::new(text)
                        .font(theme::font(Tokens::FONT_XS + 0.5))
                        .color(t.text_dim),
                );
                ui.add_space(Tokens::S1);
            }
        });
    });
}

fn pipeline_label(tag: &str) -> &str {
    match tag {
        "text-generation" => "Generación de texto",
        "image-text-to-text" => "Imagen y texto",
        "automatic-speech-recognition" => "Voz a texto",
        "text-to-speech" => "Texto a voz",
        "feature-extraction" => "Embeddings",
        "text-to-image" => "Texto a imagen",
        other => other,
    }
}

fn details(ui: &mut Ui, s: &mut Store) {
    let t = Tokens::get(ui.ctx());
    let Some(id) = s.hub.selected.clone() else {
        widgets::card(ui, |ui| {
            widgets::empty_state(
                ui,
                icons::CUBE,
                "Elige un modelo",
                "Verás sus cuantizaciones, el tamaño de cada una y si cabe en tu VRAM.",
            );
        });
        return;
    };
    let mut download = None;
    let mut cancel = None;
    let mut create = None;
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            avatar(
                ui,
                id.split('/').next().unwrap_or(""),
                Tokens::CONTROL_LG + Tokens::S3,
            );
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = Tokens::S1 / 2.;
                ui.add(
                    egui::Label::new(
                        RichText::new(id.rsplit('/').next().unwrap_or(&id))
                            .font(theme::font_semibold(Tokens::FONT_XL))
                            .color(t.text),
                    )
                    .truncate(),
                );
                widgets::dim(ui, id.split('/').next().unwrap_or(""));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, icons::ARROW_SQUARE_OUT, "Abrir en Hugging Face")
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!(
                        "{}/{id}",
                        plane_llama_core::hub::HUB_URL
                    )));
                }
                if widgets::icon_button(ui, icons::COPY, "Copiar nombre").clicked() {
                    ui.ctx().copy_text(id.clone());
                }
            });
        });
        ui.add_space(Tokens::S3);
        if s.hub.details_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                widgets::dim(ui, "Leyendo cuantizaciones…");
            });
            return;
        }
        if let Some(e) = &s.hub.details_error {
            widgets::banner(ui, crate::store::NoticeKind::Error, e);
            return;
        }
        let Some(d) = s.hub.details.clone() else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            if let Some(tag) = &d.pipeline_tag {
                widgets::chip(ui, pipeline_label(tag), t.accent_soft, t.accent_ink);
            }
            for base in &d.base_models {
                widgets::chip(ui, &format!("Base · {base}"), t.surface_alt, t.text);
            }
            if !d.mmproj.is_empty() {
                widgets::chip(ui, "Visión (mmproj)", t.surface_alt, t.text);
            }
        });
        if d.gated {
            ui.add_space(Tokens::S2);
            widgets::banner(
                ui,
                crate::store::NoticeKind::Warning,
                "Repositorio con acceso restringido: acepta las condiciones en Hugging Face y define HF_TOKEN (o inicia sesión con `hf auth login`).",
            );
        }
        ui.add_space(Tokens::S3);
        ui.horizontal(|ui| {
            widgets::section_label(ui, "Pesos");
            if !d.mmproj.is_empty() {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    widgets::toggle(ui, &mut s.hub.with_mmproj, "Incluir mmproj al descargar");
                });
            }
        });
        if d.quants.is_empty() {
            widgets::dim(ui, "Este repositorio no tiene archivos GGUF.");
        }
        for q in &d.quants {
            let key = download_key(&d.id, &q.quant);
            let is_local = s.hub.is_local(&d.id, &q.quant);
            let progress = s.hub.downloads.get(&key).map(|p| (p.done, p.total));
            let busy = !s.hub.downloads.is_empty();
            hover_row(ui, false, |ui| {
                fit_dot(ui, fit(s, q.size));
                widgets::chip(ui, &q.quant, t.surface_alt, t.text);
                if q.files.len() > 1 {
                    mono_chip(ui, &format!("{} partes", q.files.len()));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(Tokens::S1);
                    if let Some((done, total)) = progress {
                        if widgets::icon_button(ui, icons::X, "Cancelar descarga").clicked() {
                            cancel = Some(key.clone());
                        }
                        widgets::dim(
                            ui,
                            format!("{:.0}%", done as f64 * 100. / total.max(1) as f64),
                        );
                        ui.allocate_ui(vec2(Tokens::COLUMN_WIDTH, Tokens::BAR), |ui| {
                            ui.set_width(Tokens::COLUMN_WIDTH);
                            widgets::meter(ui, done as f64, total as f64, Tokens::BAR);
                        });
                    } else if is_local {
                        if widgets::button(ui, Kind::Secondary, Some(icons::PLUS), "Crear script")
                            .clicked()
                        {
                            create = Some(format!("{}:{}", d.id, q.quant));
                        }
                        widgets::chip(ui, "En el equipo", t.ok_soft, t.ok);
                    } else if widgets::enabled(ui, !busy, |ui| {
                        widgets::button(
                            ui,
                            Kind::Secondary,
                            Some(icons::DOWNLOAD_SIMPLE),
                            "Descargar",
                        )
                    })
                    .on_disabled_hover_text("Espera a que termine la descarga en curso")
                    .clicked()
                    {
                        download = Some(q.quant.clone());
                    }
                    ui.allocate_ui(vec2(Tokens::SIZE_WIDTH, Tokens::CONTROL), |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(bytes(q.size))
                                    .font(egui::FontId::monospace(Tokens::FONT_SM))
                                    .color(t.text_dim),
                            );
                        });
                    });
                });
            });
        }
        ui.add_space(Tokens::S3);
        ui.horizontal_wrapped(|ui| {
            for (f, text) in [
                (Fit::Vram, "cabe en la VRAM"),
                (Fit::Ram, "usa RAM"),
                (Fit::No, "no cabe"),
            ] {
                fit_dot(ui, f);
                widgets::dim(ui, text);
                ui.add_space(Tokens::S2);
            }
        });
    });
    if let Some(q) = download {
        s.download(&q);
    }
    if let Some(key) = cancel {
        s.actions.push(Action::CancelDownload(key));
    }
    if let Some(hf) = create {
        use_model(s, &hf, true);
    }
}

pub enum Picked {
    /// `org/modelo:QUANT` elegido.
    Model(String),
    /// Se pidió buscar en Hugging Face (ya se cambió de página).
    Hub,
}
/// Menú desplegable con los modelos descargados.
pub fn picker(ui: &mut Ui, s: &mut Store, button: &egui::Response) -> Option<Picked> {
    let t = Tokens::get(ui.ctx());
    let mut picked = None;
    egui::Popup::menu(button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(Tokens::DIALOG_SM);
            widgets::search(
                ui,
                &mut s.hub.picker_search,
                "Buscar modelos descargados",
                Tokens::DIALOG_SM - Tokens::S6,
            );
            ui.add_space(Tokens::S2);
            let groups: Vec<(String, Vec<LocalModel>)> = s
                .hub
                .local_groups(&s.hub.picker_search)
                .into_iter()
                .map(|(g, v)| (g, v.into_iter().cloned().collect()))
                .collect();
            egui::ScrollArea::vertical()
                .max_height(Tokens::CHART_HEIGHT)
                .show(ui, |ui| {
                    if groups.is_empty() {
                        widgets::dim(ui, "No hay modelos descargados que coincidan.");
                    }
                    for (org, models) in &groups {
                        group_header(ui, org, models.len());
                        for m in models {
                            let (r, ()) = hover_row(ui, false, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(m.name())
                                            .font(theme::font_medium(Tokens::FONT_BODY))
                                            .color(t.text),
                                    )
                                    .truncate()
                                    .selectable(false),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.add_space(Tokens::S2);
                                    ui.label(
                                        RichText::new(bytes(m.size_bytes))
                                            .font(egui::FontId::monospace(Tokens::FONT_XS))
                                            .color(t.text_dim),
                                    );
                                    widgets::chip(ui, &m.quant, t.surface_alt, t.text);
                                });
                            });
                            if r.clicked() {
                                picked = Some(Picked::Model(m.hf_ref()));
                            }
                        }
                    }
                });
            ui.add_space(Tokens::S2);
            if widgets::button(
                ui,
                Kind::Ghost,
                Some(icons::MAGNIFYING_GLASS),
                "Buscar en Hugging Face",
            )
            .clicked()
            {
                s.tab = TAB_MODELS;
                s.hub.view = ModelsView::Hub;
                picked = Some(Picked::Hub);
            }
            if picked.is_some() {
                ui.close();
            }
        });
    picked
}
