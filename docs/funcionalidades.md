# Funcionalidades y know-how

Plane llama tiene dos objetivos:

1. **Ejecutar modelos de llama.cpp rápido**: guardar comandos de `llama-server` y
   arrancarlos con un clic, con los modelos ya descargados.
2. **Saber qué configuración corre mejor**: medir cada script (tokens/s, VRAM, RAM,
   tiempo de carga) y comparar los resultados.

Este documento resume qué hace la app y las decisiones que no se ven en el código.

## Vistas

| Vista | Para qué | Contenido |
|---|---|---|
| **Simple** (por defecto) | Arrancar un modelo con un clic | Ventana pequeña con los scripts visibles |
| **Ampliada** | Ajustar scripts, bajar modelos, medir y comparar | Scripts, Modelos, Benchmark, Historial, Optimizador, Logs |

- La vista simple solo tiene **Vista ampliada** y cerrar. Clic en un script lo
  inicia o lo detiene; el clic derecho abre editar, benchmark, chat, favorito y
  ocultar. El orden y los scripts visibles se eligen en la vista ampliada.
- La ventana se abre centrada en el monitor, también al cambiar de vista: cambiar
  cierra y vuelve a abrir la ventana con el tamaño de esa vista. En Wayland la app no
  puede elegir su posición; GNOME centra los diálogos, así que la ventana nueva se
  marca como diálogo (`xdg_dialog_v1`) hasta que está colocada. Con descargas en
  curso solo cambia el tamaño, para no cortarlas.
- En la vista ampliada, la barra lateral muestra la VRAM de cada tarjeta por nombre
  («RTX 5070 Ti») con su identificador (`nvidia0`) debajo.
- Las filas y barras se adaptan al ancho: en ventanas estrechas se ocultan columnas
  secundarias y los botones quedan como iconos.

## Scripts (biblioteca)

Un script es el comando completo de `llama-server` con sus flags y un nombre.
Viven en `DATA_DIR/scripts.json`.

- **Primera vez**: la biblioteca se crea con el script por defecto
  (`script-default.txt`) y cada script distinto del historial, del más rápido al
  más lento. Los nombres salen del modelo («Qwen3.6 27B»). Si se repiten, llevan
  un número.
- **Lista**: agrupada por organización del modelo (unsloth, bartowski…), con
  **Favoritos** arriba y luego **Archivos locales** (`-m`). Cada fila muestra:
  - la cuantización y si es multimodal (ojo);
  - los parámetros, el contexto y el mejor tok/s medido;
  - el tamaño en disco y el botón **Iniciar** o **Detener**.
  - El punto de color indica si el servidor está corriendo (verde), si el modelo
    está descargado (acento) o si `llama-server` lo bajará al arrancar (gris).
- **Ordenar**: recientes (última ejecución), más rápidos, nombre o tamaño.
- **Elegir qué mostrar**:
  - el filtro Visibles / Ocultos / Todos;
  - el menú **Elegir visibles** (un interruptor por script);
  - **Ocultar** en el menú de cada fila.
  - Ocultar no borra nada.
- **Acciones de una fila** (menú ⋯):
  - editar;
  - ejecutar benchmark (sin tocar el editor);
  - abrir en el editor de la vista ampliada;
  - duplicar, marcar como favorito, ocultar o eliminar (con confirmación; el
    historial se conserva).
- Con el servidor en marcha aparece el botón de chat, que abre la web de
  `llama-server`.
- **Guardar desde el editor**: en la vista ampliada, el icono de marcador del
  script guarda el contenido del editor en la biblioteca.
- El mejor tok/s se une por el **script formateado**: dos scripts con los mismos
  flags en otro orden de espacios o saltos de línea cuentan como el mismo.

## Modelos

### En el equipo

La app lee la caché de Hugging Face, la misma que usa `llama-server -hf`:

