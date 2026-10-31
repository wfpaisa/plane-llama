# Progreso

## Fase 1 completada y commiteada

Workspace Rust estable (edición 2024), contratos JSON, configuración, estado por
instancia, logs con cursor monotónico, diagnóstico, parser de scripts, métricas,
GPU NVIDIA/AMD, RAM, devices e historial con escritura atómica y cap de 200.
El usuario confirmó que el commit de esta fase ya existe.

Se conservan campos adicionales y ausencias de las entradas antiguas; los
números enteros se serializan como enteros JSON. La persistencia es inmediata y
atómica, en lugar del debounce del TS, para no perder cambios al cerrar.

Fixtures: logs copiados del diagnóstico real y meminfo leído del sistema.
Devices y CSV NVIDIA son sintéticos con el formato del referente.

### Correcciones incluidas durante la Fase 2

- La velocidad de generación ignora las líneas `prompt eval time`, igual que
  los otros campos de generación. Hay una regresión con solo prompt y otra
  con prompt posterior a la generación.
- Las regex de métricas y devices se compilan una sola vez mediante `LazyLock`.
- Se verificaron las lecturas existentes de GPU (`nvidia-smi` y sysfs AMD) y
  RAM (`/proc/meminfo`). Se agregó `list_devices` con entorno del binario,
  timeout y lectura de stdout/stderr.

## Fase 2 completada y commiteada

1. `tools/fake-llama-server`: Tokio + axum, listado de devices, flags ignorados,
   host/puerto, logs de carga, health, chat OpenAI con métricas, crash configurable
   y SIGTERM limpio, también durante la carga. Permite demorar chat y timings
   para probar cancelación y llegada tardía de métricas.
2. `server_manager`: búsqueda en PATH, cwd y LD_LIBRARY_PATH del binario,
   grupo propio, drenaje concurrente de stdout/stderr, disponibilidad por logs,
   fallo rápido con código de salida, SIGTERM y escalada a SIGKILL. El grupo se
   limpia también si termina primero el líder. Un cierre posterior puede acortar
   el timeout de otro cierre que ya esté en curso.
3. `benchmark`: baseline concurrente, disponibilidad, health con fallback a `/`,
   sampling condicional, tokens ilimitados como -1, respuesta/reasoning, polling
   de métricas, deltas de GPU/RAM/devices, diagnóstico y persistencia. Cancelación
   durante carga, health, request y esperas; limpieza incluso si se abandona el
   futuro. Los errores tempranos producen resultados sin métricas, como en TS;
   el crash de carga y la cancelación no guardan resultados.
4. `optimizer`: nombres, cuantizaciones, arquitectura por tamaño, GGUF limitado
   a 2 MiB y tolerante a cortes (escalares, strings y arrays), capas híbridas,
   resolución de archivo/directorio y cache HF con HF_HOME, quant y mmproj,
   heurística y recomendación por búsqueda binaria.
5. `dryfit`: baseline, carga, estabilización de 4,5 s, delta de devices con
   fallback GPU, tiempo de carga, fits, errores inline y cierre garantizado.
6. `shutdown`: cancela la operación activa y solicita cierre con timeout de 3 s;
   no instala señales en las aplicaciones consumidoras.
7. `Core`: todos los métodos del contrato, clonable y con estado por instancia.
   Reservas compartidas impiden concurrencia de benchmark, calibración y servidor
   manual; los conflictos usan `CoreError::Conflict`. Defaults, favoritos, logs,
   suscripción de eventos, hardware e historial están expuestos.
8. `client`: formatos es-CO, modelos/badges, VRAM, escritor de flags con aliases
   y controles habilitados, estimación instantánea y derivados de estado (bests,
   modelos, filtros, selección, orden y gráficos). Exportador Bun y catálogo de
   237 flags incluido con `include_str!`.
9. Pruebas de integración reales contra el fake, sin GPU ni modelo y con
   directorios/puertos aislados.

### Validación

- `cargo fmt --all`.
- `cargo clippy --workspace --all-targets -- -D warnings` sin advertencias.
- `cargo test --workspace`: 21 pruebas activas en verde; una ignorada por defecto
  para leer el historial real del referente.
- 9 pruebas de procesos: start/ready/health/stop y eventos; benchmark con métricas
  y persistencia; conflictos y cancelación durante carga; servidor manual,
  shutdown y futuro abandonado; defaults/devices/estimate/dryfit; crash en menos
  de 2 s; cancelación durante request y calibración; timings tardíos y contrato
  HTTP; cierre de grupo y escalada a SIGKILL. Se comprueba ausencia de procesos
  residuales por PID y cierre de descendientes.
- 6 pruebas nuevas puras: regresión de métricas; GGUF sintético, híbridos y cortes
  en cada offset; diferencias de estimación backend/cliente; escritor de flags;
  formatos/catálogo/derivados; cache HF y selección por quant con mmproj.
- 6 pruebas heredadas de parsers, logs, contratos e historial siguen en verde.

### Decisiones y desviaciones

El referente TS actual manda sobre el plan: se usa polling de métricas hasta 5 s
(intervalo 150 ms), en vez del `sleep(400)` mencionado en el plan. Se conservan
sus diferencias entre heurísticas: backend con overhead base 256 MiB y pesos por
parámetros; cliente con base 128 MiB y fracción offload. Si hay archivo GGUF, el
backend usa su tamaño y la fracción offload, igual que el TS.

Las pruebas de procesos están físicamente en `crates/core/tests`, pero Cargo las
registra en el paquete del fake para disponer de `CARGO_BIN_EXE` y construir el
binario automáticamente. No invocan Cargo dentro de los tests.

El escritor reconstruye rutas con espacios entre comillas para que el parser
Rust pueda volver a leerlas. Los tipos de cache draft del frontend se conservan
en `TunedParams.extra`; no se añaden campos draft al JSON del backend cuando
este no los devuelve.

