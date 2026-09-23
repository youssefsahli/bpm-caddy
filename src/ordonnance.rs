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

/// Who the ordonnance is for, as far as the counter knows it.
///
/// **Every field is optional**, because a TROD is often done on
/// somebody whose file is a name and nothing else — quick entry must stay
/// quick. What is known changes what is offered; what is not known
/// blocks nothing, and the line's own situation text says what it
/// assumes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Who {
    pub age: Option<u32>,
    pub sex: Option<Sex>,
    pub pregnant: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sex {
    F,
    M,
}

impl Sex {
    /// The key stored in the base: `F`, `M`, or empty for « not said ».
    pub fn key(self) -> &'static str {
        match self {
            Sex::F => "F",
            Sex::M => "M",
        }
    }

    pub fn from_key(key: &str) -> Option<Sex> {
        match key.trim() {
            "F" | "f" => Some(Sex::F),
            "M" | "m" | "H" | "h" => Some(Sex::M),
            _ => None,
        }
    }

    /// What the NIR says, when nobody wrote the sex down: its first
    /// digit is 1 for a man and 2 for a woman. A temporary number (7, 8)
    /// says nothing, and neither does a missing one.
    pub fn from_nir(nir: &str) -> Option<Sex> {
        match nir.trim().chars().next() {
            Some('1') => Some(Sex::M),
            Some('2') => Some(Sex::F),
            _ => None,
        }
    }
}

/// Why a line does not apply to the person in front of the counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Barrier {
    TooYoung(u32),
    TooOld(u32),
    Sex(Sex),
    Pregnant,
}

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
    /// Who the line is for — the bounds of the protocol, in years,
    /// inclusive. `None` bounds nothing.
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
    pub sex: Option<Sex>,
    /// Whether the line may be dispensed to a pregnant woman.
    pub pregnancy: bool,
}

/// One line the officine offers after a positive TROD, as the base holds
/// it — **the shipped protocols are a starting point, seeded once**, and
/// everything after that is the team's: a molecule added, a posology
/// rewritten, a bound moved when the protocol changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Offer {
    pub id: i64,
    /// `angine` or `cystite`.
    pub protocol: String,
    pub rank: i64,
    pub name: String,
    pub situation: String,
    /// Ready-made posologies, one per line, most usual first.
    pub posologies: Vec<String>,
    pub caution: String,
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
    pub sex: Option<Sex>,
    pub pregnancy: bool,
}

impl Offer {
    /// What stops this line for `who`, the first reason found. Only a
    /// **known** fact stops anything: an unknown age is not a child.
    pub fn barrier(&self, who: &Who) -> Option<Barrier> {
        if let (Some(age), Some(min)) = (who.age, self.min_age) {
            if age < min {
                return Some(Barrier::TooYoung(min));
            }
        }
        if let (Some(age), Some(max)) = (who.age, self.max_age) {
            if age > max {
                return Some(Barrier::TooOld(max));
            }
        }
        if let (Some(sex), Some(wanted)) = (who.sex, self.sex) {
            if sex != wanted {
                return Some(Barrier::Sex(wanted));
            }
        }
        if who.pregnant && !self.pregnancy {
            return Some(Barrier::Pregnant);
        }
        None
    }
}

/// The shipped lines of a protocol, as rows to seed.
pub fn starter(id: &str) -> Vec<Offer> {
    protocols()
        .into_iter()
        .filter(|(pid, _)| *pid == id)
        .flat_map(|(pid, p)| {
            p.antibiotics
                .iter()
                .enumerate()
                .map(move |(rank, a)| Offer {
                    id: 0,
                    protocol: pid.to_owned(),
                    rank: rank as i64,
                    name: a.name.to_owned(),
                    situation: a.situation.to_owned(),
                    posologies: a.posologies.iter().map(|s| (*s).to_owned()).collect(),
                    caution: a.caution.to_owned(),
                    min_age: a.min_age,
                    max_age: a.max_age,
                    sex: a.sex,
                    pregnancy: a.pregnancy,
                })
        })
        .collect()
}

/// Every shipped line, both protocols.
pub fn starters() -> Vec<Offer> {
    protocols()
        .into_iter()
        .flat_map(|(id, _)| starter(id))
        .collect()
}

/// The key a protocol's rows carry in the base.
pub fn protocol_id(kind: InterviewKind) -> Option<&'static str> {
    match kind {
        InterviewKind::TrodAngine => Some("angine"),
        InterviewKind::TrodCystite => Some("cystite"),
        _ => None,
    }
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

/// Le document sous lequel les conseils imprimés sont adressés.
pub const DOC: &str = "ordonnance";

