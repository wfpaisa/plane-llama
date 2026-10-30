// Componentes de la interfaz: botones, tarjetas, campos, medidores, diálogos y avisos.
use crate::theme::{self, Tokens};
use egui::{
    Align, Align2, CursorIcon, Frame, Layout, Margin, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2,
};
pub use egui_phosphor::regular as icons;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

fn margin(n: f32) -> Margin {
    Margin::same(n as i8)
}

/// Botón pintado a mano: icono opcional, etiqueta accesible y estados de hover y foco.
fn draw_button(
    ui: &mut Ui,
    kind: Kind,
    icon: Option<&str>,
    label: &str,
    show_label: bool,
    min: Vec2,
) -> Response {
    let t = Tokens::get(ui.ctx());
    let enabled = ui.is_enabled();
    let painter = ui.painter().clone();
    let text = show_label.then(|| {
        painter.layout_no_wrap(
            label.to_owned(),
            theme::font_medium(Tokens::FONT_BODY),
            Tokens::INK,
        )
    });
    let glyph = icon
        .map(|i| painter.layout_no_wrap(i.to_owned(), theme::icon_font(Tokens::ICON), Tokens::INK));
    let gap = if text.is_some() && glyph.is_some() {
        Tokens::S2 - Tokens::BORDER * 2.
    } else {
        0.
    };
    let content =
        text.as_ref().map_or(0., |g| g.size().x) + glyph.as_ref().map_or(0., |g| g.size().x) + gap;
    let pad = if show_label { Tokens::S3 + 2. } else { 0. };
    let size = vec2((content + pad * 2.).max(min.x), min.y.max(Tokens::CONTROL));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered() && enabled;
        let (mut fill, mut stroke, mut ink) = match kind {
            Kind::Primary => (
                if hovered { t.accent_hover } else { t.accent },
                t.transparent,
                t.on_accent,
            ),
            Kind::Secondary => (
                if hovered { t.hover } else { t.surface_alt },
                if hovered { t.border_strong } else { t.border },
                t.text,
            ),
            Kind::Ghost => (
                if hovered { t.hover } else { t.transparent },
                t.transparent,
                if hovered { t.text } else { t.text_dim },
            ),
            Kind::Danger => (
                if hovered { t.error } else { t.error_soft },
                t.transparent,
                if hovered { t.on_accent } else { t.error },
            ),
        };
        if !enabled {
            fill = fill.gamma_multiply(0.45);
            stroke = stroke.gamma_multiply(0.45);
            ink = ink.gamma_multiply(0.45);
        }
        if response.has_focus() {
            stroke = t.accent;
        }
        painter.rect(
            rect,
            Tokens::RADIUS_MD,
            fill,
            Stroke::new(Tokens::BORDER, stroke),
            StrokeKind::Inside,
        );
        let mut x = rect.center().x - content / 2.;
        if let Some(g) = glyph {
            let width = g.size().x;
            painter.galley(pos2(x, rect.center().y - g.size().y / 2.), g, ink);
            x += width + gap;
        }
        if let Some(g) = text {
            painter.galley(pos2(x, rect.center().y - g.size().y / 2.), g, ink);
        }
    }
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn button(ui: &mut Ui, kind: Kind, icon: Option<&str>, label: &str) -> Response {
    draw_button(ui, kind, icon, label, true, Vec2::ZERO)
}
/// Botón a todo el ancho disponible, para la acción principal de una tarjeta.
pub fn button_wide(
    ui: &mut Ui,
    kind: Kind,
    icon: Option<&str>,
    label: &str,
    height: f32,
) -> Response {
    let width = ui.available_width();
    draw_button(ui, kind, icon, label, true, vec2(width, height))
}
/// Botón cuadrado de un tipo dado solo con icono, para filas estrechas.
pub fn icon_action(ui: &mut Ui, kind: Kind, icon: &str, label: &str) -> Response {
    draw_button(
        ui,
        kind,
        Some(icon),
        label,
        false,
        Vec2::splat(Tokens::CONTROL),
    )
    .on_hover_text(label)
}
/// Botón cuadrado solo con icono; la etiqueta queda como tooltip y nombre accesible.
pub fn icon_button(ui: &mut Ui, icon: &str, label: &str) -> Response {
    draw_button(
        ui,
        Kind::Ghost,
        Some(icon),
        label,
        false,
        Vec2::splat(Tokens::CONTROL),
    )
    .on_hover_text(label)
}
pub fn enabled<R>(ui: &mut Ui, enabled: bool, body: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_enabled_ui(enabled, body).inner
}

