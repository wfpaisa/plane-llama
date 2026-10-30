// Barra superior propia: título de la página, accesos rápidos, botones de ventana y
// zonas nativas de movimiento y redimensionado (sin decoraciones de winit en Wayland).
use crate::{
    theme::{self, Tokens},
    widgets::{self, icons},
};
use egui::{ResizeDirection as Direction, ViewportCommand as Command};

#[derive(Default)]
pub struct Clicks {
    pub settings: bool,
    pub theme: bool,
    /// Se eligió otra vista: `Some(true)` simple, `Some(false)` ampliada.
    pub simple: Option<bool>,
}

pub fn caption(ui: &mut egui::Ui, icon: &str, label: &str, danger: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Tokens::CAPTION.into(), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    // Círculo como los de GNOME: tenue en reposo, más marcado (o rojo en cerrar) al pasar.
    let hovered = response.hovered();
    let fill = match (hovered, danger) {
        (true, true) => t.error,
        (true, false) => t.border_strong,
        (false, _) => t.hover,
    };
    ui.painter()
        .circle_filled(rect.center(), Tokens::CAPTION_DOT / 2., fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        theme::icon_font(Tokens::ICON - 4.),
        if hovered && danger {
            t.on_accent
        } else if hovered {
            t.text
        } else {
            t.text_dim
        },
    );
    response.on_hover_text(label)
}

/// Barra de la vista ampliada; la simple usa la de `compact`.
pub fn show(ui: &mut egui::Ui, title: &str, subtitle: &str) -> Clicks {
    let t = Tokens::get(ui.ctx());
    let mut clicks = Clicks::default();
    egui::Panel::top("topbar")
        .exact_size(Tokens::TOPBAR)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            // Sin relleno: el fondo redondeado de la ventana ya está pintado debajo.
            egui::Frame::new().inner_margin(egui::Margin {
                left: Tokens::S6 as i8,
                right: Tokens::S2 as i8,
                top: 0,
                bottom: 0,
            }),
        )
        .show(ui, |ui| {
            widgets::no_select(ui);
            let line = ui.max_rect();
            ui.painter().hline(
                line.x_range(),
                line.bottom(),
                egui::Stroke::new(Tokens::BORDER, t.border),
            );
            ui.horizontal_centered(|ui| {
                widgets::page_title(ui, title, subtitle);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = Tokens::S1 / 2.;
                    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    for (icon, label, cmd, danger) in [
                        (icons::X, "Cerrar", Command::Close, true),
                        (
                            if maximized {
                                icons::CORNERS_IN
                            } else {
                                icons::SQUARE
                            },
                            "Maximizar / restaurar",
                            Command::Maximized(!maximized),
                            false,
                        ),
                        (icons::MINUS, "Minimizar", Command::Minimized(true), false),
                    ] {
                        if caption(ui, icon, label, danger).clicked() {
                            ui.ctx().send_viewport_cmd(cmd);
                        }
                    }
                    ui.add_space(Tokens::S2);
                    let (rule, _) = ui.allocate_exact_size(
                        egui::vec2(Tokens::BORDER, Tokens::ICON + 2.),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rule, 0, t.border_strong);
                    ui.add_space(Tokens::S2);
                    clicks.settings =
                        widgets::icon_button(ui, icons::GEAR_SIX, "Ajustes").clicked();
                    if widgets::icon_button(ui, icons::ARROWS_IN_SIMPLE, "Vista compacta").clicked()
                    {
                        clicks.simple = Some(true);
                    }
                    ui.add_space(Tokens::S1);
                    let theme_icon = if ui.ctx().theme() == egui::Theme::Dark {
                        icons::SUN
                    } else {
                        icons::MOON
                    };
                    clicks.theme =
                        widgets::icon_button(ui, theme_icon, "Cambiar tema claro / oscuro")
                            .clicked();
                    let rect = ui.available_rect_before_wrap();
                    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                    if response.double_clicked() {
                        ui.ctx().send_viewport_cmd(Command::Maximized(!maximized));
                    } else if response.drag_started() {
                        crate::window::start_drag(ui.ctx());
                    }
                });
            });
        });
    clicks
}

pub fn resize_zones(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    // Las zonas ocupan el margen de la sombra y unos píxeles del borde de la ventana.
    // No se solapan: egui sube al frente el área que se usó, y un borde encima de una
    // esquina la dejaba inalcanzable.
    let r = ctx.content_rect();
    let e = crate::window::margin(ctx) + Tokens::RESIZE;
    let c = crate::window::margin(ctx) + Tokens::RESIZE_CORNER;
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
        egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
    };
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let zones = [
        (
            rect(l + c, t, rr - c, t + e),
            Direction::North,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            rect(l + c, b - e, rr - c, b),
            Direction::South,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            rect(l, t + c, l + e, b - c),
            Direction::West,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            rect(rr - e, t + c, rr, b - c),
            Direction::East,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            rect(l, t, l + c, t + c),
            Direction::NorthWest,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            rect(rr - c, t, rr, t + c),
            Direction::NorthEast,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            rect(l, b - c, l + c, b),
            Direction::SouthWest,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            rect(rr - c, b - c, rr, b),
            Direction::SouthEast,
            egui::CursorIcon::ResizeNwSe,
        ),
    ];
    for (index, (rect, direction, cursor)) in zones.into_iter().enumerate() {
        egui::Area::new(egui::Id::new(("resize", index)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
                if response.hovered() || response.dragged() {
                    ctx.set_cursor_icon(cursor);
                }
                if response.drag_started() {
                    crate::window::begin_resize(ctx, direction);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[test]
    fn caption_buttons_send_native_commands() {
        for (label, expected) in [
            ("Cerrar", Command::Close),
            ("Minimizar", Command::Minimized(true)),
            ("Maximizar / restaurar", Command::Maximized(true)),
        ] {
            let mut h = Harness::builder()
                .with_size(egui::vec2(1200., 800.))
                .build_ui(|ui| {
                    show(ui, "Benchmark", "Prueba");
                });
            h.get_by_label(label).click();
            h.run_steps(1);
            assert!(
                h.output()
                    .viewport_output
                    .values()
                    .any(|v| v.commands.contains(&expected)),
                "{label}"
            );
        }
    }
}
