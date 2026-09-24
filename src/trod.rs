//! La feuille d'un TROD : ce qui se vérifie, se lit et se trace.
//!
//! Un TROD n'est pas un entretien. La fiche d'entretien porte des
//! cadres pour les propos du patient et une liste de points à couvrir ;
//! un test rapide demande autre chose, et dans un ordre fixe : les
//! signes qui orientent d'emblée vers le médecin, le score qui décide
//! s'il y a lieu de tester (angine), la lecture du test, sa traçabilité
//! — lot, péremption, heure —, puis la conduite selon le résultat. Tout
//! cela s'écrivait à la main au dos de la fiche générique.
//!
//! **Ce que la feuille ne tranche pas.** Elle liste, elle ne décide
//! rien : les bornes d'âge et de sexe sont celles des lignes du
//! protocole que l'officine tient dans sa base (`trod_lines`), et la
//! feuille ne fait que les montrer pour la personne au comptoir. Chaque
//! phrase se réécrit (`content.rs`), parce que le protocole évolue et
//! que c'est l'officine qui le signe.
//!
//! Pur et testé : aucune base, aucune horloge — l'âge est passé.

use crate::db::InterviewKind;

/// Le document sous lequel les phrases de la feuille sont adressées.
pub const DOC: &str = "trod";

/// Un item d'un score clinique.
pub struct Criterion {
    pub label: &'static str,
    /// Ce qu'il ajoute au score, signe compris.
    pub points: i8,
    /// L'item d'âge : coché d'avance quand l'âge connu l'atteint. Seul
    /// un âge **connu** coche quoi que ce soit.
    pub from_age: Option<u32>,
}

/// Tout ce que la feuille d'un protocole imprime.
pub struct Sheet {
    /// La clé du protocole, celle des lignes de `trod_lines`.
    pub protocol: &'static str,
    pub title: &'static str,
    /// Les signes qui orientent vers le médecin sans tester.
    pub signs: &'static [&'static str],
    /// Le score qui décide du test ; vide quand le protocole n'en a pas.
    pub score: &'static [Criterion],
    /// Ce que dit le score, en une phrase ; vide sans score.
    pub score_rule: &'static str,
    /// Ce qui se lit sur le test, une case positif/négatif par lecture.
    pub readings: &'static [&'static str],
    /// La conduite quand le test est positif.
    pub positive: &'static [&'static str],
    /// La conduite quand il est négatif.
    pub negative: &'static [&'static str],
}

/// La feuille d'un acte, quand l'acte est un TROD.
pub fn sheet(kind: InterviewKind) -> Option<&'static Sheet> {
    match kind {
        InterviewKind::TrodAngine => Some(&ANGINE),
        InterviewKind::TrodCystite => Some(&CYSTITE),
        _ => None,
    }
}

const ANGINE: Sheet = Sheet {
    protocol: "angine",
    title: "TROD angine — streptocoque du groupe A",
    signs: &[
        "Difficulté à avaler sa salive, hypersalivation, voix étouffée",
        "Gêne respiratoire",
        "Trismus, torticolis, tuméfaction du cou ou du voile du palais",
        "Éruption cutanée",
        "Altération marquée de l'état général",
        "Immunodépression connue, chimiothérapie en cours",
        "Traitement exposant à une agranulocytose : carbimazole, thiamazole, clozapine, métamizole",
        "Angine récidivante, ou déjà traitée pour cet épisode",
    ],
    score: &[
        Criterion {
            label: "Fièvre supérieure à 38 °C",
            points: 1,
            from_age: None,
        },
        Criterion {
            label: "Absence de toux",
            points: 1,
            from_age: None,
        },
        Criterion {
            label: "Adénopathies cervicales sensibles",
            points: 1,
            from_age: None,
        },
        Criterion {
            label: "Amygdales augmentées de volume ou exsudat",
            points: 1,
            from_age: None,
        },
        Criterion {
            label: "Âge de 45 ans ou plus",
            points: -1,
            from_age: Some(45),
        },
    ],
    score_rule: "Score de Mac Isaac, à partir de 15 ans : TROD si le score atteint 2 ; en dessous, pas de test, traitement symptomatique.",
    readings: &["Streptocoque du groupe A"],
    positive: &[
        "Dispensation selon le protocole, ordonnance protocolisée remise",
        "Médecin traitant informé",
        "Conseils et signes qui imposent de reconsulter expliqués",
    ],
    negative: &[
        "Pas d'antibiotique : traitement symptomatique",
        "Reconsulter si la fièvre persiste ou si les signes s'aggravent",
    ],
};

