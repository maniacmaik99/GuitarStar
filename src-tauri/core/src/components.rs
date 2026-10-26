//! Third-party components installed into the game folder. Each one comes from
//! its official GitHub releases; every downloaded version stays cached so the
//! user can switch back to it without a network connection.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

pub enum Source {
    /// Release archives from a GitHub repository.
    GitHub { repo: &'static str, asset_suffix: &'static str },
    /// Shipped inside GuitarStar; `files` pairs each file name with its bytes.
    Bundled {
        version: &'static str,
        files: &'static [(&'static str, &'static [u8])],
    },
}

pub struct ComponentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: Source,
    /// Files copied into the game folder.
    pub files: &'static [&'static str],
    /// Extra files kept from a release archive but not installed directly.
    pub extras: &'static [&'static str],
}

impl ComponentDef {
    pub fn repo(&self) -> Option<&'static str> {
        match self.source {
            Source::GitHub { repo, .. } => Some(repo),
            Source::Bundled { .. } => None,
        }
    }
}

pub const DPI_FIX_ID: &str = "dpi_fix";

pub const COMPONENTS: &[ComponentDef] = &[
    ComponentDef {
        id: "rs_asio",
        name: "RS_ASIO",
        description: "Leitet Ein- und Ausgabe des Spiels auf ein Audio-Interface (ASIO) um: niedrige Latenz, kein Rocksmith-Kabel nÃ¶tig.",
        source: Source::GitHub { repo: "mdias/rs_asio", asset_suffix: ".zip" },
        files: &["avrt.dll", "RS_ASIO.dll"],
        extras: &["RS_ASIO.ini"],
    },
    ComponentDef {
        id: DPI_FIX_ID,
        name: "Skalierungs-Fix",
        description: "Eigene dinput8.dll von GuitarStar: Rocksmith Ã¼bernimmt die Windows-Skalierung selbst, damit das Bild auch im (exklusiven) Vollbild genau auf den Monitor passt.",
        source: Source::Bundled {
            version: "1.2.0",
            files: &[("dinput8.dll", include_bytes!("../assets/dinput8.dll"))],
        },
        files: &["dinput8.dll"],
        extras: &[],
    },
];

pub fn find(id: &str) -> Result<&'static ComponentDef> {
    COMPONENTS
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::msg(format!("Unbekannte Komponente: {id}")))
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub published: Option<String>,
    pub page_url: String,
    pub asset_name: String,
    pub asset_url: String,
    pub asset_size: u64,
    /// GitHub's asset digest, e.g. `sha256:â€¦`, when GitHub provides one.
    pub digest: Option<String>,
    pub prerelease: bool,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    prerelease: bool,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSource {
    pub version: String,
    pub asset_url: String,
    pub sha256: String,
    pub downloaded_at: u64,
}

const USER_AGENT: &str = concat!("GuitarStar/", env!("CARGO_PKG_VERSION"));

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("HTTP client configuration is static")
}

/// Newest first, drafts and releases without a matching asset skipped.
pub async fn releases(http: &reqwest::Client, def: &ComponentDef) -> Result<Vec<ReleaseInfo>> {
    let (repo, asset_suffix) = match def.source {
        Source::GitHub { repo, asset_suffix } => (repo, asset_suffix),
        Source::Bundled { version, .. } => {
            return Ok(vec![ReleaseInfo {
                version: version.to_string(),
                published: None,
                page_url: String::new(),
                asset_name: String::new(),
                asset_url: String::new(),
                asset_size: 0,
                digest: None,
                prerelease: false,
            }]);
        }
    };
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
    let list: Vec<GhRelease> = http
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(list
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let asset = r.assets.into_iter().find(|a| a.name.to_lowercase().ends_with(asset_suffix))?;
            Some(ReleaseInfo {
                version: r.tag_name,
                published: r.published_at,
                page_url: r.html_url,
                asset_name: asset.name,
                asset_url: asset.browser_download_url,
                asset_size: asset.size,
                digest: asset.digest,
                prerelease: r.prerelease,
            })
        })
        .collect())
}

