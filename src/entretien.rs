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

/// What every entretien covers, whatever its theme.
///
/// A `static`, not a `const`: a const is inlined at each use site, and
/// the fallback must be recognisable by identity — the tests compare
/// pointers, and so could a caller.
static COMMON: &[&str] = &[
    "Ce que le patient a compris de son traitement, dans ses mots",
    "Ce qu'il prend réellement, horaires compris — y compris ce qui n'est pas sur l'ordonnance",
    "Automédication, plantes et compléments alimentaires",
    "Ce qui le gêne au quotidien",
    "Ce qu'il fait en cas d'oubli",
    "Ce qui a changé depuis la dernière fois",
    "Ce qu'on décide ensemble pour la prochaine fois",
];

/// One list per thematic, keyed on the theme as the act stores it.
const CHECKLISTS: &[(&str, &[&str])] = &[
    (
        "Initiation / bon usage",
        &[
            "À quoi sert ce traitement, dit par le patient",
            "Le plan de prise : combien, quand, avec ou sans repas",
            "Ce qu'on attend comme effet, et en combien de temps",
            "Les effets indésirables des premiers jours, et ceux qui font appeler",
            "Ce qu'on fait en cas d'oubli",
            "La durée prévue, et ce qui se passe si on l'arrête",
            "Ce qu'il faut signaler à une autre pharmacie ou à un autre médecin",
        ],
    ),
    (
        "Observance",
        &[
            "Nombre de prises oubliées dans la semaine écoulée, sans jugement",
            "À quel moment de la journée les oublis arrivent",
            "Ce qui gêne : goût, taille, nombre de boîtes, horaires, coût",
            "Ce que le patient croit du traitement — utilité, dépendance, durée",
            "Pilulier, alarme, association à un geste quotidien : ce qui pourrait aider",
            "Ce qui a déjà été essayé et n'a pas marché",
            "Un seul changement à mettre en place d'ici la prochaine fois",
        ],
    ),
    (
        "Biologie / INR",
        &[
            "Date et résultat du dernier contrôle, carnet en main",
            "La cible du patient, et s'il la connaît",
            "Ce qui a changé depuis : traitement, alimentation, épisode aigu",
            "Les signes de surdosage et de sous-dosage, en clair",
            "Ce qu'il fait en cas de saignement ou de valeur anormale",
            "La date du prochain contrôle, notée avant de partir",
        ],
    ),
    (
        "Effets indésirables",
        &[
            "Ce qui est apparu depuis l'instauration, et quand",
            "Ce que le patient a fait — arrêt, réduction, automédication",
            "Ce qui est attendu et transitoire, et ce qui ne l'est pas",
            "Les signes qui imposent d'arrêter et d'appeler",
            "Ce qui peut se corriger : horaire, prise pendant le repas, forme",
            "Déclaration en pharmacovigilance si l'effet le justifie",
        ],
    ),
    (
        "Interactions",
        &[
            "L'ordonnance complète, celle des autres prescripteurs comprise",
            "Automédication, plantes, compléments, produits achetés sur internet",
            "Pamplemousse, millepertuis, alcool : ce qui compte pour ce traitement",
            "Ce qui a été ajouté ou arrêté récemment",
            "Ce qu'il faut vérifier avant toute nouvelle délivrance",
            "Ce qui est transmis au médecin traitant",
        ],
    ),
    (
        "Technique d'inhalation",
        &[
            "Démonstration par le patient, dispositif en main",
            "Armement, inspiration, apnée : les trois temps",
            "L'erreur propre à son dispositif",
            "Chambre d'inhalation : utilité, lavage, séchage",
            "Rinçage de la bouche après le corticoïde",
            "Consommation du traitement de secours dans le mois",
            "Ce qu'il fait en cas de crise, et quand il appelle",
        ],
    ),
    (
        "Vie quotidienne / diététique",
        &[
            "Repas, horaires, appétit, poids",
            "Alcool et tabac, sans jugement",
            "Activité physique possible et acceptée",
            "Sommeil et fatigue",
            "Voyages, chaleur, jeûne : ce qui change le traitement",
            "Conduite automobile et travail",
            "Un objectif réaliste jusqu'à la prochaine fois",
        ],
    ),
    (
        "Automédication",
        &[
            "Ce qui est pris sans ordonnance, et pour quoi",
            "Antalgiques : lequel, combien, depuis quand",
            "Plantes et compléments, y compris ceux offerts par un proche",
            "Ce qui est incompatible avec le traitement en cours",
            "Ce qui peut se prendre sans risque, et à quelle dose",
            "Quand consulter plutôt que se traiter",
        ],
    ),
    (
        "Sortie d'hôpital / conciliation",
        &[
            "L'ordonnance de sortie ligne par ligne, contre celle d'avant l'hospitalisation",
            "Ce qui a été arrêté, ce qui a été ajouté, ce dont la dose a changé",
            "Ce que le patient a rapporté de chez lui et qu'il risque de reprendre",
            "Les traitements suspendus pendant le séjour : lesquels reprennent, et quand",
            "Qui suit quoi, et la date du prochain rendez-vous",
            "Les examens à faire dans les jours qui suivent, biologie comprise",
            "Le matériel et les soins prévus à domicile : qui vient, à quelle heure",
            "Ce qui doit faire rappeler le médecin plutôt qu'attendre la consultation",
        ],
    ),
    (
        "Douleur chronique",
        &[
            "L'intensité aujourd'hui et sur la semaine, avec la même échelle à chaque fois",
            "Ce que la douleur empêche de faire : marcher, dormir, travailler, sortir",
            "Le traitement de fond : pris comme prescrit, ou à la demande",
            "Les interdoses : combien par jour, et à quel moment elles reviennent",
            "La constipation, la somnolence et les nausées sous opioïde",
            "Ce que le patient prend en plus, y compris ce qu'il achète seul",
            "Ce qui a été essayé sans médicament, et ce qui pourrait l'être",
        ],
    ),
    (
        "Sommeil",
        &[
            "L'heure du coucher, l'heure du lever, et le temps réellement passé au lit",
            "Combien de temps pour s'endormir, combien de réveils, et à quelle heure",
            "Ce qui est pris pour dormir, depuis quand, et à quelle dose",
            "Les écrans, la caféine après seize heures, l'alcool qui fragmente la nuit",
            "La sieste, sa durée et son heure",
            "Le ronflement, les pauses respiratoires signalées par l'entourage",
            "Un objectif de décroissance si un hypnotique dure depuis plus de quatre semaines",
        ],
    ),
    (
        "Chute et autonomie",
        &[
            "Une chute dans les douze derniers mois, et les circonstances",
            "L'ordonnance relue pour ce qui fait tomber : psychotropes, antihypertenseurs, diurétiques",
            "La tension mesurée couchée puis debout, et le vertige au lever",
            "La vue, l'audition, et la date du dernier contrôle de chacune",
            "Le chaussage, les tapis, l'éclairage de nuit, la barre dans la salle de bain",
            "L'activité physique de la semaine, et la peur de retomber",
            "La vitamine D, le calcium alimentaire, et le dernier poids connu",
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
