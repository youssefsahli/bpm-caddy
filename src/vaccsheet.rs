//! La feuille d'une vaccination à l'officine : ce qui se demande avant
//! l'injection, ce qui se trace pendant, ce qui se fait après.
//!
//! Comme le TROD (`trod.rs`), une vaccination n'est pas un entretien :
//! la fiche générique portait des cadres pour les propos du patient, et
//! le lot, le site d'injection et l'heure s'écrivaient dans la marge.
//! Cette feuille les tient dans l'ordre où ils se font.
//!
//! **Ce qu'elle ne décide pas.** Une réponse « oui » à une question
//! n'est pas une contre-indication calculée : c'est une question à
//! instruire, et le pharmacien décide. Les vaccins dus viennent du
//! calendrier (`vaccines.rs`), lu contre le carnet du patient — la
//! feuille les liste, elle ne les prescrit pas.
//!
//! Pur et testé ; chaque phrase se réécrit (`content.rs`).

/// Le document sous lequel les phrases de la feuille sont adressées.
pub const DOC: &str = "vaccination";

/// Ce qui se demande avant l'injection.
const QUESTIONS: &[&str] = &[
    "Allergie à un composant du vaccin, ou réaction grave à une dose précédente",
    "Fièvre ou infection aiguë ce jour",
    "Grossesse en cours ou possible",
    "Immunodépression ou traitement immunosuppresseur (vaccin vivant)",
    "Traitement anticoagulant ou trouble de la coagulation (voie intramusculaire)",
    "Malaise lors d'une injection précédente",
    "Autre vaccin reçu dans les quatre dernières semaines",
];

/// Ce qui se fait après l'injection.
const AFTER: &[&str] = &[
    "Surveillance de quinze minutes après l'injection",
    "Vaccination inscrite au carnet de vaccination",
    "Effets attendus expliqués : douleur au point d'injection, fièvre modérée",
    "Consigne d'appeler en cas de gêne respiratoire, de malaise ou d'éruption",
    "Prochaine dose ou prochain rappel indiqué au patient",
];

/// La feuille avec les mots de l'officine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filled {
    pub questions: Vec<String>,
    pub after: Vec<String>,
}

fn item() -> &'static str {
    "feuille"
}

/// Les phrases de la feuille, réécrites.
pub fn fill(over: &crate::content::Overrides) -> Filled {
    let many = |field: &str, shipped: &'static [&'static str]| -> Vec<String> {
        shipped
            .iter()
            .enumerate()
            .map(|(n, s)| {
                over.get(&crate::content::key_n(DOC, item(), field, n), s)
                    .to_owned()
            })
            .collect()
    };
    Filled {
        questions: many("question", QUESTIONS),
        after: many("apres", AFTER),
    }
}

/// Toutes les phrases de la feuille, avec leur adresse.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for (field, list) in [("question", QUESTIONS), ("apres", AFTER)] {
        for (n, s) in list.iter().enumerate() {
            out.push((crate::content::key_n(DOC, item(), field, n), field, *s));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une feuille se tient sur une page, et chaque ligne dit quelque
    /// chose.
    #[test]
    fn the_sheet_is_short_and_says_something() {
        assert!((4..=9).contains(&QUESTIONS.len()));
        assert!((3..=7).contains(&AFTER.len()));
        for p in QUESTIONS.iter().chain(AFTER) {
            assert!(p.len() > 15, "« {p} » trop court");
        }
    }

    /// **Chaque phrase se réécrit, et chaque réécriture arrive sur la
    /// feuille** — dans les deux sens.
    #[test]
    fn every_vaccination_phrase_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        let mut keys: Vec<&str> = listed.iter().map(|(k, _, _)| k.as_str()).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len(), "deux phrases à la même adresse");
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        let f = fill(&over);
        let all: Vec<&String> = f.questions.iter().chain(&f.after).collect();
        assert_eq!(
            all.len(),
            listed.len(),
            "une phrase listée, une phrase imprimée"
        );
        for p in all {
            assert!(p.starts_with("réécrit:vaccination."), "{p}");
        }
        let plain = fill(&crate::content::Overrides::default());
        assert_eq!(plain.questions, QUESTIONS.to_vec());
    }
}
