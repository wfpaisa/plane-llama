// Modelos GGUF descargados en la caché de Hugging Face y su uso en scripts.
use crate::client::{flag_writer, format};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// Una cuantización descargada de un repositorio (`org/nombre:QUANT`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModel {
    pub repo: String,
    pub quant: String,
    /// Rutas de los archivos, en orden de partes (`-00001-of-0000N`).
    pub files: Vec<String>,
    pub size_bytes: u64,
    /// Proyectores multimodales (`mmproj-*.gguf`) del mismo repositorio.
    pub mmproj: Vec<String>,
    /// Última modificación en segundos Unix.
    pub modified: i64,
}
impl LocalModel {
    pub fn hf_ref(&self) -> String {
        format!("{}:{}", self.repo, self.quant)
    }
    pub fn org(&self) -> &str {
        self.repo.split_once('/').map_or("", |(org, _)| org)
    }
    pub fn name(&self) -> &str {
        self.repo
            .split_once('/')
            .map_or(&self.repo, |(_, name)| name)
    }
}

/// Carpeta `hub` de la caché: `HF_HUB_CACHE`, `HF_HOME/hub` o `~/.cache/huggingface/hub`.
pub fn hub_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("HF_HUB_CACHE") {
        return dir.into();
    }
    std::env::var_os("HF_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".cache/huggingface")
        })
        .join("hub")
}
pub fn repo_dir(hub: &Path, repo: &str) -> PathBuf {
    hub.join(format!("models--{}", repo.replace('/', "--")))
}

static SPLIT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)-(\d{5})-of-\d{5}$").expect("regex de partes"));
static QUANT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)(?:^|[-_.])((?:UD-)?(?:I?Q[1-8](?:_[A-Z0-9]+)*|TQ[12]_0|BF16|F16|F32|MXFP4(?:_MOE)?))$",
    )
    .expect("regex de cuantización")
});

pub fn is_mmproj(name: &str) -> bool {
    let n = name.to_lowercase();
    n.ends_with(".gguf") && n.contains("mmproj")
}
/// Archivo de pesos: GGUF que no es proyector ni matriz de importancia (`imatrix`).
pub fn is_weights(name: &str) -> bool {
    let n = name.rsplit('/').next().unwrap_or(name).to_lowercase();
    n.ends_with(".gguf") && !n.contains("mmproj") && !n.contains("imatrix")
}
/// Cuantización a partir del nombre del archivo (`Qwen3.5-4B-UD-Q4_K_XL.gguf` → `UD-Q4_K_XL`).
/// Sin un patrón conocido devuelve el nombre sin extensión.
pub fn quant_of(file: &str) -> String {
    let name = file.rsplit('/').next().unwrap_or(file);
    let stem = name
        .strip_suffix(".gguf")
        .or_else(|| name.strip_suffix(".GGUF"))
        .unwrap_or(name);
    let stem = SPLIT.replace(stem, "");
    QUANT
        .captures(&stem)
        .map(|c| c[1].to_uppercase())
        .unwrap_or_else(|| stem.into_owned())
}
/// Número de parte de un archivo dividido; 0 si no está dividido.
fn part_of(file: &str) -> u32 {
    let stem = file.strip_suffix(".gguf").unwrap_or(file);
    SPLIT
        .captures(stem)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0)
}

