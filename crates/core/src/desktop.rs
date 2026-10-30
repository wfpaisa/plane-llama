// Integración con el escritorio: ajustes de segundo plano y arranque automático (XDG).
//
// Los ajustes son por app (`settings.<app_id>.json` en DATA_DIR) porque la versión egui
// y la Tauri comparten DATA_DIR pero cada una tiene su propia entrada de autostart.
use crate::{Config, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Ajustes de segundo plano y arranque, editables desde la UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Cerrar la ventana la oculta en la bandeja en vez de salir.
    pub close_to_tray: bool,
    /// Arrancar la app con la sesión del usuario (entrada en ~/.config/autostart).
    pub autostart: bool,
    /// Al arrancar con la sesión, empezar oculta en la bandeja.
    pub autostart_hidden: bool,
    /// Script que se lanza con `Core::start` al arrancar la app; `None` = no lanzar nada.
    pub launch_script: Option<String>,
    /// Script guardado elegido para el arranque. Si sigue en la biblioteca, se lanza su
    /// texto actual; `launch_script` queda como copia por si se borra.
    pub launch_script_id: Option<String>,
    /// Segundos de espera antes de lanzar el script al arrancar.
    pub launch_delay_secs: u32,
    /// Fondo translúcido con desenfoque del compositor, si lo soporta.
    pub blur: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            autostart: false,
            autostart_hidden: true,
            launch_script: None,
            launch_script_id: None,
            launch_delay_secs: 0,
            blur: false,
        }
    }
}

impl AppSettings {
    /// Script que hay que lanzar: el guardado elegido (con sus cambios) o la copia.
    pub fn resolve_launch_script(&self, saved: &[crate::scripts::SavedScript]) -> Option<String> {
        self.launch_script_id
            .as_deref()
            .and_then(|id| saved.iter().find(|s| s.id == id))
            .map(|s| s.script.clone())
            .or_else(|| self.launch_script.clone())
    }
}

/// Flag de línea de comandos con el que el autostart arranca la app oculta.
pub const HIDDEN_FLAG: &str = "--hidden";

fn settings_file(cfg: &Config, app_id: &str) -> PathBuf {
    cfg.file(&format!("settings.{app_id}.json"))
}

/// Lee los ajustes de la app; si no existen o están corruptos, devuelve los de fábrica.
pub fn load_settings(cfg: &Config, app_id: &str) -> AppSettings {
    std::fs::read_to_string(settings_file(cfg, app_id))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Guarda los ajustes (escritura atómica) y sincroniza la entrada de autostart.
pub fn save_settings(cfg: &Config, app_id: &str, exec: &Path, s: &AppSettings) -> Result<()> {
    cfg.ensure_data_dir()?;
    let path = settings_file(cfg, app_id);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, &path)?;
    set_autostart(app_id, exec, s.autostart, s.autostart_hidden)
}

fn autostart_file(app_id: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("autostart").join(format!("{app_id}.desktop")))
}

/// Si la entrada de autostart de la app existe.
pub fn autostart_enabled(app_id: &str) -> bool {
    autostart_file(app_id).is_some_and(|p| p.exists())
}

/// Crea o borra `~/.config/autostart/<app_id>.desktop` apuntando a `exec`.
pub fn set_autostart(app_id: &str, exec: &Path, enabled: bool, hidden: bool) -> Result<()> {
    let Some(path) = autostart_file(app_id) else {
        return Ok(());
    };
    if !enabled {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => return Ok(()),
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, desktop_entry(app_id, exec, hidden))?;
    Ok(())
}

/// Contenido del `.desktop` de autostart. La ruta va entre comillas por si tiene espacios.
pub fn desktop_entry(app_id: &str, exec: &Path, hidden: bool) -> String {
    let flag = if hidden {
        format!(" {HIDDEN_FLAG}")
    } else {
        String::new()
    };
    format!(
        "[Desktop Entry]\nType=Application\nName=plane-llama\nComment=Benchmark de llama-server\n\
         Exec=\"{}\"{flag}\nIcon={app_id}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            data_dir: dir.path().into(),
        };
        assert_eq!(load_settings(&cfg, "x"), AppSettings::default());
        std::fs::write(
            cfg.file("settings.x.json"),
            r#"{"launchScript":"llama-server -m a.gguf"}"#,
        )
        .unwrap();
        let s = load_settings(&cfg, "x");
        assert_eq!(s.launch_script.as_deref(), Some("llama-server -m a.gguf"));
        assert!(s.close_to_tray, "los campos ausentes toman el default");
        assert_eq!(s.launch_delay_secs, 0);
    }

    #[test]
    fn launch_script_follows_saved_script_or_falls_back_to_copy() {
        let mut saved = crate::scripts::SavedScript::new("A", "llama-server -m a.gguf");
        let s = AppSettings {
            launch_script: Some("llama-server -m vieja.gguf".into()),
            launch_script_id: Some(saved.id.clone()),
            ..Default::default()
        };
        saved.script = "llama-server -m nueva.gguf".into();
        assert_eq!(
            s.resolve_launch_script(std::slice::from_ref(&saved))
                .as_deref(),
            Some("llama-server -m nueva.gguf")
        );
        assert_eq!(
            s.resolve_launch_script(&[]).as_deref(),
            Some("llama-server -m vieja.gguf")
        );
        assert_eq!(AppSettings::default().resolve_launch_script(&[]), None);
    }

    #[test]
    fn desktop_entry_quotes_exec_and_adds_hidden() {
        let e = desktop_entry(
            "plane-llama-egui",
            Path::new("/opt/mi app/plane-llama"),
            true,
        );
        assert!(e.contains("Exec=\"/opt/mi app/plane-llama\" --hidden\n"));
        assert!(!desktop_entry("a", Path::new("/a"), false).contains(HIDDEN_FLAG));
    }
}
