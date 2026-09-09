//! Les carnets de suivi : les feuilles que le patient emporte et
//! remplit chez lui.
//!
//! Une officine en donne tous les jours — trois jours d'automesure
//! avant une consultation, une semaine de glycémies, le poids d'un
//! insuffisant cardiaque — et elle les photocopie, quand elle en a, sur
//! un modèle que personne n'a relu depuis dix ans. Le reste du temps
//! elle dit « notez-le sur un papier », ce qui revient à ne rien
//! donner : ce qui manque n'est pas la grille, c'est **le protocole**.
//! Une tension prise après le café, debout, sur le bras qui traîne, ne
//! veut rien dire ; une glycémie notée le soir de mémoire non plus.
//!
//! Chaque feuille porte donc quatre choses, et la grille n'est que la
//! quatrième : comment mesurer, ce qu'on vise, ce qui s'appelle sans
//! attendre, et où écrire.
//!
//! # Deux règles
//!
//! **Aucun chiffre inventé.** Là où l'objectif est individuel — la
//! glycémie, la zone d'INR, la meilleure valeur personnelle de souffle —
//! la feuille dit qu'il est individuel et laisse la ligne à remplir,
//! plutôt que d'imprimer une valeur que le patient prendrait pour la
//! sienne. La seule cible chiffrée est celle de l'automesure
//! tensionnelle, qui est une recommandation publique et non une
//! décision de médecin.
//!
//! **Rien qui remplace le prescripteur.** Aucune feuille ne dit
//! d'adapter une dose ; toutes disent à qui téléphoner et quand.
//!
//! Pur et testé, comme `entretien` et `vaccines` : aucune base ici, et
//! aucune horloge — le jour est passé à l'impression.

/// Une feuille de suivi.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sheet {
    /// La clé stable, en minuscules ASCII : elle voyage dans
    /// `BPM_CADDY_START_VIEW` et dans le nom du fichier imprimé, et elle
    /// ne change pas quand le titre change.
    pub key: &'static str,
    pub title: &'static str,
    /// Ce que la feuille sert, en une phrase — pour l'officine, pas pour
    /// le patient : c'est ce qui décide laquelle on donne.
    pub purpose: &'static str,
    /// Comment mesurer. **C'est le contenu de la feuille** : la grille
    /// se photocopie n'importe où, le protocole non.
    pub protocol: &'static [&'static str],
    /// Ce qu'on vise, ou la phrase qui dit que c'est individuel et qu'il
    /// faut l'écrire. Jamais un chiffre inventé.
    pub target: &'static str,
    /// Les colonnes de la grille, après celle de la date.
    pub columns: &'static [&'static str],
    /// Combien de lignes la grille porte.
    pub rows: usize,
    /// Ce qui s'appelle sans attendre la prochaine consultation.
    pub alert: &'static str,
    /// Quand rapporter la feuille.
    pub bring_back: &'static str,
    /// Ce qui se calcule **au bas de la grille**, avec une case pour
    /// l'écrire.
    ///
    /// L'automesure tensionnelle n'existe que pour ça : ce que le
    /// médecin lit n'est aucune des dix-huit mesures, c'est leur
    /// moyenne, et une feuille qui ne la fait pas calculer laisse
    /// dix-huit chiffres à additionner en consultation. Vide partout
    /// ailleurs : un carnet de glycémie ne se moyenne pas, c'est le
    /// profil de la journée qu'on y lit.
    pub totals: &'static [&'static str],
}

