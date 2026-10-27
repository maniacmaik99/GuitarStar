//! Audio interface support through RS_ASIO: device profiles, ASIO driver
//! discovery and generation of `RS_ASIO.ini`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::i18n::{self, Lang};
use crate::ini::IniDoc;

/// Template shipped with RS_ASIO (MIT), used when the release archive has none.
pub const RS_ASIO_INI_TEMPLATE: &str = include_str!("../profiles/rs_asio_template.ini");
const BUILTIN_PROFILES: &str = include_str!("../profiles/devices.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub id: String,
    pub name: String,
    pub driver: String,
    /// The device only replaces the output; instrument input stays on Windows.
    #[serde(default)]
    pub output_only: bool,
    pub output_base_channel: u32,
    #[serde(default)]
    pub alt_base_channel: Option<u32>,
    pub input_channel: u32,
    #[serde(default)]
    pub second_input_channel: Option<u32>,
    pub buffer_size_mode: String,
    #[serde(default)]
    pub custom_buffer_size: Option<u32>,
    #[serde(default)]
    pub enable_ref_count_hack: Option<bool>,
    #[serde(default)]
    pub notes: Option<String>,
    /// English name and notes of a bundled profile; `list_profiles` puts them
    /// into `name`/`notes` when the UI is in English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_en: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    /// Set for profiles the user created, false for the bundled ones.
    #[serde(default)]
    pub custom: bool,
}

/// Bundled profiles with name and notes in `lang`.
fn builtin_profiles(lang: Lang) -> Result<Vec<DeviceProfile>> {
    let mut profiles: Vec<DeviceProfile> = serde_json::from_str(BUILTIN_PROFILES)?;
    for p in &mut profiles {
        let (name_en, notes_en) = (p.name_en.take(), p.notes_en.take());
        if lang == Lang::En {
            p.name = name_en.unwrap_or_else(|| p.name.clone());
            p.notes = notes_en.or(p.notes.take());
        }
    }
    Ok(profiles)
}

/// Lists bundled profiles (in the current language) followed by the user's own from `user_dir`.
pub fn list_profiles(user_dir: &Path) -> Result<Vec<DeviceProfile>> {
    let mut profiles = builtin_profiles(i18n::lang())?;
    if user_dir.is_dir() {
        for entry in fs::read_dir(user_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let mut p: DeviceProfile = serde_json::from_str(&fs::read_to_string(&path)?)?;
                p.custom = true;
                profiles.push(p);
            }
        }
    }
    Ok(profiles)
}

