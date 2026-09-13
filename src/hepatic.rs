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
//! **Ce module et la table de référence « Foie » se répondent, et ne se
//! recouvrent pas.** La table est une page curée, sourcée EASL et HAS,
//! qui parle de la cirrhose ; celui-ci répète les paliers des RCP,
//! molécule par molécule. Les confronter les a trouvés d'accord partout
//! où ils se rencontrent — les paliers des AOD, l'absence délibérée des
//! statines, le choix des benzodiazépines qui ne passent pas par
//! l'oxydation — et a montré un trou, celui des associations
//! paracétamol-opioïde. Une différence subsiste et elle est **voulue** :
//! la table écrit que les AINS sont à éviter en cirrhose « quelle qu'en
//! soit la voie, gel compris chez le patient décompensé », là où ce
//! module ne connaît que les formes systémiques. La raison est la règle
//! d'adossement : les fiches des formes locales ne parlent pas du foie,
//! et une ligne d'ici doit pouvoir se corriger en corrigeant une fiche.
//! La nuance de la cirrhose décompensée vit donc dans la table, qui est
//! faite pour cela. Ne pas « réparer » l'un avec l'autre.
//!
//! **Et cette table est indexée sur la molécule, non sur la
//! présentation.** C'est une limite, et elle a un nom : l'azithromycine
//! n'y est pas, parce qu'« azithromycine » attrape aussi l'Azyter, qui
//! est un collyre. Une contre-indication hépatique systémique prêtée à
//! deux gouttes dans un œil est le genre d'alerte qui apprend à ignorer
//! les alertes — c'est la leçon du kétoconazole local dans `cyp.rs`, et
//! celle de `crush.rs`, qui s'indexe justement sur la présentation parce
//! qu'une table par DCI s'y tromperait une fois sur deux. Ici la
//! molécule suffit pour les cent dix lignes de la table ; le jour
//! où elle ne suffira plus, c'est le type qui devra changer, pas la
//! ligne qui devra ruser.
//!
//! ## Ce que le module tient
//!
//! Six règles, une par test :
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
//! * **Un chiffre dans une conduite est un plafond, jamais une
//!   posologie.** `renal.rs` refuse tout milligramme, et sa raison est
//!   bonne chez lui : la dose réduite d'un AOD dépend aussi de
//!   l'indication, du poids et de l'âge, si bien qu'un chiffre écrit là
//!   se lirait comme une prescription. Les RCP hépatiques, eux, posent
//!   des **bornes** — « ne pas dépasser 3 g de paracétamol par jour »
//!   ne dépend d'aucune indication, et la taire perdrait la seule chose
//!   utile de la ligne. La règle est donc affinée plutôt que copiée :
//!   une conduite peut porter un chiffre s'il est dans un « ne pas
//!   dépasser » ; une dose de départ s'écrit en fraction de la dose
//!   usuelle, comme la fiche l'écrit le plus souvent elle-même.
//! * **La conduite est celle du RCP, la décision est celle du
//!   prescripteur.** Beaucoup de ces contre-indications ne tiennent pas
//!   au métabolisme mais au **risque d'encéphalopathie** — les
//!   benzodiazépines, les diurétiques de l'anse — et une dose réduite
//!   n'y change rien.
//!
//! Statique, pur et testé, comme `renal`. Il ne connaît ni la base ni
//! egui : on lui passe des traitements et un stade.
//!
//! **Et la lecture est faite à chaque image**, puisque le panneau la
//! redessine : elle ne doit donc rien allouer par ligne de table.
//! Mesuré sur neuf traitements et cent neuf lignes, mille lectures :
//! **77 ms avec `hay.contains(&sort_key(n))`, 20 ms avec
//! `fuzzy::contains_folded`** — soit soixante-dix-sept microsecondes
//! par image ramenées à vingt. Le premier alloue une `String` par mot
//! cherché et par traitement ; le second replie au vol et n'alloue
//! rien. C'est exactement ce que la documentation de `contains_folded`
//! raconte des moteurs de règles, et les tables l'avaient manqué.

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

    /// Le repère stable d'un stade, pour adresser une conduite.
    ///
    /// Trois lettres qui ne bougent pas : ni la casse, ni les accents,
    /// ni la tournure française ne s'y glissent, parce qu'une adresse
    /// qui suit la prose se périme quand la prose se corrige.
    pub fn slug(self) -> &'static str {
        match self {
            Stage::Mild => "a",
            Stage::Moderate => "b",
            Stage::Severe => "c",
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
            // `contains_folded` plutôt que `contains(&sort_key(n))` :
            // le second alloue une `String` **par mot cherché et par
            // traitement**, et cette lecture est faite à chaque image.
            // Cent neuf lignes fois deux mots fois neuf traitements font
            // deux mille allocations par image ; c'est la raison d'être
            // de `contains_folded`, et les moteurs de règles l'avaient
            // déjà appris.
            if !a
                .needs
                .iter()
                .any(|n| crate::fuzzy::contains_folded(&hay, n))
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
pub fn pending(verdicts: impl IntoIterator<Item = Verdict>) -> usize {
    verdicts
        .into_iter()
        .filter(|v| *v == Verdict::Unknown)
        .count()
}

/// Le document sous lequel les conduites du panneau sont adressées.
pub const DOC: &str = "foie";

/// Chaque conduite avec son adresse, **calculée une seule fois** et
/// parcourue par les deux côtés.
///
/// Le repère est le libellé de la molécule et le **stade** du palier :
/// un palier se désigne par le stade à partir duquel il s'applique, qui
/// vient du RCP et ne bouge pas. Le rang se décalerait dès qu'on insère
/// un palier au-dessus — et une adresse tirée de la prose deviendrait
/// introuvable à la première correction de cette prose, ce qui est pire
/// que périmée.
fn addressed() -> Vec<(String, &'static str)> {
    TABLE
        .iter()
        .flat_map(|a| {
            let id = crate::content::slug(a.label);
            a.steps
                .iter()
                .map(move |s| (crate::content::key(DOC, &id, s.from.slug()), s.conduct))
        })
        .collect()
}

/// Toutes les conduites du panneau, avec leur adresse.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    addressed()
        .into_iter()
        .map(|(key, conduct)| (key, "conduite", conduct))
        .collect()
}