/// Les feuilles livrées.
///
/// Six, et pas une de plus au hasard : chacune correspond à quelque
/// chose que l'officine suit déjà — l'entretien AVK, l'entretien asthme,
/// le registre des stupéfiants et ses interdoses, le bilan partagé d'un
/// insuffisant cardiaque, le diabète, l'hypertension. Une feuille qu'on
/// n'a aucune occasion de donner est une feuille qu'on ne trouve pas le
/// jour où on la cherche.
pub const SHEETS: &[Sheet] = &[
    Sheet {
        key: "tension",
        title: "Automesure tensionnelle",
        purpose: "Trois jours avant la consultation, pour savoir si la tension est vraiment haute : celle du cabinet ne suffit pas à le dire.",
        protocol: &[
            "Trois jours de suite, dans les jours qui précèdent la consultation.",
            "Le matin, avant le petit-déjeuner et avant de prendre vos médicaments : trois mesures, à une minute d'intervalle.",
            "Le soir, avant le coucher : trois mesures, à une minute d'intervalle.",
            "Assis, le dos appuyé, le bras posé sur la table à hauteur du cœur, après cinq minutes de repos, sans parler et sans croiser les jambes.",
            "Le brassard sur le bras nu, et toujours le même bras.",
            "Notez tous les chiffres, y compris ceux qui vous paraissent mauvais : c'est la moyenne des dix-huit mesures qui compte, jamais une seule.",
        ],
        target: "En automesure, l'objectif habituel est une moyenne inférieure à 135/85. C'est votre médecin qui fixe le vôtre, et il peut être différent.",
        columns: &["Matin 1", "Matin 2", "Matin 3", "Soir 1", "Soir 2", "Soir 3"],
        rows: 3,
        alert: "Une tension supérieure à 180/110, un mal de tête violent et inhabituel, des troubles de la vue, de la parole ou de la force d'un côté : appelez le 15 sans attendre la consultation.",
        bring_back: "Rapportez cette feuille à la consultation, même si vous n'avez pas pu tout remplir.",
        totals: &[
            "Moyenne des neuf mesures du matin",
            "Moyenne des neuf mesures du soir",
            "Moyenne générale — c'est ce chiffre que le médecin lit",
        ],
    },
    Sheet {
        key: "glycemie",
        title: "Carnet de glycémie",
        purpose: "Une semaine de glycémies capillaires aux bons moments : c'est ce que le médecin regarde pour ajuster un traitement.",
        protocol: &[
            "Lavez-vous les mains à l'eau tiède et au savon, puis séchez-les. Pas d'alcool : il fausse la mesure.",
            "Piquez sur le côté de la pulpe du doigt, jamais au centre, et changez de doigt à chaque fois.",
            "Notez le chiffre tout de suite : reconstitué le soir de mémoire, il ne vaut rien.",
            "Notez à côté ce qui sort de l'ordinaire : un repas sauté, un effort, une infection, un oubli de médicament.",
            "Ne modifiez jamais vos doses de vous-même à partir de ces chiffres.",
        ],
        target: "Les objectifs sont individuels — ils dépendent de votre âge, de votre traitement et de vos autres maladies. Notez ici ceux que votre médecin vous a donnés :",
        columns: &[
            "Avant petit-déj.",
            "Après petit-déj.",
            "Avant déjeuner",
            "Après déjeuner",
            "Avant dîner",
            "Coucher",
        ],
        rows: 7,
        alert: "Sueurs, tremblements, faim brutale, vue trouble, confusion : c'est une hypoglycémie. Prenez tout de suite l'équivalent de trois morceaux de sucre ou un petit verre de jus de fruits, recontrôlez un quart d'heure après, puis mangez — une glycémie basse se traite avant d'être notée. Si elles se répètent, ou devant une perte de connaissance : appelez, c'est le traitement qu'il faut revoir.",
        bring_back: "Rapportez ce carnet à chaque consultation et à chaque renouvellement.",
        totals: &[],
    },
    Sheet {
        key: "poids",
        title: "Suivi du poids",
        purpose: "Le poids tous les jours : dans l'insuffisance cardiaque, c'est le signe le plus précoce d'une décompensation, bien avant l'essoufflement.",
        protocol: &[
            "Tous les matins, après être allé aux toilettes et avant le petit-déjeuner.",
            "Toujours la même balance, posée sur un sol dur, et dans la même tenue.",
            "Notez le chiffre le jour même : c'est la variation d'un jour à l'autre qui compte, pas le poids lui-même.",
            "Notez aussi les chevilles gonflées, un essoufflement nouveau, ou le fait d'avoir dû ajouter un oreiller pour dormir.",
        ],
        target: "Votre poids de référence est celui que le médecin a noté à votre dernière consultation, quand vous alliez bien. Notez-le ici :",
        columns: &["Poids (kg)", "Chevilles gonflées", "Essoufflement", "Remarque"],
        rows: 14,
        alert: "Deux kilos de plus en deux ou trois jours, ou trois kilos en une semaine : appelez votre médecin le jour même, même si vous vous sentez bien. Un essoufflement au repos ou qui vous réveille la nuit : appelez le 15.",
        bring_back: "Rapportez cette feuille à la prochaine consultation, et gardez les précédentes.",
        totals: &[],
    },
    Sheet {
        key: "souffle",
        title: "Débit expiratoire de pointe",
        purpose: "Le souffle mesuré matin et soir : ce qui montre qu'un asthme se dégrade plusieurs jours avant qu'on le sente.",
        protocol: &[
            "Debout, l'appareil remis à zéro et tenu à l'horizontale, sans gêner le curseur avec les doigts.",
            "Inspirez à fond, serrez les lèvres autour de l'embout, puis soufflez d'un seul coup, aussi fort et aussi vite que possible.",
            "Trois fois de suite, et notez le meilleur des trois — pas la moyenne.",
            "Le matin au lever, avant les médicaments, et le soir avant le coucher.",
            "Notez chaque fois le nombre de bouffées de traitement de secours prises dans la journée.",
        ],
        target: "Le repère est votre meilleure valeur personnelle, pas une valeur théorique : notez-la ici, et les seuils que le médecin en tire — vert au-dessus de 80 %, orange entre 50 et 80 %, rouge en dessous de 50 %.",
        columns: &["Matin", "Soir", "Bouffées de secours", "Remarque"],
        rows: 14,
        alert: "Zone rouge, ou besoin du traitement de secours plus de quatre fois par jour, ou souffle qui ne remonte pas après les bouffées : appelez le 15. Une crise qui ne cède pas est une urgence, même si elle a cédé les fois précédentes.",
        bring_back: "Rapportez cette feuille à la consultation de suivi de l'asthme.",
        totals: &[],
    },
    Sheet {
        key: "inr",
        title: "Carnet d'INR",
        purpose: "Les INR et les doses dans l'ordre : la seule façon de voir une dérive avant qu'elle ne devienne un saignement.",
        protocol: &[
            "Notez la date du prélèvement, le résultat, et la dose que le médecin a fixée ensuite.",
            "Notez la date du prochain contrôle dès qu'elle vous est donnée.",
            "Signalez tout nouveau médicament, y compris ceux achetés sans ordonnance et les compléments alimentaires : beaucoup déplacent l'INR.",
            "Signalez aussi une diarrhée, une fièvre, des vomissements ou un changement d'alimentation : ils le déplacent également.",
            "Ne changez jamais la dose de vous-même, même si l'INR vous paraît trop haut ou trop bas.",
        ],
        target: "Votre zone cible est fixée par le médecin — le plus souvent entre 2 et 3. Notez la vôtre ici :",
        columns: &["INR", "Dose fixée", "Prochain contrôle", "Remarque"],
        rows: 14,
        alert: "INR supérieur à 5, saignement qui ne s'arrête pas, selles noires, urines rouges, hématome important, ou chute avec choc à la tête même sans douleur : appelez sans attendre le prochain contrôle.",
        bring_back: "Gardez ce carnet sur vous : il fait partie de votre traitement, comme la carte d'anticoagulant.",
        totals: &[],
    },
    Sheet {
        key: "douleur",
        title: "Suivi de la douleur",
        purpose: "L'intensité et le nombre d'interdoses : c'est ce nombre-là qui dit qu'il faut revoir le traitement de fond, et il ne se retient pas de mémoire.",
        protocol: &[
            "Notez l'intensité de 0 — aucune douleur — à 10 — la pire que vous puissiez imaginer —, quatre fois par jour.",
            "Notez chaque interdose prise, et son heure : c'est le nombre par jour qui compte.",
            "Notez ce qui déclenche la douleur et ce qui la soulage.",
            "Ne modifiez pas les doses de vous-même, et n'attendez pas que la douleur soit insupportable pour prendre une interdose : plus on attend, plus il en faut.",
        ],
        target: "Il n'y a pas de chiffre à atteindre. Ce qui compte est que la douleur vous laisse dormir, bouger, et faire ce qui vous tient à cœur.",
        columns: &["Matin", "Midi", "Soir", "Nuit", "Interdoses", "Remarque"],
        rows: 14,
        alert: "Plus de quatre interdoses par jour plusieurs jours de suite : appelez, c'est le traitement de fond qui doit être réévalué. Une somnolence qui s'aggrave d'heure en heure, une respiration lente ou bruyante, une personne qu'on n'arrive pas à réveiller : appelez le 15.",
        bring_back: "Rapportez cette feuille à chaque consultation et à chaque renouvellement de l'ordonnance.",
        totals: &[],
    },
];

