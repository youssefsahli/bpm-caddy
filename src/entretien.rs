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
            "Un seul changement à mettre en place d'ici la prochaine fois",
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
            "Un objectif réaliste jusqu'à la prochaine fois",
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
            "Matériel et soins prévus à domicile : qui vient, à quelle heure",
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
            "Écrans, caféine après seize heures, alcool qui fragmente la nuit",
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
        let expected: usize = COMMON.len() + CHECKLISTS.iter().map(|(_, p)| p.len()).sum::<usize>();
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