- Carpeta: `$HF_HUB_CACHE`, `$HF_HOME/hub` o `~/.cache/huggingface/hub`.
- Estructura: `models--org--nombre/snapshots/<commit>/*.gguf`, con enlaces a `blobs/`.
- Agrupa por cuantización. Los modelos divididos (`-00001-of-0000N`) suman sus partes.
- Los `mmproj-*.gguf` marcan el modelo como multimodal.
- Los `imatrix` se ignoran.
- Si hay varios snapshots, el de `refs/main` tiene prioridad.

Cada modelo puede:

- **Crear script**: abre el diálogo con un script nuevo, basado en el del editor y
  con `-hf org/nombre:QUANT`.
- **Usar en el script del editor**: cambia solo el modelo del script actual y
  quita `-m`, `--model`, `-hf` y `--hf-file` anteriores.
- **Copiar `-hf`**.

El mismo selector está en el editor del script (icono de cubo) y en el diálogo de
un script.

### Hugging Face

- **Búsqueda**:
  - Sin texto lista lo último de Unsloth.
  - Con texto busca repos con GGUF; `org/texto` busca dentro de una organización.
  - Orden: recientes, tendencia, descargas o likes.
  - Espera 450 ms tras dejar de escribir antes de consultar.
- **Detalle**: solo el modelo base, el tipo (texto, imagen y texto…) y los pesos.
  No se muestra la descripción.
- **Pesos**: una fila por cuantización, de la más pequeña a la más grande, con:
  - el tamaño total y el número de partes;
  - un punto de color comparado con la VRAM total: verde cabe, ámbar usa RAM, rojo no cabe.
- **Descarga**:
  - Va a la caché estándar (blob + enlace en el snapshot + `refs/main`), así que
    `llama-server -hf` la encuentra sin conexión.
  - Se descarga de una en una.
  - Con **Incluir mmproj** se baja también el proyector más pequeño.
  - Muestra progreso, velocidad y tiempo restante.
  - Cancelar deja un `.incomplete` y la próxima descarga sigue desde ahí (`Range`).
  - Al salir de la app se cancelan las descargas.
- **Repos con acceso restringido**:
  - Necesitan aceptar las condiciones en la web.
  - También necesitan un token: `HF_TOKEN` o el archivo `token` de `hf auth login`.
  - El token solo se envía a huggingface.co.

## Benchmark

Un benchmark de un script sigue estos pasos:

1. Mide la línea base de GPU, RAM y dispositivos (`--list-devices`).
2. Arranca `llama-server` en su propio grupo de procesos.
3. Espera a `/health` y envía el prompt a `/v1/chat/completions`.
4. Lee las métricas de los logs: prompt, generación, carga y draft.
5. Calcula los deltas de VRAM y RAM.
6. Guarda el resultado en `history.json` (máximo 200) y detiene el servidor.

Hay dos garantías:

- Si el proceso muere mientras carga, el benchmark falla en el acto. Antes esperaba
  el plazo completo. El plazo de `/health` es de 300 s, por los modelos grandes.
- Cancelar o cerrar la app mata todo el grupo de procesos (SIGTERM y luego SIGKILL),
  así no queda VRAM ocupada.

## Optimizador

Estima la VRAM sin arrancar el servidor:

```
VRAM = pesos + KV cache + overhead
```

- **Pesos**: el tamaño real del `.gguf`. En modelos divididos se suman todas las partes.
- **KV cache**: usa la arquitectura del header GGUF.
- **Calibrar**: arranca el servidor una vez para medir y corrige la estimación.
- **Resolución del modelo**: exige la cuantización exacta. Antes `Q4_K` podía tomar
  `Q4_K_M` o `Q4_K_XL`, y en los divididos solo contaba la primera parte.

## Hardware

- **NVIDIA**: `nvidia-smi` con índice, **nombre**, uso y memoria. La barra lateral
  muestra «RTX 5070 Ti» con su identificador `nvidia0` debajo.
- **AMD**: sysfs (`mem_info_vram_*`, `gpu_busy_percent` y `product_name` si existe).
- **Dispositivos del backend** (`CUDA0`, `Vulkan1`…): salen de `--list-devices`.
  Son los ids que acepta `--device`. El orden de CUDA puede no coincidir con el de
  `nvidia-smi`, así que el optimizador muestra el id y el nombre de cada uno.
