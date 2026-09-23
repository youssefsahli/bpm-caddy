//! Ce qu'un médicament fait dans le corps, **en valeurs** : la
//! pharmacocinétique (biodisponibilité, pic, liaison aux protéines,
//! volume de distribution, part éliminée inchangée, demi-vie) et la
//! pharmacodynamie (cible, délai et durée d'action, marge thérapeutique).
//!
//! Les fiches les disent en prose, quand elles les disent — et elles ne
//! les chiffrent presque jamais : deux biodisponibilités et aucune liaison
//! protéique sur 862 fiches livrées. Deux règles en découlent, un test
//! chacune.
//!
//! * **Aucun chiffre inventé.** Une valeur vient de la fiche — lue dans
//!   sa prose, et la phrase voyage avec elle — ou de l'officine, qui la
//!   saisit **avec sa source** (le RCP, § 5.2). Une propriété que personne
//!   n'a chiffrée s'affiche « non chiffrée » : c'est la règle de
//!   `dosing.rs`, et c'est ce qui dit quelle fiche compléter.
//! * **Ce que l'officine a sourcé l'emporte** sur ce que la prose laisse
//!   deviner, propriété par propriété — et jamais l'inverse : une lecture
//!   automatique ne remplace pas une valeur que quelqu'un a vérifiée.
//!
//! C'est la première tranche de la couche structurée que décrit
//! `docs/DATA_LAYER.md` : une propriété, une valeur, une source. Pur,
//! testé, sans base.

/// Une propriété que la fiche peut porter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Property {
    Bioavailability,
    Tmax,
    ProteinBinding,
    Volume,
    RenalUnchanged,
    HalfLife,
    Target,
    Onset,
    Duration,
    NarrowIndex,
}

/// Comment une propriété se dit : un nombre (ou une fourchette) dans une
/// unité, une phrase, ou un oui.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Number { unit: &'static str, max: f64 },
    Text,
    Flag,
}

impl Property {
    pub const ALL: [Property; 10] = [
        Property::Bioavailability,
        Property::Tmax,
        Property::ProteinBinding,
        Property::Volume,
        Property::RenalUnchanged,
        Property::HalfLife,
        Property::Target,
        Property::Onset,
        Property::Duration,
        Property::NarrowIndex,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Property::Bioavailability => "BIODISPONIBILITE",
            Property::Tmax => "TMAX",
            Property::ProteinBinding => "LIAISON",
            Property::Volume => "VOLUME",
            Property::RenalUnchanged => "INCHANGE",
            Property::HalfLife => "DEMI_VIE",
            Property::Target => "CIBLE",
            Property::Onset => "DELAI",
            Property::Duration => "DUREE",
            Property::NarrowIndex => "MARGE_ETROITE",
        }
    }

    pub fn from_key(key: &str) -> Option<Property> {
        Property::ALL.into_iter().find(|p| p.key() == key)
    }

    /// Pharmacocinétique, ou pharmacodynamie.
    pub fn is_kinetic(self) -> bool {
        matches!(
            self,
            Property::Bioavailability
                | Property::Tmax
                | Property::ProteinBinding
                | Property::Volume
                | Property::RenalUnchanged
                | Property::HalfLife
        )
    }

    pub fn shape(self) -> Shape {
        match self {
            Property::Bioavailability | Property::ProteinBinding | Property::RenalUnchanged => {
                Shape::Number {
                    unit: "%",
                    max: 100.0,
                }
            }
            Property::Tmax | Property::HalfLife | Property::Duration => Shape::Number {
                unit: "h",
                max: 24.0 * 365.0,
            },
            Property::Onset => Shape::Number {
                unit: "min",
                max: 24.0 * 60.0 * 30.0,
            },
            Property::Volume => Shape::Number {
                unit: "L/kg",
                max: 1000.0,
            },
            Property::Target => Shape::Text,
            Property::NarrowIndex => Shape::Flag,
        }
    }

    pub fn label(self) -> &'static str {
        use crate::strings::tr;
        match self {
            Property::Bioavailability => tr("pk_bioavailability"),
            Property::Tmax => tr("pk_tmax"),
            Property::ProteinBinding => tr("pk_binding"),
            Property::Volume => tr("pk_volume"),
            Property::RenalUnchanged => tr("pk_unchanged"),
            Property::HalfLife => tr("pk_half_life"),
            Property::Target => tr("pk_target"),
            Property::Onset => tr("pk_onset"),
            Property::Duration => tr("pk_duration"),
            Property::NarrowIndex => tr("pk_narrow"),
        }
    }
}

