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

    /// Every key written as a literal in the sources must exist in the
    /// embedded file. A typo used to reach the counter as a raw key on
    /// screen — visible, but only to whoever happened to open that view.
    #[test]
    fn every_key_used_in_the_code_exists() {
        // The sources are embedded rather than read from disk: the test
        // must pass wherever the binary is run from.
        const SOURCES: &[(&str, &str)] = &[
            ("app.rs", include_str!("app.rs")),
            ("pdf.rs", include_str!("pdf.rs")),
            ("config.rs", include_str!("config.rs")),
            ("maintenance.rs", include_str!("maintenance.rs")),
            ("graph.rs", include_str!("graph.rs")),
            ("ordonnancier.rs", include_str!("ordonnancier.rs")),
            ("scans.rs", include_str!("scans.rs")),
        ];
        let mut missing: Vec<String> = Vec::new();
        for (file, source) in SOURCES {
            for call in ["tr(\"", "trf(\"", "trn(\""] {
                let mut rest = *source;
                while let Some(at) = rest.find(call) {
                    // A call is only one when what precedes it is not a
                    // letter: `str(\"` and `substr(\"` are not ours.
                    let before = rest[..at].chars().next_back().unwrap_or(' ');
                    rest = &rest[at + call.len()..];
                    if before.is_alphanumeric() || before == '_' {
                        continue;
                    }
                    let Some(end) = rest.find('"') else { break };
                    let key = &rest[..end];
                    // Keys are plain identifiers; anything else is an
                    // interpolation or a false positive.
                    if key.is_empty()
                        || !key
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    {
                        continue;
                    }
                    if tr_lookup(key).is_none() {
                        missing.push(format!("{file} : {key}"));
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "clés absentes de assets/strings.fr.toml :\n{}",
            missing.join("\n")
        );
    }

    /// …and the other way round: a key nobody uses is dead weight.
    ///
    /// The file is the officine's to override, so every line in it is a
    /// promise that changing that line changes something on screen.
    /// Twenty-seven keys had outlived the views that showed them —
    /// three toolbars, an agenda header, a fee table — and an operator
    /// editing one of them would have been editing nothing.
    #[test]
    fn every_key_in_the_file_is_used_somewhere() {
        // Any string literal in the sources counts, not only the ones
        // inside `tr(`: some keys are held in tables and looked up
        // through a variable (`MONO_FIELDS`, the section labels, the
        // steps of `maintenance`, the ties of `graph`, the kinds of
        // `ordonnancier`).
        const SOURCES: &[&str] = &[
            include_str!("app.rs"),
            include_str!("pdf.rs"),
            include_str!("config.rs"),
            include_str!("db.rs"),
            include_str!("bulletin.rs"),
            include_str!("maintenance.rs"),
            include_str!("graph.rs"),
            include_str!("ordonnancier.rs"),
            include_str!("scans.rs"),
            include_str!("vigilance.rs"),
            include_str!("codebar.rs"),
        ];
        let literal = |key: &str| {
            let quoted = format!("\"{key}\"");
            SOURCES.iter().any(|s| s.contains(&quoted))
        };
        let dead: Vec<&str> = EMBEDDED
            .lines()
            .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim()))
            .filter(|k| {
                !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            })
            .filter(|k| !literal(k))
            .collect();
        assert!(
            dead.is_empty(),
            "clés de assets/strings.fr.toml que plus personne n'affiche :\n{}",
            dead.join("\n")
        );
    }

    /// Every character of every string must have a glyph in the font
    /// the application actually draws with.
    ///
    /// A missing glyph is not an error anywhere: it is a hollow box on
    /// screen, and only whoever happens to open that view ever sees it.
    /// « Saisie courte : 230826 → 23/08/2026 » was one — the arrow is in
    /// egui's monospace face, which is why the key chips beside it were
    /// fine, and not in the proportional one the sentences use. The two
    /// are checked separately for that reason.
    ///
    /// The officine's own `strings.toml` is not covered: it is theirs.
    /// This holds the shipped file, which is what everybody sees.
    #[test]
    fn every_string_can_be_drawn_with_the_font_that_ships() {
        use eframe::egui;
        let fonts = egui::text::Fonts::new(1.0, 2048, egui::FontDefinitions::default());
        let faces = [
            egui::TextStyle::Body.resolve(&egui::Style::default()),
            egui::TextStyle::Monospace.resolve(&egui::Style::default()),
        ];
        let mut bad: Vec<String> = Vec::new();
        for line in EMBEDDED.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.starts_with('#') || key.is_empty() {
                continue;
            }
            let value = value.trim().trim_matches('"');
            for c in value.chars() {
                // A character every face lacks is a box wherever it is
                // drawn; one that only the monospace face has is a box
                // in a sentence, which is where these strings go.
                if !fonts.has_glyph(&faces[0], c) {
                    let elsewhere = fonts.has_glyph(&faces[1], c);
                    bad.push(format!(
                        "{key} : « {c} » (U+{:04X}){}",
                        c as u32,
                        if elsewhere {
                            " — présent en chasse fixe seulement"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }
        bad.sort();
        bad.dedup();
        assert!(
            bad.is_empty(),
            "caractères sans glyphe dans la police livrée :\n{}",
            bad.join("\n")
        );
    }

    /// The same check for the symbols the *code* writes into
    /// user-facing text, and for the two faces separately.
    ///
    /// The shortcuts window sets « ← → » in a key chip and its meaning
    /// in a sentence beside it. The chip is drawn in the monospace face,
    /// which has the arrows; the sentence is drawn in the proportional
    /// one, which does not. That is why the chip was right and the
    /// arrow inside « 230826 → 23/08/2026 » was a hollow box, one line
    /// apart, for as long as nobody looked closely.
    #[test]
    fn every_symbol_the_code_draws_has_a_glyph_in_the_face_that_draws_it() {
        use eframe::egui;
        let fonts = egui::text::Fonts::new(1.0, 2048, egui::FontDefinitions::default());
        let body = egui::TextStyle::Body.resolve(&egui::Style::default());
        let mono = egui::TextStyle::Monospace.resolve(&egui::Style::default());
        // Written into sentences: the interaction pair, the breadcrumbs
        // of the codex and the protocols, the euro sign, the en and em
        // dashes that stand in for the arrow.
        for c in [
            '\u{2194}', '\u{203A}', '\u{2039}', '\u{20AC}', '\u{2013}', '\u{2014}', '\u{00B7}',
            '\u{2026}', '\u{00AB}', '\u{00BB}',
        ] {
            assert!(
                fonts.has_glyph(&body, c),
                "« {c} » (U+{:04X}) est écrit dans une phrase et n'a pas de glyphe",
                c as u32
            );
        }
        // Only ever set in a key chip, which is monospace.
        for c in ['\u{2190}', '\u{2192}', '\u{2191}', '\u{2193}'] {
            assert!(
                fonts.has_glyph(&mono, c),
                "« {c} » (U+{:04X}) est écrit dans une pastille de touche et n'a pas de glyphe",
                c as u32
            );
            // …and the reason the chips are the only place they may go.
            assert!(
                !fonts.has_glyph(&body, c),
                "« {c} » a un glyphe en romain : la règle « les flèches restent dans les pastilles » n'a plus de raison d'être, et ce test non plus"
            );
        }
    }

    /// Look a key up without the `'static` requirement of [`tr`].
    fn tr_lookup(key: &str) -> Option<&'static str> {
        table().get(key).map(|s| s.as_str())
    }

    #[test]
    fn embedded_strings_parse_and_resolve() {
        // The embedded file must parse: every key resolves to a value,
        // not to itself.
        assert_eq!(tr("form_last_name"), "Nom");
        // Wording is the pharmacy's to change — assert that the key
        // resolves, not what it says. Asserting the copy made an
        // ordinary edit to the lock screen look like a broken build.
        assert_ne!(tr("lock_subtitle"), "lock_subtitle");
        assert!(!tr("lock_subtitle").trim().is_empty());
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
