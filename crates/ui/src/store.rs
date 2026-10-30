// Estado de la interfaz, acciones y persistencia compatible con las claves del referente.
use crate::{
    bridge::{Action, Message},
    library::{self, Models, Scripts},
    optimizer_modal::Optimizer,
};
use plane_llama_core::{
    client::{format, store_logic},
    models, *,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Instant,
};

pub const TAB_BENCH: usize = 0;
pub const TAB_HISTORY: usize = 2;
pub const TAB_LOGS: usize = 4;
pub const TAB_SCRIPTS: usize = 5;
pub const TAB_MODELS: usize = 6;
const SIMPLE_KEY: &str = "plane-llama-simple";
/// Última página abierta en la vista ampliada.
const EXTENDED_TAB_KEY: &str = "plane-llama-extended-tab";
const EDITOR_SOURCE_KEY: &str = "plane-llama-editor-source";
const TABS: [usize; 5] = [TAB_BENCH, TAB_HISTORY, TAB_LOGS, TAB_SCRIPTS, TAB_MODELS];
/// Vista guardada en `ui.ron`, para crear la primera ventana con su tamaño; antes de
/// que eframe abra el almacenamiento. Por defecto, la simple.
pub fn saved_simple(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| ron::from_str::<std::collections::HashMap<String, String>>(&s).ok())
        .and_then(|m| m.get(SIMPLE_KEY).map(|v| v != "false"))
        .unwrap_or(true)
}
const WINDOW_KEY: &str = "plane-llama-window";
/// Tamaños de ventana guardados en `ui.ron`, también antes de abrir la primera ventana.
pub fn saved_window(path: &std::path::Path) -> crate::desktop::WindowSizes {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| ron::from_str::<std::collections::HashMap<String, String>>(&s).ok())
        .and_then(|m| m.get(WINDOW_KEY).and_then(|v| serde_json::from_str(v).ok()))
        .unwrap_or_default()
}
/// Clave donde eframe guarda la ventana con `persist_window`. La lee aunque esté apagado
/// y su tamaño gana al que pide la vista, así que un valor viejo abría siempre la ventana
/// con ese tamaño. Se borra antes de crear la primera ventana.
const EFRAME_WINDOW_KEY: &str = "window";
pub fn forget_eframe_window(path: &std::path::Path) {
    let Some(mut map) = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| ron::from_str::<std::collections::HashMap<String, String>>(&s).ok())
    else {
        return;
    };
    if map.remove(EFRAME_WINDOW_KEY).is_none() {
        return;
    }
    match ron::ser::to_string_pretty(&map, Default::default()) {
        Ok(s) => {
            if let Err(e) = std::fs::write(path, s) {
                log::warn!("no se pudo reescribir {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("no se pudo serializar {}: {e}", path.display()),
    }
}
pub fn save_window(storage: &mut dyn eframe::Storage, sizes: &crate::desktop::WindowSizes) {
    if let Ok(v) = serde_json::to_string(sizes) {
        storage.set_string(WINDOW_KEY, v);
    }
}
const LIBRARY_KEY: &str = "plane-llama-library";
/// Anchos de las columnas del historial redimensionadas a mano, por clave de columna.
const COLUMN_WIDTHS_KEY: &str = "plane-llama-history-column-widths";
pub const KEYS: [&str; 7] = [
    "plane-llama-bench-script",
    "plane-llama-bench-prompt",
    "plane-llama-bench-sort",
    "plane-llama-bench-max-tokens",
    "plane-llama-bench-max-tokens-enabled",
    "plane-llama-bench-history-columns",
    "plane-llama-bench-calibration",
];
pub const DEFAULT_COLUMNS: &[&str] = &[
    "fav",
    "model",
    "backend",
    "ctx",
    "batch",
    "cache",
    "rating",
    "genTps",
    "promptTps",
    "vram",
    "totalVram",
    "ram",
    "actions",
];
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Calibration {
    pub measured: DryfitResponse,
    pub heuristic: VramBreakdown,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sort {
    pub col: String,
    pub dir: String,
}
pub struct Confirmation {
    pub text: String,
    pub action: Action,
}
#[derive(Clone, Copy)]
pub enum NoticeKind {
    Success,
    Warning,
    Error,
}
pub struct Store {
    pub script: String,
    pub prompt: String,
    pub max_tokens: f64,
    pub max_tokens_enabled: bool,
    pub sort: Sort,
    pub columns: Vec<String>,
    pub column_widths: BTreeMap<String, f32>,
    pub calibrations: BTreeMap<String, Calibration>,
    pub status: StatusResponse,
    pub logs: VecDeque<LogEntry>,
    pub cursor: f64,
    pub gpus: Vec<GpuInfo>,
    pub ram: RamInfo,
    pub history: Vec<BenchmarkResult>,
    pub favorites: Vec<String>,
    pub selected: BTreeSet<String>,
    pub models: Vec<String>,
    pub bench_started: Option<Instant>,
    pub bench_state: String,
    pub last_result: Option<BenchmarkResult>,
    pub autoscroll: bool,
    pub timestamps: bool,
    pub show_response: bool,
    pub compare: bool,
    pub chart: bool,
    pub chart_order: Vec<String>,
    pub chart_metric: usize,
    pub optimizer: Option<Optimizer>,
    pub confirmation: Option<Confirmation>,
    pub notices: VecDeque<(Instant, String, NoticeKind)>,
    pub actions: Vec<Action>,
    pub flag_search: String,
    pub favorites_only: bool,
    pub flags_open: bool,
    pub flag_category: usize,
    pub log_filter: String,
    pub history_filter: String,
    pub page: usize,
    pub page_size: usize,
    pub manual_pending: bool,
    pub dryfit_running: bool,
    pub optimizer_serial: u64,
    pub tab: usize,
    /// Página a la que vuelve la vista ampliada (la compacta solo muestra Scripts).
    pub extended_tab: usize,
    pub prompt_seeded: bool,
    pub script_seeded: bool,
    /// Script de la biblioteca abierto en el editor; «Actualizar» lo sobrescribe.
    pub editor_source: Option<String>,
    /// Vista simple (ventana compacta con los scripts) o ampliada (todas las páginas).
    pub simple: bool,
    /// Muestra el último resultado encima de la lista de scripts.
    pub result_banner: bool,
    pub library: Scripts,
    pub hub: Models,
    /// Descarga del modelo de un script (clave en `hub.downloads`) y lo que se lanza al
    /// terminarla: arrancar el servidor o el benchmark.
    pub fetching: Option<(String, Action)>,
    /// Modelo `-hf` que falta (con mmproj o no) y lo que se lanzará si se acepta bajarlo.
    pub ask_fetch: Option<(String, bool, Action)>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            script: String::new(),
            prompt: metrics::DEFAULT_PROMPT.into(),
            max_tokens: 2048.,
            max_tokens_enabled: true,
            sort: Sort {
                col: "timestamp".into(),
                dir: "desc".into(),
            },
            columns: DEFAULT_COLUMNS.iter().map(|s| (*s).into()).collect(),
            column_widths: BTreeMap::new(),
            calibrations: BTreeMap::new(),
            status: StatusResponse {
                status: "stopped".into(),
                ..Default::default()
            },
            logs: VecDeque::new(),
            cursor: 0.,
            gpus: vec![],
            ram: RamInfo::default(),
            history: vec![],
            favorites: vec![],
            selected: BTreeSet::new(),
            models: vec![],
            bench_started: None,
            bench_state: String::new(),
            last_result: None,
            autoscroll: true,
            timestamps: false,
            show_response: false,
            compare: false,
            chart: false,
            chart_order: vec![],
            chart_metric: 0,
            optimizer: None,
            confirmation: None,
            notices: VecDeque::new(),
            actions: vec![],
            flag_search: String::new(),
            favorites_only: false,
            flags_open: false,
            flag_category: 0,
            log_filter: String::new(),
            history_filter: String::new(),
            page: 0,
            page_size: 50,
            manual_pending: false,
            dryfit_running: false,
            optimizer_serial: 0,
            tab: 0,
            extended_tab: TAB_BENCH,
            prompt_seeded: false,
            script_seeded: false,
            editor_source: None,
            simple: true,
            result_banner: false,
            library: Scripts::default(),
            hub: Models::default(),
            fetching: None,
            ask_fetch: None,
        }
    }
}
impl Store {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let mut s = Self::default();
        if let Some(storage) = storage {
            if let Some(v) = storage.get_string(KEYS[0]).filter(|s| !s.is_empty()) {
                s.script = v;
                s.script_seeded = true;
            }
            if let Some(v) = storage.get_string(KEYS[1]).filter(|s| !s.is_empty()) {
                s.prompt = v;
                s.prompt_seeded = true;
            }
            let read = |key| {
                storage
                    .get_string(key)
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            };
            if let Some(v) = read(KEYS[2])
                .and_then(|v| serde_json::from_value::<Sort>(v).ok())
                .filter(|v| matches!(v.dir.as_str(), "asc" | "desc" | ""))
            {
                s.sort = v;
                s.sort.col = match s.sort.col.as_str() {
                    "date" => "timestamp",
                    "ctx" => "config.ctxSize",
                    "promptTps" => "promptTokensPerSecond",
                    "genTps" => "generationTokensPerSecond",
                    "draftAcc" => "draftAcceptance",
                    "loadTime" => "loadTimeSeconds",
                    "generationTime" => "generationTimeMs",
                    "totalVram" => "totalVramMiB",
                    "ramUsed" => "ramUsedMiB",
                    other => other,
                }
                .into();
            }
            if let Some(n) = read(KEYS[3])
                .and_then(|v| v.as_f64())
                .filter(|n| n.is_finite() && *n > 0.)
            {
                s.max_tokens = n;
            }
            if let Some(v) = read(KEYS[4]).and_then(|v| v.as_bool()) {
                s.max_tokens_enabled = v;
            }
            if let Some(v) = read(KEYS[5]).and_then(|v| serde_json::from_value(v).ok()) {
                s.columns = v;
            }
            if let Some(v) = read(KEYS[6]).and_then(|v| serde_json::from_value(v).ok()) {
                s.calibrations = v;
            }
            if let Some(v) = read(COLUMN_WIDTHS_KEY)
                .and_then(|v| serde_json::from_value::<BTreeMap<String, f32>>(v).ok())
            {
                s.column_widths = v
                    .into_iter()
                    .filter(|(_, w)| w.is_finite() && *w > 0.)
                    .collect();
            }
            s.editor_source = storage
                .get_string(EDITOR_SOURCE_KEY)
                .filter(|v| !v.is_empty());
            if let Some(v) = read(SIMPLE_KEY).and_then(|v| v.as_bool()) {
                s.simple = v;
            }
            if let Some(tab) = read(EXTENDED_TAB_KEY)
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .filter(|n| TABS.contains(n))
            {
                s.extended_tab = tab;
            }
            if let Some(p) =
                read(LIBRARY_KEY).and_then(|v| serde_json::from_value::<library::Prefs>(v).ok())
            {
                s.library.sort = p.sort;
                s.library.filter = p.filter;
                s.hub.hub_sort = p.hub_sort.unwrap_or_default();
                s.hub.with_mmproj = p.with_mmproj.unwrap_or(true);
            }
        }
        s.tab = if s.simple {
            TAB_SCRIPTS
        } else {
            s.extended_tab
        };
        s
    }
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        let values = [
            self.script.clone(),
            self.prompt.clone(),
            serde_json::to_string(&self.sort).unwrap_or_default(),
            self.max_tokens.to_string(),
            self.max_tokens_enabled.to_string(),
            serde_json::to_string(&self.columns).unwrap_or_default(),
            serde_json::to_string(&self.calibrations).unwrap_or_default(),
        ];
        for (key, value) in KEYS.iter().zip(values) {
            if *key == KEYS[2] && self.sort.col.is_empty() {
                storage.remove_string(key);
            } else {
                storage.set_string(key, value);
            }
        }
        storage.set_string(
            COLUMN_WIDTHS_KEY,
            serde_json::to_string(&self.column_widths).unwrap_or_default(),
        );
        storage.set_string(
            EDITOR_SOURCE_KEY,
            self.editor_source.clone().unwrap_or_default(),
        );
        storage.set_string(SIMPLE_KEY, self.simple.to_string());
        let extended_tab = if self.simple {
            self.extended_tab
        } else {
            self.tab
        };
        storage.set_string(EXTENDED_TAB_KEY, extended_tab.to_string());
        let prefs = library::Prefs {
            sort: self.library.sort,
            filter: self.library.filter,
            hub_sort: Some(self.hub.hub_sort),
            with_mmproj: Some(self.hub.with_mmproj),
        };
        storage.set_string(
            LIBRARY_KEY,
            serde_json::to_string(&prefs).unwrap_or_default(),
        );
    }
    /// Recalcula los datos de cada script (mejor tok/s, tamaño, descargado).
    pub fn refresh_library(&mut self) {
        self.library.refresh_meta(&self.history, &self.hub.local);
    }
    /// Guarda la biblioteca en disco y recalcula sus datos.
    pub fn save_library(&mut self) {
        self.refresh_library();
        self.actions
            .push(Action::SaveScripts(self.library.list.clone()));
    }
    /// Arranca el servidor con un script de la biblioteca.
    pub fn start_script(&mut self, id: &str) {
        if self.running() || self.busy() || self.manual_pending {
            return;
        }
        let Some(script) = self.library.get(id).map(|s| s.script.clone()) else {
            return;
        };
        self.library.touch(id);
        self.save_library();
        self.library.started = Some(script.clone());
        self.manual_pending = true;
        self.launch(Action::Start(script));
    }
    /// Lanza `Start` o `Benchmark`; si el modelo `-hf` del script no está en la caché,
    /// pregunta antes si descargarlo (con progreso) para lanzarlo al terminar.
    pub fn launch(&mut self, action: Action) {
        let script = match &action {
            Action::Start(s) => s,
            Action::Benchmark(r) => &r.script,
            _ => return self.actions.push(action),
        };
        let missing = models::script_hf(script).filter(|(hf, _)| {
            self.hub.local_loaded && models::pick_local(&self.hub.local, hf).is_none()
        });
        let Some((hf, mmproj)) = missing else {
            return self.actions.push(action);
        };
        if self.hub.downloads.contains_key(&hf) {
            self.notice_with_kind(
                format!("{hf} ya se está descargando; vuelve a intentarlo al terminar."),
                NoticeKind::Warning,
            );
            return self.abort_launch(&action);
        }
        // Nada queda en curso mientras se pregunta si descargarlo.
        self.abort_launch(&action);
        self.ask_fetch = Some((hf, mmproj, action));
    }
    /// Acepta descargar el modelo pendiente; al terminar se lanza el script.
    pub fn accept_fetch(&mut self) {
        let Some((hf, mmproj, action)) = self.ask_fetch.take() else {
            return;
        };
        match &action {
            Action::Start(script) => {
                self.manual_pending = true;
                self.library.started = Some(script.clone());
            }
            Action::Benchmark(_) => {
                self.bench_started = Some(Instant::now());
                self.bench_state = "descargando el modelo…".into();
                self.show_response = false;
            }
            _ => {}
        }
        let (repo, quant) = hf.split_once(':').unwrap_or((&hf, ""));
        self.hub.downloads.insert(
            hf.clone(),
            library::Download {
                repo: repo.to_owned(),
                quant: quant.to_owned(),
                done: 0,
                total: 0,
                started: Instant::now(),
            },
        );
        self.actions.push(Action::Fetch(hf.clone(), mmproj));
        self.fetching = Some((hf, action));
    }
    /// Descarta arrancar el script cuyo modelo falta.
    pub fn decline_fetch(&mut self) {
        self.ask_fetch = None;
    }
    /// Detiene el benchmark, o la descarga de su modelo si aún no empezó.
    pub fn cancel_benchmark(&mut self) {
        match &self.fetching {
            Some((key, Action::Benchmark(_))) => {
                self.actions.push(Action::CancelDownload(key.clone()))
            }
            _ => self.actions.push(Action::Cancel),
        }
    }
    /// Deshace el estado de un `Start` o `Benchmark` que no llegó a correr.
    fn abort_launch(&mut self, action: &Action) {
        match action {
            Action::Benchmark(_) => {
                self.bench_started = None;
                self.bench_state.clear();
            }
            Action::Start(_) => {
                self.manual_pending = false;
                self.library.started = None;
            }
            _ => {}
        }
    }
    pub fn stop_server(&mut self) {
        if self.running() && !self.manual_pending {
            self.manual_pending = true;
            self.actions.push(Action::Stop);
        }
    }
    /// Benchmark de un script de la biblioteca sin tocar el editor.
    pub fn bench_script(&mut self, id: &str) {
        let Some(script) = self.library.get(id).map(|s| s.script.clone()) else {
            return;
        };
        if self.benchmark(script) {
            self.library.touch(id);
            self.save_library();
        }
    }
    /// Lleva un script al editor de la vista ampliada.
    pub fn open_in_editor(&mut self, script: String, source: Option<String>) {
        self.script = script;
        self.editor_source = source;
        self.script_seeded = true;
        self.simple = false;
        self.tab = TAB_BENCH;
    }
    /// El script guardado ligado al editor, si sigue en la biblioteca.
    pub fn editor_saved(&self) -> Option<&scripts::SavedScript> {
        self.library.get(self.editor_source.as_deref()?)
    }
    /// Sobrescribe el script ligado al editor con el texto del editor.
    pub fn update_saved_from_editor(&mut self) {
        let Some(saved) = self.editor_saved() else {
            return;
        };
        let name = saved.name.clone();
        let draft = library::Draft {
            id: Some(saved.id.clone()),
            name: name.clone(),
            script: self.script.clone(),
            to_editor: true,
        };
        self.library.commit(draft);
        self.save_library();
        self.notice(format!("Script «{name}» actualizado"));
    }
    /// Abre el diálogo para guardar el editor como un script nuevo de la biblioteca.
    pub fn save_editor_as_new(&mut self) {
        let name = self
            .editor_saved()
            .map(|s| format!("{} (copia)", s.name))
            .unwrap_or_default();
        self.library.draft = Some(library::Draft {
            id: None,
            name,
            script: self.script.clone(),
            to_editor: true,
        });
    }
    pub fn running(&self) -> bool {
        store_logic::running(&self.status)
    }
    pub fn busy(&self) -> bool {
        self.bench_started.is_some() || self.dryfit_running
    }
    pub fn notice(&mut self, text: impl Into<String>) {
        self.notice_with_kind(text, NoticeKind::Success);
    }
    pub fn notice_with_kind(&mut self, text: impl Into<String>, kind: NoticeKind) {
        self.notices.push_back((Instant::now(), text.into(), kind));
        while self.notices.len() > 5 {
            self.notices.pop_front();
        }
    }
    pub fn confirm(&mut self, text: impl Into<String>, action: Action) {
        self.confirmation = Some(Confirmation {
            text: text.into(),
            action,
        });
    }
    pub fn run_benchmark(&mut self) {
        if self.running() || self.busy() || self.manual_pending {
            return;
        }
        self.script = format::format_script(&self.script);
        self.benchmark(self.script.clone());
    }
    /// Lanza el benchmark de `script` con el prompt actual. `false` si no se puede ahora.
    pub fn benchmark(&mut self, script: String) -> bool {
        if self.running() || self.busy() || self.manual_pending {
            return false;
        }
        self.bench_started = Some(Instant::now());
        self.bench_state = "iniciando servidor y midiendo… (puede tardar)".into();
        self.show_response = false;
        self.launch(Action::Benchmark(BenchmarkRequest {
            script,
            prompt: Some(self.prompt.clone()),
            max_tokens: self.max_tokens_enabled.then_some(self.max_tokens),
        }));
        true
    }
    /// Pide los detalles (cuantizaciones y modelo base) de un modelo del Hub.
    pub fn select_hub_model(&mut self, id: String) {
        if self.hub.selected.as_deref() == Some(id.as_str()) {
            return;
        }
        self.hub.selected = Some(id.clone());
        self.hub.details = None;
        self.hub.details_error = None;
        self.hub.details_loading = true;
        self.actions.push(Action::HubDetails(id));
    }
    /// Busca en el Hub si cambió la consulta o el orden.
    pub fn search_hub(&mut self) {
        let wanted = (self.hub.hub_query.trim().to_owned(), self.hub.hub_sort);
        if self.hub.hub_sent.as_ref() == Some(&wanted) {
            return;
        }
        self.hub.hub_serial += 1;
        self.hub.hub_loading = true;
        self.hub.hub_edited = None;
        self.actions.push(Action::HubSearch(
            self.hub.hub_serial,
            wanted.0.clone(),
            wanted.1,
        ));
        self.hub.hub_sent = Some(wanted);
    }
    /// Descarga una cuantización (y su mmproj más pequeño si está activado).
    pub fn download(&mut self, quant: &str) {
        let Some(d) = &self.hub.details else {
            return;
        };
        let Some(q) = d.quants.iter().find(|q| q.quant == quant) else {
            return;
        };
        let mut files = q.files.clone();
        // El mmproj más pequeño (F16 o BF16) basta para la visión.
        if self.hub.with_mmproj
            && let Some(m) = d.mmproj.first()
        {
            files.push(m.clone());
        }
        let key = library::download_key(&d.id, quant);
        let plan = plane_llama_core::hub::DownloadPlan {
            repo: d.id.clone(),
            revision: d.sha.clone(),
            files,
        };
        self.hub.downloads.insert(
            key.clone(),
            library::Download {
                repo: d.id.clone(),
                quant: quant.to_owned(),
                done: 0,
                total: plan.total(),
                started: Instant::now(),
            },
        );
        self.actions.push(Action::Download(key, plan));
    }
    pub fn sort_by(&mut self, col: &str) {
        if self.sort.col == col && self.sort.dir == "asc" {
            self.sort.dir = "desc".into();
        } else if self.sort.col == col && self.sort.dir == "desc" {
            self.sort.col.clear();
            self.sort.dir.clear();
        } else {
            self.sort = Sort {
                col: col.into(),
                dir: "asc".into(),
            };
        }
    }
    pub fn rows(&self) -> Vec<&BenchmarkResult> {
        store_logic::sort_history(&self.history, &self.sort.col, self.sort.dir == "desc")
            .into_iter()
            .filter(|r| {
                self.models.is_empty()
                    || format::model_base(r.config.model.as_deref())
                        .is_some_and(|m| self.models.contains(&m))
            })
            .filter(|r| self.matches_filter(r))
            .collect()
    }
    /// Texto libre: busca en el modelo completo y en las columnas visibles, sin
    /// distinguir mayúsculas.
    fn matches_filter(&self, r: &BenchmarkResult) -> bool {
        let needle = self.history_filter.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let model = r.config.model.as_deref().unwrap_or_default().to_lowercase();
        model.contains(&needle)
            || self.columns.iter().any(|key| {
                crate::history_table::cell(r, key)
                    .to_lowercase()
                    .contains(&needle)
            })
    }
    pub fn append(&mut self, entry: LogEntry) {
        if entry.seq <= self.cursor {
            return;
        }
        self.cursor = entry.seq;
        self.logs.push_back(entry);
        while self.logs.len() > config::LOG_CAP {
            self.logs.pop_front();
        }
    }
    pub fn receive(&mut self, message: Message) {
        match message {
            Message::Event(CoreEvent::Status { status }) => {
                self.status = status;
                self.manual_pending = false;
            }
            Message::Event(CoreEvent::Log { entry }) => self.append(entry),
            Message::Logs(v) => {
                for e in v.entries {
                    self.append(e);
                }
                self.cursor = self.cursor.max(v.cursor);
            }
            Message::LogsCleared(cursor) => {
                self.logs.retain(|e| e.seq > cursor);
                self.cursor = self.cursor.max(cursor);
            }
            Message::Gpu(v) => {
                self.gpus = v.gpus;
                self.ram = v.ram;
            }
            Message::History(v) => {
                self.selected.retain(|id| v.iter().any(|r| &r.id == id));
                self.history = v;
                self.refresh_library();
            }
            Message::Scripts(v) => {
                self.library.list = v;
                self.library.loaded = true;
                self.refresh_library();
            }
            Message::LocalModels(v) => {
                self.hub.local = v;
                self.hub.local_loaded = true;
                self.refresh_library();
            }
            Message::ModelDeleted(hf, freed, v) => {
                self.hub.local = v;
                self.refresh_library();
                self.notice(format!(
                    "{hf} eliminado · {} liberados",
                    crate::models_page::bytes(freed)
                ));
            }
            Message::HubResults(serial, v) => {
                if serial == self.hub.hub_serial {
                    self.hub.hub_loading = false;
                    self.hub.hub_error = None;
                    if self.hub.selected.is_none()
                        && let Some(first) = v.first()
                    {
                        self.select_hub_model(first.id.clone());
                    }
                    self.hub.hub_results = v;
                }
            }
            Message::HubDetails(v) => {
                if self.hub.selected.as_deref() == Some(v.id.as_str()) {
                    self.hub.details_loading = false;
                    self.hub.details_error = None;
                    self.hub.details = Some(*v);
                }
            }
            Message::DownloadProgress(key, done, total) => {
                if let Some(d) = self.hub.downloads.get_mut(&key) {
                    d.done = done;
                    d.total = total;
                }
            }
            Message::DownloadEnded(key, result) => {
                self.hub.downloads.remove(&key);
                if let Some((_, action)) = self.fetching.take_if(|(k, _)| *k == key) {
                    if matches!(result, Some(Ok(_))) {
                        if matches!(action, Action::Benchmark(_)) {
                            self.bench_state =
                                "iniciando servidor y midiendo… (puede tardar)".into();
                        }
                        self.actions.push(action);
                    } else {
                        self.abort_launch(&action);
                    }
                }
                match result {
                    Some(Ok(_)) => self.notice(format!("Descargado {key}")),
                    Some(Err(e)) => self.notice_with_kind(
                        format!("No se pudo descargar {key}: {e}"),
                        NoticeKind::Error,
                    ),
                    None => self.notice_with_kind(
                        format!("Descarga cancelada: {key}. Se retoma al volver a descargar."),
                        NoticeKind::Warning,
                    ),
                }
            }
            Message::InitialScript(v) => {
                if !self.script_seeded && self.script.is_empty() {
                    self.script = v;
                }
            }
            Message::InitialPrompt(v) => {
                if !self.prompt_seeded && self.prompt == metrics::DEFAULT_PROMPT && !v.is_empty() {
                    self.prompt = v;
                }
            }
            Message::Script(v) => {
                self.script = v;
                self.editor_source = None;
            }
            Message::Prompt(v) => self.prompt = v,
            Message::Favorites(v) => self.favorites = v,
            Message::Result(v) => {
                self.bench_started = None;
                self.bench_state.clear();
                self.show_response = true;
                self.notice_with_kind(
                    if v.errors.is_empty() {
                        "Benchmark completado".into()
                    } else {
                        format!("Benchmark con errores: {}", v.errors.join("; "))
                    },
                    if v.errors.is_empty() {
                        NoticeKind::Success
                    } else {
                        NoticeKind::Warning
                    },
                );
                self.last_result = Some(*v);
                self.result_banner = true;
                self.actions.push(Action::History);
            }
            Message::Estimate(id, v) => {
                if let Some(o) = &mut self.optimizer
                    && o.id == id
                {
                    o.loading = false;
                    o.calibration = self.calibrations.get(&v.model_meta.raw).cloned();
                    o.estimate = Some(*v);
                }
            }
            Message::Dryfit(v) => {
                self.dryfit_running = false;
                if let Some(o) = &mut self.optimizer {
                    o.calibrating = false;
                    o.error = v.error.clone();
                    if v.error.is_none()
                        && v.total_mi_b.is_some()
                        && let (Some(h), Some(e)) = (o.calibrating_heuristic.take(), &o.estimate)
                    {
                        let c = Calibration {
                            measured: v,
                            heuristic: h,
                        };
                        self.calibrations
                            .insert(e.model_meta.raw.clone(), c.clone());
                        o.calibration = Some(c);
                    }
                }
            }
            Message::Notice(v) => self.notice(v),
            Message::Failed(a, e) => {
                if matches!(a, Action::Dryfit(_)) {
                    self.dryfit_running = false;
                }
                match a {
                    Action::Benchmark(_) | Action::Start(_) => self.abort_launch(&a),
                    Action::Stop => self.manual_pending = false,
                    Action::Estimate(id, _) => {
                        if let Some(o) = &mut self.optimizer
                            && o.id == id
                        {
                            o.loading = false;
                            o.error = Some(e.clone());
                        }
                    }
                    Action::Dryfit(_) => {
                        if let Some(o) = &mut self.optimizer {
                            o.loading = false;
                            o.calibrating = false;
                            o.error = Some(e.clone());
                        }
                    }
                    Action::Favorites(_) => self.actions.push(Action::LoadFavorites),
                    Action::Scripts => self.library.loaded = true,
                    // Los errores del Hub se muestran en su propia página.
                    Action::HubSearch(serial, ..) => {
                        if serial == self.hub.hub_serial {
                            self.hub.hub_loading = false;
                            self.hub.hub_error = Some(e);
                        }
                        return;
                    }
                    Action::HubDetails(id) => {
                        if self.hub.selected.as_deref() == Some(id.as_str()) {
                            self.hub.details_loading = false;
                            self.hub.details_error = Some(e);
                        }
                        return;
                    }
                    _ => {}
                }
                self.notice_with_kind(e, NoticeKind::Error);
            }
        }
    }
}

#[cfg(test)]
mod window_key_tests {
    use super::*;
    #[test]
    fn forget_eframe_window_keeps_the_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui.ron");
        let sizes = r#"{\"compact\":[700.0,500.0],\"extended\":null,\"monitor\":null}"#;
        std::fs::write(
            &path,
            format!(
                r#"{{"window": "(inner_size_points:Some((x:1916.0,y:1164.0)))", "plane-llama-simple": "false", "plane-llama-window": "{sizes}"}}"#
            ),
        )
        .unwrap();
        forget_eframe_window(&path);
        let map: std::collections::HashMap<String, String> =
            ron::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(!map.contains_key(EFRAME_WINDOW_KEY));
        assert!(!saved_simple(&path));
        assert_eq!(saved_window(&path).compact, Some([700., 500.]));
        // Sin la clave no se toca el archivo; sin archivo no pasa nada.
        forget_eframe_window(&path);
        forget_eframe_window(&dir.path().join("no-existe.ron"));
    }
}
