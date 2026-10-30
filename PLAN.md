# plane-llama — plan de trabajo

Port de **llama-bench** (`/home/projects/plane-llama-bench`, Bun + Angular + Electron)
a dos versiones nativas, para comparar cómo quedan y cómo rinden:

| Repo | Carpeta | UI | Lógica |
|---|---|---|---|
| `plane-llama` (egui) | `/home/projects/plane-llama-rust` | egui + eframe (wgpu), 100 % Rust | crate `plane-llama-core` |
| `plane-llama` (Tauri) | `/home/projects/plane-llama-tauri` | Tauri 2 + el frontend Angular existente | el **mismo** `plane-llama-core` (dependencia por path) |

La lógica del backend (TypeScript, `src/*.ts`) se porta **una sola vez** a Rust, en
`plane-llama-rust/crates/core`. Las dos apps la usan: la egui la llama directo, la
Tauri la expone como comandos. Así la comparación mide la UI y el shell, no dos
implementaciones distintas de la lógica.

El referente es siempre `/home/projects/plane-llama-bench` (solo lectura). Ante
cualquier duda de comportamiento, manda el código TS de ahí, no este plan.

---

## Decisiones fijas

- **Rust estable** (1.97), edición 2024. Workspace de Cargo.
- **Runtime async**: `tokio` (multi-thread). Cliente HTTP: `reqwest` con `rustls`, `json`.
- **Serialización**: `serde` + `serde_json`. Todos los structs públicos llevan
  `#[serde(rename_all = "camelCase")]` y los mismos nombres de campo que `src/types.ts`:
  el JSON tiene que ser **idéntico** al del backend TS. Requisito duro: el
  `history.json` que escribe llama-bench se tiene que poder leer sin conversión.
- **Errores**: `thiserror` en el core, `anyhow` solo en los binarios.
- **Logs internos**: `log` + `env_logger`.
- **Procesos**: `tokio::process::Command` con `process_group(0)` (grupo propio,
  equivalente al `detached: true`), kill del grupo con `nix::sys::signal::killpg`:
  SIGTERM → espera → SIGKILL. Mismo `cwd` y `LD_LIBRARY_PATH` que `binaryRuntimeEnv`.
- **DATA_DIR**: por defecto `~/.config/plane-llama` (crate `dirs`), override con la
  variable de entorno `DATA_DIR`. **No** usar `~/.config/llama-bench` (es el de
  Electron; no lo pisamos). Mismos nombres de archivo que el TS (`history.json`,
  `script-default.txt`, `prompt-default.txt`, `flags-favorites.json`, `vram-offset.txt`…).
- **Idioma**: UI y comentarios en **español**, igual que el referente. Identificadores en inglés.
- **Solo Linux** para métricas de hardware (igual que el original).
- **Nombre**: el producto se llama `plane-llama` en los dos repos (título de ventana,
  binario, `productName`, AppImage).

---

## Estructura del repo egui (`plane-llama-rust`)

```
Cargo.toml                  # workspace
crates/
  core/                     # plane-llama-core: lógica sin UI (la usan egui y Tauri)
    src/
      lib.rs                # re-exports + struct Core (fachada)
      config.rs             # ← src/config.ts
      types.rs              # ← src/types.ts (serde, camelCase)
      state.rs              # ← src/state.ts (Arc<AppState> con Mutex/RwLock, sin globals)
      logs.rs               # ← src/logs.ts (buffer circular + tokio::sync::broadcast)
      debug_log.rs          # ← src/debug-log.ts
      script_parser.rs      # ← src/script-parser.ts
      server_manager.rs     # ← src/server-manager.ts
      gpu.rs                # ← src/gpu.ts
      mem.rs                # ← src/mem.ts
      devices.rs            # ← src/devices.ts
      metrics.rs            # ← src/metrics.ts (regex)
      benchmark.rs          # ← src/benchmark.ts (CancellationToken en vez de AbortController)
      history.rs            # ← src/history.ts
      optimizer.rs          # ← src/optimizer.ts (parser GGUF, resolveModelFile, estimate)
      dryfit.rs             # ← src/dryfit.ts
      shutdown.rs           # ← src/shutdown.ts (sin signals: Core::shutdown() async)
      client/               # lógica que en el original vive en el FRONTEND (la necesita egui)
        mod.rs
        format.rs           # ← front/src/app/core/utils/format.ts
        flag_writer.rs      # ← front/src/app/core/utils/flag-writer.ts
        vram_estimate.rs    # ← front/src/app/core/utils/vram-estimate.ts
        flags_catalog.rs    # carga assets/llama-flags.json (include_str!)
        store_logic.rs      # derivados puros de bench.store.ts (orden, bests, filtros, modelOptions)
    tests/                  # tests de integración (usan tools/fake-llama-server)
  ui/                       # plane-llama-ui: componentes egui (fase 3)
apps/
  plane-llama/              # binario egui: main.rs (eframe::run_native)
tools/
  fake-llama-server/        # binario de prueba que imita a llama-server (fase 2)
assets/
  llama-flags.json          # catálogo de flags exportado desde front/.../llama-flags.ts
scripts/
  export-flags.ts           # bun: convierte llama-flags.ts → assets/llama-flags.json
  compare.sh                # fase 6: mide tamaño, arranque, RAM y CPU de las 3 versiones
```