fn walk(dir: &Path, base: &Path, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // `metadata` sigue los enlaces de la caché; un enlace roto se ignora.
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if depth > 0 {
                walk(&path, base, depth - 1, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"))
            && let Ok(rel) = path.strip_prefix(base)
        {
            out.push((rel.to_string_lossy().replace('\\', "/"), path));
        }
    }
}

/// Archivos GGUF de un repositorio de la caché, sin duplicados entre snapshots.
/// El snapshot de `refs/main` tiene prioridad.
fn repo_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let snapshots = dir.join("snapshots");
    let main = std::fs::read_to_string(dir.join("refs/main"))
        .ok()
        .map(|s| s.trim().to_owned());
    let mut revisions: Vec<PathBuf> = std::fs::read_dir(&snapshots)
        .map(|r| r.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    revisions.sort_by_key(|p| main.as_deref() != p.file_name().and_then(|n| n.to_str()));
    let mut seen = BTreeMap::new();
    for revision in revisions {
        let mut files = vec![];
        walk(&revision, &revision, 3, &mut files);
        for (rel, path) in files {
            seen.entry(rel).or_insert(path);
        }
    }
    seen.into_iter().collect()
}

pub fn list_local_models() -> Vec<LocalModel> {
    list_local_models_in(&hub_dir())
}
pub fn list_local_models_in(hub: &Path) -> Vec<LocalModel> {
    let Ok(entries) = std::fs::read_dir(hub) else {
        return vec![];
    };
    let mut models = vec![];
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(repo) = name.strip_prefix("models--") else {
            continue;
        };
        let repo = repo.replacen("--", "/", 1);
        let files = repo_files(&entry.path());
        let mmproj: Vec<String> = files
            .iter()
            .filter(|(rel, _)| is_mmproj(rel))
            .map(|(_, p)| p.to_string_lossy().into_owned())
            .collect();
        let mut quants: BTreeMap<String, Vec<(u32, PathBuf, u64, i64)>> = BTreeMap::new();
        for (rel, path) in files.into_iter().filter(|(rel, _)| is_weights(rel)) {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as i64);
            quants.entry(quant_of(&rel)).or_default().push((
                part_of(&rel),
                path,
                meta.len(),
                modified,
            ));
        }
        for (quant, mut parts) in quants {
            parts.sort_by_key(|p| p.0);
            models.push(LocalModel {
                repo: repo.clone(),
                quant,
                size_bytes: parts.iter().map(|p| p.2).sum(),
                modified: parts.iter().map(|p| p.3).max().unwrap_or(0),
                files: parts
                    .into_iter()
                    .map(|p| p.1.to_string_lossy().into_owned())
                    .collect(),
                mmproj: mmproj.clone(),
            });
        }
    }
    models.sort_by(|a, b| {
        a.repo
            .to_lowercase()
            .cmp(&b.repo.to_lowercase())
            .then_with(|| a.quant.cmp(&b.quant))
    });
    models
}

/// Borra de la caché una cuantización de `repo`: sus enlaces en todos los snapshots y los
/// blobs a los que apuntan. Si era la única del repositorio, borra la carpeta entera
/// (también el mmproj). Devuelve los bytes liberados.
pub fn delete_local_model(hub: &Path, repo: &str, quant: &str) -> std::io::Result<u64> {
    let dir = repo_dir(hub, repo);
    let others = list_local_models_in(hub)
        .iter()
        .any(|m| m.repo.eq_ignore_ascii_case(repo) && !m.quant.eq_ignore_ascii_case(quant));
    if !others {
        let freed = dir_size(&dir);
        std::fs::remove_dir_all(&dir)?;
        return Ok(freed);
    }
    let root = dir.canonicalize()?;
    let mut freed = 0;
    let revisions = std::fs::read_dir(dir.join("snapshots"))?;
    for revision in revisions.flatten().map(|e| e.path()) {
        let mut files = vec![];
        walk(&revision, &revision, 3, &mut files);
        for (rel, path) in files {
            if !is_weights(&rel) || !quant_of(&rel).eq_ignore_ascii_case(quant) {
                continue;
            }
            // El blob solo se borra si está dentro del repositorio; nunca se sigue un
            // enlace hacia fuera de la caché.
            if let Ok(blob) = path.canonicalize()
                && blob != path
                && blob.starts_with(&root)
                && let Ok(meta) = std::fs::metadata(&blob)
            {
                std::fs::remove_file(&blob)?;
                freed += meta.len();
            } else if let Ok(meta) = std::fs::symlink_metadata(&path)
                && meta.is_file()
            {
                freed += meta.len();
            }
            std::fs::remove_file(&path)?;
            // Carpetas que quedaron vacías (`UD-Q4_K_XL/`), sin salir del snapshot.
            let mut parent = path.parent();
            while let Some(d) = parent.filter(|d| *d != revision) {
                if std::fs::remove_dir(d).is_err() {
                    break;
                }
                parent = d.parent();
            }
        }
    }
    Ok(freed)
}
fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|e| {
            let meta = std::fs::symlink_metadata(e.path()).ok()?;
            Some(if meta.is_dir() {
                dir_size(&e.path())
            } else if meta.is_file() {
                meta.len()
            } else {
                0
            })
        })
        .sum()
}

