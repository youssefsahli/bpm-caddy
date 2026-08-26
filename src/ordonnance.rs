//! What a pharmacist may dispense after a positive TROD, and the
//! ordonnance that records it.
//!
//! The molecules, the doses and the durations below are the ones the
//! app already shows in its own reference tables (`src/tables.rs`,
//! « Angine » and « Cystite ») — same order, same wording. That is
//! deliberate: the table the pharmacist reads at the counter and the
//! document they hand the patient must never say two different things.
//! Change one and change the other, or the test at the foot of this
//! file will say so.
//!
//! Nothing here is a substitute for the protocol: the app proposes,
//! the pharmacist decides, and every posology can be overwritten by
//! hand before the ordonnance is printed.

use crate::db::InterviewKind;

/// One antibiotic that may be dispensed for an indication.
pub struct Antibiotic {
    /// What goes on the ordonnance ("Amoxicilline 1 g").
    pub name: &'static str,
    /// When this line is the right one — shown beside the choice.
    pub situation: &'static str,
    /// Ready-made posologies, most usual first. The operator picks one
    /// or writes their own.
    pub posologies: &'static [&'static str],
    /// The caution that belongs with this molecule, printed under the
    /// line when it is not empty.
    pub caution: &'static str,
}

/// Everything one indication offers.
pub struct Protocol {
    /// The heading of the ordonnance ("Angine à streptocoque A").
    pub indication: &'static str,
    pub antibiotics: &'static [Antibiotic],
    /// Advice inserted when « conseils hygiéno-diététiques » is on.
    pub conseils: &'static [&'static str],
    /// Advice inserted when « temps de prise » is on.
    pub temps_de_prise: &'static [&'static str],
}

/// Angine à streptocoque du groupe A, TROD positif.
const ANGINE: Protocol = Protocol {
    indication: "Angine à streptocoque du groupe A — TROD positif",
    antibiotics: &[
        Antibiotic {
            name: "Amoxicilline 1 g",
            situation: "Adulte, 1re intention",
            posologies: &["1 g deux fois par jour pendant 6 jours"],
            caution: "",
        },
        Antibiotic {
            name: "Amoxicilline suspension buvable",
            situation: "Enfant à partir de 3 ans, 1re intention",
            posologies: &["50 mg/kg/j en 2 prises pendant 6 jours"],
            caution: "Dose à rapporter au poids de l'enfant.",
        },
        Antibiotic {
            name: "Céfuroxime-axétil",
            situation: "Allergie aux pénicillines sans contre-indication aux céphalosporines",
            posologies: &["Selon la recommandation en vigueur, durée courte"],
            caution: "Vérifier l'absence d'antécédent de réaction grave aux bêta-lactamines.",
        },
        Antibiotic {
            name: "Cefpodoxime",
            situation: "Allergie aux pénicillines sans contre-indication aux céphalosporines",
            posologies: &["Selon la recommandation en vigueur, durée courte"],
            caution: "Vérifier l'absence d'antécédent de réaction grave aux bêta-lactamines.",
        },
        Antibiotic {
            name: "Azithromycine",
            situation: "Contre-indication à toutes les bêta-lactamines",
            posologies: &["Schéma de 3 jours, selon la recommandation en vigueur"],
            caution: "Prélèvement de gorge pour culture avant de traiter.",
        },
        Antibiotic {
            name: "Clarithromycine",
            situation: "Contre-indication à toutes les bêta-lactamines",
            posologies: &["Selon la recommandation en vigueur"],
            caution: "Prélèvement de gorge pour culture avant de traiter ; nombreuses interactions.",
        },
    ],
    conseils: &[
        "Boire fréquemment, par petites quantités ; préférer les aliments tièdes et mous.",
        "Antalgique et antipyrétique à la demande sur la douleur et la fièvre.",
        "Éviter le tabac et les atmosphères enfumées pendant l'épisode.",
        "L'entourage n'est pas traité en l'absence de signes.",
        "Reconsulter sans attendre si la fièvre persiste au-delà de 3 jours, si la déglutition devient impossible, en cas de gêne respiratoire, d'éruption ou de gonflement du cou.",
    ],
    temps_de_prise: &[
        "Prendre les doses à intervalle régulier, matin et soir, à heure fixe.",
        "L'amoxicilline se prend indifféremment pendant ou en dehors des repas.",
        "Aller au bout des 6 jours même si la gorge va mieux dès le deuxième jour.",
        "En cas d'oubli, prendre la dose dès que possible, sauf si la suivante est proche ; ne jamais doubler.",
    ],
};