fn safe_name(version: &str) -> String {
    version
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

pub fn cache_dir(data_dir: &Path, id: &str, version: &str) -> PathBuf {
    data_dir.join("components").join(id).join(safe_name(version))
}

pub fn is_cached(data_dir: &Path, id: &str, version: &str) -> bool {
    cache_dir(data_dir, id, version).join("source.json").is_file()
}

/// Cached versions, newest download first.
pub fn cached_versions(data_dir: &Path, id: &str) -> Vec<CachedSource> {
    let Ok(entries) = fs::read_dir(data_dir.join("components").join(id)) else {
        return Vec::new();
    };
    let mut out: Vec<CachedSource> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| fs::read_to_string(e.path().join("source.json")).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect();
    out.sort_by(|a, b| b.downloaded_at.cmp(&a.downloaded_at));
    out
}

/// Puts a bundled component into the version cache, so installing it works
/// exactly like installing a downloaded one.
pub fn materialize_bundled(data_dir: &Path, def: &ComponentDef) -> Result<Option<PathBuf>> {
    let Source::Bundled { version, files } = def.source else {
        return Ok(None);
    };
    let dir = cache_dir(data_dir, def.id, version);
    fs::create_dir_all(&dir)?;
    let mut hasher = Sha256::new();
    for (name, bytes) in files {
        fs::write(dir.join(name), bytes)?;
        hasher.update(bytes);
    }
    let source = CachedSource {
        version: version.to_string(),
        asset_url: String::new(),
        sha256: hex::encode(hasher.finalize()),
        downloaded_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
    };
    fs::write(dir.join("source.json"), serde_json::to_string_pretty(&source)?)?;
    Ok(Some(dir))
}

/// True when `path` holds exactly the bundled file `name` of `def`.
pub fn is_bundled_file(def: &ComponentDef, name: &str, path: &Path) -> bool {
    let Source::Bundled { files, .. } = def.source else {
        return false;
    };
    files
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .is_some_and(|(_, bytes)| fs::read(path).is_ok_and(|b| b == *bytes))
}

/// Downloads a release, verifies it against GitHub's digest and unpacks the
/// component's files into the version cache.
pub async fn download(
    http: &reqwest::Client,
    data_dir: &Path,
    def: &ComponentDef,
    rel: &ReleaseInfo,
) -> Result<PathBuf> {
    let bytes = http.get(&rel.asset_url).send().await?.error_for_status()?.bytes().await?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    if let Some(expected) = rel.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        if !expected.eq_ignore_ascii_case(&sha256) {
            return Err(AppError::msg(format!(
                "Die PrÃ¼fsumme von {} stimmt nicht mit GitHub Ã¼berein. Download abgebrochen.",
                rel.asset_name
            )));
        }
    }

    let final_dir = cache_dir(data_dir, def.id, &rel.version);
    let tmp_dir = final_dir.with_extension("partial");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir)?;

    let wanted: Vec<&str> = def.files.iter().chain(def.extras).copied().collect();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())) else {
            continue;
        };
        if let Some(target) = wanted.iter().find(|w| w.eq_ignore_ascii_case(&name)) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            fs::write(tmp_dir.join(target), buf)?;
        }
    }
    if let Some(missing) = def.files.iter().find(|f| !tmp_dir.join(f).is_file()) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(AppError::msg(format!(
            "{} {} enthÃ¤lt die Datei {missing} nicht. Diese Version wird nicht unterstÃ¼tzt.",
            def.name, rel.version
        )));
    }

    let source = CachedSource {
        version: rel.version.clone(),
        asset_url: rel.asset_url.clone(),
        sha256,
        downloaded_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
    };
    fs::write(tmp_dir.join("source.json"), serde_json::to_string_pretty(&source)?)?;
    let _ = fs::remove_dir_all(&final_dir);
    fs::rename(&tmp_dir, &final_dir)?;
    Ok(final_dir)
}