/// Primer archivo de pesos de `org/nombre[:QUANT]` en la caché. Con cuantización exige
/// coincidencia exacta (`Q4_K` no toma `Q4_K_M`); sin ella toma la primera.
pub fn find_local(hub: &Path, hf: &str) -> Option<LocalModel> {
    pick_local(&list_local_models_in(hub), hf).cloned()
}
/// `find_local` sobre una lista ya leída de la caché.
pub fn pick_local<'a>(models: &'a [LocalModel], hf: &str) -> Option<&'a LocalModel> {
    let (repo, quant) = match hf.split_once(':') {
        Some((repo, quant)) => (repo, Some(quant)),
        None => (hf, None),
    };
    let mut models = models.iter().filter(|m| m.repo.eq_ignore_ascii_case(repo));
    match quant {
        Some(q) => {
            let models: Vec<_> = models.collect();
            models
                .iter()
                .find(|m| m.quant.eq_ignore_ascii_case(q))
                .or_else(|| {
                    models
                        .iter()
                        .find(|m| m.quant.to_uppercase().contains(&q.to_uppercase()))
                })
                .copied()
        }
        None => models.next(),
    }
}

/// `org/nombre[:QUANT]` que `llama-server` bajaría de Hugging Face al arrancar el script,
/// con si quiere el mmproj. `None` si usa `-m`, `--hf-file` o no indica modelo.
pub fn script_hf(script: &str) -> Option<(String, bool)> {
    let parsed = crate::script_parser::parse_script(script).ok()?;
    let has = |flags: &[&str]| parsed.argv.iter().any(|a| flags.contains(&a.as_str()));
    if !has(&["-hf", "-hfr", "--hf-repo"]) || has(&["-hff", "--hf-file"]) {
        return None;
    }
    let hf = parsed.model.filter(|m| m.contains('/'))?;
    Some((hf, !parsed.no_mmproj))
}

/// Flags que indican el modelo y que se reemplazan al elegir otro.
const MODEL_FLAGS: &[&str] = &[
    "-hf",
    "-hfr",
    "--hf-repo",
    "-hff",
    "--hf-file",
    "-m",
    "--model",
    "-mu",
    "--model-url",
];
pub const SCRIPT_TEMPLATE: &str =
    "llama-server \\\n--ctx-size 16384 \\\n--n-gpu-layers 999 \\\n--flash-attn on \\\n--port 8080";

/// Pone `-hf org/nombre:QUANT` en el script y quita cualquier otra forma de indicar el
/// modelo. Un script vacío parte de `SCRIPT_TEMPLATE`.
pub fn script_with_model(script: &str, hf: &str) -> String {
    let source = if script.trim().is_empty() {
        SCRIPT_TEMPLATE
    } else {
        script
    };
    let mut tokens = crate::script_parser::tokenize_script(source).unwrap_or_default();
    if tokens.is_empty() {
        tokens = crate::script_parser::tokenize_script(SCRIPT_TEMPLATE).unwrap_or_default();
    }
    let mut i = 1;
    while i < tokens.len() {
        if MODEL_FLAGS.contains(&tokens[i].as_str()) {
            let end = (i + 2).min(tokens.len());
            let has_value = tokens.get(i + 1).is_some_and(|v| !v.starts_with('-'));
            tokens.drain(i..if has_value { end } else { i + 1 });
        } else {
            i += 1;
        }
    }
    let at = 1.min(tokens.len());
    tokens.splice(at..at, ["-hf".to_owned(), hf.to_owned()]);
    format::format_script(&flag_writer::render_tokens(&tokens))
}