pub fn card<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let t = Tokens::get(ui.ctx());
    Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_LG)
        .inner_margin(margin(Tokens::S5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            body(ui)
        })
        .inner
}
/// Cabecera de tarjeta: icono en cuadro de color, título, subtítulo y acciones a la derecha.
pub fn card_header(
    ui: &mut Ui,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    actions: impl FnOnce(&mut Ui),
) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        icon_tile(ui, icon, t.accent_soft, t.accent_ink);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            ui.label(
                RichText::new(title)
                    .font(theme::font_semibold(Tokens::FONT_MD))
                    .color(t.text),
            );
            if let Some(subtitle) = subtitle {
                ui.label(
                    RichText::new(subtitle)
                        .font(theme::font(Tokens::FONT_SM))
                        .color(t.text_dim),
                );
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), actions);
    });
    ui.add_space(Tokens::S3);
}
pub fn icon_tile(ui: &mut Ui, icon: &str, fill: egui::Color32, ink: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(Tokens::CONTROL + 2.), Sense::hover());
    ui.painter().rect_filled(rect, Tokens::RADIUS_MD, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::icon_font(Tokens::ICON + 1.),
        ink,
    );
}
/// Columna de líneas centrada en la altura de una fila horizontal. `ui.vertical` la
/// pega arriba; aquí se mide su alto y, desde el frame siguiente, se centra.
pub fn middle<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.next_auto_id().with("middle");
    let height = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(0.);
    let inner = ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), height),
        Layout::top_down(Align::Min),
        |ui| {
            let r = add(ui);
            (r, ui.min_rect().height())
        },
    );
    let (r, measured) = inner.inner;
    if (measured - height).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_discard("medir columna centrada");
    }
    r
}
pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    let t = Tokens::get(ui.ctx());
    middle(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.;
        ui.label(
            RichText::new(title)
                .font(theme::font_semibold(Tokens::FONT_LG))
                .color(t.text),
        );
        ui.label(
            RichText::new(subtitle)
                .font(theme::font(Tokens::FONT_SM))
                .color(t.text_dim),
        );
    });
}
/// Etiqueta de sección en mayúsculas pequeñas.
pub fn section_label(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.,
        egui::TextFormat {
            font_id: theme::font_semibold(Tokens::FONT_XS),
            color: t.text_faint,
            extra_letter_spacing: Tokens::FONT_XS * Tokens::TRACKING,
            ..Default::default()
        },
    );
    ui.label(job);
}
pub fn field_label(ui: &mut Ui, text: &str, hint: Option<&str>) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(text)
                .font(theme::font_medium(Tokens::FONT_SM + 0.5))
                .color(t.text),
        );
        if let Some(hint) = hint {
            ui.label(
                RichText::new(hint)
                    .font(egui::FontId::monospace(Tokens::FONT_XS))
                    .color(t.text_faint),
            );
        }
    });
}
pub fn dim(ui: &mut Ui, text: impl Into<String>) -> Response {
    let t = Tokens::get(ui.ctx());
    ui.label(
        RichText::new(text.into())
            .font(theme::font(Tokens::FONT_SM))
            .color(t.text_dim),
    )
}
pub fn strong(ui: &mut Ui, text: impl Into<String>) -> Response {
    let t = Tokens::get(ui.ctx());
    ui.label(
        RichText::new(text.into())
            .font(theme::font_medium(Tokens::FONT_BODY))
            .color(t.text),
    )
}

