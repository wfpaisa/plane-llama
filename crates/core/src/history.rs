// Historial en memoria y escritura atómica de cada cambio.
use crate::{
    BenchmarkResult, Config, CoreError, Result, ResultPatch, config::HISTORY_CAP,
    script_parser::parse_script,
};
pub struct History {
    cfg: Config,
    results: Vec<BenchmarkResult>,
}
impl History {
    pub fn new(cfg: Config) -> Result<Self> {
        cfg.ensure_data_dir()?;
        let mut results: Vec<BenchmarkResult> = std::fs::read(cfg.file("history.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        for r in &mut results {
            if r.config.model.is_none()
                && let Ok(p) = parse_script(&r.config.script)
            {
                r.config.model = p.model;
            }
        }
        Ok(Self { cfg, results })
    }
    pub fn all(&self) -> Vec<BenchmarkResult> {
        self.results.clone()
    }
    fn flush(&self) -> Result<()> {
        let tmp = self.cfg.file("history.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.results)?)?;
        std::fs::rename(tmp, self.cfg.file("history.json"))?;
        Ok(())
    }
    pub fn save(&mut self, r: BenchmarkResult) -> Result<()> {
        self.results.insert(0, r);
        self.results.truncate(HISTORY_CAP);
        self.flush()
    }
    pub fn delete(&mut self, ids: &[String]) -> Result<()> {
        self.results.retain(|r| !ids.contains(&r.id));
        self.flush()
    }
    pub fn clear(&mut self) -> Result<()> {
        self.results.clear();
        self.flush()
    }
    pub fn patch(&mut self, id: &str, patch: ResultPatch) -> Result<BenchmarkResult> {
        if patch
            .rating
            .flatten()
            .is_some_and(|r| !r.is_finite() || !(0.0..=10.0).contains(&r))
        {
            return Err(CoreError::Invalid("rating debe estar entre 0 y 10.".into()));
        }
        let row = self
            .results
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(CoreError::NotFound)?;
        if let Some(r) = patch.rating {
            row.rating = Some(r);
            row.missing_fields.remove("rating");
        }
        if let Some(f) = patch.favorite {
            row.favorite = Some(f);
            row.missing_fields.remove("favorite");
        }
        let result = row.clone();
        self.flush()?;
        Ok(result)
    }
}