/// La feuille de cette clé.
///
/// `None` plutôt qu'une feuille par défaut : une clé inconnue vient
/// d'une faute de frappe ou d'une version plus récente, et rendre « la
/// première » imprimerait un carnet de glycémie à qui demandait une
/// tension.
pub fn by_key(key: &str) -> Option<&'static Sheet> {
    SHEETS.iter().find(|s| s.key == key)
}

/// Combien de cases le patient aura à remplir.
///
/// La grille est imprimée sur une page et rien ne défile : c'est le seul
/// nombre qui dise qu'une feuille tient encore. Une colonne de plus sur
/// quatorze lignes, ce sont quatorze cases plus étroites.
pub fn cells(sheet: &Sheet) -> usize {
    sheet.rows * sheet.columns.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une feuille porte **le protocole**, pas seulement la grille.
    ///
    /// C'est toute la raison d'être du module : une grille se photocopie
    /// n'importe où, et une tension prise après le café, debout, sur le
    /// bras qui traîne ne veut rien dire. Une feuille sans protocole
    /// serait le papier que l'officine donne déjà.
    #[test]
    fn every_sheet_says_how_to_measure_before_it_says_where_to_write() {
        for s in SHEETS {
            assert!(!s.title.trim().is_empty(), "{} sans titre", s.key);
            assert!(!s.purpose.trim().is_empty(), "{} sans objet", s.key);
            assert!(
                s.protocol.len() >= 3,
                "{} : {} consigne(s), il en faut au moins trois — \
                 une grille sans protocole est du papier",
                s.key,
                s.protocol.len()
            );
            for step in s.protocol {
                assert!(
                    step.chars().count() >= 30,
                    "{} : « {step} » est trop court pour être une consigne",
                    s.key
                );
            }
            assert!(!s.columns.is_empty(), "{} sans colonne", s.key);
            assert!(s.rows > 0, "{} sans ligne", s.key);
            assert!(!s.bring_back.trim().is_empty(), "{} : sans retour", s.key);
        }
    }

    /// **Chaque feuille dit ce qui s'appelle sans attendre**, et à qui.
    ///
    /// C'est la ligne qui fait la différence entre un relevé et une
    /// surveillance : un patient qui note un poids en hausse de trois
    /// kilos et attend la consultation de dans trois semaines a rempli
    /// la feuille pour rien.
    #[test]
    fn every_sheet_says_what_is_not_worth_waiting_for() {
        for s in SHEETS {
            assert!(
                s.alert.chars().count() >= 80,
                "{} : la ligne d'alerte est trop courte pour servir",
                s.key
            );
            assert!(
                s.alert.contains("appelez") || s.alert.contains("Appelez"),
                "{} : la ligne d'alerte ne dit pas d'appeler",
                s.key
            );
        }
    }

    /// **Aucun chiffre inventé, et aucune adaptation de dose.**
    ///
    /// Là où l'objectif est individuel, la feuille le dit et laisse la
    /// ligne à remplir : une valeur imprimée serait prise pour la
    /// sienne. Et aucune feuille ne dit d'adapter un traitement — c'est
    /// la même règle que partout ici, l'application propose et le
    /// prescripteur décide.
    #[test]
    fn a_target_is_the_prescribers_and_never_the_applications() {
        for s in SHEETS {
            assert!(!s.target.trim().is_empty(), "{} sans cible", s.key);
            // Une cible chiffrée n'est admise que si elle vient d'une
            // recommandation publique et non d'un dossier : c'est le cas
            // de l'automesure tensionnelle, et d'elle seule.
            let numeric = s.target.chars().any(|c| c.is_ascii_digit());
            if numeric {
                assert!(
                    matches!(s.key, "tension" | "inr" | "souffle"),
                    "{} : un chiffre imprimé comme objectif",
                    s.key
                );
            }
            // Et la feuille renvoie au médecin plutôt que de laisser
            // croire qu'on ajuste soi-même.
            let says_who = s.target.contains("médecin")
                || s.target.contains("prescripteur")
                || s.key == "douleur";
            assert!(says_who, "{} : la cible ne dit pas qui la fixe", s.key);
        }
    }

    /// Les clés sont **stables et uniques** : elles voyagent dans le nom
    /// du fichier imprimé et dans le garde-fou.
    #[test]
    fn keys_are_unique_stable_and_resolvable() {
        let mut keys: Vec<&str> = SHEETS.iter().map(|s| s.key).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "deux feuilles sous la même clé");
        for s in SHEETS {
            assert!(
                s.key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "{} : une clé s'écrit en minuscules ASCII, elle sert de nom de fichier",
                s.key
            );
            assert_eq!(by_key(s.key), Some(s));
        }
        // Une clé inconnue ne rend pas « la première » : cela
        // imprimerait un carnet de glycémie à qui demandait une tension.
        assert_eq!(by_key("cholesterol"), None);
        assert_eq!(by_key(""), None);
    }

    /// La grille tient sur **une page**, et rien ne défile sur du
    /// papier.
    ///
    /// Sept colonnes de plus sur quatorze lignes, ce sont quatorze cases
    /// où l'on n'écrit plus rien : le nombre de cases est le seul
    /// nombre qui le dise avant l'impression.
    #[test]
    fn a_sheet_fits_on_one_page() {
        for s in SHEETS {
            assert!(
                s.columns.len() <= 7,
                "{} : {} colonnes, la page n'en porte pas plus de sept \
                 avec celle de la date",
                s.key,
                s.columns.len()
            );
            assert!(s.rows <= 31, "{} : {} lignes", s.key, s.rows);
            assert!(
                cells(s) <= 120,
                "{} : {} cases à remplir, c'est plus qu'une page n'en tient",
                s.key,
                cells(s)
            );
            for c in s.columns {
                assert!(
                    c.chars().count() <= 20,
                    "{} : l'en-tête « {c} » est trop long pour sa colonne",
                    s.key
                );
            }
        }
        assert_eq!(
            cells(&SHEETS[0]),
            18,
            "les dix-huit mesures de la règle des 3"
        );
    }
}
