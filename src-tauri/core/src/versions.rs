//! Newer versions of installed custom songs on CustomsForge.
//!
//! The check runs inside the embedded CustomsForge view with the user's own
//! login (see `src-tauri/src/updatecheck.js`); this module only keeps the
//! results and enforces the limits: one automatic check per day (at app
//! start), at most three manual checks per day, at most `MAX_PER_RUN` songs
//! per run. Only songs whose CustomsForge id is known (`sources.rs`) are
//! checked.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

const FILE: &str = "cdlc-versions.json";
pub const MANUAL_PER_DAY: u32 = 3;
pub const MAX_PER_RUN: usize = 50;
const DAY: u64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub checked_at: u64,
    /// Version shown on the song page, without a leading "v", e.g. "2" or "1.3".
    pub version: Option<String>,
    /// "Last revised" date as shown on the page (MM/DD/YYYY), if any.
    pub revised: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionStore {
    #[serde(default)]
    pub songs: BTreeMap<String, VersionInfo>,
    #[serde(default)]
    pub last_auto_day: Option<u64>,
    #[serde(default)]
    pub manual_day: Option<u64>,
    #[serde(default)]
    pub manual_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The automatic check already ran today.
    AutoDoneToday,
    /// The manual limit for today is used up.
    ManualLimit,
}

fn day(ts: u64) -> u64 {
    ts / DAY
}

impl VersionStore {
    pub fn load(data_dir: &Path) -> Self {
        fs::read_to_string(data_dir.join(FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir)?;
        let tmp = data_dir.join(format!("{FILE}.tmp"));
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, data_dir.join(FILE))?;
        Ok(())
    }

    pub fn manual_left(&self, now: u64) -> u32 {
        if self.manual_day == Some(day(now)) {
            MANUAL_PER_DAY.saturating_sub(self.manual_count)
        } else {
            MANUAL_PER_DAY
        }
    }

    /// Counts a run against today's limits, or says why it may not run.
    pub fn start_run(&mut self, manual: bool, now: u64) -> std::result::Result<(), Refusal> {
        let today = day(now);
        if manual {
            if self.manual_left(now) == 0 {
                return Err(Refusal::ManualLimit);
            }
            if self.manual_day != Some(today) {
                self.manual_day = Some(today);
                self.manual_count = 0;
            }
            self.manual_count += 1;
        } else {
            if self.last_auto_day == Some(today) {
                return Err(Refusal::AutoDoneToday);
            }
            self.last_auto_day = Some(today);
        }
        Ok(())
    }

    /// Which of `ids` to query in this run. The automatic run skips songs
    /// already checked today; a manual run re-checks them.
    pub fn select<'a>(&self, ids: impl IntoIterator<Item = &'a str>, manual: bool, now: u64) -> Vec<String> {
        let today = day(now);
        let mut out: Vec<String> = ids
            .into_iter()
            .filter(|id| manual || self.songs.get(*id).is_none_or(|v| day(v.checked_at) != today))
            .map(String::from)
            .collect();
        out.sort();
        out.dedup();
        out.truncate(MAX_PER_RUN);
        out
    }

    pub fn record(&mut self, id: &str, version: Option<String>, revised: Option<String>, now: u64) {
        self.songs.insert(id.to_string(), VersionInfo { checked_at: now, version, revised });
    }
}

fn clean(value: Option<&str>, max: usize, allowed: impl Fn(char) -> bool) -> Option<String> {
    let v = value?.trim();
    (!v.is_empty() && v.len() <= max && v.chars().all(allowed)).then(|| v.to_string())
}

/// Checks a report from the CustomsForge page: the values come from a
/// remote page and are treated as untrusted text.
pub fn validate_report(version: Option<&str>, revised: Option<&str>) -> (Option<String>, Option<String>) {
    let version = clean(version, 20, |c| c.is_ascii_alphanumeric() || c == '.')
        .map(|v| v.trim_start_matches(['v', 'V']).to_string())
        .filter(|v| !v.is_empty());
    let revised = clean(revised, 20, |c| c.is_ascii_digit() || c == '/' || c == '-' || c == '.');
    (version, revised)
}

