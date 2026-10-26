//! Custom songs (CDLC). GuitarStar only ever touches its own subfolder
//! `dlc/GuitarStar`: the game's `dlc` folder also holds the official songs
//! bought on Steam, which must stay untouched. Disabled songs are moved out of
//! the game folder into the app's data folder, where a Steam file check
//! cannot remove them.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::{AppError, Result};

pub const CDLC_SUBDIR: &str = "GuitarStar";
const PC_SUFFIX: &str = "_p.psarc";
const MAC_SUFFIX: &str = "_m.psarc";

pub fn enabled_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("dlc").join(CDLC_SUBDIR)
}

pub fn disabled_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("cdlc-disabled")
}

#[derive(Debug, Clone, Serialize)]
pub struct CdlcEntry {
    pub file_name: String,
    pub artist: Option<String>,
    pub title: String,
    pub version: Option<String>,
    pub size: u64,
    pub enabled: bool,
    /// Unix seconds of the last change to the file.
    pub modified: u64,
}

/// Best-effort artist/title/version from the usual CustomsForge naming,
/// e.g. `Foo-Fighters_Everlong_v2_DD_p.psarc`.
pub fn parse_name(file_name: &str) -> (Option<String>, String, Option<String>) {
    let stem = file_name.strip_suffix(PC_SUFFIX).unwrap_or(file_name);
    let mut parts: Vec<&str> = stem.split('_').filter(|p| !p.is_empty()).collect();
    let mut version = None;
    if let Some(pos) = parts.iter().position(|p| {
        let mut c = p.chars();
        matches!(c.next(), Some('v' | 'V')) && c.next().is_some_and(|d| d.is_ascii_digit())
    }) {
        // "v1_2" style versions continue with plain numbers.
        let mut v = parts[pos][1..].to_string();
        for p in &parts[pos + 1..] {
            if p.chars().all(|c| c.is_ascii_digit()) {
                v.push('.');
                v.push_str(p);
            } else {
                break;
            }
        }
        version = Some(v);
        parts.truncate(pos);
    }
    let pretty = |s: &str| s.replace('-', " ").trim().to_string();
    match parts.len() {
        0 => (None, pretty(stem), version),
        1 => (None, pretty(parts[0]), version),
        _ => (Some(pretty(parts[0])), pretty(&parts[1..].join(" ")), version),
    }
}

fn entries_in(dir: &Path, enabled: bool, out: &mut Vec<CdlcEntry>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().ends_with(PC_SUFFIX) {
            continue;
        }
        let meta = entry.metadata()?;
        let (artist, title, version) = parse_name(&name);
        out.push(CdlcEntry {
            artist,
            title,
            version,
            size: meta.len(),
            enabled,
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
            file_name: name,
        });
    }
    Ok(())
}

pub fn list(game_dir: &Path, data_dir: &Path) -> Result<Vec<CdlcEntry>> {
    let mut out = Vec::new();
    entries_in(&enabled_dir(game_dir), true, &mut out)?;
    entries_in(&disabled_dir(data_dir), false, &mut out)?;
    out.sort_by_key(|e| (e.artist.clone().unwrap_or_default().to_lowercase(), e.title.to_lowercase()));
    Ok(out)
}

