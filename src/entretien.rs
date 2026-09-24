//! What an entretien covers, theme by theme.
//!
//! The application prints the fiche and the pharmacist holds the
//! conversation. What it can do is make sure the sheet in their hand
//! carries the points that theme is for — the ones that are forgotten
//! when the entretien runs long, and the ones a patient rarely brings
//! up on their own.
//!
//! Static, pure and tested. The lists are short on purpose: a
//! checklist of twenty lines is a checklist nobody ticks.

/// The points to cover for one theme, in the order they are usually
/// asked. The theme is matched as the act stores it; anything else —
/// a theme the officine wrote itself — gets the common ground, which
/// is never wrong.
pub fn checklist(theme: &str) -> &'static [&'static str] {
    let folded = crate::fuzzy::sort_key(theme.trim());
    CHECKLISTS
        .iter()
        .find(|(key, _)| crate::fuzzy::sort_key(key) == folded)
        .map(|(_, points)| *points)
        .unwrap_or(COMMON)
}

/// Le document sous lequel les points de l'entretien sont adressés.
pub const DOC: &str = "entretien";

/// Le repère d'une thématique : son nom, replié. Le thème est ce qui ne
/// bouge pas — il est écrit sur chaque acte de la base — là où le rang
/// d'un point dans sa liste, lui, se décale dès qu'on en insère un.
fn item(theme: &str) -> String {
    crate::content::slug(theme)
}

/// Toutes les phrases de l'entretien, avec leur adresse.
///
/// Les douze thématiques **et** le fond commun : ce dernier s'imprime
/// dès qu'un acte porte un thème que l'officine a écrit elle-même, donc
/// il part sur du papier comme les autres et se réécrit comme les
/// autres.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    let mut push = |theme: &str, points: &'static [&'static str]| {
        let id = item(theme);
        for (n, point) in points.iter().enumerate() {
            out.push((crate::content::key_n(DOC, &id, "point", n), "point", *point));
        }
    };
    push(COMMON_THEME, COMMON);
    for (theme, points) in CHECKLISTS {
        push(theme, points);
    }
    // Les tableaux des types d'acte : leur titre, leurs colonnes, leurs
    // lignes — imprimés comme les points, réécrits comme eux.
    for t in tables() {
        let item = table_item(t);
        out.push((crate::content::key(DOC, &item, "titre"), "titre", t.title));
        for (n, c) in t.columns.iter().enumerate() {
            out.push((
                crate::content::key_n(DOC, &item, "colonne", n),
                "colonne",
                *c,
            ));
        }
        for (n, r) in t.rows.iter().enumerate() {
            out.push((crate::content::key_n(DOC, &item, "ligne", n), "ligne", *r));
        }
    }
    out
}

/// La liste de points d'une thématique, avec les mots de l'officine.
///
/// Sur la liste rendue et non sur le tableau : le module reste statique
/// et pur, et ses tests portent sur ce qui est livré. Même frontière que
/// les carnets et la revue.
pub fn resolve(theme: &str, over: &crate::content::Overrides) -> Vec<String> {
    let points = checklist(theme);
    // Le fond commun est adressé sous son propre nom : une thématique
    // que l'officine a inventée retombe dessus, et ses points ne sont
    // pas ceux d'un thème livré.
    let id = if std::ptr::eq(points, COMMON) {
        item(COMMON_THEME)
    } else {
        item(theme)
    };
    points
        .iter()
        .enumerate()
        .map(|(n, p)| {
            over.get(&crate::content::key_n(DOC, &id, "point", n), p)
                .to_owned()
        })
        .collect()
}

/// Un tableau propre à un type d'acte, imprimé sur la fiche entre les
/// points à couvrir et les cadres de notes : ce que l'entretien relève
/// en colonnes plutôt qu'en prose — les INR d'un patient sous AVK, les
/// grades des effets d'un anticancéreux.
///
/// Des cases vides : la feuille ne relève rien d'elle-même, elle dit où
/// l'écrire.
pub struct KindTable {
    /// L'adresse de ses phrases (`entretien.tableau-<clé>.…`).
    pub key: &'static str,
    pub title: &'static str,
    pub columns: &'static [&'static str],
    /// Les libellés des lignes ; vide, `blank_rows` lignes à remplir.
    pub rows: &'static [&'static str],
    pub blank_rows: usize,
}

