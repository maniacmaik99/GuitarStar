//! Backups of the player's Rocksmith save (profile, progress) from the Steam
//! Cloud folder `<Steam>/userdata/<account>/221680/remote`. Every file in that
//! folder counts as part of the save.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};
use crate::i18n::Label;
use crate::tr;
use crate::steam::{self, ROCKSMITH_APP_ID};

/// Automatic backups kept per account; manual ones are never deleted automatically.
pub const KEEP_AUTOMATIC: usize = 20;
/// Label key of the backup made before a restore.
pub const LABEL_BEFORE_RESTORE: &str = "labels.beforeRestore";
const MANIFEST: &str = "manifest.json";
const DAY_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize)]
pub struct SaveFolder {
    /// Steam account id (the folder name under `userdata`).
    pub account: String,
    pub path: String,
    pub file_count: usize,
    pub size: u64,
    /// Unix seconds of the newest file, `None` for an empty folder.
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    /// Path relative to the save folder, with forward slashes.
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    /// Unix seconds.
    pub created: u64,
    /// The label in the language at creation time; shown when there is no key.
    pub label: String,
    /// Key and arguments of the label, so it can be shown in the current language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_args: Option<BTreeMap<String, String>>,
    pub account: String,
    pub automatic: bool,
    /// The save folder the files came from.
    pub source: String,
    pub size: u64,
    pub files: Vec<BackupFile>,
}