El orden textual usa ICU con `en-US`, el locale por defecto detectado en Bun
en este entorno, para reproducir `localeCompare` en vez de ordenar por bytes.

Por instrucción explícita, no se intentó escribir en `.git` ni crear el commit.
Al cerrar la Fase 2 todavía no se había iniciado la UI. El usuario confirmó
después que las Fases 1 y 2 están commiteadas. Las pruebas automáticas usan
hardware simulado; no validan un modelo o GPU físicos.


## Fase 3 completada; sin commit por instrucción del usuario

### Implementación

- `crates/ui` (`plane-llama-ui`) y `apps/plane-llama` (paquete y binario
  `plane-llama`) incluidos en el workspace. `just dev` y `just dev-fake`
  ejecutan la aplicación nativa; el justfile existente no necesitó cambios.
- eframe, egui y egui_extras 0.36.2, con renderer **wgpu**, fuentes incluidas,
  X11/Wayland, accesibilidad y persistencia. egui_plot **0.37.0** es la versión
  compatible con egui 0.36; egui_plot 0.36 depende de egui 0.35.
- Ventana `plane-llama` de 1440×900, mínimo 760×480, tema oscuro.
  `eframe::Storage` usa `DATA_DIR/ui.ron`; los defaults, favoritos de flags e
  historial siguen en los archivos del core. No se usa la carpeta del referente.
- Runtime Tokio separado del hilo gráfico. El cierre de eframe avisa por
  oneshot, ejecuta `Core::shutdown()` y espera el runtime fuera del hilo de UI.
- Puente por canal: acciones async, resultados, suscripción a logs/status y
  recuperación del backlog si broadcast pierde mensajes. Hardware cada cuatro
  segundos. Los eventos despiertan egui; no hay un bucle de repintado continuo.
  El temporizador de 200 ms existe solo durante el benchmark. Cada drenaje
  procesa hasta 1024 mensajes para que un aluvión de logs no monopolice el frame;
  si queda trabajo, solicita otro repaint. También drena con la ventana oculta.
- Store con precedencia storage → defaults del core → fallback, derivados de
  `core::client`, logs deduplicados con cursor monotónico y cap de 5000, estado
  de ejecución/cancelación, selección, filtro y orden de tres estados.
- Las seis claves pedidas conservan sus nombres y valores: script, prompt,
  sort (con migración de aliases), max tokens, habilitado y columnas visibles.
  El referente actual incluye una **séptima** clave, calibración por modelo:
  también se conserva con medición y heurística de referencia. El filtro de
  modelos permanece efímero, como en TS.
- Un módulo por sección: status_bar, script_editor, benchmark_panel, gpu_grid,
  logs_viewer, response_card, last_result, history_table, compare_modal,
  chart_modal y optimizer_modal. Textos, columnas y lógica consultados en todos
  los `.ts`/`.html` de features, el store y storage del referente.
- Pestañas Script server / Benchmark; editor monoespaciado, formateo, guardar y
  obtener defaults con confirmación; catálogo incluido, búsqueda, grupos,
  favoritos persistidos, detección de flags presentes y vista ampliada.
- Benchmark con prompt, defaults, reset, límite opcional de tokens, tiempo
  transcurrido y cancelación. Respuesta seleccionable, métricas, errores,
  tarjetas de GPU/RAM y logs virtualizados con autoscroll y timestamp opcional.
- Historial con TableBuilder, las 23 columnas conmutables, paginación 25/50/100,
  filtro múltiple por modelo, selección, favoritos, diez estrellas (repetir la
  calificación la limpia a null), aplicar script y borrado con confirmación.
  Resalta los mejores valores sobre todo el historial y permite maximizarlo.
- Comparación transpuesta y gráfico de once métricas, orden de tabla capturado
  al abrir, etiquetas modelo/quant, tooltip y mejor valor. Son modales que
  bloquean la interacción con la pantalla de fondo.
- Optimizador con copia temporal, metadatos y devices cargados al abrir, todos
  los controles del referente, tensor split automático/manual, flags opcionales,
  Default, Cancelar y Aplicar. Las barras se calculan localmente con
  `client::vram_estimate`; la calibración usa dryfit, baseline y delta heurístico
  por modelo. Las respuestas de estimaciones antiguas se descartan por sesión.
  Desactivar o cerrar la calibración solicita cancelación sin liberar antes de
  tiempo el estado de operación en curso.

### Validación

- `cargo fmt --all` y comprobación de formato.
- `cargo clippy --workspace --all-targets -- -D warnings` en verde.
- `cargo test --workspace` en verde: 30 pruebas activas y una ignorada heredada
  para el historial real del referente.
- Ocho tests de UI/estado: clicks en Play/Stop, límite y cancelación, selección,
  orden de tres estados, comparar/chart/borrado, favoritos y diez estrellas,
  persistencia, logs y optimizador (Default, Cancelar, Aplicar y respuestas viejas).
- Test de punta a punta registrado en el paquete fake: arranca la app en
  egui_kittest, pulsa Benchmark, ejecuta un proceso fake real y comprueba
  respuesta, métricas, logs, history.json y cierre. Comprueba además que los
  frames se estabilizan cuando termina la operación.
- `cargo build --release -p plane-llama` en verde.
- Pruebas nativas con Xvfb y Mesa: ventana release, Play mediante eventos X11,
  health del fake, cierre por WM_DELETE_WINDOW, ui.ron escrito y ningún fake
  huérfano. También se ejecutaron las recetas reales `just dev-fake` y `just dev`.

### Decisiones, diferencias y pendientes respecto al original

**No se modificó ningún archivo de crates/core ni su API pública.** No hicieron
falta agregados. El helper privado de selección de devices se replica en la UI
como un filtro pequeño, sin cambiar su visibilidad en core.

