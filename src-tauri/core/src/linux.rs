//! Rocksmith under Proton (Linux, Steam Deck): wineasio in the game's prefix
//! and the Steam launch options that make Wine load the mod DLLs. The parsers
//! are plain text functions so they are tested on every platform; Steam files
//! are only ever read, never written.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::audio::AsioDriver;
use crate::steam::{self, ROCKSMITH_APP_ID};

/// DLLs that Wine must load from the game folder instead of its builtins,
/// in the order they appear in the launch options.
const DLL_OVERRIDES: &[(&str, &str)] = &[
    ("avrt", "avrt.dll"),           // RS_ASIO
    ("dinput8", "dinput8.dll"),     // GuitarStar's DPI fix (no effect under Wine, but harmless)
    ("xinput1_3", "xinput1_3.dll"), // RSMods
    ("D3DX9_42", "D3DX9_42.dll"),   // CustomsForge CDLC enabler
];

/// ASIO drivers registered in a Wine registry file (`pfx/system.reg`).
/// Rocksmith is 32-bit, so in a 64-bit prefix only the Wow6432Node view counts.
pub fn parse_registry_asio(system_reg: &str) -> Vec<AsioDriver> {
    let win64 = !system_reg.lines().any(|l| l.trim().eq_ignore_ascii_case("#arch=win32"));
    let mut bits32: Vec<String> = Vec::new();
    let mut bits64: Vec<String> = Vec::new();
    for line in system_reg.lines() {
        let Some(rest) = line.strip_prefix('[') else { continue };
        let Some(end) = rest.find(']') else { continue };
        // Wine doubles the backslashes of key names.
        let key = rest[..end].replace("\\\\", "\\");
        let lower = key.to_ascii_lowercase();
        let (list, name) = if let Some(n) = strip_prefix_ci(&key, &lower, "software\\wow6432node\\asio\\") {
            (&mut bits32, n)
        } else if let Some(n) = strip_prefix_ci(&key, &lower, "software\\asio\\") {
            (if win64 { &mut bits64 } else { &mut bits32 }, n)
        } else {
            continue;
        };
        // Only direct children of the ASIO key are drivers.
        if !name.is_empty() && !name.contains('\\') && !list.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            list.push(name.to_string());
        }
    }
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

fn strip_prefix_ci<'a>(key: &'a str, lower: &str, prefix: &str) -> Option<&'a str> {
    lower.starts_with(prefix).then(|| &key[prefix.len()..])
}

/// ASIO drivers of a Proton prefix (`.../compatdata/221680/pfx`).
pub fn prefix_asio_drivers(pfx: &Path) -> Vec<AsioDriver> {
    fs::read_to_string(pfx.join("system.reg")).map(|t| parse_registry_asio(&t)).unwrap_or_default()
}

/// Whether the 32-bit wineasio DLL was copied into the prefix itself. When it
/// is installed into the Proton build instead, the prefix has no copy.
pub fn wineasio_dll_in_prefix(pfx: &Path) -> bool {
    let win = pfx.join("drive_c").join("windows");
    // 64-bit prefixes keep 32-bit DLLs in syswow64; 32-bit prefixes in system32.
    let dir = if win.join("syswow64").is_dir() { win.join("syswow64") } else { win.join("system32") };
    ["wineasio32.dll", "wineasio.dll"].iter().any(|f| dir.join(f).is_file())
}

/// Wine DLL names that need a native override for the mods present in `game_dir`.
pub fn required_overrides(game_dir: &Path) -> Vec<&'static str> {
    DLL_OVERRIDES
        .iter()
        .filter(|(_, file)| game_dir.join(file).is_file())
        .map(|(name, _)| *name)
        .collect()
}

/// The line for Steam → Rocksmith → Properties → Launch options.
pub fn launch_options(overrides: &[&str]) -> Option<String> {
    if overrides.is_empty() {
        return None;
    }
    let list: Vec<String> = overrides.iter().map(|d| format!("{d}=n,b")).collect();
    Some(format!("WINEDLLOVERRIDES=\"{}\" %command%", list.join(";")))
}

/// DLL names from `WINEDLLOVERRIDES=...` in a launch option line that load
/// the native DLL first (`n` or `n,b`).
fn native_overrides(options: &str) -> Vec<String> {
    let Some(pos) = options.find("WINEDLLOVERRIDES=") else { return Vec::new() };
    let rest = &options[pos + "WINEDLLOVERRIDES=".len()..];
    let value = match rest.strip_prefix(['"', '\'']) {
        Some(quoted) => quoted.split(['"', '\'']).next().unwrap_or(""),
        None => rest.split_whitespace().next().unwrap_or(""),
    };
    value
        .split(';')
        .filter_map(|entry| entry.split_once('='))
        .filter(|(_, mode)| mode.trim().starts_with('n'))
        .flat_map(|(names, _)| names.split(',').map(|n| n.trim().to_string()))
        .collect()
}

