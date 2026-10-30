// Marco propio de la ventana: sombra, esquinas redondeadas y borde claro. La superficie
// es transparente y lo que queda fuera del rectángulo interior es solo la sombra.
use crate::theme::Tokens;
use egui::{Rect, Stroke, StrokeKind, Ui};

/// Maximizada o a pantalla completa: sin sombra ni esquinas.
pub fn tiled(ctx: &egui::Context) -> bool {
    ctx.input(|i| {
        let v = i.viewport();
        v.maximized.unwrap_or(false) || v.fullscreen.unwrap_or(false)
    })
}
pub fn margin(ctx: &egui::Context) -> f32 {
    if tiled(ctx) {
        0.
    } else {
        Tokens::WINDOW_SHADOW
    }
}
pub fn radius(ctx: &egui::Context) -> u8 {
    if tiled(ctx) { 0 } else { Tokens::WINDOW_RADIUS }
}
/// Rectángulo de la ventana visible, sin el margen de la sombra.
pub fn inner(ctx: &egui::Context) -> Rect {
    ctx.content_rect().shrink(margin(ctx))
}

/// Pinta la sombra y el fondo, y devuelve el rectángulo donde va el contenido.
pub fn begin(ui: &mut Ui) -> Rect {
    let ctx = ui.ctx().clone();
    let t = Tokens::get(&ctx);
    let rect = inner(&ctx);
    let r = radius(&ctx);
    if margin(&ctx) > 0. {
        let shadow = egui::epaint::Shadow {
            offset: [0, 3],
            blur: Tokens::WINDOW_SHADOW as u8,
            spread: 0,
            color: t.shadow,
        };
        ui.painter().add(shadow.as_shape(rect, r));
    }
    ui.painter().rect_filled(rect, r, t.bg);
    rect
}

/// Borde exterior, encima de todo el contenido de la ventana.
pub fn end(ui: &mut Ui, rect: Rect) {
    let t = Tokens::get(ui.ctx());
    ui.painter().rect_stroke(
        rect,
        radius(ui.ctx()),
        Stroke::new(Tokens::BORDER, t.window_border),
        StrokeKind::Inside,
    );
}

/// El compositor se queda con el puntero mientras mueve o redimensiona la ventana y
/// egui no recibe la liberación del botón: creería que sigue arrastrando y no haría
/// scroll hasta el siguiente clic. `release_hook` inyecta esa liberación.
fn grab(ctx: &egui::Context, cmd: egui::ViewportCommand) {
    ctx.data_mut(|d| d.insert_temp(release_id(), true));
    ctx.send_viewport_cmd(cmd);
}
fn release_id() -> egui::Id {
    egui::Id::new("window-grab-release")
}
pub fn start_drag(ctx: &egui::Context) {
    grab(ctx, egui::ViewportCommand::StartDrag);
}
pub fn begin_resize(ctx: &egui::Context, direction: egui::ResizeDirection) {
    grab(ctx, egui::ViewportCommand::BeginResize(direction));
}
/// Para `App::raw_input_hook`: tras mover o redimensionar, suelta el botón en egui.
/// La posición está lejos para que no cuente como clic (ni doble clic en la barra).
pub fn release_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
    if ctx
        .data_mut(|d| d.remove_temp::<bool>(release_id()))
        .is_some()
    {
        raw.events.push(egui::Event::PointerButton {
            pos: egui::pos2(-1e4, -1e4),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grab_releases_pointer_once_far_away() {
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        release_hook(&ctx, &mut raw);
        assert!(raw.events.is_empty());
        start_drag(&ctx);
        release_hook(&ctx, &mut raw);
        assert!(matches!(
            raw.events.as_slice(),
            [egui::Event::PointerButton { pressed: false, pos, .. }] if pos.x < -1000.
        ));
        let mut again = egui::RawInput::default();
        release_hook(&ctx, &mut again);
        assert!(again.events.is_empty());
    }
}
