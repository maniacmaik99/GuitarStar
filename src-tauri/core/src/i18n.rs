//! Texts shown to the user, in German and English. The UI tells the backend
//! its language (`set_lang`); every message is produced in that language.
//! Placeholders are written `{name}`, like in the frontend dictionary.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    De,
    En,
}

impl Lang {
    pub fn parse(code: &str) -> Option<Lang> {
        match code {
            "de" => Some(Lang::De),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// English until the UI (or the saved app state) says otherwise, like the UI's own default.
static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::De as u8 { Lang::De } else { Lang::En }
}

fn table(lang: Lang) -> &'static [(&'static str, &'static str)] {
    match lang {
        Lang::De => DE,
        Lang::En => EN,
    }
}

fn lookup(lang: Lang, key: &str) -> Option<&'static str> {
    table(lang).iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Replaces `{name}` with the value of `name`; unknown placeholders stay as they are.
fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let value = after
            .find('}')
            .and_then(|end| args.iter().find(|(n, _)| *n == &after[..end]).map(|(_, v)| (end, v)));
        match value {
            Some((end, v)) => {
                out.push_str(v);
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `key` in `lang`, falling back to German and then to the key itself.
pub fn tr_in(lang: Lang, key: &str, args: &[(&str, String)]) -> String {
    let text = lookup(lang, key).or_else(|| lookup(Lang::De, key)).unwrap_or(key);
    fill(text, args)
}

/// `key` in the current language.
pub fn tr(key: &str) -> String {
    tr_in(lang(), key, &[])
}

/// `key` in the current language with its placeholders filled.
pub fn tr_args(key: &str, args: &[(&str, String)]) -> String {
    tr_in(lang(), key, args)
}

/// `tr!("key")` or `tr!("key", name = value, …)`; values only need `Display`.
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::i18n::tr($key)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::tr_args($key, &[$((stringify!($name), ($value).to_string())),+])
    };
}

/// A label stored on disk (history, save backups). Besides the text in the
/// language at creation time, the key and arguments are stored so the UI can
/// show it in whatever language is current later. Keys starting with
/// `labels.` exist in the frontend dictionary under the same name. An argument
/// called `component` holds a component id and is shown as its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    Key { key: &'static str, args: BTreeMap<String, String> },
    /// Text that already is in its final form, e.g. typed by the user.
    Plain(String),
}

impl Label {
    pub fn new(key: &'static str) -> Self {
        Label::Key { key, args: BTreeMap::new() }
    }

    pub fn arg(mut self, name: &str, value: impl ToString) -> Self {
        if let Label::Key { args, .. } = &mut self {
            args.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// The text in the current language.
    pub fn text(&self) -> String {
        match self {
            Label::Plain(text) => text.clone(),
            Label::Key { key, args } => {
                let args: Vec<(&str, String)> = args
                    .iter()
                    .map(|(name, value)| {
                        let value = if name == "component" { component_name(value) } else { value.clone() };
                        (name.as_str(), value)
                    })
                    .collect();
                tr_args(key, &args)
            }
        }
    }

    pub fn key(&self) -> Option<String> {
        match self {
            Label::Key { key, .. } => Some(key.to_string()),
            Label::Plain(_) => None,
        }
    }

    /// `None` when there are no arguments.
    pub fn args(&self) -> Option<BTreeMap<String, String>> {
        match self {
            Label::Key { args, .. } if !args.is_empty() => Some(args.clone()),
            _ => None,
        }
    }
}

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Label::Plain(text.to_string())
    }
}

/// Display name of a component (see `components::COMPONENTS`) in the current language.
pub fn component_name(id: &str) -> String {
    let key = format!("component.{id}.name");
    if lookup(Lang::De, &key).is_some() { tr(&key) } else { id.to_string() }
}