/// Overrides from `required` that the user's launch options lack.
pub fn missing_overrides(current: &str, required: &[&str]) -> Vec<String> {
    let set = native_overrides(current);
    let mut missing: Vec<String> = required
        .iter()
        .filter(|r| !set.iter().any(|s| s.eq_ignore_ascii_case(r)))
        .map(|r| r.to_string())
        .collect();
    if !required.is_empty() && !current.contains("%command%") {
        missing.push("%command%".into());
    }
    missing
}

enum Token {
    Str(String),
    Open,
    Close,
}

fn vdf_tokens(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => break,
                        },
                        _ => s.push(c),
                    }
                }
                tokens.push(Token::Str(s));
            }
            _ => {}
        }
    }
    tokens
}

/// `LaunchOptions` of `app_id` from a Steam `localconfig.vdf`
/// (`UserLocalConfigStore/Software/Valve/Steam/apps/<id>/LaunchOptions`).
pub fn parse_launch_options(localconfig: &str, app_id: &str) -> Option<String> {
    let mut path: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    for token in vdf_tokens(localconfig) {
        match token {
            Token::Open => path.push(pending.take().unwrap_or_default()),
            Token::Close => {
                path.pop();
                pending = None;
            }
            Token::Str(s) => match pending.take() {
                None => pending = Some(s),
                Some(key) => {
                    let n = path.len();
                    if key.eq_ignore_ascii_case("LaunchOptions")
                        && n >= 2
                        && path[n - 1] == app_id
                        && path[n - 2].eq_ignore_ascii_case("apps")
                    {
                        return Some(s);
                    }
                }
            },
        }
    }
    None
}

