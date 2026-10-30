# egui-winit 0.36.2 parcheado

Copia de [egui-winit](https://github.com/emilk/egui) 0.36.2 (MIT OR Apache-2.0,
© The egui authors) con un único cambio en `src/lib.rs`: el `static WINDOW_CREATED`
y su llamada al principio de `apply_viewport_builder_to_window`.

eframe no da acceso a la ventana de winit, y para que GNOME centre la ventana hace
falta marcarla como diálogo con `xdg_dialog_v1` antes de su primer frame. El hook lo
instala `plane_llama_ui::center`.

Para actualizar egui: copiar la versión nueva del registro, repetir el cambio y
ajustar la versión aquí y en `Cargo.toml`.