/// Píldora con punto de color, para estados.
pub fn pill(ui: &mut Ui, color: egui::Color32, fill: egui::Color32, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        theme::font_medium(Tokens::FONT_SM),
        Tokens::INK,
    );
    let size = vec2(
        galley.size().x + Tokens::S3 * 2. + Tokens::S2,
        Tokens::CONTROL - Tokens::S2,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    let painter = ui.painter();
    painter.rect_filled(rect, Tokens::RADIUS_FULL, fill);
    let dot = pos2(rect.left() + Tokens::S3, rect.center().y);
    painter.circle_filled(dot, Tokens::DOT - 1., color);
    painter.galley(
        pos2(dot.x + Tokens::S2, rect.center().y - galley.size().y / 2.),
        galley,
        color,
    );
    response
}
/// Insignia pequeña para cuantización, tamaño, etiquetas o contadores.
pub fn chip(ui: &mut Ui, text: &str, fill: egui::Color32, ink: egui::Color32) -> Response {
    chip_with(ui, text, theme::font_medium(Tokens::FONT_XS), fill, ink)
}
/// Cifra destacada (mejor valor) en monoespaciada sobre fondo verde suave.
pub fn best(ui: &mut Ui, text: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    chip_with(
        ui,
        text,
        egui::FontId::monospace(Tokens::FONT_SM),
        t.ok_soft,
        t.ok,
    )
}
fn chip_with(
    ui: &mut Ui,
    text: &str,
    font: egui::FontId,
    fill: egui::Color32,
    ink: egui::Color32,
) -> Response {
    // Etiqueta (no texto pintado) para que se pueda seleccionar y copiar.
    let margin = egui::vec2((Tokens::S2 + 2.) / 2., (Tokens::S1 + 1.) / 2.);
    // Alto acotado: en una fila centrada el marco ocuparía todo el alto disponible.
    let height = ui.fonts_mut(|f| f.row_height(&font)) + 2. * margin.y;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::Frame::new()
                .fill(fill)
                .corner_radius(Tokens::RADIUS_SM)
                .inner_margin(egui::Margin::symmetric(margin.x as i8, margin.y as i8))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(text).font(font).color(ink))
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );
                })
                .response
        },
    )
    .inner
}
pub fn status_dot(ui: &mut Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(Tokens::S2 + 2.), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), Tokens::DOT + 2., color.gamma_multiply(0.25));
    ui.painter()
        .circle_filled(rect.center(), Tokens::DOT - 0.5, color);
}