### Fachada del core (contrato que usan las dos UIs)

Un solo `Core` clonable (`Arc` por dentro). Cada método equivale a un endpoint de
`src/router.ts`; los tipos de request/response son los mismos que el TS.

```rust
pub struct Core { /* Arc<Inner> */ }

impl Core {
    pub fn new(cfg: Config) -> Result<Self>;            // ensureDataDir + loadHistory
    // servidor manual
    pub fn status(&self) -> StatusResponse;              // GET  /status
    pub async fn start(&self, script: &str) -> Result<StatusResponse>;  // POST /start
    pub async fn stop(&self) -> Result<StatusResponse>;  // POST /stop
    // defaults
    pub async fn script_default(&self) -> Result<String>;           // GET/POST /script-default
    pub async fn save_script_default(&self, s: &str) -> Result<()>;
    pub async fn prompt_default(&self) -> Result<String>;           // GET/POST /prompt-default
    pub async fn save_prompt_default(&self, s: &str) -> Result<()>;
    pub async fn flags_favorites(&self) -> Result<Vec<String>>;     // GET/POST /flags-favorites
    pub async fn save_flags_favorites(&self, v: &[String]) -> Result<()>;
    // logs y eventos
    pub fn logs(&self, since: Option<u64>) -> LogsResponse;        // GET  /logs
    pub fn clear_logs(&self);                                      // POST /logs/clear
    pub fn subscribe(&self) -> broadcast::Receiver<CoreEvent>;     // GET  /events (SSE)
    // hardware
    pub async fn gpu(&self) -> GpuResponse;                        // GET  /gpu (+ RAM)
    // benchmark
    pub async fn benchmark(&self, req: BenchmarkRequest) -> Result<BenchmarkResult>; // POST /benchmark
    pub fn stop_benchmark(&self);                                  // POST /benchmark/stop
    pub async fn dryfit(&self, req: DryfitRequest) -> Result<DryfitResponse>;        // POST /dryfit
    pub async fn estimate(&self, req: EstimateRequest) -> Result<EstimateResponse>;  // POST /estimate
    // historial
    pub async fn history(&self) -> Result<Vec<BenchmarkResult>>;   // GET    /history
    pub async fn clear_history(&self) -> Result<()>;               // DELETE /history
    pub async fn delete_results(&self, ids: &[String]) -> Result<()>;   // POST /history/delete
    pub async fn delete_result(&self, id: &str) -> Result<()>;          // DELETE /history/:id
    pub async fn patch_result(&self, id: &str, p: ResultPatch) -> Result<BenchmarkResult>; // PATCH /history/:id
    // cierre
    pub async fn shutdown(&self);                                  // shutdownCleanup()
}

pub enum CoreEvent { Log(LogEntry), Status(StatusResponse) } // serde: { "type": "log" | "status", ... }
```

Los errores de "conflicto" (benchmark en curso, servidor manual corriendo → 409 en
el TS) son una variante propia `CoreError::Conflict(String)` para que la UI muestre
el mismo mensaje.

---

## Fases

Cada fase termina con: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` en verde, y **un commit** con mensaje
`feat(fase-N): <resumen>`. No pasar a la fase siguiente con algo en rojo.

### Fase 1 — Workspace y core puro (sin procesos)

1. Workspace de Cargo, `rust-toolchain.toml` (stable), `rustfmt.toml`, `.gitignore`
   (`target/`, `data/`), `README.md` corto.
2. `types.rs`: todas las interfaces de `src/types.ts` (+ las de requests que use
   `router.ts`). Campos opcionales del TS → `Option<T>` con
   `#[serde(skip_serializing_if = "Option::is_none")]` **solo** si el TS los omite;
   si el TS escribe `null`, serializar `null`.