/// Une valeur, et d'où elle vient.
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    pub property: Property,
    pub low: Option<f64>,
    pub high: Option<f64>,
    /// La valeur telle qu'elle est écrite — une phrase pour une cible,
    /// la phrase de la fiche pour une valeur lue.
    pub text: String,
    /// Où la lire : « fiche » pour une lecture de la prose, sinon ce que
    /// l'officine a écrit (« RCP Eliquis, 5.2 »).
    pub source: String,
}

/// La source d'une valeur lue dans la fiche elle-même.
pub const FROM_CARD: &str = "fiche";

impl Fact {
    pub fn from_card(&self) -> bool {
        self.source == FROM_CARD
    }

    /// La valeur, écrite pour l'écran : « 60 à 80 % », « 2 h », « oui ».
    pub fn value(&self) -> String {
        // Whole figures without a decimal: « 99 % », never « 99,0 % ».
        let n = |v: f64| {
            if v.fract() == 0.0 {
                format!("{v:.0}")
            } else {
                crate::strings::decimal(v, 1)
            }
        };
        match (self.property.shape(), self.low, self.high) {
            (Shape::Number { unit, .. }, Some(low), Some(high)) if high > low => {
                format!("{} à {} {unit}", n(low), n(high))
            }
            (Shape::Number { unit, .. }, Some(low), _) => format!("{} {unit}", n(low)),
            (Shape::Flag, ..) => crate::strings::tr("pk_yes").to_owned(),
            _ => self.text.clone(),
        }
    }
}

/// Lire une valeur saisie par l'officine : « 60 », « 60 % », « 50-70 »,
/// « 0,5 à 1 ». Une fourchette reste une fourchette ; hors des bornes de
/// la propriété, c'est une faute de frappe et c'est refusé.
pub fn parse(property: Property, typed: &str, source: &str) -> Result<Fact, String> {
    let typed = typed.trim();
    let source = source.trim();
    if typed.is_empty() {
        return Err(crate::strings::tr("pk_err_empty").to_owned());
    }
    if source.is_empty() || source == FROM_CARD {
        return Err(crate::strings::tr("pk_err_source").to_owned());
    }
    let base = Fact {
        property,
        low: None,
        high: None,
        text: typed.to_owned(),
        source: source.to_owned(),
    };
    match property.shape() {
        Shape::Text | Shape::Flag => Ok(base),
        Shape::Number { max, .. } => {
            let numbers: Vec<f64> = typed
                .replace(',', ".")
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.parse::<f64>().ok())
                .collect();
            let (low, high) = match numbers.as_slice() {
                [one] => (*one, *one),
                [a, b, ..] => (a.min(*b), a.max(*b)),
                [] => return Err(crate::strings::tr("pk_err_number").to_owned()),
            };
            if low < 0.0 || high > max {
                return Err(crate::strings::tr("pk_err_range").to_owned());
            }
            Ok(Fact {
                low: Some(low),
                high: Some(high),
                ..base
            })
        }
    }
}

/// Les champs de la fiche que la lecture regarde.
pub struct Prose<'a> {
    pub half_life: &'a str,
    /// Tout le reste de la prose clinique, mis bout à bout.
    pub text: &'a str,
}

