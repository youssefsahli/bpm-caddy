//! Grossesse et allaitement, comme niveau et non comme paragraphe.
//!
//! Une ordonnance de treize lignes chez une femme enceinte, ce sont
//! treize paragraphes à ouvrir. Ce module en fait treize niveaux, et
//! l'ordonnance se lit d'un coup.
//!
//! **Ce module ne remplace pas le CRAT.** Le Centre de référence sur
//! les agents tératogènes est la référence française, il est tenu à
//! jour molécule par molécule, et il est en ligne. Ce qui est écrit ici
//! est un rappel de comptoir qui dit où aller : chaque ligne cite sa
//! source, et le panneau comme la feuille renvoient au CRAT. Une table
//! figée dans un binaire vieillit ; la référence, non.
//!
//! Six règles le tiennent, une par test :
//!
//! * **« Pas de donnée » n'est pas « pas de risque ».** C'est la règle
//!   qui décide de tout le reste, et c'est la même que celle du module
//!   d'écrasement sous un autre habit : ce qu'on n'a pas écrit ne se lit
//!   jamais comme un feu vert. Une molécule absente de la table est
//!   `SansDonnee`, pas `Compatible`, et le type interdit de confondre
//!   les deux.
//! * **Grossesse et allaitement sont deux questions.** Une molécule
//!   peut être compatible avec l'une et non avec l'autre — la codéine
//!   est l'exemple que tout le monde connaît — et une réponse unique
//!   serait fausse une fois sur deux.
//! * **Le terme change la réponse.** Un AINS n'est pas « à éviter » : il
//!   est formellement contre-indiqué à partir de vingt-quatre semaines
//!   d'aménorrhée, et déconseillé avant. Une entrée dont le niveau
//!   dépend du terme doit le dire, et c'est un test.
//! * **Une contre-indication dit ce qu'on met à la place**, quand il y
//!   a quelque chose — sans quoi la ligne renvoie le problème au
//!   comptoir sans l'avancer.
//! * **Une ligne de classe ne contredit pas la fiche qu'elle
//!   revendique.** Une classe parle de la classe, et un membre — ou une
//!   association — peut dire autre chose : le RCP du rabéprazole le
//!   contre-indique là où les IPP sont utilisables, la Lamaline porte de
//!   l'opium sous son paracétamol, le Xigduo une gliflozine sous sa
//!   metformine, l'Actifed Rhume et le Dérinox un vasoconstricteur sous
//!   leur composant rassurant. Les deux textes étaient dans le logiciel
//!   et rien ne les mettait face à face : la table était cohérente avec
//!   elle-même, et c'est la **rencontre** avec les fiches livrées qui le
//!   montre. Le plus précis passe devant, et le test est étroit — il ne
//!   parle que d'une ligne compatible des deux côtés dont la fiche écrit
//!   une contre-indication, parce qu'un test qui crie au loup finit
//!   désactivé.
//! * **La table ne décide de rien.** Elle rappelle, elle cite, elle
//!   renvoie. L'arrêt ou le maintien d'un traitement chez une femme
//!   enceinte est une décision médicale, et une grossesse mal
//!   accompagnée est plus dangereuse qu'un traitement poursuivi.
//!
//! Statique, pur et testé. Il ne connaît ni la base ni egui.

/// Les deux questions, qui ne sont pas la même.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Grossesse,
    Allaitement,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Grossesse => "Grossesse",
            Stage::Allaitement => "Allaitement",
        }
    }
}

/// Ce que la référence dit. L'ordre est celui de la gravité : ce qui
/// est interdit se lit avant ce qui est possible.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    /// Contre-indication formelle.
    Interdit,
    /// À éviter : un autre choix existe et vaut mieux.
    Eviter,
    /// Possible, sous conditions ou sous surveillance.
    Prudence,
    /// Utilisable — c'est ce que la référence dit, pas une absence
    /// d'information.
    Compatible,
    /// **La table ne sait pas.** Ce n'est pas « compatible » : c'est ce
    /// qu'on répond quand on n'a pas ouvert le CRAT, et cela demande de
    /// l'ouvrir.
    SansDonnee,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Interdit => "Contre-indiqué",
            Level::Eviter => "À éviter",
            Level::Prudence => "Possible, avec précautions",
            Level::Compatible => "Compatible",
            Level::SansDonnee => "À vérifier au CRAT",
        }
    }

    /// La ligne demande qu'on s'arrête dessus.
    pub fn worrying(self) -> bool {
        matches!(self, Level::Interdit | Level::Eviter | Level::SansDonnee)
    }
}

/// Ce que la référence dit d'une molécule, des deux côtés.
pub struct Advice {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes.
    /// **Les plus précis d'abord** : la table est lue dans l'ordre.
    pub needs: &'static [&'static str],
    /// Ce que la ligne **ne réclame pas**, bien que ses mots l'attrapent.
    ///
    /// Même veto que dans `renal.rs`, et pour la même raison : la table
    /// est indexée sur la molécule, et une forme locale de la même
    /// molécule y tombe. L'Exocine — collyre à l'ofloxacine —, le
    /// Lithioderm — gel au gluconate de lithium — et l'Auréomycine
    /// Evans — pommade à la chlortétracycline — recevaient le niveau de
    /// la voie générale, quand leurs fiches écrivent l'inverse en
    /// toutes lettres : « utilisable pendant la grossesse […] compte
    /// tenu du passage systémique négligeable », « contrairement au
    /// lithium administré par voie générale », « l'usage local […] est
    /// acceptable ».
    ///
    /// Ce n'est pas vrai de toute forme locale, et le veto se pose donc
    /// boîte par boîte : le Sterdex, pommade ophtalmique, garde son
    /// « à éviter » parce que sa propre fiche le déconseille à partir
    /// du deuxième trimestre, « même si l'exposition par voie locale
    /// est très faible ». Et les vasoconstricteurs nasaux restent dans
    /// leur ligne, qui est écrite pour eux.
    pub never: &'static [&'static str],
    pub label: &'static str,
    pub pregnancy: Level,
    /// Ce que le terme change, quand il change quelque chose. Vide
    /// sinon — et un niveau qui dépend du terme sans le dire est refusé
    /// par un test.
    pub term: &'static str,
    pub pregnancy_note: &'static str,
    pub breastfeeding: Level,
    pub breastfeeding_note: &'static str,
    /// Toujours le CRAT, plus le RCP quand il ajoute quelque chose.
    pub source: &'static str,
}

/// Ce que la table répond pour une ligne d'ordonnance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    pub treatment: String,
    pub label: &'static str,
    pub pregnancy: Level,
    pub term: &'static str,
    pub pregnancy_note: &'static str,
    pub breastfeeding: Level,
    pub breastfeeding_note: &'static str,
    pub source: &'static str,
}

impl Finding {
    /// Le pire des deux niveaux : ce qui décide de l'ordre de lecture.
    pub fn worst(&self) -> Level {
        self.pregnancy.min(self.breastfeeding)
    }
}

