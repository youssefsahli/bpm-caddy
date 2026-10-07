//! Les conseils imprimés des fiches « Mesures et conseils » : la
//! compression médicale, les compléments nutritionnels oraux et les
//! protections périodiques réutilisables.
//!
//! Chaque phrase vient d'une source nommée dans le commentaire de sa
//! section, et chaque phrase se réécrit (`content.rs`) : l'équipe garde
//! ses mots, la feuille garde l'ordre.
//!
//! Pur et testé.

/// Une section d'une feuille : son champ (l'adresse des phrases), son
/// titre imprimé et ses lignes.
pub struct Section {
    pub field: &'static str,
    pub title: &'static str,
    pub lines: &'static [&'static str],
}

/// Une feuille de conseils.
pub struct Sheet {
    /// Le sujet sous lequel ses phrases sont adressées (`conseil.contention`).
    pub doc: &'static str,
    pub title: &'static str,
    pub sections: &'static [Section],
}

// Compression : ameli, « Comment utiliser vos chaussettes, bas ou
// collants de compression » (14/10/2025) ; SFMV, fiches patient 1 et 2
// (07/2022) et Lettre du médecin vasculaire n° 61 (12/2022).
const CONTENTION: &[Section] = &[
    Section {
        field: "pose",
        title: "Mise en place",
        lines: &[
            "Enfiler les bas le matin, dès le lever et après la toilette, sur des jambes sèches et sans crème.",
            "Retirer bagues et bracelets ; des gants de ménage fins facilitent la pose et protègent le tissu.",
            "Retourner le bas sur l'envers jusqu'au talon, enfiler le pied, placer le talon, puis dérouler le tissu vers le haut sans tirer et sans former de plis.",
            "En cas de difficulté à se pencher ou de manque de force dans les mains, un enfile-bas ou l'aide d'un tiers facilite la pose.",
            "Retirer les bas le soir au coucher, sauf indication contraire du prescripteur.",
        ],
    },
    Section {
        field: "port",
        title: "Port",
        lines: &[
            "Porter la compression chaque jour, y compris en été, en position debout ou assise.",
            "Ne pas rouler le haut du bas sur lui-même : le bourrelet crée un effet de garrot.",
        ],
    },
    Section {
        field: "entretien",
        title: "Entretien et renouvellement",
        lines: &[
            "Disposer de deux paires pour permettre un lavage en alternance.",
            "Laver à la main à l'eau tiède avec un savon neutre, ou en machine selon la notice du fabricant, sans adoucissant.",
            "Essorer sans tordre et sécher à plat, à l'abri d'une source de chaleur ; ni sèche-linge, ni repassage.",
            "Hydrater la peau le soir, après le retrait des bas.",
            "En port quotidien, les bas se renouvellent tous les 4 à 6 mois, ou plus tôt si la bande de maintien frise ou si la cheville devient lâche.",
        ],
    },
    Section {
        field: "alerte",
        title: "Consulter",
        lines: &[
            "Retirer les bas et consulter en cas de douleur, de froideur, de pâleur, de coloration bleutée ou de perte de sensibilité des orteils.",
            "Signaler toute irritation, démangeaison ou plaie de la peau, en particulier sur le tibia.",
            "Une jambe brutalement rouge, chaude, gonflée et douloureuse justifie un avis médical le jour même.",
        ],
    },
];

// CNO : HAS, stratégie de prise en charge en cas de dénutrition
// protéino-énergétique chez la personne âgée (2007) ; Assurance Maladie,
// mémo « Compléments nutritionnels oraux chez l'adulte » (08/2025) ;
// Cespharm, fiche d'aide à la délivrance.
const NUTRITION: &[Section] = &[
    Section {
        field: "prise",
        title: "Prise",
        lines: &[
            "Prendre les compléments en collation, à distance d'au moins deux heures d'un repas, ou pendant le repas en plus de celui-ci, jamais à la place.",
            "Respecter le nombre d'unités prescrit par jour ; une unité peut se consommer en plusieurs fois dans la journée.",
            "Une prise au lever ou au coucher évite un jeûne nocturne de plus de 12 heures.",
            "Servir de préférence frais ; ne réchauffer que si la notice le permet, sans porter à ébullition.",
            "Alterner les saveurs et les textures pour maintenir la consommation dans la durée.",
        ],
    },
    Section {
        field: "conservation",
        title: "Conservation",
        lines: &[
            "Avant ouverture : à température ambiante, à l'abri de la chaleur.",
            "Après ouverture : 2 heures au plus à température ambiante, ou 24 heures au réfrigérateur, refermé.",
        ],
    },
    Section {
        field: "alimentation",
        title: "Alimentation",
        lines: &[
            "Fractionner l'alimentation en plusieurs prises dans la journée.",
            "Enrichir les plats habituels : lait en poudre, fromage râpé, œuf, crème, beurre ou huile.",
        ],
    },
    Section {
        field: "suivi",
        title: "Suivi",
        lines: &[
            "Se peser une fois par mois, dans les mêmes conditions, et noter le poids.",
            "Signaler une perte de poids, une baisse d'appétit, des troubles digestifs ou une difficulté à terminer les unités.",
            "La première délivrance couvre 10 jours : la consommation et la tolérance sont vérifiées à la délivrance suivante.",
        ],
    },
];