const CYSTITE: Sheet = Sheet {
    protocol: "cystite",
    title: "TROD cystite — bandelette urinaire",
    signs: &[
        "Grossesse en cours ou possible",
        "Fièvre, frissons",
        "Douleur lombaire ou du flanc",
        "Nausées, vomissements",
        "Sang visible dans les urines",
        "Pertes vaginales ou démangeaisons",
        "Cystite récidivante, ou épisode récent déjà traité",
        "Anomalie connue de l'appareil urinaire, sonde, geste urologique récent",
        "Immunodépression, insuffisance rénale sévère",
    ],
    score: &[],
    score_rule: "",
    readings: &["Leucocytes", "Nitrites"],
    positive: &[
        "Leucocytes ou nitrites positifs : dispensation selon le protocole",
        "Médecin traitant informé",
        "Conseils et signes qui imposent de consulter expliqués",
    ],
    negative: &[
        "Leucocytes et nitrites négatifs : pas d'antibiotique",
        "Orienter vers le médecin si les signes persistent",
    ],
};

fn sheets() -> [&'static Sheet; 2] {
    [&ANGINE, &CYSTITE]
}

/// La feuille avec les mots de l'officine, et l'âge connu déjà coché.
#[derive(Clone, Debug, PartialEq)]
pub struct Filled {
    pub title: String,
    pub signs: Vec<String>,
    /// Chaque item : son libellé, ses points, et s'il est coché d'avance.
    pub score: Vec<(String, i8, bool)>,
    pub score_rule: String,
    pub readings: Vec<String>,
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

/// La feuille d'un protocole, réécrite et remplie de ce qui est su.
///
/// Seul l'item d'âge se coche, et seulement sur un âge connu : les
/// autres se constatent devant le patient, et une case cochée par le
/// logiciel se lit comme une case constatée.
pub fn fill(sheet: &Sheet, over: &crate::content::Overrides, age: Option<u32>) -> Filled {
    let id = sheet.protocol;
    let one = |field: &str, shipped: &'static str| -> String {
        over.get(&crate::content::key(DOC, id, field), shipped)
            .to_owned()
    };
    let many = |field: &str, shipped: &'static [&'static str]| -> Vec<String> {
        shipped
            .iter()
            .enumerate()
            .map(|(n, s)| {
                over.get(&crate::content::key_n(DOC, id, field, n), s)
                    .to_owned()
            })
            .collect()
    };
    Filled {
        title: one("titre", sheet.title),
        signs: many("signe", sheet.signs),
        score: sheet
            .score
            .iter()
            .enumerate()
            .map(|(n, c)| {
                let label = over
                    .get(&crate::content::key_n(DOC, id, "score", n), c.label)
                    .to_owned();
                let ticked = matches!((c.from_age, age), (Some(from), Some(a)) if a >= from);
                (label, c.points, ticked)
            })
            .collect(),
        score_rule: if sheet.score_rule.is_empty() {
            String::new()
        } else {
            one("regle", sheet.score_rule)
        },
        readings: many("lecture", sheet.readings),
        positive: many("positif", sheet.positive),
        negative: many("negatif", sheet.negative),
    }
}

/// Toutes les phrases des deux feuilles, avec leur adresse.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for sheet in sheets() {
        let id = sheet.protocol;
        out.push((crate::content::key(DOC, id, "titre"), "titre", sheet.title));
        let mut many = |field: &'static str, list: &'static [&'static str]| {
            for (n, s) in list.iter().enumerate() {
                out.push((crate::content::key_n(DOC, id, field, n), field, *s));
            }
        };
        many("signe", sheet.signs);
        many("lecture", sheet.readings);
        many("positif", sheet.positive);
        many("negatif", sheet.negative);
        for (n, c) in sheet.score.iter().enumerate() {
            out.push((crate::content::key_n(DOC, id, "score", n), "score", c.label));
        }
        if !sheet.score_rule.is_empty() {
            out.push((
                crate::content::key(DOC, id, "regle"),
                "regle",
                sheet.score_rule,
            ));
        }
    }
    out
}