/// Les deux protocoles, avec le nom sous lequel ils sont adressés.
///
/// L'indication est ce qui les distingue et ne bouge pas — elle est le
/// titre imprimé en tête de l'ordonnance.
fn protocols() -> [(&'static str, &'static Protocol); 2] {
    [("angine", &ANGINE), ("cystite", &CYSTITE)]
}

/// Toutes les phrases imprimées sur l'ordonnance, avec leur adresse.
///
/// Les conseils et les temps de prise : ce sont les lignes qui partent
/// chez le patient. Les posologies n'en sont pas — ce sont des doses, et
/// elles s'accordent déjà avec les tables de référence, qu'un test tient
/// (`every_molecule_appears_in_its_reference_table`).
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for (id, p) in protocols() {
        for (n, c) in p.conseils.iter().enumerate() {
            out.push((crate::content::key_n(DOC, id, "conseil", n), "conseil", *c));
        }
        for (n, t) in p.temps_de_prise.iter().enumerate() {
            out.push((crate::content::key_n(DOC, id, "temps", n), "temps", *t));
        }
    }
    out
}

/// Les conseils d'un protocole, avec les mots de l'officine.
pub fn resolve_conseils(
    protocol: &Protocol,
    over: &crate::content::Overrides,
) -> (Vec<String>, Vec<String>) {
    let id = protocols()
        .into_iter()
        .find(|(_, p)| p.indication == protocol.indication)
        .map_or("", |(id, _)| id);
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
    (
        many("conseil", protocol.conseils),
        many("temps", protocol.temps_de_prise),
    )
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
            min_age: Some(15),
            max_age: None,
            sex: None,
            pregnancy: true,
        },
        Antibiotic {
            name: "Amoxicilline suspension buvable",
            situation: "Enfant de 10 à 14 ans, 1re intention",
            posologies: &["50 mg/kg/j en 2 prises pendant 6 jours"],
            caution: "Dose à rapporter au poids de l'enfant.",
            min_age: Some(10),
            max_age: Some(14),
            sex: None,
            pregnancy: true,
        },
        Antibiotic {
            name: "Céfuroxime-axétil 250 mg",
            situation: "Allergie aux pénicillines sans contre-indication aux céphalosporines",
            // Les doses de la recommandation, écrites : « selon la
            // recommandation en vigueur » s'imprimait tel quel sur
            // l'ordonnance remise au patient — sans dose, sans rythme,
            // sans durée (SPILF / HAS, angine de l'adulte).
            posologies: &["250 mg deux fois par jour pendant 4 jours"],
            caution: "Vérifier l'absence d'antécédent de réaction grave aux bêta-lactamines.",
            min_age: Some(15),
            max_age: None,
            sex: None,
            pregnancy: true,
        },
        Antibiotic {
            name: "Cefpodoxime-proxétil 100 mg",
            situation: "Allergie aux pénicillines sans contre-indication aux céphalosporines",
            posologies: &["100 mg deux fois par jour pendant 5 jours"],
            caution: "Vérifier l'absence d'antécédent de réaction grave aux bêta-lactamines.",
            min_age: Some(15),
            max_age: None,
            sex: None,
            pregnancy: true,
        },
        Antibiotic {
            name: "Azithromycine 250 mg",
            situation: "Contre-indication à toutes les bêta-lactamines",
            posologies: &["500 mg une fois par jour pendant 3 jours"],
            caution: "Prélèvement de gorge pour culture avant de traiter.",
            min_age: Some(15),
            max_age: None,
            sex: None,
            pregnancy: true,
        },
        Antibiotic {
            name: "Clarithromycine 250 mg",
            situation: "Contre-indication à toutes les bêta-lactamines",
            posologies: &["250 mg deux fois par jour pendant 5 jours"],
            caution: "Prélèvement de gorge pour culture avant de traiter ; nombreuses interactions.",
            min_age: Some(15),
            max_age: None,
            sex: None,
            pregnancy: true,
        },
    ],
    conseils: &[
        "Boire fréquemment, par petites quantités ; préférer les aliments tièdes et mous.",
        "Antalgique et antipyrétique à la demande sur la douleur et la fièvre.",
        "Éviter le tabac et les atmosphères enfumées pendant l'épisode.",
        "L'entourage n'est pas traité en l'absence de signes.",
        "Reconsulter sans attendre si la fièvre persiste au-delà de 3 jours, si la déglutition devient impossible, en cas de gêne respiratoire, d'éruption ou de gonflement du cou.",
    ],
    // **Des conseils qui valent pour chacune des molécules** : ils
    // parlaient de l'amoxicilline — « matin et soir », « aller au bout
    // des 6 jours » — et s'imprimaient sous l'azithromycine, qui se
    // prend une fois par jour pendant trois.
    temps_de_prise: &[
        "Prendre les doses à intervalle régulier, à heure fixe, comme l'ordonnance l'indique.",
        // Pas « indifféremment » pour tous : la table « Antibiotiques »
        // met céfuroxime, cefpodoxime et clarithromycine au repas, et le
        // céfuroxime-axétil s'absorbe mal à jeun.
        "Amoxicilline et azithromycine se prennent pendant ou en dehors des repas ; céfuroxime, cefpodoxime et clarithromycine, au cours d'un repas.",
        "Aller au bout du traitement prescrit même si la gorge va mieux dès le deuxième jour.",
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
            min_age: Some(16),
            max_age: Some(65),
            sex: Some(Sex::F),
            pregnancy: false,
        },
        Antibiotic {
            name: "Pivmécillinam 400 mg (Selexid)",
            situation: "2e intention",
            posologies: &[
                "400 mg deux fois par jour pendant 3 jours",
                "400 mg deux fois par jour pendant 5 jours",
            ],
            caution: "Contre-indiqué en cas d'allergie aux pénicillines. À avaler assis, avec un grand verre d'eau.",
            min_age: Some(16),
            max_age: Some(65),
            sex: Some(Sex::F),
            pregnancy: false,
        },
        Antibiotic {
            name: "Nitrofurantoïne 100 mg (Furadantine)",
            situation: "3e intention",
            posologies: &["100 mg trois fois par jour pendant 5 jours"],
            caution: "Jamais en traitement prolongé ni préventif ; contre-indiquée en cas d'insuffisance rénale. Prévenir de la coloration brune des urines.",
            min_age: Some(16),
            max_age: Some(65),
            sex: Some(Sex::F),
            pregnancy: false,
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
    kind.is_trod()
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
    /// The chosen line, by its identifier in the base — not an index:
    /// the list is the team's and may be edited while the box is open.
    pub antibiotic: Option<i64>,
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
    pub fn lines(&self, offers: &[Offer]) -> Vec<Line> {
        let mut out = Vec::new();
        if let Some(atb) = self
            .antibiotic
            .and_then(|id| offers.iter().find(|o| o.id == id))
        {
            out.push(Line {
                name: atb.name.clone(),
                posology: self.posology.trim().to_owned(),
                caution: atb.caution.clone(),
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
    ///
    /// Avec les mots de l'officine : ces lignes partent chez le patient,
    /// sous une ordonnance signée par elle. Le point unique où elles
    /// atteignent la page — il n'y en a pas d'autre, donc une réécriture
    /// ne peut pas manquer le papier.
    pub fn advice(&self, protocol: &Protocol, over: &crate::content::Overrides) -> Vec<String> {
        let (conseils, temps) = resolve_conseils(protocol, over);
        let mut out = Vec::new();
        if self.conseils {
            out.extend(conseils);
        }
        if self.temps_de_prise {
            out.extend(temps);
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
        let offers = numbered(starter("angine"));
        let choice = Choice {
            antibiotic: Some(offers[0].id),
            posology: "1 g deux fois par jour pendant 6 jours".to_owned(),
            adjuvant: Some("Lactéol".to_owned()),
            adjuvant_posology: "2 gélules deux fois par jour".to_owned(),
            conseils: true,
            temps_de_prise: false,
            extra: "Paracétamol 1 g si douleur\n\n  \n".to_owned(),
        };
        let lines = choice.lines(&offers);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].name, "Amoxicilline 1 g");
        assert_eq!(lines[0].posology, "1 g deux fois par jour pendant 6 jours");
        assert_eq!(lines[1].name, "Lactéol");
        // Blank lines in the free text do not become empty prescriptions.
        assert_eq!(lines[2].name, "Paracétamol 1 g si douleur");

        let advice = choice.advice(protocol, &crate::content::Overrides::default());
        assert_eq!(advice, protocol.conseils.to_vec());
        // Both toggles: the hygiène advice comes before the timing.
        let both = Choice {
            conseils: true,
            temps_de_prise: true,
            ..choice.clone()
        };
        assert_eq!(
            both.advice(protocol, &crate::content::Overrides::default())
                .len(),
            protocol.conseils.len() + protocol.temps_de_prise.len()
        );
        assert!(Choice::default()
            .advice(protocol, &crate::content::Overrides::default())
            .is_empty());
    }

    #[test]
    fn an_empty_choice_prints_nothing() {
        let offers = numbered(starter("cystite"));
        assert!(Choice::default().lines(&offers).is_empty());
        // A free-written line alone is still an ordonnance.
        let only_extra = Choice {
            extra: "Ibuprofène 400 mg".to_owned(),
            ..Default::default()
        };
        assert_eq!(only_extra.lines(&offers).len(), 1);
    }

    /// The shipped rows, numbered as the base would number them.
    fn numbered(mut offers: Vec<Offer>) -> Vec<Offer> {
        for (i, o) in offers.iter_mut().enumerate() {
            o.id = i as i64 + 1;
        }
        offers
    }

    /// **What is known changes what is offered; what is not known blocks
    /// nothing.** A TROD done on a name alone offers every line; a
    /// twelve-year-old gets the children's line; a man, a pregnant woman
    /// or a woman of seventy gets no cystitis line at all.
    #[test]
    fn a_known_age_sex_or_pregnancy_bounds_the_lines_and_an_unknown_one_does_not() {
        let angine = starter("angine");
        let unknown = Who::default();
        assert!(angine.iter().all(|o| o.barrier(&unknown).is_none()));
        let child = Who {
            age: Some(12),
            ..Who::default()
        };
        let open: Vec<&str> = angine
            .iter()
            .filter(|o| o.barrier(&child).is_none())
            .map(|o| o.name.as_str())
            .collect();
        assert_eq!(open, vec!["Amoxicilline suspension buvable"]);
        assert_eq!(angine[0].barrier(&child), Some(Barrier::TooYoung(15)));
        let small = Who {
            age: Some(6),
            ..Who::default()
        };
        assert!(angine.iter().all(|o| o.barrier(&small).is_some()));

        let cystite = starter("cystite");
        let man = Who {
            sex: Some(Sex::M),
            age: Some(40),
            ..Who::default()
        };
        assert!(cystite
            .iter()
            .all(|o| o.barrier(&man) == Some(Barrier::Sex(Sex::F))));
        let pregnant = Who {
            sex: Some(Sex::F),
            age: Some(30),
            pregnant: true,
        };
        assert!(cystite
            .iter()
            .all(|o| o.barrier(&pregnant) == Some(Barrier::Pregnant)));
        let older = Who {
            sex: Some(Sex::F),
            age: Some(70),
            pregnant: false,
        };
        assert!(cystite
            .iter()
            .all(|o| o.barrier(&older) == Some(Barrier::TooOld(65))));
        let woman = Who {
            sex: Some(Sex::F),
            age: Some(30),
            pregnant: false,
        };
        assert!(cystite.iter().all(|o| o.barrier(&woman).is_none()));
    }

    #[test]
    fn the_nir_says_the_sex_when_the_file_does_not() {
        assert_eq!(Sex::from_nir("2 84 05 75 111 222 33"), Some(Sex::F));
        assert_eq!(Sex::from_nir("185057511122233"), Some(Sex::M));
        assert_eq!(Sex::from_nir("7"), None);
        assert_eq!(Sex::from_nir(""), None);
        assert_eq!(Sex::from_key(Sex::F.key()), Some(Sex::F));
        assert_eq!(Sex::from_key(""), None);
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
    /// **Toute phrase qui part chez le patient s'édite, et toute
    /// réécriture arrive sur le papier.**
    ///
    /// Les deux sens, comme pour les huit autres documents réécrivables.
    /// Une phrase absente de `phrases()` ne peut pas être corrigée ; une
    /// phrase absente de la résolution **s'imprime telle que livrée
    /// pendant qu'on croit l'avoir corrigée**, ce qui est pire. Ce
    /// document-ci était le seul du registre à n'avoir ni l'un ni
    /// l'autre écrit noir sur blanc.
    #[test]
    fn every_ordonnance_advice_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        assert!(!listed.is_empty(), "aucune phrase à éditer");
        // Une adresse par phrase, et pas deux phrases à une adresse :
        // la seconde hériterait de la réécriture de la première.
        let mut keys: Vec<&String> = listed.iter().map(|(k, ..)| k).collect();
        keys.sort();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "deux phrases à une même adresse");

        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        // Tout coché : les deux familles de lignes partent sur la page.
        let choices = Choice {
            conseils: true,
            temps_de_prise: true,
            ..Choice::default()
        };
        let mut seen = 0usize;
        for (_, protocol) in protocols() {
            for line in choices.advice(protocol, &over) {
                assert!(line.starts_with("réécrit:"), "{line}");
                seen += 1;
            }
            // Et sans réécriture, ce sont les mots livrés.
            let plain = choices.advice(protocol, &crate::content::Overrides::default());
            assert_eq!(
                plain.len(),
                protocol.conseils.len() + protocol.temps_de_prise.len()
            );
            for line in &plain {
                assert!(!line.starts_with("réécrit:"), "{line}");
            }
        }
        assert_eq!(
            seen,
            listed.len(),
            "toutes les phrases listées doivent atteindre la page"
        );
    }
}
