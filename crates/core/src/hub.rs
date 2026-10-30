// Búsqueda de modelos GGUF en Hugging Face y descarga a la caché estándar (`hub/`), la
// misma que usa `llama-server -hf`, para que los modelos queden listos sin conexión.
use crate::{CoreError, Result, models};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

pub const HUB_URL: &str = "https://huggingface.co";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HubSort {
    #[default]
    Recent,
    Downloads,
    Likes,
    Trending,
}
impl HubSort {
    fn param(self) -> &'static str {
        match self {
            Self::Recent => "lastModified",
            Self::Downloads => "downloads",
            Self::Likes => "likes",
            Self::Trending => "trendingScore",
        }
    }
}

/// Resultado de búsqueda: lo justo para la lista, sin descripción.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubModel {
    pub id: String,
    pub likes: u64,
    pub downloads: u64,
    pub last_modified: Option<String>,
    #[serde(rename = "pipeline_tag")]
    pub pipeline_tag: Option<String>,
    pub tags: Vec<String>,
}
impl HubModel {
    pub fn author(&self) -> &str {
        self.id.split_once('/').map_or("", |(a, _)| a)
    }
    pub fn name(&self) -> &str {
        self.id.split_once('/').map_or(&self.id, |(_, n)| n)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubFile {
    pub path: String,
    pub size: u64,
    /// sha256 del archivo LFS; es el nombre del blob en la caché.
    pub sha256: Option<String>,
    pub blob_id: Option<String>,
}
impl HubFile {
    fn blob(&self) -> Option<&str> {
        self.sha256.as_deref().or(self.blob_id.as_deref())
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubQuant {
    pub quant: String,
    pub files: Vec<HubFile>,
    pub size: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubDetails {
    pub id: String,
    /// Commit de `main`: carpeta del snapshot en la caché.
    pub sha: String,
    pub base_models: Vec<String>,
    pub pipeline_tag: Option<String>,
    pub gated: bool,
    pub likes: u64,
    pub downloads: u64,
    pub last_modified: Option<String>,
    /// Cuantizaciones ordenadas de menor a mayor tamaño.
    pub quants: Vec<HubQuant>,
    pub mmproj: Vec<HubFile>,
}

/// Token de Hugging Face para repos con acceso restringido: `HF_TOKEN` o el archivo `token`
/// que deja `hf auth login`. Solo se envía a huggingface.co.
fn token() -> Option<String> {
    if let Ok(t) = std::env::var("HF_TOKEN")
        && !t.trim().is_empty()
    {
        return Some(t.trim().to_owned());
    }
    let home = std::env::var_os("HF_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cache/huggingface")))?;
    std::fs::read_to_string(home.join("token"))
        .ok()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("plane-llama/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default()
}
fn get(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    let req = client.get(url);
    match token() {
        Some(t) => req.bearer_auth(t),
        None => req,
    }
}

/// `org/texto` busca `texto` dentro de `org`; sin texto lista lo último de `author`.
pub fn search_url(query: &str, author: Option<&str>, sort: HubSort, limit: usize) -> String {
    let query = query.trim();
    let (author, text) = match query.split_once('/') {
        Some((a, t)) if !a.is_empty() && !a.contains(' ') => (Some(a), t),
        _ => (author.filter(|_| query.is_empty()), query),
    };
    let mut url = format!(
        "{HUB_URL}/api/models?filter=gguf&sort={}&direction=-1&limit={limit}",
        sort.param()
    );
    if let Some(a) = author {
        url.push_str("&author=");
        url.push_str(&encode(a));
    }
    if !text.is_empty() {
        url.push_str("&search=");
        url.push_str(&encode(text));
    }
    url
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub async fn search(
    client: &reqwest::Client,
    query: &str,
    author: Option<&str>,
    sort: HubSort,
) -> Result<Vec<HubModel>> {
    let res = get(client, &search_url(query, author, sort, 40))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?
        .error_for_status()?;
    Ok(res.json().await?)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawSibling {
    rfilename: String,
    size: Option<u64>,
    #[serde(rename = "blobId")]
    blob_id: Option<String>,
    lfs: Option<RawLfs>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawLfs {
    sha256: Option<String>,
    size: Option<u64>,
}
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawDetails {
    id: String,
    sha: String,
    #[serde(rename = "pipeline_tag")]
    pipeline_tag: Option<String>,
    gated: serde_json::Value,
    likes: u64,
    downloads: u64,
    last_modified: Option<String>,
    tags: Vec<String>,
    card_data: serde_json::Value,
    siblings: Vec<RawSibling>,
}

/// Agrupa la respuesta de `/api/models/{id}?blobs=true` por cuantización.
pub fn parse_details(raw: &str) -> Result<HubDetails> {
    let d: RawDetails = serde_json::from_str(raw)?;
    let mut base_models: Vec<String> = match &d.card_data["base_model"] {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(v) => v
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => vec![],
    };
    if base_models.is_empty() {
        base_models = d
            .tags
            .iter()
            .filter_map(|t| t.strip_prefix("base_model:"))
            .filter(|t| !t.contains(':'))
            .map(str::to_owned)
            .collect();
    }
    let mut quants: Vec<HubQuant> = vec![];
    let mut mmproj = vec![];
    for s in d.siblings {
        if !models::is_weights(&s.rfilename) && !models::is_mmproj(&s.rfilename) {
            continue;
        }
        let file = HubFile {
            size: s.lfs.as_ref().and_then(|l| l.size).or(s.size).unwrap_or(0),
            sha256: s.lfs.and_then(|l| l.sha256),
            blob_id: s.blob_id,
            path: s.rfilename,
        };
        if models::is_mmproj(&file.path) {
            mmproj.push(file);
            continue;
        }
        let quant = models::quant_of(&file.path);
        match quants.iter_mut().find(|q| q.quant == quant) {
            Some(q) => {
                q.size += file.size;
                q.files.push(file);
            }
            None => quants.push(HubQuant {
                quant,
                size: file.size,
                files: vec![file],
            }),
        }
    }
    for q in &mut quants {
        q.files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    quants.sort_by_key(|q| q.size);
    mmproj.sort_by_key(|f| f.size);
    Ok(HubDetails {
        id: d.id,
        sha: d.sha,
        base_models,
        pipeline_tag: d.pipeline_tag,
        gated: !matches!(
            d.gated,
            serde_json::Value::Bool(false) | serde_json::Value::Null
        ),
        likes: d.likes,
        downloads: d.downloads,
        last_modified: d.last_modified,
        quants,
        mmproj,
    })
}

pub async fn details(client: &reqwest::Client, id: &str) -> Result<HubDetails> {
    let res = get(client, &format!("{HUB_URL}/api/models/{id}?blobs=true"))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?
        .error_for_status()?;
    parse_details(&res.text().await?)
}

/// Qué descargar: archivos de un repo en un commit concreto.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPlan {
    pub repo: String,
    pub revision: String,
    pub files: Vec<HubFile>,
}
impl DownloadPlan {
    pub fn total(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

/// Lo que baja `llama-server -hf repo[:QUANT]`: la cuantización pedida (o `Q4_K_M`, o la
/// primera si no hay etiqueta) y el mmproj más pequeño si `mmproj`.
pub fn plan_for(details: &HubDetails, tag: Option<&str>, mmproj: bool) -> Option<DownloadPlan> {
    let find = |q: &str| {
        let q = q.to_uppercase();
        details
            .quants
            .iter()
            .find(|x| x.quant.to_uppercase() == q)
            .or_else(|| {
                details
                    .quants
                    .iter()
                    .find(|x| x.quant.to_uppercase().contains(&q))
            })
    };
    let quant = match tag {
        Some(t) => find(t)?,
        None => find("Q4_K_M").or(details.quants.first())?,
    };
    let mut files = quant.files.clone();
    if mmproj && let Some(m) = details.mmproj.first() {
        files.push(m.clone());
    }
    Some(DownloadPlan {
        repo: details.id.clone(),
        revision: details.sha.clone(),
        files,
    })
}

/// Enlace relativo del snapshot al blob, como lo crea `huggingface_hub`.
fn blob_link(path: &str, blob: &str) -> PathBuf {
    let depth = path.split('/').count() + 1;
    let mut link = PathBuf::new();
    for _ in 0..depth {
        link.push("..");
    }
    link.join("blobs").join(blob)
}

/// Descarga los archivos que falten, retomando los `.incomplete` con `Range`. Llama a
/// `progress(hecho, total)` tras cada bloque; quien llama decide cada cuánto repintar.
/// Devuelve la ruta del primer archivo en el snapshot.
pub async fn download(
    client: &reqwest::Client,
    hub: &Path,
    plan: &DownloadPlan,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, u64),
) -> Result<PathBuf> {
    download_from(HUB_URL, client, hub, plan, cancel, progress).await
}
/// `download` contra otro servidor con las mismas rutas `/{repo}/resolve/{rev}/{archivo}`.
pub async fn download_from(
    base: &str,
    client: &reqwest::Client,
    hub: &Path,
    plan: &DownloadPlan,
    cancel: &CancellationToken,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf> {
    if plan.files.is_empty() || plan.revision.is_empty() {
        return Err(CoreError::Invalid("No hay archivos para descargar.".into()));
    }
    let repo = models::repo_dir(hub, &plan.repo);
    let blobs = repo.join("blobs");
    let snapshot = repo.join("snapshots").join(&plan.revision);
    tokio::fs::create_dir_all(&blobs).await?;
    let total = plan.total();
    let mut done = 0;
    for file in &plan.files {
        let blob_name = file
            .blob()
            .ok_or_else(|| CoreError::Invalid(format!("{} no tiene identificador.", file.path)))?;
        let blob = blobs.join(blob_name);
        let complete = tokio::fs::metadata(&blob)
            .await
            .is_ok_and(|m| m.len() == file.size);
        if !complete {
            let partial = blobs.join(format!("{blob_name}.incomplete"));
            let mut have = tokio::fs::metadata(&partial).await.map_or(0, |m| m.len());
            if have > file.size {
                have = 0;
            }
            let url = format!(
                "{base}/{}/resolve/{}/{}",
                plan.repo, plan.revision, file.path
            );
            let mut req = get(client, &url);
            if have > 0 {
                req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
            }
            let mut res = tokio::select! {
                r = req.send() => r?,
                _ = cancel.cancelled() => return Err(CoreError::Cancelled),
            };
            if res.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                have = 0;
                res = get(client, &url).send().await?;
            }
            let res = res.error_for_status()?;
            let resumed = res.status() == reqwest::StatusCode::PARTIAL_CONTENT;
            let mut out = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(resumed)
                .truncate(!resumed)
                .open(&partial)
                .await?;
            let mut written = if resumed { have } else { 0 };
            progress(done + written, total);
            let mut res = res;
            loop {
                let chunk = tokio::select! {
                    c = res.chunk() => c?,
                    _ = cancel.cancelled() => {
                        out.flush().await?;
                        return Err(CoreError::Cancelled);
                    }
                };
                let Some(chunk) = chunk else { break };
                out.write_all(&chunk).await?;
                written += chunk.len() as u64;
                progress(done + written, total);
            }
            out.flush().await?;
            drop(out);
            if written != file.size {
                return Err(CoreError::Invalid(format!(
                    "{}: se recibieron {written} de {} bytes; vuelve a intentarlo para retomar.",
                    file.path, file.size
                )));
            }
            tokio::fs::rename(&partial, &blob).await?;
        }
        done += file.size;
        progress(done, total);
        let link = snapshot.join(&file.path);
        if let Some(parent) = link.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let _ = tokio::fs::remove_file(&link).await;
        #[cfg(unix)]
        tokio::fs::symlink(blob_link(&file.path, blob_name), &link).await?;
        #[cfg(not(unix))]
        tokio::fs::copy(&blob, &link).await?;
    }
    tokio::fs::create_dir_all(repo.join("refs")).await?;
    tokio::fs::write(repo.join("refs/main"), &plan.revision).await?;
    Ok(snapshot.join(&plan.files[0].path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_search_urls() {
        assert_eq!(
            search_url("", Some("unsloth"), HubSort::Recent, 40),
            "https://huggingface.co/api/models?filter=gguf&sort=lastModified&direction=-1&limit=40&author=unsloth"
        );
        assert!(
            search_url("qwen 3", Some("unsloth"), HubSort::Downloads, 10)
                .ends_with("sort=downloads&direction=-1&limit=10&search=qwen%203")
        );
        assert!(
            search_url("bartowski/llama", None, HubSort::Likes, 5)
                .ends_with("&author=bartowski&search=llama")
        );
    }
    #[test]
    fn groups_quants_and_base_model() {
        let raw = r#"{"id":"unsloth/X-GGUF","sha":"abc","gated":false,"likes":3,"downloads":9,
            "pipeline_tag":"text-generation",
            "tags":["gguf","base_model:quantized:Org/X","base_model:Org/X"],
            "cardData":{"base_model":"Org/X"},
            "siblings":[
              {"rfilename":"README.md","size":10},
              {"rfilename":"imatrix_unsloth.gguf","size":10},
              {"rfilename":"X-Q8_0.gguf","size":800,"lfs":{"sha256":"s8","size":800}},
              {"rfilename":"BF16/X-BF16-00002-of-00002.gguf","lfs":{"sha256":"b2","size":600}},
              {"rfilename":"BF16/X-BF16-00001-of-00002.gguf","lfs":{"sha256":"b1","size":1000}},
              {"rfilename":"X-UD-Q4_K_XL.gguf","lfs":{"sha256":"s4","size":400}},
              {"rfilename":"mmproj-F16.gguf","lfs":{"sha256":"m","size":50}}]}"#;
        let d = parse_details(raw).unwrap();
        assert_eq!(d.base_models, vec!["Org/X"]);
        assert_eq!(d.pipeline_tag.as_deref(), Some("text-generation"));
        assert!(!d.gated);
        let names: Vec<_> = d.quants.iter().map(|q| q.quant.as_str()).collect();
        assert_eq!(names, ["UD-Q4_K_XL", "Q8_0", "BF16"]);
        assert_eq!(d.quants[2].size, 1600);
        assert!(d.quants[2].files[0].path.contains("00001"));
        assert_eq!(d.mmproj.len(), 1);
        let gated = parse_details(r#"{"id":"a/b","gated":"manual"}"#).unwrap();
        assert!(gated.gated);
    }
    #[test]
    fn relative_links() {
        assert_eq!(blob_link("a.gguf", "x"), PathBuf::from("../../blobs/x"));
        assert_eq!(
            blob_link("d/a.gguf", "x"),
            PathBuf::from("../../../blobs/x")
        );
    }

    /// Servidor HTTP mínimo que entrega `body` y respeta `Range: bytes=N-`.
    async fn serve(body: Vec<u8>) -> String {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    return;
                };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0; 4096];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                    let from = req
                        .split("range: bytes=")
                        .nth(1)
                        .and_then(|r| r.split('-').next())
                        .and_then(|n| n.trim().parse::<usize>().ok());
                    let (status, part) = match from {
                        Some(f) => ("206 Partial Content", &body[f..]),
                        None => ("200 OK", &body[..]),
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        part.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(part).await;
                });
            }
        });
        format!("http://{addr}")
    }
    #[test]
    fn plans_like_llama_server() {
        let file = |path: &str, size| HubFile {
            path: path.into(),
            size,
            ..Default::default()
        };
        let quant = |q: &str, size| HubQuant {
            quant: q.into(),
            files: vec![file(&format!("M-{q}.gguf"), size)],
            size,
        };
        let d = HubDetails {
            id: "org/M".into(),
            sha: "rev".into(),
            quants: vec![quant("Q4_K_S", 1), quant("Q4_K_M", 2), quant("Q8_0", 3)],
            mmproj: vec![file("mmproj-F16.gguf", 5)],
            ..Default::default()
        };
        let paths = |p: DownloadPlan| p.files.into_iter().map(|f| f.path).collect::<Vec<_>>();
        assert_eq!(
            paths(plan_for(&d, Some("q8_0"), true).unwrap()),
            ["M-Q8_0.gguf", "mmproj-F16.gguf"]
        );
        assert_eq!(paths(plan_for(&d, None, false).unwrap()), ["M-Q4_K_M.gguf"]);
        assert!(plan_for(&d, Some("Q2_K"), false).is_none());
    }
    #[tokio::test]
    async fn downloads_and_resumes_into_cache() {
        let body: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        let base = serve(body.clone()).await;
        let hub = tempfile::tempdir().unwrap();
        let plan = DownloadPlan {
            repo: "org/M-GGUF".into(),
            revision: "rev1".into(),
            files: vec![HubFile {
                path: "Q8_0/M-Q8_0.gguf".into(),
                size: body.len() as u64,
                sha256: Some("sha".into()),
                blob_id: None,
            }],
        };
        let blobs = models::repo_dir(hub.path(), "org/M-GGUF").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(blobs.join("sha.incomplete"), &body[..1200]).unwrap();
        let mut last = (0, 0);
        let path = download_from(
            &base,
            &client(),
            hub.path(),
            &plan,
            &CancellationToken::new(),
            |d, t| last = (d, t),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(last, (5000, 5000));
        let local = models::list_local_models_in(hub.path());
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].hf_ref(), "org/M-GGUF:Q8_0");
        let cancel = CancellationToken::new();
        cancel.cancel();
        // Ya completo: no vuelve a pedir nada y no se cancela.
        assert!(
            download_from(&base, &client(), hub.path(), &plan, &cancel, |_, _| {})
                .await
                .is_ok()
        );
    }

    /// Contra la caché real y huggingface.co: `cargo test -p plane-llama-core live -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn live_cache_and_hub() {
        for m in models::list_local_models() {
            println!(
                "{} {} MiB {:?}",
                m.hf_ref(),
                m.size_bytes >> 20,
                m.mmproj.len()
            );
        }
        let c = client();
        let list = search(&c, "", Some("unsloth"), HubSort::Recent)
            .await
            .unwrap();
        assert!(!list.is_empty());
        let d = details(&c, "unsloth/Qwen3.5-4B-GGUF").await.unwrap();
        println!("{:?} {}", d.base_models, d.quants.len());
        assert!(d.quants.iter().any(|q| q.quant == "UD-Q4_K_XL"));
    }
}
