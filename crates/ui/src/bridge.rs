// Ejecuta operaciones del core fuera del hilo gráfico y despierta la UI al recibir datos.
use egui::Context;
use plane_llama_core::{
    hub::{DownloadPlan, HubDetails, HubModel, HubSort},
    models::LocalModel,
    scripts::SavedScript,
    *,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub enum Action {
    Start(String),
    Stop,
    Benchmark(BenchmarkRequest),
    Cancel,
    ScriptDefault,
    SaveScript(String),
    PromptDefault,
    SavePrompt(String),
    Favorites(Vec<String>),
    LoadFavorites,
    ClearLogs,
    History,
    ClearHistory,
    Delete(Vec<String>),
    Patch(String, ResultPatch),
    Estimate(u64, EstimateRequest),
    Dryfit(String),
    Scripts,
    SaveScripts(Vec<SavedScript>),
    /// Guarda la biblioteca sin un script y la devuelve (tras confirmar el borrado).
    DeleteScript(Vec<SavedScript>),
    LocalModels,
    /// Borra de la caché `repo` y su cuantización (tras confirmar).
    DeleteModel(String, String),
    HubSearch(u64, String, HubSort),
    HubDetails(String),
    /// Clave `repo:quant` y archivos a bajar.
    Download(String, DownloadPlan),
    /// Baja el modelo `-hf` de un script (`org/nombre[:QUANT]`, con mmproj) antes de
    /// arrancarlo; la clave de la descarga es la misma referencia.
    Fetch(String, bool),
    CancelDownload(String),
}
#[derive(Debug)]
pub enum Message {
    Event(CoreEvent),
    Logs(LogsResponse),
    LogsCleared(f64),
    Gpu(GpuResponse),
    History(Vec<BenchmarkResult>),
    InitialScript(String),
    InitialPrompt(String),
    Script(String),
    Prompt(String),
    Favorites(Vec<String>),
    Result(Box<BenchmarkResult>),
    Estimate(u64, Box<EstimateResponse>),
    Dryfit(DryfitResponse),
    Notice(String),
    Failed(Action, String),
    Scripts(Vec<SavedScript>),
    LocalModels(Vec<LocalModel>),
    /// Modelo borrado (`repo:quant`), bytes liberados y la caché que quedó.
    ModelDeleted(String, u64, Vec<LocalModel>),
    HubResults(u64, Vec<HubModel>),
    HubDetails(Box<HubDetails>),
    DownloadProgress(String, u64, u64),
    /// Fin de una descarga: ruta del primer archivo, o error; `None` si se canceló.
    DownloadEnded(String, Option<std::result::Result<String, String>>),
}
fn send(tx: &Sender<Message>, ctx: &Context, visible: &AtomicBool, message: Message) {
    if tx.send(message).is_ok() && visible.load(Ordering::Relaxed) {
        ctx.request_repaint();
    }
}
async fn poll_visible<F, Fut>(visible: Arc<AtomicBool>, wake: Arc<tokio::sync::Notify>, mut poll: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        if !visible.load(Ordering::Acquire) {
            wake.notified().await;
            continue;
        }
        poll().await;
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(4)) => {},
            _ = wake.notified() => {},
        }
    }
}
pub struct Bridge {
    core: Core,
    runtime: Handle,
    tx: Sender<Message>,
    pub receiver: Receiver<Message>,
    ctx: Context,
    tasks: Vec<JoinHandle<()>>,
    visible: Arc<AtomicBool>,
    wake: Arc<tokio::sync::Notify>,
    hub: reqwest::Client,
    downloads: Arc<Mutex<HashMap<String, CancellationToken>>>,
}
impl Bridge {
    pub fn new(core: Core, runtime: Handle, ctx: Context) -> Self {
        Self::with_visibility(core, runtime, ctx, Arc::new(AtomicBool::new(true)))
    }
    pub fn with_visibility(
        core: Core,
        runtime: Handle,
        ctx: Context,
        visible: Arc<AtomicBool>,
    ) -> Self {
        let wake = Arc::new(tokio::sync::Notify::new());
        let (tx, receiver) = mpsc::channel();
        let mut events = core.subscribe();
        let c = core.clone();
        let t = tx.clone();
        let x = ctx.clone();
        let visibility = visible.clone();
        let events_task = runtime.spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        if visibility.load(Ordering::Acquire) {
                            send(&t, &x, &visibility, Message::Event(event));
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        if !visibility.load(Ordering::Acquire) {
                            continue;
                        }
                        send(&t, &x, &visibility, Message::Logs(c.logs(None)));
                        send(
                            &t,
                            &x,
                            &visibility,
                            Message::Event(CoreEvent::Status { status: c.status() }),
                        );
                    }
                    Err(_) => break,
                }
            }
        });
        let c = core.clone();
        let t = tx.clone();
        let x = ctx.clone();
        let visibility = visible.clone();
        let gpu_wake = wake.clone();
        let gpu_task = runtime.spawn(async move {
            poll_visible(visibility.clone(), gpu_wake, || async {
                send(&t, &x, &visibility, Message::Gpu(c.gpu().await));
            })
            .await;
        });
        let c = core.clone();
        let t = tx.clone();
        let x = ctx.clone();
        let visibility = visible.clone();
        let initial = runtime.spawn(async move {
            send(
                &t,
                &x,
                &visibility,
                Message::Event(CoreEvent::Status { status: c.status() }),
            );
            send(&t, &x, &visibility, Message::Logs(c.logs(None)));
            if let Ok(v) = c.history().await {
                send(&t, &x, &visibility, Message::History(v));
            }
            if let Ok(v) = c.script_default().await {
                send(&t, &x, &visibility, Message::InitialScript(v));
            }
            if let Ok(v) = c.prompt_default().await {
                send(&t, &x, &visibility, Message::InitialPrompt(v));
            }
            if let Ok(v) = c.flags_favorites().await {
                send(&t, &x, &visibility, Message::Favorites(v));
            }
            match c.scripts().await {
                Ok(v) => send(&t, &x, &visibility, Message::Scripts(v)),
                Err(e) => send(
                    &t,
                    &x,
                    &visibility,
                    Message::Failed(Action::Scripts, e.to_string()),
                ),
            }
            send(
                &t,
                &x,
                &visibility,
                Message::LocalModels(c.local_models().await),
            );
        });
        Self {
            core,
            runtime,
            tx,
            receiver,
            ctx,
            visible,
            wake,
            tasks: vec![events_task, gpu_task, initial],
            hub: hub::client(),
            downloads: Default::default(),
        }
    }
    pub fn refresh(&self) {
        self.wake.notify_one();
        let c = self.core.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        let visible = self.visible.clone();
        self.runtime.spawn(async move {
            send(&tx, &ctx, &visible, Message::Logs(c.logs(None)));
            send(
                &tx,
                &ctx,
                &visible,
                Message::Event(CoreEvent::Status { status: c.status() }),
            );
        });
    }
    /// Descarga en segundo plano; avisa el progreso como mucho cinco veces por segundo.
    /// Sin `plan`, primero pide al Hub los archivos de `key` (`org/nombre[:QUANT]`).
    fn download(&self, key: String, plan: Option<DownloadPlan>, mmproj: bool) {
        let Ok(mut running) = self.downloads.lock() else {
            return;
        };
        if running.contains_key(&key) {
            return;
        }
        let token = CancellationToken::new();
        running.insert(key.clone(), token.clone());
        drop(running);
        let (c, tx, ctx, visible, client, downloads) = (
            self.core.clone(),
            self.tx.clone(),
            self.ctx.clone(),
            self.visible.clone(),
            self.hub.clone(),
            self.downloads.clone(),
        );
        self.runtime.spawn(async move {
            let plan = match plan {
                Some(p) => Ok(p),
                None => plan_for(&client, &key, mmproj).await,
            };
            let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let result = match plan {
                Ok(plan) => {
                    hub::download(&client, &models::hub_dir(), &plan, &token, |done, total| {
                        if last.elapsed() >= std::time::Duration::from_millis(200) || done == total
                        {
                            last = std::time::Instant::now();
                            send(
                                &tx,
                                &ctx,
                                &visible,
                                Message::DownloadProgress(key.clone(), done, total),
                            );
                        }
                    })
                    .await
                }
                Err(e) => Err(e),
            };
            if let Ok(mut running) = downloads.lock() {
                running.remove(&key);
            }
            let ended = match result {
                Ok(path) => Some(Ok(path.to_string_lossy().into_owned())),
                Err(CoreError::Cancelled) => None,
                Err(e) => Some(Err(e.to_string())),
            };
            send(&tx, &ctx, &visible, Message::DownloadEnded(key, ended));
            send(
                &tx,
                &ctx,
                &visible,
                Message::LocalModels(c.local_models().await),
            );
        });
    }
    /// Cancela las descargas en curso (al salir de la app).
    pub fn cancel_downloads(&self) {
        if let Ok(running) = self.downloads.lock() {
            for token in running.values() {
                token.cancel();
            }
        }
    }
    pub fn dispatch(&self, action: Action) {
        match action {
            Action::Download(key, plan) => return self.download(key, Some(plan), false),
            Action::Fetch(hf, mmproj) => return self.download(hf, None, mmproj),
            Action::CancelDownload(key) => {
                if let Ok(running) = self.downloads.lock()
                    && let Some(token) = running.get(&key)
                {
                    token.cancel();
                }
                return;
            }
            _ => {}
        }
        let hub_client = self.hub.clone();
        let c = self.core.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        let visible = self.visible.clone();
        self.runtime.spawn(async move {
            let result: plane_llama_core::Result<Option<Message>> = async {
                Ok(Some(match &action {
                    Action::Start(s) => Message::Event(CoreEvent::Status {
                        status: c.start(s).await?,
                    }),
                    Action::Stop => Message::Event(CoreEvent::Status {
                        status: c.stop().await?,
                    }),
                    Action::Cancel => {
                        c.stop_benchmark();
                        Message::Notice("deteniendo…".into())
                    }
                    Action::Benchmark(r) => {
                        Message::Result(Box::new(c.benchmark(r.clone()).await?))
                    }
                    Action::ScriptDefault => Message::Script(c.script_default().await?),
                    Action::PromptDefault => Message::Prompt(c.prompt_default().await?),
                    Action::SaveScript(s) => {
                        c.save_script_default(s).await?;
                        Message::Notice("Default guardado".into())
                    }
                    Action::SavePrompt(s) => {
                        c.save_prompt_default(s).await?;
                        Message::Notice("Prompt default guardado".into())
                    }
                    Action::LoadFavorites => Message::Favorites(c.flags_favorites().await?),
                    Action::Favorites(v) => {
                        c.save_flags_favorites(v).await?;
                        Message::Favorites(v.clone())
                    }
                    Action::ClearLogs => {
                        c.clear_logs();
                        Message::LogsCleared(c.logs(None).cursor)
                    }
                    Action::History => Message::History(c.history().await?),
                    Action::ClearHistory => {
                        c.clear_history().await?;
                        Message::History(c.history().await?)
                    }
                    Action::Delete(ids) => {
                        c.delete_results(ids).await?;
                        Message::History(c.history().await?)
                    }
                    Action::Patch(id, p) => {
                        c.patch_result(id, p.clone()).await?;
                        Message::History(c.history().await?)
                    }
                    Action::Estimate(id, r) => {
                        Message::Estimate(*id, Box::new(c.estimate(r.clone()).await?))
                    }
                    Action::Dryfit(s) => {
                        Message::Dryfit(c.dryfit(DryfitRequest { script: s.clone() }).await?)
                    }
                    Action::Scripts => Message::Scripts(c.scripts().await?),
                    Action::SaveScripts(v) => {
                        c.save_scripts(v).await?;
                        return Ok(None);
                    }
                    Action::DeleteScript(v) => {
                        c.save_scripts(v).await?;
                        Message::Scripts(v.clone())
                    }
                    Action::LocalModels => Message::LocalModels(c.local_models().await),
                    Action::DeleteModel(repo, quant) => {
                        let freed = c.delete_local_model(repo.clone(), quant.clone()).await?;
                        Message::ModelDeleted(
                            format!("{repo}:{quant}"),
                            freed,
                            c.local_models().await,
                        )
                    }
                    Action::HubSearch(serial, query, sort) => Message::HubResults(
                        *serial,
                        hub::search(&hub_client, query, None, *sort).await?,
                    ),
                    Action::HubDetails(id) => {
                        Message::HubDetails(Box::new(hub::details(&hub_client, id).await?))
                    }
                    Action::Download(..) | Action::Fetch(..) | Action::CancelDownload(_) => {
                        return Ok(None);
                    }
                }))
            }
            .await;
            match result {
                Ok(Some(m)) => send(&tx, &ctx, &visible, m),
                Ok(None) => {}
                Err(e) => send(&tx, &ctx, &visible, Message::Failed(action, e.to_string())),
            }
        });
    }
}
/// Archivos que bajaría `llama-server -hf` para `hf` (`org/nombre[:QUANT]`).
async fn plan_for(client: &reqwest::Client, hf: &str, mmproj: bool) -> Result<DownloadPlan> {
    let (repo, tag) = match hf.split_once(':') {
        Some((repo, tag)) => (repo, Some(tag)),
        None => (hf, None),
    };
    let details = hub::details(client, repo).await?;
    hub::plan_for(&details, tag, mmproj).ok_or_else(|| {
        CoreError::Invalid(format!(
            "{repo} no tiene la cuantización {}.",
            tag.unwrap_or("Q4_K_M")
        ))
    })
}
impl Drop for Bridge {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn gpu_polling_sleeps_while_hidden_and_resumes() {
        let visible = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(tokio::sync::Notify::new());
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = calls.clone();
        let task = tokio::spawn(poll_visible(visible.clone(), wake.clone(), move || {
            count.fetch_add(1, Ordering::Relaxed);
            std::future::ready(())
        }));
        tokio::task::yield_now().await;
        tokio::time::advance(std::time::Duration::from_secs(16)).await;
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        visible.store(true, Ordering::Release);
        wake.notify_one();
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        visible.store(false, Ordering::Release);
        tokio::time::advance(std::time::Duration::from_secs(16)).await;
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        task.abort();
    }
    #[test]
    fn hidden_bridge_does_not_repaint_or_poll_gpu() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::new(Config {
            data_dir: dir.path().into(),
        })
        .unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let ctx = Context::default();
        let repaints = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = repaints.clone();
        ctx.set_request_repaint_callback(move |_| {
            count.fetch_add(1, Ordering::Relaxed);
        });
        let visible = Arc::new(AtomicBool::new(false));
        let bridge = Bridge::with_visibility(core.clone(), runtime.handle().clone(), ctx, visible);
        std::thread::sleep(std::time::Duration::from_millis(150));
        core.clear_logs();
        bridge.refresh();
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert_eq!(repaints.load(Ordering::Relaxed), 0);
        assert!(
            !bridge
                .receiver
                .try_iter()
                .any(|m| matches!(m, Message::Gpu(_)))
        );
        bridge.visible.store(true, Ordering::Release);
        bridge.refresh();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(repaints.load(Ordering::Relaxed) > 0);
    }
}