3. `config.rs`, `state.rs`, `logs.rs`, `debug_log.rs`.
4. `script_parser.rs`: `tokenize_script`, `parse_script`, `flag_value`, `has_flag`,
   `to_num_or_null`. Comillas simples/dobles, continuaciones `\`, comentarios.
5. `metrics.rs`: las regex de `parseMetricsFromLogs` (recorrido desde el final,
   draft-mtp, fallback de load time por timestamps), `DEFAULT_PROMPT`.
6. `gpu.rs` (nvidia-smi CSV + sysfs AMD, `subtract_gpu_baseline`), `mem.rs`
   (`/proc/meminfo`), `devices.rs` (`parse_list_devices`, `detect_backend`,
   `vendor_from_name`, `compute_device_vram`). Separar **parseo** (puro, testeable con
   strings) de **lectura** (proceso/archivo).
7. `history.rs`: carga, guardado atómico (escribir a `.tmp` + rename), cap 200,
   borrar, borrar varios, limpiar, patch (favorito, rating y lo que permita el TS).
8. **Tests unitarios** de todo lo parseable. Fixtures de texto en
   `crates/core/tests/fixtures/` copiando salidas reales: líneas de log de
   llama-server (sacarlas de `/home/projects/plane-llama-bench/data/log_debug.txt`),
   salida de `--list-devices`, CSV de nvidia-smi, `/proc/meminfo`.
9. Test de compatibilidad: un `history.json` sintético de 3 entradas con todos los
   campos, y además un test `#[ignore]` que lee
   `/home/projects/plane-llama-bench/data/history.json` (solo lectura), lo
   deserializa y lo vuelve a serializar sin perder campos (comparar como
   `serde_json::Value`).

### Fase 2 — Procesos, benchmark, optimizador y fake server

1. `tools/fake-llama-server`: binario (tokio + axum) que imita a llama-server lo
   suficiente para probar todo sin GPU ni modelo:
   - `--list-devices` → imprime un listado con el formato real y sale.
   - `--port`, `--host` y el resto de flags: los acepta y los ignora.
   - Al arrancar imprime líneas de carga realistas (`loading model`, `model loaded`,
     `server is listening`), espera `FAKE_LOAD_MS` (default 300).
   - `GET /health` → 200. `POST /v1/chat/completions` → respuesta OpenAI con
     `content`, y escribe por stdout las líneas `prompt eval time` / `eval time` /
     `draft acceptance` con el formato que matchean las regex.
   - `FAKE_CRASH=1` → sale con código 1 durante la carga (prueba el fallo rápido).
   - Atiende SIGTERM y sale limpio.
2. `server_manager.rs`: `start_server` (devuelve el managed con un futuro `ready` que
   se resuelve al ver la línea de listo o falla si el proceso muere, con el exit code),
   `stop_server` (SIGTERM al grupo → timeout → SIGKILL), `url_for`, búsqueda del
   binario en el `PATH` (`which`) y `assert_binary_exists`. stdout/stderr → `logs.rs`.
3. `benchmark.rs`: el ciclo completo de `runBenchmark` con los mismos checkpoints
   de cancelación (`tokio_util::sync::CancellationToken`), baseline en paralelo
   (`tokio::join!`), `await ready`, health-check (500 ms / 120 s), request de chat,
   `sleep(400)`, parseo, deltas, `finally` que siempre detiene el server, guardado.
4. `optimizer.rs`: `read_gguf_arch` (parser binario del header, primeros 2 MB,
   tolerante a cortes), `parse_model_meta`, `resolve_model_file` (HF cache con
   `HF_HOME`, `--model`, mmproj), `build_estimate_response`, `recommend_params`.
   Test con un `.gguf` sintético generado en el test (header mínimo válido).
5. `dryfit.rs` ← `src/dryfit.ts`.
6. `shutdown.rs` + `Core::shutdown()`: aborta el benchmark y para el server (timeout 3 s).
7. `lib.rs`: la fachada `Core` completa (ver contrato arriba).
8. `client/`: port de `format.ts`, `flag-writer.ts`, `vram-estimate.ts` y la lógica
   pura de `bench.store.ts`. `scripts/export-flags.ts` + `assets/llama-flags.json`
   generado y commiteado; `flags_catalog.rs` lo carga.