/// Ce que la grossesse et l'allaitement font à cette ordonnance.
///
/// **Toute ligne reçoit une réponse**, y compris « à vérifier au
/// CRAT » : une feuille qui ne montrerait que les interdits se lirait
/// comme une autorisation pour tout le reste.
///
/// L'ordre est celui de la gravité, puis celui du nom, pour qu'une
/// lecture faite deux fois soit deux fois la même.
/// Cette ligne réclame-t-elle cette boîte ?
///
/// Écrit **une fois**, comme dans `renal.rs`, et appelé par `read` comme
/// par le test qui confronte la table aux fiches livrées. Les deux
/// avaient chacun leur copie.
fn claims(a: &Advice, hay: &str) -> bool {
    a.needs
        .iter()
        .any(|n| crate::fuzzy::contains_folded(hay, n))
        && !a
            .never
            .iter()
            .any(|n| crate::fuzzy::contains_folded(hay, n))
}

pub fn read(treatments: &[crate::revue::Treatment]) -> Vec<Finding> {
    let mut out: Vec<Finding> = treatments
        .iter()
        .map(|t| {
            let hay =
                crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags));
            let hit = TABLE.iter().find(|a| claims(a, &hay));
            match hit {
                Some(a) => Finding {
                    treatment: t.name.trim().to_owned(),
                    label: a.label,
                    pregnancy: a.pregnancy,
                    term: a.term,
                    pregnancy_note: a.pregnancy_note,
                    breastfeeding: a.breastfeeding,
                    breastfeeding_note: a.breastfeeding_note,
                    source: a.source,
                },
                None => Finding {
                    treatment: t.name.trim().to_owned(),
                    label: "",
                    pregnancy: Level::SansDonnee,
                    term: "",
                    pregnancy_note: "Cette molécule n'est pas dans la table. Le CRAT la traite molécule par molécule et se tient à jour : c'est lui qu'il faut ouvrir.",
                    breastfeeding: Level::SansDonnee,
                    breastfeeding_note: "",
                    source: "lecrat.fr",
                },
            }
        })
        .collect();
    out.sort_by(|a, b| {
        a.worst()
            .cmp(&b.worst())
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

/// Ce que la référence dit, molécule par molécule.
///
/// **Rien ici ne remplace le CRAT** : ces lignes sont un rappel de
/// comptoir, elles citent leur source, et elles disent où aller quand
/// elles ne savent pas. Les molécules retenues sont celles dont la
/// question se pose vraiment devant un comptoir français.
/// Le document sous lequel les phrases du panneau sont adressées.
pub const DOC: &str = "grossesse";

/// Toutes les phrases du panneau, avec leur adresse.
///
/// Le repère est le **libellé de la molécule** : il la désigne, il ne
/// bouge pas, et une molécule ajoutée au tableau ne périme donc aucune
/// réécriture. La source n'en est pas une — c'est une référence, pas une
/// tournure.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::new();
    for a in TABLE {
        let id = crate::content::slug(a.label);
        out.push((
            crate::content::key(DOC, &id, "grossesse"),
            "grossesse",
            a.pregnancy_note,
        ));
        out.push((
            crate::content::key(DOC, &id, "allaitement"),
            "allaitement",
            a.breastfeeding_note,
        ));
        // Le terme n'existe que pour les molécules dont le niveau en
        // dépend : une ligne vide n'est pas une phrase à proposer.
        if !a.term.is_empty() {
            out.push((crate::content::key(DOC, &id, "terme"), "terme", a.term));
        }
    }
    out
}

/// Appliquer les réécritures de l'officine à ce que la table répond.
pub fn resolve(findings: Vec<Finding>, over: &crate::content::Overrides) -> Vec<Resolved> {
    findings
        .into_iter()
        .map(|f| {
            let id = crate::content::slug(f.label);
            Resolved {
                treatment: f.treatment,
                label: f.label,
                pregnancy: f.pregnancy,
                term: over
                    .get(&crate::content::key(DOC, &id, "terme"), f.term)
                    .to_owned(),
                pregnancy_note: over
                    .get(
                        &crate::content::key(DOC, &id, "grossesse"),
                        f.pregnancy_note,
                    )
                    .to_owned(),
                breastfeeding: f.breastfeeding,
                breastfeeding_note: over
                    .get(
                        &crate::content::key(DOC, &id, "allaitement"),
                        f.breastfeeding_note,
                    )
                    .to_owned(),
                source: f.source,
            }
        })
        .collect()
}

/// Ce que la table répond, avec les mots de l'officine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resolved {
    pub treatment: String,
    pub label: &'static str,
    pub pregnancy: Level,
    pub term: String,
    pub pregnancy_note: String,
    pub breastfeeding: Level,
    pub breastfeeding_note: String,
    pub source: &'static str,
}

impl Resolved {
    /// Le pire des deux niveaux : ce qui décide de l'ordre de lecture.
    ///
    /// `min` et non `max` : la table est déclarée du pire au meilleur —
    /// `Interdit` d'abord — donc le plus grave est le plus petit. Écrit
    /// à l'envers ici, une contre-indication de grossesse passait pour
    /// une prudence d'allaitement, et le test l'a dit.
    pub fn worst(&self) -> Level {
        self.pregnancy.min(self.breastfeeding)
    }
}

