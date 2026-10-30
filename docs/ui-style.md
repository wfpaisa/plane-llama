# Estilo de interfaz

Guía de la interfaz egui de plane-llama. Los valores viven en
`crates/ui/src/theme.rs` y los componentes en `crates/ui/src/widgets.rs`.

## Principios

1. **Todo color, espaciado, radio y tamaño sale de `Tokens`.** Ningún componente
   escribe colores literales; el test `theme::tests::colours_only_in_theme` falla si
   aparece uno fuera de `theme.rs`.
2. **Jerarquía clara**: una acción principal por zona (violeta), el resto en
   botones secundarios o de icono. Cifras importantes grandes, detalle en rejillas.
3. **Superficies por capas**: barra lateral (la más oscura) → fondo → tarjetas →
   controles hundidos (editor, prompt, logs).
4. **Consumo mínimo**: nada de animaciones en bucle. Solo transiciones cortas de
   egui (hover, interruptores); el reloj del benchmark repinta a 5 fps mientras corre.

## Paleta

| Token | Oscuro | Claro | Uso |
|---|---|---|---|
| `sidebar` | 16, 17, 22 | 239, 240, 244 | Barra lateral |
| `bg` | 21, 22, 28 | 247, 248, 250 | Fondo del contenido y barra superior |
| `surface` | 28, 30, 37 | 255, 255, 255 | Tarjetas y diálogos |
| `surface_alt` | 36, 38, 47 | 243, 244, 247 | Botones secundarios, chips, pistas |
| `sunken` | 18, 19, 24 | 248, 249, 251 | Editor, prompt, logs, búsqueda |
| `border` / `border_strong` | 41, 44, 54 / 56, 60, 73 | 226, 229, 235 / 208, 212, 221 | Bordes |
| `text` / `text_dim` / `text_faint` | 233, 235, 241 / 154, 160, 176 / 105, 111, 128 | 24, 26, 33 / 90, 97, 112 / 140, 146, 160 | Texto |
| `accent` | 124, 108, 246 | 99, 82, 232 | Acción principal, selección, medidores |
| `accent_soft` / `accent_ink` | acento al 16 % / 196, 188, 255 | acento al 12 % / 78, 60, 205 | Navegación activa, filas seleccionadas |
| `ok` / `warning` / `error` | 52, 211, 153 / 251, 191, 36 / 248, 113, 113 | 5, 150, 105 / 196, 132, 4 / 220, 38, 38 | Estados, mejor valor, avisos |
| `series` | violeta, celeste, rosa, gris | ídem | Pesos, KV cache, overhead, VRAM en uso |

Cada color de estado tiene su variante `_soft` para fondos de chips y avisos.

## Tipografía

- **Inter** Regular, Medium y SemiBold para la interfaz (`theme::font`,
  `font_medium`, `font_semibold`). egui no sintetiza negritas, por eso hay tres cortes.
- **IBM Plex Mono** para script, logs y cifras de tablas.
- **Phosphor** para iconos, siempre con `theme::icon_font`: Inter trae glifos propios
  en el área de uso privado donde Phosphor pone los suyos.
- Escala: 11, 12, 13,5 (cuerpo), 15, 18 (título de página), 22, 28 (indicadores).

## Espaciado y forma

- Espacios de 4 en 4: 4, 8, 12, 16, 20, 24, 32.
- Radios: 6 (chips), 8 (controles), 12 (tarjetas y diálogos), píldora completa
  para medidores e interruptores.
- Controles de 30 px; acción principal de tarjeta de 40 px a todo el ancho.

## Estructura de la ventana

```
┌ barra lateral ─┬ barra superior: título · subtítulo        ☀ ⚙ │ ─ ☐ ✕ ┐
│ ✈ plane-llama  ├──────────────────────────────────────────────────────┤
│ NAVEGACIÓN     │ contenido de la página (máx. 1480 px de ancho)        │
│ ◆ Scripts   31 │                                                      │
│ ◇ Modelos      │                                                      │
│ ▣ Benchmark    │                                                      │
│ ◷ Historial 31 │                                                      │
│ ⚡ Optimizador  │                                                      │
│ ▤ Logs         │                                                      │
│                │                                                      │
│ HARDWARE       │                                                      │
│ RTX 5070 Ti    │                                                      │
│ nvidia0 ▓▓░░   │                                                      │
│ RAM     ▓░░░   │                                                      │
│ ● Servidor     │                                                      │
│ [▶ Iniciar]    │                                                      │
└────────────────┴──────────────────────────────────────────────────────┘
```