/// German, the reference: every key must exist here and in `EN`.
const DE: &[(&str, &str)] = &[
    // Errors from libraries, with their own text appended.
    ("error.io", "Datei- oder Ordnerfehler: {error}"),
    ("error.http", "Netzwerkfehler: {error}"),
    ("error.json", "Ungültige Daten: {error}"),
    ("error.zip", "Archivfehler: {error}"),
    // Components.
    ("component.rs_asio.name", "RS_ASIO"),
    (
        "component.rs_asio.description",
        "Leitet Ein- und Ausgabe des Spiels auf ein Audio-Interface (ASIO) um: niedrige Latenz, kein Rocksmith-Kabel nötig.",
    ),
    ("component.dpi_fix.name", "Skalierungs-Fix"),
    (
        "component.dpi_fix.description",
        "Eigene dinput8.dll von GuitarStar: Rocksmith übernimmt die Windows-Skalierung selbst, damit das Bild auch im (exklusiven) Vollbild genau auf den Monitor passt.",
    ),
    ("components.unknown", "Unbekannte Komponente: {id}"),
    ("components.checksumMismatch", "Die Prüfsumme von {file} stimmt nicht mit GitHub überein. Download abgebrochen."),
    ("components.fileMissing", "{name} {version} enthält die Datei {file} nicht. Diese Version wird nicht unterstützt."),
    ("components.notOnGithub", "{name} {version} wurde auf GitHub nicht gefunden."),
    // Game folder.
    ("game.exeMissing", "{exe} wurde in diesem Ordner nicht gefunden."),
    ("game.notSteam", "Das scheint nicht die Steam-Version zu sein; RS_ASIO unterstützt nur diese."),
    ("game.chooseFirst", "Bitte zuerst den Rocksmith-Spielordner auswählen."),
    ("snapshot.invalidPath", "Ungültiger Pfad im Spielordner: {path}"),
    ("snapshot.gone", "Diese Sicherung gibt es nicht mehr."),
    // Audio.
    ("audio.unknownDevice", "Unbekanntes Gerät"),
    ("audio.installRsAsioFirst", "Bitte zuerst RS_ASIO installieren."),
    ("display.installFixFirst", "Bitte zuerst den Skalierungs-Fix installieren."),
    // Save backups.
    ("saves.invalidPath", "Ungültiger Pfad in der Spielstand-Sicherung: {path}"),
    ("saves.folderMissing", "Der Spielstand-Ordner existiert nicht: {path}"),
    ("saves.backupGone", "Diese Spielstand-Sicherung gibt es nicht mehr."),
    ("saves.damaged", "Die Sicherung ist beschädigt ({file}); es wurde nichts verändert."),
    ("saves.outsideFolder", "Pfad außerhalb des Spielstand-Ordners: {path}"),
    ("saves.noneFound", "Kein Rocksmith-Spielstand gefunden. Rocksmith muss über Steam mindestens einmal gestartet worden sein."),
    ("saves.backupFailed", "Dein Rocksmith-Spielstand konnte nicht gesichert werden, deshalb wurde abgebrochen. {error}"),
    ("saves.openFolderFailed", "Der Ordner konnte nicht geöffnet werden: {error}"),
    // Custom songs.
    ("cdlc.invalidName", "Ungültiger Songname: {name}"),
    ("cdlc.macVersion", "Mac-Version; für Windows wird die Datei mit _p.psarc gebraucht"),
    ("cdlc.notASong", "keine Rocksmith-Songdatei (_p.psarc)"),
    ("cdlc.zipWithoutSong", "Archiv enthält keine Windows-Songdatei (_p.psarc)"),
    ("cdlc.notFound", "{name} wurde nicht gefunden."),
    ("cdlc.downloadCanceled", "Download abgebrochen."),
    ("cdlc.enablerNoInstaller", "CustomsForge hat keine Installationsdatei geliefert. Bitte später erneut versuchen."),
    ("installer.startFailed", "Der Installer konnte nicht gestartet werden: {error}"),
    // RSMods.
    ("rsmods.noInstaller", "Auf GitHub wurde kein RSMods-Installer gefunden."),
    ("rsmods.guiMissing", "Die RSMods-Oberfläche wurde im Spielordner nicht gefunden."),
    ("rsmods.openFailed", "RSMods konnte nicht geöffnet werden: {error}"),
    // CustomsForge view.
    ("customsforge.openFailed", "CustomsForge konnte nicht geöffnet werden: {error}"),
    ("customsforge.mainWindowMissing", "Hauptfenster nicht gefunden."),
    ("customsforge.unknownAction", "Unbekannte Aktion."),
    ("customsforge.invalidAddress", "Ungültige Adresse."),
    ("customsforge.foreignPage", "Nur Seiten von ignition4.customsforge.com können geöffnet werden."),
    ("customsforge.notOpen", "CustomsForge ist nicht geöffnet."),
    ("app.unknownLanguage", "Unbekannte Sprache: {lang}"),
    // Labels of history entries and save backups (also in the frontend dictionary).
    ("labels.gameSettings", "Spieleinstellungen geändert"),
    ("labels.audioSettings", "Audio-Einstellungen geändert"),
    ("labels.componentSwitched", "{component} {from} → {to}"),
    ("labels.componentInstalled", "{component} {version} installiert"),
    ("labels.componentRemoved", "{component} entfernt"),
    ("labels.minimizeOff", "Minimieren bei Fokusverlust aus"),
    ("labels.minimizeOn", "Minimieren bei Fokusverlust an"),
    ("labels.dllLogOn", "Diagnose-Protokolle an"),
    ("labels.dllLogOff", "Diagnose-Protokolle aus"),
    ("labels.windowsScalingOff", "Windows-Skalierung für Rocksmith aus"),
    ("labels.windowsScalingOn", "Windows-Skalierung für Rocksmith an"),
    ("labels.enablerBefore", "CDLC-Enabler (vor der Installation)"),
    ("labels.rsmodsBefore", "RSMods {version} (vor der Installation)"),
    ("labels.rsmodsRemoved", "RSMods entfernt"),
    ("labels.autoBeforeReset", "Automatisch vor dem Zurücksetzen des Spielordners"),
    ("labels.autoBeforeEnabler", "Automatisch vor CDLC-Enabler-Installation"),
    ("labels.autoBeforeRsmods", "Automatisch vor RSMods-Installation"),
    ("labels.autoBeforeRsmodsRemoval", "Automatisch vor RSMods-Entfernung"),
    ("labels.autoOnStart", "Automatisch beim Start"),
    ("labels.manual", "Von Hand gesichert"),
    ("labels.beforeRestore", "Vor Wiederherstellung"),
];