pub const TABLE: &[Advice] = &[
    // --- Ce qui ne se discute pas -------------------------------------
    Advice {
        needs: &["valproate", "depakine", "depakote", "micropakine"],
        never: &[],
        label: "Valproate",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse, et avant elle.",
        pregnancy_note: "Malformations chez environ un enfant exposé sur dix et troubles neurodéveloppementaux chez trois à quatre sur dix. Chez toute femme en âge d'avoir des enfants, la délivrance est conditionnée au programme de prévention des grossesses : accord de soins annuel et prescription initiale spécialisée.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Passage faible dans le lait ; l'allaitement est possible, avec surveillance du nourrisson.",
        source: "CRAT ; ANSM, programme de prévention des grossesses",
    },
    Advice {
        needs: &["isotretinoine", "acitretine", "curacne", "procuta", "soriatane"],
        never: &[],
        label: "Rétinoïdes oraux",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse, et un mois après l'arrêt pour l'isotrétinoïne, deux ans pour l'acitrétine.",
        pregnancy_note: "Tératogène majeur. Contraception efficace et test de grossesse mensuel, ordonnance de sept jours pour l'isotrétinoïne : c'est le programme de prévention des grossesses.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; ANSM, programme de prévention des grossesses",
    },
    Advice {
        needs: &[
            // **Jamais « imeth ».** Aucune fiche livrée ne porte ce
            // nom de spécialité, et le fragment attrape en revanche le
            // « diméthylfumarate » du Skilarence — d-i-m-e-t-h. Les
            // mots cherchés sont des sous-chaînes d'un texte replié sans
            // espaces : un fragment court attrape ce qu'il ne vise pas.
            "methotrexate",
            "novatrex",
        ],
        never: &[],
        label: "Méthotrexate",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse.",
        pregnancy_note: "Tératogène et abortif. L'arrêt doit précéder la conception ; le délai se décide avec le prescripteur.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; RCP méthotrexate",
    },
    Advice {
        needs: &["mycophenolate", "cellcept", "myfortic"],
        never: &[],
        label: "Mycophénolate",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse.",
        pregnancy_note: "Tératogène : malformations de la face et des oreilles. Contraception obligatoire, programme de prévention des grossesses.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; ANSM",
    },
    // --- Ce que le terme change ---------------------------------------
    // **Avant les AINS, le paracétamol et les corticoïdes.** Un remède du
    // rhume porte un paracétamol ou une prednisolone, et c'est le
    // vasoconstricteur qui décide : l'Actifed Rhume est
    // « contre-indiqué pendant la grossesse » pour sa pseudoéphédrine,
    // le Dérinox pour sa naphazoline. Rangés sous leur composant
    // rassurant, ils s'annonçaient compatibles — et ce sont des boîtes
    // qu'on achète sans ordonnance. Le Rhinadvil porte les deux — un
    // AINS *et* une pseudoéphédrine — et c'est pourquoi cette ligne
    // passe aussi devant celle des AINS : le vasoconstricteur est
    // contre-indiqué à tout terme, là où l'AINS l'est à partir de 24 SA.
    Advice {
        needs: &[
            "pseudoephedrine",
            "naphazoline",
            "oxymetazoline",
            "ephedrine",
            "tuaminoheptane",
            "phenylephrine",
            "actifed",
            "dolirhume",
            "humexrhume",
            "rhinadvil",
            "derinox",
            "deturgylone",
            "aturgyl",
        ],
        never: &[],
        label: "Vasoconstricteurs du rhume",
        pregnancy: Level::Interdit,
        term: "",
        pregnancy_note: "C'est le vasoconstricteur qui décide, pas le paracétamol ni le corticoïde qui l'accompagne. Le lavage de nez au sérum physiologique est l'alternative de première intention, et le paracétamol seul pour la douleur ou la fièvre.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Déconseillé. Le lavage de nez reste le premier geste.",
        source: "CRAT ; Actifed Rhume : « Contre-indiqué pendant la grossesse et déconseillé pendant l'allaitement. » ; Dérinox : « Contre-indiqué ou fortement déconseillé pendant la grossesse et l'allaitement, du fait de l'effet vasoconstricteur systémique des sympathomimétiques. »",
    },
    Advice {
        needs: &[
            // **Jamais « ains » tout court.** Les mots cherchés sont des
            // sous-chaînes d'un texte replié sans espaces : « Apidra
            // insuline glulisine » devient « apidrainsulineglulisine »,
            // qui contient « ains ». Deux insulines recevaient ainsi la
            // ligne des AINS — « contre-indication formelle à partir de
            // 24 SA », c'est-à-dire, lue chez une diabétique enceinte,
            // d'arrêter son insuline. Les molécules sont nommées.
            "ibuprofene",
            "diclofenac",
            "ketoprofene",
            "naproxene",
            "celecoxib",
            "etoricoxib",
            "piroxicam",
            "acideniflumique",
            "acidetiaprofenique",
            "acidemefenamique",
            "advil",
            "nurofen",
        ],
        never: &[],
        label: "AINS",
        pregnancy: Level::Interdit,
        // « Début du 6e mois » et non « 5 mois et demi » : c'est la
        // formulation de l'ANSM, celle des dix fiches d'AINS livrées et
        // celle des deux autres tables. Le chiffre — 24 SA — était
        // partout le même ; c'est la glose qui divergeait.
        term: "À partir du début du 6e mois de grossesse, soit 24 semaines d'aménorrhée : contre-indication formelle, même en prise unique. Avant ce terme : à éviter.",
        pregnancy_note: "Après 24 SA, ils ferment le canal artériel du fœtus et atteignent son rein — l'accident est décrit après une seule prise. Le paracétamol est l'antalgique de la grossesse.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "L'ibuprofène passe très peu dans le lait : c'est l'AINS de l'allaitement.",
        source: "CRAT ; ANSM, alerte AINS et grossesse",
    },
    Advice {
        needs: &["aspirine", "acide acetylsalicylique", "kardegic", "aspegic"],
        never: &[],
        label: "Aspirine",
        pregnancy: Level::Prudence,
        term: "À dose antalgique (500 mg et plus) : contre-indiquée à partir de 24 SA, comme les AINS. À dose antiagrégante (75 à 160 mg), elle est au contraire prescrite dans la prévention de la prééclampsie.",
        pregnancy_note: "C'est la dose qui décide, et l'écart entre les deux est complet : la même molécule est un traitement de la grossesse à faible dose et un interdit à forte dose.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "À dose antalgique, préférer le paracétamol ou l'ibuprofène. La faible dose antiagrégante est compatible.",
        source: "CRAT",
    },
    Advice {
        needs: &["ramipril", "perindopril", "coversyl", "enalapril", "lisinopril", "captopril", "valsartan", "losartan", "candesartan", "irbesartan", "sartan"],
        never: &[],
        label: "IEC et sartans",
        pregnancy: Level::Interdit,
        term: "Aux deuxième et troisième trimestres : contre-indication. Au premier, à remplacer dès que la grossesse est connue.",
        pregnancy_note: "Fœtopathie : atteinte rénale, oligoamnios, défaut d'ossification du crâne. Un relais existe et se décide avec le prescripteur — l'hypertension de la grossesse se traite, elle ne se laisse pas.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Le captopril et l'énalapril sont utilisables pendant l'allaitement ; les autres molécules de la classe, non documentées.",
        source: "CRAT",
    },
    Advice {
        needs: &["codeine", "codoliprane", "dafalgan codeine", "tramadol", "contramal", "topalgic"],
        never: &[],
        label: "Codéine et tramadol",
        pregnancy: Level::Prudence,
        term: "En fin de grossesse : syndrome de sevrage et dépression respiratoire du nouveau-né si la prise est prolongée ou proche de l'accouchement.",
        pregnancy_note: "Ponctuellement, l'usage est possible. Le paracétamol reste le premier choix.",
        // La ligne porte le niveau de ce qu'elle a de plus inquiétant,
        // comme les associations : la codéine est **contre-indiquée**
        // pendant l'allaitement — c'est ce qu'écrivent la table de
        // référence « Grossesse et allaitement », la fiche de la
        // Lamaline, et la ligne voisine « Paracétamol + opium », déjà
        // en interdit. Elle était ici « à éviter », c'est-à-dire plus
        // permissive que les trois autres textes de l'application pour
        // la molécule qui, seule, a tué des nourrissons.
        breastfeeding: Level::Interdit,
        breastfeeding_note: "La codéine est contre-indiquée pendant l'allaitement : une mère métaboliseuse ultrarapide transforme trop de codéine en morphine, et des dépressions respiratoires du nourrisson ont été décrites, dont des décès. Le tramadol est à éviter pour la même raison, sans porter la même interdiction formelle. Préférer le paracétamol ou l'ibuprofène, compatibles l'un et l'autre.",
        source: "CRAT ; ANSM, contre-indication de la codéine pendant l'allaitement",
    },
    Advice {
        needs: &["nitrofurantoine", "furadantine"],
        never: &[],
        label: "Nitrofurantoïne",
        pregnancy: Level::Prudence,
        term: "En fin de grossesse et à l'accouchement : à éviter, risque d'hémolyse néonatale.",
        pregnancy_note: "Utilisable pendant le reste de la grossesse dans la cystite.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Possible ; à éviter chez le nourrisson de moins d'un mois ou déficitaire en G6PD.",
        source: "CRAT",
    },
    Advice {
        needs: &["doxycycline", "tetracycline", "minocycline", "tolexine"],
        // La pommade à la chlortétracycline n'est pas une cycline
        // générale : sa fiche écrit que « l'usage local sur une petite
        // surface et pour une durée brève est acceptable ». Le Sterdex,
        // lui, reste dedans : sa propre fiche le déconseille.
        never: &["aureomycine"],
        label: "Cyclines",
        pregnancy: Level::Eviter,
        term: "À partir du deuxième trimestre : coloration des dents de lait. Avant, l'exposition n'a pas d'effet connu.",
        pregnancy_note: "Une alternative existe presque toujours ; en parler au prescripteur.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Une cure courte est possible ; les cures prolongées ne le sont pas.",
        source: "CRAT",
    },
    // --- Ce qui se remplace -------------------------------------------
    Advice {
        needs: &["atorvastatine", "simvastatine", "rosuvastatine", "pravastatine", "vastatine", "tahor", "crestor"],
        never: &[],
        label: "Statines",
        pregnancy: Level::Eviter,
        term: "",
        pregnancy_note: "L'arrêt le temps de la grossesse est la règle : le bénéfice d'un traitement de prévention se compte en années, et neuf mois d'interruption ne changent rien au risque cardiovasculaire.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Non documentées dans le lait ; l'arrêt se poursuit.",
        source: "CRAT",
    },
    Advice {
        needs: &["warfarine", "coumadine", "fluindione", "previscan", "acenocoumarol", "avk"],
        never: &[],
        label: "AVK",
        pregnancy: Level::Interdit,
        term: "Entre 6 et 9 semaines d'aménorrhée surtout : embryopathie. Et risque hémorragique fœtal sur toute la grossesse.",
        pregnancy_note: "Le relais par héparine de bas poids moléculaire est la conduite habituelle, et il se décide avant la conception quand c'est possible.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "La warfarine et l'acénocoumarol passent très peu dans le lait ; l'allaitement est possible.",
        source: "CRAT",
    },
    Advice {
        needs: &["apixaban", "rivaroxaban", "edoxaban", "dabigatran", "eliquis", "xarelto", "pradaxa", "lixiana"],
        never: &[],
        label: "Anticoagulants oraux directs",
        pregnancy: Level::Interdit,
        term: "",
        pregnancy_note: "Pas de donnée suffisante et passage placentaire : le relais par héparine de bas poids moléculaire est la conduite.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Non documentés ; l'héparine, elle, est compatible.",
        source: "CRAT ; RCP des AOD",
    },
    Advice {
        needs: &["fluconazole", "triflucan"],
        never: &[],
        label: "Fluconazole",
        pregnancy: Level::Prudence,
        term: "",
        pregnancy_note: "La dose unique de 150 mg d'une mycose vaginale est utilisable. Les fortes doses prolongées, non : elles sont tératogènes.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable.",
        source: "CRAT",
    },
    Advice {
        needs: &["ciprofloxacine", "ofloxacine", "levofloxacine", "norfloxacine", "quinolone"],
        never: &["collyre", "exocine"],
        label: "Fluoroquinolones",
        pregnancy: Level::Prudence,
        term: "",
        pregnancy_note: "Utilisables si l'indication le demande — les données humaines sont rassurantes — mais une alternative est presque toujours préférable.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Une cure courte est possible.",
        source: "CRAT",
    },
    Advice {
        needs: &["lithium", "teralithe"],
        never: &["lithioderm", "gluconate de lithium"],
        label: "Lithium",
        pregnancy: Level::Prudence,
        term: "Au premier trimestre : légère augmentation du risque de malformation cardiaque. À l'accouchement : imprégnation du nouveau-né.",
        pregnancy_note: "L'arrêt d'un thymorégulateur pendant une grossesse est un risque en soi. C'est une décision de psychiatre, jamais une décision de comptoir. Lithiémie rapprochée.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Passage important dans le lait.",
        source: "CRAT",
    },
    // --- Ce que le comptoir rencontre et que la table ignorait ---------
    Advice {
        needs: &[
            "gliclazide", "glimépiride", "glibenclamide", "glipizide",
            "diamicron", "amarel", "daonil",
        ],
        never: &[],
        label: "Sulfamides hypoglycémiants",
        pregnancy: Level::Interdit,
        term: "",
        pregnancy_note: "Le diabète s'équilibre à l'insuline, seule option validée, et le relais s'organise avant la conception ou dès la découverte de la grossesse : c'est l'hypoglycémie néonatale sévère qui est en jeu. Un diabète non traité est plus dangereux qu'un relais bien conduit — le relais, pas l'arrêt.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Passage dans le lait et hypoglycémie du nourrisson.",
        source: "CRAT ; Diamicron et Amarel : « Contre-indiqué pendant la grossesse : le diabète doit être équilibré par l'insuline […] le relais doit être organisé avant la conception ou dès la découverte de la grossesse ».",
    },
    Advice {
        needs: &[
            "furosémide", "lasilix", "hydrochlorothiazide", "esidrex",
            "indapamide", "fludex", "bumétanide", "burinex",
        ],
        never: &[],
        label: "Diurétiques",
        pregnancy: Level::Eviter,
        term: "",
        pregnancy_note: "Ils réduisent le volume plasmatique et la perfusion placentaire. Jamais pour les œdèmes de la grossesse ni pour l'hypertension gravidique — c'est le contresens le plus fréquent, et les œdèmes des jambes en fin de grossesse sont physiologiques. Une indication cardiologique ou rénale stricte se poursuit, avec le prescripteur.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Passage dans le lait et inhibition de la lactation.",
        source: "CRAT ; Lasilix : « les diurétiques ne doivent pas être utilisés pour traiter les œdèmes physiologiques ni l'hypertension gravidique » ; Esidrex : « Ils ne doivent jamais être utilisés pour traiter les œdèmes physiologiques de la grossesse ».",
    },
    Advice {
        needs: &["cotrimoxazole", "bactrim", "triméthoprime", "sulfaméthoxazole"],
        never: &[],
        label: "Cotrimoxazole",
        term: "Au premier trimestre : à éviter, l'effet antifolique portant sur la fermeture du tube neural. En fin de grossesse : contre-indiqué, du fait de l'ictère nucléaire chez le nouveau-né.",
        pregnancy: Level::Eviter,
        pregnancy_note: "Le terme décide, et dans les deux sens : ce n'est pas la même raison au début et à la fin. Une cystite de la grossesse a d'autres traitements.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Déconseillé chez le nouveau-né prématuré, ictérique ou déficitaire en G6PD.",
        source: "CRAT ; Bactrim : « À éviter au premier trimestre en raison de l'effet antifolique et contre-indiqué en fin de grossesse du fait du risque d'ictère nucléaire ».",
    },
    Advice {
        needs: &["amiodarone", "cordarone"],
        never: &[],
        label: "Amiodarone",
        pregnancy: Level::Interdit,
        term: "",
        pregnancy_note: "Sauf situation exceptionnelle : c'est la charge iodée qui est en cause, et elle donne une dysthyroïdie et un goitre fœtal. La décision appartient au cardiologue, pas au comptoir.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; Cordarone : « Contre-indiquée pendant la grossesse sauf situation exceptionnelle, du fait de la charge iodée et du risque de dysthyroïdie et de goitre fœtal ».",
    },
    Advice {
        needs: &["hydroxyzine", "atarax"],
        never: &[],
        label: "Hydroxyzine",
        pregnancy: Level::Interdit,
        term: "En fin de grossesse surtout : effets atropiniques et sédation chez le nouveau-né.",
        pregnancy_note: "Contre-indiqué par son RCP. Pour l'anxiété comme pour le prurit de la grossesse, il existe d'autres réponses — c'est une question de prescripteur, pas de conseil.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "L'hydroxyzine et son métabolite passent dans le lait et sédatent le nourrisson.",
        source: "CRAT ; Atarax : « Contre-indiqué pendant la grossesse selon le résumé des caractéristiques du produit, notamment en fin de grossesse en raison des effets atropiniques et sédatifs chez le nouveau-né ».",
    },
    Advice {
        needs: &["alendronate", "fosamax", "risédronate", "actonel", "acide zolédronique", "biphosphonate", "bisphosphonate"],
        never: &[],
        label: "Bisphosphonates",
        pregnancy: Level::Interdit,
        term: "",
        pregnancy_note: "Contre-indiqués. Ils se fixent sur l'os et s'en relarguent pendant des années : la question se pose donc aussi avant une grossesse, et c'est au prescripteur d'en juger.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqués.",
        source: "CRAT ; Fosamax : « Contre-indiqué pendant la grossesse et l'allaitement ».",
    },
    Advice {
        needs: &["clozapine", "leponex"],
        never: &[],
        label: "Clozapine",
        pregnancy: Level::Prudence,
        term: "",
        pregnancy_note: "Poursuivie quand le bénéfice l'emporte, et elle l'emporte souvent : arrêter un antipsychotique pendant une grossesse est un risque en soi. La glycémie maternelle se surveille, l'équipe obstétricale est prévenue d'un risque de syndrome extrapyramidal, de sevrage et d'agranulocytose néonatale, et le nouveau-né a une NFS.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; Leponex : « À n'utiliser pendant la grossesse que si le bénéfice est clairement supérieur au risque, en surveillant la glycémie maternelle […] avec NFS du nouveau-né ».",
    },
    // --- Ce qu'on peut rassurer ---------------------------------------
    //
    // **Avant le paracétamol, parce qu'une association n'est pas son
    // composant le plus rassurant.** La Lamaline porte du paracétamol,
    // mais aussi de la poudre d'opium : sa propre fiche écrit « à éviter
    // pendant la grossesse » et « contre-indiqué pendant l'allaitement en
    // raison du passage des opiacés dans le lait ». Annoncée
    // « compatible » parce qu'elle contient du paracétamol, elle disait
    // le contraire de sa fiche — et les deux textes étaient dans le
    // logiciel sans que rien ne les mette face à face.
    Advice {
        needs: &[
            // **Jamais « opium » seul** : « tiotr**opium** » et
            // « ipratr**opium** » le contiennent, et le Spiriva comme
            // l'Atrovent recevaient alors la ligne d'un antalgique
            // opiacé. Les deux spécialités concernées se nomment.
            "lamaline",
            "izalgi",
            "poudre d'opium",
        ],
        never: &[],
        label: "Paracétamol + opium",
        pregnancy: Level::Eviter,
        term: "En fin de grossesse, une utilisation prolongée expose le nouveau-né à un syndrome de sevrage et à une dépression respiratoire.",
        pregnancy_note: "C'est l'opium qui décide, pas le paracétamol. Si un antalgique est nécessaire, le paracétamol seul, à la dose efficace la plus faible et le moins longtemps possible.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué : les opiacés passent dans le lait. Le paracétamol seul est le premier choix.",
        source: "CRAT ; Lamaline : « À éviter pendant la grossesse […] Contre-indiqué pendant l'allaitement en raison du passage des opiacés dans le lait. »",
    },
    Advice {
        needs: &["paracetamol", "doliprane", "dafalgan", "efferalgan"],
        never: &[],
        label: "Paracétamol",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "L'antalgique et l'antipyrétique de la grossesse, à toute période. À la dose utile et le moins longtemps possible, comme pour tout le monde.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Premier choix.",
        source: "CRAT",
    },
    Advice {
        needs: &["amoxicilline", "clamoxyl", "augmentin", "penicilline"],
        never: &[],
        label: "Amoxicilline",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "Utilisable à toute période de la grossesse. C'est l'antibiotique le mieux documenté.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable ; une diarrhée du nourrisson est possible et bénigne.",
        source: "CRAT",
    },
    Advice {
        needs: &["levothyrox", "levothyroxine", "l-thyroxine", "euthyrox"],
        never: &[],
        label: "Lévothyroxine",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "À poursuivre sans interruption : c'est l'arrêt qui est dangereux. Le besoin augmente souvent dès le premier trimestre, et la TSH se contrôle tôt.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "À poursuivre.",
        source: "CRAT",
    },
    Advice {
        needs: &["insuline", "lantus", "novorapid", "humalog", "tresiba", "levemir"],
        never: &[],
        label: "Insuline",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "C'est le traitement du diabète de la grossesse. Les besoins changent beaucoup au fil des trimestres.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable ; surveiller les hypoglycémies maternelles, plus fréquentes pendant les tétées.",
        source: "CRAT",
    },
    // **Avant la metformine, et pour la même raison.** Le Xigduo est une
    // association metformine + dapagliflozine, et c'est la gliflozine qui
    // décide : sa fiche écrit « contre-indiqué ou déconseillé pendant la
    // grossesse selon les composants […] le relais par l'insuline
    // s'impose dès le projet de grossesse ».
    Advice {
        needs: &["dapagliflozine", "empagliflozine", "gliflozine", "xigduo", "forxiga", "jardiance"],
        never: &[],
        label: "Gliflozines (et leurs associations)",
        pregnancy: Level::Interdit,
        term: "La dapagliflozine ne doit pas être utilisée aux deuxième et troisième trimestres.",
        pregnancy_note: "Le relais par l'insuline s'impose dès le projet de grossesse ou dès sa découverte. Dans une association, c'est la gliflozine qui décide et non la metformine.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Allaitement déconseillé.",
        source: "CRAT ; Xigduo : « Contre-indiqué ou déconseillé pendant la grossesse selon les composants […] le relais par l'insuline s'impose dès le projet de grossesse ou dès sa découverte. Allaitement déconseillé. »",
    },
    Advice {
        needs: &["metformine", "glucophage", "stagid"],
        never: &[],
        label: "Metformine",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "Utilisable ; l'insuline reste le traitement de référence du diabète gestationnel en France.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable.",
        source: "CRAT",
    },
    Advice {
        needs: &[
            // **Jamais « corticoïde » seul**, pour une ligne intitulée
            // « par voie générale » : le mot est dans la classe de tous
            // les dermocorticoïdes, des corticoïdes inhalés, nasaux et
            // ophtalmiques — et jusque dans « antagoniste **non
            // stéroïdien** des récepteurs minéralo**corticoïde**s »,
            // c'est-à-dire le Kerendia, qui n'en est justement pas un.
            // Une crème et un collyre ne se lisent pas comme une
            // corticothérapie générale.
            "prednisone",
            "prednisolone",
            "cortancyl",
            "solupred",
            "célestène",
            "médrol",
            // La classe exacte de l'hydrocortisone substitutive, et
            // d'elle seule : « dermocorticoïde » ne la contient pas.
            "corticoïde substitutif",
        ],
        never: &[],
        label: "Corticoïdes par voie générale",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "Utilisables à toute période quand l'indication le demande. Une maladie inflammatoire non traitée est un risque pour la grossesse.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisables.",
        source: "CRAT",
    },
    Advice {
        needs: &["ondansetron", "zophren", "metoclopramide", "primperan", "doxylamine", "cariban"],
        never: &[],
        label: "Antiémétiques",
        pregnancy: Level::Prudence,
        term: "",
        pregnancy_note: "La doxylamine associée à la vitamine B6 est le premier choix des nausées de la grossesse. Le métoclopramide est utilisable. L'ondansétron l'est aussi si les autres échouent, malgré un signal faible sur les fentes labiales au premier trimestre.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Le métoclopramide est utilisable en cure courte.",
        source: "CRAT",
    },
    // **Avant la ligne de classe, parce que le membre dit autre chose
    // qu'elle.** La classe est compatible et l'oméprazole est le mieux
    // documenté ; le RCP du rabéprazole, lui, le contre-indique pendant
    // la grossesse et l'allaitement. Sa propre fiche l'écrit, et la
    // ligne de classe la contredisait en silence — c'est le genre de
    // désaccord qu'on ne voit qu'en confrontant la table aux fiches
    // livrées.
    Advice {
        needs: &["rabeprazole", "pariet"],
        never: &[],
        label: "Rabéprazole",
        pregnancy: Level::Eviter,
        term: "",
        pregnancy_note: "Son RCP le contre-indique pendant la grossesse, là où la classe est utilisable : l'oméprazole, mieux documenté, lui est préféré. Le reflux de la grossesse est fréquent et se traite.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Contre-indiqué par son RCP ; l'oméprazole lui est préféré.",
        source: "CRAT — les IPP sont utilisables et l'oméprazole est le mieux documenté ; RCP du rabéprazole, qui le contre-indique. Pariet : « Le résumé des caractéristiques du produit contre-indique le rabéprazole pendant la grossesse et l'allaitement. Lorsqu'un inhibiteur de la pompe à protons est réellement nécessaire chez la femme enceinte, l'oméprazole, mieux documenté, lui est préféré. »",
    },
    Advice {
        needs: &[
            // Et jamais « ipp » : « grippe » le contient, si bien que le
            // Tamiflu et le Relenza recevaient la ligne des IPP.
            "omeprazole",
            "esomeprazole",
            "pantoprazole",
            "lansoprazole",
            "inexium",
        ],
        never: &[],
        label: "Inhibiteurs de la pompe à protons",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "Utilisables ; l'oméprazole est le mieux documenté. Le reflux de la grossesse est fréquent et se traite.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisables.",
        source: "CRAT",
    },
    Advice {
        needs: &["sertraline", "fluoxetine", "paroxetine", "citalopram", "escitalopram", "venlafaxine"],
        never: &[],
        label: "Antidépresseurs (ISRS et IRSNA)",
        pregnancy: Level::Prudence,
        term: "En fin de grossesse : syndrome d'adaptation du nouveau-né, transitoire, à surveiller les premiers jours.",
        pregnancy_note: "Utilisables. L'arrêt brutal d'un antidépresseur pendant une grossesse est un risque en soi : une dépression non traitée pèse sur la grossesse et sur l'après. C'est une décision de prescripteur.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "La sertraline et la paroxétine passent très peu dans le lait : ce sont les molécules de l'allaitement.",
        source: "CRAT",
    },
];

