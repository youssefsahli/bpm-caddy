//! Ce que l'âge fait à une ordonnance.
//!
//! `renal.rs` répond « ce DFG-là change cette dose », `hepatic.rs` « ce
//! stade-là change cette conduite », `gravidity.rs` « cette grossesse-là
//! interdit cette molécule ». Il manquait le troisième axe, et c'est le
//! seul des quatre dont **le chiffre est déjà au dossier** : la date de
//! naissance y est depuis la création de la fiche, personne n'a rien à
//! taper, et c'est précisément pour cela que personne ne la regarde. Le
//! rein se demande parce qu'il faut aller chercher un compte rendu ; le
//! foie se demande parce qu'il faut cliquer un stade ; l'âge ne se
//! demande pas, et ne se regarde donc jamais.
//!
//! Huit cent quatre-vingt-neuf fois, les fiches livrées écrivent « chez
//! le sujet âgé » — dans leurs effets indésirables, leur posologie, leur
//! surveillance. Le Valium dit « l'accumulation entraîne confusion,
//! chutes et fractures, ce qui fait préférer une benzodiazépine à
//! demi-vie courte à cet âge » ; le Ditropan, « l'oxybutynine étant l'un
//! des antimuscariniques les plus délétères sur le plan central » ; le
//! Daonil, « hypoglycémies prolongées, principal risque du traitement
//! chez le sujet âgé ». C'est écrit, molécule par molécule, et il fallait
//! ouvrir les fiches une à une pour le rapprocher d'une date de
//! naissance. Ce module le fait d'un coup, ligne par ligne.
//!
//! **Et la différence avec son voisin rénal tient en une phrase : le
//! rein change la dose, l'âge change le choix.** Une conduite rénale
//! dit « réduire de moitié » ; une conduite d'âge dit « il existe
//! mieux ». C'est pourquoi la ligne porte un champ que `renal` n'a pas
//! et ne peut pas avoir — [`Inappropriate::instead`], ce qu'on met à la
//! place — et pourquoi il est **obligatoire** : c'est la règle que
//! `crush.rs` a écrite avant celui-ci, « un "non" sans alternative
//! laisse le problème entier ».
//!
//! Sept règles, un test chacune :
//!
//! * **Sans date de naissance, pas de verdict.** La ligne existe — elle
//!   dit que ce traitement est de ceux que l'âge décide — et elle ne
//!   conclut pas. Le type l'empêche : `level` vaut `None`, comme le DFG
//!   manquant de `renal` et l'écart de caisse sans recette attendue.
//! * **Un « non » nomme ce qu'on met à la place**, et **nomme aussi ce
//!   qu'il évite** : « à éviter » tout seul n'apprend rien, et ne se
//!   discute pas avec un prescripteur. Chaque ligne porte donc son
//!   risque et son alternative, tous deux obligatoires.
//! * **On n'envoie jamais d'un médicament à éviter vers un autre.**
//!   L'alternative d'une ligne ne doit pas être réclamée, ailleurs dans
//!   la table, par une ligne « à éviter ». C'est la faute qu'aucune
//!   relecture ne voit — chaque ligne est juste, et la paire envoie en
//!   rond.
//! * **L'alternative est un choix, jamais une posologie.** Aucun
//!   milligramme dans ce qu'on met à la place : la dose dépend de
//!   l'indication, du poids et du rein, et c'est l'affaire du
//!   prescripteur. Le risque, lui, peut citer le seuil de la
//!   référence — c'est l'affinage que `hepatic` a fait avant celui-ci.
//! * **Une forme locale ne porte pas le niveau systémique.** La table
//!   est indexée sur la molécule, comme les six autres, si bien qu'un
//!   collyre à l'indométacine tombe dans la ligne des AINS. Le veto par
//!   ligne — le `never` de `renal` et de `gravidity` — et
//!   `classes::is_local_form` l'écartent.
//! * **Un seuil d'âge vient d'une liste publiée, jamais d'une
//!   intuition.** 75 ans est celui de la liste française, 65 celui de
//!   STOPP et de Beers ; il n'y en a pas d'autre, et un test le tient.
//! * **Une ligne qui se tait ne fait pas taire la suivante.** Sous son
//!   seuil, la ligne passe son tour au lieu de clore la lecture de
//!   cette boîte — sans quoi, le jour où une ligne citera 65 ans, une
//!   boîte que la ligne large attrape d'abord perdrait sa ligne précise
//!   à soixante-dix ans, sans que rien le dise.
//!
//! Et une confrontation, qui n'est pas une règle du module mais la
//! preuve qu'il ne dérive pas : l'application **livre déjà** une table
//! de référence « Sujet âgé », et
//! `the_table_of_reference_and_this_module_agree` les met face à face.
//! Elle a mordu le jour où elle a été écrite — la table nommait le
//! bromazépam parmi les benzodiazépines à demi-vie longue et le module
//! le laissait passer, à vingt heures pile.
//!
//! **Deux niveaux, et pas trois.** La liste française a un second axe —
//! les médicaments « à efficacité discutable » — qui n'est pas ici, et
//! c'est une frontière plutôt qu'un oubli : une efficacité modeste n'est
//! pas une question d'âge, c'est une question de revue d'ordonnance, et
//! ce module a un voisin pour cela. Le naftidrofuryl a été écrit puis
//! retiré pour cette raison exacte : sa fiche dit « niveau de preuve
//! modeste » et ne dit rien de l'âge, si bien que la ligne parlait
//! d'autre chose que de ce panneau. Un niveau sans membre est un niveau
//! qui attire la mauvaise ligne suivante.
//!
//! Ce que ce module ne fait **pas** : il ne remplace pas la revue
//! d'ordonnance, qui lit l'ensemble — doublons, associations, cascades —
//! là où celui-ci lit chaque ligne contre un seul nombre. Il ne sait ni
//! la dose, ni la durée, ni l'indication : une ligne « à éviter » sur une
//! ordonnance de trois jours n'est pas la même chose que sur une
//! ordonnance de trois ans, et le module ne peut pas les distinguer. Et
//! il ne dit jamais d'arrêter : arrêter brutalement un psychotrope chez
//! un sujet âgé est plus dangereux que de le poursuivre, et ce que le
//! panneau écrit en pied est « le remplacement se prépare », pas
//! « arrêtez ».
//!
//! Statique, pur, testé, sans horloge : l'âge est passé.

/// Ce que l'âge fait à une ligne. L'ordre est celui de la gravité, et
/// c'est lui qui trie la lecture.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    /// Le rapport bénéfice/risque est défavorable à cet âge et une autre
    /// option existe.
    Avoid,
    /// Utilisable, mais à une dose, une durée ou sous une condition que
    /// l'âge change.
    Caution,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Avoid => "À éviter à cet âge",
            Level::Caution => "Sous conditions",
        }
    }
}

/// Une molécule ou une classe, et ce que l'âge lui fait.
pub struct Inappropriate {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes de
    /// l'officine — repliés par `fuzzy::sort_key`, comme partout ici.
    pub needs: &'static [&'static str],
    /// Ce que la ligne **ne réclame pas**, bien que ses mots
    /// l'attrapent : le veto de `renal` et de `gravidity`, pour la même
    /// raison — une table indexée sur la molécule attrape les formes
    /// locales de cette molécule.
    pub never: &'static [&'static str],
    /// Ce que la ligne annonce : « Benzodiazépines à demi-vie longue ».
    pub label: &'static str,
    /// L'âge à partir duquel la ligne parle. **75 ou 65, et rien
    /// d'autre** : 75 est le seuil de la liste française, 65 celui de
    /// STOPP et de Beers. Un seuil intermédiaire serait une invention,
    /// et un test le refuse.
    pub from: u8,
    pub level: Level,
    /// Ce que l'âge fait courir. Obligatoire : « à éviter » tout seul
    /// n'apprend rien et ne se discute pas avec un prescripteur.
    pub risk: &'static str,
    /// Ce qu'on met à la place. Obligatoire, et **sans milligrammes** :
    /// l'alternative est un choix, la dose reste au prescripteur.
    pub instead: &'static str,
    /// D'où vient la ligne.
    pub source: &'static str,
}

