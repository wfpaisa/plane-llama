// Lógica compartida de plane-llama, sin dependencias de interfaz.
pub mod config;
pub mod debug_log;
pub mod desktop;
pub mod devices;
pub mod gpu;
pub mod history;
pub mod hub;
pub mod logs;
pub mod mem;
pub mod metrics;
pub mod models;
pub mod script_parser;
pub mod scripts;
pub mod state;
pub mod types;

pub use config::Config;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Resultado no encontrado.")]
    NotFound,
    #[error("Benchmark cancelado por el usuario.")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
pub type Result<T> = std::result::Result<T, CoreError>;

pub mod benchmark;
pub mod client;
pub mod dryfit;
pub mod optimizer;
pub mod server_manager;
pub mod shutdown;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

struct Inner {
    cfg: Config,
    state: Arc<state::AppState>,
    history: Mutex<history::History>,
    operation: Mutex<Option<CancellationToken>>,
    server: tokio::sync::Mutex<Option<server_manager::ManagedServer>>,
}
#[derive(Clone)]
pub struct Core {
    inner: Arc<Inner>,
}
// La reserva se libera incluso si el llamador abandona el futuro.
struct Operation {
    core: Core,
    token: CancellationToken,
    armed: bool,
}
impl Drop for Operation {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.token.cancel();
        let core = self.core.clone();
        tokio::spawn(async move {
            core.stop_managed(std::time::Duration::from_secs(3)).await;
            *core.inner.operation.lock() = None;
        });
    }
}
impl Core {
    pub fn new(cfg: Config) -> Result<Self> {
        let history = history::History::new(cfg.clone())?;
        Ok(Self {
            inner: Arc::new(Inner {
                cfg,
                history: Mutex::new(history),
                state: Arc::new(state::AppState::default()),
                operation: Mutex::new(None),
                server: tokio::sync::Mutex::new(None),
            }),
        })
    }
    pub fn status(&self) -> StatusResponse {
        self.inner.state.status.read().clone()
    }
    pub fn logs(&self, since: Option<u64>) -> LogsResponse {
        self.inner.state.logs.read(since)
    }
    pub fn clear_logs(&self) {
        self.inner.state.logs.clear();
    }
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CoreEvent> {
        self.inner.state.logs.events.subscribe()
    }
    pub async fn start(&self, script: &str) -> Result<StatusResponse> {
        let mut server = self.inner.server.lock().await;
        if self.inner.operation.lock().is_some() {
            return Err(CoreError::Conflict("Ya hay un benchmark corriendo.".into()));
        }
        if server.as_ref().is_some_and(|s| !s.is_exited()) {
            return Err(CoreError::Conflict("Ya hay un servidor corriendo.".into()));
        }
        let parsed = script_parser::parse_script(script)?;
        *server = Some(server_manager::start_server(&parsed, self.inner.state.clone()).await?);
        Ok(self.status())
    }
    async fn stop_managed(&self, timeout: std::time::Duration) {
        let server = self.inner.server.lock().await.clone();
        if let Some(server) = server {
            server.stop(timeout).await;
            let mut slot = self.inner.server.lock().await;
            if slot
                .as_ref()
                .is_some_and(|current| current.pid == server.pid)
            {
                *slot = None;
                server_manager::set_status(
                    &self.inner.state,
                    StatusResponse {
                        status: "stopped".into(),
                        ..Default::default()
                    },
                );
            }
        }
    }
    pub async fn stop(&self) -> Result<StatusResponse> {
        self.stop_benchmark();
        self.stop_managed(std::time::Duration::from_secs(8)).await;
        server_manager::set_status(
            &self.inner.state,
            StatusResponse {
                status: "stopped".into(),
                ..Default::default()
            },
        );
        Ok(self.status())
    }
    async fn reserve(&self, dryfit: bool) -> Result<Operation> {
        let server = self.inner.server.lock().await;
        let mut operation = self.inner.operation.lock();
        if operation.is_some() {
            return Err(CoreError::Conflict(
                if dryfit {
                    "Ya hay un benchmark o calibración corriendo."
                } else {
                    "Ya hay un benchmark corriendo."
                }
                .into(),
            ));
        }
        if server.as_ref().is_some_and(|s| !s.is_exited()) {
            return Err(CoreError::Conflict(
                if dryfit {
                    "Detén el servidor manual antes de calibrar."
                } else {
                    "Detén el servidor manual antes de benchmark automático."
                }
                .into(),
            ));
        }
        let token = CancellationToken::new();
        *operation = Some(token.clone());
        Ok(Operation {
            core: self.clone(),
            token,
            armed: true,
        })
    }
    pub async fn benchmark(&self, req: BenchmarkRequest) -> Result<BenchmarkResult> {
        let mut operation = self.reserve(false).await?;
        let result = benchmark::run_benchmark(self, req, &operation.token).await;
        self.stop_managed(std::time::Duration::from_secs(8)).await;
        // Evita que la limpieza diferida alcance una operación posterior.
        *self.inner.operation.lock() = None;
        operation.armed = false;
        result
    }
    pub fn stop_benchmark(&self) {
        if let Some(token) = self.inner.operation.lock().as_ref() {
            token.cancel();
        }
    }
    pub async fn dryfit(&self, req: DryfitRequest) -> Result<DryfitResponse> {
        let mut operation = self.reserve(true).await?;
        let result = dryfit::run_dryfit(self, &req.script, &operation.token).await;
        self.stop_managed(std::time::Duration::from_secs(8)).await;
        *self.inner.operation.lock() = None;
        operation.armed = false;
        result
    }
    pub async fn estimate(&self, req: EstimateRequest) -> Result<EstimateResponse> {
        optimizer::estimate(req).await
    }
    pub async fn gpu(&self) -> GpuResponse {
        let (gpus, ram) = tokio::join!(gpu::read_gpu_stats(), mem::read_ram_stats());
        GpuResponse { gpus, ram }
    }
    async fn read_default(&self, file: &str, fallback: &str) -> Result<String> {
        match tokio::fs::read_to_string(self.inner.cfg.file(file)).await {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(fallback.into()),
            Err(e) => Err(e.into()),
        }
    }
    pub async fn script_default(&self) -> Result<String> {
        Ok(tokio::fs::read_to_string(self.inner.cfg.file("script-default.txt")).await?)
    }
    pub async fn save_script_default(&self, s: &str) -> Result<()> {
        tokio::fs::write(self.inner.cfg.file("script-default.txt"), s).await?;
        Ok(())
    }
    pub async fn prompt_default(&self) -> Result<String> {
        self.read_default("prompt-default.txt", metrics::DEFAULT_PROMPT)
            .await
    }
    pub async fn save_prompt_default(&self, s: &str) -> Result<()> {
        tokio::fs::write(self.inner.cfg.file("prompt-default.txt"), s).await?;
        Ok(())
    }
    pub async fn flags_favorites(&self) -> Result<Vec<String>> {
        Ok(
            serde_json::from_str(&self.read_default("flags-favorites.json", "[]").await?)
                .unwrap_or_default(),
        )
    }
    pub async fn save_flags_favorites(&self, v: &[String]) -> Result<()> {
        tokio::fs::write(
            self.inner.cfg.file("flags-favorites.json"),
            serde_json::to_vec_pretty(v)?,
        )
        .await?;
        Ok(())
    }
    /// Biblioteca de scripts; la primera vez se crea con el default y el historial.
    pub async fn scripts(&self) -> Result<Vec<scripts::SavedScript>> {
        if let Some(v) = scripts::load(&self.inner.cfg)? {
            return Ok(v);
        }
        let default = self.script_default().await.ok();
        let seeded = scripts::seed(&self.inner.history.lock().all(), default.as_deref());
        scripts::save(&self.inner.cfg, &seeded)?;
        Ok(seeded)
    }
    pub async fn save_scripts(&self, v: &[scripts::SavedScript]) -> Result<()> {
        scripts::save(&self.inner.cfg, v)
    }
    /// Modelos GGUF de la caché de Hugging Face. Recorre el disco fuera del runtime.
    pub async fn local_models(&self) -> Vec<models::LocalModel> {
        tokio::task::spawn_blocking(models::list_local_models)
            .await
            .unwrap_or_default()
    }
    /// Borra una cuantización de la caché. Devuelve los bytes liberados.
    pub async fn delete_local_model(&self, repo: String, quant: String) -> Result<u64> {
        let hub = models::hub_dir();
        Ok(
            tokio::task::spawn_blocking(move || models::delete_local_model(&hub, &repo, &quant))
                .await
                .map_err(|e| CoreError::Invalid(e.to_string()))??,
        )
    }
    pub async fn history(&self) -> Result<Vec<BenchmarkResult>> {
        Ok(self.inner.history.lock().all())
    }
    pub async fn clear_history(&self) -> Result<()> {
        self.inner.history.lock().clear()
    }
    pub async fn delete_results(&self, ids: &[String]) -> Result<()> {
        self.inner.history.lock().delete(ids)
    }
    pub async fn delete_result(&self, id: &str) -> Result<()> {
        self.delete_results(&[id.into()]).await
    }
    pub async fn patch_result(&self, id: &str, patch: ResultPatch) -> Result<BenchmarkResult> {
        self.inner.history.lock().patch(id, patch)
    }
    pub async fn shutdown(&self) {
        shutdown::shutdown(self).await;
    }
}
