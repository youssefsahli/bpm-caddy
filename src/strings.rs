//! User-facing UI strings, loaded from the embedded French TOML
//! (`assets/strings.fr.toml`). Any key can be overridden by a
//! `strings.toml` placed next to `config.toml`, so a pharmacy can adapt
//! the wording (or translate the app) without recompiling.
//!
//! **Une réécriture se souvient de ce qu'elle remplaçait.** C'est la
//! règle de `content.rs`, et elle manquait ici : une surcharge plate
//! `clé = "texte"` s'applique pour toujours, y compris quand la version
//! suivante a changé le libellé livré. L'officine croit alors lire sa
//! correction, et elle lit une phrase que personne n'a relue — le pire
//! des deux états, puisque rien ne le dit.
//!
//! Une entrée peut donc s'écrire de deux façons :
//!
//! ```toml
//! # Ancienne forme, toujours lue : elle s'applique sans condition.
//! form_last_name = "Patronyme"
//!
//! # Nouvelle : elle dit contre quoi elle a été écrite, et ne
//! # s'applique que tant que ce texte-là est celui qu'on livre.
//! [form_first_name]
//! texte = "Prénom usuel"
//! livre = "Prénom"
//! ```
//!
//! La forme plate reste valide — un fichier écrit il y a un an doit
//! continuer de marcher —, et l'écran qui édite les textes écrit la
//! seconde.

use std::collections::HashMap;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("../assets/strings.fr.toml");

static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();
static SHIPPED: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Ce qu'une surcharge demande : le texte, et le texte livré qu'elle
/// visait — `None` pour la forme plate, qui ne visait rien et
/// s'applique donc toujours.
#[derive(Clone, PartialEq, Debug)]
pub struct Rewrite {
    pub value: String,
    pub aimed_at: Option<String>,
}

/// Ce qu'une clé est devenue, pour l'écran qui les relit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Telle qu'elle est livrée : personne n'y a touché.
    Shipped,
    /// Réécrite, et toujours en face du texte qu'elle remplaçait.
    Rewritten,
    /// Réécrite, mais le texte livré a changé depuis : la surcharge ne
    /// s'applique plus et attend une relecture.
    Outdated,
}

