//! Every word the v2 window shows, read from `assets/textes.fr.toml`.
//!
//! One flat table, keyed `screen.thing`. Two tests hold it to the code:
//! every key the sources ask for exists, and every key in the file is
//! asked for somewhere — a dead line in the file is a sentence nobody
//! will ever proofread against the screen it no longer appears on.

use std::collections::HashMap;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("../assets/textes.fr.toml");

fn table() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| parse(EMBEDDED))
}

/// Flatten `[section] key = "…"` into `section.key`.
fn parse(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(value) = text.parse::<toml::Table>() else {
        return out;
    };
    for (section, inner) in value {
        match inner {
            toml::Value::Table(t) => {
                for (k, v) in t {
                    if let toml::Value::String(s) = v {
                        out.insert(format!("{section}.{k}"), s);
                    }
                }
            }
            toml::Value::String(s) => {
                out.insert(section, s);
            }
            _ => {}
        }
    }
    out
}

/// The text under `key`, or the key itself when it is missing (a test
/// makes sure none is).
pub fn t(key: &'static str) -> &'static str {
    table().get(key).map(String::as_str).unwrap_or(key)
}

/// The text under `key` with each `{}` replaced, in order, by `args`.
pub fn tf(key: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = t(key);
    let mut args = args.iter();
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// `count` with the singular or plural form: the key's text holds both,
/// split by `|` (« {} dossier|{} dossiers »). Zero takes the singular, as
/// French does.
pub fn tn(key: &'static str, count: usize) -> String {
    let text = t(key);
    let form = match text.split_once('|') {
        Some((one, _)) if count <= 1 => one,
        Some((_, many)) => many,
        None => text,
    };
    form.replacen("{}", &count.to_string(), 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                sources(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                out.push((
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
    }

    /// Every `t("…")`, `tf("…"` and `tn("…"` key in the sources.
    fn keys_used() -> Vec<(String, String)> {
        let mut files = Vec::new();
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut keys = Vec::new();
        for (name, text) in &files {
            for opener in ["t(", "tf(", "tn("] {
                let mut rest = text.as_str();
                while let Some(at) = rest.find(opener) {
                    let before = rest[..at].chars().last();
                    rest = &rest[at + opener.len()..];
                    // `get(`, `set(` and friends end in « t( » too.
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    // rustfmt may put the key on the next line.
                    let Some(quoted) = rest.trim_start().strip_prefix('"') else {
                        continue;
                    };
                    if let Some(end) = quoted.find('"') {
                        let rest = quoted;
                        let key = &rest[..end];
                        if key.contains('.') && !key.contains(' ') {
                            keys.push((name.clone(), key.to_owned()));
                        }
                    }
                }
            }
        }
        keys
    }

    #[test]
    fn every_key_the_code_asks_for_is_written() {
        let missing: Vec<_> = keys_used()
            .into_iter()
            .filter(|(_, k)| !table().contains_key(k))
            .collect();
        assert!(
            missing.is_empty(),
            "absentes de textes.fr.toml : {missing:?}"
        );
    }

    #[test]
    fn every_written_key_is_used() {
        let used: std::collections::HashSet<String> =
            keys_used().into_iter().map(|(_, k)| k).collect();
        let mut dead: Vec<_> = table().keys().filter(|k| !used.contains(*k)).collect();
        dead.sort();
        assert!(dead.is_empty(), "jamais affichées : {dead:?}");
    }

    #[test]
    fn the_file_parses_and_is_not_empty() {
        assert!(EMBEDDED.parse::<toml::Table>().is_ok());
        assert!(table().len() > 20);
    }

    /// The bundled faces have no glyph for arrows or the narrow
    /// no-break space: a box would show where the character was.
    #[test]
    fn no_character_the_faces_cannot_draw() {
        for (k, v) in table() {
            for c in v.chars() {
                assert!(
                    !matches!(c, '\u{2190}'..='\u{21ff}' | '\u{202f}'),
                    "{k} : caractère {c:?} sans glyphe"
                );
            }
        }
    }

    #[test]
    fn placeholders_fill_in_order() {
        assert_eq!(parse("[a]\nb = \"x\"").get("a.b").unwrap(), "x");
        let s = tf("app.find_drug", &[&"Doliprane"]);
        assert!(s.ends_with("Doliprane"), "{s}");
        assert_eq!(tn("patients.count", 1), "1 patient");
        assert_eq!(tn("patients.count", 3), "3 patients");
    }
}