/// Ce que le foie impose, avec les mots de l'officine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resolved {
    pub treatment: String,
    pub label: &'static str,
    pub verdict: Verdict,
    pub from: Option<Stage>,
    pub conduct: String,
    pub source: &'static str,
}

/// Appliquer les réécritures de l'officine à ce que le foie impose.
///
/// Les deux phrases que le module compose lui-même — « aucun stade au
/// dossier » et « rien à changer à ce stade » — ne sont pas adressées :
/// elles ne viennent d'aucune fiche, elles disent l'état de la lecture
/// et non une conduite. Les réécrire serait réécrire le fonctionnement.
pub fn resolve(findings: Vec<Finding>, over: &crate::content::Overrides) -> Vec<Resolved> {
    findings
        .into_iter()
        .map(|f| {
            let id = crate::content::slug(f.label);
            // Le palier retenu est retrouvé par sa conduite **livrée** :
            // une conduite déjà réécrite se retrouve donc quand même,
            // puisque c'est le tableau qu'on interroge.
            let key = TABLE
                .iter()
                .find(|a| a.label == f.label)
                .and_then(|a| a.steps.iter().find(|s| s.conduct == f.conduct))
                .map(|s| crate::content::key(DOC, &id, s.from.slug()));
            Resolved {
                conduct: match &key {
                    Some(k) => over.get(k, f.conduct).to_owned(),
                    None => f.conduct.to_owned(),
                },
                treatment: f.treatment,
                label: f.label,
                verdict: f.verdict,
                from: f.from,
                source: f.source,
            }
        })
        .collect()
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
    // **Avant le paracétamol seul**, parce que ces boîtes en contiennent
    // et que ce n'est pas lui qui décide. Un Codoliprane, un Ixprim, un
    // Izalgi tombaient sur la ligne du paracétamol : le verdict était
    // juste au stade sévère, mais la conduite citait le Doliprane et
    // parlait d'un plafond de trois grammes, là où c'est l'opioïde
    // associé qui porte la contre-indication. La table de référence
    // « Foie » le dit de son côté — « éviter la codéine ; si un opioïde
    // est nécessaire, dose réduite, intervalle allongé » — et c'est en
    // confrontant les deux que la ligne manquante s'est vue.
    Adaptation {
        needs: &[
            "codoliprane",
            "ixprim",
            "izalgi",
            "klipal",
            "lamaline",
            "paracétamol + codéine",
            "paracétamol + tramadol",
            "poudre d'opium",
        ],
        label: "Paracétamol associé à un opioïde",
        steps: &[
            step(
                Mild,
                Reduce,
                "Dose réduite et intervalle allongé : c'est l'opioïde qui décide, pas le paracétamol. Un laxatif s'envisage d'emblée.",
            ),
            step(
                Severe,
                Contraindicated,
                "Insuffisance hépatocellulaire sévère : contre-indiqué. Le paracétamol seul, à dose réduite, reste l'antalgique de première intention.",
            ),
        ],
        source: "Codoliprane, Ixprim et Izalgi : contre-indication en « insuffisance hépatocellulaire sévère » ; Izalgi : « La dose est réduite chez le sujet âgé, l'insuffisant rénal ou hépatique et le patient de faible poids ».",
    },
    Adaptation {
        needs: &["paracetamol"],
        label: "Paracétamol",
        steps: &[
            step(
                Mild,
                Reduce,
                // Les deux grammes viennent de la table de référence
                // « Foie », qui les réserve au cumul : cirrhose **et**
                // dénutrition, alcoolisation active ou poids faible.
                // Ce module n'écrivait que les trois grammes de la
                // fiche, et le cumul est le cas ordinaire, pas l'exotique.
                "Ne pas dépasser 3 g par jour, et espacer les prises d'au moins six heures. Deux grammes seulement si s'y ajoutent une dénutrition, une alcoolisation active ou un poids faible.",
            ),
            step(Severe, Contraindicated, "Insuffisance hépatocellulaire sévère : contre-indiqué."),
        ],
        source: "Doliprane : « Sujet âgé, poids inférieur à 50 kg, dénutrition, alcoolisme chronique ou insuffisance hépatocellulaire : ne pas dépasser 3 g et espacer les prises d'au moins 6 heures » ; contre-indication : « insuffisance hépatocellulaire sévère ». EASL et HAS, prise en charge de la cirrhose, pour le plafond abaissé à 2 g quand la dénutrition ou l'alcoolisation s'ajoutent.",
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
                "Ne pas dépasser la moitié de la dose adulte, en débutant à la plus faible.",
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
            step(Mild, Reduce, "Ne pas dépasser 5 mg par jour."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Stilnox : « Chez le sujet de plus de 65 ans, l'insuffisant hépatique ou le patient fragile, la posologie est de 5 mg par jour et ne doit pas être dépassée ».",
    },
    Adaptation {
        needs: &["zopiclone"],
        label: "Zopiclone",
        steps: &[
            step(Mild, Reduce, "La moitié de la dose usuelle, soit un demi-comprimé."),
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
    // **Avant la morphine, parce qu'« apomorphine » la contient.**
    // L'Apokinon est un agoniste dopaminergique injectable de la maladie
    // de Parkinson, et il n'a rien d'un opioïde ; sans cette ligne, il
    // recevait la conduite hépatique de la morphine. Sa fiche porte sa
    // propre contre-indication, et c'est elle qui est écrite ici.
    Adaptation {
        needs: &["apomorphine"],
        label: "Apomorphine",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère : contre-indiqué.",
        )],
        source: "Apokinon : contre-indication en « insuffisance hépatique sévère ».",
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
    // Après l'ésoméprazole, et jamais avant : « esomeprazole » contient
    // « omeprazole », si bien que l'Inexium serait lu comme du Mopral.
    // C'est la leçon de `crush.rs` et de `cyp.rs`, et elle vaut ici.
    Adaptation {
        needs: &["omeprazole"],
        label: "Oméprazole",
        steps: &[step(Severe, Reduce, "Ne pas dépasser 20 mg par jour.")],
        source: "Mopral : « En insuffisance hépatique sévère, ne pas dépasser 20 mg par jour ».",
    },
    Adaptation {
        needs: &["losartan"],
        label: "Losartan",
        steps: &[
            step(Mild, Reduce, "Débuter à la moitié de la dose usuelle."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Cozaar : « Débuter à 25 mg chez le sujet de plus de 75 ans, en cas de déplétion volémique, de traitement diurétique à forte dose ou d'insuffisance hépatique » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["amlodipine"],
        label: "Amlodipine",
        steps: &[step(Mild, Reduce, "Débuter à la dose la plus faible et titrer lentement ; contrôler les transaminases.")],
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
        needs: &["edoxaban"],
        label: "Édoxaban",
        steps: &[step(Severe, Contraindicated, "Coagulopathie hépatique : contre-indiqué.")],
        source: "Lixiana : contre-indication en « coagulopathie hépatique ».",
    },
    Adaptation {
        needs: &["acetylsalicylique"],
        label: "Acide acétylsalicylique",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Kardégic : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["pantoprazole"],
        label: "Pantoprazole",
        steps: &[step(
            Severe,
            Reduce,
            "Ne pas dépasser 20 mg par jour, et surveiller les transaminases.",
        )],
        source: "Inipomp : « En insuffisance hépatique sévère, ne pas dépasser 20 mg par jour et surveiller les transaminases ».",
    },
    Adaptation {
        needs: &["ivabradine"],
        label: "Ivabradine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Procoralan : « Prudence en cas d'insuffisance hépatique modérée, contre-indication en cas d'atteinte sévère ».",
    },
    Adaptation {
        needs: &["sildenafil"],
        label: "Sildénafil",
        steps: &[
            step(Mild, Reduce, "Dose initiale réduite de moitié."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Viagra : « Chez le sujet âgé, l'insuffisant hépatique, l'insuffisant rénal sévère ou en cas d'association à un inhibiteur du CYP3A4, la dose initiale doit être de 25 mg » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["candesartan"],
        label: "Candésartan",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère et cholestase : contre-indiqué.",
        )],
        source: "Atacand : contre-indication en « insuffisance hépatique sévère et cholestase ».",
    },
    // **Le produit qui traite l'ascite du cirrhotique, et que le stade
    // sévère retire.** Les deux phrases sont dans la même fiche, et
    // c'est exactement le genre de rapprochement qu'on ne fait pas en
    // lisant une monographie de haut en bas.
    Adaptation {
        needs: &["spironolactone"],
        label: "Spironolactone",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère : contre-indiqué — alors même qu'elle traite l'ascite du cirrhotique à un stade plus précoce.",
        )],
        source: "Aldactone : contre-indication en « insuffisance hépatique sévère » ; posologie : « Ascite cirrhotique : 100 mg par jour en moyenne » ; surveillance : « Natrémie, en particulier chez le cirrhotique ».",
    },
    Adaptation {
        needs: &["diclofenac"],
        label: "Diclofénac",
        steps: &[step(
            Severe,
            Contraindicated,
            "Insuffisance hépatique sévère : contre-indiqué. C'est l'AINS le plus hépatotoxique de sa classe.",
        )],
        source: "Voltarène : contre-indication en « insuffisance rénale ou hépatique sévère » ; surveillance : « le diclofénac étant le plus hépatotoxique de la classe ».",
    },
    Adaptation {
        needs: &["metopimazine"],
        label: "Métopimazine",
        steps: &[step(
            Mild,
            Reduce,
            "Réduire la dose et espacer les prises : la sédation et la confusion sont majorées.",
        )],
        source: "Vogalène : « Prudence en cas d'insuffisance rénale ou hépatique : réduire la dose et espacer les prises, le risque de sédation et de confusion étant majoré ».",
    },
    Adaptation {
        needs: &["amitriptyline"],
        label: "Amitriptyline",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Laroxyl : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["sumatriptan"],
        label: "Sumatriptan",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Imigrane : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["nicardipine"],
        label: "Nicardipine",
        steps: &[step(
            Mild,
            Reduce,
            "Instaurer à posologie réduite et augmenter progressivement.",
        )],
        source: "Loxen : « L'instauration se fait à posologie réduite chez le sujet âgé, l'insuffisant hépatique et l'insuffisant rénal, avec augmentation progressive ».",
    },
    Adaptation {
        needs: &["ropinirole"],
        label: "Ropinirole",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Requip : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["olanzapine"],
        label: "Olanzapine",
        steps: &[step(Moderate, Reduce, "Débuter à la moitié de la dose usuelle.")],
        source: "Zyprexa : « il est toutefois recommandé de débuter à 5 mg par jour en cas d'insuffisance rénale, comme en cas d'insuffisance hépatique modérée ».",
    },
    Adaptation {
        needs: &["prazepam"],
        label: "Prazépam",
        steps: &[
            step(Mild, Reduce, "Posologie réduite de moitié environ."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère avec risque d'encéphalopathie : contre-indiqué."),
        ],
        source: "Lysanxia : « Chez le sujet âgé, l'insuffisant rénal ou l'insuffisant hépatique, la posologie doit être réduite de moitié environ » ; contre-indication en « insuffisance hépatique sévère du fait du risque d'encéphalopathie ».",
    },
    Adaptation {
        needs: &["clobazam"],
        label: "Clobazam",
        steps: &[
            step(Mild, Reduce, "Réduire de moitié la posologie initiale."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Urbanyl : « Chez le sujet âgé et l'insuffisant hépatique, réduire de moitié la posologie initiale » ; contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["tamsulosine"],
        label: "Tamsulosine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Josir : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["dutasteride"],
        label: "Dutastéride",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Avodart : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["eplerenone"],
        label: "Éplérénone",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Inspra : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["metronidazole"],
        label: "Métronidazole",
        steps: &[step(
            Severe,
            Reduce,
            "Réduire la posologie ; prudence, l'atteinte sévère majorant aussi le risque neurologique.",
        )],
        source: "Flagyl : « la posologie doit être réduite en cas d'insuffisance hépatique sévère » ; contre-indications : « Prudence en cas d'antécédent de neuropathie périphérique, de trouble hématologique ou d'atteinte hépatique sévère ».",
    },
    Adaptation {
        needs: &["ornidazole"],
        label: "Ornidazole",
        steps: &[step(Severe, Reduce, "Réduire la posologie.")],
        source: "Tibéral : « la posologie doit être réduite en cas d'insuffisance hépatique sévère » ; « Prudence en cas d'atteinte hépatique sévère ».",
    },
    Adaptation {
        needs: &["molsidomine"],
        label: "Molsidomine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Corvasal : « Contre-indiqué en cas d'insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["voriconazole"],
        label: "Voriconazole",
        steps: &[
            step(Mild, Reduce, "Dose d'entretien réduite de moitié."),
            step(
                Severe,
                Contraindicated,
                "Insuffisance hépatique sévère : non évaluée, contre-indiqué.",
            ),
        ],
        source: "Vfend : « Une insuffisance hépatique légère à modérée impose de réduire de moitié la dose d'entretien » ; contre-indication : « Insuffisance hépatique sévère non évaluée ».",
    },
    Adaptation {
        needs: &["griseofulvine"],
        label: "Griséofulvine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Griséfuline : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["mefloquine"],
        label: "Méfloquine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Lariam : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["safinamide"],
        label: "Safinamide",
        steps: &[
            step(Moderate, Reduce, "Réduire la posologie."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Xadago : « La posologie est réduite en cas d'insuffisance hépatique modérée et le médicament est contre-indiqué en cas d'insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["moclobemide"],
        label: "Moclobémide",
        steps: &[step(Severe, Reduce, "Posologie nettement réduite.")],
        source: "Moclamine : « la posologie doit être nettement réduite en cas d'insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["tofacitinib"],
        label: "Tofacitinib",
        steps: &[
            step(Moderate, Reduce, "Dose diminuée de moitié."),
            step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué."),
        ],
        source: "Xeljanz : « La dose est diminuée de moitié en cas […] d'insuffisance hépatique modérée » ; contre-indication : « Insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["mirabegron"],
        label: "Mirabégron",
        steps: &[
            step(Moderate, Reduce, "Réduire la posologie."),
            step(
                Severe,
                Contraindicated,
                "L'utilisation n'est pas recommandée à ce stade.",
            ),
        ],
        source: "Betmiga : « La posologie est réduite en cas […] d'insuffisance hépatique modérée » ; « L'utilisation n'est pas recommandée en cas […] d'insuffisance hépatique sévère ».",
    },
    // **La trimipramine avant l'imipramine**, parce que la seconde est
    // une sous-chaîne de la première : la table est lue dans l'ordre, et
    // sans cela le Surmontil citerait la fiche du Tofranil.
    Adaptation {
        needs: &["trimipramine"],
        label: "Trimipramine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Surmontil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["imipramine"],
        label: "Imipramine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Tofranil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["clomipramine"],
        label: "Clomipramine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Anafranil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["doxepine"],
        label: "Doxépine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Quitaxon : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["miansérine"],
        label: "Miansérine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Mianserine : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["clonazepam"],
        label: "Clonazépam",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Rivotril : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["loprazolam"],
        label: "Loprazolam",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Havlane : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["lormetazepam"],
        label: "Lormétazépam",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Noctamide : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["triazolam"],
        label: "Triazolam",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Halcion : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["doxylamine"],
        label: "Doxylamine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Donormyl : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["buspirone"],
        label: "Buspirone",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Buspirone : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["carvedilol"],
        label: "Carvédilol",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Kredex : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["lercanidipine"],
        label: "Lercanidipine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Lercanidipine : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["felodipine"],
        label: "Félodipine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Flodil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["bumetanide"],
        label: "Bumétanide",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Burinex : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["prasugrel"],
        label: "Prasugrel",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Efient : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["ticlopidine"],
        label: "Ticlopidine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Ticlopidine : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["glibenclamide"],
        label: "Glibenclamide",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Daonil : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["glipizide"],
        label: "Glipizide",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Glipizide : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["finerenone"],
        label: "Finérénone",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Kerendia : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["moxifloxacine"],
        label: "Moxifloxacine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Izilox : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["itraconazole"],
        label: "Itraconazole",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Sporanox : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["cotrimoxazole"],
        label: "Cotrimoxazole",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Bactrim : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["dextromethorphane"],
        label: "Dextrométhorphane",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Tussidane : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["zonisamide"],
        label: "Zonisamide",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Zonegran : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["perampanel"],
        label: "Pérampanel",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Fycompa : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["phenobarbital"],
        label: "Phénobarbital",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Gardénal : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["frovatriptan"],
        label: "Frovatriptan",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Frovatriptan : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["hydromorphone"],
        label: "Hydromorphone",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Sophidone : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["nalbuphine"],
        label: "Nalbuphine",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Nalbuphine : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["naproxene"],
        label: "Naproxène",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Apranax : contre-indication en « insuffisance hépatique sévère ».",
    },
    Adaptation {
        needs: &["etoricoxib"],
        label: "Étoricoxib",
        steps: &[step(Severe, Contraindicated, "Insuffisance hépatique sévère : contre-indiqué.")],
        source: "Arcoxia : contre-indication en « insuffisance hépatique sévère ».",
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
        assert_eq!(pending(found.iter().map(|f| f.verdict)), 2);
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

    /// **Aucun mot cherché n'est mangé par un mot placé plus haut.**
    ///
    /// La table est lue dans l'ordre et la première ligne qui répond
    /// gagne. Si le mot d'une ligne contient celui d'une ligne
    /// antérieure, la seconde ne répondra jamais : « esomeprazole »
    /// contient « omeprazole », « trimipramine » contient
    /// « imipramine », « apomorphine » contient « morphine ». À cent
    /// neuf lignes, la prochaine collision ne se verra pas à l'œil — et
    /// elle ne se verrait pas non plus à l'écran, puisque la ligne
    /// mangée se contente de citer la mauvaise fiche.
    #[test]
    fn no_need_is_eaten_by_one_placed_above_it() {
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

    /// **Une association ne se lit pas sur son composant le plus
    /// rassurant.**
    ///
    /// Un Codoliprane, un Ixprim, un Izalgi contiennent du paracétamol,
    /// et la ligne du paracétamol les attrapait : le verdict était juste
    /// au stade sévère — les deux sont contre-indiqués — mais la
    /// conduite citait le Doliprane et parlait d'un plafond de trois
    /// grammes, quand c'est l'opioïde associé qui porte la
    /// contre-indication. Le comptoir aurait lu « espacer les prises »
    /// là où la fiche dit « contre-indiqué ».
    ///
    /// C'est la table de référence « Foie » qui a montré le trou, en
    /// disant de son côté « éviter la codéine ».
    #[test]
    fn a_combination_is_not_read_on_its_mildest_part() {
        let codo = read(
            &[t("Codoliprane", "paracétamol + codéine")],
            Some(Stage::Mild),
        );
        assert_eq!(codo[0].label, "Paracétamol associé à un opioïde");
        assert!(codo[0].conduct.contains("opioïde qui décide"));
        // Le paracétamol seul garde la sienne.
        let doli = read(&[t("Doliprane", "paracétamol")], Some(Stage::Mild));
        assert_eq!(doli[0].label, "Paracétamol");
        assert!(doli[0].conduct.contains("3 g"));
    }

    /// **Le plus précis d'abord** : « esomeprazole » contient
    /// « omeprazole ».
    ///
    /// La table est lue dans l'ordre et la première ligne qui répond
    /// gagne, si bien qu'une ligne « oméprazole » placée avant ferait
    /// lire l'Inexium comme du Mopral — leurs plafonds sont les mêmes
    /// ici, mais la ligne citerait la mauvaise fiche, et le jour où
    /// l'un des deux change ce serait une conduite fausse. Même piège
    /// que « actiskenan »/« skenan » dans `crush.rs`.
    #[test]
    fn the_more_precise_row_answers_first() {
        let eso = read(&[t("Inexium", "ésoméprazole")], Some(Stage::Severe));
        assert_eq!(eso[0].label, "Ésoméprazole");
        assert!(eso[0].source.contains("Inexium"));
        let om = read(&[t("Mopral", "oméprazole")], Some(Stage::Severe));
        assert_eq!(om[0].label, "Oméprazole");
        assert!(om[0].source.contains("Mopral"));
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
            .map(|(name, dci, class, _antidote)| {
                (
                    crate::fuzzy::sort_key(name),
                    crate::fuzzy::sort_key(&format!("{name} {dci} {class}")),
                )
            })
            .collect();
        let mut orphans: Vec<&str> = Vec::new();
        let mut unbacked: Vec<String> = Vec::new();
        for a in TABLE {
            // **Toutes les fiches que la ligne attrape, et pas seulement
            // la première.** C'est le piège de `crush.rs` et de
            // `cyp.rs` — « actiskenan » contient « skenan »,
            // « esomeprazole » contient « omeprazole » — et il mord ici
            // aussi : « desloratadine » contient « loratadine », si bien
            // qu'une ligne écrite pour la Clarityne prêterait sa conduite
            // à l'Aerius, dont la fiche ne dit pas un mot du foie. La
            // version qui ne regardait que la première fiche trouvée
            // aurait laissé passer exactement cela, en silence et pour
            // le seul produit qu'elle ne visait pas.
            let caught: Vec<&String> = drugs
                .iter()
                .filter(|(_, hay)| {
                    a.needs
                        .iter()
                        .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
                })
                .map(|(name, _)| name)
                .collect();
            if caught.is_empty() {
                orphans.push(a.label);
                continue;
            }
            for card in caught {
                let Some((_, body)) = cards.iter().find(|(n, _)| n == card) else {
                    continue;
                };
                if !["hepat", "cirrhos"].iter().any(|w| body.contains(w)) {
                    unbacked.push(format!("{} → {card}", a.label));
                }
            }
        }
        assert!(
            orphans.is_empty(),
            "molécules sans fiche livrée : {orphans:?}"
        );
        assert!(
            unbacked.is_empty(),
            "lignes qui prêtent leur conduite à une fiche qui ne parle pas \
             du foie : {unbacked:?}"
        );
    }

    /// **Un chiffre dans une conduite est un plafond, jamais une
    /// posologie.**
    ///
    /// `renal.rs` refuse tout milligramme, et il a raison chez lui : la
    /// dose réduite d'un AOD dépend aussi de l'indication, du poids et
    /// de l'âge. Les RCP hépatiques posent des bornes, et « ne pas
    /// dépasser 3 g de paracétamol par jour » ne dépend d'aucune
    /// indication — la taire perdrait la seule chose utile de la ligne.
    /// Le test tient la version affinée : un chiffre n'est admis que
    /// dans un « ne pas dépasser ». Une dose de **départ** s'écrit en
    /// fraction de la dose usuelle, comme la fiche l'écrit elle-même.
    ///
    /// Vérifié en remettant « Débuter à 0,25 mg » chez l'alprazolam.
    #[test]
    fn a_figure_in_a_conduct_is_a_ceiling_and_never_a_dose() {
        for a in TABLE {
            for s in a.steps {
                let numbered = s.conduct.contains(" mg")
                    || s.conduct.contains(" g ")
                    || s.conduct.contains(" g.");
                if numbered {
                    assert!(
                        s.conduct.contains("dépasser"),
                        "{} : « {} » porte un chiffre hors d'un plafond",
                        a.label,
                        s.conduct
                    );
                }
            }
        }
    }

    /// **Chaque conduite est réécrivable, et chaque réécriture
    /// arrive.** Les deux sens, parce qu'ils échouent différemment.
    ///
    /// Une phrase absente de `phrases()` ne peut pas être corrigée :
    /// l'officine ne la voit pas dans « Textes imprimés ». Une phrase
    /// absente de `resolve()` s'affiche telle qu'elle est livrée
    /// pendant qu'on la croit corrigée, ce qui est pire — c'est la
    /// faute exacte que ce panneau a faite le jour où il a été branché
    /// sur les modules bruts.
    ///
    /// Le niveau, lui, ne bouge jamais : c'est un stade de RCP, pas une
    /// tournure. Et les deux phrases que le module compose lui-même —
    /// « aucun stade au dossier », « rien à changer à ce stade » — ne
    /// sont pas adressées : elles disent l'état de la lecture, pas une
    /// conduite.
    #[test]
    fn every_hepatic_conduct_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        assert_eq!(listed.len(), addressed().len());

        let found = read(&[t("Xanax", "alprazolam")], Some(Stage::Severe));
        assert!(!found.is_empty());
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        for f in resolve(found.clone(), &over) {
            assert!(f.conduct.starts_with("réécrit:"), "{}", f.conduct);
        }
        // Sans réécriture, la conduite est celle du RCP, et le verdict
        // comme le stade sont inchangés.
        let plain = resolve(found.clone(), &crate::content::Overrides::default());
        assert_eq!(plain[0].conduct, found[0].conduct);
        assert_eq!(plain[0].verdict, found[0].verdict);
        assert_eq!(plain[0].from, found[0].from);

        // Les deux phrases composées ne sont adressées par personne, et
        // traversent `resolve` telles quelles.
        let none = resolve(read(&[t("Xanax", "alprazolam")], None), &over);
        assert!(
            none[0].conduct.contains("aucun stade"),
            "{}",
            none[0].conduct
        );
        let nothing = resolve(read(&[t("Séresta", "oxazépam")], Some(Stage::Mild)), &over);
        assert!(
            nothing[0].conduct.contains("ne demande pas"),
            "{}",
            nothing[0].conduct
        );
    }

    /// **Deux paliers ne partagent jamais une adresse.** Deux conduites
    /// sous une même clé feraient hériter la seconde de la réécriture de
    /// la première — sur cette table, la conduite d'un stade appliquée à
    /// un autre.
    #[test]
    fn a_step_is_addressed_by_its_molecule_and_its_stage() {
        let mut keys: Vec<String> = addressed().into_iter().map(|(k, _)| k).collect();
        let seen = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(seen, keys.len(), "deux paliers partagent une adresse");
    }

    /// **Le cliquet : la table ne perd pas de molécules.**
    ///
    /// La règle de la maison pour tout catalogue clinique — une ligne
    /// retirée est une question à laquelle le comptoir ne sait plus
    /// répondre, et sans plancher cela arrive sans que personne le
    /// voie. Le nombre est écrit **une fois**, dans une constante que
    /// le message relit : écrit deux fois, en chiffres dans
    /// l'assertion et en lettres dans le message, il finit par se
    /// contredire — c'est arrivé dans `biology.rs`, dans `revue.rs` et
    /// dans le plancher de toxicité de `db.rs`.
    ///
    /// Il ne monte que lorsqu'une ligne est ajoutée, et jamais pour
    /// faire passer un test.
    #[test]
    fn the_table_only_ever_grows() {
        const FLOOR: usize = 110;
        assert!(
            TABLE.len() >= FLOOR,
            "{} molécules hépatiques, il y en avait {FLOOR}",
            TABLE.len()
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
