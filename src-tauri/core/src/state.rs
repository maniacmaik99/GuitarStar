use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::audio::AudioConfig;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledComponent {
    pub version: String,
    /// Files placed in the game folder, relative to it.
    pub files: Vec<String>,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub game_dir: Option<String>,
    #[serde(default)]
    pub installed: BTreeMap<String, InstalledComponent>,
    #[serde(default)]
    pub audio: Option<AudioConfig>,
    /// Whether GuitarStar set the high-DPI compatibility flag for the game exe.
    #[serde(default)]
    pub dpi_override: bool,
    /// Keep the game visible on focus loss (handled by GuitarStar's dinput8.dll).
    #[serde(default)]
    pub keep_visible: bool,
    /// Diagnostic logs of GuitarStar's dinput8.dll.
    #[serde(default)]
    pub dll_log: bool,
    /// Hide ads in the embedded CustomsForge view. Off by default: the ads are
    /// CustomsForge's, and they help pay for the site.
    #[serde(default)]
    pub hide_ads: bool,
}

impl AppState {
    pub fn load(data_dir: &Path) -> Self {
        fs::read_to_string(data_dir.join("state.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Writes through a temporary file so a crash never leaves half a state behind.
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir)?;
        let tmp = data_dir.join("state.json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, data_dir.join("state.json"))?;
        Ok(())
    }
}
