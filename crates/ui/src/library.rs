// Estado de la biblioteca de scripts y de los modelos (descargados y de Hugging Face).
use plane_llama_core::{
    BenchmarkResult,
    client::format,
    hub::{HubDetails, HubModel, HubSort},
    models::LocalModel,
    script_parser,
    scripts::SavedScript,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScriptSort {
    /// El orden de `scripts.json`, que el usuario cambia arrastrando las filas.
    Manual,
    /// Último uso primero. También absorbe los órdenes que ya no existen (nombre,
    /// velocidad, tamaño) guardados por versiones anteriores; por eso va al final.
    #[default]
    #[serde(other)]
    Recent,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScriptFilter {
    #[default]
    Visible,
    Hidden,
    All,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelsView {
    #[default]
    Local,
    Hub,
}

/// Datos derivados de un script; se recalculan solo cuando cambian scripts, historial o
/// modelos, no en cada cuadro.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptMeta {
    pub repo: Option<String>,
    pub quant: Option<String>,
    pub params: Option<String>,
    pub ctx: Option<f64>,
    pub best_tps: Option<f64>,
    pub runs: usize,
    pub size_bytes: Option<u64>,
    pub vision: bool,
    /// El modelo está en la caché; si no, `llama-server -hf` lo descarga al arrancar.
    pub downloaded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub id: Option<String>,
    pub name: String,
    pub script: String,
}

#[derive(Default)]
pub struct Scripts {
    pub list: Vec<SavedScript>,
    pub loaded: bool,
    pub meta: BTreeMap<String, ScriptMeta>,
    pub search: String,
    pub sort: ScriptSort,
    pub filter: ScriptFilter,
    pub draft: Option<Draft>,
    /// Texto del script con el que se arrancó el servidor desde la app.
    pub started: Option<String>,
    /// Scripts marcados en la página para eliminarlos juntos.
    pub selected: BTreeSet<String>,
}

/// Mejor generación (tok/s) y número de corridas sin error por script formateado.
pub type Bests = BTreeMap<String, (Option<f64>, usize)>;
pub fn bests(history: &[BenchmarkResult]) -> Bests {
    let mut out = Bests::new();
    for r in history.iter().filter(|r| r.errors.is_empty()) {
        let entry = out
            .entry(format::format_script(&r.config.script))
            .or_default();
        entry.1 += 1;
        if let Some(tps) = r.generation_tokens_per_second
            && entry.0.is_none_or(|b| tps > b)
        {
            entry.0 = Some(tps);
        }
    }
    out
}
pub fn meta_for(script: &str, bests: &Bests, local: &[LocalModel]) -> ScriptMeta {
    let parsed = script_parser::parse_script(script).ok();
    let model = parsed.as_ref().and_then(|p| p.model.clone());
    let parsed_model = format::parse_model(model.as_deref());
    let repo = model
        .as_deref()
        .map(|m| m.split(':').next().unwrap_or(m).to_owned())
        // `org/nombre` de Hugging Face, no una ruta de `-m`.
        .filter(|r| {
            r.contains('/')
                && !r.starts_with(['/', '.', '~'])
                && !r.to_lowercase().ends_with(".gguf")
        });
    let quant = parsed_model.as_ref().and_then(|m| m.quant.clone());
    let local_model = repo.as_deref().and_then(|repo| {
        let mut candidates = local.iter().filter(|m| m.repo.eq_ignore_ascii_case(repo));
        match quant.as_deref() {
            Some(q) => candidates.find(|m| m.quant.eq_ignore_ascii_case(q)),
            None => candidates.next(),
        }
    });
    let file = parsed.as_ref().and_then(|p| p.model_file.clone());
    let file_size = file
        .as_deref()
        .and_then(|f| std::fs::metadata(f).ok())
        .map(|m| m.len());
    let (best, runs) = bests.get(script).copied().unwrap_or_default();
    let no_mmproj = parsed.as_ref().is_some_and(|p| p.no_mmproj);
    ScriptMeta {
        quant,
        params: parsed_model.and_then(|m| m.size),
        ctx: parsed.as_ref().and_then(|p| p.ctx_size),
        best_tps: best,
        runs,
        size_bytes: local_model.map(|m| m.size_bytes).or(file_size),
        vision: local_model.is_some_and(|m| !m.mmproj.is_empty()) && !no_mmproj,
        downloaded: local_model.is_some() || file_size.is_some(),
        repo,
    }
}

impl Scripts {
    pub fn refresh_meta(&mut self, history: &[BenchmarkResult], local: &[LocalModel]) {
        let bests = bests(history);
        self.meta = self
            .list
            .iter()
            .map(|s| (s.id.clone(), meta_for(&s.script, &bests, local)))
            .collect();
    }
    pub fn get(&self, id: &str) -> Option<&SavedScript> {
        self.list.iter().find(|s| s.id == id)
    }
    fn get_mut(&mut self, id: &str) -> Option<&mut SavedScript> {
        self.list.iter_mut().find(|s| s.id == id)
    }
    fn matches(&self, s: &SavedScript) -> bool {
        let shown = match self.filter {
            ScriptFilter::Visible => !s.hidden,
            ScriptFilter::Hidden => s.hidden,
            ScriptFilter::All => true,
        };
        let q = self.search.trim().to_lowercase();
        shown
            && (q.is_empty()
                || s.name.to_lowercase().contains(&q)
                || s.script.to_lowercase().contains(&q))
    }
    /// Scripts que pasan el filtro, en una sola lista. En «Recientes» los favoritos van
    /// primero; en «Manual» manda el orden de la lista, que se cambia arrastrando.
    pub fn sorted(&self) -> Vec<&SavedScript> {
        let mut rows: Vec<&SavedScript> = self.list.iter().filter(|s| self.matches(s)).collect();
        if self.sort == ScriptSort::Recent {
            let key = |s: &SavedScript| {
                s.last_run_at
                    .clone()
                    .unwrap_or_else(|| s.updated_at.clone())
            };
            rows.sort_by(|a, b| b.favorite.cmp(&a.favorite).then(key(b).cmp(&key(a))));
        }
        rows
    }
    /// El script que corre ahora, si se arrancó desde la app y sigue en la biblioteca.
    pub fn running_id(&self, running: bool) -> Option<&str> {
        let started = self.started.as_deref().filter(|_| running)?;
        self.list
            .iter()
            .find(|s| s.script == started)
            .map(|s| s.id.as_str())
    }
    /// Guarda el borrador (nuevo o existente). Devuelve el id.
    pub fn commit(&mut self, draft: Draft) -> String {
        let script = format::format_script(&draft.script);
        match draft.id.as_deref().and_then(|id| self.get_mut(id)) {
            Some(s) => {
                s.name = draft.name.trim().to_owned();
                if s.name.is_empty() {
                    s.name = plane_llama_core::models::script_title(&script);
                }
                s.script = script;
                s.updated_at = chrono::Utc::now().to_rfc3339();
                s.id.clone()
            }
            None => {
                let s = SavedScript::new(draft.name.trim(), script);
                let id = s.id.clone();
                self.list.push(s);
                id
            }
        }
    }
    pub fn duplicate(&mut self, id: &str) -> Option<String> {
        let mut copy = self.get(id)?.clone();
        copy.id = uuid_like();
        copy.name = format!("{} (copia)", copy.name);
        copy.favorite = false;
        copy.hidden = false;
        copy.last_run_at = None;
        let new = copy.id.clone();
        let at = self
            .list
            .iter()
            .position(|s| s.id == id)
            .map_or(0, |i| i + 1);
        self.list.insert(at, copy);
        Some(new)
    }
    /// Orden manual: pone el script `id` justo antes (o después) de `target`.
    pub fn move_to(&mut self, id: &str, target: &str, after: bool) {
        if id == target {
            return;
        }
        let Some(from) = self.list.iter().position(|s| s.id == id) else {
            return;
        };
        let item = self.list.remove(from);
        let to = self
            .list
            .iter()
            .position(|s| s.id == target)
            .map_or(self.list.len(), |i| i + usize::from(after));
        self.list.insert(to, item);
    }
    pub fn remove(&mut self, id: &str) {
        self.list.retain(|s| s.id != id);
        self.meta.remove(id);
    }
    pub fn set_hidden(&mut self, id: &str, hidden: bool) {
        if let Some(s) = self.get_mut(id) {
            s.hidden = hidden;
        }
    }
    pub fn toggle_favorite(&mut self, id: &str) {
        if let Some(s) = self.get_mut(id) {
            s.favorite = !s.favorite;
        }
    }
    pub fn touch(&mut self, id: &str) {
        if let Some(s) = self.get_mut(id) {
            s.last_run_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}
fn uuid_like() -> String {
    SavedScript::new("x", "").id
}

pub struct Download {
    pub repo: String,
    pub quant: String,
    pub done: u64,
    pub total: u64,
    pub started: Instant,
}
pub struct Models {
    pub local: Vec<LocalModel>,
    pub local_loaded: bool,
    pub view: ModelsView,
    pub search: String,
    pub hub_query: String,
    pub hub_sort: HubSort,
    pub hub_results: Vec<HubModel>,
    pub hub_serial: u64,
    pub hub_loading: bool,
    pub hub_error: Option<String>,
    /// Última búsqueda pedida, para no repetirla.
    pub hub_sent: Option<(String, HubSort)>,
    /// Cuándo se editó la búsqueda: espera una pausa antes de consultar.
    pub hub_edited: Option<Instant>,
    pub selected: Option<String>,
    pub details: Option<HubDetails>,
    pub details_loading: bool,
    pub details_error: Option<String>,
    pub with_mmproj: bool,
    pub downloads: BTreeMap<String, Download>,
    pub picker_search: String,
}
impl Default for Models {
    fn default() -> Self {
        Self {
            local: vec![],
            local_loaded: false,
            view: ModelsView::Local,
            search: String::new(),
            hub_query: String::new(),
            hub_sort: HubSort::Recent,
            hub_results: vec![],
            hub_serial: 0,
            hub_loading: false,
            hub_error: None,
            hub_sent: None,
            hub_edited: None,
            selected: None,
            details: None,
            details_loading: false,
            details_error: None,
            with_mmproj: true,
            downloads: BTreeMap::new(),
            picker_search: String::new(),
        }
    }
}
impl Models {
    /// Modelos locales filtrados, agrupados por organización.
    pub fn local_groups(&self, search: &str) -> Vec<(String, Vec<&LocalModel>)> {
        let q = search.trim().to_lowercase();
        let mut groups: Vec<(String, Vec<&LocalModel>)> = vec![];
        for m in self
            .local
            .iter()
            .filter(|m| q.is_empty() || m.hf_ref().to_lowercase().contains(&q))
        {
            let org = m.org().to_owned();
            match groups.iter_mut().find(|(g, _)| *g == org) {
                Some((_, v)) => v.push(m),
                None => groups.push((org, vec![m])),
            }
        }
        groups
    }
    pub fn is_local(&self, repo: &str, quant: &str) -> bool {
        self.local
            .iter()
            .any(|m| m.repo.eq_ignore_ascii_case(repo) && m.quant.eq_ignore_ascii_case(quant))
    }
}
pub fn download_key(repo: &str, quant: &str) -> String {
    format!("{repo}:{quant}")
}

/// Preferencias de la biblioteca que se conservan entre sesiones.
#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct Prefs {
    pub sort: ScriptSort,
    pub filter: ScriptFilter,
    pub hub_sort: Option<HubSort>,
    pub with_mmproj: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn saved(name: &str, script: &str) -> SavedScript {
        SavedScript::new(name, script)
    }
    #[test]
    fn filters_and_sorts() {
        let mut lib = Scripts::default();
        let a = saved(
            "Alfa",
            "llama-server -hf unsloth/A-GGUF:Q8_0 --ctx-size 8192",
        );
        let b = saved("Beta", "llama-server -hf bartowski/B-GGUF:Q4_K_M");
        let mut c = saved("Gamma", "llama-server -m /no/existe.gguf");
        c.hidden = true;
        let mut d = saved("Delta", "llama-server -hf unsloth/D-GGUF:Q4_0");
        d.favorite = true;
        lib.list = vec![a.clone(), b.clone(), c.clone(), d.clone()];
        let mut r = BenchmarkResult {
            generation_tokens_per_second: Some(50.),
            ..Default::default()
        };
        r.config.script = "llama-server  -hf unsloth/A-GGUF:Q8_0   --ctx-size 8192".into();
        let local = vec![LocalModel {
            repo: "unsloth/A-GGUF".into(),
            quant: "Q8_0".into(),
            size_bytes: 9,
            mmproj: vec!["m".into()],
            ..Default::default()
        }];
        lib.refresh_meta(&[r], &local);
        let m = &lib.meta[&a.id];
        assert_eq!(
            (m.best_tps, m.runs, m.size_bytes, m.vision, m.downloaded),
            (Some(50.), 1, Some(9), true, true)
        );
        assert!(!lib.meta[&b.id].downloaded);

        // «Recientes»: favoritos primero, luego lo último usado.
        let names = |lib: &Scripts| {
            lib.sorted()
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&lib)[0], "Delta");
        // «Manual»: el orden de la lista, sin adelantar favoritos; se mueve cualquiera.
        lib.sort = ScriptSort::Manual;
        assert_eq!(names(&lib), ["Alfa", "Beta", "Delta"]);
        lib.move_to(&d.id, &b.id, true);
        assert_eq!(names(&lib), ["Alfa", "Beta", "Delta"]);
        lib.move_to(&a.id, &d.id, true);
        assert_eq!(names(&lib), ["Beta", "Delta", "Alfa"]);
        lib.move_to(&d.id, &b.id, false);
        assert_eq!(names(&lib), ["Delta", "Beta", "Alfa"]);
        // Los órdenes que ya no existen se leen como «Recientes».
        assert_eq!(
            serde_json::from_str::<ScriptSort>("\"Fastest\"").unwrap(),
            ScriptSort::Recent
        );
        lib.filter = ScriptFilter::Hidden;
        assert_eq!(names(&lib), ["Gamma"]);
        lib.filter = ScriptFilter::All;
        lib.search = "beta".into();
        assert_eq!(names(&lib), ["Beta"]);
    }
    #[test]
    fn drafts_duplicates_and_running() {
        let mut lib = Scripts::default();
        let id = lib.commit(Draft {
            id: None,
            name: " ".into(),
            script: "llama-server   -hf a/Q-GGUF:Q8_0".into(),
        });
        assert_eq!(lib.list[0].name, "a/Q");
        assert_eq!(lib.list[0].script, "llama-server \\\n-hf a/Q-GGUF:Q8_0");
        let copy = lib.duplicate(&id).unwrap();
        assert_ne!(copy, id);
        assert_eq!(lib.list[1].name, "a/Q (copia)");
        lib.commit(Draft {
            id: Some(id.clone()),
            name: "Mío".into(),
            script: "llama-server -hf a/Q-GGUF:Q4_0".into(),
        });
        assert_eq!(lib.get(&id).unwrap().name, "Mío");
        lib.started = Some(lib.get(&id).unwrap().script.clone());
        assert_eq!(lib.running_id(true), Some(id.as_str()));
        assert_eq!(lib.running_id(false), None);
        lib.remove(&id);
        assert_eq!(lib.list.len(), 1);
    }
}