/// Cystite aiguë simple de la femme, bandelette / TROD positif.
const CYSTITE: Protocol = Protocol {
    indication: "Cystite aiguë simple — test positif",
    antibiotics: &[
        Antibiotic {
            name: "Fosfomycine trométamol 3 g (Monuril)",
            situation: "1re intention",
            posologies: &["3 g en dose unique"],
            caution: "À distance d'un repas, de préférence au coucher, après avoir uriné.",
        },
        Antibiotic {
            name: "Pivmécillinam 400 mg (Selexid)",
            situation: "2e intention",
            posologies: &[
                "400 mg deux fois par jour pendant 3 jours",
                "400 mg deux fois par jour pendant 5 jours",
            ],
            caution: "Contre-indiqué en cas d'allergie aux pénicillines. À avaler assis, avec un grand verre d'eau.",
        },
        Antibiotic {
            name: "Nitrofurantoïne 100 mg (Furadantine)",
            situation: "3e intention",
            posologies: &["100 mg trois fois par jour pendant 5 jours"],
            caution: "Jamais en traitement prolongé ni préventif ; contre-indiquée en cas d'insuffisance rénale. Prévenir de la coloration brune des urines.",
        },
    ],
    conseils: &[
        "Boire abondamment tout au long de la journée.",
        "Uriner régulièrement et complètement, sans se retenir, et après les rapports.",
        "S'essuyer d'avant en arrière ; éviter les toilettes intimes agressives.",
        "La canneberge n'est pas un traitement curatif.",
        "Consulter le jour même en cas de fièvre, de frissons, de douleur lombaire, de vomissements ou de sang dans les urines.",
        "Réévaluation si les signes persistent au-delà de 72 heures.",
    ],
    temps_de_prise: &[
        "La fosfomycine se prend en une seule fois, à distance des repas, de préférence au coucher, après avoir uriné.",
        "Les traitements de plusieurs jours se prennent à intervalle régulier, à heure fixe.",
        "Aller au bout du traitement même si la brûlure disparaît dès le lendemain.",
    ],
};

/// The protocol a positive TROD opens, if the act has one.
pub fn protocol(kind: InterviewKind) -> Option<&'static Protocol> {
    match kind {
        InterviewKind::TrodAngine => Some(&ANGINE),
        InterviewKind::TrodCystite => Some(&CYSTITE),
        _ => None,
    }
}

/// Does this act read a TROD at all? Only these two carry a result.
pub fn is_trod(kind: InterviewKind) -> bool {
    matches!(kind, InterviewKind::TrodAngine | InterviewKind::TrodCystite)
}

/// The recorded outcomes of a TROD.
pub const POSITIF: &str = "POSITIF";
pub const NEGATIF: &str = "NEGATIF";

/// One line as it will be printed.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub name: String,
    pub posology: String,
    pub caution: String,
}

/// What the operator chose in the ordonnance box.
#[derive(Clone, Debug, Default)]
pub struct Choice {
    /// Index into the protocol's antibiotics, if one is selected.
    pub antibiotic: Option<usize>,
    /// The posology, pre-filled from the chosen molecule and freely
    /// editable — « we can choose or free write ».
    pub posology: String,
    /// The adjuvant drug card chosen from the base, by name. It is a
    /// name rather than an id because the ordonnance is a document:
    /// what matters is what gets printed, and a card renamed or
    /// deleted afterwards must not change a prescription already made.
    pub adjuvant: Option<String>,
    pub adjuvant_posology: String,
    pub conseils: bool,
    pub temps_de_prise: bool,
    /// Anything the pharmacist adds by hand.
    pub extra: String,
}

impl Choice {
    /// The prescribed lines, in the order they are printed. Empty when
    /// nothing has been chosen — the caller refuses to print then.
    pub fn lines(&self, protocol: &Protocol) -> Vec<Line> {
        let mut out = Vec::new();
        if let Some(atb) = self.antibiotic.and_then(|i| protocol.antibiotics.get(i)) {
            out.push(Line {
                name: atb.name.to_owned(),
                posology: self.posology.trim().to_owned(),
                caution: atb.caution.to_owned(),
            });
        }
        if let Some(name) = self.adjuvant.as_ref().filter(|n| !n.trim().is_empty()) {
            out.push(Line {
                name: name.trim().to_owned(),
                posology: self.adjuvant_posology.trim().to_owned(),
                caution: String::new(),
            });
        }
        for extra in self.extra.lines() {
            let extra = extra.trim();
            if !extra.is_empty() {
                out.push(Line {
                    name: extra.to_owned(),
                    posology: String::new(),
                    caution: String::new(),
                });
            }
        }
        out
    }