// Protections périodiques réutilisables : décret n° 2023-1427 du
// 30/12/2023 (informations obligatoires sur les protections intimes) ;
// avis de l'ANSES 2026-SA-0069 (22/06/2026).
const PROTECTIONS: &[Section] = &[
    Section {
        field: "usage",
        title: "Utilisation",
        lines: &[
            "Se laver les mains avant la mise en place et avant le retrait.",
            "Laver ou désinfecter le produit avant la première utilisation, selon la notice du fabricant.",
            "Coupe menstruelle : une seule à la fois, 6 heures au plus, uniquement pendant les règles, d'un volume adapté au flux.",
            "La nuit, préférer une protection externe à la coupe menstruelle.",
            "Culotte menstruelle : la changer régulièrement, selon le flux ; la durée des essais de fabrication n'est pas une durée de port recommandée.",
        ],
    },
    Section {
        field: "entretien",
        title: "Entretien",
        lines: &[
            "Laver selon la notice du fabricant ; lorsque la notice l'autorise, un lavage à 60 °C limite mieux le risque microbiologique.",
            "Coupe menstruelle : nettoyer et désinfecter selon la notice entre deux cycles.",
        ],
    },
    Section {
        field: "alerte",
        title: "Syndrome de choc toxique",
        lines: &[
            "Le syndrome de choc toxique est une infection grave, potentiellement mortelle, associée aux protections internes.",
            "Retirer la coupe et consulter sans délai en cas de fièvre supérieure à 39 °C, de vomissements, de diarrhée, d'éruption évoquant un coup de soleil, de maux de gorge, de vertiges ou de malaise.",
            "Après un syndrome de choc toxique, ne plus utiliser de protection interne.",
        ],
    },
];

pub const SHEETS: [Sheet; 3] = [
    Sheet {
        doc: "conseil.contention",
        title: "Compression médicale",
        sections: CONTENTION,
    },
    Sheet {
        doc: "conseil.nutrition",
        title: "Compléments nutritionnels oraux",
        sections: NUTRITION,
    },
    Sheet {
        doc: "conseil.protections",
        title: "Protections périodiques réutilisables",
        sections: PROTECTIONS,
    },
];

/// La feuille dont le sujet est `doc`.
pub fn sheet(doc: &str) -> Option<&'static Sheet> {
    SHEETS.iter().find(|s| s.doc == doc)
}

fn item() -> &'static str {
    "feuille"
}

/// Une section, avec les mots de l'officine.
pub struct Filled {
    pub title: &'static str,
    pub lines: Vec<String>,
}

/// Les sections d'une feuille, réécrites.
pub fn fill(sheet: &Sheet, over: &crate::content::Overrides) -> Vec<Filled> {
    sheet
        .sections
        .iter()
        .map(|s| Filled {
            title: s.title,
            lines: s
                .lines
                .iter()
                .enumerate()
                .map(|(n, l)| {
                    over.get(&crate::content::key_n(sheet.doc, item(), s.field, n), l)
                        .to_owned()
                })
                .collect(),
        })
        .collect()
}

/// Toutes les phrases d'une feuille, avec leur adresse.
pub fn phrases(sheet: &Sheet) -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for s in sheet.sections {
        for (n, l) in s.lines.iter().enumerate() {
            out.push((
                crate::content::key_n(sheet.doc, item(), s.field, n),
                s.field,
                *l,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Chaque phrase se réécrit, et chaque réécriture arrive sur la
    /// feuille**, dans les deux sens, pour les trois feuilles.
    #[test]
    fn every_counsel_phrase_is_editable_and_every_rewrite_arrives() {
        for sheet in &SHEETS {
            let listed = phrases(sheet);
            let mut keys: Vec<&str> = listed.iter().map(|(k, _, _)| k.as_str()).collect();
            keys.sort_unstable();
            let n = keys.len();
            keys.dedup();
            assert_eq!(n, keys.len(), "deux phrases à la même adresse");
            let over = crate::content::Overrides::from_rows(
                listed
                    .iter()
                    .map(|(k, _, shipped)| {
                        (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned())
                    })
                    .collect::<Vec<_>>(),
            );
            let filled = fill(sheet, &over);
            let all: Vec<&String> = filled.iter().flat_map(|f| &f.lines).collect();
            assert_eq!(all.len(), listed.len());
            for p in all {
                assert!(p.starts_with(&format!("réécrit:{}.", sheet.doc)), "{p}");
            }
        }
    }

    /// Une feuille tient sur une page et chaque ligne est une consigne :
    /// une phrase se termine par un point, et dit quelque chose.
    #[test]
    fn every_line_is_a_sentence() {
        for sheet in &SHEETS {
            assert!(super::sheet(sheet.doc).is_some());
            let n: usize = sheet.sections.iter().map(|s| s.lines.len()).sum();
            assert!((6..=20).contains(&n), "{} : {n} lignes", sheet.doc);
            for s in sheet.sections {
                for l in s.lines {
                    assert!(l.ends_with('.'), "« {l} »");
                    assert!(l.len() > 30, "« {l} » trop court");
                }
            }
        }
    }
}