/// Ce que le module rend pour une ligne d'ordonnance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    /// Le nom tel qu'il est écrit au dossier.
    pub treatment: String,
    pub label: &'static str,
    /// `None` quand aucune date de naissance n'est au dossier : la ligne
    /// dit alors que ce traitement est de ceux que l'âge décide, et que
    /// la date manque. **Ce n'est pas un verdict**, et le type est ce
    /// qui l'empêche d'en devenir un.
    pub level: Option<Level>,
    /// Le seuil de la ligne, pour que la lecture porte son « dès 75
    /// ans » plutôt qu'une affirmation sans âge.
    pub from: u8,
    pub risk: &'static str,
    pub instead: &'static str,
    pub source: &'static str,
}

/// Cette ligne réclame-t-elle cette boîte ?
///
/// Écrite **une fois**, et appelée par [`read`] comme par les tests qui
/// confrontent la table aux fiches livrées : `renal.rs` a payé pour
/// apprendre que trois copies d'une même question finissent par ne plus
/// poser la même.
fn claims(a: &Inappropriate, hay: &str) -> bool {
    a.needs
        .iter()
        .any(|n| crate::fuzzy::contains_folded(hay, n))
        && !a
            .never
            .iter()
            .any(|n| crate::fuzzy::contains_folded(hay, n))
}

/// Ce que l'âge fait à cette ordonnance.
///
/// `age` est en années révolues — `db::age_on` le calcule à partir de la
/// date de naissance du dossier. `None` — aucune date au dossier —
/// n'est **pas** zéro : le module rend alors les traitements concernés
/// sans verdict, et c'est à la vue de dire que la date manque.
///
/// L'ordre est celui de la gravité, puis celui du dossier : deux lignes
/// de même niveau ne doivent pas échanger leur place d'une image à
/// l'autre.
pub fn read(treatments: &[crate::revue::Treatment], age: Option<u32>) -> Vec<Finding> {
    read_in(TABLE, treatments, age)
}

/// La même lecture, sur une table donnée.
///
/// Écrite à part pour être **éprouvable** : la règle « une ligne qui se
/// tait ne fait pas taire la suivante » ne se voit que sur deux lignes
/// de seuils différents, et il n'y en a pas encore dans la table
/// livrée. Un comportement qu'aucun test ne peut atteindre est un
/// comportement qui dérivera sans bruit — c'est la raison pour laquelle
/// `chart::hbar_fit` est écrit à part lui aussi.
fn read_in(
    table: &'static [Inappropriate],
    treatments: &[crate::revue::Treatment],
    age: Option<u32>,
) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for t in treatments {
        // **Une forme locale ne relève pas d'une table de molécules.**
        // Écarté une fois, en haut de la boucle, plutôt que ligne par
        // ligne : aucune des règles de cette table ne porte sur une
        // forme locale, et le jour où l'une le fera — un collyre
        // bêtabloquant ralentit bien le cœur — c'est cette ligne-là
        // qu'on rouvrira, avec un mot dans la règle plutôt qu'un oubli.
        if crate::classes::is_local_form(t.class) {
            continue;
        }
        let hay = crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags));
        for a in table {
            if !claims(a, &hay) {
                continue;
            }
            // Au-dessous du seuil, la ligne n'a rien à dire — et le
            // silence est la bonne réponse : la moitié de la base est
            // inappropriée à quelqu'un, et une liste qui parle de tout
            // le monde ne parle de personne.
            //
            // **`continue` et non `break`** : une ligne qui se tait ne
            // fait pas taire la suivante. Aujourd'hui toutes les lignes
            // parlent à 75 ans et cela ne change rien ; le jour où l'une
            // d'elles citera STOPP à 65, une boîte que la ligne large
            // attrape d'abord perdrait sa ligne précise, à soixante-dix
            // ans, sans que rien le dise. C'est le choix de
            // `renal::read` devant un palier non franchi, et pour la
            // même raison.
            if age.is_some_and(|v| u32::from(a.from) > v) {
                continue;
            }
            out.push(Finding {
                treatment: t.name.trim().to_owned(),
                label: a.label,
                level: age.map(|_| a.level),
                from: a.from,
                risk: a.risk,
                instead: a.instead,
                source: a.source,
            });
            break;
        }
    }
    out.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then(a.label.cmp(b.label))
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

/// Combien de lignes dépendent de l'âge sans qu'une date permette de
/// conclure. Sur ce qui est **résolu**, comme `renal::undecided` : c'est
/// ce que la vue tient.
pub fn undecided(findings: &[Resolved]) -> usize {
    findings.iter().filter(|f| !f.decided()).count()
}

/// Le document sous lequel les phrases du panneau sont adressées.
pub const DOC: &str = "age";

/// Chaque phrase avec son adresse, **calculée une seule fois** et
/// parcourue par les deux côtés.
///
/// Le repère est le libellé de la ligne et le rôle de la phrase — le
/// risque ou l'alternative. Ni le rang, qui se décale dès qu'on insère
/// une ligne, ni la prose, qui disparaît dès qu'on la corrige.
fn addressed() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for a in TABLE {
        let id = crate::content::slug(a.label);
        out.push((crate::content::key(DOC, &id, "risque"), "risque", a.risk));
        out.push((
            crate::content::key(DOC, &id, "alternative"),
            "alternative",
            a.instead,
        ));
    }
    out
}

/// Toutes les phrases du panneau, avec leur adresse.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    addressed()
}

/// Appliquer les réécritures de l'officine à ce que l'âge impose.
pub fn resolve(findings: Vec<Finding>, over: &crate::content::Overrides) -> Vec<Resolved> {
    findings
        .into_iter()
        .map(|f| {
            let id = crate::content::slug(f.label);
            Resolved {
                risk: over
                    .get(&crate::content::key(DOC, &id, "risque"), f.risk)
                    .to_owned(),
                instead: over
                    .get(&crate::content::key(DOC, &id, "alternative"), f.instead)
                    .to_owned(),
                treatment: f.treatment,
                label: f.label,
                level: f.level,
                from: f.from,
                source: f.source,
            }
        })
        .collect()
}

/// Ce que l'âge impose, avec les mots de l'officine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resolved {
    pub treatment: String,
    pub label: &'static str,
    pub level: Option<Level>,
    pub from: u8,
    pub risk: String,
    pub instead: String,
    pub source: &'static str,
}

impl Resolved {
    /// La ligne porte un verdict. Sans date de naissance, non.
    pub fn decided(&self) -> bool {
        self.level.is_some()
    }
}

