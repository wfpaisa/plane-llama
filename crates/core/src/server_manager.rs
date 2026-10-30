// Procesos en grupos propios, salida a logs y detección de disponibilidad.
use crate::{CoreError, CoreEvent, ParsedScript, Result, StatusResponse, state::AppState};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use std::{path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    sync::{mpsc, watch},
};

pub fn assert_binary_exists(binary: &str) -> Result<PathBuf> {
    if binary.is_empty() {
        return Err(CoreError::Invalid(
            "El script no especifica un binario de llama-server.".into(),
        ));
    }
    if binary.contains('/') || binary.starts_with('.') {
        let path = std::path::absolute(binary)?;
        if path.exists() {
            return Ok(path);
        }
        return Err(CoreError::Invalid(format!(
            "El binario no existe: {}",
            path.display()
        )));
    }
    which::which(binary).map_err(|_| {
        CoreError::Invalid(format!(
            "No se encontró el binario \"{binary}\" en el PATH."
        ))
    })
}
pub fn command(binary: &str) -> Result<tokio::process::Command> {
    let path = assert_binary_exists(binary)?;
    let dir = path
        .parent()
        .ok_or_else(|| CoreError::Invalid("Directorio del binario inválido.".into()))?;
    let mut cmd = tokio::process::Command::new(&path);
    let mut ld = dir.as_os_str().to_os_string();
    if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
        ld.push(":");
        ld.push(existing);
    }
    cmd.current_dir(dir).env("LD_LIBRARY_PATH", ld);
    Ok(cmd)
}
/// URL para conectarse al servidor. `0.0.0.0` y `::` escuchan en todas las interfaces,
/// pero como destino se usa el equipo local.
pub fn url_for(parsed: &ParsedScript) -> String {
    let host = match parsed.host.as_str() {
        "" | "0.0.0.0" => "127.0.0.1",
        "::" | "[::]" => "[::1]",
        h if h.contains(':') && !h.starts_with('[') => {
            return format!("http://[{h}]:{}", parsed.port);
        }
        h => h,
    };
    format!("http://{host}:{}", parsed.port)
}
pub fn set_status(state: &AppState, status: StatusResponse) {
    *state.status.write() = status.clone();
    let _ = state.logs.events.send(CoreEvent::Status { status });
}
#[derive(Clone)]
pub struct ManagedServer {
    pub pid: u32,
    ready: watch::Receiver<Option<std::result::Result<(), String>>>,
    exited: watch::Receiver<bool>,
    stop: mpsc::UnboundedSender<Duration>,
}
impl ManagedServer {
    pub async fn ready(&self) -> Result<()> {
        let mut ready = self.ready.clone();
        loop {
            if let Some(result) = ready.borrow().clone() {
                return result.map_err(CoreError::Invalid);
            }
            ready
                .changed()
                .await
                .map_err(|_| CoreError::Invalid("El proceso terminó sin estar listo.".into()))?;
        }
    }
    pub fn is_exited(&self) -> bool {
        *self.exited.borrow()
    }
    pub async fn stop(&self, timeout: Duration) {
        let _ = self.stop.send(timeout);
        let mut exited = self.exited.clone();
        while !*exited.borrow() {
            if exited.changed().await.is_err() {
                break;
            }
        }
    }
}
async fn drain(
    reader: impl AsyncRead + Unpin,
    stream: &'static str,
    state: Arc<AppState>,
    ready: watch::Sender<Option<std::result::Result<(), String>>>,
    pid: u32,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        state.logs.push(stream, &line);
        let lower = line.to_lowercase();
        if [
            "server is listening",
            "http server listening",
            "all slots are ready",
            "listening on http://",
            "listening on https://",
        ]
        .iter()
        .any(|s| lower.contains(s))
            && ready.borrow().is_none()
        {
            ready.send_replace(Some(Ok(())));
            let mut status = state.status.read().clone();
            if status.pid == Some(pid as f64) {
                status.status = "running".into();
                set_status(&state, status);
            }
        }
    }
}
pub async fn start_server(parsed: &ParsedScript, state: Arc<AppState>) -> Result<ManagedServer> {
    let mut cmd = command(&parsed.binary)?;
    cmd.args(&parsed.argv)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    state.logs.push("command", &parsed.script);
    let mut child = cmd.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| CoreError::Invalid("El proceso no tiene PID.".into()))?;
    let (ready_tx, ready) = watch::channel(None);
    let (exit_tx, exited) = watch::channel(false);
    let (stop, mut stop_rx) = mpsc::unbounded_channel();
    set_status(
        &state,
        StatusResponse {
            status: "starting".into(),
            pid: Some(pid as f64),
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            url: Some(url_for(parsed)),
            ..Default::default()
        },
    );
    let stdout = child
        .stdout
        .take()
        .map(|r| tokio::spawn(drain(r, "stdout", state.clone(), ready_tx.clone(), pid)));
    let stderr = child
        .stderr
        .take()
        .map(|r| tokio::spawn(drain(r, "stderr", state.clone(), ready_tx.clone(), pid)));
    tokio::spawn(async move {
        let mut stopped = false;
        let result = tokio::select! {
            result = child.wait() => result,
            timeout = stop_rx.recv() => {
                stopped = true;
                let _ = killpg(Pid::from_raw(pid as i32), Signal::SIGTERM);
                let mut deadline = tokio::time::Instant::now() + timeout.unwrap_or(Duration::from_secs(3));
                loop {
                    tokio::select! {
                        result = child.wait() => break result,
                        next = stop_rx.recv(), if !stop_rx.is_closed() => {
                            deadline = deadline.min(tokio::time::Instant::now() + next.unwrap_or(Duration::from_secs(3)));
                        }
                        _ = tokio::time::sleep_until(deadline) => {
                            let _ = killpg(Pid::from_raw(pid as i32), Signal::SIGKILL);
                            break child.wait().await;
                        }
                    }
                }
            }
        };
        // El líder puede terminar dejando descendientes con los pipes abiertos.
        let _ = killpg(Pid::from_raw(pid as i32), Signal::SIGKILL);
        if let Some(task) = stdout {
            let _ = task.await;
        }
        if let Some(task) = stderr {
            let _ = task.await;
        }
        let error = format!(
            "llama-server terminó con código {}",
            result
                .as_ref()
                .ok()
                .and_then(|s| s.code())
                .map(|c| c.to_string())
                .unwrap_or_else(|| "señal".into())
        );
        let failed_start = ready_tx.borrow().is_none() && !stopped;
        if ready_tx.borrow().is_none() {
            ready_tx.send_replace(Some(Err(error.clone())));
        }
        if state.status.read().pid == Some(pid as f64) {
            set_status(
                &state,
                StatusResponse {
                    status: if failed_start { "error" } else { "stopped" }.into(),
                    error: failed_start.then_some(error),
                    ..Default::default()
                },
            );
        }
        exit_tx.send_replace(true);
    });
    Ok(ManagedServer {
        pid,
        ready,
        exited,
        stop,
    })
}
pub async fn stop_server(server: &ManagedServer) {
    server.stop(Duration::from_secs(8)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_urls_for_wildcard_hosts() {
        let with = |host: &str| {
            url_for(&ParsedScript {
                host: host.into(),
                port: 8080.,
                ..Default::default()
            })
        };
        assert_eq!(with(""), "http://127.0.0.1:8080");
        assert_eq!(with("0.0.0.0"), "http://127.0.0.1:8080");
        assert_eq!(with("::"), "http://[::1]:8080");
        assert_eq!(with("fe80::1"), "http://[fe80::1]:8080");
        assert_eq!(with("192.168.1.5"), "http://192.168.1.5:8080");
    }
}
