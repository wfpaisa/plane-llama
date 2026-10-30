// Rutas y límites de persistencia.
use std::path::PathBuf;
pub const HISTORY_CAP: usize = 200;
pub const LOG_CAP: usize = 5000;
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: std::env::var_os("DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::config_dir()
                        .unwrap_or_else(|| PathBuf::from(".config"))
                        .join("plane-llama")
                }),
        }
    }
}
impl Config {
    pub fn file(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }
    pub fn ensure_data_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }
}