fn version_parts(v: &str) -> Option<Vec<u32>> {
    v.trim_start_matches(['v', 'V']).split('.').map(|p| p.parse().ok()).collect()
}

/// Unix day of a "MM/DD/YYYY" date.
fn date_day(text: &str) -> Option<u64> {
    let mut parts = text.split('/').map(|p| p.parse::<i64>().ok());
    let (m, d, y) = (parts.next()??, parts.next()??, parts.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1970 {
        return None;
    }
    // Days from civil, proleptic Gregorian (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    u64::try_from(era * 146_097 + doe - 719_468).ok()
}

/// Whether CustomsForge has a newer version than the installed file.
/// Compares versions when both are known; otherwise falls back to a
/// "last revised" date after the download day.
pub fn update_available(installed: Option<&str>, downloaded_at: Option<u64>, remote: &VersionInfo) -> bool {
    if let (Some(local), Some(remote)) = (installed.and_then(version_parts), remote.version.as_deref().and_then(version_parts)) {
        return remote > local;
    }
    match (remote.revised.as_deref().and_then(date_day), downloaded_at) {
        (Some(revised), Some(downloaded)) => revised > day(downloaded),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_793_000_000;

    #[test]
    fn auto_runs_once_a_day() {
        let mut s = VersionStore::default();
        assert_eq!(s.start_run(false, NOW), Ok(()));
        assert_eq!(s.start_run(false, NOW + 60), Err(Refusal::AutoDoneToday));
        assert_eq!(s.start_run(false, NOW + DAY), Ok(()));
    }

    #[test]
    fn manual_limited_to_three_per_day() {
        let mut s = VersionStore::default();
        for _ in 0..3 {
            assert_eq!(s.start_run(true, NOW), Ok(()));
        }
        assert_eq!(s.manual_left(NOW), 0);
        assert_eq!(s.start_run(true, NOW), Err(Refusal::ManualLimit));
        assert_eq!(s.manual_left(NOW + DAY), 3);
        assert_eq!(s.start_run(true, NOW + DAY), Ok(()));
    }

    #[test]
    fn auto_skips_songs_checked_today() {
        let mut s = VersionStore::default();
        s.record("1", Some("1".into()), None, NOW);
        assert_eq!(s.select(["1", "2"], false, NOW + 10), vec!["2"]);
        assert_eq!(s.select(["1", "2"], true, NOW + 10), vec!["1", "2"]);
        let many: Vec<String> = (0..80).map(|i| i.to_string()).collect();
        assert_eq!(s.select(many.iter().map(|x| x.as_str()), true, NOW).len(), MAX_PER_RUN);
    }

    #[test]
    fn validates_untrusted_reports() {
        assert_eq!(validate_report(Some("v2"), Some("06/21/2025")), (Some("2".into()), Some("06/21/2025".into())));
        assert_eq!(validate_report(Some("<script>"), Some("x".repeat(30).as_str())), (None, None));
        assert_eq!(validate_report(Some(" "), Some("-")), (None, Some("-".into())));
    }

    #[test]
    fn compares_versions_then_dates() {
        let remote = |v: Option<&str>, r: Option<&str>| VersionInfo {
            checked_at: NOW,
            version: v.map(String::from),
            revised: r.map(String::from),
        };
        assert!(update_available(Some("1"), None, &remote(Some("2"), None)));
        assert!(!update_available(Some("2"), None, &remote(Some("2"), None)));
        assert!(update_available(Some("1.2"), None, &remote(Some("1.10"), None)));
        assert!(!update_available(Some("3"), None, &remote(Some("2"), None)));
        // No local version: a revision after the download counts.
        let downloaded = date_day("06/21/2025").unwrap() * DAY;
        assert!(update_available(None, Some(downloaded), &remote(None, Some("07/01/2025"))));
        assert!(!update_available(None, Some(downloaded), &remote(None, Some("06/21/2025"))));
        assert!(!update_available(None, Some(downloaded), &remote(None, Some("-"))));
    }

    #[test]
    fn date_conversion() {
        assert_eq!(date_day("01/01/1970"), Some(0));
        assert_eq!(date_day("03/01/2000"), Some(11_017));
        assert_eq!(date_day("13/01/2000"), None);
    }
}