- Sin decoraciones de winit (`with_decorations(false)`): la barra superior y la
  cabecera de la barra lateral mueven la ventana; doble clic maximiza; los bordes
  redimensionan. Cerrar respeta «mantener en la bandeja».
- La barra lateral se contrae a 64 px, solo iconos con tooltip.
- Las páginas usan dos columnas desde 900 px de contenido; debajo se apilan.

### Marco de la ventana

La superficie es transparente (`with_transparent`, `clear_color` a cero). `window.rs`
pinta la sombra en un margen de `WINDOW_SHADOW` px, el fondo redondeado
(`WINDOW_RADIUS`) y, al final, el borde `window_border`. Los paneles no tienen relleno
salvo la barra lateral, con las esquinas izquierdas redondeadas. Las zonas de
redimensionado ocupan el margen de la sombra y no se solapan (las esquinas tienen
zona propia de `RESIZE_CORNER`). Tras mover o redimensionar, `window::release_hook`
suelta el botón en egui: el compositor se queda con la liberación y egui seguiría
creyendo que arrastra (sin scroll hasta el siguiente clic).

La ventana nueva se centra marcándola como diálogo modal con `xdg_dialog_v1` antes
de su primer frame (`center.rs`, con el hook de `vendor/egui-winit`); al segundo
frame vuelve a ser normal. Maximizada, margen y radio son cero.

Con **Desenfocar el fondo**, `theme::set_glass` baja la opacidad de `bg` y `sidebar`
(`GLASS_ALPHA`) y `blur.rs` fija la región desenfocada (el rectángulo redondeado en
franjas) con `ext_background_effect_v1`, usando la conexión Wayland de winit.

### Vista simple

Ventana compacta (`COMPACT_SIZE`) sin barra lateral: barra de 40 px con logo, nombre,
**Vista ampliada** y cerrar; debajo, los scripts visibles agrupados en filas de 30 px. Cada fila muestra
punto de estado, nombre, cuantización, ojo si es multimodal, mejor t/s y tamaño (desde
`COMPACT_WIDE`); al pasar el ratón aparece ▶ y, si corre, ■.

### Vista ampliada

- **Scripts**: barra de herramientas (búsqueda, Visibles/Ocultos/Todos, orden,
  Elegir visibles, Nuevo script), zona de actividad (benchmark en curso o último
  resultado) y una tarjeta con grupos plegables por organización. Las filas
  tienen alto fijo (`ROW_LG`) y celdas de ancho fijo alineadas a la derecha
  (`CHIP_SLOT`, `QUANT_SLOT`, `TPS_SLOT`, `ACTION_SLOT`…), para que las columnas
  coincidan entre filas.
- **Modelos**: segmentado «En el equipo / Hugging Face». Hugging Face usa dos
  columnas (resultados de `HUB_LIST` px y detalle) y se apila en ventanas
  estrechas. El detalle muestra chips (tipo, base, visión) y una fila por
  cuantización con un punto de cabida (verde VRAM, ámbar RAM, rojo no cabe).

### Acento y logo

`theme::set_accent` guarda el acento en un atómico y `Tokens::of(theme)` deriva
la paleta; el texto sobre el acento pasa a oscuro en acentos claros. El logo
(`theme::logo`) se carga una vez como textura desde `assets/plane-llama-128.png`.

En layouts de derecha a izquierda, `segmented` y los medidores en línea invierten
el orden de pintado para leerse igual. Los iconos Phosphor van en su propia
etiqueta con `icon_font`, nunca dentro de un texto Inter.

## Componentes (`widgets.rs`)

- `button` / `button_wide` / `icon_button` con `Kind::{Primary, Secondary, Ghost,
  Danger}`. Pintados a mano; el nombre accesible es la etiqueta sin el icono.
- `card` + `card_header` (icono en cuadro de color, título, subtítulo, acciones).
- `stat` (indicador grande), `metric` (celda de detalle), `grid` (rejilla adaptable).
- `toggle` (interruptor, accesible como casilla), `segmented`, `search`, `sunken`.
- `meter` / `meter_row` / `stacked_bar` / `legend` para consumos.
- `chip`, `best` (mejor valor en verde), `pill`, `status_dot`.
- `banner` (aviso en línea), `empty_state`, `dialog` + `dialog_footer`, `toasts`.

## Revisión visual

`just screenshots` genera capturas de todas las páginas, en claro y oscuro y en
ventana estrecha, con el historial real (`DATA_DIR/history.json`, solo lectura).