La presentación es nativa egui: TextEdit monoespaciado en lugar de CodeMirror,
sin resaltado de sintaxis; controles, desplazamiento y tooltips de egui en lugar
de PrimeNG y Chart.js. No reproduce píxel por píxel CSS, iconos ni animaciones.
Las fechas se muestran en hora local con formato español. Favoritos/calificación
se reflejan al confirmar el core, en lugar del cambio optimista y rollback del
frontend. Las barras por device muestran el baseline en el texto y en el total;
no reproducen los dos segmentos de color del original.

El entorno de prueba no expone GPU acelerada a wgpu: se usó su backend GL sobre
Mesa llvmpipe, manteniendo el renderer wgpu de eframe. Xvfb requería
`__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json` y quitar
WAYLAND_DISPLAY para evitar el driver NVIDIA del host y aislar la ventana.
El muestreo breve de CPU con llvmpipe dio aproximadamente 4–8 %; no equivale a
una medición con GPU física. La ausencia de repaint continuo está probada;
queda pendiente medir el objetivo de ~0 % en una sesión acelerada y ejecutar
benchmarks con GPU/modelo reales. La comparación formal corresponde a la Fase 6.

No se tocaron las fases de Tauri, empaquetado ni comparación. No se escribió en
`.git` ni se creó un commit. El commit queda a cargo del usuario.

## Segundo plano y arranque automático de egui (2026-10-24)

### Implementado

- Bandeja StatusNotifierItem con ksni, icono PNG embebido, abrir por activación,
  estado por `Core::subscribe()`, iniciar con el script guardado o el default,
  detener y salir. Los callbacks delegan las operaciones al runtime.
- Cerrar oculta con `CancelClose` y `Visible(false)` según los ajustes.
  Salir omite esa opción y ejecuta el cierre ordenado existente del core.
- Visibilidad compartida mediante `Arc<AtomicBool>`: los eventos no piden
  repintados cuando está oculta y no se acumulan logs ocultos en el canal.
  El polling de GPU queda esperando una notificación; al abrir se refrescan
  logs, estado y GPU. Los componentes y sus timers solo se ejecutan visibles.
- `--hidden`, app_id `plane-llama-egui` y socket de instancia única por sesión
  en XDG_RUNTIME_DIR. Un bloqueo breve con fs2 serializa el arranque y permite
  recuperar sockets obsoletos sin que dos procesos reserven la misma instancia.
- Ejecución inicial de `launch_script` y diálogo de ajustes con copia del script,
  resumen en una línea, guardar y cancelar. Se reutiliza `desktop::save_settings`.
- `just install` y `just uninstall`, con binario, acceso e icono por usuario.
  README explica la bandeja y que el autostart apunta al ejecutable que lo activó.

### Decisiones y validación

No se modificó ningún archivo ni API pública de `crates/core`; no hubo agregados.
Se mantiene glow por defecto y la presentación existente, con el botón de ajustes.
Si falla crear la bandeja, se muestra la ventana y se desactiva cerrar a bandeja
en memoria para que la aplicación siga accesible.

Se encontró en la prueba nativa que eframe 0.36 muestra el viewport raíz tras
el primer render incluso con `with_visible(false)`. La lógica inicial manda
también `Visible(false)` una sola vez para conservar el arranque oculto sin
introducir un ciclo de repintado.

Tests añadidos: instancia secundaria solicita mostrar, recuperación de socket,
ocho arranques concurrentes con un único propietario, bridge oculto sin
repintados y polling con callback contado y reloj Tokio pausado; controles de
ajustes con egui_kittest (opciones, copiar y quitar script).

`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` y `cargo build --release -p plane-llama` en verde.
El test del historial local del referente conserva su `ignore` existente.

No se creó commit, según lo solicitado. La prueba con GPU física y GNOME Wayland
queda pendiente: `nvidia-smi` no puede comunicarse con el driver en este entorno.

Prueba nativa adicional del binario release en Xvfb/Mesa con un watcher D-Bus
simulado y fake-llama-server real: se verificaron arranque oculto, ejecución del
script inicial y health, segunda instancia mostrando la ventana, cierre que
oculta, activación que abre, estado «Servidor: corriendo» y «Salir» cerrando el
proceso y dejando de responder el servidor. CPU del proceso durante 15 s por
estado: **oculta 0,0 %**, **visible 3,0 %** (100 % equivale a un núcleo).
Son cifras con renderizado por software llvmpipe y glow; VRAM no disponible por
el fallo del driver NVIDIA. No representan el rendimiento de GNOME con GPU física.

## Rediseño Solidcraft de egui (2026-10-24)

### Implementado

- Leídos el encargo `docs/ui-style.md` y los módulos de referencia del clon local:
  theme, palettes, fonts, titlebar, toolbar y diálogos (incluidos dense y frame).
  El referente se mantuvo de solo lectura.
- `theme::Tokens` concentra las dos paletas, escala tipográfica, espaciados,
  radios y dimensiones. Oscuro por defecto; Claro y Sistema se eligen en Ajustes.
  Las dos variantes de estilo se instalan al crear la app; no se reconstruyen
  en cada frame. La preferencia se guarda en `ui.ron` con la apariencia del lateral.
- IBM Plex Sans Variable y Mono Regular embebidas, copiadas con sus OFL originales
  a `assets/fonts`. Iconos Phosphor 0.14, compatible con egui 0.36. Atribuciones
  del diseño/paleta de Solidcraft (MIT OR Apache-2.0), fuentes e iconos en
  `ATTRIBUTION.md`; licencias originales de Solidcraft incluidas en assets.
- Ventana sin decoraciones, barra propia con arrastre y doble clic sobre espacio
  vacío, minimizar, maximizar/restaurar y cerrar. Ocho zonas de borde/esquina
  envían BeginResize. Cerrar envía Close y conserva el interceptor de bandeja.
  En main solo se cambiaron las decoraciones y las dimensiones mediante Tokens;
  señales, instancia única, runtime, --hidden y renderer glow siguen intactos.
- Toolbar con Ejecutar, Historial, Optimizar y Logs; selección con línea azul,
  acciones de servidor y botón para mostrar/ocultar hardware. Script y prompt
  comparten columnas cuando hay espacio; se apilan en ventanas pequeñas.
