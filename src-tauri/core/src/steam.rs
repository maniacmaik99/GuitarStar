//! Locating Steam libraries and the Rocksmith 2014 install. Linux paths are
//! included already so the Proton build only needs testing, not a redesign.

use std::fs;
use std::path::{Path, PathBuf};

pub const ROCKSMITH_APP_ID: &str = "221680";
pub const ROCKSMITH_EXE: &str = "Rocksmith2014.exe";

pub(crate) fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("SteamPath") {
                roots.push(PathBuf::from(path));
            }
        }
        roots.push(PathBuf::from("C:\\Program Files (x86)\\Steam"));
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        // Flatpak Steam
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    roots
}

/// Reads the `"path"` entries of `steamapps/libraryfolders.vdf`.
fn parse_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('"').collect();
            // A key/value line splits into ["", key, ws, value, ""].
            (parts.len() >= 5 && parts[1].eq_ignore_ascii_case("path"))
                .then(|| PathBuf::from(parts[3].replace("\\\\", "\\")))
        })
        .collect()
}

fn library_folders() -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        let mut found = fs::read_to_string(&vdf).map(|t| parse_library_paths(&t)).unwrap_or_default();
        found.push(root);
        for lib in found {
            let canonical = fs::canonicalize(&lib).unwrap_or(lib);
            if canonical.join("steamapps").is_dir() && !libs.contains(&canonical) {
                libs.push(canonical);
            }
        }
    }
    libs
}

pub fn find_rocksmith_installs() -> Vec<PathBuf> {
    library_folders()
        .into_iter()
        .map(|lib| lib.join("steamapps").join("common").join("Rocksmith2014"))
        .filter(|dir| dir.join(ROCKSMITH_EXE).is_file())
        .map(|dir| strip_verbatim(&dir))
        .collect()
}

/// The Proton prefix that holds the game's Windows registry and drivers on Linux.
pub fn proton_prefix(game_dir: &Path) -> Option<PathBuf> {
    let steamapps = game_dir.parent()?.parent()?;
    let pfx = steamapps.join("compatdata").join(ROCKSMITH_APP_ID).join("pfx");
    pfx.is_dir().then_some(pfx)
}

/// `fs::canonicalize` returns `\\?\C:\...` on Windows, which looks odd in the UI.
pub fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => p.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_vdf() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t\t\"apps\"\n\t}\n}\n";
        assert_eq!(
            parse_library_paths(vdf),
            vec![PathBuf::from("C:\\Program Files (x86)\\Steam"), PathBuf::from("D:\\SteamLibrary")]
        );
    }
}
