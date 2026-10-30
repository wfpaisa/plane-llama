// Estado compartido por instancia; no contiene variables globales.
use crate::{StatusResponse, logs::Logs};
use parking_lot::RwLock;
pub struct AppState {
    pub logs: Logs,
    pub status: RwLock<StatusResponse>,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            logs: Logs::default(),
            status: RwLock::new(StatusResponse {
                status: "stopped".into(),
                ..Default::default()
            }),
        }
    }
}