- Lateral derecho ajustable con hardware y último resultado. Su visibilidad y
  ancho persisten. Status bar con estado, VRAM, tokens/s y versión.
- Paneles con cabecera de 24 px y títulos con espaciado de letras; superficies
  por capas, controles hundidos y bordes finos. Diálogos de ajustes, optimizador,
  comparación, gráfico y confirmaciones con cabecera/cierre, fondo atenuado y
  pie de acciones. Primarios azules, sin sombras grandes.
- Historial con filas de 24 px, cabeceras diferenciadas, zebra, hover y selección;
  cifras en Plex Mono alineadas a la derecha. Logs conservan virtualización y
  aprovechan la altura disponible. Píldora de estado con punto dibujado y avisos
  abajo a la derecha con borde de 3 px según éxito, advertencia o error.

### Validación

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` y `cargo build --release -p plane-llama` en verde.
- Se conservan todos los tests anteriores y se actualizan los selectores de
  Iniciar/Detener servidor, Ejecutar benchmark y Gráfico.
- Cinco tests nuevos: prohibición recursiva de colores fuera de theme.rs;
  paletas/proporciones; persistencia de apariencia y defaults; botones de título
  enviando los comandos nativos; navegación, lateral y selector de tema con
  estabilización de frames. Continúan las pruebas de instancia única, bridge
  oculto sin repintado/polling y benchmark real con fake sin bucle de repintado.
- Revisión visual nativa del binario release con Xvfb/Mesa: Ejecutar en oscuro,
  Ajustes en oscuro y cambio a Claro. Capturas de revisión en
  `target/ui-review` (no se distribuyen). Se corrigieron glifos ausentes y sombras.
- Muestra inicial de CPU visible en reposo: 1,6 % durante cinco segundos con
  llvmpipe. No valida el objetivo de ~0 % con GPU física. Los tests comprueban
  ausencia de repintado continuo y suspensión del bridge oculto; no se añadió
  ningún temporizador periódico por el estilo.

### Diferencias y pendientes frente a docs/ui-style.md

- Optimizar es una entrada de toolbar que abre el diálogo existente y conserva
  la vista de trabajo debajo, para mantener edición temporal, cancelar y aplicar.
- El catálogo queda plegado debajo del script para dar espacio al prompt; en
  anchos pequeños las columnas se apilan. El lateral contiene las métricas
  completas del último resultado, además del resumen en la barra de estado.
- El cambio de tema se aplica al seleccionarlo; Cancelar en Ajustes descarta los
  ajustes de escritorio, mientras el tema conserva la elección visible.
- Se usa Phosphor compatible; no hace falta la alternativa Lucide/SVG.
- Quedan por comprobar rendimiento con GPU física y movimiento/redimensionado
  en GNOME Wayland. Xvfb permite revisar superficies y controles, pero no
  sustituye al compositor de la sesión real.

No se modificó `crates/core` ni se escribió en `.git`. No se creó commit, por
instrucción explícita del usuario. No se avanzó en Tauri, empaquetado ni comparación.

## Rediseño moderno de egui (2026-10-24)

Hecho a mano, sin codex. Reemplaza el estilo Solidcraft.

- Paleta propia con acento violeta (oscuro y claro), Inter Regular/Medium/SemiBold
  y Plex Mono. Phosphor se pinta con su propia familia (`theme::icon_font`).
- Barra lateral con navegación, medidores de hardware y tarjeta del servidor;
  contraíble a iconos. Barra superior con título de página, cambio rápido de
  tema, ajustes y botones de ventana.
- Benchmark: indicadores grandes, script y prueba lado a lado, respuesta con
  copia y detalle en rejilla. Catálogo de flags en diálogo con búsqueda,
  categorías y destacados.
- Historial: barra de filtros, barra de acciones al seleccionar, tabla con chips
  de modelo y mejor valor resaltado. Optimizador como página con barra apilada
  de VRAM. Logs con filtro y números de línea. Diálogos y avisos rediseñados.
- `just screenshots` genera capturas con wgpu. Tests actualizados a las nuevas
  etiquetas; workspace, clippy y fmt limpios.

## Biblioteca de scripts, modelos y vista simple (2026-10-24)

- Core: `models` (caché de Hugging Face, cuantizaciones, partes, mmproj),
  `hub` (búsqueda, detalle con `blobs=true`, descarga reanudable al formato de la
  caché) y `scripts` (`scripts.json`, sembrado desde el historial).
- UI: vista simple por defecto con **Scripts** y **Modelos**; la extendida
  conserva todo lo anterior y suma esas dos páginas. Diálogo de script con
  selector de modelos descargados.
- GPU con nombre (`nvidia-smi --query-gpu=…,name`, `product_name` en AMD) e
  identificador. Color de acento en Ajustes. Logo de plane-llama-bench en la
  app, la ventana y el acceso.
- Lógica revisada: la resolución de `-hf` exige la cuantización exacta y suma las
  partes; el benchmark falla en cuanto muere el proceso y espera `/health`
  hasta 300 s; `url_for` traduce `0.0.0.0` y `::` a direcciones locales.
- Validación: tests del workspace (incluido el recorrido de la vista simple y la
  descarga reanudable contra un servidor local), clippy y fmt limpios; test
  `live` ignorado contra la caché real y huggingface.co. En reposo: 0 ticks de
  CPU en 10 s, ~154 MB de RAM y 32 MiB de VRAM.
- Sin verificar: una descarga grande desde la UI. La app Tauri no tiene aún estas
  funciones.

## Ventana compacta, marco propio y desenfoque (2026-10-24)

- Vista simple como ventana pequeña: lista de scripts sin buscador, clic para
  iniciar o detener y menú contextual; solo **Vista ampliada** y cerrar. Cambiar de
  vista recrea la ventana para que GNOME la centre (`Desktop::reopen`).
- Marco propio: ventana transparente con sombra, esquinas redondeadas y borde claro.
- **Desenfocar el fondo** en Ajustes con `ext_background_effect_v1` (GNOME 51).
  Verificado contra el compositor real: el protocolo responde con capacidad de blur.
- Anchos adaptables: sin tope de ancho de contenido; filas y barras ocultan columnas
  o etiquetas en ventanas estrechas.
- Nombre visible «Plane llama» (`desktop::APP_NAME`).
- En reposo con desenfoque: 0 ticks de CPU en 10 s y ~150 MB de RAM. No se pudo
  capturar la pantalla real (GNOME no permite capturas desde fuera).

## Centrado, scroll y esquinas (2026-10-24)

- La ventana se abre centrada (también al cambiar de vista): se marca como diálogo
  modal con `xdg_dialog_v1` antes del primer frame y vuelve a normal al segundo. eframe
  no da acceso a la ventana de winit, así que `vendor/egui-winit` añade un hook.
  Verificado contra GNOME 51: el protocolo acepta la marca y la quita sin errores.
- Ventana compacta de 480×340 con filas de 30 px.
- Scroll: tras mover o redimensionar, el compositor se quedaba con la liberación del
  botón y egui seguía «arrastrando», sin scroll hasta el siguiente clic. Ahora se
  inyecta la liberación (`window::release_hook`).
- Esquinas: las zonas de los bordes se solapaban con las de las esquinas y quedaban
  encima tras usarlas; ya no se solapan y las esquinas son más grandes.
- El diálogo de script deja el pie fijo y desplaza los campos en ventanas bajas.

## Icono, márgenes y vista compacta (2026-10-24)

- Icono: `just install` regenera `icon-theme.cache` de `~/.local/share/icons/hicolor`.
  La caché era anterior al icono y GNOME no lo encontraba.
- Márgenes: `widgets::middle` centra en vertical las columnas de texto dentro de filas
  (título de página, marca de la barra lateral, filas de Hugging Face); `ui.vertical`
  las pegaba arriba. Filas de Hugging Face de 56 px.
- Cambiar de vista redimensiona la ventana en vez de recrearla. La ventana nueva quedaba
  detrás de las demás: Mutter solo la activa con un token de xdg-activation emitido por
  una superficie viva y enfocada, y la vieja ya estaba destruida. Ya no se recentra al
  cambiar de vista (sí al abrirla).
- Vista compacta de 300×250 (mínimo 260×200) con las mismas filas, cabeceras de grupo
  y tarjeta que la página Scripts (tarjeta con margen S3). Por debajo de `ROW_NARROW`
  Iniciar/Detener quedan solo con icono; por debajo de `ROW_QUANT` y `ROW_TPS` se
  omiten la cuantización, los tok/s y el icono de visión, que pasan al tooltip del
  nombre. El estado vacío no lleva el icono grande para que el botón quepa.
- Crear o editar un script desde la vista compacta pasa a la ampliada (el diálogo no
  cabe en 250 px) y, al guardar o cancelar, vuelve a la compacta.
- Diálogos: el modal limita su alto con el de la ventana y no con el tamaño del frame
  anterior. Antes, si la ventana crecía con el diálogo abierto, el cuerpo seguía con la
  altura vieja y el clic en «Guardar» caía en el fondo y cerraba el diálogo. El pie
  ocupa solo el alto de un botón (dejaba un hueco vacío encima).
- Icono nuevo a partir del glifo de la extensión `plane-llamacpp`: `assets/plane-llama.svg`
  (glifo blanco sobre cuadrado violeta, color de acento), PNG de 256 y 128 px, versión
  simbólica y `plane-llama-tray.png` (solo el glifo claro) para el indicador de la
  bandeja. `just install` instala también el SVG escalable y el simbólico.
- Sin verificar en GNOME real: el icono en el dock y en la bandeja, y el foco al cambiar
  de vista. La app abierta durante esta sesión era un binario anterior a estos cambios.

## Tamaño por vista, foco y chat (2026-10-24)

- Cada vista guarda su tamaño (`desktop::WindowSizes`, clave `plane-llama-window` de
  `ui.ron`) cuando el usuario la redimensiona; maximizada o a pantalla completa no cuenta.
  Por defecto, compacta de 900×600 y ampliada de 1600×1000, limitadas al último monitor
  visto menos `MONITOR_MARGIN`. Esto sustituye a los 300×250 de la ronda anterior; con
  900×600 la compacta muestra las filas completas de Scripts y el diálogo de script se
  abre en ella (se quitó el paso por la vista ampliada).
- Cambiar de vista vuelve a crear la ventana, que GNOME centra con el tamaño de esa
  vista (con descargas en curso se redimensiona, como antes). Para que quede al frente,
  `activate.rs` escucha clics y teclas en una cola propia de la conexión de winit, pide
  antes de cerrar la ventana vieja un token de xdg-activation con ese serial y su
  superficie, y activa con él la ventana nueva desde el hook de creación.
- Posición: en Wayland la app no puede leer ni fijar dónde está su ventana, así que no
  se guarda; cada ventana se abre centrada.
- «Abrir el chat de llama-server» no hacía nada: egui-winit estaba sin la feature
  `links` y `open_url` solo dejaba un aviso en el log. Activada.
- Sin verificar en GNOME real: el foco y el tamaño al cambiar de vista, y el chat.

## Scripts: visibilidad por fila, orden manual y compacta sin menú (2026-10-24)

- Se quitó «Elegir visibles» de la barra. Cada fila de la página Scripts tiene un ojo
  que la muestra u oculta (sale del menú «⋯»). El icono de multimodal pasa de ojo a
  imagen, en Scripts y en Modelos, para no confundirlos.
- Orden: solo «Recientes» (por defecto) y «Manual»; la elección se guarda en las
  preferencias de la biblioteca. Los órdenes viejos guardados (nombre, velocidad,
  tamaño) se leen como «Recientes» (`#[serde(other)]`).
