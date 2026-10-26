//! Every change to the game folder goes through a snapshot: the original files
//! (or the fact that they did not exist) are saved first, so any operation can
//! be undone and the folder can always be returned to its untouched state.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// Path relative to the game folder, with forward slashes.
    pub path: String,
    pub existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    /// Unix seconds.
    pub created: u64,
    pub label: String,
    pub files: Vec<SnapshotFile>,
    /// App state as it was before this operation, restored together with the files.
    pub state_before: serde_json::Value,
}

pub struct SnapshotStore {
    root: PathBuf,
}

pub struct SnapshotWriter {
    dir: PathBuf,
    game_dir: PathBuf,
    snapshot: Snapshot,
}

fn checked_relative(rel: &str) -> Result<PathBuf> {
    let p = Path::new(rel);
    if p.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(p.to_path_buf())
    } else {
        Err(AppError::msg(format!("Ungültiger Pfad im Spielordner: {rel}")))
    }
}

fn copy_with_parents(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to)?;
    Ok(())
}

impl SnapshotStore {
    pub fn new(root: PathBuf) -> Self {
        SnapshotStore { root }
    }

    /// Newest first.
    pub fn list(&self) -> Result<Vec<Snapshot>> {
        let mut out = Vec::new();
        if !self.root.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(&self.root)? {
            let manifest = entry?.path().join("manifest.json");
            if let Ok(text) = fs::read_to_string(&manifest) {
                out.push(serde_json::from_str::<Snapshot>(&text)?);
            }
        }
        out.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(out)
    }

    pub fn begin(&self, game_dir: &Path, label: &str, state_before: serde_json::Value) -> Result<SnapshotWriter> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        // Zero-padded nanoseconds sort lexically in creation order.
        let id = format!("{:020}", now.as_nanos());
        let dir = self.root.join(&id);
        fs::create_dir_all(dir.join("files"))?;
        Ok(SnapshotWriter {
            dir,
            game_dir: game_dir.to_path_buf(),
            snapshot: Snapshot {
                id,
                created: now.as_secs(),
                label: label.to_string(),
                files: Vec::new(),
                state_before,
            },
        })
    }

    fn restore_one(&self, game_dir: &Path, snap: &Snapshot) -> Result<()> {
        let saved = self.root.join(&snap.id).join("files");
        for f in &snap.files {
            let rel = checked_relative(&f.path)?;
            let target = game_dir.join(&rel);
            if f.existed {
                copy_with_parents(&saved.join(&rel), &target)?;
            } else if target.exists() {
                fs::remove_file(&target)?;
            }
        }
        Ok(())
    }

    /// Undoes `id` and every later snapshot, newest first, and returns the
    /// app state from before `id`.
    pub fn restore_to(&self, game_dir: &Path, id: &str) -> Result<serde_json::Value> {
        let all = self.list()?;
        if !all.iter().any(|s| s.id == id) {
            return Err(AppError::msg("Diese Sicherung gibt es nicht mehr."));
        }
        let mut state = serde_json::Value::Null;
        for snap in all.iter().take_while(|s| s.id.as_str() >= id) {
            self.restore_one(game_dir, snap)?;
            state = snap.state_before.clone();
            fs::remove_dir_all(self.root.join(&snap.id))?;
        }
        Ok(state)
    }

    /// Returns the folder to the state before the first snapshot.
    pub fn restore_original(&self, game_dir: &Path) -> Result<Option<serde_json::Value>> {
        match self.list()?.last() {
            Some(oldest) => self.restore_to(game_dir, &oldest.id.clone()).map(Some),
            None => Ok(None),
        }
    }
}

impl SnapshotWriter {
    /// Saves the current version of `rel` before it is changed. Calling it
    /// twice for the same file keeps the first (original) copy.
    pub fn protect(&mut self, rel: &str) -> Result<()> {
        let rel_norm = rel.replace('\\', "/");
        if self.snapshot.files.iter().any(|f| f.path.eq_ignore_ascii_case(&rel_norm)) {
            return Ok(());
        }
        let rel_path = checked_relative(&rel_norm)?;
        let src = self.game_dir.join(&rel_path);
        let existed = src.is_file();
        if existed {
            copy_with_parents(&src, &self.dir.join("files").join(&rel_path))?;
        }
        self.snapshot.files.push(SnapshotFile { path: rel_norm, existed });
        Ok(())
    }

    pub fn commit(self) -> Result<Option<Snapshot>> {
        if self.snapshot.files.is_empty() {
            fs::remove_dir_all(&self.dir)?;
            return Ok(None);
        }
        fs::write(self.dir.join("manifest.json"), serde_json::to_string_pretty(&self.snapshot)?)?;
        Ok(Some(self.snapshot))
    }

    /// Like `commit`, but keeps the snapshot even without files, for changes
    /// outside the game folder that are tracked through the saved app state.
    pub fn commit_always(self) -> Result<Snapshot> {
        fs::write(self.dir.join("manifest.json"), serde_json::to_string_pretty(&self.snapshot)?)?;
        Ok(self.snapshot)
    }

    /// Discards an unfinished snapshot after rolling back what was written so far.
    pub fn abort(self) -> Result<()> {
        let saved = self.dir.join("files");
        for f in &self.snapshot.files {
            let rel = checked_relative(&f.path)?;
            let target = self.game_dir.join(&rel);
            if f.existed {
                copy_with_parents(&saved.join(&rel), &target)?;
            } else if target.exists() {
                fs::remove_file(&target)?;
            }
        }
        fs::remove_dir_all(&self.dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("guitarstar-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn restore_original_undoes_all_changes() {
        let base = temp_dir("snap");
        let game = base.join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("Rocksmith.ini"), "original").unwrap();
        let store = SnapshotStore::new(base.join("snapshots"));

        let mut w = store.begin(&game, "eins", serde_json::json!({"step": 0})).unwrap();
        w.protect("Rocksmith.ini").unwrap();
        w.protect("avrt.dll").unwrap();
        fs::write(game.join("Rocksmith.ini"), "geändert").unwrap();
        fs::write(game.join("avrt.dll"), "neu").unwrap();
        w.commit().unwrap();

        let mut w = store.begin(&game, "zwei", serde_json::json!({"step": 1})).unwrap();
        w.protect("Rocksmith.ini").unwrap();
        fs::write(game.join("Rocksmith.ini"), "nochmal").unwrap();
        w.commit().unwrap();

        assert_eq!(store.list().unwrap().len(), 2);
        let state = store.restore_original(&game).unwrap().unwrap();
        assert_eq!(state, serde_json::json!({"step": 0}));
        assert_eq!(fs::read_to_string(game.join("Rocksmith.ini")).unwrap(), "original");
        assert!(!game.join("avrt.dll").exists());
        assert!(store.list().unwrap().is_empty());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_paths_outside_game_folder() {
        assert!(checked_relative("../evil.dll").is_err());
        assert!(checked_relative("C:/Windows/x.dll").is_err());
        assert!(checked_relative("dlc/song_p.psarc").is_ok());
    }
}