- Con la ventana oculta no se consulta la GPU.

## Apariencia

- Tema oscuro, claro o del sistema.
- **Color de acento**: ocho colores predefinidos o uno propio, en **Ajustes**.
  - Cambia botones, selección, medidores y la primera serie de los gráficos.
  - En acentos claros (ámbar, cian) el texto encima pasa a oscuro.
  - Se guarda junto con el tema.
- **Marco**: esquinas redondeadas, borde claro y sombra pintados por la app sobre
  una ventana transparente. Al maximizar desaparecen sombra y esquinas.
- **Desenfocar el fondo** (Ajustes): el fondo pasa a translúcido y se pide a GNOME
  que desenfoque lo que hay detrás con `ext_background_effect_v1` (GNOME 51+). Si el
  escritorio no lo ofrece, la opción aparece desactivada.
- **Logo**: el de plane-llama-bench (`assets/plane-llama.svg`). Se usa en la app, la
  ventana, la bandeja y el acceso del menú.

## Segundo plano

- Bandeja del sistema: abrir, iniciar o detener el servidor y salir.
- Mantener en la bandeja al cerrar.
- Iniciar con la sesión, opcionalmente oculta.
- Un script que se ejecuta al abrir la app.
- Instancia única: una segunda ejecución muestra la ventana existente.

## Datos

Todo vive en `DATA_DIR` (por defecto `~/.config/plane-llama`).

| Archivo | Contenido |
|---|---|
| `history.json` | Resultados de benchmark (máx. 200) |
| `scripts.json` | Biblioteca de scripts |
| `script-default.txt`, `prompt-default.txt` | Valores por defecto del editor |
| `flags-favorites.json` | Flags destacados del catálogo |
| `settings.<app>.json` | Segundo plano y autostart |
| `ui.ron` | Estado de la interfaz: vista, tema, acento, orden, columnas, calibraciones |

El desenfoque se guarda en `settings.<app>.json`.

Los modelos no se copian: se usan desde la caché de Hugging Face.

## Know-how

- **El modelo se indica con `-hf org/nombre:QUANT`**. La cuantización debe
  coincidir con el nombre del archivo (`UD-Q4_K_XL`, `Q8_0`, `IQ4_XS`). La app
  la saca del nombre: quita la extensión y el sufijo de partes, y toma la etiqueta
  final.
- **Cabe en VRAM** se calcula sobre la VRAM total (90 %), no la libre. El
  escritorio reserva memoria que el modelo puede usar. El KV cache suma aparte;
  para afinarlo está el optimizador.
- **Velocidad**: si el modelo no cabe en una tarjeta, repartir con
  `--device CUDA0,CUDA1 --tensor-split a,b` suele ser mejor que descargar capas a
  CPU (`--n-gpu-layers` menor). En MoE, `--n-cpu-moe` mueve expertos a RAM
  perdiendo menos velocidad.
- **Comparar bien**: usa el mismo prompt y el mismo límite de tokens. La
  generación (tok/s) es la cifra que más pesa en el uso interactivo; el
  procesado del prompt importa con contextos largos.
- **Consumo de la app**:
  - Sin animaciones en bucle; repinta solo con entrada o eventos.
  - El reloj del benchmark y las descargas repintan a unos 5 fps solo mientras duran.
  - Medido en release, con la ventana visible y en reposo: 0 ticks de CPU en 10 s,
    unos 150 MB de RAM y 32 MiB de VRAM.
- **Pruebas**:
  - `just test` corre todo.
  - `ui_benchmark.rs` recorre la app contra el `fake-llama-server`: un benchmark
    completo, y arrancar y detener un script desde la vista simple.
  - `just screenshots` genera capturas con tu historial y tu caché reales.
  - `cargo test -p plane-llama-core live -- --ignored` prueba la caché real y
    huggingface.co.