/// Ce que la prose d'une fiche chiffre d'elle-même. **Rien n'est
/// deviné** : une propriété n'est rendue que si la phrase la dit avec un
/// nombre et son unité, et la phrase voyage avec elle.
pub fn mine(prose: &Prose) -> Vec<Fact> {
    let mut out = Vec::new();
    if let Some(h) = parse_hours(prose.half_life) {
        out.push(Fact {
            property: Property::HalfLife,
            low: Some(h),
            high: Some(h),
            text: prose.half_life.trim().to_owned(),
            source: FROM_CARD.to_owned(),
        });
    }
    // Sentence by sentence: a value is read in the sentence that names
    // its property, and that sentence — as the card writes it — is what
    // travels with it.
    let sentences: Vec<(&str, String)> = prose
        .text
        .split_inclusive(['.', ';'])
        .map(|s| (s.trim(), crate::fuzzy::sort_key(s)))
        .filter(|(s, _)| !s.is_empty())
        .collect();
    let find = |words: &[&str]| {
        sentences.iter().find_map(|(original, folded)| {
            words.iter().find_map(|w| {
                folded
                    .find(w)
                    .map(|at| (*original, folded.as_str(), at + w.len()))
            })
        })
    };
    // Every sentence that names the property, in order: the first one
    // that also gives a figure is the one read — a sentence saying
    // « délai d'action rapide » does not stop the search.
    let each = |words: &str| -> Vec<(&str, &str, usize)> {
        sentences
            .iter()
            .filter_map(|(original, folded)| {
                folded
                    .find(words)
                    .map(|at| (*original, folded.as_str(), at + words.len()))
            })
            .collect()
    };
    if let Some((sentence, ..)) =
        find(&["marge therapeutique etroite", "index therapeutique etroit"])
    {
        out.push(Fact {
            property: Property::NarrowIndex,
            low: None,
            high: None,
            text: sentence.to_owned(),
            source: FROM_CARD.to_owned(),
        });
    }
    // « durée d'action de 12 heures », « biodisponibilité d'environ 60 % » :
    // the figure and its unit within the clause that follows the words.
    for (property, words) in [
        (Property::Duration, "duree d'action"),
        (Property::Onset, "delai d'action"),
        (Property::Bioavailability, "biodisponibilite"),
    ] {
        for (sentence, folded, after) in each(words) {
            let rest = &folded[after..];
            let clause = &rest[..rest.find([',', ';', '.']).unwrap_or(rest.len())];
            let value = match property {
                Property::Duration => hours_range(clause),
                Property::Onset => hours_range(clause).map(|(l, h)| (l * 60.0, h * 60.0)),
                _ => percent(clause),
            };
            if let Some((low, high)) = value {
                out.push(Fact {
                    property,
                    low: Some(low),
                    high: Some(high),
                    text: sentence.to_owned(),
                    source: FROM_CARD.to_owned(),
                });
                break;
            }
        }
    }
    out
}

/// « 60 % », « 60 à 80 % » dans une proposition : la fourchette que suit
/// un signe pour cent, et rien d'autre.
fn percent(clause: &str) -> Option<(f64, f64)> {
    let pct = clause.find('%')?;
    // The figures just before the sign: « 60 », or « 60 a 80 » (folded).
    let tail: String = clause[..pct]
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || matches!(c, ' ' | ',' | '.' | 'a' | '-'))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let numbers: Vec<f64> = tail
        .replace(',', ".")
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    let (low, high) = match numbers.as_slice() {
        [.., a, b] => (a.min(*b), a.max(*b)),
        [one] => (*one, *one),
        [] => return None,
    };
    (high <= 100.0).then_some((low, high))
}

/// Une ligne du tableau : la propriété, et ce qu'on en sait — la valeur
/// sourcée de l'officine, sinon celle de la fiche, sinon rien.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub property: Property,
    pub fact: Option<Fact>,
}