fn parse_shipped(text: &str) -> HashMap<String, String> {
    text.parse::<toml::Table>()
        .map(|t| {
            t.into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

/// Les surcharges telles qu'elles sont écrites, sans décider encore si
/// elles s'appliquent.
pub fn parse_rewrites(text: &str) -> HashMap<String, Rewrite> {
    text.parse::<toml::Table>()
        .map(|t| {
            t.into_iter()
                .filter_map(|(k, v)| match &v {
                    // La forme plate : elle ne vise rien, elle
                    // s'applique.
                    toml::Value::String(s) => Some((
                        k,
                        Rewrite {
                            value: s.clone(),
                            aimed_at: None,
                        },
                    )),
                    toml::Value::Table(inner) => {
                        let value = inner.get("texte")?.as_str()?.to_owned();
                        Some((
                            k,
                            Rewrite {
                                value,
                                aimed_at: inner
                                    .get("livre")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_owned),
                            },
                        ))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Le texte livré, sans aucune surcharge : ce que l'écran de relecture
/// montre à côté de ce que l'officine a écrit.
pub fn shipped() -> &'static HashMap<String, String> {
    SHIPPED.get_or_init(|| parse_shipped(EMBEDDED))
}

/// Ce qu'une surcharge est devenue face au texte livré d'aujourd'hui.
///
/// **Une réécriture posée sur autre chose que ce qu'elle visait serait
/// pire que pas de réécriture du tout** : elle ne s'applique pas, et
/// l'écran la montre pour relecture.
pub fn state(rewrite: &Rewrite, shipped_now: Option<&str>) -> State {
    match (&rewrite.aimed_at, shipped_now) {
        (None, _) => State::Rewritten,
        (Some(seen), Some(now)) if seen == now => State::Rewritten,
        _ => State::Outdated,
    }
}

/// Le chemin du fichier de surcharges, à côté de `config.toml`.
pub fn overrides_path() -> std::path::PathBuf {
    crate::config::Config::path().with_file_name("strings.toml")
}

fn table() -> &'static HashMap<String, String> {
    STRINGS.get_or_init(|| {
        let mut map = shipped().clone();
        if let Ok(text) = std::fs::read_to_string(overrides_path()) {
            for (k, r) in parse_rewrites(&text) {
                // Une surcharge qui ne vise plus le texte livré ne
                // s'applique pas : c'est la règle du module, et c'est
                // ici qu'elle mord.
                if state(&r, map.get(&k).map(String::as_str)) == State::Rewritten {
                    map.insert(k, r.value);
                }
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

/// Un nombre décimal **à la française** : la virgule, jamais le point.
///
/// Sept endroits l'écrivaient à l'anglaise sur un écran en français —
/// « 15.00 € » sur l'infobulle d'un honoraire et dans les Options,
/// « 1.2 Mo » sur une pièce scannée, « 6.3 » mégaoctets dans « À
/// propos », « accumulation ×2.0 » au comptoir, et la demi-vie d'une
/// fiche. C'est le même repli que `codex::format_quantity` fait depuis
/// toujours pour les quantités ; il manquait partout ailleurs.
///
/// Les entiers n'ont pas de séparateur décimal et ne passent donc pas
/// par ici : `{v:.0}` reste écrit tel quel.
pub fn decimal(value: f64, places: usize) -> String {
    format!("{value:.places$}").replace('.', ",")
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
            include_str!("timeline.rs"),
            include_str!("script.rs"),
            include_str!("selfcheck.rs"),
            // Le registre des textes imprimés y nomme ses
            // documents : une source de clés comme les autres.
            include_str!("content.rs"),
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
        // U+00A0 n'est écrit dans aucune chaîne : il est **produit** par
        // `app::help_bound`, qui lie la ponctuation double française au
        // mot qui la précède pour qu'un « ; » ne commence pas une ligne
        // du volet d'aide. C'est l'insécable ordinaire et non l'espace
        // fine U+202F que la typographie voudrait, précisément parce que
        // la fonte livrée n'a pas de glyphe pour celle-là — la panne du
        // séparateur de milliers de la caisse, deux lignes plus bas.
        for c in [
            '\u{2194}', '\u{203A}', '\u{2039}', '\u{20AC}', '\u{2013}', '\u{2014}', '\u{00B7}',
            '\u{2026}', '\u{00AB}', '\u{00BB}', '\u{00A0}',
        ] {
            assert!(
                fonts.has_glyph(&body, c),
                "« {c} » (U+{:04X}) est écrit dans une phrase et n'a pas de glyphe",
                c as u32
            );
        }
        // **Et les caractères qu'aucune chaîne ne porte** : ceux qu'un
        // format produit. Le séparateur de milliers de `caisse::euros`
        // était une espace fine insécable (U+202F), qui n'a pas de
        // glyphe dans la fonte livrée : tout montant à quatre chiffres
        // sortait « 1□240,50 » à l'écran, et aucun test de chaîne ne
        // pouvait le voir puisque le caractère n'est écrit nulle part.
        // On passe ici la **sortie de la fonction**, pas une constante.
        for c in crate::caisse::euros(1_234_567).chars() {
            assert!(
                fonts.has_glyph(&body, c),
                "« {c} » (U+{:04X}) sort de `caisse::euros` et n'a pas de glyphe",
                c as u32
            );
        }

        // **Et le français que les modules portent eux-mêmes.** Le
        // libellé d'une nature de poste et celui d'un rythme sont
        // écrits avec leur type, comme son vocabulaire — c'est une
        // exception assumée à « les chaînes vivent dans le fichier »,
        // et elle ne doit pas coûter la vérification que le fichier,
        // lui, subit. Une phrase d'explication de rythme tient trois
        // lignes : c'est exactement l'endroit où une flèche ou une
        // espace fine se glisse sans qu'on la voie.
        for text in crate::planning::Cadence::ALL
            .into_iter()
            .flat_map(|c| [c.label(), c.hint()])
            .chain(
                crate::planning::ShiftKind::ALL
                    .into_iter()
                    .map(crate::planning::ShiftKind::label),
            )
            .chain(
                crate::config::Role::ALL
                    .into_iter()
                    .map(crate::config::Role::label),
            )
        {
            for c in text.chars() {
                assert!(
                    fonts.has_glyph(&body, c),
                    "« {c} » (U+{:04X}) est écrit dans « {text} » et n'a pas de glyphe",
                    c as u32
                );
            }
        }

        // **Et le mode d'emploi livré**, qui part à l'écran comme
        // n'importe quelle chaîne sans être dans le fichier de chaînes :
        // c'est de la prose, écrite en markdown à côté, et le tiret
        // cadratin, le chevron et l'espace fine s'y glissent aussi bien
        // que dans un libellé. Avec elle, ce que la console dit d'elle-
        // même — les bornes et les descriptions de l'API — qui est du
        // français porté par le code, pour la même raison que les
        // rythmes ci-dessus.
        let manual = include_str!("../assets/aide.md");
        for (c, whence) in manual
            .chars()
            .map(|c| (c, "le mode d'emploi"))
            .chain(
                crate::script::LIMITS
                    .iter()
                    .flat_map(|l| l.chars())
                    .map(|c| (c, "une borne de la console")),
            )
            .chain(
                crate::script::API
                    .iter()
                    .flat_map(|call| {
                        [call.call, call.returns, call.note]
                            .into_iter()
                            .chain(call.fields.iter().flat_map(|(f, w)| [*f, *w]))
                    })
                    .flat_map(str::chars)
                    .map(|c| (c, "la description de l'API")),
            )
            .filter(|(c, _)| !c.is_control())
        {
            assert!(
                fonts.has_glyph(&body, c),
                "« {c} » (U+{:04X}) est écrit dans {whence} et n'a pas de glyphe",
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

    /// **Une réécriture se souvient de ce qu'elle remplaçait**, et ne
    /// s'applique que tant que le texte livré n'a pas bougé.
    ///
    /// C'est la règle de `content.rs`, et elle manquait ici : une
    /// surcharge plate s'applique pour toujours, y compris quand la
    /// version suivante a changé le libellé. L'officine croit alors lire
    /// sa correction et lit une phrase que personne n'a relue — le pire
    /// des deux états, puisque rien ne le dit.
    ///
    /// La forme plate reste valide : un fichier écrit il y a un an doit
    /// continuer de marcher, et il n'a jamais rien visé.
    #[test]
    fn a_rewrite_remembers_what_it_replaced() {
        let rows = parse_rewrites(
            r#"
form_last_name = "Patronyme"

[form_first_name]
texte = "Prénom usuel"
livre = "Prénom"

[form_birth_date]
texte = "Date de naissance complète"
livre = "Une phrase qui n'est plus livrée"
"#,
        );
        assert_eq!(rows.len(), 3);
        // La forme plate ne vise rien, donc elle s'applique toujours.
        let flat = &rows["form_last_name"];
        assert_eq!(flat.aimed_at, None);
        assert_eq!(state(flat, Some("Nom")), State::Rewritten);
        assert_eq!(state(flat, Some("autre chose")), State::Rewritten);
        // Celle qui vise juste s'applique.
        let aimed = &rows["form_first_name"];
        assert_eq!(aimed.aimed_at.as_deref(), Some("Prénom"));
        assert_eq!(state(aimed, Some("Prénom")), State::Rewritten);
        // Celle qui vise à côté ne s'applique pas : elle attend une
        // relecture, et c'est l'écran qui la montre.
        assert_eq!(state(aimed, Some("Prénom (usuel)")), State::Outdated);
        assert_eq!(
            state(&rows["form_birth_date"], Some("Date de naissance")),
            State::Outdated
        );
        // Une clé que l'application ne livre plus du tout : la
        // surcharge ne vise plus rien.
        assert_eq!(state(aimed, None), State::Outdated);
        // Ce qui ne se lit pas ne casse rien : une valeur d'un autre
        // type, ou une table sans `texte`, est ignorée plutôt que de
        // faire tomber tout le fichier.
        let junk = parse_rewrites("a = 3\n[b]\nlivre = \"x\"\n");
        assert!(junk.is_empty());
    }

    /// Le texte livré se lit sans surcharge, et il n'est pas vide : c'est
    /// lui que l'écran de relecture montre en face de ce que l'officine
    /// a écrit.
    #[test]
    fn the_shipped_text_is_readable_on_its_own() {
        let ship = shipped();
        assert!(ship.len() > 500, "{} clés livrées", ship.len());
        assert_eq!(ship.get("form_last_name").map(String::as_str), Some("Nom"));
        assert!(ship.values().all(|v| !v.is_empty()));
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
        assert_ne!(tr("app_tagline"), "app_tagline");
        assert!(!tr("app_tagline").trim().is_empty());
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
    /// **Deux tests ne partagent pas un répertoire temporaire.**
    ///
    /// Ils tournent en parallèle **dans un seul processus**, si bien que
    /// `format!("bpm-caddy-x-{}", std::process::id())` écrit par deux
    /// tests donne le **même** chemin : chacun efface la base de
    /// l'autre, et le perdant échoue sur « disk I/O error », au gré de
    /// l'ordonnancement. Trois paires avaient dérivé ainsi, et le défaut
    /// ne se reproduit pas à la demande — c'est le pire des deux mondes,
    /// un échec qui ne revient pas quand on le cherche.
    ///
    /// Le gabarit littéral est ce qui compte : le numéro de processus
    /// est le même pour tous, il ne sépare rien. Quatre-vingt-huit
    /// gabarits pour quatre-vingt-huit sites aujourd'hui.
    #[test]
    fn no_two_temporary_directories_share_a_name() {
        const SOURCES: &[(&str, &str)] = &[
            ("app.rs", include_str!("app.rs")),
            ("config.rs", include_str!("config.rs")),
            ("db.rs", include_str!("db.rs")),
            ("maintenance.rs", include_str!("maintenance.rs")),
            ("pdf.rs", include_str!("pdf.rs")),
            ("release.rs", include_str!("release.rs")),
            ("scans.rs", include_str!("scans.rs")),
        ];
        // Assemblé, sinon le test se trouve lui-même.
        let call = concat!("temp_", "dir()");
        let mut seen: Vec<(String, String)> = Vec::new();
        let mut clashes: Vec<String> = Vec::new();
        for (file, src) in SOURCES {
            for (i, line) in src.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") || !t.contains(call) {
                    continue;
                }
                // Le gabarit est le littéral qui suit l'appel — sur la
                // même ligne, ou sur la suivante quand `cargo fmt` a
                // replié l'appel.
                let after = t.split_once(call).map(|(_, r)| r).unwrap_or("");
                let literal = after
                    .split_once('"')
                    .and_then(|(_, r)| r.split_once('"'))
                    .map(|(lit, _)| lit.to_owned())
                    .or_else(|| {
                        src.lines().nth(i + 1).and_then(|n| {
                            n.split_once('"')
                                .and_then(|(_, r)| r.split_once('"'))
                                .map(|(lit, _)| lit.to_owned())
                        })
                    });
                let Some(literal) = literal else { continue };
                let here = format!("{file}:{}", i + 1);
                if let Some((_, first)) = seen.iter().find(|(l, _)| *l == literal) {
                    clashes.push(format!("« {literal} » : {first} et {here}"));
                } else {
                    seen.push((literal, here));
                }
            }
        }
        assert!(
            seen.len() > 60,
            "seulement {} répertoires temporaires lus : le motif a changé \
             et ce test ne lit plus rien",
            seen.len()
        );
        assert!(
            clashes.is_empty(),
            "deux chemins temporaires identiques — les tests tournent dans \
             un seul processus, donc le numéro de processus ne les sépare \
             pas :\n{}",
            clashes.join("\n")
        );
    }

    /// **Tout document réécrivable a son test apparié.**
    ///
    /// `content::documents()` est le registre des phrases que l'officine
    /// peut réécrire, et chaque source doit prouver **les deux sens** :
    /// toute phrase listée pour l'édition, toute réécriture atteignant
    /// la page. Sans les deux, une phrase absente de `phrases()` ne se
    /// corrige pas, et une phrase absente de la résolution part telle
    /// qu'elle est livrée pendant qu'on la croit corrigée — ce qui est
    /// pire.
    ///
    /// Sur onze sources, deux n'avaient pas ce test : l'ordonnance du
    /// TROD et les conseils du voyageur. Toutes deux étaient bien
    /// câblées ; c'est la preuve qui manquait, et elle manque toujours
    /// au moment où quelqu'un déplace la résolution. Le douzième
    /// document aurait eu le même trou.
    #[test]
    fn every_rewritable_document_has_its_paired_test() {
        // Le nom des tests n'est pas uniforme — « reaches_the_paper »
        // pour les carnets, « arrives » ailleurs —, et il n'a pas à
        // l'être : c'est le mot `rewrite` qui les réunit.
        const SOURCES: &[(&str, &str)] = &[
            ("biology.rs", include_str!("biology.rs")),
            ("crush.rs", include_str!("crush.rs")),
            ("entretien.rs", include_str!("entretien.rs")),
            ("gravidity.rs", include_str!("gravidity.rs")),
            ("hepatic.rs", include_str!("hepatic.rs")),
            ("ordonnance.rs", include_str!("ordonnance.rs")),
            ("renal.rs", include_str!("renal.rs")),
            ("revue.rs", include_str!("revue.rs")),
            ("selfcheck.rs", include_str!("selfcheck.rs")),
            ("surveillance.rs", include_str!("surveillance.rs")),
            ("vaccines.rs", include_str!("vaccines.rs")),
        ];
        // Autant de sources que le registre en cite : les carnets
        // partagent la leur, d'où un module de moins que de documents.
        let subjects: std::collections::HashSet<String> = crate::content::documents()
            .iter()
            .map(|d| {
                d.subject
                    .split_once('.')
                    .map_or(d.subject.clone(), |(head, _)| head.to_owned())
            })
            .collect();
        assert_eq!(
            subjects.len(),
            SOURCES.len(),
            "le registre cite {} sujets et ce test lit {} modules",
            subjects.len(),
            SOURCES.len()
        );
        let mut mute: Vec<&str> = Vec::new();
        for (file, src) in SOURCES {
            let paired = src.lines().any(|l| {
                let t = l.trim_start();
                !t.starts_with("//") && t.starts_with("fn ") && t.contains("rewrite")
            });
            if !paired {
                mute.push(file);
            }
        }
        assert!(
            mute.is_empty(),
            "document réécrivable sans son test apparié — il faut prouver \
             les deux sens, la phrase listée et la réécriture qui \
             arrive :\n{mute:?}"
        );
    }

    /// **Toute vue documentée est balayée par le passage de fumée.**
    ///
    /// `BPM_CADDY_START_VIEW` est la seule façon d'atteindre certaines
    /// vues — « aide » n'est dans aucune barre d'onglets, « finances »
    /// n'a pas de porte du tout —, et `smoke.sh` est ce qui tient
    /// l'interface à la place des tests qu'une vue ne peut pas avoir.
    /// Une clé documentée mais absente de son tableau est donc une vue
    /// que **rien** n'ouvre jamais, ni la main ni la garde.
    ///
    /// Deux l'étaient : « search » et « planning_mois », toutes deux
    /// vraies clés du code, décrites en prose plus bas dans CLAUDE.md et
    /// absentes de la liste qu'on lit pour savoir ce qui existe.
    #[test]
    fn every_documented_view_is_swept_by_the_smoke_pass() {
        const CLAUDE: &str = include_str!("../CLAUDE.md");
        const SMOKE: &str = include_str!("../scripts/smoke.sh");
        // `smoke.sh` nomme aussi ses **formes** dans ce tableau —
        // « drug_edit » et « drug_kin » ne sont pas des vues mais des
        // variables d'environnement posées par-dessus `drug_card`, et
        // CLAUDE.md les documente à leur place, avec elles.
        const SHAPES_NOT_VIEWS: &[&str] = &["drug_edit", "drug_kin"];

        let list = CLAUDE
            .split_once("BPM_CADDY_START_VIEW=")
            .expect("la liste des vues dans CLAUDE.md")
            .1
            .split_once('`')
            .expect("la liste se ferme par une apostrophe inverse")
            .0;
        let documented: Vec<&str> = list
            .split(['|', '\n', ' '])
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .collect();
        assert!(
            documented.len() > 60,
            "seulement {} vues lues dans CLAUDE.md : le format de la liste \
             a changé, et ce test ne lit plus rien",
            documented.len()
        );

        // Et `eyeball.sh` balaie les mêmes : `smoke.sh` prouve que rien
        // n'a paniqué, il ne dit rien d'un titre qui déborde ou d'une
        // bande coupée — **ça, c'est en regardant qu'on le voit**, et
        // une vue absente de la liste des captures est une vue que
        // personne ne regarde jamais. Trois l'étaient : les deux pages
        // de la caisse et le lecteur de carte Vitale.
        const EYEBALL: &str = include_str!("../scripts/eyeball.sh");
        let eyed_block = EYEBALL
            .split_once("views=(")
            .expect("le tableau des vues dans eyeball.sh")
            .1
            .split_once(')')
            .expect("le tableau se ferme")
            .0;
        let eyed: Vec<&str> = eyed_block.split_whitespace().collect();

        let swept_block = SMOKE
            .split_once("views=(")
            .expect("le tableau des vues dans smoke.sh")
            .1
            .split_once(')')
            .expect("le tableau se ferme")
            .0;
        let swept: Vec<&str> = swept_block
            .split_whitespace()
            .filter(|k| !k.is_empty())
            .collect();

        let mut missing: Vec<&str> = documented
            .iter()
            .filter(|k| !swept.contains(k))
            .copied()
            .collect();
        missing.sort_unstable();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "vues documentées que `smoke.sh` n'ouvre jamais : {missing:?}"
        );

        let mut unseen: Vec<&str> = swept
            .iter()
            .filter(|k| !eyed.contains(k))
            .copied()
            .collect();
        unseen.sort_unstable();
        unseen.dedup();
        assert!(
            unseen.is_empty(),
            "vues que `smoke.sh` ouvre et que `eyeball.sh` ne capture \
             jamais — elles ne paniquent pas, et personne ne les \
             regarde : {unseen:?}"
        );

        let mut undocumented: Vec<&str> = swept
            .iter()
            .filter(|k| !documented.contains(k) && !SHAPES_NOT_VIEWS.contains(k))
            .copied()
            .collect();
        undocumented.sort_unstable();
        undocumented.dedup();
        assert!(
            undocumented.is_empty(),
            "vues ouvertes par `smoke.sh` que CLAUDE.md ne liste pas : \
             {undocumented:?}"
        );
    }

    /// **Ce que « Aller à… » cherche est une liste, et une liste se
    /// confronte à son registre.**
    ///
    /// Le manuel l'énumérait à la main et en avait perdu deux sur onze :
    /// les vues elles-mêmes et les textes imprimés. Une liste recopiée
    /// vieillit là où personne ne la relit — la règle que ce dépôt
    /// applique déjà au mode d'emploi imprimé et aux comptes de
    /// `CLAUDE.md`.
    ///
    /// Les mots ne peuvent pas être cherchés tels quels : le registre
    /// dit « patient » là où le manuel écrit « dossiers ». La table
    /// ci-dessous porte donc les deux, et un douzième genre ajouté à
    /// `strings.fr.toml` fait tomber le test tant qu'il n'y figure pas.
    #[test]
    fn everything_the_jump_box_searches_is_named_in_the_manual() {
        const AIDE: &str = include_str!("../assets/aide.md");
        // Le genre tel que le registre le nomme, et le mot que le manuel
        // emploie pour la même chose.
        const SAID: &[(&str, &str)] = &[
            ("goto_kind_view", "les vues"),
            ("goto_kind_patient", "les dossiers"),
            ("goto_kind_drug", "les fiches"),
            ("goto_kind_table", "les tables de conversion"),
            ("goto_kind_prep", "les préparations"),
            ("goto_kind_protocol", "les protocoles"),
            ("goto_kind_dispositif", "les dispositifs"),
            ("goto_kind_stup", "le registre"),
            ("goto_kind_carnet", "les carnets de suivi"),
            ("goto_kind_script", "les scripts"),
            ("goto_kind_text", "les textes imprimés"),
        ];
        let sentence = AIDE
            .split("« Aller à… » cherche partout")
            .nth(1)
            .expect("la phrase du « Aller à… » a disparu du manuel")
            .split("\n\n")
            .next()
            .unwrap_or_default()
            // Le manuel est enveloppé à la main : « les\ndossiers » est
            // le même mot que « les dossiers ».
            .replace('\n', " ");
        for (key, said) in SAID {
            assert!(
                shipped().contains_key(*key),
                "{key} n'est plus dans la table des chaînes : le manuel parle \
                 d'un genre que la boîte ne cherche plus"
            );
            assert!(
                sentence.contains(said),
                "« Aller à… » cherche {key} et le manuel ne dit pas « {said} »"
            );
        }
        let known: usize = shipped()
            .keys()
            .filter(|k| k.starts_with("goto_kind_"))
            .count();
        assert_eq!(
            known,
            SAID.len(),
            "la boîte cherche {known} genres et le manuel en nomme {} : \
             ajouter le manquant des deux côtés",
            SAID.len()
        );
    }

    /// **Un nombre décimal s'écrit à la virgule.**
    ///
    /// L'écran est en français et sept endroits l'écrivaient à
    /// l'anglaise : « 15.00 € » sur l'infobulle d'un honoraire et dans
    /// les Options, « 1.2 Mo » sur une pièce scannée, « 6.3 »
    /// mégaoctets dans « À propos », « accumulation ×2.0 » au comptoir,
    /// la demi-vie sur deux fiches. Aucun n'est visible sur une capture
    /// prise au bon endroit — ce sont des infobulles et des coins
    /// d'écran —, et c'est exactement ce qu'un lint attrape.
    ///
    /// `{v:.0}` reste permis : un entier n'a pas de séparateur décimal.
    /// `bulletin.rs` n'est pas lu : ses `{:.2}` sont de la syntaxe PDF,
    /// où le point est la seule écriture valable.
    #[test]
    fn no_decimal_number_is_written_with_an_english_point() {
        const SOURCES: [(&str, &str); 3] = [
            ("app.rs", include_str!("app.rs")),
            ("scans.rs", include_str!("scans.rs")),
            ("caisse.rs", include_str!("caisse.rs")),
        ];
        let mut found = Vec::new();
        for (name, src) in SOURCES {
            for (n, line) in src.lines().enumerate() {
                let trimmed = line.trim_start();
                // Les tests écrivent des attendus, et un attendu porte
                // justement la virgule qu'on exige ailleurs.
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }
                if !(line.contains(":.1}") || line.contains(":.2}") || line.contains(":.3}")) {
                    continue;
                }
                // Le repli explicite est la réponse attendue, et
                // `strings::decimal` le fait pour tout le monde.
                if line.contains("replace('.'") || line.contains("decimal(") {
                    continue;
                }
                found.push(format!("{name}:{}", n + 1));
            }
        }
        assert!(
            found.is_empty(),
            "un nombre décimal écrit au point sur un écran en français : {found:?} — \
             passer par `strings::decimal`"
        );
    }

    /// **Le manuel nomme les prodrogues une par une**, et une liste
    /// recopiée vieillit là où personne ne la relit — c'est la règle que
    /// ce dépôt applique déjà au mode d'emploi imprimé. Celle-ci compte
    /// double : une prodrogue est la seule ligne dont le croisement se
    /// lit **à l'envers**, et la phrase du manuel est ce qui l'explique.
    /// Les deux sens, parce qu'une molécule retirée de la table laisse
    /// le manuel promettant une lecture qui n'a plus lieu.
    #[test]
    fn every_prodrug_the_table_turns_round_is_named_in_the_manual() {
        const AIDE: &str = include_str!("../assets/aide.md");
        let sentence = AIDE
            .split("Une prodrogue s'y lit à l'envers")
            .nth(1)
            .expect("la phrase des prodrogues a disparu du manuel")
            .split("\n\n")
            .next()
            .unwrap_or_default();
        let folded = crate::fuzzy::sort_key(sentence);
        let named: Vec<&str> = crate::cyp::TABLE
            .iter()
            .filter(|p| p.actions.iter().any(|a| a.prodrug))
            .map(|p| p.needs[0])
            .collect();
        for molecule in &named {
            assert!(
                crate::fuzzy::contains_folded(&folded, &crate::fuzzy::sort_key(molecule)),
                "« {molecule} » est une prodrogue de la table et le manuel ne la \
                 nomme pas : le croisement qui s'y lit à l'envers n'est expliqué \
                 nulle part"
            );
        }
        // Et le sens inverse : chaque nom cité est bien une prodrogue.
        // La phrase les énumère après « le métabolite actif », séparés
        // par des virgules et un « ou » : c'est cette énumération-là
        // qu'on lit, et non les mots de la phrase, pour n'avoir aucune
        // liste de mots de prose à tenir à jour à côté.
        let list = sentence
            .split("le métabolite actif")
            .nth(1)
            .expect("l'énumération des prodrogues a changé de forme")
            .split("ne les fait pas")
            .next()
            .unwrap_or_default();
        let mut cited = 0;
        for piece in list.replace(" ou ", ", ").split(',') {
            let word = piece
                .trim()
                .split([' ', '\n', '\''])
                .next_back()
                .unwrap_or_default();
            if word.is_empty() {
                continue;
            }
            cited += 1;
            let folded_word = crate::fuzzy::sort_key(word);
            assert!(
                named.iter().any(|m| crate::fuzzy::contains_folded(
                    &crate::fuzzy::sort_key(m),
                    &folded_word
                )),
                "le manuel nomme « {word} » parmi les prodrogues et la table ne \
                 le connaît pas ainsi"
            );
        }
        assert_eq!(
            cited,
            named.len(),
            "le manuel cite {cited} prodrogues, la table en porte {}",
            named.len()
        );
    }

    /// **Les chiffres que la documentation affirme, le code les tient.**
    ///
    /// `CLAUDE.md` et `docs/CONTENU.md` sont lus avant chaque décision,
    /// et ils donnent des comptes pour vrais : tant de fiches livrées,
    /// tant de présentations au catalogue, tant de phrases imprimables.
    /// Ce sont des phrases et non des assertions, si bien qu'ils
    /// vieillissent sans bruit — le 13/09/2026, **six sur dix** avaient
    /// dérivé, dont un que la barre d'état de l'application dément à
    /// chaque instant (851 fiches contre 862).
    ///
    /// **Et le papier compte aussi.** Le même jour, le README — dont
    /// trois chiffres seulement étaient tenus — en portait six autres
    /// qui avaient dérivé, deux se contredisant à l'intérieur d'une même
    /// phrase ; et le mode d'emploi **imprimé** annonçait six onglets au
    /// dossier, qui en a sept. Une liste recopiée à la main vieillit là
    /// où personne ne la relit, et celle-là se pose près du poste. Les
    /// sources de ce test incluent donc `src/pdf.rs`.
    ///
    /// Chaque ligne ci-dessous est cherchée **telle quelle**. Une
    /// reformulation la fait donc échouer, et c'est voulu : un filet
    /// qui ne trouve plus sa phrase et se tait est un filet mort, comme
    /// un test sans son attribut.
    #[test]
    fn the_documentation_counts_what_the_code_holds() {
        const CLAUDE: &str = include_str!("../CLAUDE.md");
        const CONTENU: &str = include_str!("../docs/CONTENU.md");
        // Le manuel aussi : celui-là, l'officine le lit à l'écran.
        const AIDE: &str = include_str!("../assets/aide.md");
        // Et le README, que lisent ceux qui n'ont pas encore installé.
        const README: &str = include_str!("../README.md");
        // Le mode d'emploi imprimé vit dans le code, pas dans un
        // fichier de documentation : on le lit donc à la source.
        const PDF_SOURCE: &str = include_str!("pdf.rs");
        let cards = crate::db::STARTER_DRUG_COUNT;
        let labels = {
            let mut v: Vec<&str> = crate::db::STARTER_DRUGS
                .iter()
                .map(|(_, _, class, _)| *class)
                .filter(|c| !c.trim().is_empty())
                .collect();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        // Les milliers séparés comme le README les écrit — « 1 736 » et
        // non « 1736 ». Espace ordinaire : c'est celle qui y est, et le
        // test suit le texte plutôt que de le corriger en passant.
        fn thousands(n: usize) -> String {
            let digits = n.to_string();
            let mut out = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i).is_multiple_of(3) {
                    out.push(' ');
                }
                out.push(c);
            }
            out
        }
        let presentations: usize = crate::ordonnancier::CATALOGUE
            .iter()
            .map(|f| f.items.len())
            .sum();
        let families = crate::ordonnancier::CATALOGUE.len();
        let printed: usize = crate::content::documents()
            .iter()
            .map(|d| d.phrases.len())
            .sum();
        let expected: &[(&str, &str, String)] = &[
            (
                "CLAUDE.md",
                CLAUDE,
                format!("{labels} distinct labels over {cards} cards"),
            ),
            (
                "CLAUDE.md",
                CLAUDE,
                format!("the {printed} printed phrases the officine may rewrite"),
            ),
            (
                "CLAUDE.md",
                CLAUDE,
                format!("{presentations} presentations of the French market"),
            ),
            (
                "CLAUDE.md",
                CLAUDE,
                format!("in {families} families, each with its dosage"),
            ),
            (
                "CLAUDE.md",
                CLAUDE,
                format!(
                    "pregnancy and breastfeeding as a level: {}",
                    match crate::gravidity::TABLE.len() {
                        36 => "thirty-six",
                        n => panic!(
                            "la table de grossesse porte {n} molécules : écrire le \
                             nombre en toutes lettres dans CLAUDE.md et ici"
                        ),
                    }
                ),
            ),
            (
                "docs/CONTENU.md",
                CONTENU,
                format!("{presentations} présentations du marché français"),
            ),
            (
                "docs/CONTENU.md",
                CONTENU,
                format!("{labels} libellés pour {cards}"),
            ),
            (
                "README.md",
                README,
                format!("A fresh base starts with {cards} common drugs"),
            ),
            (
                "README.md",
                README,
                format!("A catalogue of {presentations} presentations"),
            ),
            (
                "README.md",
                README,
                format!(
                    "the officine's magistral and officinal formulas, {} to start with",
                    match crate::db::STARTER_PREPARATIONS.len() {
                        80 => "eighty",
                        n => panic!(
                            "le codex porte {n} préparations : l'écrire en toutes \
                             lettres dans le README et ici"
                        ),
                    }
                ),
            ),
            (
                "assets/aide.md",
                AIDE,
                format!(
                    "ne connaît que {} cytochromes",
                    match crate::cyp::Enzyme::ALL.len() {
                        7 => "sept",
                        n => panic!(
                            "la table porte {n} cytochromes : l'écrire en toutes \
                             lettres dans le manuel et ici"
                        ),
                    }
                ),
            ),
            // Le manuel annonce **cinq** lectures sous la biologie, et
            // la sixième attend — le foie n'a pas de panneau côté
            // patient. Le jour où il en aura un, c'est cette phrase-là
            // qui mentira, sur l'écran que l'officine lit pour
            // apprendre ce que l'application sait faire.
            (
                "assets/aide.md",
                AIDE,
                format!(
                    "{} lectures de la même ordonnance",
                    match crate::app::BIO_SIDE_TABS {
                        5 => "cinq",
                        n => panic!(
                            "la biologie porte {n} lectures : l'écrire en toutes \
                             lettres dans le manuel et ici"
                        ),
                    }
                ),
            ),
            (
                "assets/aide.md",
                AIDE,
                format!(
                    "{} lectures, et la première passe avant les autres",
                    match crate::conciliation::Change::ALL.len() {
                        6 => "Six",
                        n => panic!(
                            "la conciliation porte {n} lectures : l'écrire en \
                             toutes lettres dans le manuel et ici"
                        ),
                    }
                ),
            ),
            // **Le README affirmait six chiffres que le code démentait.**
            // Le catalogue des stupéfiants y était compté deux fois dans
            // la même phrase — 158 puis 106 —, les lignes de posologie
            // deux fois à deux endroits — 1 736 et 1 319 —, et quatre
            // tables avaient grandi sans que la phrase bouge. C'est la
            // page que lisent ceux qui n'ont pas encore installé : elle
            // vieillit sans que personne la relise contre le code.
            (
                "README.md",
                README,
                format!("all {presentations} followed would be {presentations} zero balances"),
            ),
            (
                "README.md",
                README,
                format!(
                    "and so are the {} posology lines",
                    thousands(crate::db::STARTER_POSOLOGIES.len())
                ),
            ),
            (
                "README.md",
                README,
                format!(
                    "{} counter references browsable in-app",
                    match crate::tables::TABLES.len() {
                        46 => "forty-six",
                        n => panic!("{n} tables de conversion : l'écrire ici et au README"),
                    }
                ),
            ),
            (
                "README.md",
                README,
                format!(
                    "{} of them.",
                    match crate::surveillance::WATCHES.len() {
                        65 => "Sixty-five",
                        n => panic!("{n} surveillances : l'écrire ici et au README"),
                    }
                ),
            ),
            (
                "README.md",
                README,
                format!(
                    "applies {} rules that tie a value",
                    match crate::biology::rule_count() {
                        103 => "a hundred and three",
                        n => panic!("{n} règles de biologie : l'écrire ici et au README"),
                    }
                ),
            ),
            (
                "README.md",
                README,
                format!(
                    "{} sections in two columns on one sheet",
                    match crate::pdf::guide_section_count() {
                        16 => "sixteen",
                        n => panic!("{n} sections au mode d'emploi : l'écrire ici et au README"),
                    }
                ),
            ),
            // **Le mode d'emploi imprimé compte les onglets du dossier.**
            // Il en annonçait six et les énumérait sans le fil : il avait
            // été écrit avant lui, et rien ne reliait la phrase à la
            // liste. C'est une feuille qu'une officine imprime et pose
            // près du poste — elle vieillit là où personne ne la relit.
            (
                "src/pdf.rs",
                PDF_SOURCE,
                format!(
                    "{} onglets",
                    match crate::app::PatientTab::ALL.len() {
                        7 => "sept",
                        n => panic!(
                            "le dossier porte {n} onglets : l'écrire en toutes \
                             lettres dans le mode d'emploi et ici"
                        ),
                    }
                ),
            ),
        ];
        let mut wrong: Vec<String> = Vec::new();
        for (name, text, phrase) in expected {
            if !text.contains(phrase.as_str()) {
                wrong.push(format!("{name} ne dit pas « {phrase} »"));
            }
        }
        assert!(
            wrong.is_empty(),
            "la documentation affirme un compte que le code dément — ou sa \
             phrase a été reformulée, auquel cas c'est ici qu'il faut la \
             suivre :\n{}",
            wrong.join("\n")
        );
    }

    /// **Un test sans `#[test]` est un gardien mort**, et il meurt sans
    /// bruit : la suite repasse au vert avec un test de moins, et le
    /// nombre affiché ne se lit pas.
    ///
    /// C'est arrivé le 13/09/2026, en insérant un filet juste au-dessus
    /// d'un autre : l'insertion a avalé le `#[test]` de
    /// `no_font_size_is_written_in_pixels`, quatre cents littéraux de
    /// taille de police ont cessé d'être refusés, et tout était vert.
    ///
    /// Le repère est étroit : une fonction **sans paramètre et sans
    /// valeur de retour**, dans un module de tests, n'est rien d'autre
    /// qu'un test. Les aides en prennent (`treat(name, dci, …)`) ou en
    /// rendent (`fn over() -> Overrides`), et sortent donc du filet.
    #[test]
    fn no_test_has_lost_its_attribute() {
        const SOURCES: &[(&str, &str)] = &[
            ("app.rs", include_str!("app.rs")),
            ("biology.rs", include_str!("biology.rs")),
            ("classes.rs", include_str!("classes.rs")),
            ("crush.rs", include_str!("crush.rs")),
            ("cyp.rs", include_str!("cyp.rs")),
            ("gravidity.rs", include_str!("gravidity.rs")),
            ("hepatic.rs", include_str!("hepatic.rs")),
            ("ordonnancier.rs", include_str!("ordonnancier.rs")),
            ("renal.rs", include_str!("renal.rs")),
            ("revue.rs", include_str!("revue.rs")),
            ("strings.rs", include_str!("strings.rs")),
            ("surveillance.rs", include_str!("surveillance.rs")),
            ("tables.rs", include_str!("tables.rs")),
        ];
        let mut orphans: Vec<String> = Vec::new();
        for (file, src) in SOURCES {
            let lines: Vec<&str> = src.lines().collect();
            let mut in_tests = false;
            for (i, l) in lines.iter().enumerate() {
                let t = l.trim_start();
                if t.starts_with("mod tests") {
                    in_tests = true;
                }
                if !in_tests {
                    continue;
                }
                // Sans paramètre et sans flèche : un test, et rien
                // d'autre.
                let Some(name) = t.strip_prefix("fn ") else {
                    continue;
                };
                let Some(name) = name.strip_suffix("() {") else {
                    continue;
                };
                // L'attribut est sur la dernière ligne qui n'est ni
                // vide, ni un commentaire, ni de la documentation.
                let attribute = lines[..i].iter().rev().find(|p| {
                    let p = p.trim_start();
                    !p.is_empty() && !p.starts_with("//")
                });
                if !attribute.is_some_and(|a| a.trim_start().starts_with("#[")) {
                    orphans.push(format!("{file}:{} — fn {name}()", i + 1));
                }
            }
        }
        assert!(
            orphans.is_empty(),
            "une fonction de test sans attribut : lui rendre son `#[test]`, \
             ou lui donner un paramètre ou un retour si c'est une aide.\n{}",
            orphans.join("\n")
        );
    }

    /// **Aucune liste de mots cherchés ne se répète.**
    ///
    /// Un doublon dans un `needs` ne casse rien — la règle attrape la
    /// même boîte deux fois pour le même prix — et c'est précisément
    /// pourquoi il s'installe. Il en est resté neuf, dont une liste
    /// d'IPP qui nommait quatre molécules deux fois chacune : c'est le
    /// résidu d'une expansion de classe en molécules faite deux fois, à
    /// deux endroits, par deux mains. Un mot en double est un mot que la
    /// relecture suivante croira être un autre.
    ///
    /// Lu dans le **texte** des modules, comme les deux tests d'à côté,
    /// parce que les `needs` de huit tables n'ont pas le même type et
    /// que la question, elle, est la même.
    #[test]
    fn no_list_of_searched_words_repeats_itself() {
        const SOURCES: &[(&str, &str)] = &[
            ("biology.rs", include_str!("biology.rs")),
            ("crush.rs", include_str!("crush.rs")),
            ("cyp.rs", include_str!("cyp.rs")),
            ("gravidity.rs", include_str!("gravidity.rs")),
            ("hepatic.rs", include_str!("hepatic.rs")),
            ("renal.rs", include_str!("renal.rs")),
            ("revue.rs", include_str!("revue.rs")),
            ("surveillance.rs", include_str!("surveillance.rs")),
        ];
        let mut offenders: Vec<String> = Vec::new();
        for (file, src) in SOURCES {
            for (i, line) in src.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") {
                    continue;
                }
                // Une liste sur une seule ligne, telle que `cargo fmt`
                // l'écrit dès qu'elle tient : c'est la forme de toutes
                // celles de ces tables.
                let Some(rest) = t
                    .strip_prefix("needs: &[")
                    .or_else(|| t.strip_prefix("never: &["))
                else {
                    continue;
                };
                let mut seen: Vec<&str> = Vec::new();
                for word in rest.split('"').skip(1).step_by(2) {
                    if seen.contains(&word) {
                        offenders.push(format!("{file}:{} — « {word} »", i + 1));
                    }
                    seen.push(word);
                }
            }
        }
        offenders.dedup();
        assert!(
            offenders.is_empty(),
            "un mot cherché deux fois dans la même liste :\n{}",
            offenders.join("\n")
        );
    }

    /// **Aucune table statique n'écrit de balisage dans ce qu'elle
    /// dessine**, et la question se pose une fois pour toutes plutôt
    /// qu'une fois par module.
    ///
    /// `RichText` n'interprète rien et le modèle Typst pas davantage :
    /// une astérisque tapée pour appuyer un mot arrive à l'écran, et
    /// parfois sur le papier de l'officine, comme une astérisque.
    /// Cinq modules avaient chacun leur propre test ; deux ne l'avaient
    /// pas, et c'est là qu'étaient les deux fautes — la note de famille
    /// du catalogue des stupéfiants et une phrase du plan de
    /// surveillance, qui s'imprime. Le sixième module écrit un jour
    /// aurait eu le même trou.
    ///
    /// Ce test-ci lit le **texte** de tous les modules, comme
    /// `no_font_size_is_written_in_pixels` lit celui de `app.rs`, et il
    /// couvre donc celui qu'on n'a pas encore écrit. Les commentaires
    /// et la documentation en écrivent, du balisage, et c'est très
    /// bien : ils ne vont nulle part. Seules les valeurs de champ sont
    /// lues.
    #[test]
    fn no_static_table_writes_markup_in_what_it_draws() {
        // Chaque fichier est lu par `include_str!` : le test suit donc
        // la source, et non ce qui se trouve sur le disque à l'exécution.
        const SOURCES: &[(&str, &str)] = &[
            ("biology.rs", include_str!("biology.rs")),
            ("crush.rs", include_str!("crush.rs")),
            ("entretien.rs", include_str!("entretien.rs")),
            ("gravidity.rs", include_str!("gravidity.rs")),
            ("hepatic.rs", include_str!("hepatic.rs")),
            ("insulin.rs", include_str!("insulin.rs")),
            ("ordonnance.rs", include_str!("ordonnance.rs")),
            ("ordonnancier.rs", include_str!("ordonnancier.rs")),
            ("renal.rs", include_str!("renal.rs")),
            ("revue.rs", include_str!("revue.rs")),
            ("selfcheck.rs", include_str!("selfcheck.rs")),
            ("surveillance.rs", include_str!("surveillance.rs")),
            ("tables.rs", include_str!("tables.rs")),
            ("vaccines.rs", include_str!("vaccines.rs")),
            ("vigilance.rs", include_str!("vigilance.rs")),
        ];
        // Les champs qui finissent sous les yeux de quelqu'un.
        const FIELDS: &[&str] = &[
            "note",
            "why",
            "text",
            "detail",
            "conduct",
            "label",
            "instead",
            "title",
            "source",
            "term",
            "pregnancy_note",
            "breastfeeding_note",
        ];
        // Assemblée, sinon le test se trouve lui-même.
        let markup = concat!("*", "*");
        let mut offenders: Vec<String> = Vec::new();
        for (file, src) in SOURCES {
            for (i, line) in src.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") {
                    continue;
                }
                let Some(rest) = FIELDS.iter().find_map(|f| {
                    t.strip_prefix(f)
                        .and_then(|r| r.strip_prefix(':'))
                        .map(str::trim_start)
                }) else {
                    continue;
                };
                if rest.starts_with('"') && rest.contains(markup) {
                    offenders.push(format!("{file}:{}", i + 1));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "du balisage dans ce qui se dessine, et rien ne l'interprète :\n{}",
            offenders.join("\n")
        );
    }
}