pub fn ratio_color(t: &Tokens, ratio: f64) -> egui::Color32 {
    if ratio >= 0.9 {
        t.error
    } else if ratio >= 0.75 {
        t.warning
    } else {
        t.accent
    }
}
/// Textos no seleccionables en `ui`: filas clicables, barras de título y navegación.
pub fn no_select(ui: &mut Ui) {
    ui.style_mut().interaction.selectable_labels = false;
}
/// Barra de progreso fina y redondeada.
pub fn meter(ui: &mut Ui, used: f64, capacity: f64, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let ratio = if capacity > 0. {
        (used / capacity).clamp(0., 1.)
    } else {
        0.
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    ui.painter()
        .rect_filled(rect, Tokens::RADIUS_FULL, t.surface_alt);
    if ratio > 0. {
        let mut filled = rect;
        filled.set_width((rect.width() * ratio as f32).max(height));
        ui.painter()
            .rect_filled(filled, Tokens::RADIUS_FULL, ratio_color(&t, ratio));
    }
    response
}
/// Medidor con título a la izquierda, valor a la derecha y barra debajo.
pub fn meter_row(ui: &mut Ui, title: &str, value: &str, used: f64, capacity: f64) {
    let t = Tokens::get(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = Tokens::S1 + 1.;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::font_medium(Tokens::FONT_SM))
                    .color(t.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(value)
                        .font(egui::FontId::monospace(Tokens::FONT_XS))
                        .color(t.text_dim),
                );
            });
        });
        meter(ui, used, capacity, Tokens::BAR);
    });
}
/// Barra apilada por segmentos (pesos, KV, overhead…) sobre una capacidad total.
pub fn stacked_bar(ui: &mut Ui, segments: &[(f64, egui::Color32)], capacity: f64) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), Tokens::BAR_LG), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Tokens::RADIUS_FULL, t.surface_alt);
    let total: f64 = segments.iter().map(|(v, _)| v.max(0.)).sum();
    let scale = capacity.max(total);
    if scale <= 0. {
        return;
    }
    let clip = painter.with_clip_rect(rect);
    // Un rectángulo redondeado del total recorta las esquinas de los segmentos.
    let mut x = rect.left();
    let mut shapes = vec![];
    for (value, color) in segments {
        let width = rect.width() * (value.max(0.) / scale) as f32;
        if width <= 0. {
            continue;
        }
        shapes.push((
            egui::Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height())),
            *color,
        ));
        x += width;
    }
    let used = egui::Rect::from_min_max(rect.min, pos2(x, rect.bottom()));
    let mask = clip.with_clip_rect(used);
    for (i, (r, color)) in shapes.iter().enumerate() {
        let radius = if shapes.len() == 1 {
            egui::CornerRadius::same(Tokens::RADIUS_FULL)
        } else if i == 0 {
            egui::CornerRadius {
                nw: Tokens::RADIUS_FULL,
                sw: Tokens::RADIUS_FULL,
                ne: 0,
                se: 0,
            }
        } else if i + 1 == shapes.len() {
            egui::CornerRadius {
                nw: 0,
                sw: 0,
                ne: Tokens::RADIUS_FULL,
                se: Tokens::RADIUS_FULL,
            }
        } else {
            egui::CornerRadius::ZERO
        };
        mask.rect_filled(*r, radius, *color);
    }
    if capacity > 0. && total > capacity {
        painter.rect_stroke(
            rect,
            Tokens::RADIUS_FULL,
            Stroke::new(Tokens::BORDER + 0.5, t.error),
            StrokeKind::Outside,
        );
    }
}
pub fn legend(ui: &mut Ui, color: egui::Color32, label: &str, value: &str) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(Tokens::S2 + 2.), Sense::hover());
        ui.painter().rect_filled(rect, Tokens::RADIUS_SM / 2, color);
        ui.label(
            RichText::new(label)
                .font(theme::font(Tokens::FONT_SM))
                .color(t.text_dim),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .font(egui::FontId::monospace(Tokens::FONT_SM))
                    .color(t.text),
            );
        });
    });
}

/// Indicador numérico grande: etiqueta, valor y unidad.
pub fn stat(ui: &mut Ui, icon: &str, label: &str, value: &str, unit: &str, highlight: bool) {
    let t = Tokens::get(ui.ctx());
    Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_LG)
        .inner_margin(margin(Tokens::S4))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = Tokens::S1;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(icon)
                        .font(theme::icon_font(Tokens::ICON - 2.))
                        .color(if highlight {
                            t.accent_ink
                        } else {
                            t.text_faint
                        }),
                );
                ui.label(
                    RichText::new(label)
                        .font(theme::font_medium(Tokens::FONT_SM))
                        .color(t.text_dim),
                );
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = Tokens::S1 + 1.;
                ui.label(
                    RichText::new(value)
                        .font(theme::font_semibold(Tokens::FONT_KPI))
                        .color(if highlight { t.accent_ink } else { t.text }),
                );
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    ui.add_space(0.);
                    ui.label(
                        RichText::new(unit)
                            .font(theme::font_medium(Tokens::FONT_SM))
                            .color(t.text_faint),
                    );
                });
            });
        });
}
/// Celda compacta de métrica para rejillas de detalle.
pub fn metric(ui: &mut Ui, label: &str, value: &str) {
    let t = Tokens::get(ui.ctx());
    Frame::new()
        .fill(t.surface_alt)
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(Margin::symmetric(Tokens::S3 as i8, Tokens::S2 as i8 + 2))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = Tokens::S1 / 2.;
            ui.label(
                RichText::new(label)
                    .font(theme::font(Tokens::FONT_XS + 0.5))
                    .color(t.text_dim),
            );
            ui.label(
                RichText::new(value)
                    .font(egui::FontId::monospace(Tokens::FONT_BODY))
                    .color(t.text),
            );
        });
}
/// Reparte el ancho disponible en `n` columnas iguales sin el reparto fijo de `ui.columns`.
pub fn grid<T>(ui: &mut Ui, items: &[T], min: f32, mut cell: impl FnMut(&mut Ui, &T)) {
    let gap = Tokens::S3;
    let width = ui.available_width();
    let per_row = (((width + gap) / (min + gap)).floor() as usize).clamp(1, items.len().max(1));
    let cell_width = (width - gap * (per_row as f32 - 1.)) / per_row as f32;
    for row in items.chunks(per_row) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for item in row {
                ui.allocate_ui_with_layout(
                    vec2(cell_width, 0.),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(cell_width);
                        cell(ui, item)
                    },
                );
            }
        });
        ui.add_space(gap - ui.spacing().item_spacing.y);
    }
}