9. Tests de integración (`crates/core/tests/`) contra `fake-llama-server`:
   start → ready → stop; benchmark completo que guarda un resultado con métricas
   no nulas; crash durante la carga devuelve error en < 2 s; cancelación a mitad;
   conflicto 409 si ya hay un benchmark; no quedan procesos huérfanos.

### Fase 3 — App egui (`crates/ui` + `apps/plane-llama`)

Mismas secciones y textos (en español) que `front/src/app/features/*`. Mirar cada
`.ts`/`.html` del referente para copiar textos, columnas y comportamiento.

1. `apps/plane-llama/src/main.rs`: `eframe::run_native` con `wgpu`, título
   `plane-llama`, 1440×900, mínimo 760×480, persistencia (`persistence`) en
   `DATA_DIR/ui.ron`, tema oscuro. Un runtime tokio en un hilo aparte;
   `Core::new` al arrancar y `Core::shutdown` al cerrar (`on_exit`).
2. Puente UI ↔ core (`crates/ui/src/bridge.rs`): la UI nunca bloquea. Las acciones
   se mandan al runtime con `tokio::spawn`; los resultados vuelven por un canal
   (`std::sync::mpsc` o `crossbeam`) que la UI vacía en cada frame, y la tarea llama
   `ctx.request_repaint()`. Un task suscrito a `Core::subscribe()` empuja logs y
   status; otro hace polling de `gpu()` cada 4 s. **Nada** de repintado continuo:
   en reposo la app debe estar a ~0 % de CPU.
3. `store.rs`: el equivalente de `bench.store.ts` (estado + acciones + derivados),
   usando `plane_llama_core::client::*`. Persistir las 6 claves de `storage.service.ts`
   con `eframe::Storage`.
4. Componentes (un módulo por sección):
   - `status_bar` — estado del server, botones arrancar/parar.
   - `script_editor` — `TextEdit` multilínea con fuente mono, guardar/cargar default,
     botón **Optimizar ⚡** (abre el optimizador), catálogo de flags con favoritos.
   - `benchmark_panel` — prompt, max tokens (checkbox + valor), ejecutar/cancelar.
   - `gpu_grid` — tarjetas por GPU/device + RAM.
   - `logs_viewer` — lista virtualizada (`ScrollArea::show_rows`), autoscroll, limpiar.
   - `response_card`, `last_result`.
   - `history_table` — `egui_extras::TableBuilder`, orden por columna, filtro por
     modelo, columnas visibles configurables, selección múltiple, favoritos/rating, borrar.
   - `compare_modal` — comparación lado a lado de los seleccionados.
   - `chart_modal` — `egui_plot`, mismas series que `chart-modal.ts`.
   - `optimizer_modal` — sliders, barras de VRAM en vivo (cálculo client-side con
     `client::vram_estimate`), Default, Aplicar al script / Cancelar.
   - Toasts y diálogos de confirmación propios (ventanas modales simples).
5. Tests: `egui_kittest` para status_bar, benchmark_panel e history_table (clicks y
   estado), más un test que arranca la app contra `fake-llama-server` y corre un
   benchmark de punta a punta.
6. `cargo build --release` y comprobar que el binario arranca contra el fake
   (`PATH=target/release:$PATH` con el fake renombrado/enlazado como `llama-server`).

### Fase 4 — App Tauri (repo `plane-llama-tauri`) — ver su `PLAN.md`

Puede correr **en paralelo** con la fase 3: solo depende de que el core de la fase 2
esté terminado y commiteado. No modifica `plane-llama-rust`.

### Fase 5 — Empaquetado

- egui: AppImage con `cargo-appimage` o `appimagetool` (`scripts/appimage.sh`),
  `.desktop` + icono.
- Tauri: `tauri build` → AppImage + `.deb`.

### Fase 6 — Comparación

`scripts/compare.sh` mide, para Electron (referente), egui y Tauri:

- tamaño del binario y del AppImage;
- tiempo hasta la primera ventana (`time` + espera a la ventana con `xdotool`/`swaymsg` o log de primer frame);
- RSS total del árbol de procesos en reposo a los 10 s y 60 s (`ps --ppid`, suma);
- CPU media en reposo 30 s y durante un benchmark contra `fake-llama-server` con
  logs a 50 líneas/s.

Resultado en `docs/comparison.md` (tabla + notas).