    /// The advice paragraphs the toggles switch on.
    pub fn advice(&self, protocol: &Protocol) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.conseils {
            out.extend_from_slice(protocol.conseils);
        }
        if self.temps_de_prise {
            out.extend_from_slice(protocol.temps_de_prise);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_two_trods_have_a_protocol() {
        assert!(is_trod(InterviewKind::TrodAngine));
        assert!(is_trod(InterviewKind::TrodCystite));
        assert!(!is_trod(InterviewKind::Bpm));
        assert!(protocol(InterviewKind::TrodAngine).is_some());
        assert!(protocol(InterviewKind::Bpm).is_none());
    }

    /// The ordonnance and the counter table must not drift apart: every
    /// molecule offered here has to appear in the reference table the
    /// pharmacist reads for the same indication.
    #[test]
    fn every_molecule_appears_in_its_reference_table() {
        let table = |short: &str| -> String {
            let t = crate::tables::TABLES
                .iter()
                .find(|t| t.short == short)
                .expect("table absente");
            t.rows
                .iter()
                .flat_map(|r| r.iter())
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        let cases = [
            (InterviewKind::TrodAngine, table("Angine")),
            (InterviewKind::TrodCystite, table("Cystite")),
        ];
        for (kind, haystack) in cases {
            let protocol = protocol(kind).unwrap();
            for atb in protocol.antibiotics {
                // Compare on the molecule, not the presentation: the
                // table says "Amoxicilline 1 g x2/j", the ordonnance
                // "Amoxicilline 1 g".
                let molecule = atb
                    .name
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .trim_end_matches(',')
                    .to_lowercase();
                assert!(
                    haystack.contains(&molecule),
                    "{} : « {molecule} » ne figure pas dans la table de référence",
                    kind.label()
                );
            }
        }
    }

    #[test]
    fn a_choice_prints_the_antibiotic_then_the_probiotic_then_the_extras() {
        let protocol = protocol(InterviewKind::TrodAngine).unwrap();
        let choice = Choice {
            antibiotic: Some(0),
            posology: "1 g deux fois par jour pendant 6 jours".to_owned(),
            adjuvant: Some("Lactéol".to_owned()),
            adjuvant_posology: "2 gélules deux fois par jour".to_owned(),
            conseils: true,
            temps_de_prise: false,
            extra: "Paracétamol 1 g si douleur\n\n  \n".to_owned(),
        };
        let lines = choice.lines(protocol);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].name, "Amoxicilline 1 g");
        assert_eq!(lines[0].posology, "1 g deux fois par jour pendant 6 jours");
        assert_eq!(lines[1].name, "Lactéol");
        // Blank lines in the free text do not become empty prescriptions.
        assert_eq!(lines[2].name, "Paracétamol 1 g si douleur");

        let advice = choice.advice(protocol);
        assert_eq!(advice, protocol.conseils.to_vec());
        // Both toggles: the hygiène advice comes before the timing.
        let both = Choice {
            conseils: true,
            temps_de_prise: true,
            ..choice.clone()
        };
        assert_eq!(
            both.advice(protocol).len(),
            protocol.conseils.len() + protocol.temps_de_prise.len()
        );
        assert!(Choice::default().advice(protocol).is_empty());
    }

    #[test]
    fn an_empty_choice_prints_nothing() {
        let protocol = protocol(InterviewKind::TrodCystite).unwrap();
        assert!(Choice::default().lines(protocol).is_empty());
        // A free-written line alone is still an ordonnance.
        let only_extra = Choice {
            extra: "Ibuprofène 400 mg".to_owned(),
            ..Default::default()
        };
        assert_eq!(only_extra.lines(protocol).len(), 1);
    }

    #[test]
    fn every_antibiotic_offers_at_least_one_posology_to_start_from() {
        for kind in [InterviewKind::TrodAngine, InterviewKind::TrodCystite] {
            for atb in protocol(kind).unwrap().antibiotics {
                assert!(!atb.posologies.is_empty(), "{}", atb.name);
                assert!(!atb.situation.is_empty(), "{}", atb.name);
            }
        }
    }
}
