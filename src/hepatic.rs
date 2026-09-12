//! Ce que la fonction hépatique fait à une ordonnance.
//!
//! `renal.rs` répond à « ce dossier porte un DFG à 28 ; que devient
//! chaque ligne ? ». La même question se pose pour le foie et n'avait
//! pas de réponse : les fiches la portent — deux cents d'entre elles
//! nomment le foie dans leurs contre-indications — et il fallait les
//! ouvrir une par une.
//!
//! ## Ce qui sépare ce module de son voisin rénal
//!
//! **Le foie n'a pas de DFG.** Le rein donne un chiffre que l'officine
//! lit sur un compte rendu ; le foie donne un **stade** qu'un clinicien
//! attribue, à partir de cinq éléments dont deux — l'ascite et
//! l'encéphalopathie — ne sont pas des valeurs de laboratoire. Ce
//! module prend donc un stade et jamais une valeur : demander un
//! chiffre inviterait à en inventer un, et une table qui calculerait un
//! Child-Pugh à partir de ce qu'une pharmacie peut voir rendrait un
//! score faux avec l'aplomb d'un vrai.
//!
//! **Une hépatopathie évolutive n'est pas un stade.** Les statines, le
//! léflunomide, l'agomélatine sont contre-indiqués en cas d'« affection
//! hépatique évolutive » — c'est-à-dire une maladie en cours, quel que
//! soit le Child-Pugh. Les ranger sous un palier dirait la chose à un
//! stade et la tairait aux autres, ce qui est faux des deux côtés. Ils
//! ne sont pas dans cette table, et leur fiche le dit là où c'est vrai.
//!
//! ## Ce que le module tient
//!
//! Cinq règles, une par test :
//!
//! * **Sans stade, pas de verdict.** Le module nomme alors ce qui
//!   dépend du foie et dit que le stade manque — jamais ce qu'il
//!   faudrait faire. C'est la règle de `renal.rs`, et celle de l'écart
//!   de caisse sans recette attendue.
//! * **Le palier atteint est le plus grave franchi.** Un produit qui se
//!   réduit dès le stade A et se contre-indique au stade C, lu en C, est
//!   contre-indiqué : prendre le premier palier de la liste dirait
//!   « réduire la dose » d'un traitement qu'il faut arrêter.
//! * **« Rien à changer » est une réponse.** Une ligne connue de la
//!   table et qu'aucun palier n'atteint à ce stade-là le **dit**, au
//!   lieu de disparaître. L'oxazépam est le cas qui justifie la règle à
//!   lui seul : sa fiche écrit « aucune adaptation n'est nécessaire du
//!   fait de l'insuffisance hépatique légère à modérée », et c'est
//!   précisément la benzodiazépine qu'on cherche chez un cirrhotique.
//!   Une liste qui la tairait la rendrait aussi muette qu'un produit
//!   dont personne n'a rien écrit.
//! * **Un palier vient du RCP, jamais d'une interpolation.** Le module
//!   ne calcule pas une dose à partir d'un stade : il répète ce que la
//!   fiche écrit.
//! * **La conduite est celle du RCP, la décision est celle du
//!   prescripteur.** Beaucoup de ces contre-indications ne tiennent pas
//!   au métabolisme mais au **risque d'encéphalopathie** — les
//!   benzodiazépines, les diurétiques de l'anse — et une dose réduite
//!   n'y change rien.
//!
//! Statique, pur et testé, comme `renal`. Il ne connaît ni la base ni
//! egui : on lui passe des traitements et un stade.

/// Le stade de l'insuffisance hépatique, **et non un chiffre**.
///
/// Les trois classes de Child-Pugh, sous les mots que les fiches
/// emploient : légère (A), modérée (B), sévère (C). L'ordre est celui
/// de la gravité, et c'est lui qui décide du palier atteint.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Stage {
    /// Child-Pugh A.
    Mild,
    /// Child-Pugh B.
    Moderate,
    /// Child-Pugh C.
    Severe,
}

impl Stage {
    pub const ALL: &'static [Stage] = &[Stage::Mild, Stage::Moderate, Stage::Severe];

