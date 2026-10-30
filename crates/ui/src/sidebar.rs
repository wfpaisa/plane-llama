// Barra lateral: marca, navegación entre páginas, hardware y control del servidor.
use crate::{
    gpu_grid, optimizer_modal, status_bar,
    store::{Store, TAB_BENCH, TAB_HISTORY, TAB_LOGS, TAB_MODELS, TAB_OPTIMIZER, TAB_SCRIPTS},
    theme::{self, Tokens},
    widgets::{self, icons},
};
use egui::{Align2, CursorIcon, RichText, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

pub const PAGES: [(usize, &str, &str); 6] = [
    (TAB_SCRIPTS, icons::ROCKET_LAUNCH, "Scripts"),
    (TAB_MODELS, icons::CUBE, "Modelos"),
    (TAB_BENCH, icons::GAUGE, "Benchmark"),
    (TAB_HISTORY, icons::CLOCK_COUNTER_CLOCKWISE, "Historial"),
    (TAB_OPTIMIZER, icons::LIGHTNING, "Optimizador"),
    (TAB_LOGS, icons::TERMINAL_WINDOW, "Logs"),
];

pub fn go(s: &mut Store, tab: usize) {
    if tab == TAB_OPTIMIZER && s.optimizer.is_none() {
        optimizer_modal::open(s);
    }
    s.tab = tab;
}

fn nav_item(
    ui: &mut Ui,
    icon: &str,
    label: &str,
    active: bool,
    badge: Option<String>,
    expanded: bool,
) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), Tokens::NAV_ITEM), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, label));
    let hovered = response.hovered();
    let painter = ui.painter();
    if active {
        painter.rect_filled(rect, Tokens::RADIUS_MD, t.accent_soft);
    } else if hovered {
        painter.rect_filled(rect, Tokens::RADIUS_MD, t.hover);
    }
    let ink = if active {
        t.accent_ink
    } else if hovered {
        t.text
    } else {
        t.text_dim
    };
    let icon_x = if expanded {
        rect.left() + Tokens::S3 + Tokens::S1
    } else {
        rect.center().x - Tokens::ICON_LG / 2.
    };
    painter.text(
        pos2(icon_x, rect.center().y),
        Align2::LEFT_CENTER,
        icon,
        theme::icon_font(Tokens::ICON_LG - 2.),
        ink,
    );
    if expanded {
        painter.text(
            pos2(icon_x + Tokens::ICON_LG + Tokens::S2 + 2., rect.center().y),
            Align2::LEFT_CENTER,
            label,
            theme::font_medium(Tokens::FONT_BODY),
            if active || hovered {
                t.text
            } else {
                t.text_dim
            },
        );
        if let Some(badge) = badge {
            let galley = painter.layout_no_wrap(
                badge,
                egui::FontId::monospace(Tokens::FONT_XS),
                Tokens::INK,
            );
            let chip = egui::Rect::from_center_size(
                pos2(
                    rect.right() - Tokens::S3 - galley.size().x / 2. - Tokens::S1,
                    rect.center().y,
                ),
                galley.size() + vec2(Tokens::S2 + 2., Tokens::S1),
            );
            painter.rect_filled(
                chip,
                Tokens::RADIUS_FULL,
                if active { t.accent_soft } else { t.surface_alt },
            );
            painter.galley(chip.center() - galley.size() / 2., galley, ink);
        }
    }
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if expanded {
        response
    } else {
        response.on_hover_text(label)
    }
}

/// Logotipo de la aplicación (el mismo de plane-llama-bench).
pub fn logo(ui: &mut Ui, size: f32) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), Sense::hover());
    match theme::logo(ui.ctx()) {
        Some(texture) => {
            egui::Image::new((texture.id(), rect.size())).paint_at(ui, rect);
        }
        None => {
            ui.painter().rect_filled(rect, Tokens::RADIUS_MD, t.accent);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                icons::AIRPLANE_TILT,
                theme::icon_font(Tokens::ICON + 1.),
                t.on_accent,
            );
        }
    }
}

