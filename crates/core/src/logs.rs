// Buffer circular con cursor monotónico y difusión de eventos.
use crate::{CoreEvent, LogEntry, LogsResponse, config::LOG_CAP};
use parking_lot::Mutex;
use std::{collections::VecDeque, time::Instant};
use tokio::sync::broadcast;
pub struct Logs {
    boot: Instant,
    inner: Mutex<(u64, VecDeque<LogEntry>)>,
    pub events: broadcast::Sender<CoreEvent>,
}
impl Default for Logs {
    fn default() -> Self {
        Self {
            boot: Instant::now(),
            inner: Mutex::new((0, VecDeque::new())),
            events: broadcast::channel(LOG_CAP).0,
        }
    }
}
impl Logs {
    pub fn push(&self, stream: &str, msg: &str) {
        let msg = if let Some(line) = msg.strip_suffix('\n') {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            msg
        };
        if msg.is_empty() {
            return;
        }
        let mut inner = self.inner.lock();
        inner.0 += 1;
        let entry = LogEntry {
            seq: inner.0 as f64,
            t: self.boot.elapsed().as_millis() as f64,
            stream: stream.into(),
            msg: msg.into(),
            ..Default::default()
        };
        inner.1.push_back(entry.clone());
        if inner.1.len() > LOG_CAP {
            inner.1.pop_front();
        }
        let _ = self.events.send(CoreEvent::Log { entry });
    }
    pub fn system(&self, msg: &str) {
        self.push("system", &format!("[backend] {msg}"));
    }
    pub fn read(&self, since: Option<u64>) -> LogsResponse {
        let inner = self.inner.lock();
        LogsResponse {
            cursor: inner.0 as f64,
            entries: inner
                .1
                .iter()
                .filter(|e| e.seq > since.unwrap_or(0) as f64)
                .cloned()
                .collect(),
            ..Default::default()
        }
    }
    pub fn clear(&self) {
        self.inner.lock().1.clear();
    }
}