    pub fn label(self) -> &'static str {
        match self {
            Stage::Mild => "Insuffisance hépatique légère (Child-Pugh A)",
            Stage::Moderate => "Insuffisance hépatique modérée (Child-Pugh B)",
            Stage::Severe => "Insuffisance hépatique sévère (Child-Pugh C)",
        }
    }

    /// Le mot court, pour un bouton.
    pub fn short(self) -> &'static str {
        match self {
            Stage::Mild => "Légère",
            Stage::Moderate => "Modérée",
            Stage::Severe => "Sévère",
        }
    }

    /// La forme qui tient dans une phrase, derrière « dès ».
    ///
    /// `label` s'accorde avec « insuffisance » et ne se laisse pas
    /// enchâsser : « à réduire dès légère » ne se lit pas. Trois formes
    /// pour trois places, comme `richest_form` en donne plusieurs à une
    /// même chose.
    pub fn gradation(self) -> &'static str {
        match self {
            Stage::Mild => "le stade léger (Child-Pugh A)",
            Stage::Moderate => "le stade modéré (Child-Pugh B)",
            Stage::Severe => "le stade sévère (Child-Pugh C)",
        }
    }
}

/// Ce qu'un palier demande. L'ordre est celui de la gravité.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    /// Le traitement ne doit pas être poursuivi à ce stade.
    Contraindicated,
    /// La dose se réduit, selon ce que dit la conduite.
    Reduce,
    /// Ni l'un ni l'autre : à surveiller de plus près.
    Watch,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Contraindicated => "Contre-indiqué",
            Level::Reduce => "Dose à réduire",
            Level::Watch => "À surveiller",
        }
    }
}

/// Un palier : **à partir de ce stade**, le traitement devient ceci.
///
/// « À partir de », et non « en dessous de » comme au rein : le rein se
/// dégrade quand le chiffre baisse, le foie quand le stade monte. Les
/// deux modules lisent donc leur table dans des sens opposés, et c'est
/// la seule raison pour laquelle ils ne partagent pas leur type.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub from: Stage,
    pub level: Level,
    /// Ce que le RCP dit, en une phrase de comptoir.
    pub conduct: &'static str,
}

/// Une molécule et ce que le foie lui fait.
pub struct Adaptation {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes —
    /// repliés par `fuzzy::sort_key`, comme partout ici.
    pub needs: &'static [&'static str],
    pub label: &'static str,
    /// Du stade le plus léger au plus grave, pour la lecture ; le
    /// calcul, lui, ne s'y fie pas (voir [`read`]).
    pub steps: &'static [Step],
    /// D'où vient le palier.
    pub source: &'static str,
}

/// Ce que le module conclut d'une ligne.
///
/// **Trois réponses et non deux.** « On ne sait pas », « on sait, et il
/// n'y a rien à changer » et « voici ce qu'il faut faire » sont trois
/// choses différentes, et les deux premières se ressemblent sur un écran
/// qui les tairait toutes les deux.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// Aucun stade au dossier : la ligne dépend du foie, et le module ne
    /// conclut pas.
    Unknown,
    /// À ce stade-là, la fiche ne demande rien.
    Nothing,
    /// Ce que la fiche demande.
    Adapt(Level),
}

impl Verdict {
    /// L'ordre de lecture : ce qu'il faut arrêter d'abord, ce qui ne
    /// demande rien en dernier.
    fn rank(self) -> u8 {
        match self {
            Verdict::Adapt(Level::Contraindicated) => 0,
            Verdict::Adapt(Level::Reduce) => 1,
            Verdict::Adapt(Level::Watch) => 2,
            Verdict::Unknown => 3,
            Verdict::Nothing => 4,
        }
    }
}

/// Ce que le module rend pour une ligne d'ordonnance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    /// Le nom tel qu'il est écrit au dossier.
    pub treatment: String,
    pub label: &'static str,
    pub verdict: Verdict,
    /// Le stade du palier retenu, quand il y en a un.
    pub from: Option<Stage>,
    pub conduct: &'static str,
    pub source: &'static str,
}