#[cfg(test)]
mod tests {
    /// **Le cliquet : la table ne perd pas de lignes.**
    ///
    /// La règle de la maison pour tout catalogue clinique — une ligne
    /// retirée est une question à laquelle le comptoir ne sait plus
    /// répondre, et sans plancher cela arrive sans que personne le voie.
    /// Le nombre est écrit **une fois**, dans une constante que le
    /// message relit : écrit deux fois, en chiffres dans l'assertion et
    /// en lettres dans le message, il finit par se contredire — c'est
    /// arrivé dans `biology.rs`, dans `revue.rs` et dans le plancher de
    /// toxicité de `db.rs`.
    #[test]
    fn the_table_only_ever_grows() {
        const FLOOR: usize = 36;
        assert!(
            TABLE.len() >= FLOOR,
            "{} molécules de la table grossesse, il y en avait {FLOOR}",
            TABLE.len()
        );
    }

    use super::*;

    /// **Le libellé d'une molécule est son adresse, donc il est unique.**
    ///
    /// Deux molécules au même libellé feraient hériter la seconde de la
    /// réécriture de la première — sur un panneau qui dit ce qu'une
    /// femme enceinte peut prendre.
    #[test]
    fn a_molecule_label_is_an_address_and_no_two_share_one() {
        let mut ids: Vec<String> = TABLE
            .iter()
            .map(|a| crate::content::slug(a.label))
            .collect();
        assert!(ids.len() >= 20, "{} molécules", ids.len());
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "deux molécules partagent une adresse");
    }

    /// Toute phrase du panneau s'édite, et toute réécriture arrive.
    #[test]
    fn every_gravidity_phrase_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        let with_term = TABLE.iter().filter(|a| !a.term.is_empty()).count();
        assert_eq!(listed.len(), TABLE.len() * 2 + with_term);

        let treatments = [crate::revue::Treatment {
            name: "Codéine",
            dci: "codéine",
            class: "opioïde",
            tags: "",
        }];
        let found = read(&treatments);
        assert!(!found.is_empty());
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        for f in resolve(found.clone(), &over) {
            assert!(
                f.pregnancy_note.starts_with("réécrit:"),
                "{}",
                f.pregnancy_note
            );
            assert!(f.breastfeeding_note.starts_with("réécrit:"));
        }
        // Sans réécriture, les deux notes sont celles qui sont livrées.
        let plain = resolve(found.clone(), &crate::content::Overrides::default());
        assert_eq!(plain[0].pregnancy_note, found[0].pregnancy_note);
        assert_eq!(plain[0].breastfeeding_note, found[0].breastfeeding_note);
        assert_eq!(plain[0].worst(), found[0].worst(), "le niveau ne bouge pas");
    }

    fn treat(name: &str) -> crate::revue::Treatment<'_> {
        crate::revue::Treatment {
            name,
            dci: "",
            class: "",
            tags: "",
        }
    }

    /// **Ce que la table écrit est dessiné tel quel.** Le panneau la
    /// peint avec `RichText`, qui n'interprète aucun balisage : une
    /// astérisque écrite pour insister sort à l'écran comme une
    /// astérisque, et « **à partir de 24 SA** » se lit avec ses quatre
    /// étoiles. Trouvé sur une capture, corrigé ici pour de bon.
    /// **Aucune ligne « compatible » ne contredit la fiche qu'elle
    /// revendique.**
    ///
    /// Une ligne de classe parle de la classe, et un membre peut dire
    /// autre chose : le RCP du rabéprazole le contre-indique pendant la
    /// grossesse là où les IPP sont utilisables, et sa propre fiche
    /// l'écrit. La table l'annonçait « compatible » — les deux textes
    /// étaient dans le logiciel, personne ne les mettait face à face, et
    /// aucun test ne pouvait le voir puisque la table était cohérente
    /// avec elle-même.
    ///
    /// Le contrôle est volontairement étroit : il ne se déclenche que
    /// lorsque la ligne est compatible **des deux côtés** et que la
    /// fiche écrit une contre-indication. Une ligne compatible d'un côté
    /// et prudente de l'autre n'a rien à se reprocher, et un test qui
    /// crie au loup finit désactivé.
    ///
    /// Vérifié en retirant la ligne du rabéprazole : la ligne de classe
    /// reprend le Pariet, et le test le nomme.
    #[test]
    fn no_compatible_row_contradicts_the_card_it_claims() {
        let cards: Vec<(String, String)> = crate::db::STARTER_DETAILS
            .iter()
            .map(|d| (crate::fuzzy::sort_key(d.name), d.pregnancy.to_lowercase()))
            .collect();
        let mut wrong: Vec<String> = Vec::new();
        for (name, dci, class, tags) in crate::db::STARTER_DRUGS {
            let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}"));
            let Some(a) = TABLE.iter().find(|a| claims(a, &hay)) else {
                continue;
            };
            if a.pregnancy != Level::Compatible || a.breastfeeding != Level::Compatible {
                continue;
            }
            let Some((_, said)) = cards
                .iter()
                .find(|(n, _)| *n == crate::fuzzy::sort_key(name))
            else {
                continue;
            };
            if ["contre-indi", "tératog", "teratog", "abortif", "proscrit"]
                .iter()
                .any(|w| said.contains(w))
            {
                wrong.push(format!("{} → {name} : « {} »", a.label, said.trim()));
            }
        }
        assert!(
            wrong.is_empty(),
            "lignes annoncées compatibles que leur propre fiche contredit :\n{}",
            wrong.join("\n")
        );
    }

    #[test]
    fn the_table_writes_no_markup() {
        for a in TABLE {
            for text in [
                a.term,
                a.pregnancy_note,
                a.breastfeeding_note,
                a.label,
                a.source,
            ] {
                assert!(
                    !text.contains("**") && !text.contains("`"),
                    "{} : « {text} » porte du balisage",
                    a.label
                );
            }
        }
    }

    /// **« Pas de donnée » n'est pas « pas de risque ».**
    ///
    /// C'est la règle qui décide de tout le reste. Une molécule que la
    /// table ne connaît pas reçoit `SansDonnee` et non `Compatible`, et
    /// le type interdit de confondre les deux. La ligne dit où aller.
    #[test]
    fn no_data_is_never_read_as_no_risk() {
        let found = read(&[treat("Zoltruc 40 mg")]);
        assert_eq!(found.len(), 1, "toute ligne reçoit une réponse");
        assert_eq!(found[0].pregnancy, Level::SansDonnee);
        assert_ne!(found[0].pregnancy, Level::Compatible);
        assert_eq!(found[0].breastfeeding, Level::SansDonnee);
        assert!(found[0].pregnancy_note.contains("CRAT"));
        assert_eq!(found[0].source, "lecrat.fr");
        assert!(Level::SansDonnee.worrying(), "elle demande qu'on s'arrête");
    }

    /// **Grossesse et allaitement sont deux questions.** La codéine est
    /// l'exemple que tout le monde connaît, et l'écart y est complet :
    /// ponctuellement possible enceinte, contre-indiquée en allaitant.
    /// Une réponse unique serait fausse une fois sur deux.
    #[test]
    fn pregnancy_and_breastfeeding_are_two_questions() {
        let f = read(&[treat("Codoliprane")]).remove(0);
        assert_eq!(f.pregnancy, Level::Prudence);
        assert_eq!(f.breastfeeding, Level::Interdit);
        assert_ne!(f.pregnancy, f.breastfeeding);
        // Et l'inverse existe aussi : les AVK sont contre-indiqués
        // pendant la grossesse et compatibles avec l'allaitement.
        let avk = read(&[treat("Previscan")]).remove(0);
        assert_eq!(avk.pregnancy, Level::Interdit);
        assert_eq!(avk.breastfeeding, Level::Compatible);
        // Au moins un quart de la table répond différemment des deux
        // côtés : si ce n'était pas le cas, deux colonnes seraient une
        // colonne.
        let split = TABLE
            .iter()
            .filter(|a| a.pregnancy != a.breastfeeding)
            .count();
        assert!(
            split * 4 >= TABLE.len(),
            "{split} lignes sur {}",
            TABLE.len()
        );
    }

    /// **Le terme change la réponse.** Un AINS n'est pas « à éviter » :
    /// il est formellement contre-indiqué à partir de vingt-quatre
    /// semaines d'aménorrhée, et l'accident est décrit après une seule
    /// prise.
    #[test]
    fn the_term_is_written_where_it_decides() {
        let ains = read(&[treat("Ibuprofène 400")]).remove(0);
        assert_eq!(ains.pregnancy, Level::Interdit);
        assert!(ains.term.contains("24 semaines"));
        // L'aspirine dit l'autre moitié de la même histoire : c'est la
        // dose qui décide, et la même molécule est un traitement de la
        // grossesse à faible dose.
        let aspirine = read(&[treat("Kardégic 75")]).remove(0);
        assert!(aspirine.term.contains("antiagrégante"));
    }

    /// La table est une règle : des mots à chercher, un libellé, une
    /// note de chaque côté et une source qui cite le CRAT. Et une note
    /// vide ferait une ligne qui donne un verdict sans le justifier.
    #[test]
    fn every_row_cites_the_crat_and_says_why() {
        for a in TABLE {
            assert!(!a.needs.is_empty(), "{} sans mot à chercher", a.label);
            assert!(!a.label.trim().is_empty());
            assert!(
                a.source.contains("CRAT"),
                "{} : la référence est le CRAT",
                a.label
            );
            assert!(
                a.pregnancy_note.trim().len() > 30,
                "{} : la note de grossesse est trop courte",
                a.label
            );
            assert!(
                !a.breastfeeding_note.trim().is_empty(),
                "{} : rien sur l'allaitement",
                a.label
            );
            assert_ne!(
                a.pregnancy,
                Level::SansDonnee,
                "{} : hors de la table, la réponse est « sans donnée »",
                a.label
            );
            for n in a.needs {
                assert_eq!(*n, n.trim(), "{} : « {n} » a une espace en trop", a.label);
                assert_eq!(
                    *n,
                    n.to_lowercase(),
                    "{} : « {n} » n'est pas replié",
                    a.label
                );
            }
        }
    }

    /// **Les entrées les plus précises d'abord**, puisque la table est
    /// lue dans l'ordre et que la première qui accroche gagne. C'est
    /// une erreur qu'on ne voit pas en relisant.
    #[test]
    fn a_more_precise_row_never_sits_behind_a_broader_one() {
        for (i, a) in TABLE.iter().enumerate() {
            for n in a.needs {
                let folded = crate::fuzzy::sort_key(n);
                for earlier in TABLE.iter().take(i) {
                    for m in earlier.needs {
                        assert!(
                            !folded.contains(&crate::fuzzy::sort_key(m)),
                            "« {n} » ({}) est mangé par « {m} » ({})",
                            a.label,
                            earlier.label
                        );
                    }
                }
            }
        }
    }

    /// **Ce que le comptoir rencontre vraiment n'est plus « à vérifier
    /// au CRAT ».**
    ///
    /// Cent quatre-vingt-huit fiches livrées portent une
    /// contre-indication de grossesse que la table ignorait : elles
    /// tombaient toutes sur `SansDonnee`, qui est honnête et ne sert à
    /// rien devant une femme enceinte. Ces trois-là sont les plus
    /// fréquentes, et chacune porte un contresens que le panneau existe
    /// pour défaire.
    #[test]
    fn the_everyday_contraindications_are_named() {
        let one = |name: &str, dci: &str, class: &str| {
            read(&[crate::revue::Treatment {
                name,
                dci,
                class,
                tags: "",
            }])[0]
                .clone()
        };
        // Le contresens du diurétique : il est prescrit *contre* les
        // œdèmes de la grossesse, qui sont physiologiques.
        let lasilix = one("Lasilix", "furosémide", "diurétique de l'anse");
        assert_eq!(lasilix.label, "Diurétiques");
        assert_eq!(lasilix.pregnancy, Level::Eviter);
        assert!(lasilix.pregnancy_note.contains("œdèmes"));
        // Le sulfamide : c'est un relais vers l'insuline, pas un arrêt.
        let diamicron = one("Diamicron", "gliclazide", "sulfamide hypoglycémiant");
        assert_eq!(diamicron.pregnancy, Level::Interdit);
        assert!(diamicron.pregnancy_note.contains("insuline"));
        // Et le terme décide dans les deux sens pour le cotrimoxazole.
        let bactrim = one("Bactrim", "cotrimoxazole", "sulfamide antibactérien");
        assert!(bactrim.term.contains("premier trimestre"));
        assert!(bactrim.term.contains("fin de grossesse"));
    }

    /// L'ordre de lecture est celui du pire des deux niveaux : ce qui
    /// est interdit d'un côté ou de l'autre se lit d'abord.
    #[test]
    fn the_worst_of_the_two_decides_the_order() {
        let found = read(&[
            treat("Doliprane"),
            treat("Dépakine"),
            treat("Zoltruc"),
            treat("Tahor"),
        ]);
        let names: Vec<&str> = found.iter().map(|f| f.treatment.as_str()).collect();
        assert_eq!(names[0], "Dépakine", "l'interdit d'abord");
        assert_eq!(names.last(), Some(&"Zoltruc"), "l'inconnu en dernier");
        assert_eq!(found[0].worst(), Level::Interdit);
    }
    /// **Une forme locale ne porte pas le niveau de la voie générale.**
    ///
    /// La table est indexée sur la molécule, si bien qu'un collyre, une
    /// pommade ou un gel de la même molécule y tombe. Trois le
    /// faisaient : l'Exocine, collyre à l'ofloxacine, lisait la prudence
    /// des fluoroquinolones ; le Lithioderm, gel pour la dermite
    /// séborrhéique, lisait celle du lithium ; l'Auréomycine Evans,
    /// pommade, lisait l'interdit des cyclines. Les trois fiches
    /// écrivent l'inverse, et par contraste avec la voie générale —
    /// « contrairement au lithium administré par voie générale ».
    ///
    /// Le repère est la **classe** et non la prose : « passage
    /// systémique négligeable » se trouve aussi dans des fiches d'ISRS
    /// que « prudence » qualifie très bien. Deux exceptions, chacune
    /// avec sa raison, et ce sont les deux seules que la base livrée
    /// présente.
    #[test]
    fn a_local_form_does_not_wear_the_systemic_level() {
        // « sous-cutané » et « gel intestinal » ne sont pas des formes
        // locales : l'héparine et le Duodopa passent bien dans le sang.
        // « percutané » non plus — un gel d'estradiol est un estrogène
        // général. Le vocabulaire est donc étroit.
        const LOCAL: &[&str] = &[
            "collyre",
            "topique",
            "nasal",
            "auriculaire",
            "dermocorticoide",
            "pommade ophtalmique",
            "gel ophtalmique",
            "ovule",
            "lotion",
            "shampoing",
        ];
        // Une ligne écrite **pour** les formes locales : les
        // vasoconstricteurs du rhume sont nasaux, et c'est justement
        // par cette voie qu'ils sont contre-indiqués à tout terme.
        const ABOUT_LOCAL_FORMS: &[&str] = &["Vasoconstricteurs du rhume"];
        // Et une boîte dont la fiche est elle-même prudente : le
        // Sterdex est « déconseillé à partir du deuxième trimestre […]
        // même si l'exposition par voie locale est très faible ».
        const ITS_OWN_CARD_IS_CAUTIOUS: &[&str] = &["Sterdex"];

        let mut wrong: Vec<String> = Vec::new();
        for (name, dci, class, tags) in crate::db::STARTER_DRUGS {
            let folded_class = crate::fuzzy::sort_key(class);
            if !LOCAL
                .iter()
                .any(|v| crate::fuzzy::contains_folded(&folded_class, v))
            {
                continue;
            }
            if ITS_OWN_CARD_IS_CAUTIOUS.contains(name) {
                continue;
            }
            let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}"));
            let Some(a) = TABLE.iter().find(|a| claims(a, &hay)) else {
                continue;
            };
            if ABOUT_LOCAL_FORMS.contains(&a.label) {
                continue;
            }
            // **Tout ce qui n'est pas « compatible »**, et non
            // `worrying()` : celui-ci laisse passer « prudence », qui
            // est justement le niveau que lisait l'Exocine. La morsure
            // l'a montré — écrite avec `worrying()`, elle ne mordait
            // pas.
            if a.pregnancy != Level::Compatible || a.breastfeeding != Level::Compatible {
                wrong.push(format!(
                    "« {name} » ({class}) lit « {} », écrite pour la voie générale",
                    a.label
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "une forme locale porte le niveau de la voie générale :\n{}",
            wrong.join("\n")
        );
    }
}