- Orden manual: con «Manual» y sin búsqueda, cada fila tiene un asa para arrastrarla;
  al soltarla sobre otra fila queda antes de ella (`Scripts::move_before`) y el orden se
  guarda en `scripts.json`. Los favoritos siguen arriba y los grupos aparecen en el orden
  de su primera fila.
- Vista compacta: las filas no tienen menú «⋯», ojo ni asa; solo Iniciar/Detener y
  doble clic para editar.

## Tamaño por vista en Wayland (2026-10-24)

- El tamaño de cada vista no se guardaba nunca: en Wayland egui-winit deja
  `viewport().inner_rect` en `None` (winit no conoce la posición de la ventana).
  `track_window` ahora usa `viewport_rect()`, que siempre trae el tamaño.
- Al cambiar de vista, la ventana que se cierra ya tiene la vista nueva pero el
  tamaño de la anterior; mientras `reopening` está activo no se guarda nada.
- Test `window_tests::window_size_is_saved_per_view_without_inner_rect`.
- La posición sigue sin poder guardarse: Wayland no deja leerla ni fijarla, y la
  ventana abre centrada.

## Botones de ventana redondos (2026-10-24)

- `titlebar::caption` pinta un círculo de `CAPTION_DOT` (24 px), como en GNOME:
  `hover` en reposo, `border_strong` al pasar y `error` en «Cerrar». El área de clic
  pasa a `CAPTION` 32×30 y el icono a 12 px.