pub fn save_user_profile(user_dir: &Path, profile: &DeviceProfile) -> Result<()> {
    fs::create_dir_all(user_dir)?;
    let safe_id: String = profile
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let mut p = profile.clone();
    p.custom = true;
    fs::write(user_dir.join(format!("{safe_id}.json")), serde_json::to_string_pretty(&p)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct AsioDriver {
    pub name: String,
    /// Rocksmith is a 32-bit game and can only load 32-bit ASIO drivers.
    pub has_32bit: bool,
    pub has_64bit: bool,
}

#[cfg(windows)]
pub fn asio_drivers() -> Vec<AsioDriver> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let read = |path: &str| -> Vec<String> {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(path)
            .map(|k| k.enum_keys().filter_map(|n| n.ok()).collect())
            .unwrap_or_default()
    };
    // This app is 64-bit: SOFTWARE\ASIO is the 64-bit view, WOW6432Node the 32-bit one.
    let bits32 = read("SOFTWARE\\WOW6432Node\\ASIO");
    let bits64 = read("SOFTWARE\\ASIO");
    let mut names: Vec<String> = bits32.iter().chain(bits64.iter()).cloned().collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
        .into_iter()
        .map(|name| AsioDriver {
            has_32bit: bits32.iter().any(|n| n.eq_ignore_ascii_case(&name)),
            has_64bit: bits64.iter().any(|n| n.eq_ignore_ascii_case(&name)),
            name,
        })
        .collect()
}

#[cfg(not(windows))]
pub fn asio_drivers() -> Vec<AsioDriver> {
    Vec::new()
}

/// ASIO drivers the game can see: the Windows registry on Windows, the
/// registry of the game's Proton prefix (wineasio) elsewhere.
pub fn asio_drivers_for(game_dir: Option<&Path>) -> Vec<AsioDriver> {
    if cfg!(windows) {
        return asio_drivers();
    }
    game_dir
        .and_then(crate::steam::proton_prefix)
        .map(|pfx| crate::linux::prefix_asio_drivers(&pfx))
        .unwrap_or_default()
}

/// The Windows default playback device, i.e. where "Windows" output mode plays.
#[derive(Debug, Clone, Serialize)]
pub struct OutputDevice {
    pub name: String,
    /// The hardware behind the endpoint, e.g. "WH-1000XM3".
    pub device: Option<String>,
    pub bluetooth: bool,
}

#[cfg(windows)]
pub fn default_output() -> Option<OutputDevice> {
    use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
    use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree};
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::types::FromRegValue;

    let id = unsafe {
        // Fails harmlessly if the thread already uses another COM mode.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let raw = device.GetId().ok()?;
        let id = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as _));
        id?
    };
    // "{0.0.0.00000000}.{guid}": the guid names the endpoint's registry key.
    let guid = id.rsplit('.').next()?;
    let props = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{guid}\Properties"))
        .ok()?;
    let text = |key: &str| props.get_value::<String, _>(key).ok();
    let bluetooth = props
        .enum_values()
        .filter_map(|v| v.ok())
        .filter_map(|(_, value)| String::from_reg_value(&value).ok())
        .any(|s| s.contains("BTHENUM") || s.contains("BTHLE"));
    Some(OutputDevice {
        name: text("{a45c254e-df1c-4efd-8020-67d146a850e0},2").unwrap_or_else(|| crate::tr!("audio.unknownDevice")),
        device: text("{b3f8fa53-0004-438e-9003-51a46e139bfc},6"),
        bluetooth,
    })
}