/// Toutes les propriétés, dans l'ordre, chacune avec la meilleure valeur
/// connue. **Ce que l'officine a sourcé l'emporte** sur la lecture.
pub fn rows(mined: &[Fact], stored: &[Fact]) -> Vec<Row> {
    Property::ALL
        .into_iter()
        .map(|property| Row {
            property,
            fact: stored
                .iter()
                .find(|f| f.property == property)
                .or_else(|| mined.iter().find(|f| f.property == property))
                .cloned(),
        })
        .collect()
}

/// A half-life in hours, read off the card's free text.
///
/// **The unit is the word that follows the figure**, never a word met
/// anywhere in the sentence. The first version picked « jours » or
/// « semaines » wherever they sat and applied them to the first figure:
/// aspirin's « 15 à 20 minutes … persiste 7 à 10 jours » read 420 hours,
/// Aricept's « ≈ 70 heures … deux à trois semaines » eleven thousand,
/// and the companion told the counter « il reste ≈ 96 % à 24 h ». A
/// figure that no time unit follows is passed over (« 50 % », « DFG
/// < 30 »), and one stuck to a letter or a hyphen is a name, not a
/// figure — « E-3174 », « GS-331007 », « M1 ».
pub fn parse_hours(text: &str) -> Option<f64> {
    hours_range(text).map(|(low, high)| (low + high) / 2.0)
}