- También aplica a «Vista ampliada» y «Cerrar» de la vista compacta.

## Tamaño de ventana pisado por eframe (2026-10-24)

- Causa real de que cada vista perdiera su tamaño: eframe lee siempre la clave
  `"window"` de `ui.ron` (aunque `persist_window` esté apagado) y su tamaño gana al
  que pide la vista. Esa clave venía de una versión vieja y, como ya no se escribe,
  quedó fija en 1916×1164: cada ventana nueva abría así y se guardaba ese tamaño.
- `store::forget_eframe_window` la borra de `ui.ron` antes de crear la primera
  ventana (`main.rs`). Test `window_key_tests`. Verificado con `WAYLAND_DEBUG`: la
  geometría inicial ya es el tamaño guardado de la vista más la sombra.
- Logs `debug` de los tamaños (al abrir, al cambiar y al guardar):
  `RUST_LOG=plane_llama=debug,plane_llama_ui=debug`.

## Foco al cambiar de vista, maximizada por vista, borrado múltiple e historial → script (2026-10-24)

- Foco: el token de activación salía sin serial (`_TIME0`) y GNOME dejaba la ventana
  nueva detrás. La cola propia de `activate.rs` solo se procesaba al pedir el token,
  así que el seat recibía sus capacidades tarde y nunca había clic guardado. Ahora
  `track` hace una ida y vuelta al crear la ventana y `activate::poll()` procesa la
  cola en cada frame. Si aun así no hay serial, se avisa en el log.
- `WindowSizes` recuerda si cada vista quedó maximizada (`compact_maximized`,
  `extended_maximized`, `#[serde(default)]`). La ventana se crea con
  `with_maximized` y al restaurarla toma el tamaño guardado. Verificado en GNOME.
- Scripts: casilla por fila (etiqueta accesible «Seleccionar <nombre>») y, con algo
  marcado, la misma barra que el historial: «Marcar todos», «Eliminar seleccionados»
  (con confirmación) y «Quitar selección». La selección no se guarda.
- Historial: botón «Guardar como script» en la columna de acciones
  (`HISTORY_ACTIONS` = 76): abre el diálogo de nuevo script con el comando de esa
  corrida.
- Tests: `scripts_select_several_and_delete_them_together`,
  `history_row_opens_new_script_with_its_command` y el de `WindowSizes` ampliado.
  Captura `00-scripts-seleccion`.

## Vista compacta sin grupos (2026-10-24)

- La lista compacta ya no agrupa por organización: muestra todos los scripts
  visibles en una sola lista, favoritos primero y luego en el orden elegido
  (Recientes o Manual). `Scripts::sorted()` hace el filtrado y el orden, y
  `groups()` lo reutiliza para la vista ampliada, que sigue agrupada.
- Test `compact_list_has_no_groups`.

## Scripts sin grupos, orden manual libre y nombre con organización

- La vista ampliada también muestra los scripts en una sola lista, como la compacta. Se
  quitaron `Scripts::groups()`, la cabecera de grupo, el estado de grupos contraídos (y su
  preferencia guardada) y `ScriptMeta::group`.
- `Scripts::sorted()`: en «Recientes» los favoritos van primero y luego lo último usado;
  en «Manual» manda solo el orden de la lista, así que cualquier fila (favorita o no) se
  puede mover a cualquier posición.
- Arrastre: `move_before` pasó a `move_to(id, target, after)`. La línea de inserción se
  pinta arriba o abajo de la fila según la mitad donde esté el puntero, para poder
  soltar también al final.
- `models::script_title` antepone la organización de Hugging Face («unsloth/Qwen3.5 4B»);
  con `-m` sigue siendo solo el modelo. Afecta a scripts nuevos, sembrados del historial
  y al hint del diálogo; los nombres ya guardados no cambian.

## Tooltip del gráfico

- egui_plot pinta la etiqueta de la barra dentro del área del gráfico, recortada y con el
  texto a medias. Ahora el `BarChart` va con `allow_hover(false)` y `chart_modal` muestra
  los datos de la barra bajo el puntero en un tooltip de egui (`on_hover_ui_at_pointer`),
  que flota sobre el diálogo. Captura nueva: `04-grafico-tooltip`.

## Eliminar modelos descargados

- `models::delete_local_model(hub, repo, quant)`:
  - Borra los enlaces de esa cuantización en todos los snapshots y los blobs a los que
    apuntan, solo si están dentro del repositorio.
  - Quita las carpetas que quedan vacías.
  - Si era la última cuantización del repositorio, borra la carpeta `models--…` entera,
    con el mmproj.
  - Devuelve los bytes liberados.