/// Logotipo y nombre; la zona vacía mueve la ventana.
fn brand(ui: &mut Ui, expanded: &mut bool) {
    let t = Tokens::get(ui.ctx());
    let height = Tokens::TOPBAR - Tokens::S3;
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let drag = ui.interact(
                ui.max_rect(),
                ui.id().with("brand-drag"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() {
                crate::window::start_drag(ui.ctx());
            }
            if *expanded {
                ui.add_space(Tokens::S1);
            }
            logo(ui, Tokens::CONTROL);
            if *expanded {
                ui.add_space(Tokens::S1);
                widgets::middle(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    ui.label(
                        RichText::new(crate::desktop::APP_NAME)
                            .font(theme::font_semibold(Tokens::FONT_MD))
                            .color(t.text),
                    );
                    ui.label(
                        RichText::new("bench de llama-server")
                            .font(theme::font(Tokens::FONT_XS))
                            .color(t.text_faint),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, icons::SIDEBAR_SIMPLE, "Contraer menú").clicked() {
                        *expanded = false;
                    }
                });
            }
        },
    );
    if !*expanded {
        ui.vertical_centered(|ui| {
            if widgets::icon_button(ui, icons::SIDEBAR_SIMPLE, "Expandir menú").clicked() {
                *expanded = true;
            }
        });
    }
}

pub fn show(root: &mut Ui, s: &mut Store, expanded: &mut bool) {
    let t = Tokens::get(root.ctx());
    egui::Panel::left("sidebar")
        .resizable(false)
        .exact_size(if *expanded {
            Tokens::SIDEBAR
        } else {
            Tokens::SIDEBAR_COLLAPSED
        })
        .show_separator_line(false)
        .frame(
            // Esquinas izquierdas redondeadas como las de la ventana.
            egui::Frame::new()
                .fill(t.sidebar)
                .corner_radius(egui::CornerRadius {
                    nw: crate::window::radius(root.ctx()),
                    sw: crate::window::radius(root.ctx()),
                    ne: 0,
                    se: 0,
                })
                .inner_margin(egui::Margin::symmetric(Tokens::S3 as i8, Tokens::S2 as i8)),
        )
        .show(root, |ui| {
            widgets::no_select(ui);
            let edge = ui.max_rect().right() + Tokens::S3;
            ui.painter().vline(
                edge,
                ui.clip_rect().y_range(),
                Stroke::new(Tokens::BORDER, t.border),
            );
            egui::Panel::bottom("sidebar-footer")
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::new().inner_margin(egui::Margin {
                    bottom: Tokens::S1 as i8,
                    ..Default::default()
                }))
                .show(ui, |ui| {
                    if *expanded {
                        widgets::section_label(ui, "Hardware");
                        gpu_grid::show(ui, s);
                        ui.add_space(Tokens::S2);
                        status_bar::show(ui, s);
                    } else {
                        status_bar::compact(ui, s);
                    }
                });
            brand(ui, expanded);
            ui.add_space(Tokens::S3);
            if *expanded {
                widgets::section_label(ui, "Navegación");
            }
            ui.spacing_mut().item_spacing.y = Tokens::S1;
            for (tab, icon, label) in PAGES {
                let downloading = !s.hub.downloads.is_empty();
                let badge = (tab == TAB_HISTORY && !s.history.is_empty())
                    .then(|| s.history.len().to_string())
                    .or_else(|| {
                        (tab == TAB_SCRIPTS && !s.library.list.is_empty())
                            .then(|| s.library.list.len().to_string())
                    })
                    .or_else(|| (tab == TAB_MODELS && downloading).then(|| "bajando".to_owned()))
                    .or_else(|| (tab == TAB_LOGS && s.running()).then(|| "en vivo".to_owned()));
                if nav_item(ui, icon, label, s.tab == tab, badge, *expanded).clicked() {
                    go(s, tab);
                }
            }
        });
}
