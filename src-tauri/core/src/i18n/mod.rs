//! Texts shown to the user. The UI tells the backend its language
//! (`set_lang`); every message is produced in that language. Each language
//! has its own table in this folder; German is the reference. Missing keys
//! fall back to English, then German. Placeholders are written `{name}`, like
//! in the frontend dictionary.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

mod ar;
mod bn;
mod de;
mod en;
mod es;
mod fr;
mod hi;
mod pt;
mod ru;
mod ur;
mod zh;

/// Same codes as `LANGUAGES` in `src/lib/i18n.svelte.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    De,
    En,
    Zh,
    Hi,
    Es,
    Fr,
    Ar,
    Bn,
    Pt,
    Ru,
    Ur,
}

pub const ALL: [Lang; 11] = [
    Lang::De,
    Lang::En,
    Lang::Zh,
    Lang::Hi,
    Lang::Es,
    Lang::Fr,
    Lang::Ar,
    Lang::Bn,
    Lang::Pt,
    Lang::Ru,
    Lang::Ur,
];

impl Lang {
    pub fn parse(code: &str) -> Option<Lang> {
        ALL.into_iter().find(|l| l.code() == code)
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::De => "de",
            Lang::En => "en",
            Lang::Zh => "zh",
            Lang::Hi => "hi",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::Ar => "ar",
            Lang::Bn => "bn",
            Lang::Pt => "pt",
            Lang::Ru => "ru",
            Lang::Ur => "ur",
        }
    }
}

/// English until the UI (or the saved app state) says otherwise, like the UI's own default.
static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    let current = CURRENT.load(Ordering::Relaxed);
    ALL.into_iter().find(|l| *l as u8 == current).unwrap_or(Lang::En)
}

fn table(lang: Lang) -> &'static [(&'static str, &'static str)] {
    match lang {
        Lang::De => de::TABLE,
        Lang::En => en::TABLE,
        Lang::Zh => zh::TABLE,
        Lang::Hi => hi::TABLE,
        Lang::Es => es::TABLE,
        Lang::Fr => fr::TABLE,
        Lang::Ar => ar::TABLE,
        Lang::Bn => bn::TABLE,
        Lang::Pt => pt::TABLE,
        Lang::Ru => ru::TABLE,
        Lang::Ur => ur::TABLE,
    }
}

fn lookup(lang: Lang, key: &str) -> Option<&'static str> {
    table(lang).iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Whether `lang`'s own table has `key` (without falling back).