/// Le résultat enregistré sur l'acte : `Some(true)` positif,
/// `Some(false)` négatif, `None` tant que le test n'est pas lu.
pub fn result(recorded: &str) -> Option<bool> {
    match recorded {
        crate::ordonnance::POSITIF => Some(true),
        crate::ordonnance::NEGATIF => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux actes TROD ont leur feuille, et aucun autre.
    #[test]
    fn only_the_two_trods_have_a_sheet() {
        for kind in InterviewKind::ALL {
            assert_eq!(sheet(kind).is_some(), kind.is_trod(), "{kind:?}");
        }
        // La clé du protocole est celle des lignes de la base : la
        // feuille et l'ordonnance parlent du même protocole.
        for kind in [InterviewKind::TrodAngine, InterviewKind::TrodCystite] {
            assert_eq!(
                Some(sheet(kind).unwrap().protocol),
                crate::ordonnance::protocol_id(kind)
            );
        }
    }

    /// Une feuille se tient sur une page : des listes courtes, des
    /// phrases qui disent quelque chose, et une lecture au moins.
    #[test]
    fn each_sheet_is_short_and_says_something() {
        for s in sheets() {
            assert!((4..=10).contains(&s.signs.len()), "{}", s.protocol);
            assert!(!s.readings.is_empty(), "{}", s.protocol);
            assert!(!s.positive.is_empty() && !s.negative.is_empty());
            for p in s.signs.iter().chain(s.positive).chain(s.negative) {
                assert!(p.len() > 10, "{} : « {p} » trop court", s.protocol);
            }
            // Un score sans sa règle serait des cases sans conclusion.
            assert_eq!(
                s.score.is_empty(),
                s.score_rule.is_empty(),
                "{}",
                s.protocol
            );
        }
    }

    /// **Seul un âge connu coche l'item d'âge**, et seulement celui-là.
    #[test]
    fn the_age_item_is_ticked_only_on_a_known_age() {
        let none = crate::content::Overrides::default();
        let ticked = |age| {
            fill(&ANGINE, &none, age)
                .score
                .iter()
                .filter(|(_, _, t)| *t)
                .count()
        };
        assert_eq!(ticked(None), 0, "âge inconnu");
        assert_eq!(ticked(Some(30)), 0);
        assert_eq!(ticked(Some(44)), 0);
        assert_eq!(ticked(Some(45)), 1);
        let filled = fill(&ANGINE, &none, Some(70));
        let age_item = filled.score.iter().find(|(_, _, t)| *t).unwrap();
        assert_eq!(age_item.1, -1, "l'âge retire un point");
        // Sans score, rien à cocher.
        assert!(fill(&CYSTITE, &none, Some(70)).score.is_empty());
    }

    /// **Chaque phrase de la feuille se réécrit, et chaque réécriture
    /// arrive sur la feuille** — dans un sens comme dans l'autre : une
    /// phrase listée est une phrase imprimée, et inversement.
    #[test]
    fn every_trod_phrase_is_editable_and_every_rewrite_arrives() {
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
        let mut printed = 0;
        for s in sheets() {
            let f = fill(s, &over, None);
            let all: Vec<&String> = std::iter::once(&f.title)
                .chain(&f.signs)
                .chain(f.score.iter().map(|(l, _, _)| l))
                .chain(std::iter::once(&f.score_rule).filter(|r| !r.is_empty()))
                .chain(&f.readings)
                .chain(&f.positive)
                .chain(&f.negative)
                .collect();
            for p in &all {
                assert!(p.starts_with("réécrit:trod."), "{} : {p}", s.protocol);
            }
            printed += all.len();
            // Sans réécriture, la feuille est celle qui est livrée.
            let plain = fill(s, &crate::content::Overrides::default(), None);
            assert_eq!(plain.title, s.title);
            assert_eq!(plain.signs, s.signs.to_vec());
        }
        assert_eq!(
            printed,
            listed.len(),
            "une phrase listée, une phrase imprimée"
        );
    }

    #[test]
    fn the_recorded_result_reads_three_ways() {
        assert_eq!(result("POSITIF"), Some(true));
        assert_eq!(result("NEGATIF"), Some(false));
        assert_eq!(result(""), None);
    }
}
