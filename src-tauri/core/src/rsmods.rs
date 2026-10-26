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
}

fn find_gui(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
        let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        name.ends_with(".exe") && name.contains("rsmods") && !name.contains("install")
    })
}

pub fn status(game_dir: &Path) -> RsModsStatus {
    let gui = find_gui(game_dir).or_else(|| find_gui(&game_dir.join("RSMods")));
    RsModsStatus {
        installed: game_dir.join("xinput1_3.dll").is_file() && game_dir.join("RSMods.ini").is_file(),
        gui: gui.map(|p| p.to_string_lossy().into_owned()),
    }
}
