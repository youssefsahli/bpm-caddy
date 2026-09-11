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
//! Cinq règles le tiennent, une par test :
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
pub fn read(treatments: &[crate::revue::Treatment]) -> Vec<Finding> {
    let mut out: Vec<Finding> = treatments
        .iter()
        .map(|t| {
            let hay =
                crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags));
            let hit = TABLE.iter().find(|a| {
                a.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            });
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
        label: "Rétinoïdes oraux",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse, et un mois après l'arrêt pour l'isotrétinoïne, deux ans pour l'acitrétine.",
        pregnancy_note: "Tératogène majeur. Contraception efficace et test de grossesse mensuel, ordonnance de sept jours pour l'isotrétinoïne : c'est le programme de prévention des grossesses.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; ANSM, programme de prévention des grossesses",
    },
    Advice {
        needs: &["methotrexate", "novatrex", "imeth"],
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
        label: "Mycophénolate",
        pregnancy: Level::Interdit,
        term: "Sur toute la grossesse.",
        pregnancy_note: "Tératogène : malformations de la face et des oreilles. Contraception obligatoire, programme de prévention des grossesses.",
        breastfeeding: Level::Interdit,
        breastfeeding_note: "Contre-indiqué.",
        source: "CRAT ; ANSM",
    },
    // --- Ce que le terme change ---------------------------------------
    Advice {
        needs: &["ains", "ibuprofene", "diclofenac", "ketoprofene", "naproxene", "celecoxib", "advil", "nurofen"],
        label: "AINS",
        pregnancy: Level::Interdit,
        term: "À partir de 24 semaines d'aménorrhée (5 mois et demi) : contre-indication formelle, même en prise unique. Avant ce terme : à éviter.",
        pregnancy_note: "Après 24 SA, ils ferment le canal artériel du fœtus et atteignent son rein — l'accident est décrit après une seule prise. Le paracétamol est l'antalgique de la grossesse.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "L'ibuprofène passe très peu dans le lait : c'est l'AINS de l'allaitement.",
        source: "CRAT ; ANSM, alerte AINS et grossesse",
    },
    Advice {
        needs: &["aspirine", "acide acetylsalicylique", "kardegic", "aspegic"],
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
        label: "Codéine et tramadol",
        pregnancy: Level::Prudence,
        term: "En fin de grossesse : syndrome de sevrage et dépression respiratoire du nouveau-né si la prise est prolongée ou proche de l'accouchement.",
        pregnancy_note: "Ponctuellement, l'usage est possible. Le paracétamol reste le premier choix.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "La codéine est déconseillée : une mère métaboliseuse ultrarapide transforme trop de codéine en morphine, et des dépressions respiratoires du nourrisson ont été décrites. Préférer le paracétamol ou l'ibuprofène.",
        source: "CRAT ; ANSM",
    },
    Advice {
        needs: &["nitrofurantoine", "furadantine"],
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
        needs: &["atorvastatine", "simvastatine", "rosuvastatine", "pravastatine", "statine", "tahor", "crestor"],
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
        label: "Lithium",
        pregnancy: Level::Prudence,
        term: "Au premier trimestre : légère augmentation du risque de malformation cardiaque. À l'accouchement : imprégnation du nouveau-né.",
        pregnancy_note: "L'arrêt d'un thymorégulateur pendant une grossesse est un risque en soi. C'est une décision de psychiatre, jamais une décision de comptoir. Lithiémie rapprochée.",
        breastfeeding: Level::Eviter,
        breastfeeding_note: "Passage important dans le lait.",
        source: "CRAT",
    },
    // --- Ce qu'on peut rassurer ---------------------------------------
    Advice {
        needs: &["paracetamol", "doliprane", "dafalgan", "efferalgan"],
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
        label: "Insuline",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "C'est le traitement du diabète de la grossesse. Les besoins changent beaucoup au fil des trimestres.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable ; surveiller les hypoglycémies maternelles, plus fréquentes pendant les tétées.",
        source: "CRAT",
    },
    Advice {
        needs: &["metformine", "glucophage", "stagid"],
        label: "Metformine",
        pregnancy: Level::Compatible,
        term: "",
        pregnancy_note: "Utilisable ; l'insuline reste le traitement de référence du diabète gestationnel en France.",
        breastfeeding: Level::Compatible,
        breastfeeding_note: "Utilisable.",
        source: "CRAT",
    },
    Advice {
        needs: &["prednisone", "prednisolone", "cortancyl", "solupred", "corticoide"],
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
        label: "Antiémétiques",
        pregnancy: Level::Prudence,
        term: "",
        pregnancy_note: "La doxylamine associée à la vitamine B6 est le premier choix des nausées de la grossesse. Le métoclopramide est utilisable. L'ondansétron l'est aussi si les autres échouent, malgré un signal faible sur les fentes labiales au premier trimestre.",
        breastfeeding: Level::Prudence,
        breastfeeding_note: "Le métoclopramide est utilisable en cure courte.",
        source: "CRAT",
    },
    Advice {
        needs: &["omeprazole", "esomeprazole", "pantoprazole", "lansoprazole", "inexium", "ipp"],
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
    /// l'exemple que tout le monde connaît : ponctuellement possible
    /// enceinte, déconseillée en allaitant. Une réponse unique serait
    /// fausse une fois sur deux.
    #[test]
    fn pregnancy_and_breastfeeding_are_two_questions() {
        let f = read(&[treat("Codoliprane")]).remove(0);
        assert_eq!(f.pregnancy, Level::Prudence);
        assert_eq!(f.breastfeeding, Level::Eviter);
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
}