/// Interruptor con etiqueta; se expone como casilla para accesibilidad y tests.
pub fn toggle(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let enabled = ui.is_enabled();
    let galley = ui.painter().layout(
        label.to_owned(),
        theme::font(Tokens::FONT_BODY),
        Tokens::INK,
        ui.available_width() - Tokens::TOGGLE[0] - Tokens::S3,
    );
    let size = vec2(
        Tokens::TOGGLE[0] + Tokens::S3 + galley.size().x,
        galley.size().y.max(Tokens::CONTROL - Tokens::S1),
    );
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *value, label));
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *value);
        let track = egui::Rect::from_min_size(
            pos2(rect.left(), rect.center().y - Tokens::TOGGLE[1] / 2.),
            Tokens::TOGGLE.into(),
        );
        let off = if response.hovered() {
            t.border_strong
        } else {
            t.border
        };
        let mut fill = off.lerp_to_gamma(t.accent, how_on);
        let mut ink = t.text;
        if !enabled {
            fill = fill.gamma_multiply(0.45);
            ink = t.text_faint;
        }
        let painter = ui.painter();
        painter.rect_filled(track, Tokens::RADIUS_FULL, fill);
        let radius = Tokens::TOGGLE[1] / 2. - 2.;
        let x = egui::lerp(
            (track.left() + radius + 2.)..=(track.right() - radius - 2.),
            how_on,
        );
        painter.circle_filled(pos2(x, track.center().y), radius, t.on_accent);
        painter.galley(
            pos2(
                track.right() + Tokens::S3,
                rect.center().y - galley.size().y / 2.,
            ),
            galley,
            ink,
        );
    }
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}
/// Control segmentado (p. ej. Oscuro / Claro / Sistema).
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let t = Tokens::get(ui.ctx());
    let mut changed = false;
    Frame::new()
        .fill(t.sunken)
        .stroke(Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(margin(Tokens::S1 - 1.))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = Tokens::S1 - 1.;
                // Dentro de un layout de derecha a izquierda se mantiene el orden de lectura.
                let mut order: Vec<_> = options.iter().collect();
                if ui.layout().prefer_right_to_left() {
                    order.reverse();
                }
                for (option, label) in order {
                    let selected = *value == *option;
                    let galley = ui.painter().layout_no_wrap(
                        (*label).to_owned(),
                        theme::font_medium(Tokens::FONT_SM + 0.5),
                        Tokens::INK,
                    );
                    let size = vec2(
                        galley.size().x + Tokens::S4 * 2.,
                        Tokens::CONTROL - Tokens::S1 - 2.,
                    );
                    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, *label)
                    });
                    if response.clicked() && !selected {
                        *value = *option;
                        changed = true;
                    }
                    let painter = ui.painter();
                    if selected {
                        painter.rect(
                            rect,
                            Tokens::RADIUS_SM,
                            t.surface_alt,
                            Stroke::new(Tokens::BORDER, t.border_strong),
                            StrokeKind::Inside,
                        );
                    } else if response.hovered() {
                        painter.rect_filled(rect, Tokens::RADIUS_SM, t.hover);
                    }
                    painter.galley(
                        rect.center() - galley.size() / 2.,
                        galley,
                        if selected { t.text } else { t.text_dim },
                    );
                }
            });
        });
    changed
}
/// Campo de búsqueda con icono y fondo hundido.
pub fn search(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    Frame::new()
        .fill(t.sunken)
        .stroke(Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(Margin::symmetric(Tokens::S2 as i8 + 2, 0))
        .show(ui, |ui| {
            // Mismo alto que los botones (`CONTROL`), contando el borde.
            let size = egui::vec2(width, Tokens::CONTROL - 2. * Tokens::BORDER);
            ui.allocate_ui_with_layout(
                size,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(size);
                    ui.label(
                        RichText::new(icons::MAGNIFYING_GLASS)
                            .font(theme::icon_font(Tokens::ICON - 2.))
                            .color(t.text_faint),
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(value)
                            .hint_text(hint)
                            .frame(egui::Frame::new())
                            .desired_width(f32::INFINITY),
                    );
                    if !value.is_empty()
                        && ui
                            .add(
                                egui::Label::new(
                                    RichText::new(icons::X)
                                        .font(theme::icon_font(Tokens::ICON - 3.))
                                        .color(t.text_faint),
                                )
                                .sense(Sense::click()),
                            )
                            .on_hover_text("Limpiar filtro")
                            .clicked()
                    {
                        value.clear();
                    }
                    response
                },
            )
            .inner
        })
        .inner
}
/// Marco hundido para editores de texto multilínea.
pub fn sunken<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let t = Tokens::get(ui.ctx());
    Frame::new()
        .fill(t.sunken)
        .stroke(Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(margin(Tokens::S3))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            body(ui)
        })
        .inner
}
/// Aviso en línea con icono, en el color del tipo.
pub fn banner(ui: &mut Ui, kind: crate::store::NoticeKind, text: &str) {
    let t = Tokens::get(ui.ctx());
    let (icon, ink, fill) = notice_style(&t, kind);
    Frame::new()
        .fill(fill)
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(Margin::symmetric(Tokens::S3 as i8, Tokens::S2 as i8 + 2))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new(icon)
                        .font(theme::icon_font(Tokens::ICON))
                        .color(ink),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(text)
                            .font(theme::font(Tokens::FONT_SM + 0.5))
                            .color(t.text),
                    )
                    .wrap(),
                );
            });
        });
}
fn notice_style(
    t: &Tokens,
    kind: crate::store::NoticeKind,
) -> (&'static str, egui::Color32, egui::Color32) {
    use crate::store::NoticeKind;
    match kind {
        NoticeKind::Success => (icons::CHECK_CIRCLE, t.ok, t.ok_soft),
        NoticeKind::Warning => (icons::WARNING, t.warning, t.warning_soft),
        NoticeKind::Error => (icons::WARNING_CIRCLE, t.error, t.error_soft),
    }
}
/// Estado vacío centrado con icono, título y texto de ayuda.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, body: &str) {
    let t = Tokens::get(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(Tokens::S8);
        ui.label(
            RichText::new(icon)
                .font(theme::icon_font(Tokens::FONT_KPI + Tokens::S2))
                .color(t.text_faint),
        );
        ui.add_space(Tokens::S1);
        ui.label(
            RichText::new(title)
                .font(theme::font_semibold(Tokens::FONT_MD))
                .color(t.text),
        );
        ui.label(
            RichText::new(body)
                .font(theme::font(Tokens::FONT_SM + 0.5))
                .color(t.text_dim),
        );
        ui.add_space(Tokens::S6);
    });
}
pub fn separator(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), Tokens::BORDER), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(Tokens::BORDER, t.border),
    );
}