const AVK_TABLE: KindTable = KindTable {
    key: "avk",
    title: "Suivi de l'INR",
    columns: &["Date", "INR", "Dose prise", "Prochain contrôle"],
    rows: &[],
    blank_rows: 5,
};

const AOD_TABLE: KindTable = KindTable {
    key: "aod",
    title: "Suivi du traitement anticoagulant",
    columns: &["Date", "Prises oubliées", "Saignement", "DFG"],
    rows: &[],
    blank_rows: 4,
};

const ANTICANCER_TABLE: KindTable = KindTable {
    key: "anticancereux",
    title: "Effets indésirables — grade de 0 à 4",
    columns: &["Effet", "0", "1", "2", "3", "4"],
    rows: &[
        "Fatigue",
        "Nausées, vomissements",
        "Diarrhée",
        "Mucite, aphtes",
        "Syndrome main-pied",
        "Éruption cutanée",
    ],
    blank_rows: 0,
};

const ASTHMA_TABLE: KindTable = KindTable {
    key: "asthme",
    title: "Technique d'inhalation observée",
    columns: &["Étape", "Oui", "Non"],
    rows: &[
        "Expiration complète avant la prise",
        "Embout bien serré entre les lèvres",
        "Inspiration adaptée au dispositif",
        "Apnée de quelques secondes",
        "Rinçage de la bouche après un corticoïde inhalé",
    ],
    blank_rows: 0,
};

const BPM_TABLE: KindTable = KindTable {
    key: "bpm",
    title: "Analyse des traitements",
    columns: &[
        "Médicament",
        "Problème repéré",
        "Proposition au prescripteur",
    ],
    rows: &[],
    blank_rows: 4,
};

/// Les sujets d'un rendez-vous de prévention sont déjà sur la fiche,
/// cochés dans la liste de l'officine (`config.prevention`) : le tableau
/// porte ce qui en sort, le plan convenu avec le patient.
const PREVENTION_TABLE: KindTable = KindTable {
    key: "prevention",
    title: "Plan personnalisé de prévention",
    columns: &["Sujet", "Objectif convenu", "Orientation"],
    rows: &[],
    blank_rows: 3,
};

fn tables() -> [&'static KindTable; 6] {
    [
        &AVK_TABLE,
        &AOD_TABLE,
        &ANTICANCER_TABLE,
        &ASTHMA_TABLE,
        &BPM_TABLE,
        &PREVENTION_TABLE,
    ]
}

/// Le tableau d'un type d'acte, quand il en a un.
pub fn kind_table(kind: crate::db::InterviewKind) -> Option<&'static KindTable> {
    use crate::db::InterviewKind as K;
    match kind {
        K::Avk => Some(&AVK_TABLE),
        K::Aod => Some(&AOD_TABLE),
        K::AnticancereuxLc | K::AnticancereuxAutres => Some(&ANTICANCER_TABLE),
        K::Asthme => Some(&ASTHMA_TABLE),
        K::Bpm => Some(&BPM_TABLE),
        K::Prevention => Some(&PREVENTION_TABLE),
        _ => None,
    }
}

/// Un tableau avec les mots de l'officine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilledTable {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<String>,
    pub blank_rows: usize,
}

fn table_item(t: &KindTable) -> String {
    format!("tableau-{}", t.key)
}

/// Le tableau d'un type d'acte, réécrit par l'officine.
pub fn resolve_table(
    kind: crate::db::InterviewKind,
    over: &crate::content::Overrides,
) -> Option<FilledTable> {
    let t = kind_table(kind)?;
    let item = table_item(t);
    let many = |field: &str, shipped: &'static [&'static str]| -> Vec<String> {
        shipped
            .iter()
            .enumerate()
            .map(|(n, s)| {
                over.get(&crate::content::key_n(DOC, &item, field, n), s)
                    .to_owned()
            })
            .collect()
    };
    Some(FilledTable {
        title: over
            .get(&crate::content::key(DOC, &item, "titre"), t.title)
            .to_owned(),
        columns: many("colonne", t.columns),
        rows: many("ligne", t.rows),
        blank_rows: t.blank_rows,
    })
}