/// Nombre legible de un script: la organización de Hugging Face, el modelo y su tamaño
/// («unsloth/Qwen3.5 4B»). La cuantización y el contexto se muestran aparte, como
/// etiquetas.
pub fn script_title(script: &str) -> String {
    let parsed = crate::script_parser::parse_script(script).ok();
    let model = parsed.as_ref().and_then(|p| p.model.as_deref());
    // `org/nombre` de Hugging Face, no una ruta de `-m`.
    let org = model
        .filter(|m| !m.starts_with(['/', '.', '~']) && !m.to_lowercase().ends_with(".gguf"))
        .and_then(|m| m.split_once('/'))
        .map(|(org, _)| org);
    format::parse_model(model)
        .map(|m| match m.size {
            Some(size) => format!("{} {size}", m.base),
            None => m.base,
        })
        .map(|title| match org {
            Some(org) => format!("{org}/{title}"),
            None => title,
        })
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|p| p.model_file.as_deref())
                .map(|f| {
                    f.rsplit('/')
                        .next()
                        .unwrap_or(f)
                        .trim_end_matches(".gguf")
                        .to_owned()
                })
        })
        .unwrap_or_else(|| "Script sin modelo".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quant_names() {
        for (file, quant) in [
            ("Qwen3.5-4B-UD-Q4_K_XL.gguf", "UD-Q4_K_XL"),
            ("Agents-A1-4B-Q8_0.gguf", "Q8_0"),
            ("parakeet-tdt-0.6b-v3-Q8_0.gguf", "Q8_0"),
            ("Qwen3.5-4B-IQ4_XS.gguf", "IQ4_XS"),
            ("Qwen3.5-4B-BF16.gguf", "BF16"),
            (
                "UD-Q4_K_XL/Big-UD-Q4_K_XL-00002-of-00003.gguf",
                "UD-Q4_K_XL",
            ),
            ("gpt-oss-20b-MXFP4.gguf", "MXFP4"),
            ("weird.gguf", "weird"),
        ] {
            assert_eq!(quant_of(file), quant, "{file}");
        }
        assert_eq!(part_of("a-00002-of-00003.gguf"), 2);
    }
    #[test]
    fn scans_cache_layout() {
        let hub = tempfile::tempdir().unwrap();
        let repo = repo_dir(hub.path(), "org/Model-GGUF");
        let blobs = repo.join("blobs");
        let old = repo.join("snapshots/old");
        let main = repo.join("snapshots/abc");
        for dir in [&blobs, &old, &main.join("UD-Q4_K_XL")] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::create_dir_all(repo.join("refs")).unwrap();
        std::fs::write(repo.join("refs/main"), "abc").unwrap();
        std::fs::write(blobs.join("q8"), vec![0; 10]).unwrap();
        std::fs::write(blobs.join("p1"), vec![0; 3]).unwrap();
        std::fs::write(blobs.join("p2"), vec![0; 4]).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("../../blobs/q8", main.join("Model-Q8_0.gguf")).unwrap();
            symlink("../../blobs/q8", old.join("Model-Q8_0.gguf")).unwrap();
            symlink(
                "../../../blobs/p2",
                main.join("UD-Q4_K_XL/Model-UD-Q4_K_XL-00002-of-00002.gguf"),
            )
            .unwrap();
            symlink(
                "../../../blobs/p1",
                main.join("UD-Q4_K_XL/Model-UD-Q4_K_XL-00001-of-00002.gguf"),
            )
            .unwrap();
            symlink("../../blobs/missing", main.join("mmproj-F16.gguf")).unwrap();
        }
        std::fs::write(main.join("mmproj-BF16.gguf"), b"x").unwrap();
        let models = list_local_models_in(hub.path());
        assert_eq!(models.len(), 2);
        let split = models.iter().find(|m| m.quant == "UD-Q4_K_XL").unwrap();
        assert_eq!(split.size_bytes, 7);
        assert!(split.files[0].ends_with("00001-of-00002.gguf"));
        assert_eq!(split.mmproj.len(), 1);
        assert_eq!(models[0].repo, "org/Model-GGUF");
        let found = find_local(hub.path(), "org/model-gguf:q8_0").unwrap();
        assert_eq!(found.quant, "Q8_0");
        assert!(find_local(hub.path(), "org/Model-GGUF:Q4_K").is_some());
        assert!(find_local(hub.path(), "org/Otro:Q8_0").is_none());
        assert!(find_local(hub.path(), "org/Model-GGUF").is_some());

        // Borrar una cuantización deja la otra; borrar la última quita el repositorio.
        #[cfg(unix)]
        {
            let freed = delete_local_model(hub.path(), "org/Model-GGUF", "UD-Q4_K_XL").unwrap();
            assert_eq!(freed, 7);
            assert!(!blobs.join("p1").exists() && !blobs.join("p2").exists());
            assert!(!main.join("UD-Q4_K_XL").exists());
            let left = list_local_models_in(hub.path());
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].quant, "Q8_0");
            delete_local_model(hub.path(), "org/Model-GGUF", "Q8_0").unwrap();
            assert!(list_local_models_in(hub.path()).is_empty());
            assert!(!repo_dir(hub.path(), "org/Model-GGUF").exists());
        }
    }
    #[test]
    fn detects_hub_models_in_scripts() {
        assert_eq!(
            script_hf("llama-server -hf org/M-GGUF:Q8_0 --no-mmproj"),
            Some(("org/M-GGUF:Q8_0".into(), false))
        );
        assert_eq!(
            script_hf("llama-server --hf-repo org/M-GGUF"),
            Some(("org/M-GGUF".into(), true))
        );
        assert_eq!(script_hf("llama-server -m /m/a.gguf"), None);
        assert_eq!(script_hf("llama-server -hf org/M -hff a.gguf"), None);
    }
    #[test]
    fn replaces_model_in_script() {
        let script = "llama-server \\\n-m /tmp/a.gguf \\\n--ctx-size 4096 \\\n--port 8081";
        let out = script_with_model(script, "unsloth/Qwen3.5-4B-GGUF:UD-Q4_K_XL");
        assert_eq!(
            out,
            "llama-server \\\n-hf unsloth/Qwen3.5-4B-GGUF:UD-Q4_K_XL \\\n--ctx-size 4096 \\\n--port 8081"
        );
        let fresh = script_with_model("", "a/b:Q8_0");
        assert!(fresh.starts_with("llama-server \\\n-hf a/b:Q8_0 \\\n--ctx-size 16384"));
        let twice = script_with_model(&fresh, "c/d:Q4_0");
        assert!(twice.contains("-hf c/d:Q4_0") && !twice.contains("a/b"));
    }
    #[test]
    fn titles() {
        assert_eq!(
            script_title("llama-server -hf unsloth/Qwen3.5-4B-GGUF:UD-Q4_K_XL --ctx-size 32768"),
            "unsloth/Qwen3.5 4B"
        );
        assert_eq!(
            script_title("llama-server -m /modelos/Qwen3.5-4B-Q8_0.gguf"),
            "Qwen3.5 4B"
        );
        assert_eq!(script_title("llama-server"), "Script sin modelo");
    }
}