pub struct Dialog<R> {
    pub inner: R,
    pub close: bool,
}
/// Diálogo modal con cabecera (título y ✕), fondo atenuado y ancho adaptable.
pub fn dialog<R>(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    width: f32,
    body: impl FnOnce(&mut Ui) -> R,
) -> Dialog<R> {
    let t = Tokens::get(ctx);
    let mut close = false;
    let width = width
        .min(ctx.content_rect().width() - Tokens::DIALOG_INSET)
        .max(Tokens::DIALOG_MIN);
    let modal = egui::Modal::new(egui::Id::new(id))
        .backdrop_color(t.backdrop)
        .frame(
            Frame::new()
                .fill(t.surface)
                .stroke(Stroke::new(Tokens::BORDER, t.border_strong))
                .corner_radius(Tokens::RADIUS_LG)
                .shadow(ctx.global_style().visuals.window_shadow),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            // El área del modal parte del tamaño del frame anterior; sin esto, si la
            // ventana crece con el diálogo abierto, el cuerpo sigue con la altura vieja.
            ui.set_max_height(ctx.content_rect().height());
            Frame::new()
                .inner_margin(Margin {
                    left: Tokens::S6 as i8,
                    right: Tokens::S3 as i8,
                    top: Tokens::S4 as i8,
                    bottom: Tokens::S3 as i8,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(title)
                                .font(theme::font_semibold(Tokens::FONT_MD + 1.))
                                .color(t.text),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_button(ui, icons::X, "Cerrar diálogo").clicked() {
                                close = true;
                            }
                        });
                    });
                });
            separator(ui);
            // En ventanas bajas (la compacta) el cuerpo se desplaza para que el pie
            // con los botones siga a mano.
            let max_height = (ctx.content_rect().height() - Tokens::DIALOG_CHROME).max(Tokens::ROW);
            egui::ScrollArea::vertical()
                .id_salt((id, "body"))
                .max_height(max_height)
                .show(ui, |ui| {
                    Frame::new()
                        .inner_margin(Margin::symmetric(Tokens::S6 as i8, Tokens::S5 as i8))
                        .show(ui, body)
                        .inner
                })
                .inner
        });
    Dialog {
        close: close || modal.should_close(),
        inner: modal.inner,
    }
}
/// Pie de diálogo: línea divisoria a todo el ancho y botones alineados a la derecha.
pub fn dialog_footer<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let t = Tokens::get(ui.ctx());
    ui.add_space(Tokens::S3);
    let y = ui.cursor().top();
    let x = ui.max_rect().x_range();
    ui.painter().hline(
        (x.min - Tokens::S6)..=(x.max + Tokens::S6),
        y,
        Stroke::new(Tokens::BORDER, t.border),
    );
    ui.add_space(Tokens::S4);
    // Con la altura de un botón: `with_layout` ocuparía todo el alto libre del modal.
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), Tokens::CONTROL),
        Layout::right_to_left(Align::Center),
        body,
    )
    .inner
}