- `Core::delete_local_model` lo corre fuera del runtime, con `spawn_blocking`.
- UI:
  - Papelera «Eliminar del disco» en cada fila de Modelos → confirmación con el tamaño →
    `Action::DeleteModel`.
  - `Message::ModelDeleted` refresca la lista y avisa del espacio liberado.
  - Si el servidor corre con ese modelo, pide detenerlo primero.
- Tests:
  - `scans_cache_layout` cubre el borrado parcial y el total.
  - `local_model_is_deleted_after_confirming` cubre la UI.

## Ventana al frente al cambiar de vista (de verdad) y página de la vista ampliada

- Causa real, vista con `WAYLAND_DEBUG`: la ventana nueva sí abría enfocada. Al volverla
  ventana normal (`xdg_dialog_v1.unset_modal`, el truco para que Mutter la centre),
  Mutter le quitaba el foco unos milisegundos después y quedaba detrás de las demás.
  Reactivarla con un token propio no sirve: sin clic el token sale `_TIME0` y Mutter lo
  ignora.
- Ahora `center::placed()` se llama la primera vez que la ventana pierde el foco
  (`App::had_focus`), no en el segundo frame. Mientras tanto sigue siendo diálogo;
  maximizar funciona igual (verificado en la traza).
- `activate`: el serial del token es el de la pulsación (botón o tecla), no el de
  soltar.
- `Store::extended_tab` recuerda la página de la vista ampliada:
  - Se guarda en `plane-llama-extended-tab`.
  - `set_simple` la guarda al pasar a compacta y la restaura al volver.
  - `Store::load` la usa al abrir en vista ampliada.
  - Test: `extended_view_remembers_its_page`.

## Buscador de Hugging Face sin limitarse a Unsloth

- El listado inicial (búsqueda vacía) pedía solo `author=unsloth`. Ahora `HubSearch` va
  sin autor: lista y busca GGUF de cualquier autor, en el orden elegido. El título pasó
  a «Modelos GGUF de Hugging Face».
- Se mantiene `filter=gguf` (llama-server solo carga GGUF). Escribir `autor/texto` sigue
  limitando a ese autor.

## Padding en las celdas del historial

- `history_table::padded` deja `Tokens::CELL_PAD` (8 px) a cada lado del contenido de
  las cabeceras y celdas de datos; el contenido ya no queda pegado al de la columna
  vecina.
- Los anchos iniciales de Modelo, Calificación, favorito y acciones suman ese margen,
  para que las etiquetas del modelo sigan cabiendo.

## Descargar el modelo que falta antes de arrancar un script

- Al arrancar o medir un script con `-hf org/nombre[:QUANT]` que no está en la caché
  (`Store::launch`), sale el modal «Modelo no descargado» con «Descargar» o «Cancelar»
  (`models_page::fetch_dialog`). Mientras pregunta no queda nada en curso.
- Al aceptar (`Store::accept_fetch`), la app baja el modelo con su propio descargador
  (`Action::Fetch`): pide los detalles al Hub y elige los mismos archivos que bajaría
  `llama-server` (`hub::plan_for`: la cuantización pedida, o `Q4_K_M`, o la primera; y
  el mmproj más pequeño salvo `--no-mmproj`). Al terminar, arranca el servidor o el
  benchmark.
- Barra de progreso (`models_page::fetch_progress`): encima de cualquier página de la
  vista ampliada y de la lista compacta, con bytes, velocidad, tiempo restante y
  cancelar. En Modelos aparece en «Descargando». «Detener» del benchmark cancela la
  descarga; si se cancela o falla, no se arranca nada.
- Scripts con `-m` o `--hf-file` no cambian (los resuelve llama-server).
- `models::script_hf` y `models::pick_local` (la búsqueda de `find_local` sobre una lista).
- Tests: `missing_model_asks_and_downloads_before_starting`,
  `missing_model_can_be_declined_or_cancelled`, `plans_like_llama_server`,
  `detects_hub_models_in_scripts`. Los tests de `ui_benchmark` usan una caché falsa
  (`HF_HUB_CACHE`) con el modelo del fake. Capturas `12-modelo-no-descargado` y
  `13-descargando-modelo`.

## Textos seleccionables

- `theme` activa `selectable_labels`: los textos (logs, resultados, respuesta, diálogos,
  avisos, detalles de modelos, optimizador…) se pueden seleccionar con el ratón y copiar
  con Ctrl+C.
- `widgets::no_select` lo desactiva donde el texto robaría el clic o el arrastre: filas
  clicables (`row_with_height`: scripts y modelos), barra lateral y barras de título
  (arrastran la ventana).
- La tabla del historial sí se puede seleccionar (las filas se marcan con su casilla,
  no con un clic). Las fichas (`widgets::chip`, `widgets::best`: tamaño, cuantización,
  mejor valor) pasaron de texto pintado a `Label` dentro de un `Frame`, para poder
  copiarlas también. Test: `history_cells_text_can_be_copied` (aplica el tema de la app).

## Minimizar en GNOME y optimizador en un diálogo

- Minimizar no hacía nada: la ventana nueva se marca como diálogo modal
  (`xdg_dialog_v1`, `center`) para que Mutter la centre y solo vuelve a ser normal al
  perder el foco. Mutter no minimiza (ni maximiza) ventanas que no son normales. Los
  botones de la barra y el doble clic llaman antes a `center::placed`.
- El optimizador ya no es una página: `optimizer_modal::show` es un diálogo
  (`widgets::dialog`, ancho `DIALOG_LG`) en `overlays`, encima de cualquier página.
  Se abre con «Optimizar» (botón primario) en la tarjeta del script; ya no está en la
  barra lateral, cuya entrada «Benchmark» pasa a llamarse «Editor» (igual que el título de la página). Cerrar con ✕ o Escape equivale a «Cancelar». «Aplicar al script» sigue
  llevando a Benchmark. `TAB_OPTIMIZER` ya no se guarda como última página.

