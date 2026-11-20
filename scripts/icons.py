"""Genera los iconos de plane-llama a partir de HugeIcons (stroke rounded).

Uso: python3 -I scripts/icons.py <dir_hugeicons>
     (<dir_hugeicons> contiene fuente/icons.css, fuente/ttf/*.ttf y svg/)

Escribe:
- assets/fonts/hugeicons/hugeicons-subset.ttf: solo los glifos usados.
- crates/ui/src/icons.rs: constantes con el punto de código de cada icono.
- assets/tray/*.png: iconos del menú de la bandeja (gris medio, 32 px).
"""

import re
import subprocess
import sys
from pathlib import Path

from fontTools import subset

RAIZ = Path(__file__).resolve().parent.parent

# Constante en Rust -> nombre en HugeIcons. Los nombres de constante vienen de
# Phosphor, que era la fuente anterior, para no cambiar las llamadas.
ICONOS = {
    "AIRPLANE_TILT": "airplane-01",
    "ARROW_COUNTER_CLOCKWISE": "rotate-left-01",
    "ARROW_DOWN": "arrow-down-02",
    "ARROW_FAT_LINES_RIGHT": "arrow-right-double",
    "ARROW_SQUARE_IN": "square-arrow-right-enter",
    "ARROW_SQUARE_OUT": "link-square-02",
    "ARROW_U_UP_LEFT": "arrow-turn-backward",
    "ARROW_UP": "arrow-up-02",
    "ARROWS_CLOCKWISE": "refresh",
    "ARROWS_IN_SIMPLE": "arrow-shrink-01",
    "ARROWS_OUT_SIMPLE": "arrow-expand-01",
    "BOOK_OPEN_TEXT": "book-open-01",
    "CARET_LEFT": "arrow-left-01",
    "CARET_RIGHT": "arrow-right-01",
    "CHART_BAR": "chart-histogram",
    "CHART_LINE_UP": "chart-increase",
    "CHART_PIE_SLICE": "pie-chart",
    "CHAT_CENTERED_TEXT": "message-01",
    "CHAT_CIRCLE_TEXT": "bubble-chat",
    "CHECK": "tick-02",
    "CHECK_CIRCLE": "checkmark-circle-02",
    "CHECK_SQUARE": "checkmark-square-02",
    "CLOCK_COUNTER_CLOCKWISE": "history",
    "CODE": "code",
    "COLUMNS": "layout-three-column",
    "COPY": "copy-01",
    "CORNERS_IN": "minimize-screen",
    "CPU": "cpu",
    "CUBE": "cube",
    "DATABASE": "database",
    "DOTS_SIX_VERTICAL": "drag-drop-vertical",
    "DOTS_THREE": "more-horizontal",
    "DOWNLOAD_SIMPLE": "download-01",
    "EYE": "view",
    "EYE_SLASH": "view-off",
    "FILE_PLUS": "file-add",
    "FLOPPY_DISK": "floppy-disk",
    "FUNNEL": "filter",
    "GAUGE": "dashboard-speed-01",
    "GEAR_SIX": "settings-02",
    "HEART": "favourite",
    "IMAGE": "image-01",
    "LIGHTNING": "flash",
    "LIST_BULLETS": "left-to-right-list-bullet",
    "MAGIC_WAND": "magic-wand-01",
    "MAGNIFYING_GLASS": "search-01",
    "MEMORY": "ram-memory",
    "MINUS": "minus-sign",
    "MOON": "moon-02",
    "PENCIL_SIMPLE": "pencil-edit-01",
    "PLAY": "play",
    "PLUS": "add-01",
    "PUSH_PIN": "pin",
    "ROCKET_LAUNCH": "rocket-01",
    "RULER": "ruler",
    "SIDEBAR_SIMPLE": "sidebar-left",
    "SORT_ASCENDING": "sorting-01",
    "SQUARE": "square",
    "SQUARES_FOUR": "grid-view",
    "STACK": "layers-01",
    "STAR": "star",
    "STOP": "stop",
    "SUN": "sun-03",
    "TERMINAL_WINDOW": "terminal",
    "TEXT_INDENT": "text-indent",
    "TIMER": "timer-02",
    "TRASH": "delete-02",
    "WARNING": "alert-02",
    "WARNING_CIRCLE": "alert-circle",
    "X": "cancel-01",
}

# Archivo PNG del menú de la bandeja -> nombre en HugeIcons.
BANDEJA = {
    "open": "app-window",
    "server": "server-stack-01",
    "start": "play",
    "stop": "stop",
    "quit": "logout-03",
}
COLOR_BANDEJA = "#808080"  # se lee sobre menús claros y oscuros

PATRON = re.compile(
    r'\.hgi-stroke\.hgi-([a-zA-Z0-9-]+)::before\s*\{[^}]*?content:\s*"\\([0-9a-fA-F]+)"'
)


def main(origen):
    origen = Path(origen)
    css = (origen / "fuente/icons.css").read_text(encoding="utf-8")
    codigos = {n: int(c, 16) for n, c in PATRON.findall(css)}
    faltan = [n for n in ICONOS.values() if n not in codigos]
    if faltan:
        sys.exit(f"Sin punto de código: {faltan}")

    ttf = next((origen / "fuente/ttf").glob("*.ttf"))
    destino = RAIZ / "assets/fonts/hugeicons/hugeicons-subset.ttf"
    destino.parent.mkdir(parents=True, exist_ok=True)
    opciones = subset.Options()
    opciones.layout_features = []
    opciones.notdef_outline = True
    fuente = subset.load_font(str(ttf), opciones)
    sub = subset.Subsetter(opciones)
    sub.populate(unicodes=sorted({codigos[n] for n in ICONOS.values()}))
    sub.subset(fuente)
    subset.save_font(fuente, str(destino), opciones)

    lineas = [
        "//! Iconos de HugeIcons (stroke rounded) como puntos de código de la fuente",
        "//! `hugeicons-subset.ttf`. Generado por `scripts/icons.py`; no editar a mano.",
        "",
    ]
    for const, nombre in sorted(ICONOS.items()):
        lineas.append(f"/// `{nombre}`")
        lineas.append(f'pub const {const}: &str = "\\u{{{codigos[nombre]:x}}}";')
    (RAIZ / "crates/ui/src/icons.rs").write_text("\n".join(lineas) + "\n", encoding="utf-8")

    bandeja = RAIZ / "assets/tray"
    bandeja.mkdir(parents=True, exist_ok=True)
    for archivo, nombre in BANDEJA.items():
        svg = (origen / "svg" / f"{nombre}.svg").read_text(encoding="utf-8")
        svg = svg.replace('fill="currentColor"', f'fill="{COLOR_BANDEJA}"')
        subprocess.run(
            ["rsvg-convert", "-w", "32", "-h", "32", "-o", str(bandeja / f"{archivo}.png")],
            input=svg.encode(),
            check=True,
        )
    print(f"{len(ICONOS)} iconos, {len(BANDEJA)} PNG de bandeja")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("Uso: python3 -I scripts/icons.py <dir_hugeicons>")
    main(sys.argv[1])