#[cfg(not(windows))]
pub fn default_output() -> Option<OutputDevice> {
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Game sound through the interface: lowest latency.
    Asio,
    /// Game sound through the Windows default device, e.g. Bluetooth headphones.
    Windows,
    /// RS_ASIO asks on every game start.
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub profile_id: Option<String>,
    pub driver: String,
    pub output_mode: OutputMode,
    #[serde(default)]
    pub output_only: bool,
    pub output_base_channel: u32,
    pub alt_base_channel: Option<u32>,
    pub input_channel: u32,
    pub second_input_channel: Option<u32>,
    pub buffer_size_mode: String,
    pub custom_buffer_size: Option<u32>,
    pub enable_ref_count_hack: Option<bool>,
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

pub fn render_rs_asio_ini(template: &str, cfg: &AudioConfig) -> String {
    let mut doc = IniDoc::parse(template);
    let wasapi_outputs = match cfg.output_mode {
        OutputMode::Asio => "0",
        OutputMode::Windows => "1",
        OutputMode::Ask => "-1",
    };
    doc.set("Config", "EnableWasapiOutputs", wasapi_outputs);
    doc.set("Config", "EnableWasapiInputs", if cfg.output_only { "1" } else { "0" });
    doc.set("Config", "EnableAsio", "1");

    doc.set("Asio", "BufferSizeMode", &cfg.buffer_size_mode);
    doc.set("Asio", "CustomBufferSize", &opt(cfg.custom_buffer_size));

    let hack = opt(cfg.enable_ref_count_hack.map(u8::from));
    let output_driver = if cfg.output_mode == OutputMode::Windows { "" } else { cfg.driver.as_str() };
    doc.set("Asio.Output", "Driver", output_driver);
    doc.set("Asio.Output", "BaseChannel", &cfg.output_base_channel.to_string());
    doc.set("Asio.Output", "AltBaseChannel", &opt(cfg.alt_base_channel));
    doc.set("Asio.Output", "EnableRefCountHack", &hack);

    let input_driver = if cfg.output_only { "" } else { cfg.driver.as_str() };
    doc.set("Asio.Input.0", "Driver", input_driver);
    doc.set("Asio.Input.0", "Channel", &cfg.input_channel.to_string());
    doc.set("Asio.Input.0", "EnableRefCountHack", &hack);

    let second_driver = if cfg.second_input_channel.is_some() { input_driver } else { "" };
    doc.set("Asio.Input.1", "Driver", second_driver);
    doc.set("Asio.Input.1", "Channel", &opt(cfg.second_input_channel.or(Some(1))));
    doc.set("Asio.Input.1", "EnableRefCountHack", &hack);
    doc.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_follow_the_language() {
        let find = |lang| builtin_profiles(lang).unwrap().into_iter().find(|p| p.id == "asus-strix-soar").unwrap();
        let (de, en) = (find(Lang::De), find(Lang::En));
        assert_eq!(de.name, "Asus Strix Soar (Soundkarte)");
        assert!(de.notes.unwrap().starts_with("Nur Ausgabe"));
        assert_eq!(en.name, "Asus Strix Soar (sound card)");
        assert!(en.notes.unwrap().starts_with("Output only"));
        assert!(en.name_en.is_none() && en.notes_en.is_none());
        // Every German note has an English one.
        for p in serde_json::from_str::<Vec<DeviceProfile>>(BUILTIN_PROFILES).unwrap() {
            assert_eq!(p.notes.is_some(), p.notes_en.is_some(), "{}", p.id);
        }
    }

    fn scarlett(mode: OutputMode) -> AudioConfig {
        AudioConfig {
            profile_id: Some("focusrite-scarlett-solo-gen2-3".into()),
            driver: "Focusrite USB ASIO".into(),
            output_mode: mode,
            output_only: false,
            output_base_channel: 0,
            alt_base_channel: None,
            input_channel: 1,
            second_input_channel: None,
            buffer_size_mode: "custom".into(),
            custom_buffer_size: Some(48),
            enable_ref_count_hack: None,
        }
    }

    #[test]
    fn builtin_profiles_parse() {
        let profiles = list_profiles(Path::new("does-not-exist")).unwrap();
        assert!(profiles.iter().any(|p| p.id == "focusrite-scarlett-solo-gen4"));
    }

    #[test]
    fn asio_mode_matches_rs_asio_docs() {
        let doc = IniDoc::parse(&render_rs_asio_ini(RS_ASIO_INI_TEMPLATE, &scarlett(OutputMode::Asio)));
        assert_eq!(doc.get("Config", "EnableWasapiOutputs").as_deref(), Some("0"));
        assert_eq!(doc.get("Asio.Output", "Driver").as_deref(), Some("Focusrite USB ASIO"));
        assert_eq!(doc.get("Asio.Input.0", "Channel").as_deref(), Some("1"));
        assert_eq!(doc.get("Asio", "CustomBufferSize").as_deref(), Some("48"));
        assert_eq!(doc.get("Asio.Input.1", "Driver").as_deref(), Some(""));
    }

    #[test]
    fn windows_output_clears_asio_output_driver() {
        let doc = IniDoc::parse(&render_rs_asio_ini(RS_ASIO_INI_TEMPLATE, &scarlett(OutputMode::Windows)));
        assert_eq!(doc.get("Config", "EnableWasapiOutputs").as_deref(), Some("1"));
        assert_eq!(doc.get("Asio.Output", "Driver").as_deref(), Some(""));
        assert_eq!(doc.get("Asio.Input.0", "Driver").as_deref(), Some("Focusrite USB ASIO"));
    }
}