/// A bare file name, so a name from the UI can never point outside our folders.
fn checked_name(name: &str) -> Result<&str> {
    let ok = !name.is_empty()
        && !name.contains(['/', '\\', ':'])
        && name != "."
        && name != ".."
        && name.to_lowercase().ends_with(PC_SUFFIX);
    if ok {
        Ok(name)
    } else {
        Err(AppError::msg(format!("Ungültiger Songname: {name}")))
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportResult {
    pub imported: Vec<String>,
    /// File name and reason.
    pub skipped: Vec<(String, String)>,
}

fn store(game_dir: &Path, data_dir: &Path, name: &str, bytes: &[u8], result: &mut ImportResult) -> Result<()> {
    let name = checked_name(name)?;
    let target_dir = enabled_dir(game_dir);
    fs::create_dir_all(&target_dir)?;
    // Write to a temporary name first so the game never sees half a file.
    let tmp = target_dir.join(format!("{name}.part"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, target_dir.join(name))?;
    // A re-imported song replaces a disabled copy as well.
    let disabled = disabled_dir(data_dir).join(name);
    if disabled.is_file() {
        fs::remove_file(disabled)?;
    }
    result.imported.push(name.to_string());
    Ok(())
}

fn skip_reason(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.ends_with(MAC_SUFFIX) {
        Some("Mac-Version; für Windows wird die Datei mit _p.psarc gebraucht")
    } else if !lower.ends_with(PC_SUFFIX) {
        Some("keine Rocksmith-Songdatei (_p.psarc)")
    } else {
        None
    }
}

/// Imports `.psarc` files directly and the PC songs from `.zip` archives.
pub fn import(game_dir: &Path, data_dir: &Path, paths: &[PathBuf]) -> Result<ImportResult> {
    let mut result = ImportResult::default();
    for path in paths {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.to_lowercase().ends_with(".zip") {
            import_zip(game_dir, data_dir, &name, &fs::read(path)?, &mut result)?;
            continue;
        }
        match skip_reason(&name) {
            Some(reason) => result.skipped.push((name, reason.into())),
            None => store(game_dir, data_dir, &name, &fs::read(path)?, &mut result)?,
        }
    }
    Ok(result)
}

fn import_zip(game_dir: &Path, data_dir: &Path, zip_name: &str, bytes: &[u8], result: &mut ImportResult) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        else {
            continue;
        };
        if skip_reason(&name).is_some() {
            continue;
        }
        found = true;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        store(game_dir, data_dir, &name, &buf, result)?;
    }
    if !found {
        result.skipped.push((zip_name.to_string(), "Archiv enthält keine Windows-Songdatei (_p.psarc)".into()));
    }
    Ok(())
}

pub fn set_enabled(game_dir: &Path, data_dir: &Path, name: &str, enabled: bool) -> Result<()> {
    let name = checked_name(name)?;
    let (from, to) = if enabled {
        (disabled_dir(data_dir), enabled_dir(game_dir))
    } else {
        (enabled_dir(game_dir), disabled_dir(data_dir))
    };
    let src = from.join(name);
    if !src.is_file() {
        return Err(AppError::msg(format!("{name} wurde nicht gefunden.")));
    }
    fs::create_dir_all(&to)?;
    // Game folder and app data can be on different drives, so copy instead of rename.
    fs::copy(&src, to.join(name))?;
    fs::remove_file(src)?;
    Ok(())
}

pub fn delete(game_dir: &Path, data_dir: &Path, name: &str) -> Result<()> {
    let name = checked_name(name)?;
    for dir in [enabled_dir(game_dir), disabled_dir(data_dir)] {
        let p = dir.join(name);
        if p.is_file() {
            fs::remove_file(p)?;
        }
    }
    Ok(())
}

/// Whether the free "Cherub Rock" DLC (app 248750) is installed; older CDLC
/// enablers need it.
pub fn cherub_rock_installed(game_dir: &Path) -> Option<bool> {
    let steamapps = game_dir.parent()?.parent()?;
    let acf = fs::read_to_string(steamapps.join("appmanifest_221680.acf")).ok()?;
    Some(acf.contains("\"248750\""))
}

/// The enabler installed by CustomsForge's installer lives next to the game exe.
pub fn enabler_installed(game_dir: &Path) -> bool {
    game_dir.join("D3DX9_42.dll").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_customsforge_names() {
        assert_eq!(
            parse_name("Foo-Fighters_Everlong_v2_DD_p.psarc"),
            (Some("Foo Fighters".into()), "Everlong".into(), Some("2".into()))
        );
        assert_eq!(
            parse_name("Metallica_Master-of-Puppets_v1_2_p.psarc"),
            (Some("Metallica".into()), "Master of Puppets".into(), Some("1.2".into()))
        );
        assert_eq!(parse_name("song_p.psarc"), (None, "song".into(), None));
    }

    #[test]
    fn rejects_unsafe_names() {
        assert!(checked_name("../evil_p.psarc").is_err());
        assert!(checked_name("x\\y_p.psarc").is_err());
        assert!(checked_name("song_m.psarc").is_err());
        assert!(checked_name("Band_Song_v1_p.psarc").is_ok());
    }

    #[test]
    fn import_enable_disable_delete() {
        let base = std::env::temp_dir().join(format!("guitarstar-test-cdlc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let (game, data, src) = (base.join("game"), base.join("data"), base.join("src"));
        fs::create_dir_all(game.join("dlc")).unwrap();
        fs::create_dir_all(&src).unwrap();
        // An official song directly in dlc/ must never be listed or touched.
        fs::write(game.join("dlc").join("official_p.psarc"), b"x").unwrap();
        fs::write(src.join("Band_Song_v1_p.psarc"), b"song").unwrap();
        fs::write(src.join("Band_Song_v1_m.psarc"), b"mac").unwrap();
        let mut zip_bytes = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut zip_bytes));
            z.start_file("folder/Other_Tune_p.psarc", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"tune").unwrap();
            z.finish().unwrap();
        }
        fs::write(src.join("pack.zip"), &zip_bytes).unwrap();

        let r = import(
            &game,
            &data,
            &[src.join("Band_Song_v1_p.psarc"), src.join("Band_Song_v1_m.psarc"), src.join("pack.zip")],
        )
        .unwrap();
        assert_eq!(r.imported, vec!["Band_Song_v1_p.psarc", "Other_Tune_p.psarc"]);
        assert_eq!(r.skipped.len(), 1);
        assert_eq!(list(&game, &data).unwrap().len(), 2);

        set_enabled(&game, &data, "Band_Song_v1_p.psarc", false).unwrap();
        let songs = list(&game, &data).unwrap();
        assert!(songs.iter().any(|s| s.file_name == "Band_Song_v1_p.psarc" && !s.enabled));
        assert!(!enabled_dir(&game).join("Band_Song_v1_p.psarc").exists());

        delete(&game, &data, "Band_Song_v1_p.psarc").unwrap();
        assert_eq!(list(&game, &data).unwrap().len(), 1);
        assert!(game.join("dlc").join("official_p.psarc").exists());
        fs::remove_dir_all(&base).unwrap();
    }
}