/// Le nom sous lequel le fond commun est adressé. Ce n'est pas une
/// thématique de la base : c'est ce qui s'imprime quand le thème n'en
/// est pas une.
const COMMON_THEME: &str = "Fond commun";

/// What every entretien covers, whatever its theme.
///
/// A `static`, not a `const`: a const is inlined at each use site, and
/// the fallback must be recognisable by identity — the tests compare
/// pointers, and so could a caller.
static COMMON: &[&str] = &[
    "Compréhension du traitement, reformulée par le patient",
    "Prises réelles et horaires, produits hors ordonnance compris",
    "Automédication, plantes et compléments alimentaires",
    "Gêne ressentie au quotidien",
    "Conduite en cas d'oubli",
    "Changements depuis le dernier entretien",
    "Décision retenue pour le prochain entretien",
];

/// One list per thematic, keyed on the theme as the act stores it.
const CHECKLISTS: &[(&str, &[&str])] = &[
    (
        "Initiation / bon usage",
        &[
            "Objectif du traitement, formulé par le patient",
            "Plan de prise : quantité, horaires, repas",
            "Effet attendu et délai d'action",
            "Effets indésirables des premiers jours, et ceux qui imposent d'appeler",
            "Conduite en cas d'oubli",
            "Durée prévue et conséquences d'un arrêt",
            "Éléments à signaler à un autre pharmacien ou médecin",
        ],
    ),
    (
        "Observance",
        &[
            "Nombre de prises oubliées dans la semaine écoulée, sans jugement",
            "Moment de la journée où surviennent les oublis",
            "Gêne : goût, taille, nombre de boîtes, horaires, coût",
            "Représentations du traitement : utilité, dépendance, durée",
            "Aides possibles : pilulier, alarme, association à un geste quotidien",
            "Solutions déjà essayées sans résultat",
            "Un seul changement à mettre en place avant le prochain entretien",
        ],
    ),
    (
        "Biologie / INR",
        &[
            "Date et résultat du dernier contrôle, carnet en main",
            "Cible du patient, et connaissance qu'il en a",
            "Changements depuis : traitement, alimentation, épisode aigu",
            "Signes de surdosage et de sous-dosage, en termes simples",
            "Conduite en cas de saignement ou de valeur anormale",
            "Date du prochain contrôle, notée avant le départ",
        ],
    ),
    (
        "Effets indésirables",
        &[
            "Effets apparus depuis l'instauration, et leur date",
            "Réaction du patient : arrêt, réduction, automédication",
            "Effets attendus et transitoires, à distinguer des autres",
            "Signes imposant l'arrêt et un appel",
            "Corrections possibles : horaire, prise au repas, forme galénique",
            "Déclaration en pharmacovigilance si l'effet le justifie",
        ],
    ),
    (
        "Interactions",
        &[
            "Ordonnance complète, celle des autres prescripteurs comprise",
            "Automédication, plantes, compléments, produits achetés sur internet",
            "Pamplemousse, millepertuis, alcool : pertinence pour ce traitement",
            "Traitements ajoutés ou arrêtés récemment",
            "Points à vérifier avant toute nouvelle délivrance",
            "Éléments transmis au médecin traitant",
        ],
    ),
    (
        "Technique d'inhalation",
        &[
            "Démonstration par le patient, dispositif en main",
            "Armement, inspiration, apnée : les trois temps",
            "Erreur propre à son dispositif",
            "Chambre d'inhalation : utilité, lavage, séchage",
            "Rinçage de la bouche après le corticoïde",
            "Consommation du traitement de secours dans le mois",
            "Conduite en cas de crise, et moment de l'appel",
        ],
    ),
    (
        "Vie quotidienne / diététique",
        &[
            "Repas, horaires, appétit, poids",
            "Alcool et tabac, sans jugement",
            "Activité physique possible et acceptée",
            "Sommeil et fatigue",
            "Voyages, chaleur, jeûne : adaptations du traitement",
            "Conduite automobile et travail",
            "Un objectif réaliste pour le prochain entretien",
        ],
    ),
    (
        "Automédication",
        &[
            "Produits pris sans ordonnance, et leur motif",
            "Antalgiques : lequel, combien, depuis quand",
            "Plantes et compléments, y compris ceux offerts par un proche",
            "Incompatibilités avec le traitement en cours",
            "Produits utilisables sans risque, et à quelle dose",
            "Quand consulter plutôt que se traiter",
        ],
    ),
    (
        "Sortie d'hôpital / conciliation",
        &[
            "Ordonnance de sortie ligne par ligne, contre celle d'avant l'hospitalisation",
            "Traitements arrêtés, traitements ajoutés, doses modifiées",
            "Boîtes rapportées du domicile, susceptibles d'être reprises",
            "Traitements suspendus pendant le séjour : lesquels reprennent, et quand",
            "Répartition du suivi, et date du prochain rendez-vous",
            "Examens à faire dans les jours qui suivent, biologie comprise",
            "Soins et matériel à domicile : intervenants et horaires",
            "Motifs de rappeler le médecin sans attendre la consultation",
        ],
    ),
    (
        "Douleur chronique",
        &[
            "Intensité aujourd'hui et sur la semaine, avec la même échelle à chaque fois",
            "Retentissement : marche, sommeil, travail, sorties",
            "Traitement de fond : pris comme prescrit, ou à la demande",
            "Interdoses : combien par jour, et à quel moment elles reviennent",
            "Constipation, somnolence et nausées sous opioïde",
            "Prises complémentaires, achats hors ordonnance compris",
            "Approches non médicamenteuses essayées, et à envisager",
        ],
    ),
    (
        "Sommeil",
        &[
            "Heure du coucher, heure du lever, et temps réellement passé au lit",
            "Délai d'endormissement, nombre de réveils, et leur heure",
            "Produits pris pour dormir, depuis quand, et à quelle dose",
            "Écrans, caféine après 16 h, alcool le soir",
            "Sieste : durée et horaire",
            "Ronflement, pauses respiratoires signalées par l'entourage",
            "Objectif de décroissance si un hypnotique dure depuis plus de quatre semaines",
        ],
    ),
    (
        "Chute et autonomie",
        &[
            "Chute dans les douze derniers mois, et circonstances",
            "Ordonnance relue pour les médicaments à risque de chute : psychotropes, antihypertenseurs, diurétiques",
            "Tension mesurée couchée puis debout, et vertige au lever",
            "Vue, audition, et date du dernier contrôle de chacune",
            "Chaussage, tapis, éclairage de nuit, barre dans la salle de bain",
            "Activité physique de la semaine, et peur de retomber",
            "Vitamine D, calcium alimentaire, et dernier poids connu",
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_thematic_of_the_base_has_its_checklist() {
        // The thematics only ever grow: one withdrawn is an entretien
        // the officine can no longer bill under its own name.
        assert!(
            crate::db::THEMES.len() >= 12,
            "{} thèmes livrés, il y en avait douze",
            crate::db::THEMES.len()
        );
        let mut seen: Vec<&str> = crate::db::THEMES.to_vec();
        seen.sort_unstable();
        let n = seen.len();
        seen.dedup();
        assert_eq!(n, seen.len(), "deux thèmes portent le même nom");
        for theme in crate::db::THEMES {
            let points = checklist(theme);
            assert!(
                !std::ptr::eq(points, COMMON),
                "thème sans liste propre : {theme}"
            );
            assert!(
                (5..=9).contains(&points.len()),
                "{theme} : {} points, une liste se tient sur une fiche",
                points.len()
            );
            for point in points {
                assert!(point.len() > 15, "{theme} : « {point} » trop court");
            }
        }
    }

    /// **Chaque point de l'entretien s'édite, et chaque réécriture
    /// arrive sur la fiche.**
    ///
    /// La liste part sur le papier de chaque entretien : c'est le
    /// document que le pharmacien a sous les yeux face au patient.
    #[test]
    fn every_checklist_point_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        let tables_len: usize = tables()
            .iter()
            .map(|t| 1 + t.columns.len() + t.rows.len())
            .sum();
        let expected: usize =
            COMMON.len() + CHECKLISTS.iter().map(|(_, p)| p.len()).sum::<usize>() + tables_len;
        assert_eq!(listed.len(), expected, "un point, une adresse");

        // Une adresse par point, fond commun compris.
        let mut keys: Vec<&str> = listed.iter().map(|(k, _, _)| k.as_str()).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len(), "deux points à la même adresse");

        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        for theme in crate::db::THEMES {
            let mine = resolve(theme, &over);
            assert_eq!(mine.len(), checklist(theme).len(), "{theme}");
            for point in &mine {
                assert!(point.starts_with("réécrit:"), "{theme} : {point}");
            }
            // Sans réécriture, la liste est celle qui est livrée.
            let plain = resolve(theme, &crate::content::Overrides::default());
            assert_eq!(plain, checklist(theme).to_vec(), "{theme}");
        }

        // **Le fond commun aussi.** Il s'imprime dès qu'un acte porte un
        // thème que l'officine a écrit elle-même, donc il part sur du
        // papier — et il se réécrit sous son propre nom, sans emprunter
        // l'adresse d'une thématique livrée.
        let invented = resolve("Entretien du mardi", &over);
        assert_eq!(invented.len(), COMMON.len());
        for point in &invented {
            assert!(point.starts_with("réécrit:"), "fond commun : {point}");
        }
    }

    /// **Les tableaux des types d'acte se réécrivent, et chaque
    /// réécriture arrive sur la fiche** — titre, colonnes, lignes.
    #[test]
    fn every_kind_table_phrase_is_editable_and_every_rewrite_arrives() {
        use crate::db::InterviewKind as K;
        let listed: Vec<(String, &str, &str)> = phrases()
            .into_iter()
            .filter(|(k, _, _)| k.contains(".tableau-"))
            .collect();
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        let mut printed = 0;
        for kind in [
            K::Avk,
            K::Aod,
            K::AnticancereuxLc,
            K::Asthme,
            K::Bpm,
            K::Prevention,
        ] {
            let t = resolve_table(kind, &over).unwrap();
            for p in std::iter::once(&t.title).chain(&t.columns).chain(&t.rows) {
                assert!(
                    p.starts_with("réécrit:entretien.tableau-"),
                    "{kind:?} : {p}"
                );
                printed += 1;
            }
        }
        assert_eq!(
            printed,
            listed.len(),
            "une phrase listée, une phrase imprimée"
        );
        // Les deux anticancéreux partagent le leur ; les autres actes
        // n'en ont pas.
        assert_eq!(
            resolve_table(K::AnticancereuxAutres, &over),
            resolve_table(K::AnticancereuxLc, &over)
        );
        assert!(kind_table(K::TrodAngine).is_none());
        assert!(kind_table(K::Vaccination).is_none());
        // Chaque tableau porte des lignes, écrites ou à remplir.
        for t in tables() {
            assert!(!t.rows.is_empty() || t.blank_rows > 0, "{}", t.key);
            assert!(t.columns.len() >= 3, "{}", t.key);
        }
    }

    #[test]
    fn an_unknown_theme_falls_back_to_the_common_ground() {
        // A theme the officine wrote itself, or none at all.
        assert!(std::ptr::eq(checklist(""), COMMON));
        assert!(std::ptr::eq(
            checklist("Entretien de sortie d'hôpital"),
            COMMON
        ));
        // The match ignores case and accents, as everywhere else.
        assert_eq!(checklist("observance").len(), checklist("Observance").len());
        assert!(!std::ptr::eq(checklist("biologie / inr"), COMMON));
    }
}
