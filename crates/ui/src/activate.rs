// Ventana recreada al frente (GNOME, Wayland).
//
// Cambiar de vista cierra la ventana y crea otra para que el compositor la centre. Mutter
// solo enfoca la nueva con un token de xdg-activation que pidió una superficie viva y
// enfocada, con el serial de un clic o tecla reciente; winit lo pide sin serial y la
// ventana nueva quedaba detrás de las demás. Aquí se escuchan los clics y teclas en una
// cola propia de la conexión de winit, se pide el token antes de cerrar la ventana vieja
// y se activa la nueva con él.
use egui_winit::winit::window::Window;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::sync::Mutex;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    backend::{Backend, ObjectId},
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_surface::WlSurface},
};
use wayland_protocols::xdg::activation::v1::client::{
    xdg_activation_token_v1::{self, XdgActivationTokenV1},
    xdg_activation_v1::XdgActivationV1,
};

/// Última pulsación (botón o tecla) en la ventana y el token recibido.
#[derive(Default)]
struct Input {
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    serial: Option<u32>,
    token: Option<String>,
}

/// Escucha de la ventana abierta. Vive mientras exista su conexión.
struct Tracker {
    conn: Connection,
    queue: EventQueue<Input>,
    input: Input,
    activation: XdgActivationV1,
    surface: WlSurface,
}
impl Tracker {
    /// Libera los objetos propios mientras la conexión de winit sigue abierta.
    fn release(self) {
        if let Some(p) = &self.input.pointer {
            p.release();
        }
        if let Some(k) = &self.input.keyboard {
            k.release();
        }
        if let Some(s) = &self.input.seat {
            s.release();
        }
        self.activation.destroy();
        let _ = self.conn.flush();
    }
}

// Solo se usan desde el hilo de la interfaz; los Mutex son para los `static`.
static TRACKER: Mutex<Option<Tracker>> = Mutex::new(None);
/// Token pedido por la ventana que se cierra, para la siguiente.
static TOKEN: Mutex<Option<String>> = Mutex::new(None);

/// Ventana nueva: la activa con el token pendiente y empieza a escuchar su entrada.
pub(crate) fn on_window(window: &Window) {
    let tracker = track(window);
    let token = TOKEN.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let (Some(t), Some(token)) = (&tracker, token) {
        t.activation.activate(token, &t.surface);
        let _ = t.conn.flush();
        log::info!("ventana nueva activada con el token de la anterior");
    }
    if let Some(old) = std::mem::replace(
        &mut *TRACKER.lock().unwrap_or_else(|e| e.into_inner()),
        tracker,
    ) {
        old.release();
    }
}

/// Cada frame: procesa los clics y teclas recibidos, para que el serial esté al día y
/// la cola no acumule los movimientos del puntero.
pub fn poll() {
    if let Some(t) = TRACKER.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let _ = t.queue.dispatch_pending(&mut t.input);
    }
}

/// Antes de cerrar la ventana para recrearla: pide el token con el último clic o tecla.
pub fn prepare() {
    let Some(mut t) = TRACKER.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return;
    };
    let _ = t.queue.dispatch_pending(&mut t.input);
    let qh = t.queue.handle();
    let token = t.activation.get_activation_token(&qh, ());
    if let (Some(seat), Some(serial)) = (&t.input.seat, t.input.serial) {
        token.set_serial(serial, seat);
    } else {
        log::warn!("token de activación sin clic ni tecla: la ventana nueva puede quedar detrás");
    }
    token.set_surface(&t.surface);
    token.commit();
    // El compositor responde en orden: tras la ida y vuelta, `done` ya llegó.
    if t.queue.roundtrip(&mut t.input).is_err() {
        log::warn!("no se pudo pedir el token de activación");
    }
    token.destroy();
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = t.input.token.take();
    t.release();
}

/// La ventana se cierra sin recrearse: suelta la escucha antes que su conexión.
pub fn forget() {
    if let Some(t) = TRACKER.lock().unwrap_or_else(|e| e.into_inner()).take() {
        t.release();
    }
}

fn track(window: &Window) -> Option<Tracker> {
    let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
        return None;
    };
    let RawWindowHandle::Wayland(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: display y superficie son de la ventana de winit recién creada; la escucha
    // se suelta (`prepare`/`forget`) antes de que la ventana y su conexión se cierren.
    let conn = Connection::from_backend(unsafe {
        Backend::from_foreign_display(display.display.as_ptr().cast())
    });
    let surface =
        unsafe { ObjectId::from_ptr(WlSurface::interface(), handle.surface.as_ptr().cast()) }
            .ok()
            .and_then(|id| WlSurface::from_id(&conn, id).ok())?;
    let (globals, queue) = registry_queue_init::<Input>(&conn).ok()?;
    let qh = queue.handle();
    let activation: XdgActivationV1 = globals.bind(&qh, 1..=1, ()).ok()?;
    let mut input = Input {
        seat: globals.bind(&qh, 5..=7, ()).ok(),
        ..Default::default()
    };
    // Sin esta ida y vuelta las capacidades del seat solo llegaban al pedir el token:
    // el puntero y el teclado se creaban tarde y nunca había serial.
    let mut queue = queue;
    queue.roundtrip(&mut input).ok()?;
    Some(Tracker {
        conn,
        queue,
        input,
        activation,
        surface,
    })
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Input {
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
impl Dispatch<wl_seat::WlSeat, ()> for Input {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}
impl Dispatch<wl_pointer::WlPointer, ()> for Input {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Solo la pulsación: Mutter valida el token contra el serial con que empezó el
        // clic, y egui reacciona al soltar, así que el último evento era siempre el de
        // soltar y el token se descartaba.
        if let wl_pointer::Event::Button {
            serial,
            state: WEnum::Value(wl_pointer::ButtonState::Pressed),
            ..
        } = event
        {
            state.serial = Some(serial);
        }
    }
}
impl Dispatch<wl_keyboard::WlKeyboard, ()> for Input {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            serial,
            state: WEnum::Value(wl_keyboard::KeyState::Pressed),
            ..
        } = event
        {
            state.serial = Some(serial);
        }
    }
}
impl Dispatch<XdgActivationTokenV1, ()> for Input {
    fn event(
        state: &mut Self,
        _: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            state.token = Some(token);
        }
    }
}
wayland_client::delegate_noop!(Input: ignore XdgActivationV1);