pub fn has_key(lang: Lang, key: &str) -> bool {
    lookup(lang, key).is_some()
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

/// `key` in `lang`, falling back to English, German and then the key itself.
pub fn tr_in(lang: Lang, key: &str, args: &[(&str, String)]) -> String {
    let text = lookup(lang, key)
        .or_else(|| lookup(Lang::En, key))
        .or_else(|| lookup(Lang::De, key))
        .unwrap_or(key);
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

/// Fills `template`'s placeholders from `text`, if `text` has its shape.
/// Each placeholder takes the text up to the next literal part.
fn match_template(template: &str, text: &str) -> Option<BTreeMap<String, String>> {
    let mut args = BTreeMap::new();
    let mut rest = text;
    let mut tpl = template;
    // Literal prefix.
    let first = tpl.find('{').unwrap_or(tpl.len());
    rest = rest.strip_prefix(&tpl[..first])?;
    tpl = &tpl[first..];
    while let Some(after) = tpl.strip_prefix('{') {
        let end = after.find('}')?;
        let name = &after[..end];
        tpl = &after[end + 1..];
        let literal_end = tpl.find('{').unwrap_or(tpl.len());
        let literal = &tpl[..literal_end];
        let value_end = if literal.is_empty() {
            if literal_end == tpl.len() { rest.len() } else { return None }
        } else {
            rest.find(literal)?
        };
        let value = &rest[..value_end];
        if value.is_empty() {
            return None;
        }
        args.insert(name.to_string(), value.to_string());
        rest = &rest[value_end + literal.len()..];
        tpl = &tpl[literal_end..];
    }
    rest.is_empty().then_some(args)
}

/// Turns a label stored as plain German text (written before labels got
/// keys) back into a translatable label, so old history entries follow the
/// UI language too. Component names are mapped back to their ids.
pub fn legacy_label(text: &str) -> Option<Label> {
    let mut candidates: Vec<&(&'static str, &'static str)> =
        de::TABLE.iter().filter(|(k, _)| k.starts_with("labels.")).collect();
    // Templates with fewer placeholders are more specific; try them first.
    candidates.sort_by_key(|(_, t)| t.matches('{').count());
    for (key, template) in candidates {
        let Some(mut args) = match_template(template, text) else { continue };
        if let Some(name) = args.get("component").cloned() {
            let id = crate::components::COMPONENTS
                .iter()
                .map(|c| c.id)
                .find(|id| lookup(Lang::De, &format!("component.{id}.name")) == Some(name.as_str()));
            match id {
                Some(id) => {
                    args.insert("component".into(), id.to_string());
                }
                None => continue,
            }
        }
        return Some(Label::Key { key, args });
    }
    None
}

/// Display name of a component (see `components::COMPONENTS`) in the current language.
pub fn component_name(id: &str) -> String {
    let key = format!("component.{id}.name");
    if lookup(Lang::De, &key).is_some() { tr(&key) } else { id.to_string() }
}


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

    /// English must be complete; every other language is either not started
    /// yet (empty table, falls back to English) or complete as well.
    #[test]
    fn all_languages_have_the_same_keys_and_placeholders() {
        let de = keys(de::TABLE);
        for lang in ALL.into_iter().filter(|l| *l != Lang::De) {
            let table = table(lang);
            if table.is_empty() && lang != Lang::En {
                continue;
            }
            let other = keys(table);
            let code = lang.code();
            assert_eq!(de.difference(&other).collect::<Vec<_>>(), Vec::<&&str>::new(), "missing in {code}");
            assert_eq!(other.difference(&de).collect::<Vec<_>>(), Vec::<&&str>::new(), "unknown keys in {code}");
            for key in &de {
                let (d, o) = (lookup(Lang::De, key).unwrap(), lookup(lang, key).unwrap());
                assert_eq!(placeholders(d), placeholders(o), "placeholders differ for {key} in {code}");
                assert!(!o.trim().is_empty(), "empty text for {key} in {code}");
            }
        }
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(tr_in(Lang::En, "cdlc.notFound", &[("name", "x".into())]), "x was not found.");
        if zh::TABLE.is_empty() {
            assert_eq!(tr_in(Lang::Zh, "cdlc.notFound", &[("name", "x".into())]), "x was not found.");
        }
        for lang in ALL {
            assert_eq!(Lang::parse(lang.code()), Some(lang));
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
        assert_eq!(Lang::parse("xx"), None);
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
    fn old_german_labels_become_translatable() {
        let label = |text: &str| legacy_label(text).map(|l| (l.key().unwrap(), l.args().unwrap_or_default()));
        let args = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(label("Audio-Einstellungen geändert"), Some(("labels.audioSettings".into(), BTreeMap::new())));
        assert_eq!(
            label("RS_ASIO v0.7.5 installiert"),
            Some(("labels.componentInstalled".into(), args(&[("component", "rs_asio"), ("version", "v0.7.5")])))
        );
        assert_eq!(
            label("Skalierungs-Fix 1.1.0 → 1.2.0"),
            Some(("labels.componentSwitched".into(), args(&[("component", "dpi_fix"), ("from", "1.1.0"), ("to", "1.2.0")])))
        );
        assert_eq!(
            label("RSMods RSModsInstaller-v1.2.8.4_OnCommit (vor der Installation)"),
            Some(("labels.rsmodsBefore".into(), args(&[("version", "RSModsInstaller-v1.2.8.4_OnCommit")])))
        );
        assert_eq!(label("Minimieren bei Fokusverlust aus"), Some(("labels.minimizeOff".into(), BTreeMap::new())));
        assert_eq!(label("Etwas ganz anderes"), None);
        assert_eq!(label("Unbekannt 1.0 installiert"), None);
        assert_eq!(match_template("{a} → {b}", "x → y → z"), Some(args(&[("a", "x"), ("b", "y → z")])));
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
        // The i18n folder itself is a directory, so it is skipped by the extension filter.
        for file in files.iter().filter(|p| p.extension().is_some_and(|e| e == "rs")) {
            let source = std::fs::read_to_string(file).unwrap();
            for marker in ["tr!(\"", "Label::new(\""] {
                for key in literal_keys_after(&source, marker) {
                    assert!(lookup(Lang::De, key).is_some(), "{}: unknown key {key}", file.display());
                    used += 1;
                }
            }
        }
        assert!(used > 30, "only {used} keys found; did the scan break?");

        // Label keys are shown by the frontend, so its German and English
        // dictionaries need them (other languages are checked by the type checker).
        for code in ["de", "en"] {
            let frontend = std::fs::read_to_string(root.join(format!("../../src/lib/i18n/{code}.ts"))).unwrap();
            let start = frontend.find("\n  labels: {").expect("labels section");
            let section = &frontend[start..];
            let section = &section[..section.find("\n  },").expect("section end")];
            for (key, _) in de::TABLE.iter().filter(|(k, _)| k.starts_with("labels.")) {
                let leaf = key.trim_start_matches("labels.");
                assert!(section.contains(&format!("\n    {leaf}: ")), "{key} is missing in src/lib/i18n/{code}.ts");
            }
        }
    }
}