/// Ce que le foie fait à cette ordonnance, à ce stade.
///
/// `stage` à `None` — aucun stade au dossier — n'est **pas** le stade
/// léger : le module rend alors les traitements concernés sans verdict,
/// et c'est à la vue de dire que le stade manque.
pub fn read(treatments: &[crate::revue::Treatment], stage: Option<Stage>) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for t in treatments {
        let hay = crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags));
        for a in TABLE {
            if !a
                .needs
                .iter()
                .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            {
                continue;
            }
            // **Le palier atteint est le plus grave franchi.** Pas le
            // premier de la liste : un produit qui se réduit dès le
            // stade A et se contre-indique au stade C, lu en C, est
            // contre-indiqué.
            let hit = stage.and_then(|s| {
                a.steps
                    .iter()
                    .filter(|p| p.from <= s)
                    .max_by_key(|p| p.from)
            });
            let verdict = match (stage, hit) {
                (None, _) => Verdict::Unknown,
                (Some(_), None) => Verdict::Nothing,
                (Some(_), Some(p)) => Verdict::Adapt(p.level),
            };
            out.push(Finding {
                treatment: t.name.trim().to_owned(),
                label: a.label,
                verdict,
                from: hit.map(|p| p.from),
                conduct: match verdict {
                    Verdict::Unknown => {
                        "Ce traitement s'adapte à la fonction hépatique ; aucun stade n'est noté au dossier."
                    }
                    Verdict::Nothing => {
                        "À ce stade, la fiche ne demande pas d'adaptation."
                    }
                    Verdict::Adapt(_) => hit.map_or("", |p| p.conduct),
                },
                source: a.source,
            });
            break;
        }
    }
    out.sort_by(|a, b| {
        a.verdict
            .rank()
            .cmp(&b.verdict.rank())
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

/// Combien de lignes attendent un stade qu'on n'a pas.
///
/// Le même service que `renal::undecided` : « aucun stade » tout seul
/// est une remarque, « aucun stade, et six lignes en dépendent » est une
/// question à poser au prescripteur.
pub fn pending(findings: &[Finding]) -> usize {
    findings
        .iter()
        .filter(|f| f.verdict == Verdict::Unknown)
        .count()
}

use Level::{Contraindicated, Reduce, Watch};
use Stage::{Mild, Moderate, Severe};

const fn step(from: Stage, level: Level, conduct: &'static str) -> Step {
    Step {
        from,
        level,
        conduct,
    }
}

/// Ce que le foie change, molécule par molécule.
///
/// **Rien ici n'est inventé** : chaque palier est celui de la fiche
/// livrée, citée. Les molécules retenues sont celles dont la conduite
/// est nette et l'usage courant au comptoir — c'est la règle de
/// `renal.rs`, et une fiche qui dit « prudence » sans dire quoi faire
/// n'y est pas.
///
/// Les paliers sont écrits du plus léger au plus grave, pour la lecture.
pub const TABLE: &[Adaptation] = &[
    Adaptation {
        needs: &["paracetamol"],
        label: "Paracétamol",
        steps: &[
            step(
                Mild,
                Reduce,
                "Ne pas dépasser 3 g par jour, et espacer les prises d'au moins six heures.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatocellulaire sévère : contre-indiqué."),
        ],
        source: "Doliprane : « Sujet âgé, poids inférieur à 50 kg, dénutrition, alcoolisme chronique ou insuffisance hépatocellulaire : ne pas dépasser 3 g et espacer les prises d'au moins 6 heures » ; contre-indication : « insuffisance hépatocellulaire sévère ».",
    },
    // **La benzodiazépine du cirrhotique**, et la seule ligne de cette
    // table dont l'intérêt est de ne rien demander.
    Adaptation {
        needs: &["oxazepam"],
        label: "Oxazépam",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère avec risque d'encéphalopathie : contre-indiqué.",
        )],
        source: "Séresta : « aucune adaptation n'est nécessaire du fait de l'insuffisance hépatique légère à modérée » — il n'est pas oxydé par les cytochromes ; contre-indication : « insuffisance hépatique sévère avec risque d'encéphalopathie ».",
    },
    Adaptation {
        needs: &["alprazolam"],
        label: "Alprazolam",
        steps: &[
            step(
                Mild,
                Reduce,
                "Débuter à 0,25 mg une à deux fois par jour et ne pas dépasser la moitié de la dose adulte.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère avec risque d'encéphalopathie : contre-indiqué."),
        ],
        source: "Xanax : « Chez le sujet âgé, l'insuffisant hépatique ou respiratoire, débuter à 0,25 mg une à deux fois par jour et ne pas dépasser la moitié de la dose adulte ».",
    },
    Adaptation {
        needs: &["bromazepam"],
        label: "Bromazépam",
        steps: &[
            step(Mild, Reduce, "Dose réduite de moitié."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère avec risque d'encéphalopathie : contre-indiqué."),
        ],
        source: "Lexomil : « Chez le sujet âgé, l'insuffisant hépatique ou l'insuffisant respiratoire, la dose est réduite de moitié ».",
    },
    Adaptation {
        needs: &["diazepam"],
        label: "Diazépam",
        steps: &[
            step(
                Mild,
                Reduce,
                "Réduire d'au moins la moitié : les métabolites actifs s'accumulent.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Valium : « Chez le sujet âgé, l'insuffisant hépatique ou l'insuffisant rénal, la posologie est réduite de moitié au moins, en raison de l'accumulation des métabolites actifs ».",
    },
    Adaptation {
        needs: &["lorazepam"],
        label: "Lorazépam",
        steps: &[
            step(Mild, Reduce, "Dose initiale réduite de moitié."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Temesta : « Chez le sujet âgé et l'insuffisant hépatique, la dose initiale est réduite de moitié ».",
    },
    Adaptation {
        needs: &["zolpidem"],
        label: "Zolpidem",
        steps: &[
            step(Mild, Reduce, "5 mg par jour, à ne pas dépasser."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Stilnox : « Chez le sujet de plus de 65 ans, l'insuffisant hépatique ou le patient fragile, la posologie est de 5 mg par jour et ne doit pas être dépassée ».",
    },
    Adaptation {
        needs: &["zopiclone"],
        label: "Zopiclone",
        steps: &[
            step(Mild, Reduce, "3,75 mg, soit un demi-comprimé."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Imovane : « Chez le sujet de plus de 65 ans, l'insuffisant hépatique, l'insuffisant rénal ou l'insuffisant respiratoire chronique, la posologie est de 3,75 mg, soit un demi-comprimé ».",
    },
    Adaptation {
        needs: &["colchicine"],
        label: "Colchicine",
        steps: &[
            step(
                Mild,
                Reduce,
                "Réduire de moitié et surveiller étroitement : la marge thérapeutique est très étroite.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Colchicine : « Chez le sujet âgé, l'insuffisant rénal ou l'insuffisant hépatique, réduire les doses de moitié et surveiller étroitement, la marge thérapeutique étant très étroite ».",
    },
    Adaptation {
        needs: &["celecoxib"],
        label: "Célécoxib",
        steps: &[
            step(Moderate, Reduce, "Débuter à la moitié de la dose habituelle."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Celebrex : « Chez le sujet âgé de moins de 50 kg et en cas d'insuffisance hépatique modérée, débuter à la moitié de la dose habituelle ».",
    },
    Adaptation {
        needs: &["ibuprofene"],
        label: "Ibuprofène",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Advil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["ketoprofene"],
        label: "Kétoprofène",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Bi-Profénid : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["tramadol"],
        label: "Tramadol",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Tramadol : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["oxycodone"],
        label: "Oxycodone",
        steps: &[
            step(Mild, Reduce, "Débuter plus bas et espacer les prises."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Oxycontin : « chez le sujet âgé, fragile ou insuffisant rénal ou hépatique, débuter plus bas et espacer » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["morphine"],
        label: "Morphine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatocellulaire sévère : contre-indiqué.")],
        source: "Skenan : contre-indication en « insuffisance hépatocellulaire sévère ».",
    },
    Adaptation {
        needs: &["domperidone"],
        label: "Dompéridone",
        steps: &[
            step(Mild, Watch, "La dose la plus faible, et pas plus longtemps qu'il ne faut."),
            step(
                Moderate,
                Contraindicated,
                "Insuffisance hépatique modérée à sévère : contre-indiqué, le risque de torsades étant majoré.",
            ),
        ],
        source: "Motilium : contre-indication en « insuffisance hépatique modérée à sévère » ; « Chez le sujet âgé et l'insuffisant hépatique léger, la prudence conduit à la dose la plus faible ».",
    },
    Adaptation {
        needs: &["ondansetron"],
        label: "Ondansétron",
        steps: &[step(
            Moderate,
            Reduce,
            "Ne pas dépasser 8 mg par jour : la clairance est fortement réduite.",
        )],
        source: "Zophren : « En insuffisance hépatique modérée à sévère, ne pas dépasser 8 mg par jour, la clairance étant fortement réduite ».",
    },
    Adaptation {
        needs: &["loperamide"],
        label: "Lopéramide",
        steps: &[
            step(
                Mild,
                Watch,
                "L'effet de premier passage est réduit : passage central possible.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Imodium : « l'insuffisance hépatique réduit l'effet de premier passage et expose au passage central : la prudence s'impose et la forme sévère est une contre-indication ».",
    },
    Adaptation {
        needs: &["ezetimibe"],
        label: "Ézétimibe",
        steps: &[step(
            Moderate,
            Contraindicated,
            "Contre-indiqué en association à une statine à partir du stade modéré.",
        )],
        source: "Ezetrol : « Pas d'adaptation en cas d'insuffisance hépatique légère » ; contre-indication en « insuffisance hépatique modérée à sévère lorsque l'ézétimibe est associé à une statine ».",
    },
    Adaptation {
        needs: &["metformine"],
        label: "Metformine",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère : contre-indiqué, le risque d'acidose lactique étant majoré.",
        )],
        source: "Glucophage : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["gliclazide"],
        label: "Gliclazide",
        steps: &[
            step(Mild, Reduce, "Réduire la dose et surveiller les hypoglycémies."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Diamicron : « Réduire la dose en cas d'insuffisance rénale légère à modérée, d'insuffisance hépatique, de dénutrition » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["glimepiride"],
        label: "Glimépiride",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Amarel : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["repaglinide"],
        label: "Répaglinide",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Novonorm : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["esomeprazole"],
        label: "Ésoméprazole",
        steps: &[step(Severe, Reduce, "Ne pas dépasser 20 mg par jour.")],
        source: "Inexium : « En cas d'insuffisance hépatique sévère, la dose ne doit pas dépasser 20 mg par jour ».",
    },
    Adaptation {
        needs: &["losartan"],
        label: "Losartan",
        steps: &[
            step(Mild, Reduce, "Débuter à 25 mg."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Cozaar : « Débuter à 25 mg chez le sujet de plus de 75 ans, en cas de déplétion volémique, de traitement diurétique à forte dose ou d'insuffisance hépatique » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["amlodipine"],
        label: "Amlodipine",
        steps: &[step(Mild, Reduce, "Débuter à 2,5 mg et titrer lentement ; contrôler les transaminases.")],
        source: "Amlor : « En cas d'insuffisance hépatique, débuter à 2,5 mg et titrer lentement » ; « Chez l'insuffisant hépatique, contrôler les transaminases ».",
    },
    Adaptation {
        needs: &["verapamil"],
        label: "Vérapamil",
        steps: &[step(
            Mild,
            Reduce,
            "Débuter au tiers ou à la moitié de la dose usuelle : l'exposition peut être doublée ou triplée.",
        )],
        source: "Isoptine : « Réduire la posologie chez le sujet âgé et en cas d'insuffisance hépatique, où l'exposition peut être doublée ou triplée : débuter à environ un tiers ou la moitié de la dose usuelle ».",
    },
    Adaptation {
        needs: &["propranolol"],
        label: "Propranolol",
        steps: &[step(Mild, Reduce, "Réduire la dose.")],
        source: "Avlocardyl : « Réduire la dose chez le sujet âgé et en cas d'insuffisance hépatique ».",
    },
    Adaptation {
        needs: &["flecainide"],
        label: "Flécaïnide",
        steps: &[step(Mild, Reduce, "Réduire la dose.")],
        source: "Flécaïne : « Réduire la dose chez le sujet de plus de 70 ans, en cas d'insuffisance rénale et en cas d'insuffisance hépatique ».",
    },
    // **Ce n'est pas le métabolisme qui contre-indique, c'est
    // l'encéphalopathie.** Une dose réduite n'y change rien, et c'est
    // pourquoi le palier est une contre-indication et non une réduction.
    Adaptation {
        needs: &["furosemide"],
        label: "Furosémide",
        steps: &[step(
            Severe,
            Contraindicated,
            "Encéphalopathie hépatique et insuffisance hépatique sévère : contre-indiqué.",
        )],
        source: "Lasilix : contre-indication en « encéphalopathie hépatique et insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["hydrochlorothiazide"],
        label: "Hydrochlorothiazide",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère et encéphalopathie hépatique : contre-indiqué.",
        )],
        source: "Esidrex : contre-indication en « insuffisance hépatique sévère et encéphalopathie hépatique ».",
    },
    Adaptation {
        needs: &["indapamide"],
        label: "Indapamide",
        steps: &[step(
            Severe,
            Contraindicated,
            "Encéphalopathie hépatique et insuffisance hépatique sévère : contre-indiqué.",
        )],
        source: "Fludex : contre-indication en « encéphalopathie hépatique et insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["warfarine"],
        label: "Warfarine",
        steps: &[
            step(Mild, Reduce, "Dose initiale réduite, sans dose de charge, et INR rapproché."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Coumadine : « Dose initiale usuelle de 5 mg par jour chez l'adulte, réduite à 4 mg chez le sujet âgé, de faible poids ou insuffisant hépatique, sans dose de charge » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["acenocoumarol"],
        label: "Acénocoumarol",
        steps: &[
            step(Mild, Reduce, "Dose initiale réduite, sans dose de charge, et INR rapproché."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Sintrom : « Dose initiale habituelle de 4 mg par jour chez l'adulte, réduite chez le sujet âgé, de faible poids ou insuffisant hépatique, sans dose de charge » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["fluindione"],
        label: "Fluindione",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Previscan : contre-indication en « insuffisance hépatique sévère ».",
    },
    // **Le stade est écrit dans la fiche**, en toutes lettres : Child B
    // et C. C'est le seul produit de cette table dont la
    // contre-indication nomme la classification.
    Adaptation {
        needs: &["rivaroxaban"],
        label: "Rivaroxaban",
        steps: &[step(
            Moderate,
            Contraindicated,
            "Coagulopathie hépatique, Child B et C : contre-indiqué.",
        )],
        source: "Xarelto : contre-indication en « coagulopathie hépatique (Child B et C) ».",
    },
    Adaptation {
        needs: &["dabigatran"],
        label: "Dabigatran",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Pradaxa : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["apixaban"],
        label: "Apixaban",
        steps: &[step(
            Severe,
            Contraindicated,
            "Hépatopathie avec coagulopathie : contre-indiqué.",
        )],
        source: "Eliquis : contre-indication en « hépatopathie avec coagulopathie ».",
    },
    Adaptation {
        needs: &["ticagrelor"],
        label: "Ticagrélor",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Brilique : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["clopidogrel"],
        label: "Clopidogrel",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Plavix : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["fluoxetine"],
        label: "Fluoxétine",
        steps: &[step(
            Mild,
            Reduce,
            "Ne pas dépasser 40 mg par jour et augmenter plus lentement.",
        )],
        source: "Prozac : « Chez le sujet âgé, l'insuffisant hépatique ou le patient polymédiqué, ne pas dépasser 40 mg par jour et augmenter plus lentement ».",
    },
    Adaptation {
        needs: &["sertraline"],
        label: "Sertraline",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Zoloft : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["duloxetine"],
        label: "Duloxétine",
        steps: &[step(
            Mild,
            Contraindicated,
            "Insuffisance hépatique ou hépatopathie évolutive : contre-indiqué, sans distinction de stade.",
        )],
        source: "Cymbalta : contre-indication en « insuffisance hépatique ou hépatopathie évolutive » — la fiche ne distingue pas les stades.",
    },
    Adaptation {
        needs: &["alfuzosine"],
        label: "Alfuzosine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Xatral : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["solifenacine"],
        label: "Solifénacine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Vesicare : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["dronedarone"],
        label: "Dronédarone",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Multaq : contre-indication en « insuffisance hépatique sévère ».",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn t<'a>(name: &'a str, dci: &'a str) -> crate::revue::Treatment<'a> {
        crate::revue::Treatment {
            name,
            dci,
            class: "",
            tags: "",
        }
    }

    /// **Sans stade, pas de verdict.**
    ///
    /// Le module nomme ce qui dépend du foie et dit que le stade
    /// manque ; il ne dit jamais ce qu'il faudrait faire. Un logiciel
    /// qui annoncerait « contre-indiqué » sans savoir où en est le foie
    /// dirait une chose qu'il ne sait pas — c'est la règle de
    /// `renal.rs`, et le type est ce qui l'empêche de la trahir.
    #[test]
    fn without_a_stage_there_is_no_verdict() {
        let found = read(
            &[t("Xanax", "alprazolam"), t("Doliprane", "paracétamol")],
            None,
        );
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.verdict == Verdict::Unknown));
        assert!(found.iter().all(|f| f.from.is_none()));
        assert_eq!(pending(&found), 2);
        // Et la phrase dit ce qui manque, pas ce qu'il faut faire.
        assert!(found[0].conduct.contains("aucun stade"));
    }

    /// **Le palier atteint est le plus grave franchi.**
    ///
    /// L'alprazolam se réduit dès le stade léger et se contre-indique au
    /// stade sévère. Lu en sévère, prendre le premier palier de la liste
    /// dirait « réduire la dose » d'un traitement qu'il faut arrêter.
    #[test]
    fn the_step_that_speaks_is_the_worst_one_crossed() {
        let at = |s: Stage| read(&[t("Xanax", "alprazolam")], Some(s))[0].verdict;
        assert_eq!(at(Stage::Mild), Verdict::Adapt(Level::Reduce));
        assert_eq!(at(Stage::Moderate), Verdict::Adapt(Level::Reduce));
        assert_eq!(at(Stage::Severe), Verdict::Adapt(Level::Contraindicated));
    }

    /// **« Rien à changer » est une réponse**, et c'est l'oxazépam qui
    /// la justifie à lui seul.
    ///
    /// Sa fiche écrit « aucune adaptation n'est nécessaire du fait de
    /// l'insuffisance hépatique légère à modérée » : c'est précisément
    /// la benzodiazépine qu'on cherche chez un cirrhotique, et une liste
    /// qui la tairait la rendrait aussi muette qu'un produit dont
    /// personne n'a rien écrit.
    #[test]
    fn nothing_to_change_is_an_answer_and_not_a_silence() {
        let found = read(&[t("Séresta", "oxazépam")], Some(Stage::Moderate));
        assert_eq!(found.len(), 1, "la ligne ne disparaît pas");
        assert_eq!(found[0].verdict, Verdict::Nothing);
        assert!(found[0].conduct.contains("ne demande pas"));
        // Au stade sévère, elle parle comme les autres.
        let severe = read(&[t("Séresta", "oxazépam")], Some(Stage::Severe));
        assert_eq!(severe[0].verdict, Verdict::Adapt(Level::Contraindicated));
        // Et la comparaison qui fait tout l'intérêt : au même stade, le
        // bromazépam demande quelque chose et l'oxazépam non.
        let both = read(
            &[t("Séresta", "oxazépam"), t("Lexomil", "bromazépam")],
            Some(Stage::Moderate),
        );
        let verdicts: Vec<Verdict> = both.iter().map(|f| f.verdict).collect();
        assert!(verdicts.contains(&Verdict::Nothing));
        assert!(verdicts.contains(&Verdict::Adapt(Level::Reduce)));
    }

    /// **Une hépatopathie évolutive n'est pas un stade**, donc les
    /// molécules qui s'y contre-indiquent ne sont pas dans cette table.
    ///
    /// Les statines sont contre-indiquées en « affection hépatique
    /// évolutive » : une maladie en cours, quel que soit le Child-Pugh.
    /// Les ranger sous un palier dirait la chose à un stade et la
    /// tairait aux autres, ce qui est faux des deux côtés.
    #[test]
    fn an_active_liver_disease_is_not_a_stage() {
        for statine in [
            t("Zocor", "simvastatine"),
            t("Tahor", "atorvastatine"),
            t("Crestor", "rosuvastatine"),
        ] {
            assert!(
                read(&[statine], Some(Stage::Severe)).is_empty(),
                "une hépatopathie évolutive n'est pas un stade"
            );
        }
    }

    /// L'ordre est celui de la gravité : ce qu'il faut arrêter d'abord,
    /// ce qui ne demande rien en dernier.
    #[test]
    fn the_order_is_the_one_you_read_in() {
        let found = read(
            &[
                t("Séresta", "oxazépam"),
                t("Lexomil", "bromazépam"),
                t("Motilium", "dompéridone"),
            ],
            Some(Stage::Moderate),
        );
        let ranks: Vec<u8> = found.iter().map(|f| f.verdict.rank()).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{ranks:?}");
        assert_eq!(found[0].treatment, "Motilium");
    }

    /// Chaque molécule de la table est une fiche que le logiciel livre,
    /// **et la fiche parle du foie**.
    ///
    /// C'est la règle de `facets.rs` et de `cyp.rs` : une ligne est
    /// adossée à ce que la fiche écrit, et se corrige en corrigeant la
    /// fiche.
    ///
    /// Le champ `renal` est lu avec les autres, et ce n'est pas une
    /// négligence : c'est la section « adaptation posologique » des
    /// fiches, et elle porte les deux organes. Zophren y écrit « Aucune
    /// adaptation en cas d'insuffisance rénale. En insuffisance
    /// hépatique modérée à sévère, ne pas dépasser 8 mg par jour ».
    /// C'est aussi ce qui explique que la moitié hépatique soit restée
    /// invisible si longtemps : elle vivait sous le nom de l'autre rein.
    #[test]
    fn every_row_is_backed_by_a_card_that_speaks_of_the_liver() {
        let cards: Vec<(String, String)> = crate::db::STARTER_DETAILS
            .iter()
            .map(|d| {
                (
                    crate::fuzzy::sort_key(d.name),
                    crate::fuzzy::sort_key(
                        &[d.contraindications, d.dosage, d.monitoring, d.renal].join(" "),
                    ),
                )
            })
            .collect();
        let drugs: Vec<(String, String)> = crate::db::STARTER_DRUGS
            .iter()
            .map(|(name, dci, class, tags)| {
                (
                    crate::fuzzy::sort_key(name),
                    crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}")),
                )
            })
            .collect();
        let mut orphans: Vec<&str> = Vec::new();
        let mut unbacked: Vec<&str> = Vec::new();
        for a in TABLE {
            let Some((card, _)) = drugs.iter().find(|(_, hay)| {
                a.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            }) else {
                orphans.push(a.label);
                continue;
            };
            let Some((_, body)) = cards.iter().find(|(n, _)| n == card) else {
                continue;
            };
            if !["hepat", "cirrhos"].iter().any(|w| body.contains(w)) {
                unbacked.push(a.label);
            }
        }
        assert!(
            orphans.is_empty(),
            "molécules sans fiche livrée : {orphans:?}"
        );
        assert!(
            unbacked.is_empty(),
            "molécules dont la fiche ne parle pas du foie : {unbacked:?}"
        );
    }

    /// Chaque ligne cite sa source, ne se répète pas, et ses paliers
    /// vont du plus léger au plus grave — c'est l'ordre de lecture, et
    /// deux paliers du même stade rendraient le plus grave dépendant de
    /// l'ordre d'écriture.
    #[test]
    fn every_row_names_itself_once_and_orders_its_steps() {
        let mut seen: Vec<&str> = Vec::new();
        for a in TABLE {
            assert!(!a.needs.is_empty(), "{} n'a pas de motif", a.label);
            assert!(!a.steps.is_empty(), "{} n'a pas de palier", a.label);
            assert!(a.source.len() > 30, "{} : source trop courte", a.label);
            assert!(!seen.contains(&a.label), "{} est en double", a.label);
            seen.push(a.label);
            let stages: Vec<Stage> = a.steps.iter().map(|s| s.from).collect();
            assert!(
                stages.windows(2).all(|w| w[0] < w[1]),
                "{} : paliers dans le désordre ou en double",
                a.label
            );
            for s in a.steps {
                assert!(s.conduct.len() > 15, "{} : conduite trop courte", a.label);
            }
        }
    }
}