fn unix_secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Regular files below `dir` (relative, forward slashes). Symlinks are skipped,
/// so nothing outside the folder is ever read.
fn list_files(dir: &Path) -> Result<Vec<(String, fs::Metadata)>> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, fs::Metadata)>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            if kind.is_dir() {
                walk(&entry.path(), &rel, out)?;
            } else if kind.is_file() {
                out.push((rel, entry.metadata()?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn checked_relative(rel: &str) -> Result<PathBuf> {
    crate::snapshot::checked_relative(rel)
        .map_err(|_| AppError::msg(tr!("saves.invalidPath", path = rel)))
}

fn sha256_of(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    io::copy(&mut fs::File::open(path)?, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// The account id from `userdata/<account>/221680/remote`.
pub fn account_of(remote_dir: &Path) -> String {
    remote_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unbekannt".to_string())
}

pub fn inspect_folder(remote_dir: &Path) -> Result<SaveFolder> {
    let files = list_files(remote_dir)?;
    Ok(SaveFolder {
        account: account_of(remote_dir),
        path: remote_dir.to_string_lossy().into_owned(),
        file_count: files.len(),
        size: files.iter().map(|(_, m)| m.len()).sum(),
        modified: files.iter().filter_map(|(_, m)| m.modified().ok()).map(unix_secs).max(),
    })
}

/// Save folders below the given Steam roots. Steam keeps `userdata` in its main
/// install only, never in additional libraries.
pub fn save_folders_in(steam_roots: &[PathBuf]) -> Vec<SaveFolder> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for root in steam_roots {
        let Ok(accounts) = fs::read_dir(root.join("userdata")) else { continue };
        for account in accounts.flatten() {
            let remote = account.path().join(ROCKSMITH_APP_ID).join("remote");
            if !remote.is_dir() {
                continue;
            }
            let canonical = fs::canonicalize(&remote).unwrap_or_else(|_| remote.clone());
            if seen.contains(&canonical) {
                continue;
            }
            seen.push(canonical.clone());
            if let Ok(folder) = inspect_folder(&steam::strip_verbatim(&canonical)) {
                out.push(folder);
            }
        }
    }
    out
}

pub fn find_save_folders() -> Vec<SaveFolder> {
    save_folders_in(&steam::steam_roots())
}

fn read_manifest(dir: &Path) -> Result<BackupInfo> {
    let mut info: BackupInfo = serde_json::from_str(&fs::read_to_string(dir.join(MANIFEST))?)?;
    // Backups from before labels had keys carry plain German text.
    if info.label_key.is_none() {
        if let Some(label) = crate::i18n::legacy_label(&info.label) {
            info.label_key = label.key();
            info.label_args = label.args();
        }
    }
    Ok(info)
}

/// Copies every file of `remote_dir` into a new backup below `backups_root`.
/// The backup is written to a temporary folder and only renamed into place once
/// complete, so an interrupted backup never looks like a valid one.
fn write_backup(remote_dir: &Path, backups_root: &Path, label: &Label, automatic: bool) -> Result<BackupInfo> {
    let account = account_of(remote_dir);
    let account_dir = backups_root.join(&account);
    fs::create_dir_all(&account_dir)?;
    remove_unfinished(&account_dir);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    // Zero-padded nanoseconds sort lexically in creation order.
    let id = format!("{:020}", now.as_nanos());
    let tmp = account_dir.join(format!(".tmp-{id}"));
    let result = (|| {
        let mut files = Vec::new();
        for (rel, _) in list_files(remote_dir)? {
            let to = tmp.join("files").join(checked_relative(&rel)?);
            fs::create_dir_all(to.parent().expect("has a parent"))?;
            let size = fs::copy(remote_dir.join(&rel), &to)?;
            files.push(BackupFile { path: rel, size, sha256: sha256_of(&to)? });
        }
        let info = BackupInfo {
            id: id.clone(),
            created: now.as_secs(),
            label: label.text(),
            label_key: label.key(),
            label_args: label.args(),
            account: account.clone(),
            automatic,
            source: remote_dir.to_string_lossy().into_owned(),
            size: files.iter().map(|f| f.size).sum(),
            files,
        };
        fs::create_dir_all(&tmp)?;
        fs::write(tmp.join(MANIFEST), serde_json::to_string_pretty(&info)?)?;
        fs::rename(&tmp, account_dir.join(&id))?;
        Ok(info)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    result
}

/// Leftovers of a backup that was interrupted (crash, power loss).
fn remove_unfinished(account_dir: &Path) {
    let Ok(entries) = fs::read_dir(account_dir) else { return };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(".tmp-") {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Backs up the save folder and then trims old automatic backups.
pub fn backup(remote_dir: &Path, backups_root: &Path, label: &Label, automatic: bool) -> Result<BackupInfo> {
    if !remote_dir.is_dir() {
        return Err(AppError::msg(tr!("saves.folderMissing", path = remote_dir.display())));
    }
    let info = write_backup(remote_dir, backups_root, label, automatic)?;
    prune(backups_root, &info.account, KEEP_AUTOMATIC)?;
    Ok(info)
}

/// All backups of all accounts, newest first.
pub fn list_backups(backups_root: &Path) -> Result<Vec<BackupInfo>> {
    let mut out = Vec::new();
    let Ok(accounts) = fs::read_dir(backups_root) else { return Ok(out) };
    for account in accounts.flatten() {
        let Ok(backups) = fs::read_dir(account.path()) else { continue };
        for b in backups.flatten() {
            if b.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if let Ok(info) = read_manifest(&b.path()) {
                out.push(info);
            }
        }
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(out)
}

fn backup_dir(backups_root: &Path, info: &BackupInfo) -> Result<PathBuf> {
    checked_relative(&info.account)?;
    Ok(backups_root.join(&info.account).join(&info.id))
}

pub fn find_backup(backups_root: &Path, id: &str) -> Result<BackupInfo> {
    list_backups(backups_root)?
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| AppError::msg(tr!("saves.backupGone")))
}

pub fn delete_backup(backups_root: &Path, id: &str) -> Result<()> {
    let info = find_backup(backups_root, id)?;
    fs::remove_dir_all(backup_dir(backups_root, &info)?)?;
    Ok(())
}

/// Deletes automatic backups of `account` beyond the newest `keep`.
pub fn prune(backups_root: &Path, account: &str, keep: usize) -> Result<()> {
    let old: Vec<BackupInfo> = list_backups(backups_root)?
        .into_iter()
        .filter(|b| b.account == account && b.automatic)
        .skip(keep)
        .collect();
    for b in old {
        fs::remove_dir_all(backup_dir(backups_root, &b)?)?;
    }
    Ok(())
}

/// Checks that every file of the backup is present and unchanged.
fn verify(dir: &Path, info: &BackupInfo) -> Result<()> {
    for f in &info.files {
        let path = dir.join("files").join(checked_relative(&f.path)?);
        if !path.is_file() || sha256_of(&path)? != f.sha256 {
            return Err(AppError::msg(tr!("saves.damaged", file = f.path)));
        }
    }
    Ok(())
}

/// Makes sure `target` (whose parent exists) does not lead out of `root`, e.g.
/// through a symlinked subfolder.
fn ensure_inside(root: &Path, target: &Path) -> Result<()> {
    let parent = fs::canonicalize(target.parent().unwrap_or(root))?;
    if parent.starts_with(fs::canonicalize(root)?) {
        Ok(())
    } else {
        Err(AppError::msg(tr!("saves.outsideFolder", path = target.display())))
    }
}

/// Replaces the content of `remote_dir` with the backup `id`. The current state
/// is backed up first and returned, so a restore can itself be undone.
pub fn restore(backups_root: &Path, id: &str, remote_dir: &Path) -> Result<BackupInfo> {
    let info = find_backup(backups_root, id)?;
    let dir = backup_dir(backups_root, &info)?;
    verify(&dir, &info)?;
    fs::create_dir_all(remote_dir)?;
    let safety = write_backup(remote_dir, backups_root, &Label::new(LABEL_BEFORE_RESTORE), true)?;

    let wanted: Vec<&str> = info.files.iter().map(|f| f.path.as_str()).collect();
    for (rel, _) in list_files(remote_dir)? {
        if !wanted.contains(&rel.as_str()) {
            fs::remove_file(remote_dir.join(checked_relative(&rel)?))?;
        }
    }
    for f in &info.files {
        let rel = checked_relative(&f.path)?;
        let target = remote_dir.join(&rel);
        fs::create_dir_all(target.parent().expect("has a parent"))?;
        ensure_inside(remote_dir, &target)?;
        fs::copy(dir.join("files").join(&rel), &target)?;
    }
    // Only now, so the backup being restored cannot be trimmed away before it is used.
    prune(backups_root, &safety.account, KEEP_AUTOMATIC)?;
    Ok(safety)
}

/// The daily automatic backup is due when the save changed since the newest
/// backup of the account and that backup is at least a day old.
pub fn daily_backup_due(folder: &SaveFolder, backups: &[BackupInfo], now: u64) -> bool {
    if folder.file_count == 0 {
        return false;
    }
    match backups.iter().filter(|b| b.account == folder.account).map(|b| b.created).max() {
        None => true,
        Some(last) => now >= last + DAY_SECS && folder.modified.is_some_and(|m| m >= last),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("guitarstar-saves-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// A fake Steam root with one account's save folder.
    fn steam_with_save(base: &Path) -> PathBuf {
        let remote = base.join("steam/userdata/12345/221680/remote");
        fs::create_dir_all(remote.join("sub")).unwrap();
        fs::write(remote.join("abc_PRFLDB"), "profil-v1").unwrap();
        fs::write(remote.join("LocalProfiles.json"), "{}").unwrap();
        fs::write(remote.join("sub/crd"), "crd").unwrap();
        remote
    }

    #[test]
    fn finds_save_folders() {
        let base = temp_dir("find");
        steam_with_save(&base);
        fs::create_dir_all(base.join("steam/userdata/999/440/remote")).unwrap();
        let roots = vec![base.join("steam"), base.join("steam"), base.join("missing")];
        let found = save_folders_in(&roots);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].account, "12345");
        assert_eq!(found[0].file_count, 3);
        assert_eq!(found[0].size, 9 + 2 + 3);
        assert!(found[0].modified.is_some());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn backup_copies_all_files_and_lists_newest_first() {
        let base = temp_dir("backup");
        let remote = steam_with_save(&base);
        let root = base.join("backups");
        let first = backup(&remote, &root, &"eins".into(), false).unwrap();
        let second = backup(&remote, &root, &"zwei".into(), true).unwrap();
        assert_eq!(first.account, "12345");
        assert_eq!(first.files.len(), 3);
        let copy = root.join("12345").join(&first.id).join("files/sub/crd");
        assert_eq!(fs::read_to_string(copy).unwrap(), "crd");
        let list = list_backups(&root).unwrap();
        assert_eq!(list.iter().map(|b| b.id.clone()).collect::<Vec<_>>(), vec![second.id, first.id]);
        // Interrupted backups are ignored and cleaned up by the next one.
        fs::create_dir_all(root.join("12345/.tmp-1/files")).unwrap();
        assert_eq!(list_backups(&root).unwrap().len(), 2);
        backup(&remote, &root, &"drei".into(), false).unwrap();
        assert!(!root.join("12345/.tmp-1").exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn restore_replaces_files_after_safety_backup() {
        let base = temp_dir("restore");
        let remote = steam_with_save(&base);
        let root = base.join("backups");
        let saved = backup(&remote, &root, &"gut".into(), false).unwrap();

        fs::write(remote.join("abc_PRFLDB"), "kaputt").unwrap();
        fs::write(remote.join("extra.tmp"), "neu").unwrap();
        fs::remove_file(remote.join("sub/crd")).unwrap();

        let safety = restore(&root, &saved.id, &remote).unwrap();
        assert_eq!(safety.label_key.as_deref(), Some(LABEL_BEFORE_RESTORE));
        assert!(safety.automatic);
        assert!(safety.files.iter().any(|f| f.path == "extra.tmp"));
        assert_eq!(fs::read_to_string(remote.join("abc_PRFLDB")).unwrap(), "profil-v1");
        assert_eq!(fs::read_to_string(remote.join("sub/crd")).unwrap(), "crd");
        assert!(!remote.join("extra.tmp").exists());
        assert_eq!(list_backups(&root).unwrap().len(), 2);

        // The safety backup undoes the restore.
        restore(&root, &safety.id, &remote).unwrap();
        assert_eq!(fs::read_to_string(remote.join("abc_PRFLDB")).unwrap(), "kaputt");
        assert!(remote.join("extra.tmp").exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn restore_refuses_damaged_or_unsafe_backups() {
        let base = temp_dir("unsafe");
        let remote = steam_with_save(&base);
        let root = base.join("backups");
        let saved = backup(&remote, &root, &"gut".into(), false).unwrap();
        let dir = root.join("12345").join(&saved.id);

        fs::write(dir.join("files/abc_PRFLDB"), "verändert").unwrap();
        assert!(restore(&root, &saved.id, &remote).is_err());

        let mut info = read_manifest(&dir).unwrap();
        info.files = vec![BackupFile { path: "../../evil".into(), size: 0, sha256: String::new() }];
        fs::write(dir.join(MANIFEST), serde_json::to_string(&info).unwrap()).unwrap();
        assert!(restore(&root, &saved.id, &remote).is_err());

        // Nothing was touched and no safety backup was made.
        assert_eq!(fs::read_to_string(remote.join("abc_PRFLDB")).unwrap(), "profil-v1");
        assert_eq!(list_backups(&root).unwrap().len(), 1);
        assert!(checked_relative("C:/x").is_err());
        assert!(checked_relative("sub/crd").is_ok());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn retention_keeps_manual_backups() {
        let base = temp_dir("prune");
        let remote = steam_with_save(&base);
        let root = base.join("backups");
        let manual = backup(&remote, &root, &"von Hand".into(), false).unwrap();
        let mut autos = Vec::new();
        for _ in 0..4 {
            autos.push(write_backup(&remote, &root, &"auto".into(), true).unwrap());
        }
        prune(&root, "12345", 2).unwrap();
        let ids: Vec<String> = list_backups(&root).unwrap().into_iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![autos[3].id.clone(), autos[2].id.clone(), manual.id.clone()]);
        delete_backup(&root, &manual.id).unwrap();
        assert_eq!(list_backups(&root).unwrap().len(), 2);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn daily_backup_only_when_old_and_changed() {
        let folder = SaveFolder {
            account: "1".into(),
            path: String::new(),
            file_count: 2,
            size: 10,
            modified: Some(5_000),
        };
        let b = |created| BackupInfo {
            id: String::new(),
            created,
            label: String::new(),
            label_key: None,
            label_args: None,
            account: "1".into(),
            automatic: true,
            source: String::new(),
            size: 0,
            files: Vec::new(),
        };
        assert!(daily_backup_due(&folder, &[], 10));
        assert!(!daily_backup_due(&folder, &[b(4_000)], 4_000 + DAY_SECS - 1));
        assert!(daily_backup_due(&folder, &[b(4_000)], 4_000 + DAY_SECS));
        assert!(!daily_backup_due(&folder, &[b(6_000)], 6_000 + DAY_SECS));
    }
}