/// Ce que l'âge change, molécule par molécule.
///
/// **Rien ici n'est inventé** : chaque ligne vient d'une liste publiée —
/// la liste française de Laroche, les critères STOPP/START, les critères
/// de Beers, une recommandation de la HAS ou une restriction de
/// l'ANSM — et chacune a été confrontée à la fiche livrée qu'elle
/// attrape. Les fiches disent déjà tout cela, dans leur prose ; ce
/// tableau ne fait que le mettre en face d'une date de naissance.
///
/// **L'ordre compte** : la première ligne qui accroche une boîte est
/// celle que [`read`] retient, et un test le tient. Les lignes précises
/// passent donc avant les lignes larges — la benzodiazépine à demi-vie
/// longue avant toutes les benzodiazépines, l'antihistaminique sédatif
/// avant l'hypnotique.
pub const TABLE: &[Inappropriate] = &[
    Inappropriate {
        needs: &[
            "diazepam",
            "valium",
            "prazepam",
            "lysanxia",
            "clorazepate",
            "tranxene",
            "nordazepam",
            "nordaz",
            "clobazam",
            "urbanyl",
            "loflazepate",
            "victan",
            // **Le bromazépam, à vingt heures pile.** La liste française
            // le nomme, la table de référence « Sujet âgé » le nomme, et
            // sa fiche écrit vingt heures : le seuil se lit donc « au
            // moins vingt », et non « plus de vingt ». Écrit dans
            // l'autre sens, le module et la table de l'application
            // disaient deux choses différentes du même produit — ce que
            // seule leur confrontation montre.
            "bromazepam",
            "lexomil",
        ],
        // Le clonazépam et le midazolam sont dans cette famille et n'y
        // relèvent pas : leur indication est l'épilepsie et la crise
        // convulsive, et le choix ne se discute pas au comptoir.
        never: &["antiepileptique", "crise convulsive"],
        label: "Benzodiazépines à demi-vie longue",
        from: 75,
        level: Level::Avoid,
        risk: "Accumulation des métabolites actifs : somnolence diurne, confusion, troubles de la mémoire, et surtout chute et fracture du col. Le risque est le plus élevé dans les deux semaines qui suivent l'instauration, et après chaque augmentation de dose.",
        instead: "Une benzodiazépine à demi-vie courte — oxazépam, lorazépam — à la moitié de la dose adulte et pour la durée la plus courte ; hors indication antiépileptique, où le choix ne se discute pas ici. Le remplacement se prépare avec le prescripteur et se fait par décroissance progressive : un arrêt brutal expose au rebond, à la confusion et à la convulsion.",
        source: "Laroche 2007, liste française des médicaments potentiellement inappropriés ; HAS — arrêt des benzodiazépines chez le sujet âgé",
    },
    Inappropriate {
        needs: &["tricyclique"],
        never: &[],
        label: "Antidépresseurs tricycliques",
        from: 75,
        level: Level::Avoid,
        risk: "Effets atropiniques constants — confusion, rétention urinaire, constipation, glaucome aigu par fermeture de l'angle — auxquels s'ajoutent l'hypotension orthostatique et les troubles de la conduction cardiaque.",
        instead: "Un inhibiteur de la recapture de la sérotonine, instauré bas et augmenté lentement, avec un contrôle de la natrémie dans le mois qui suit : l'hyponatrémie est l'effet que cet âge rend fréquent. Une indication de douleur neuropathique ne se règle pas au comptoir : elle se discute avec le prescripteur, à la plus faible dose qui soulage.",
        source: "Laroche 2007 ; HAS — prise en charge de la dépression de la personne âgée",
    },
    Inappropriate {
        // « isrs » n'attrape que les six ISRS de la base : vérifié, comme
        // tout mot de trois lettres doit l'être ici.
        needs: &["isrs"],
        never: &[],
        label: "Inhibiteurs de la recapture de la sérotonine",
        from: 75,
        level: Level::Caution,
        risk: "Hyponatrémie par sécrétion inappropriée d'hormone antidiurétique, d'autant plus fréquente que l'âge avance et que s'y ajoute un diurétique — elle se manifeste par une confusion, des chutes ou des nausées, et non par un signe qui la nomme. S'y ajoutent le risque hémorragique digestif en association à un AINS ou à un anticoagulant, l'allongement du QT et les chutes.",
        instead: "La classe reste le premier choix à cet âge — c'est elle qu'on met à la place des tricycliques — et ce qui change n'est pas le choix mais la façon de l'instaurer : demi-dose, natrémie contrôlée dans le mois qui suit et à chaque changement de dose, et un protecteur gastrique si un anti-inflammatoire ou un anticoagulant est associé. La paroxétine est la plus atropinique de la famille et celle dont l'arrêt est le plus difficile ; la sertraline et le citalopram se manient mieux ici.",
        source: "Critères STOPP/START v2 ; HAS — prise en charge de la dépression de la personne âgée",
    },
    Inappropriate {
        needs: &["antihistaminique h1 sedatif", "mequitazine", "doxylamine"],
        never: &[],
        label: "Antihistaminiques H1 de première génération",
        from: 75,
        level: Level::Avoid,
        risk: "Sédation prolongée jusqu'au lendemain et effets atropiniques : confusion, bouche sèche, constipation, rétention urinaire, aggravation d'un glaucome. La somnolence et les vertiges se paient en chutes.",
        instead: "Un antihistaminique de seconde génération — cétirizine, loratadine, desloratadine — qui ne sédate pas. Et pour l'insomnie, aucun antihistaminique : ni le bénéfice ni la durée ne le justifient à cet âge.",
        source: "Laroche 2007 ; critères de Beers 2023 — charge anticholinergique",
    },
    Inappropriate {
        needs: &["benzodiazepine", "hypnotique", "zolpidem", "zopiclone"],
        never: &["antiepileptique", "crise convulsive"],
        label: "Benzodiazépines et apparentés",
        from: 75,
        level: Level::Caution,
        risk: "Chute, fracture et confusion, qui ne tiennent pas qu'à la demi-vie : la vigilance du lendemain et la conduite automobile sont touchées par toutes, y compris les hypnotiques apparentés.",
        instead: "La moitié de la dose adulte, la durée la plus courte, et une réévaluation à chaque renouvellement. Les mesures d'hygiène du sommeil passent avant tout hypnotique ; au-delà de quatre semaines le bénéfice n'est plus démontré, tandis que la dépendance, elle, est installée.",
        source: "HAS — améliorer la prescription des psychotropes chez le sujet âgé ; ANSM — état des lieux de la consommation des benzodiazépines",
    },
    Inappropriate {
        needs: &["anticholinergique vesical", "oxybutynine"],
        never: &[],
        label: "Anticholinergiques vésicaux",
        from: 75,
        level: Level::Avoid,
        risk: "Confusion, troubles de la mémoire et aggravation d'un déclin cognitif, en plus de la bouche sèche, de la constipation et de la rétention. L'oxybutynine est la plus délétère des quatre sur le plan central.",
        instead: "Calendrier mictionnel, rééducation périnéale, et revue de ce qui aggrave l'incontinence — un diurétique pris le soir, un sédatif — avant tout antimuscarinique. Si l'un reste nécessaire, le moins atropinique, réévalué après quelques semaines et arrêté s'il ne change rien.",
        source: "Laroche 2007 ; critères de Beers 2023",
    },
    Inappropriate {
        needs: &[
            "trihexyphenidyle",
            "artane",
            "tropatepine",
            "anticholinergique antiparkinsonien",
        ],
        never: &[],
        label: "Anticholinergiques antiparkinsoniens",
        from: 75,
        level: Level::Avoid,
        risk: "Troubles de la mémoire, confusion, désorientation et hallucinations, particulièrement à cet âge, en plus des effets atropiniques périphériques et du risque de coup de chaleur par diminution de la sudation.",
        instead: "Devant un syndrome parkinsonien induit par un neuroleptique, c'est le neuroleptique qu'on rediscute et non un correcteur qu'on ajoute. Devant un tremblement, la conduite se décide avec le prescripteur sans passer par un anticholinergique.",
        source: "Laroche 2007 ; RCP trihexyphénidyle",
    },
    Inappropriate {
        needs: &["ains"],
        // Le collyre et la forme locale sont déjà écartés par la classe ;
        // la pastille pour la gorge ne l'est pas, parce que « pastille »
        // n'est pas un mot de voie pour `classes::is_local_form` — et
        // n'a pas à le devenir pour une seule boîte.
        never: &["pastille"],
        label: "Anti-inflammatoires non stéroïdiens",
        from: 75,
        level: Level::Avoid,
        risk: "Hémorragie digestive, dont le risque est le plus élevé à cet âge et qui survient souvent sans douleur annonciatrice ; insuffisance rénale aiguë sur un rein déjà diminué ; poussée hypertensive et décompensation d'une insuffisance cardiaque. Avec un IEC ou un sartan et un diurétique, c'est le trio qui décompense le rein.",
        instead: "Le paracétamol en première intention, à dose pleine et régulière. Si un AINS reste indispensable, la dose la plus faible pour la durée la plus courte, un inhibiteur de la pompe à protons associé, et la créatininémie contrôlée.",
        source: "Laroche 2007 ; critères STOPP/START v2 ; ANSM — bon usage des AINS",
    },
    Inappropriate {
        needs: &["glibenclamide", "daonil"],
        never: &[],
        label: "Glibenclamide",
        from: 75,
        level: Level::Avoid,
        risk: "Hypoglycémies prolongées et récidivantes, volontiers nocturnes, du fait de la durée d'action et des métabolites actifs : c'est le principal risque de ce traitement à cet âge, et celui qui conduit aux urgences.",
        instead: "Un sulfamide de durée d'action plus courte — gliclazide, glimépiride — introduit bas, ou la metformine si la fonction rénale le permet. Et une cible d'HbA1c discutée pour ce patient-là : l'objectif se desserre avec l'âge et la fragilité.",
        source: "Laroche 2007 ; HAS — diabète de type 2 chez la personne âgée",
    },
    Inappropriate {
        needs: &["antihypertenseur central"],
        never: &[],
        label: "Antihypertenseurs à action centrale",
        from: 75,
        level: Level::Avoid,
        risk: "Hypotension orthostatique, bradycardie, sédation et syndrome dépressif ; et une hypertension de rebond sévère en cas d'arrêt brutal, ce qui rend le remplacement lui-même délicat.",
        instead: "Les classes de première intention à cet âge — thiazidique à faible dose, IEC ou sartan, inhibiteur calcique. La pression se mesure aussi debout : c'est l'hypotension orthostatique qui fait tomber, et elle ne se voit pas assis.",
        source: "Laroche 2007 ; ESC/ESH — hypertension artérielle du sujet âgé",
    },
    Inappropriate {
        // **Les molécules, et non la classe.** « myorelaxant » attrape
        // cinq fiches, dont le Liorésal, le Dantrium et le Botox — la
        // spasticité d'une sclérose en plaques ou d'un blessé
        // médullaire, qui n'a rien à voir avec la contracture d'une
        // lombalgie. La conduite écrite ici — paracétamol, chaleur,
        // reprise du mouvement — y serait absurde, et l'arrêt brutal
        // d'un baclofène donne convulsions et hyperthermie. Deux
        // molécules nommées valent mieux qu'une classe qui ratisse.
        needs: &["thiocolchicoside", "methocarbamol"],
        never: &[],
        label: "Myorelaxants d'appoint",
        from: 75,
        level: Level::Avoid,
        risk: "Sédation, sensation d'ébriété, troubles de la coordination et chute — effets plus marqués à cet âge — pour un bénéfice antalgique faible et bref. Le thiocolchicoside porte en outre une restriction d'emploi pour un risque génotoxique.",
        instead: "Le paracétamol, la chaleur locale et surtout la reprise précoce du mouvement, qui est ce qui traite une lombalgie commune ; l'immobilité, elle, l'aggrave. Cette ligne ne vise pas les myorelaxants de la spasticité, qui relèvent d'une tout autre question.",
        source: "Critères de Beers 2023 ; HAS — prise en charge de la lombalgie commune",
    },
    Inappropriate {
        needs: &["nitrofurantoine", "furadantine"],
        never: &[],
        label: "Nitrofurantoïne",
        from: 75,
        level: Level::Avoid,
        risk: "En traitement prolongé ou en cures répétées : pneumopathie d'hypersensibilité, fibrose pulmonaire et hépatite ; neuropathie périphérique, que l'insuffisance rénale, le diabète et l'âge favorisent. L'efficacité, elle, baisse avec la fonction rénale.",
        instead: "Réservée à la cystite aiguë documentée et pour quelques jours, jamais en prophylaxie prolongée. Le choix suit l'antibiogramme et la fonction rénale.",
        source: "ANSM — nitrofurantoïne, restriction d'indication ; critères STOPP/START v2",
    },
    Inappropriate {
        needs: &["fluoroquinolone"],
        never: &[],
        label: "Fluoroquinolones",
        from: 75,
        level: Level::Avoid,
        risk: "Tendinopathie et rupture du tendon d'Achille, dont le risque est le plus élevé après 60 ans et davantage encore sous corticoïde ; confusion, hallucinations et convulsions ; neuropathie périphérique parfois durable ; allongement du QT ; anévrisme et dissection de l'aorte. C'est ce faisceau qui a fait restreindre la famille, et non un effet isolé.",
        instead: "Réservées aux infections où aucune autre famille ne convient, et jamais pour une infection bénigne : une cystite simple se traite par la fosfomycine ou le pivmécillinam, une infection respiratoire courante par l'amoxicilline. Toute douleur tendineuse fait arrêter et cesser l'appui, y compris après la fin du traitement.",
        source: "ANSM et EMA 2019 — restriction d'emploi des fluoroquinolones ; critères STOPP/START v2",
    },
    Inappropriate {
        needs: &["colchimax"],
        never: &[],
        label: "Colchicine associée à un atropinique et à l'opium",
        from: 75,
        level: Level::Avoid,
        risk: "Trois problèmes dans une boîte : la marge étroite de la colchicine, dont la toxicité digestive puis médullaire s'installe dès que le rein baisse ; un antispasmodique atropinique ; et de l'opium. L'association est faite pour limiter la diarrhée, qui est précisément le premier signe de surdosage — elle masque donc le signal d'alerte.",
        instead: "La colchicine seule, à dose faible et adaptée à la fonction rénale, la diarrhée étant lue comme un signal d'arrêt et non comme un effet à corriger. Une corticothérapie courte est l'autre option de l'accès de goutte à cet âge.",
        source: "ANSM — colchicine, rappel de bon usage ; Laroche 2007",
    },
    Inappropriate {
        needs: &["theophylline"],
        never: &[],
        label: "Théophylline",
        from: 75,
        level: Level::Avoid,
        risk: "Marge thérapeutique étroite : nausées, tachycardie, insomnie, puis troubles du rythme et convulsion au-delà. La clairance baisse avec l'âge, et une interaction suffit à passer le seuil sans que la dose ait changé.",
        instead: "Les bronchodilatateurs inhalés, dont l'effet est local et la marge large. La technique d'inhalation se vérifie à chaque renouvellement, et une chambre d'inhalation règle la plupart des échecs à cet âge.",
        source: "Laroche 2007 ; GOLD — bronchopneumopathie chronique obstructive",
    },
    Inappropriate {
        needs: &["trimetazidine", "vastarel"],
        never: &[],
        label: "Trimétazidine",
        from: 75,
        level: Level::Avoid,
        risk: "Syndrome extrapyramidal : tremblement, akinésie, instabilité à la marche, chutes — réversibles à l'arrêt, mais attribués à tort au vieillissement ou à une maladie de Parkinson débutante. C'est ce qui a fait restreindre l'indication.",
        instead: "L'indication se rediscute : il ne reste qu'un traitement additionnel de l'angor d'effort insuffisamment contrôlé. Devant un tremblement ou une instabilité nouvelle chez quelqu'un qui en prend, c'est la première ligne à suspendre avec le prescripteur.",
        source: "ANSM 2012 — réévaluation de la trimétazidine",
    },
    Inappropriate {
        needs: &["antipsychotique"],
        never: &[],
        label: "Antipsychotiques",
        from: 75,
        level: Level::Caution,
        risk: "Dans les troubles du comportement liés à une démence : surmortalité et accidents vasculaires cérébraux démontrés. À tout âge avancé : syndrome parkinsonien, chute, hypotension orthostatique, sédation, et allongement du QT pour plusieurs d'entre eux.",
        instead: "Devant une agitation nouvelle liée à une démence : les mesures non médicamenteuses d'abord, et la recherche de ce qui a changé — une douleur, une infection urinaire, une rétention, un déménagement, qui en sont les causes ordinaires ; si un antipsychotique reste nécessaire, la dose la plus faible, la durée la plus courte et une date de réévaluation écrite. Devant une maladie psychiatrique ancienne, le traitement ne se rediscute pas au comptoir : la précaution porte alors sur la chute, l'hypotension orthostatique et le QT.",
        source: "HAS — maladie d'Alzheimer, troubles du comportement perturbateurs ; ANSM",
    },
    Inappropriate {
        needs: &["nefopam", "acupan"],
        never: &[],
        label: "Néfopam",
        from: 75,
        level: Level::Caution,
        risk: "Profil atropinique complet — bouche sèche, tachycardie, rétention urinaire, glaucome — auquel s'ajoutent, à cet âge, l'excitation, les hallucinations et la confusion que sa propre fiche nomme, et un seuil épileptogène abaissé. Les sueurs et les vertiges d'une injection trop rapide font tomber.",
        instead: "Le paracétamol à dose pleine et régulière d'abord, puis, si cela ne suffit pas, un opioïde faible à dose réduite avec un laxatif prescrit d'emblée. Si le néfopam est retenu, la perfusion lente et jamais chez un patient au glaucome à angle fermé ou à l'adénome prostatique.",
        source: "RCP néfopam ; critères de Beers 2023 — charge anticholinergique",
    },
    Inappropriate {
        needs: &["tramadol"],
        never: &[],
        label: "Tramadol",
        from: 75,
        level: Level::Caution,
        risk: "Vertiges, confusion et chute ; hyponatrémie ; abaissement du seuil épileptogène ; et syndrome sérotoninergique en association à un antidépresseur, qui est l'association la plus fréquente à cet âge.",
        instead: "Le paracétamol d'abord, à dose pleine et régulière. Si un opioïde faible reste nécessaire, la dose la plus faible et des prises espacées, avec un laxatif prescrit d'emblée et la natrémie contrôlée dans les premières semaines.",
        source: "Critères STOPP/START v2 ; ANSM — bon usage du tramadol",
    },
    Inappropriate {
        needs: &["antiarythmique classe ia"],
        never: &[],
        label: "Antiarythmiques de classe Ia",
        from: 75,
        level: Level::Avoid,
        risk: "Effet proarythmique — torsades de pointes pour l'hydroquinidine, dès les premières prises et indépendamment de la dose ; élargissement du QRS et troubles de conduction pour la cibenzoline, qui donne en outre des hypoglycémies prolongées particulièrement chez le sujet âgé, l'insuffisant rénal et le patient de faible poids. Les deux ont des effets atropiniques et aggravent une insuffisance cardiaque.",
        instead: "Dans la fibrillation atriale, le contrôle de la fréquence par un bêta-bloquant et l'anticoagulation selon le score, qui est ce qui protège ; le maintien du rythme se discute avec le cardiologue et n'est pas un objectif en soi à cet âge.",
        source: "Laroche 2007 ; critères de Beers 2023 ; ESC — fibrillation atriale",
    },
    Inappropriate {
        // « antiarythmique » tout court : la ligne des classes Ia
        // cherche « antiarythmique classe ia », que cette fiche-ci ne
        // porte pas. Les deux mots ne se croisent donc pas.
        needs: &["amiodarone", "cordarone"],
        never: &[],
        label: "Amiodarone",
        from: 75,
        level: Level::Caution,
        risk: "Dysthyroïdies à l'iode dans les deux sens, pneumopathie interstitielle, hépatite, neuropathie périphérique et dépôts cornéens — des atteintes qui s'installent avec la dose cumulée, donc avec la durée, et qui se lisent d'abord comme un vieillissement : une fatigue, un essoufflement, une marche instable. Sa demi-vie se compte en semaines : une interaction dure encore un mois après l'arrêt.",
        instead: "Dans la fibrillation atriale, le contrôle de la fréquence par un bêta-bloquant est ce qu'on cherche d'abord à cet âge, avec l'anticoagulation selon le score. Si l'amiodarone est maintenue, la TSH et les transaminases sont contrôlées régulièrement, et tout essoufflement nouveau fait penser au poumon avant de penser au cœur.",
        source: "Critères de Beers 2023 ; ESC — fibrillation atriale ; ANSM",
    },
    Inappropriate {
        needs: &["digoxine", "digitalique"],
        never: &[],
        label: "Digoxine",
        from: 75,
        level: Level::Caution,
        risk: "Marge thérapeutique étroite et élimination rénale : nausées, troubles visuels, confusion et troubles du rythme. L'accumulation s'installe sans qu'aucune dose ait changé, dès que la fonction rénale baisse.",
        instead: "Dans la fibrillation atriale, un bêta-bloquant pour le contrôle de la fréquence. Si la digoxine reste indiquée, la plus faible dose qui suffit, la créatininémie et la kaliémie surveillées, et une digoxinémie au moindre doute.",
        source: "Laroche 2007 ; ESC — fibrillation atriale",
    },
    Inappropriate {
        needs: &["alpha-bloquant"],
        never: &[],
        label: "Alpha-bloquants",
        from: 75,
        level: Level::Caution,
        risk: "Hypotension orthostatique et syncope, surtout à l'instauration, à chaque augmentation de dose et en association à un autre antihypertenseur : c'est un pourvoyeur classique de chute.",
        instead: "La première prise le soir au coucher et une montée lente. La pression se mesure debout. Devant des chutes répétées, la classe se rediscute plutôt qu'elle ne se poursuit.",
        source: "Critères STOPP/START v2 ; RCP alfuzosine et tamsulosine",
    },
    Inappropriate {
        needs: &["metoclopramide"],
        never: &[],
        label: "Métoclopramide",
        from: 75,
        level: Level::Caution,
        risk: "Syndrome extrapyramidal et dyskinésies tardives, dont le risque croît avec l'âge et la durée du traitement ; somnolence et confusion.",
        instead: "Cinq jours au maximum et jamais en traitement de fond : la restriction de l'ANSM vaut à tout âge et davantage ici. Devant des nausées qui durent, c'est la cause qu'on cherche et non l'antiémétique qu'on renouvelle.",
        source: "ANSM 2012 — restriction d'indication du métoclopramide",
    },
    Inappropriate {
        // **Jamais « ipp ».** Trois lettres qui vivent dans
        // « gr-ipp-e » : le Tamiflu, le Relenza et deux vaccins
        // grippaux tombent dedans, dont l'Efluelda, qui est le vaccin
        // *du* sujet âgé. Les molécules sont nommées une par une.
        needs: &[
            "omeprazole",
            "pantoprazole",
            "lansoprazole",
            "rabeprazole",
        ],
        never: &[],
        label: "IPP au long cours",
        from: 75,
        level: Level::Caution,
        risk: "Au long cours et sans indication réévaluée : hypomagnésémie, carence en vitamine B12, fractures, infections digestives à Clostridioides difficile, et interactions — c'est le premier candidat à l'arrêt sur une ordonnance de dix lignes.",
        instead: "Réévaluer l'indication à chaque bilan de médication : un traitement d'entretien se justifie par un œsophagite sévère, un Barrett ou une prévention sous anti-inflammatoire, et non par une prescription qu'on renouvelle. Sinon la demi-dose, puis la prise à la demande, puis l'arrêt avec décroissance — un arrêt net donne un rebond acide qu'on prend pour une rechute.",
        source: "Critères STOPP/START v2 ; HAS — bon usage des inhibiteurs de la pompe à protons",
    },
    Inappropriate {
        needs: &["laxatif stimulant"],
        never: &[],
        label: "Laxatifs stimulants",
        from: 75,
        level: Level::Caution,
        risk: "Au long cours : douleurs abdominales, perte de potassium, et un transit qui finit par dépendre du produit — c'est-à-dire la constipation qu'on traite, entretenue.",
        instead: "Un laxatif osmotique en traitement de fond — macrogol, lactulose — avec les fibres, l'eau et la marche. Le stimulant garde sa place en dépannage et pour quelques jours.",
        source: "Laroche 2007 ; HAS — constipation chez la personne âgée",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn treat(name: &str) -> crate::revue::Treatment<'_> {
        crate::revue::Treatment {
            name,
            dci: "",
            class: "",
            tags: "",
        }
    }

    /// **Sans date de naissance, pas de verdict.**
    ///
    /// La ligne existe — elle dit que ce traitement est de ceux que
    /// l'âge décide — et elle ne conclut pas. Le type l'empêche : il n'y
    /// a nulle part où écrire « à éviter » sans avoir vu d'âge.
    #[test]
    fn without_a_birth_date_there_is_no_verdict() {
        let ordo = [treat("Valium"), treat("Doliprane")];
        let found = read(&ordo, None);
        assert_eq!(found.len(), 1, "seul le Valium relève de l'âge");
        assert_eq!(found[0].level, None);
        let seen = resolve(found, &crate::content::Overrides::default());
        assert!(!seen[0].decided());
        assert_eq!(undecided(&seen), 1);
    }

    /// **Au-dessous du seuil, la ligne se tait.**
    ///
    /// La moitié de la base est inappropriée à quelqu'un ; une liste qui
    /// parle de tout le monde ne parle de personne.
    #[test]
    fn a_file_younger_than_the_threshold_reads_nothing() {
        let ordo = [treat("Valium")];
        assert!(read(&ordo, Some(60)).is_empty());
        assert_eq!(read(&ordo, Some(80)).len(), 1);
        // Et le seuil s'applique **à partir de** l'âge écrit, pas au-delà.
        assert_eq!(read(&ordo, Some(75)).len(), 1);
        assert!(read(&ordo, Some(74)).is_empty());
    }

    /// **Une ligne qui se tait ne fait pas taire la suivante.**
    ///
    /// Toutes les lignes livrées parlent à 75 ans, si bien que le choix
    /// entre `break` et `continue` ne change rien aujourd'hui — et
    /// c'est exactement pourquoi il fallait l'éprouver maintenant. Le
    /// jour où une ligne citera STOPP à 65 ans, une boîte que la ligne
    /// large attrape d'abord perdrait sa ligne précise, à soixante-dix
    /// ans, sans que rien le dise.
    #[test]
    fn a_row_that_says_nothing_at_this_age_does_not_silence_the_next() {
        static TWO: &[Inappropriate] = &[
            Inappropriate {
                needs: &["benzodiazepine"],
                never: &[],
                label: "La large, à 75",
                from: 75,
                level: Level::Caution,
                risk: "Ce que l'âge fait courir, en une phrase assez longue pour le test.",
                instead: "Ce qu'on met à la place, en une phrase assez longue pour le test.",
                source: "test",
            },
            Inappropriate {
                needs: &["diazepam"],
                never: &[],
                label: "La précise, à 65",
                from: 65,
                level: Level::Avoid,
                risk: "Ce que l'âge fait courir, en une phrase assez longue pour le test.",
                instead: "Ce qu'on met à la place, en une phrase assez longue pour le test.",
                source: "test",
            },
        ];
        let valium = || crate::revue::Treatment {
            name: "Valium",
            dci: "diazépam",
            class: "benzodiazépine",
            tags: "",
        };
        // À 70 ans, la large se tait et la précise parle.
        let found = read_in(TWO, &[valium()], Some(70));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "La précise, à 65");
        // À 80, c'est l'ordre qui décide, et la large est la première.
        let found = read_in(TWO, &[valium()], Some(80));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "La large, à 75");
        // À 60, personne ne parle.
        assert!(read_in(TWO, &[valium()], Some(60)).is_empty());
    }

    /// **Un « non » nomme ce qu'il évite et ce qu'on met à la place.**
    ///
    /// « À éviter » tout seul n'apprend rien et ne se discute pas avec
    /// un prescripteur : c'est la règle que `crush.rs` a écrite en
    /// premier — un « non » sans alternative laisse le problème entier.
    #[test]
    fn every_row_names_a_risk_and_an_alternative() {
        for a in TABLE {
            assert!(
                a.risk.trim().len() > 40,
                "{} : le risque n'est pas nommé",
                a.label
            );
            assert!(
                a.instead.trim().len() > 40,
                "{} : aucune alternative",
                a.label
            );
            assert!(!a.source.trim().is_empty(), "{} : sans source", a.label);
            for text in [a.risk, a.instead] {
                assert!(
                    text.trim_end().ends_with('.'),
                    "{} : « {} » n'est pas une phrase",
                    a.label,
                    text
                );
            }
        }
    }

    /// **L'alternative est un choix, jamais une posologie.**
    ///
    /// Aucun milligramme dans ce qu'on met à la place : la dose dépend
    /// de l'indication, du poids et du rein, et c'est l'affaire du
    /// prescripteur. Les fractions — « la moitié de la dose adulte » —
    /// sont l'écriture retenue, comme dans `renal.rs`.
    #[test]
    fn an_alternative_is_a_choice_and_never_a_dose() {
        for a in TABLE {
            let folded = crate::fuzzy::sort_key(a.instead);
            for forbidden in [" mg", "mg ", "microgramme", "ui/", " g/j"] {
                assert!(
                    !folded.contains(forbidden),
                    "{} : l'alternative écrit une posologie — « {} »",
                    a.label,
                    a.instead
                );
            }
        }
    }

    /// **On n'envoie jamais d'un médicament à éviter vers un autre.**
    ///
    /// La faute qu'aucune relecture ne voit : chaque ligne est juste
    /// prise seule, et la paire envoie en rond — « à la place du
    /// diazépam, l'hydroxyzine », quand l'hydroxyzine est trois lignes
    /// plus bas. Le test lit chaque alternative, cherche les fiches
    /// livrées qu'elle nomme, et refuse qu'une seule d'entre elles soit
    /// réclamée par une ligne « à éviter ».
    #[test]
    fn an_alternative_is_never_itself_a_row_to_avoid() {
        let mut wrong: Vec<String> = Vec::new();
        for a in TABLE {
            let said = crate::fuzzy::sort_key(a.instead);
            for (name, dci, class, _) in crate::db::STARTER_DRUGS {
                // La DCI plutôt que le nom de marque : c'est la molécule
                // qu'une alternative nomme. Et une DCI d'un seul mot,
                // sinon « fer » et « opium » attrapent des phrases.
                if dci.split_whitespace().count() != 1 || dci.len() < 6 {
                    continue;
                }
                if !crate::fuzzy::contains_folded(&said, dci) {
                    continue;
                }
                let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class}"));
                if let Some(other) = TABLE
                    .iter()
                    .find(|o| claims(o, &hay) && o.level == Level::Avoid)
                {
                    wrong.push(format!(
                        "« {} » propose {dci}, que « {} » dit d'éviter",
                        a.label, other.label
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "renvois en rond :\n{}", wrong.join("\n"));
    }

    /// **Un seuil d'âge vient d'une liste publiée.**
    ///
    /// 75 ans est celui de la liste française, 65 celui de STOPP et de
    /// Beers. Un seuil intermédiaire serait une intuition écrite comme
    /// une règle, et c'est exactement ce que `renal.rs` refuse d'une
    /// clairance interpolée.
    #[test]
    fn a_threshold_comes_from_a_published_list() {
        for a in TABLE {
            assert!(
                a.from == 75 || a.from == 65,
                "{} : seuil de {} ans, qui n'est celui d'aucune liste",
                a.label,
                a.from
            );
        }
    }

    /// **Une ligne qu'aucune fiche n'atteint ne se voit jamais.**
    ///
    /// Le filet qui attrape une faute de frappe dans un `needs` le jour
    /// où la ligne est écrite : jusque-là, la ligne se contente de ne
    /// rien faire, ce qu'aucun test ne distingue d'une ligne correcte.
    #[test]
    fn every_row_can_fire_on_the_base_as_shipped() {
        for row in TABLE {
            let reachable = row.needs.iter().any(|needle| {
                crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::contains_folded(
                            &crate::fuzzy::sort_key(&format!("{name} {dci} {class}")),
                            needle,
                        )
                    })
            });
            assert!(
                reachable,
                "{} : aucune fiche livrée ne correspond à {:?}",
                row.label, row.needs
            );
        }
    }

    /// **Et chacun de ses mots, pris seul, atteint quelque chose.**
    ///
    /// Un `needs` mort ne fait rien de mal et ne fait rien du tout : il
    /// donne l'impression d'une couverture qui n'existe pas. C'est la
    /// version par mot du test ci-dessus, et elle a trouvé sa faute de
    /// frappe le jour où elle a été écrite.
    #[test]
    fn no_needle_of_the_table_is_dead() {
        let mut dead: Vec<String> = Vec::new();
        for row in TABLE {
            for needle in row.needs {
                let alive = crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::contains_folded(
                            &crate::fuzzy::sort_key(&format!("{name} {dci} {class}")),
                            needle,
                        )
                    });
                if !alive {
                    dead.push(format!("{} : « {needle} »", row.label));
                }
            }
        }
        assert!(dead.is_empty(), "mots morts :\n{}", dead.join("\n"));
    }

    /// **Une forme locale ne porte pas le niveau systémique.**
    ///
    /// La table est indexée sur la molécule, comme les six autres : un
    /// collyre à l'indométacine tombe dans la ligne des AINS et
    /// recevrait « hémorragie digestive ». Écarté en haut de [`read`],
    /// par `classes::is_local_form`.
    #[test]
    fn a_local_form_does_not_wear_the_systemic_level() {
        let collyre = crate::revue::Treatment {
            name: "Indocollyre",
            dci: "indométacine",
            class: "collyre — AINS",
            tags: "",
        };
        assert!(read(&[collyre], Some(82)).is_empty());
        // La pastille pour la gorge n'est pas une forme locale au sens
        // de `classes` — « pastille » n'est pas un mot de voie — et
        // c'est le veto de la ligne qui l'écarte.
        let pastille = crate::revue::Treatment {
            name: "Strefen",
            dci: "flurbiprofène",
            class: "AINS — pastille pour la gorge",
            tags: "",
        };
        assert!(read(&[pastille], Some(82)).is_empty());
        // Et l'AINS général, lui, parle.
        let systemic = crate::revue::Treatment {
            name: "Advil",
            dci: "ibuprofène",
            class: "AINS",
            tags: "",
        };
        assert_eq!(read(&[systemic], Some(82)).len(), 1);
    }

    /// **L'ordre décide quand deux lignes pourraient réclamer une
    /// boîte**, et c'est la première qui gagne — comme dans `crush.rs`,
    /// où « actiskenan » contient « skenan ».
    ///
    /// Le Valium est une benzodiazépine à demi-vie longue *et* une
    /// benzodiazépine : sans l'ordre, il recevrait « demi-dose » là où
    /// la liste française dit « une autre molécule ». Le Donormyl est un
    /// hypnotique *et* un antihistaminique sédatif : c'est la charge
    /// atropinique qui le décrit.
    #[test]
    fn the_first_row_that_claims_a_box_is_the_one_that_speaks() {
        for (name, dci, class, expected) in [
            (
                "Valium",
                "diazépam",
                "benzodiazépine",
                "Benzodiazépines à demi-vie longue",
            ),
            (
                "Séresta",
                "oxazépam",
                "benzodiazépine",
                "Benzodiazépines et apparentés",
            ),
            (
                "Donormyl",
                "doxylamine",
                "hypnotique antihistaminique",
                "Antihistaminiques H1 de première génération",
            ),
            (
                "Colchimax",
                "colchicine + tiémonium + opium",
                "anti-goutteux",
                "Colchicine associée à un atropinique et à l'opium",
            ),
        ] {
            let t = crate::revue::Treatment {
                name,
                dci,
                class,
                tags: "",
            };
            let found = read(&[t], Some(82));
            assert_eq!(found.len(), 1, "{name}");
            assert_eq!(found[0].label, expected, "{name}");
        }
    }

    /// **Ce que la table appelle « à demi-vie longue », la fiche
    /// l'écrit.**
    ///
    /// Le seuil se lit « au moins vingt heures » : le Lexomil en écrit
    /// vingt pile, la liste française le nomme et la table de référence
    /// « Sujet âgé » aussi. Écrit « plus de vingt », le module disait du
    /// bromazépam le contraire de la table livrée avec lui.
    ///
    /// La base porte déjà la demi-vie plasmatique de chaque boîte
    /// (`facets::HALF_LIVES`), et c'est elle qui décide : une ligne
    /// adossée à un chiffre que la fiche donne se corrige en corrigeant
    /// la fiche. Le test tient les deux sens — aucune benzodiazépine
    /// sous vingt heures n'est attrapée par la ligne longue, et aucune
    /// au-dessus ne lui échappe.
    #[test]
    fn a_long_half_life_is_the_one_the_fiche_writes() {
        let long = TABLE
            .iter()
            .find(|a| a.label == "Benzodiazépines à demi-vie longue")
            .expect("la ligne des benzodiazépines longues");
        let mut wrong: Vec<String> = Vec::new();
        for (name, dci, class, _) in crate::db::STARTER_DRUGS {
            if !crate::fuzzy::contains_folded(&crate::fuzzy::sort_key(class), "benzodiazepine") {
                continue;
            }
            // Le clonazépam et le midazolam sont écartés par le veto :
            // leur indication décide, pas leur demi-vie.
            let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class}"));
            if long
                .never
                .iter()
                .any(|n| crate::fuzzy::contains_folded(&hay, n))
            {
                continue;
            }
            let Some(f) = crate::facets::facets(name) else {
                continue;
            };
            let Some(high) = f.half_life.sort_key() else {
                continue;
            };
            let claimed = claims(long, &hay);
            if claimed != (high >= 20.0) {
                wrong.push(format!(
                    "{name} : demi-vie {} h, {} par la ligne longue",
                    high,
                    if claimed { "attrapé" } else { "laissé" }
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "la table et les fiches ne disent pas la même chose :\n{}",
            wrong.join("\n")
        );
    }

    /// **Le module et la table de référence disent la même chose.**
    ///
    /// L'application livre déjà une table « Sujet âgé — médicaments à
    /// réévaluer et alternatives », lue dans « Tables de conversion »,
    /// et ce module en est la lecture *ordonnance par ordonnance*. Deux
    /// écritures d'une même question : sans confrontation, elles
    /// dérivent, et c'est celle qu'on regarde le moins qui aura tort.
    /// Elle a d'ailleurs déjà mordu — la table nommait le bromazépam
    /// parmi les benzodiazépines à demi-vie longue et le module le
    /// laissait passer, à vingt heures pile.
    ///
    /// Ce que le test vérifie est ce qu'il peut vérifier : chaque ligne
    /// de la table qui nomme une **molécule** doit trouver un écho dans
    /// le module. Trois lignes de la table n'en ont pas, et c'est
    /// voulu : l'association de trois psychotropes est une question sur
    /// l'**ensemble** de l'ordonnance — c'est `revue.rs` qui la pose —,
    /// l'antihypertenseur inchangé après un amaigrissement ne se
    /// reconnaît à aucun mot d'une fiche, et les traitements
    /// **manquants** de START sont l'autre moitié du sujet, que ce
    /// module ne fait pas : il lit ce qui est là.
    #[test]
    fn the_table_of_reference_and_this_module_agree() {
        let table = crate::tables::TABLES
            .iter()
            .find(|t| t.short == "Sujet âgé")
            .expect("la table « Sujet âgé »");
        // Ce que le module sait dire, tous mots confondus.
        let known: String = TABLE
            .iter()
            .flat_map(|a| a.needs.iter().copied().chain(std::iter::once(a.label)))
            .collect::<Vec<_>>()
            .join(" ");
        let known = crate::fuzzy::sort_key(&known);
        // Les trois lignes qui n'ont pas à trouver d'écho, nommées et
        // non devinées : une exemption muette est une dérive qui passe.
        const ELSEWHERE: &[&str] = &[
            "Association de trois psychotropes",
            "Antihypertenseurs à dose inchangée",
            "Traitements souvent manquants",
        ];
        let mut orphans: Vec<&str> = Vec::new();
        for row in table.rows {
            let subject = row[0];
            if ELSEWHERE.iter().any(|e| subject.starts_with(e)) {
                continue;
            }
            // **Dans les deux sens**, parce que les deux écritures ne
            // nomment pas la même chose de la même façon : la table
            // écrit « AINS au long cours » là où le module écrit
            // « Anti-inflammatoires non stéroïdiens » et cherche
            // « ains ». Un mot du sujet reconnu par le module, ou un
            // mot du module trouvé dans le sujet : l'un des deux suffit.
            let folded = crate::fuzzy::sort_key(subject);
            let by_subject = subject
                .split(|c: char| !c.is_alphanumeric() && c != '\'')
                .filter(|w| w.chars().count() > 5)
                .any(|w| crate::fuzzy::contains_folded(&known, &crate::fuzzy::sort_key(w)));
            // Les mots cherchés du module, et le **premier mot** de son
            // libellé — « Digoxine », « Tramadol », « IPP ». Le libellé
            // entier ne servirait à rien (« IPP au long cours » n'est
            // dans aucun sujet) et ses autres mots serviraient trop
            // (« long », « cours » sont partout).
            let by_module = TABLE.iter().any(|a| {
                a.needs
                    .iter()
                    .copied()
                    .chain(a.label.split_whitespace().take(1))
                    .filter(|w| w.chars().count() >= 3)
                    .any(|w| crate::fuzzy::contains_folded(&folded, w))
            });
            if !by_subject && !by_module {
                orphans.push(subject);
            }
        }
        assert!(
            orphans.is_empty(),
            "la table de référence nomme ce que le module ignore — l'une \
             des deux a dérivé :\n{}",
            orphans.join("\n")
        );
    }

    /// Deux entrées ne doivent pas se disputer un mot : la première
    /// gagne, et si deux se recouvrent c'est un choix à faire dans la
    /// table plutôt qu'à laisser à l'ordre.
    #[test]
    fn no_two_rows_claim_the_same_word() {
        for a in TABLE {
            for n in a.needs {
                let key = crate::fuzzy::sort_key(n);
                let claimers: Vec<&str> = TABLE
                    .iter()
                    .filter(|o| claims(o, &key))
                    .map(|o| o.label)
                    .collect();
                assert_eq!(
                    claimers,
                    vec![a.label],
                    "« {n} » est réclamé par plusieurs lignes"
                );
            }
        }
    }

    /// **Ce que la table écrit est dessiné tel quel.** `RichText`
    /// n'interprète aucun balisage : une astérisque écrite pour insister
    /// sort à l'écran comme une astérisque.
    #[test]
    fn the_table_writes_no_markup() {
        for a in TABLE {
            for text in [a.label, a.source, a.risk, a.instead] {
                assert!(
                    !text.contains("**") && !text.contains('`'),
                    "{} : « {text} » porte du balisage",
                    a.label
                );
            }
        }
    }

    /// **Une phrase s'adresse par sa ligne et son rôle.**
    ///
    /// Ni le rang, qui se décale dès qu'on insère une ligne, ni la
    /// prose, qui disparaît dès qu'on la corrige : deux phrases
    /// partageant une adresse feraient hériter la seconde de la
    /// réécriture de la première.
    #[test]
    fn a_phrase_is_addressed_by_its_row_and_its_role() {
        let mut ids: Vec<String> = addressed().into_iter().map(|(k, ..)| k).collect();
        assert_eq!(ids.len(), TABLE.len() * 2);
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "deux phrases partagent une adresse");
    }

    /// Toute phrase s'édite, et toute réécriture arrive.
    #[test]
    fn every_phrase_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        assert_eq!(listed.len(), addressed().len());
        let found = read(&[treat("Valium")], Some(82));
        assert_eq!(found.len(), 1);
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        for f in resolve(found.clone(), &over) {
            assert!(f.risk.starts_with("réécrit:"), "{}", f.risk);
            assert!(f.instead.starts_with("réécrit:"), "{}", f.instead);
        }
        // Sans réécriture, les mots de la table — et le niveau ne bouge
        // jamais : c'est un seuil, pas une tournure.
        let plain = resolve(found.clone(), &crate::content::Overrides::default());
        assert_eq!(plain[0].risk, found[0].risk);
        assert_eq!(plain[0].instead, found[0].instead);
        assert_eq!(plain[0].level, found[0].level);
        assert_eq!(plain[0].from, found[0].from);
    }

    /// **Le pire se lit d'abord, et l'ordre est total.**
    ///
    /// Deux lignes de même niveau ne doivent pas échanger leur place
    /// d'une image à l'autre : un panneau qui se réordonne entre deux
    /// trames est un panneau qu'on relit à chaque fois.
    #[test]
    fn the_worst_is_read_first_and_the_order_is_total() {
        // Des traitements entiers, et non des noms nus : « Advil » ne
        // porte « AINS » que dans sa classe, et une ordonnance réelle
        // arrive ici avec ses quatre champs.
        let card = |name, dci, class| crate::revue::Treatment {
            name,
            dci,
            class,
            tags: "",
        };
        let ordo = [
            card("Tramadol", "tramadol", "opioïde faible"),
            card("Valium", "diazépam", "benzodiazépine"),
            card("Digoxine", "digoxine", "digitalique"),
            card("Advil", "ibuprofène", "AINS"),
        ];
        let read = read(&ordo, Some(82));
        let levels: Vec<Option<Level>> = read.iter().map(|f| f.level).collect();
        assert_eq!(
            levels,
            vec![
                Some(Level::Avoid),
                Some(Level::Avoid),
                Some(Level::Caution),
                Some(Level::Caution),
            ]
        );
        // À niveau égal, l'ordre est celui du libellé puis du dossier, et
        // jamais celui de l'ordonnance : « AINS » avant « benzodiazépine »,
        // « Digoxine » avant « Tramadol ».
        let labels: Vec<&str> = read.iter().map(|f| f.label).collect();
        assert_eq!(
            labels,
            vec![
                "Anti-inflammatoires non stéroïdiens",
                "Benzodiazépines à demi-vie longue",
                "Digoxine",
                "Tramadol",
            ]
        );
        assert_eq!(read, super::read(&ordo, Some(82)));
    }

    /// **Le cliquet : la table ne perd pas de lignes.**
    ///
    /// La règle de la maison pour tout catalogue clinique. Le nombre est
    /// écrit **une fois**, dans une constante que le message relit.
    #[test]
    fn the_table_only_ever_grows() {
        const FLOOR: usize = 26;
        assert!(
            TABLE.len() >= FLOOR,
            "{} lignes, il y en avait {FLOOR}",
            TABLE.len()
        );
    }

    /// **Aucune ligne ne prête à une fiche ce que cette fiche ne dit
    /// pas.**
    ///
    /// La confrontation de `cyp` et de `hepatic` : pour chaque fiche
    /// livrée, prendre la **première** ligne qui l'accroche — celle que
    /// [`read`] rendra — et la juger contre cette fiche-là. Ici la
    /// question est simple et forte : la fiche parle-t-elle de l'âge ?
    /// Une ligne posée sur une fiche dont aucune section ne nomme le
    /// sujet âgé est une ligne écrite de mémoire, et c'est ainsi qu'on
    /// prête à une boîte ce qu'on a lu d'une autre.
    #[test]
    fn no_row_claims_a_card_that_never_speaks_of_age() {
        let cards: std::collections::HashMap<String, String> = crate::db::STARTER_DETAILS
            .iter()
            .map(|d| {
                (
                    crate::fuzzy::sort_key(d.name),
                    crate::fuzzy::sort_key(&format!(
                        "{} {} {} {} {}",
                        d.adverse, d.monitoring, d.dosage, d.contraindications, d.indications
                    )),
                )
            })
            .collect();
        let mut mute: Vec<String> = Vec::new();
        for (name, dci, class, _) in crate::db::STARTER_DRUGS {
            if crate::classes::is_local_form(class) {
                continue;
            }
            let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class}"));
            let Some(a) = TABLE.iter().find(|a| claims(a, &hay)) else {
                continue;
            };
            let Some(prose) = cards.get(&crate::fuzzy::sort_key(name)) else {
                continue;
            };
            let speaks = ["age", "sujet age", "personne age", "75 ans", "65 ans"]
                .iter()
                .any(|w| crate::fuzzy::contains_folded(prose, w));
            if !speaks {
                mute.push(format!("{} → {name}", a.label));
            }
        }
        assert!(
            mute.is_empty(),
            "lignes posées sur une fiche qui ne parle jamais de l'âge :\n{}",
            mute.join("\n")
        );
    }
}
