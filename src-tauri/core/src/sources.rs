//! Remembers which CustomsForge song a downloaded file came from. The
//! download button on CustomsForge opens `…/cdlc/<id>/download?…` before the
//! file host (Google Drive, Mega …) delivers the actual file, so the id is
//! noted as "pending" and attached to the files the next finished download
//! installs. The mapping lives in `cdlc-sources.json` in the app data folder,
//! keyed by file name, so it survives enabling and disabling a song.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub const CUSTOMSFORGE_HOST: &str = "ignition4.customsforge.com";
const FILE_NAME: &str = "cdlc-sources.json";
/// A download that finishes later than this after the click on CustomsForge
/// is not attributed to that song any more.
pub const PENDING_WINDOW_SECS: u64 = 10 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongSource {
    /// CustomsForge's id of the song, as in `https://ignition4.customsforge.com/cdlc/<id>`.
    pub cf_id: String,
    /// Unix seconds.
    pub downloaded_at: u64,
    /// Host that delivered the file, e.g. `drive.usercontent.google.com`.
    #[serde(default)]
    pub source_url_host: Option<String>,
}

/// A download started on CustomsForge whose file has not arrived yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDownload {
    pub cf_id: String,
    pub requested_at: u64,
}

/// The CustomsForge song id in a download address such as
/// `https://ignition4.customsforge.com/cdlc/123/download?expires=…`.
pub fn download_cf_id(url: &reqwest::Url) -> Option<String> {
    if url.scheme() != "https" || url.host_str() != Some(CUSTOMSFORGE_HOST) {
        return None;
    }
    let mut segments = url.path_segments()?;
    match (segments.next(), segments.next(), segments.next(), segments.next()) {
        (Some("cdlc"), Some(id), Some("download"), None) if valid_id(id) => Some(id.to_string()),
        _ => None,
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The song page on CustomsForge for a stored id.
pub fn song_page_url(cf_id: &str) -> Option<String> {
    valid_id(cf_id).then(|| format!("https://{CUSTOMSFORGE_HOST}/cdlc/{cf_id}"))
}

/// Only pages of CustomsForge's song site may be opened in the embedded view
/// on request of the UI.
pub fn is_customsforge_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some(CUSTOMSFORGE_HOST) && url.port().is_none()
}

/// The pending id if the download finished within the time window.
pub fn match_pending(pending: Option<&PendingDownload>, now: u64) -> Option<&str> {
    let p = pending?;
    (now >= p.requested_at && now - p.requested_at <= PENDING_WINDOW_SECS).then_some(p.cf_id.as_str())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sources(pub BTreeMap<String, SongSource>);

impl Sources {
    pub fn load(data_dir: &Path) -> Self {
        fs::read_to_string(data_dir.join(FILE_NAME))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Writes through a temporary file so a crash never leaves half a file behind.
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir)?;
        let tmp = data_dir.join(format!("{FILE_NAME}.tmp"));
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, data_dir.join(FILE_NAME))?;
        Ok(())
    }

    pub fn record(&mut self, file_names: &[String], cf_id: &str, now: u64, host: Option<&str>) {
        for name in file_names {
            self.0.insert(
                name.clone(),
                SongSource { cf_id: cf_id.to_string(), downloaded_at: now, source_url_host: host.map(str::to_string) },
            );
        }
    }

    /// Returns whether anything was removed.
    pub fn remove(&mut self, file_names: &[String]) -> bool {
        let before = self.0.len();
        for name in file_names {
            self.0.remove(name);
        }
        self.0.len() != before
    }
}

/// Stores the origin of freshly installed files if a CustomsForge download
/// is pending; returns whether a mapping was written.
pub fn record_download(
    data_dir: &Path,
    imported: &[String],
    pending: Option<&PendingDownload>,
    now: u64,
    host: Option<&str>,
) -> Result<bool> {
    let Some(cf_id) = match_pending(pending, now) else { return Ok(false) };
    if imported.is_empty() {
        return Ok(false);
    }
    let mut sources = Sources::load(data_dir);
    sources.record(imported, cf_id, now, host);
    sources.save(data_dir)?;
    Ok(true)
}

/// Forgets deleted songs.
pub fn forget(data_dir: &Path, file_names: &[String]) -> Result<()> {
    let mut sources = Sources::load(data_dir);
    if sources.remove(file_names) {
        sources.save(data_dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> reqwest::Url {
        s.parse().unwrap()
    }

    #[test]
    fn finds_id_in_download_links() {
        assert_eq!(
            download_cf_id(&url(
                "https://ignition4.customsforge.com/cdlc/12345/download?expires=1&platform=pc&signature=abc"
            ))
            .as_deref(),
            Some("12345")
        );
        assert_eq!(download_cf_id(&url("https://ignition4.customsforge.com/cdlc/12345")), None);
        assert_eq!(download_cf_id(&url("https://evil.example/cdlc/1/download")), None);
        assert_eq!(download_cf_id(&url("http://ignition4.customsforge.com/cdlc/1/download")), None);
        assert_eq!(download_cf_id(&url("https://ignition4.customsforge.com/cdlc/a%22b/download")), None);
    }

    #[test]
    fn only_customsforge_pages_are_allowed() {
        assert!(is_customsforge_url(&url("https://ignition4.customsforge.com/cdlc/1")));
        assert!(!is_customsforge_url(&url("https://ignition4.customsforge.com.evil.example/")));
        assert!(!is_customsforge_url(&url("http://ignition4.customsforge.com/")));
        assert!(!is_customsforge_url(&url("https://customsforge.com/")));
        assert_eq!(song_page_url("42").as_deref(), Some("https://ignition4.customsforge.com/cdlc/42"));
        assert_eq!(song_page_url("../x"), None);
    }

    #[test]
    fn pending_download_expires() {
        let p = PendingDownload { cf_id: "7".into(), requested_at: 1000 };
        assert_eq!(match_pending(Some(&p), 1000), Some("7"));
        assert_eq!(match_pending(Some(&p), 1000 + PENDING_WINDOW_SECS), Some("7"));
        assert_eq!(match_pending(Some(&p), 1001 + PENDING_WINDOW_SECS), None);
        assert_eq!(match_pending(Some(&p), 999), None);
        assert_eq!(match_pending(None, 1000), None);
    }

    #[test]
    fn records_and_forgets() {
        let dir = std::env::temp_dir().join(format!("guitarstar-test-sources-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let names = vec!["A_B_v1_p.psarc".to_string(), "C_D_p.psarc".to_string()];
        let p = PendingDownload { cf_id: "99".into(), requested_at: 100 };
        // Too late: nothing is stored.
        assert!(!record_download(&dir, &names, Some(&p), 100 + PENDING_WINDOW_SECS + 1, None).unwrap());
        assert!(Sources::load(&dir).0.is_empty());

        assert!(record_download(&dir, &names, Some(&p), 160, Some("drive.usercontent.google.com")).unwrap());
        let s = Sources::load(&dir);
        assert_eq!(s.0.len(), 2);
        assert_eq!(
            s.0["A_B_v1_p.psarc"],
            SongSource { cf_id: "99".into(), downloaded_at: 160, source_url_host: Some("drive.usercontent.google.com".into()) }
        );
        assert!(!dir.join("cdlc-sources.json.tmp").exists());

        forget(&dir, &["A_B_v1_p.psarc".to_string()]).unwrap();
        assert_eq!(Sources::load(&dir).0.keys().collect::<Vec<_>>(), vec!["C_D_p.psarc"]);
        fs::remove_dir_all(&dir).unwrap();
    }
}
