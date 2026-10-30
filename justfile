# Tareas del proyecto (equivalente a los "scripts" de package.json).
# `just` lista todo; `just <tarea>` ejecuta una.

# Carpeta donde se enlaza el fake como `llama-server`.
fake_bin := justfile_directory() / "target/fake-bin"

# Lista las tareas disponibles.
default:
    @just --list

# Arranca la app egui (debug).
dev:
    cargo run -p plane-llama

# Arranca la app egui con el fake-llama-server como `llama-server` (sin GPU ni modelo).
dev-fake: fake-link
    PATH="{{fake_bin}}:$PATH" cargo run -p plane-llama

# Arranca la app egui en release.
run:
    cargo run -p plane-llama --release

# Corre el fake-llama-server solo (para probar el core o la versión Tauri).
fake *args:
    cargo run -p fake-llama-server -- {{args}}

# Compila el fake y lo enlaza como `llama-server` en target/fake-bin.
fake-link:
    cargo build -p fake-llama-server
    mkdir -p {{fake_bin}}
    ln -sf {{justfile_directory()}}/target/debug/fake-llama-server {{fake_bin}}/llama-server

# Tests de todo el workspace.
test:
    cargo test --workspace

# Tests incluyendo los ignorados (p. ej. compatibilidad con el history.json real).
test-all:
    cargo test --workspace -- --include-ignored

# Capturas PNG de todas las páginas con el historial real (necesita GPU).
screenshots dir="target/screenshots":
    SCREENSHOT_DIR="{{dir}}" cargo test -p plane-llama-ui --test screenshots -- --ignored
    @echo "Capturas en {{dir}}"

# Formato y clippy, como en CI (falla si hay algo).
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Formatea el código.
fix:
    cargo fmt --all

# Build de release de todo el workspace.
build:
    cargo build --workspace --release

# Regenera assets/llama-flags.json desde el catálogo del frontend de llama-bench.
export-flags:
    bun scripts/export-flags.ts

# Genera el AppImage de la versión egui (fase 5).
appimage:
    scripts/appimage.sh

# Compara tamaño, arranque, RAM y CPU de Electron, egui y Tauri (fase 6).
compare:
    scripts/compare.sh

# Borra los artefactos de compilación.
clean:
    cargo clean

# Instala la versión release para el usuario actual.
install:
    cargo build --release -p plane-llama
    install -Dm755 target/release/plane-llama "$HOME/.local/bin/plane-llama-egui"
    install -Dm644 assets/plane-llama-egui.png "$HOME/.local/share/icons/hicolor/256x256/apps/plane-llama-egui.png"
    install -Dm644 assets/plane-llama.svg "$HOME/.local/share/icons/hicolor/scalable/apps/plane-llama-egui.svg"
    install -Dm644 assets/plane-llama-symbolic.svg "$HOME/.local/share/icons/hicolor/symbolic/apps/plane-llama-egui-symbolic.svg"
    mkdir -p "$HOME/.local/share/applications"
    printf '[Desktop Entry]\nType=Application\nName=Plane llama\nComment=Benchmark de llama-server\nExec="%s/.local/bin/plane-llama-egui"\nIcon=plane-llama-egui\nTerminal=false\nStartupWMClass=plane-llama-egui\n' "$HOME" > "$HOME/.local/share/applications/plane-llama-egui.desktop"
    # Sin regenerar la caché, GNOME no ve el icono nuevo si hicolor ya tenía una.
    -gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor"

# Retira la instalación y el arranque automático; conserva los datos.
uninstall:
    rm -f "$HOME/.local/bin/plane-llama-egui" "$HOME/.local/share/applications/plane-llama-egui.desktop" "$HOME/.local/share/icons/hicolor/256x256/apps/plane-llama-egui.png" "$HOME/.local/share/icons/hicolor/scalable/apps/plane-llama-egui.svg" "$HOME/.local/share/icons/hicolor/symbolic/apps/plane-llama-egui-symbolic.svg" "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/plane-llama-egui.desktop"
    -gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor"