/// Avisos flotantes abajo a la derecha, cada uno en su propia tarjeta.
pub fn toasts(
    ctx: &egui::Context,
    notices: &std::collections::VecDeque<(std::time::Instant, String, crate::store::NoticeKind)>,
) {
    if notices.is_empty() {
        return;
    }
    let t = Tokens::get(ctx);
    egui::Area::new(egui::Id::new("toasts"))
        .order(egui::Order::Tooltip)
        .anchor(Align2::RIGHT_BOTTOM, [-Tokens::S5, -Tokens::S5])
        .interactable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = Tokens::S2;
            for (_, text, kind) in notices {
                let (icon, ink, _) = notice_style(&t, *kind);
                Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(Tokens::BORDER, t.border_strong))
                    .corner_radius(Tokens::RADIUS_MD + 2)
                    .shadow(ctx.global_style().visuals.popup_shadow)
                    .inner_margin(Margin::symmetric(Tokens::S4 as i8, Tokens::S3 as i8))
                    .show(ui, |ui| {
                        ui.set_width(Tokens::TOAST_WIDTH);
                        ui.horizontal_top(|ui| {
                            ui.label(
                                RichText::new(icon)
                                    .font(theme::icon_font(Tokens::ICON + 1.))
                                    .color(ink),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(text)
                                        .font(theme::font(Tokens::FONT_BODY))
                                        .color(t.text),
                                )
                                .wrap(),
                            );
                        });
                    });
            }
        });
}