/// [`parse_hours`], **fourchette gardée** : « 3 à 5 h » rend (3, 5). La
/// demi-vie d'une courbe se lit en son milieu ; une durée d'action se dit
/// telle que la fiche l'écrit.
pub fn hours_range(text: &str) -> Option<(f64, f64)> {
    #[derive(Debug)]
    enum Tok {
        Num(f64),
        Word(String),
    }
    // « Notion peu pertinente pour cette suspension : le délai d'action
    // est d'environ 1 heure 30 » gives a delay, not a half-life; a card
    // that says the notion does not apply is taken at its word.
    let folded = crate::fuzzy::sort_key(text);
    if ["sans objet", "non pertinente", "notion peu pertinente"]
        .iter()
        .any(|w| folded.trim_start().starts_with(&crate::fuzzy::sort_key(w)))
    {
        return None;
    }
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || ((chars[i] == ',' || chars[i] == '.')
                        && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit())))
            {
                i += 1;
            }
            // A figure glued to a name: a letter just before, or a hyphen
            // that itself follows a letter.
            let before = start.checked_sub(1).map(|k| chars[k]);
            let named = before.is_some_and(|b| b.is_alphabetic())
                || (before == Some('-')
                    && start
                        .checked_sub(2)
                        .is_some_and(|k| chars[k].is_alphanumeric() && !chars[k].is_ascii_digit()));
            let figure: String = chars[start..i].iter().collect();
            match figure.replace(',', ".").parse::<f64>() {
                Ok(v) if !named => toks.push(Tok::Num(v)),
                _ => toks.push(Tok::Word(String::new())),
            }
        } else if c.is_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            toks.push(Tok::Word(chars[start..i].iter().collect()));
        } else {
            if matches!(c, '-' | '–' | '—') {
                toks.push(Tok::Word("à".to_owned()));
            } else if !c.is_whitespace() && c != '≈' && c != '~' {
                // Any other sign ends a phrase: « 12 h ; 30 % » must not
                // read the 30 as the far end of a range.
                toks.push(Tok::Word(c.to_string()));
            }
            i += 1;
        }
    }
    let factor = |w: &str| match w {
        "h" | "heure" | "heures" => Some(1.0),
        "min" | "mn" | "minute" | "minutes" => Some(1.0 / 60.0),
        "j" | "jour" | "jours" => Some(24.0),
        "semaine" | "semaines" => Some(24.0 * 7.0),
        "s" | "seconde" | "secondes" => Some(1.0 / 3600.0),
        _ => None,
    };
    let mut k = 0;
    while k < toks.len() {
        if let Tok::Num(low) = toks[k] {
            let mut high = low;
            let mut next = k + 1;
            if let (Some(Tok::Word(w)), Some(Tok::Num(h))) = (toks.get(k + 1), toks.get(k + 2)) {
                if w == "à" || w == "a" || w == "et" {
                    high = *h;
                    next = k + 3;
                }
            }
            if let Some(Tok::Word(w)) = toks.get(next) {
                if let Some(f) = factor(w) {
                    return Some((low * f, high * f));
                }
            }
        }
        k += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Aucun chiffre inventé** : la prose ne rend que ce qu'elle chiffre
    /// avec son unité, la phrase voyage avec la valeur, et une propriété
    /// que personne n'a chiffrée reste vide.
    #[test]
    fn the_prose_gives_only_what_it_writes_with_a_unit() {
        let prose = Prose {
            half_life: "Environ 12 heures",
            text: "Anticoagulant à marge thérapeutique étroite. Sa durée d'action de 24 heures \
                   permet une prise par jour ; biodisponibilité d'environ 60 %, réduite à jeun.",
        };
        let found = mine(&prose);
        let get = |p: Property| found.iter().find(|f| f.property == p);
        assert_eq!(get(Property::HalfLife).and_then(|f| f.low), Some(12.0));
        assert_eq!(get(Property::Duration).and_then(|f| f.low), Some(24.0));
        assert_eq!(
            get(Property::Bioavailability).and_then(|f| f.low),
            Some(60.0)
        );
        let narrow = get(Property::NarrowIndex).expect("marge étroite");
        assert!(
            narrow.text.contains("marge thérapeutique étroite"),
            "{}",
            narrow.text
        );
        assert!(get(Property::ProteinBinding).is_none(), "jamais deviné");
        assert!(found.iter().all(Fact::from_card));
        // « biodisponibilité faible » : pas de chiffre, pas de valeur.
        let vague = mine(&Prose {
            half_life: "",
            text: "Biodisponibilité faible et variable.",
        });
        assert!(vague.is_empty());
    }

    /// **Ce que l'officine a sourcé l'emporte**, et une saisie se lit
    /// comme elle est écrite — fourchette comprise —, sans source refusée.
    #[test]
    fn a_sourced_value_wins_and_a_typed_one_must_carry_its_source() {
        let mined = mine(&Prose {
            half_life: "Environ 12 heures",
            text: "",
        });
        let typed = parse(Property::HalfLife, "10 à 14", "RCP Eliquis, 5.2").unwrap();
        assert_eq!((typed.low, typed.high), (Some(10.0), Some(14.0)));
        let table = rows(&mined, &[typed]);
        assert_eq!(table.len(), Property::ALL.len());
        let hl = table
            .iter()
            .find(|r| r.property == Property::HalfLife)
            .unwrap();
        assert_eq!(
            hl.fact.as_ref().map(|f| f.source.as_str()),
            Some("RCP Eliquis, 5.2")
        );
        assert!(table
            .iter()
            .find(|r| r.property == Property::Volume)
            .unwrap()
            .fact
            .is_none());
        assert!(
            parse(Property::ProteinBinding, "99", "").is_err(),
            "sans source"
        );
        assert!(parse(Property::ProteinBinding, "99", FROM_CARD).is_err());
        assert!(
            parse(Property::ProteinBinding, "150", "RCP").is_err(),
            "hors bornes"
        );
        assert!(parse(Property::ProteinBinding, "beaucoup", "RCP").is_err());
        assert_eq!(
            parse(Property::ProteinBinding, "99 %", "RCP")
                .unwrap()
                .value(),
            "99 %"
        );
        assert_eq!(
            parse(Property::Target, "facteur Xa", "RCP")
                .unwrap()
                .value(),
            "facteur Xa"
        );
    }

    #[test]
    fn every_key_reads_back() {
        for p in Property::ALL {
            assert_eq!(Property::from_key(p.key()), Some(p));
        }
    }
}