const EN: &[(&str, &str)] = &[
    ("error.io", "File or folder error: {error}"),
    ("error.http", "Network error: {error}"),
    ("error.json", "Invalid data: {error}"),
    ("error.zip", "Archive error: {error}"),
    ("component.rs_asio.name", "RS_ASIO"),
    (
        "component.rs_asio.description",
        "Routes the game's input and output to an audio interface (ASIO): low latency, no Rocksmith cable needed.",
    ),
    ("component.dpi_fix.name", "Scaling fix"),
    (
        "component.dpi_fix.description",
        "GuitarStar's own dinput8.dll: Rocksmith handles Windows scaling itself, so the image fits the monitor exactly, even in (exclusive) fullscreen.",
    ),
    ("components.unknown", "Unknown component: {id}"),
    ("components.checksumMismatch", "The checksum of {file} does not match GitHub's. Download canceled."),
    ("components.fileMissing", "{name} {version} does not contain the file {file}. This version is not supported."),
    ("components.notOnGithub", "{name} {version} was not found on GitHub."),
    ("game.exeMissing", "{exe} was not found in this folder."),
    ("game.notSteam", "This doesn't seem to be the Steam version; RS_ASIO only supports that one."),
    ("game.chooseFirst", "Please select the Rocksmith game folder first."),
    ("snapshot.invalidPath", "Invalid path in the game folder: {path}"),
    ("snapshot.gone", "This backup no longer exists."),
    ("audio.unknownDevice", "Unknown device"),
    ("audio.installRsAsioFirst", "Please install RS_ASIO first."),
    ("display.installFixFirst", "Please install the scaling fix first."),
    ("saves.invalidPath", "Invalid path in the save backup: {path}"),
    ("saves.folderMissing", "The save folder does not exist: {path}"),
    ("saves.backupGone", "This save backup no longer exists."),
    ("saves.damaged", "The backup is damaged ({file}); nothing was changed."),
    ("saves.outsideFolder", "Path outside the save folder: {path}"),
    ("saves.noneFound", "No Rocksmith save found. Rocksmith has to be started through Steam at least once."),
    ("saves.backupFailed", "Your Rocksmith save could not be backed up, so nothing was done. {error}"),
    ("saves.openFolderFailed", "The folder could not be opened: {error}"),
    ("cdlc.invalidName", "Invalid song name: {name}"),
    ("cdlc.macVersion", "Mac version; Windows needs the file ending in _p.psarc"),
    ("cdlc.notASong", "not a Rocksmith song file (_p.psarc)"),
    ("cdlc.zipWithoutSong", "archive contains no Windows song file (_p.psarc)"),
    ("cdlc.notFound", "{name} was not found."),
    ("cdlc.downloadCanceled", "Download canceled."),
    ("cdlc.enablerNoInstaller", "CustomsForge did not deliver an installer file. Please try again later."),
    ("installer.startFailed", "The installer could not be started: {error}"),
    ("rsmods.noInstaller", "No RSMods installer was found on GitHub."),
    ("rsmods.guiMissing", "The RSMods interface was not found in the game folder."),
    ("rsmods.openFailed", "RSMods could not be opened: {error}"),
    ("customsforge.openFailed", "CustomsForge could not be opened: {error}"),
    ("customsforge.mainWindowMissing", "Main window not found."),
    ("customsforge.unknownAction", "Unknown action."),
    ("customsforge.invalidAddress", "Invalid address."),
    ("customsforge.foreignPage", "Only pages from ignition4.customsforge.com can be opened."),
    ("customsforge.notOpen", "CustomsForge is not open."),
    ("app.unknownLanguage", "Unknown language: {lang}"),
    ("labels.gameSettings", "Game settings changed"),
    ("labels.audioSettings", "Audio settings changed"),
    ("labels.componentSwitched", "{component} {from} → {to}"),
    ("labels.componentInstalled", "{component} {version} installed"),
    ("labels.componentRemoved", "{component} removed"),
    ("labels.minimizeOff", "Minimize on focus loss off"),
    ("labels.minimizeOn", "Minimize on focus loss on"),
    ("labels.dllLogOn", "Diagnostic logs on"),
    ("labels.dllLogOff", "Diagnostic logs off"),
    ("labels.windowsScalingOff", "Windows scaling for Rocksmith off"),
    ("labels.windowsScalingOn", "Windows scaling for Rocksmith on"),
    ("labels.enablerBefore", "CDLC Enabler (before installation)"),
    ("labels.rsmodsBefore", "RSMods {version} (before installation)"),
    ("labels.rsmodsRemoved", "RSMods removed"),
    ("labels.autoBeforeReset", "Automatically before resetting the game folder"),
    ("labels.autoBeforeEnabler", "Automatically before installing the CDLC Enabler"),
    ("labels.autoBeforeRsmods", "Automatically before installing RSMods"),
    ("labels.autoBeforeRsmodsRemoval", "Automatically before removing RSMods"),
    ("labels.autoOnStart", "Automatically on start"),
    ("labels.manual", "Backed up manually"),
    ("labels.beforeRestore", "Before restore"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    out.insert(after[..end].to_string());
                    rest = &after[end + 1..];
                }
                None => break,
            }
        }
        out
    }

    fn keys(table: &'static [(&'static str, &'static str)]) -> BTreeSet<&'static str> {
        let mut seen = BTreeSet::new();
        for (k, _) in table {
            assert!(seen.insert(*k), "duplicate key {k}");
        }
        seen
    }

    #[test]
    fn german_and_english_have_the_same_keys_and_placeholders() {
        let (de, en) = (keys(DE), keys(EN));
        assert_eq!(de.difference(&en).collect::<Vec<_>>(), Vec::<&&str>::new(), "missing in English");
        assert_eq!(en.difference(&de).collect::<Vec<_>>(), Vec::<&&str>::new(), "missing in German");
        for key in &de {
            let (d, e) = (lookup(Lang::De, key).unwrap(), lookup(Lang::En, key).unwrap());
            assert_eq!(placeholders(d), placeholders(e), "placeholders differ for {key}");
            assert!(!e.trim().is_empty(), "empty English text for {key}");
        }
    }

    #[test]
    fn fills_placeholders() {
        let args = [("name", "RS_ASIO".to_string()), ("version", "{x}".to_string())];
        assert_eq!(fill("{name} {version} {missing} {", &args), "RS_ASIO {x} {missing} {");
        assert_eq!(
            tr_in(Lang::En, "components.notOnGithub", &args),
            "RS_ASIO {x} was not found on GitHub."
        );
        assert_eq!(tr_in(Lang::De, "cdlc.notFound", &[("name", "a_p.psarc".into())]), "a_p.psarc wurde nicht gefunden.");
        assert_eq!(tr_in(Lang::En, "no.such.key", &[]), "no.such.key");
        assert_eq!(Lang::parse("de"), Some(Lang::De));
        assert_eq!(Lang::parse("fr"), None);
    }

    #[test]
    fn labels_keep_key_and_args() {
        let l = Label::new("labels.componentSwitched").arg("component", "rs_asio").arg("from", "v1").arg("to", "v2");
        assert_eq!(l.key().as_deref(), Some("labels.componentSwitched"));
        assert_eq!(l.args().unwrap()["component"], "rs_asio");
        assert_eq!(l.text(), "RS_ASIO v1 → v2");
        assert_eq!(Label::new("labels.manual").args(), None);
        assert_eq!(Label::from("eins").key(), None);
    }

    #[test]
    fn components_have_texts() {
        for def in crate::components::COMPONENTS {
            for part in ["name", "description"] {
                let key = format!("component.{}.{part}", def.id);
                assert!(lookup(Lang::De, &key).is_some(), "missing {key}");
            }
        }
    }

    /// String literals that follow `marker` (not as the end of a longer name,
    /// like `include_str!`) in `source`.
    fn literal_keys_after<'a>(source: &'a str, marker: &str) -> Vec<&'a str> {
        source
            .match_indices(marker)
            .filter(|(i, _)| !source[..*i].ends_with(|c: char| c.is_alphanumeric() || c == '_'))
            .filter_map(|(i, _)| {
                let rest = &source[i + marker.len()..];
                rest.find('"').map(|end| &rest[..end])
            })
            .collect()
    }

    /// Every key used in the backend code exists, and every label key exists in
    /// the frontend dictionary as well.
    #[test]
    fn keys_used_in_code_exist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files: Vec<_> = std::fs::read_dir(root.join("src")).unwrap().flatten().map(|e| e.path()).collect();
        files.extend(std::fs::read_dir(root.join("../src")).unwrap().flatten().map(|e| e.path()));
        let mut used = 0;
        for file in files.iter().filter(|p| p.extension().is_some_and(|e| e == "rs")) {
            if file.ends_with("i18n.rs") {
                continue;
            }
            let source = std::fs::read_to_string(file).unwrap();
            for marker in ["tr!(\"", "Label::new(\""] {
                for key in literal_keys_after(&source, marker) {
                    assert!(lookup(Lang::De, key).is_some(), "{}: unknown key {key}", file.display());
                    used += 1;
                }
            }
        }
        assert!(used > 30, "only {used} keys found; did the scan break?");

        let frontend = std::fs::read_to_string(root.join("../../src/lib/i18n.svelte.ts")).unwrap();
        let sections: Vec<&str> = frontend
            .match_indices("\n  labels: {")
            .map(|(i, _)| {
                let rest = &frontend[i..];
                &rest[..rest.find("\n  },").expect("section end")]
            })
            .collect();
        assert_eq!(sections.len(), 2, "German and English `labels` section in the frontend dictionary");
        for (key, _) in DE.iter().filter(|(k, _)| k.starts_with("labels.")) {
            let leaf = key.trim_start_matches("labels.");
            for section in &sections {
                assert!(section.contains(&format!("\n    {leaf}: ")), "{key} is missing in the frontend dictionary");
            }
        }
    }
}