## Optimizador más claro

- Cada parámetro explica para qué sirve y cómo afecta a la VRAM, con su flag a la
  derecha. Apagado solo se ve la explicación (queda el valor por defecto de
  llama-server); encendido, también su control. Flash attention, visión, dispositivos
  y reparto siguen el mismo formato.
- Los grupos tienen subtítulo y van por impacto: contexto y capas, caché KV, batch,
  memoria y especulativo, dispositivos.
- En ventana ancha, la cabecera del modelo queda arriba y las columnas se desplazan
  por separado: el consumo sigue a la vista al mover los parámetros.
- Veredicto bajo la barra: «Cabe: quedan X GB libres» o «No cabe: faltan X GB» con
  qué bajar. La calibración explica que la estimación es aproximada.
- «Default» pasa a «Restablecer». Las fichas del modelo omiten los datos que faltan.
- `widgets::chip` reserva solo el ancho de su texto: en filas con salto cada ficha
  bajaba a su propia línea (también bajo el script del Editor).

## Script al iniciar elegido entre los guardados, con espera, y caja del editor

- Ajustes → «Script al iniciar»: desplegable con «Ninguno» y los scripts guardados (ya no
  se copia el del editor). Debajo, vista previa del script y «Esperar antes de iniciar»
  (0–600 s).
- `AppSettings` suma `launchScriptId` y `launchDelaySecs`. `resolve_launch_script` lanza
  el texto actual del script guardado; si se borró, la copia en `launchScript` (también
  la de versiones anteriores, que se muestra como «Script sin guardar»). Lo usan el
  arranque (tras la espera) y «Iniciar servidor» de la bandeja.
- `script_editor::code_box`: caja del script con números de línea, binario atenuado,
  flags en el color de acento, `\` tenue y borde de acento con el foco. La usan el
  Editor y el diálogo de scripts.
- Tests: `desktop_settings_pick_saved_launch_script`,
  `launch_script_follows_saved_script_or_falls_back_to_copy`, `flags_are_not_negative_numbers`.

### Ajustes finos: ajustes, foco del editor y entrada de diálogos

- Ajustes → «Script al iniciar» ya no muestra la vista previa del script; basta el
  nombre en el desplegable.
- La caja del editor ya no se mueve al tomar el foco: el borde del Frame mantiene su
  grosor y el realce en acento se pinta encima, hacia dentro.
- Los diálogos (`widgets::dialog`) entran con un fundido y un leve desplazamiento hacia
  arriba (`DIALOG_FADE` = 0,18 s, `DIALOG_SLIDE` = 12 px, `cubic_out`); el fondo también
  aparece gradualmente. Se reinicia cada vez que el diálogo se abre.

### Script arrancado al cambiar de vista y carga del modelo

- Cambiar entre vista compacta y ampliada (o volver desde la bandeja) vuelve a crear la
  ventana y la `Store` se recarga del disco, donde no está `library.started`: el servidor
  seguía corriendo pero ninguna fila lo reconocía, así que todos los «Iniciar» quedaban
  bloqueados y no había «Detener». Ahora `Desktop::started` guarda ese script entre
  ventanas (se escribe en `on_exit` y se lee al crear la `App`).
- Mientras el servidor está en `starting`, el fondo de la fila del script se llena de
  izquierda a derecha (progreso simulado desde `startedAt`, `LOAD_PACE` = 6 s, tope 95 %)
  y al pasar a `running` queda marcada entera.

### Filas de Scripts compactas

- La fila ya no reparte las etiquetas en columnas de ancho fijo (que dejaban huecos):
  el nombre va seguido de sus etiquetas (cuantización, visión, parámetros, contexto,
  tok/s, tamaño) y a la derecha quedan Iniciar/Detener, el ojo y el menú.
- Las etiquetas que no caben se omiten de la menos importante a la más (tamaño,
  contexto, parámetros, visión, tok/s, cuantización) y pasan a la ayuda del nombre; el
  nombre conserva al menos `NAME_MIN` = 140 px antes de truncarse.
- La carga simulada del modelo se pinta en verde (`ok_soft`).
- Se quitaron los tokens de columnas fijas (`CHIP_SLOT`, `QUANT_SLOT`, `TPS_SLOT`,
  `ICON_SLOT`, `ROW_QUANT`, `ROW_TPS`, `ROW_WIDE`, `ROW_WIDER`).

### Crear o actualizar un script desde el editor

- El editor recuerda de qué script de la biblioteca viene (`Store::editor_source`, se
  persiste en `plane-llama-editor-source`). Se liga al usar «Abrir en el editor» en
  Scripts o al guardar el editor como script nuevo; se suelta al cargar el script por
  defecto o uno del historial.
- Ligado: botón «Actualizar script» (activo solo si hay cambios respecto al guardado) y
  el subtítulo muestra «nombre» · cambios sin guardar. Sin ligar: «Guardar como script»
  abre el diálogo con el texto del editor.
- Las acciones menos usadas pasan al menú «⋯»: guardar como script nuevo, formatear,
  guardar como script por defecto y volver al script por defecto.

### Editor destacado y página separada en script y benchmark

- Con un script de la biblioteca ligado, la tarjeta del editor lleva el borde en acento
  y una franja «Editando *nombre*» con su estado (cambios sin guardar / guardado),
  «Actualizar script» y una X para dejar de editarlo sin tocar el texto. En la cabecera
  queda «Guardar como nuevo» (crea otro script con el texto del editor). `widgets::card_marked` pinta el borde sin cambiar grosor.
- La página Editor queda en dos secciones: arriba el script a todo el ancho; debajo,
  con su título (`widgets::section_title`), «Benchmark»: indicadores, la prueba y el
  detalle del último resultado lado a lado, y la respuesta del modelo.
