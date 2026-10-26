//! Rocksmith is not DPI aware: with Windows display scaling above 100 % its
//! window gets stretched beyond the monitor. The fix is the per-user
//! compatibility flag "High DPI scaling performed by: Application"
//! (`HIGHDPIAWARE`), the same setting as in the exe's Properties dialog.

use std::path::Path;

use crate::error::Result;

#[cfg(windows)]
const LAYERS: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const FLAG: &str = "HIGHDPIAWARE";

/// Adds or removes `FLAG` in a Layers value such as `~ RUNASADMIN HIGHDPIAWARE`,
/// keeping any other flags. Returns `None` when no flag remains.
fn edit_layers(current: Option<&str>, enable: bool) -> Option<String> {
    let mut flags: Vec<&str> = current
        .unwrap_or("")
        .split_whitespace()
        .filter(|f| *f != "~" && !f.eq_ignore_ascii_case(FLAG))
        .collect();
    if enable {
        flags.push(FLAG);
    }
    (!flags.is_empty()).then(|| format!("~ {}", flags.join(" ")))
}

#[cfg(windows)]
pub fn dpi_override(exe: &Path) -> Option<bool> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let value: Option<String> = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(LAYERS)
        .ok()
        .and_then(|k| k.get_value(exe.to_string_lossy().as_ref()).ok());
    Some(value.is_some_and(|v| v.split_whitespace().any(|f| f.eq_ignore_ascii_case(FLAG))))
}

#[cfg(windows)]
pub fn set_dpi_override(exe: &Path, enable: bool) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(LAYERS)?;
    let name = exe.to_string_lossy();
    let current: Option<String> = key.get_value(name.as_ref()).ok();
    match edit_layers(current.as_deref(), enable) {
        Some(v) => key.set_value(name.as_ref(), &v)?,
        None if current.is_some() => key.delete_value(name.as_ref())?,
        None => {}
    }
    Ok(())
}

/// Under Proton the scaling is handled by Wine; nothing to set.
#[cfg(not(windows))]
pub fn dpi_override(_exe: &Path) -> Option<bool> {
    None
}

#[cfg(not(windows))]
pub fn set_dpi_override(_exe: &Path, _enable: bool) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_other_flags() {
        assert_eq!(edit_layers(None, true).as_deref(), Some("~ HIGHDPIAWARE"));
        assert_eq!(edit_layers(Some("~ RUNASADMIN"), true).as_deref(), Some("~ RUNASADMIN HIGHDPIAWARE"));
        assert_eq!(edit_layers(Some("~ RUNASADMIN HIGHDPIAWARE"), false).as_deref(), Some("~ RUNASADMIN"));
        assert_eq!(edit_layers(Some("~ HIGHDPIAWARE"), false), None);
        assert_eq!(edit_layers(Some("~ HIGHDPIAWARE"), true).as_deref(), Some("~ HIGHDPIAWARE"));
    }
}
