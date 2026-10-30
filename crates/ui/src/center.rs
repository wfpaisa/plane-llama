// Centrado de la ventana al abrirla en GNOME (Wayland).
//
// En Wayland la app no puede elegir su posición. Mutter centra en el monitor las
// ventanas de tipo diálogo, así que la ventana nueva se marca como diálogo modal con
// `xdg_dialog_v1` antes de su primer frame y vuelve a ser normal cuando pierde el foco:
// Mutter se lo quita al cambiar el tipo, y hacerlo con la ventana enfocada la mandaba
// detrás de las demás.
// El hook lo llama egui-winit (parcheado en vendor/) al crear cada ventana.
use egui_winit::winit::{platform::wayland::WindowExtWayland, window::Window};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::sync::Mutex;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::{Backend, ObjectId},
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};
use wayland_protocols::xdg::{
    dialog::v1::client::{xdg_dialog_v1::XdgDialogV1, xdg_wm_dialog_v1::XdgWmDialogV1},
    shell::client::xdg_toplevel::XdgToplevel,
};

struct State;

/// Diálogo pendiente de volver a ventana normal, con su conexión.
struct Pending {
    conn: Connection,
    _queue: EventQueue<State>,
    dialog: XdgDialogV1,
}
// Solo se usa desde el hilo de la interfaz; el Mutex es para el `static`.
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Instala el hook; las ventanas creadas a partir de ahora se abren centradas.
pub fn install() {
    let _ = egui_winit::WINDOW_CREATED.set(on_window);
}

fn on_window(window: &Window) {
    crate::activate::on_window(window);
    if let Some(pending) = mark(window) {
        *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(pending);
    }
}

fn mark(window: &Window) -> Option<Pending> {
    let toplevel = window.xdg_toplevel()?;
    let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: display y toplevel son de la ventana de winit que se está creando; viven
    // más que `Pending`, que se libera en cuanto la ventana está colocada.
    let conn = Connection::from_backend(unsafe {
        Backend::from_foreign_display(display.display.as_ptr().cast())
    });
    let toplevel =
        unsafe { ObjectId::from_ptr(XdgToplevel::interface(), toplevel.as_ptr().cast()) }
            .ok()
            .and_then(|id| XdgToplevel::from_id(&conn, id).ok())?;
    let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
    let qh = queue.handle();
    let wm: XdgWmDialogV1 = globals.bind(&qh, 1..=1, ()).ok()?;
    let dialog = wm.get_xdg_dialog(&toplevel, &qh, ());
    dialog.set_modal();
    wm.destroy();
    conn.flush().ok()?;
    log::info!("ventana marcada como diálogo para que el compositor la centre");
    Some(Pending {
        conn,
        _queue: queue,
        dialog,
    })
}

/// La ventana ya está colocada y perdió el foco: deja de ser diálogo.
pub fn placed() {
    if let Some(p) = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take() {
        p.dialog.unset_modal();
        p.dialog.destroy();
        let _ = p.conn.flush();
        log::info!("ventana colocada; vuelve a ser normal");
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
wayland_client::delegate_noop!(State: ignore XdgWmDialogV1);
wayland_client::delegate_noop!(State: ignore XdgDialogV1);
