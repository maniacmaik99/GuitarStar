//! RSMods (github.com/Lovrom8/RSMods) is installed with its own installer, so
//! GuitarStar only fetches that installer, saves the files it changes and
//! starts it. RSMods has no license file, so nothing of it is bundled here.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub const REPO: &str = "Lovrom8/RSMods";
pub const INSTALLER_SUFFIX: &str = ".exe";
/// The mod DLL, its settings, and the game file its "Fast Load" and other GUI
/// features rewrite.
pub const PROTECTED_FILES: &[&str] = &["xinput1_3.dll", "RSMods.ini", "cache.psarc"];

#[derive(Debug, Clone, Serialize)]
pub struct RsModsStatus {
    pub installed: bool,
    /// The RSMods settings program, if found.
    pub gui: Option<String>,
    /// Whether cache.psarc differs from the copy saved before RSMods was
    /// installed, i.e. a "Set And Forget" mod such as Fast Load is applied.
    /// `None` when there is no saved original to compare with.
    pub cache_modified: Option<bool>,
}

fn same_content(a: &Path, b: &Path) -> Option<bool> {
    let (ma, mb) = (fs::metadata(a).ok()?, fs::metadata(b).ok()?);
    if ma.len() != mb.len() {
        return Some(false);
    }
    Some(fs::read(a).ok()? == fs::read(b).ok()?)
}

fn find_gui(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
        let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        name.ends_with(".exe") && name.contains("rsmods") && !name.contains("install")
    })
}

/// `original_cache` is GuitarStar's saved copy of cache.psarc, if any.
pub fn status(game_dir: &Path, original_cache: Option<&Path>) -> RsModsStatus {
    let gui = find_gui(game_dir).or_else(|| find_gui(&game_dir.join("RSMods")));
    RsModsStatus {
        installed: game_dir.join("xinput1_3.dll").is_file() && game_dir.join("RSMods.ini").is_file(),
        gui: gui.map(|p| p.to_string_lossy().into_owned()),
        cache_modified: original_cache
            .and_then(|orig| same_content(orig, &game_dir.join("cache.psarc")))
            .map(|same| !same),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_changed_cache() {
        let dir = std::env::temp_dir().join(format!("guitarstar-test-rsmods-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("game").join("RSMods")).unwrap();
        fs::write(dir.join("orig.psarc"), b"original").unwrap();
        fs::write(dir.join("game").join("cache.psarc"), b"original").unwrap();
        fs::write(dir.join("game").join("RSMods").join("RSMods.exe"), b"").unwrap();
        let game = dir.join("game");

        let s = status(&game, Some(&dir.join("orig.psarc")));
        assert_eq!(s.cache_modified, Some(false));
        assert!(s.gui.is_some_and(|g| g.ends_with("RSMods.exe")));
        assert!(!s.installed);

        fs::write(game.join("cache.psarc"), b"patched!").unwrap();
        assert_eq!(status(&game, Some(&dir.join("orig.psarc"))).cache_modified, Some(true));
        assert_eq!(status(&game, None).cache_modified, None);
        fs::remove_dir_all(&dir).unwrap();
    }
}
