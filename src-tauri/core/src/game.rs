//! The game folder itself: validation and the settings in `Rocksmith.ini`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ini::IniDoc;
use crate::snapshot::SnapshotWriter;
use crate::steam;

pub const ROCKSMITH_INI: &str = "Rocksmith.ini";
const AUDIO: &str = "Audio";
const RENDERER: &str = "Renderer.Win32";

#[derive(Debug, Clone, Serialize)]
pub struct GameInfo {
    pub path: String,
    pub valid: bool,
    pub steam_version: bool,
    pub has_rocksmith_ini: bool,
    pub proton_prefix: Option<String>,
    pub problems: Vec<String>,
}

pub fn inspect(dir: &Path) -> GameInfo {
    let mut problems = Vec::new();
    let exe = dir.join(steam::ROCKSMITH_EXE).is_file();
    if !exe {
        problems.push(format!("{} wurde in diesem Ordner nicht gefunden.", steam::ROCKSMITH_EXE));
    }
    let steam_version = dir.join("steam_api.dll").is_file();
    if exe && !steam_version {
        problems.push("Das scheint nicht die Steam-Version zu sein; RS_ASIO unterstützt nur diese.".into());
    }
    GameInfo {
        path: dir.to_string_lossy().into_owned(),
        valid: exe,
        steam_version,
        has_rocksmith_ini: dir.join(ROCKSMITH_INI).is_file(),
        proton_prefix: steam::proton_prefix(dir).map(|p| p.to_string_lossy().into_owned()),
        problems,
    }
}

/// The settings GuitarStar manages. `None` means the key is missing from the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameSettings {
    pub screen_width: Option<u32>,
    pub screen_height: Option<u32>,
    /// 0 = window, 1 = fullscreen window, 2 = exclusive fullscreen.
    pub fullscreen: Option<u8>,
    pub exclusive_mode: Option<bool>,
    pub ultra_low_latency: Option<bool>,
    pub latency_buffer: Option<u8>,
}

fn num<T: std::str::FromStr>(doc: &IniDoc, section: &str, key: &str) -> Option<T> {
    doc.get(section, key)?.parse().ok()
}

fn flag(doc: &IniDoc, section: &str, key: &str) -> Option<bool> {
    num::<i32>(doc, section, key).map(|v| v != 0)
}

/// Returns `None` until the game has been started once and created the file.
pub fn read_settings(dir: &Path) -> Result<Option<GameSettings>> {
    let path = dir.join(ROCKSMITH_INI);
    if !path.is_file() {
        return Ok(None);
    }
    let doc = IniDoc::parse(&fs::read_to_string(path)?);
    Ok(Some(GameSettings {
        screen_width: num(&doc, RENDERER, "ScreenWidth"),
        screen_height: num(&doc, RENDERER, "ScreenHeight"),
        fullscreen: num(&doc, RENDERER, "Fullscreen"),
        exclusive_mode: flag(&doc, AUDIO, "ExclusiveMode"),
        ultra_low_latency: flag(&doc, AUDIO, "Win32UltraLowLatencyMode"),
        latency_buffer: num(&doc, AUDIO, "LatencyBuffer"),
    }))
}

/// Leaves the file (and the history) untouched when nothing actually changes.
pub fn write_settings(dir: &Path, s: &GameSettings, snapshot: &mut SnapshotWriter) -> Result<()> {
    let path = dir.join(ROCKSMITH_INI);
    let original = fs::read_to_string(&path)?;
    let mut doc = IniDoc::parse(&original);
    let set = |doc: &mut IniDoc, section: &str, key: &str, v: Option<String>| {
        if let Some(v) = v {
            doc.set(section, key, &v);
        }
    };
    let b = |v: Option<bool>| v.map(|x| u8::from(x).to_string());
    set(&mut doc, RENDERER, "ScreenWidth", s.screen_width.map(|v| v.to_string()));
    set(&mut doc, RENDERER, "ScreenHeight", s.screen_height.map(|v| v.to_string()));
    set(&mut doc, RENDERER, "Fullscreen", s.fullscreen.map(|v| v.to_string()));
    set(&mut doc, AUDIO, "ExclusiveMode", b(s.exclusive_mode));
    set(&mut doc, AUDIO, "Win32UltraLowLatencyMode", b(s.ultra_low_latency));
    set(&mut doc, AUDIO, "LatencyBuffer", s.latency_buffer.map(|v| v.to_string()));
    let rendered = doc.render();
    if rendered == original {
        return Ok(());
    }
    snapshot.protect(ROCKSMITH_INI)?;
    fs::write(&path, rendered)?;
    Ok(())
}

/// Settings file read by GuitarStar's dinput8.dll at game start.
pub const GUITARSTAR_INI: &str = "GuitarStar.ini";

/// Writes the window options for the DLL. `log` makes it record focus messages
/// in `GuitarStar-window.log`.
pub fn write_window_config(dir: &Path, keep_visible: bool, log: bool, snapshot: &mut SnapshotWriter) -> Result<()> {
    let path = dir.join(GUITARSTAR_INI);
    let original = fs::read_to_string(&path).unwrap_or_default();
    let mut doc = IniDoc::parse(&original);
    doc.set("Window", "KeepVisible", if keep_visible { "1" } else { "0" });
    doc.set("Window", "Log", if log { "1" } else { "0" });
    let rendered = doc.render();
    if rendered != original {
        snapshot.protect(GUITARSTAR_INI)?;
        fs::write(&path, rendered)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotStore;

    #[test]
    fn unchanged_settings_create_no_snapshot() {
        let base = std::env::temp_dir().join(format!("guitarstar-test-game-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let game = base.join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join(ROCKSMITH_INI), "[Renderer.Win32]\r\nFullscreen=2\r\n").unwrap();
        let store = SnapshotStore::new(base.join("snapshots"));
        let same = GameSettings { fullscreen: Some(2), ..Default::default() };

        let mut w = store.begin(&game, "gleich", serde_json::Value::Null).unwrap();
        write_settings(&game, &same, &mut w).unwrap();
        assert!(w.commit().unwrap().is_none());

        let changed = GameSettings { fullscreen: Some(1), ..Default::default() };
        let mut w = store.begin(&game, "anders", serde_json::Value::Null).unwrap();
        write_settings(&game, &changed, &mut w).unwrap();
        assert!(w.commit().unwrap().is_some());
        assert!(fs::read_to_string(game.join(ROCKSMITH_INI)).unwrap().contains("Fullscreen=1"));
        fs::remove_dir_all(&base).unwrap();
    }
}
