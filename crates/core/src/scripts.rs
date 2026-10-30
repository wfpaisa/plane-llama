// Biblioteca de scripts guardados (`DATA_DIR/scripts.json`): los que la vista simple
// lista para arrancar con un clic.
use crate::{BenchmarkResult, Config, Result, client::format, models};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedScript {
    pub id: String,
    pub name: String,
    pub script: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: Option<String>,
    /// Oculto en la lista simple; sigue en la biblioteca.
    pub hidden: bool,
    pub favorite: bool,
}
impl SavedScript {
    pub fn new(name: impl Into<String>, script: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let script = format::format_script(&script.into());
        let name = name.into();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: if name.trim().is_empty() {
                models::script_title(&script)
            } else {
                name
            },
            script,
            created_at: now.clone(),
            updated_at: now,
            ..Default::default()
        }
    }
}

const FILE: &str = "scripts.json";

/// `None` si todavía no existe la biblioteca.
pub fn load(cfg: &Config) -> Result<Option<Vec<SavedScript>>> {
    match std::fs::read_to_string(cfg.file(FILE)) {
        Ok(s) => Ok(Some(serde_json::from_str(&s)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn save(cfg: &Config, scripts: &[SavedScript]) -> Result<()> {
    cfg.ensure_data_dir()?;
    let tmp = cfg.file("scripts.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(scripts)?)?;
    std::fs::rename(tmp, cfg.file(FILE))?;
    Ok(())
}

/// Primera biblioteca: el script por defecto y cada script distinto del historial, del más
/// rápido al más lento, con el nombre de su modelo.
pub fn seed(history: &[BenchmarkResult], default_script: Option<&str>) -> Vec<SavedScript> {
    let mut best: Vec<(String, f64, String)> = vec![];
    for r in history.iter().filter(|r| r.errors.is_empty()) {
        let script = format::format_script(&r.config.script);
        if script.trim().is_empty() {
            continue;
        }
        let tps = r.generation_tokens_per_second.unwrap_or(0.);
        match best.iter_mut().find(|(s, _, _)| *s == script) {
            Some(entry) => {
                if tps > entry.1 {
                    entry.1 = tps;
                }
                if r.timestamp > entry.2 {
                    entry.2 = r.timestamp.clone();
                }
            }
            None => best.push((script, tps, r.timestamp.clone())),
        }
    }
    best.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out: Vec<SavedScript> = vec![];
    if let Some(default) = default_script.filter(|s| !s.trim().is_empty()) {
        let formatted = format::format_script(default);
        if !best.iter().any(|(s, _, _)| *s == formatted) {
            out.push(SavedScript::new("", formatted));
        }
    }
    for (script, _, last) in best.into_iter().take(40) {
        let mut saved = SavedScript::new("", script);
        saved.last_run_at = Some(last);
        out.push(saved);
    }
    // Nombres repetidos llevan un número para distinguirlos en la lista.
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for s in &mut out {
        let n = seen.entry(s.name.clone()).or_default();
        *n += 1;
        if *n > 1 {
            s.name = format!("{} ({n})", s.name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn result(script: &str, tps: f64, ts: &str) -> BenchmarkResult {
        let mut r = BenchmarkResult {
            timestamp: ts.into(),
            generation_tokens_per_second: Some(tps),
            ..Default::default()
        };
        r.config.script = script.into();
        r
    }
    #[test]
    fn seeds_unique_scripts_by_speed() {
        let a = "llama-server -hf a/X-GGUF:Q8_0 --ctx-size 8192";
        let b = "llama-server -hf b/Y-GGUF:Q4_K_M";
        let history = [
            result(a, 10., "2026-01-01"),
            result(b, 30., "2026-01-02"),
            result(&format::format_script(a), 20., "2026-01-03"),
        ];
        let seeded = seed(&history, Some(a));
        assert_eq!(seeded.len(), 2);
        assert!(seeded[0].script.contains("b/Y-GGUF"));
        assert_eq!(seeded[1].last_run_at.as_deref(), Some("2026-01-03"));
        assert!(seeded[1].name.starts_with("a/X"));
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            data_dir: dir.path().into(),
        };
        assert!(load(&cfg).unwrap().is_none());
        save(&cfg, &seeded).unwrap();
        assert_eq!(load(&cfg).unwrap().unwrap(), seeded);
    }
}
