//! User-facing UI strings, loaded from the embedded French TOML
//! (`assets/strings.fr.toml`). Any key can be overridden by a
//! `strings.toml` placed next to `config.toml`, so a pharmacy can adapt
//! the wording (or translate the app) without recompiling.

use std::collections::HashMap;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("../assets/strings.fr.toml");

static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();

fn parse(text: &str) -> HashMap<String, String> {
    text.parse::<toml::Table>()
        .map(|t| {
            t.into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

fn table() -> &'static HashMap<String, String> {
    STRINGS.get_or_init(|| {
        let mut map = parse(EMBEDDED);
        let override_path = crate::config::Config::path().with_file_name("strings.toml");
        if let Ok(text) = std::fs::read_to_string(override_path) {
            for (k, v) in parse(&text) {
                map.insert(k, v);
            }
        }
        map
    })
}

/// Look up a UI string. A missing key shows up as the key itself, so a
/// typo is visible in the UI instead of failing silently.
pub fn tr(key: &'static str) -> &'static str {
    table().get(key).map(|s| s.as_str()).unwrap_or(key)
}

/// Fill the `{}` placeholders of a string, in order.
pub fn trn(key: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = tr(key).to_owned();
    for a in args {
        out = out.replacen("{}", &a.to_string(), 1);
    }
    out
}

/// One-placeholder convenience over [`trn`].
pub fn trf(key: &'static str, value: impl std::fmt::Display) -> String {
    trn(key, &[&value])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_strings_parse_and_resolve() {
        // The embedded file must parse: every key resolves to a value,
        // not to itself.
        assert_eq!(tr("form_last_name"), "Nom");
        assert!(tr("lock_subtitle").contains("mot de passe"));
        assert_eq!(trf("patient_born", "03/07/1958"), "Né(e) le 03/07/1958");
        assert_eq!(
            trn("status_summary", &[&5, &4, &58]),
            "5 patient(s)   ·   4 entretien(s) en cours   ·   58 médicaments"
        );
        // Missing keys fall back to the key, visibly.
        assert_eq!(tr("missing_key_xyz"), "missing_key_xyz");
        // The team-notes template survives as a multiline value.
        assert!(tr("team_doc_template").contains("## Consignes du jour"));
    }
}