/// Steam roots that may hold `userdata`: the known install locations plus
/// the library that contains the game (often the Steam root itself).
fn userdata_roots(game_dir: &Path) -> Vec<PathBuf> {
    let mut roots = steam::steam_roots();
    if let Some(lib) = game_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
        roots.push(lib.to_path_buf());
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for r in roots {
        let c = fs::canonicalize(&r).unwrap_or(r);
        if c.join("userdata").is_dir() && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// What Steam stores for the game's launch options. `None` when no
/// `localconfig.vdf` was found at all; `Some(None)` when Rocksmith has none.
/// With several Steam accounts the most recently written file wins.
pub fn current_launch_options(game_dir: &Path) -> Option<Option<String>> {
    let mut newest: Option<(SystemTime, Option<String>)> = None;
    for root in userdata_roots(game_dir) {
        let Ok(users) = fs::read_dir(root.join("userdata")) else { continue };
        for user in users.flatten() {
            let file = user.path().join("config").join("localconfig.vdf");
            let Ok(text) = fs::read_to_string(&file) else { continue };
            let modified = fs::metadata(&file).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(t, _)| modified > *t) {
                newest = Some((modified, parse_launch_options(&text, ROCKSMITH_APP_ID)));
            }
        }
    }
    newest.map(|(_, opts)| opts)
}

#[derive(Debug, Clone, Serialize)]
pub struct LinuxSetupStatus {
    pub proton_prefix: Option<String>,
    pub asio_drivers: Vec<AsioDriver>,
    /// wineasio is registered for 32-bit programs in the prefix.
    pub wineasio_registered: bool,
    pub wineasio_dll_in_prefix: bool,
    /// Launch option line for the mods in the game folder; `None` if none need one.
    pub launch_options: Option<String>,
    /// False when no Steam `localconfig.vdf` could be read.
    pub launch_options_checked: bool,
    pub current_launch_options: Option<String>,
    /// Overrides (or `%command%`) missing from the current launch options.
    pub missing: Vec<String>,
}

pub fn setup_status(game_dir: &Path) -> LinuxSetupStatus {
    let pfx = steam::proton_prefix(game_dir);
    let asio_drivers = pfx.as_deref().map(prefix_asio_drivers).unwrap_or_default();
    let required = required_overrides(game_dir);
    let current = current_launch_options(game_dir);
    let current_text = current.clone().flatten().unwrap_or_default();
    LinuxSetupStatus {
        proton_prefix: pfx.as_ref().map(|p| p.to_string_lossy().into_owned()),
        wineasio_registered: asio_drivers.iter().any(|d| d.has_32bit && d.name.to_lowercase().contains("wineasio")),
        wineasio_dll_in_prefix: pfx.as_deref().is_some_and(wineasio_dll_in_prefix),
        asio_drivers,
        launch_options: launch_options(&required),
        launch_options_checked: current.is_some(),
        missing: missing_overrides(&current_text, &required),
        current_launch_options: current.flatten(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM_REG_WIN64: &str = r#"WINE REGISTRY Version 2
;; All keys relative to \\Machine

#arch=win64

[Software\\ASIO] 1717171717
#time=1dab1c2d3e4f5a6

[Software\\ASIO\\wineasio] 1717171717
#time=1dab1c2d3e4f5a6
"CLSID"="{48D0C522-BFCC-45CC-8B84-17F25F33E6E8}"
"Description"="wineasio-driver"

[Software\\Classes\\CLSID\\{48D0C522-BFCC-45CC-8B84-17F25F33E6E8}\\InprocServer32] 1717171717
#time=1dab1c2d3e4f5a6
@="C:\\windows\\system32\\wineasio64.dll"
"ThreadingModel"="Apartment"

[Software\\Wow6432Node\\ASIO\\wineasio] 1717171718
#time=1dab1c2d3e4f5a7
"CLSID"="{48D0C522-BFCC-45CC-8B84-17F25F33E6E8}"
"Description"="wineasio-driver"

[Software\\Wow6432Node\\ASIO\\wineasio\\Settings] 1717171718
"Fixed buffersize"=dword:00000001
"#;

    #[test]
    fn wineasio_in_64bit_prefix() {
        let d = parse_registry_asio(SYSTEM_REG_WIN64);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "wineasio");
        assert!(d[0].has_32bit && d[0].has_64bit);
    }

    #[test]
    fn only_64bit_registration_is_not_enough() {
        let reg = "#arch=win64\n\n[Software\\\\ASIO\\\\wineasio] 1717171717\n\"CLSID\"=\"{x}\"\n";
        let d = parse_registry_asio(reg);
        assert_eq!(d.len(), 1);
        assert!(!d[0].has_32bit && d[0].has_64bit);
    }

    #[test]
    fn plain_asio_key_is_32bit_in_32bit_prefix() {
        let reg = "WINE REGISTRY Version 2\n#arch=win32\n\n[Software\\\\ASIO\\\\wineasio] 1\n";
        let d = parse_registry_asio(reg);
        assert!(d[0].has_32bit && !d[0].has_64bit);
    }

    #[test]
    fn no_asio_in_fresh_prefix() {
        assert!(parse_registry_asio("WINE REGISTRY Version 2\n#arch=win64\n\n[Software\\\\Wine] 1\n").is_empty());
    }

    #[test]
    fn builds_launch_options() {
        assert_eq!(launch_options(&[]), None);
        assert_eq!(
            launch_options(&["avrt", "xinput1_3", "D3DX9_42"]).unwrap(),
            r#"WINEDLLOVERRIDES="avrt=n,b;xinput1_3=n,b;D3DX9_42=n,b" %command%"#
        );
    }

    #[test]
    fn overrides_follow_game_folder() {
        let dir = std::env::temp_dir().join(format!("guitarstar-test-linux-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for f in ["avrt.dll", "RS_ASIO.dll", "D3DX9_42.dll"] {
            fs::write(dir.join(f), b"").unwrap();
        }
        assert_eq!(required_overrides(&dir), vec!["avrt", "D3DX9_42"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn detects_missing_overrides() {
        let req = ["avrt", "xinput1_3"];
        assert!(missing_overrides(r#"WINEDLLOVERRIDES="avrt=n,b;xinput1_3=n,b" %command%"#, &req).is_empty());
        assert!(missing_overrides(r#"PIPEWIRE_LATENCY=256/48000 WINEDLLOVERRIDES="xinput1_3,AVRT=n" %command%"#, &req).is_empty());
        assert_eq!(missing_overrides("WINEDLLOVERRIDES=avrt=n,b %command%", &req), vec!["xinput1_3"]);
        assert_eq!(missing_overrides(r#"WINEDLLOVERRIDES="avrt=b;xinput1_3=n,b" %command%"#, &req), vec!["avrt"]);
        assert_eq!(missing_overrides("", &req), vec!["avrt", "xinput1_3", "%command%"]);
        assert!(missing_overrides("", &[]).is_empty());
    }

    #[test]
    fn reads_launch_options_from_localconfig() {
        let vdf = r#""UserLocalConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"221680"
					{
						"LastPlayed"		"1717171717"
						"LaunchOptions"		"WINEDLLOVERRIDES=\"avrt=n,b\" %command%"
					}
					"440"
					{
						"LaunchOptions"		"-novid"
					}
				}
			}
		}
	}
	"WebStorage"
	{
		"221680"		"{\"LaunchOptions\":\"x\"}"
	}
}
"#;
        assert_eq!(
            parse_launch_options(vdf, "221680").as_deref(),
            Some(r#"WINEDLLOVERRIDES="avrt=n,b" %command%"#)
        );
        assert_eq!(parse_launch_options(vdf, "440").as_deref(), Some("-novid"));
        assert_eq!(parse_launch_options(vdf, "570"), None);
    }
}
