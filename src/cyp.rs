//! Les cytochromes, comme table et non comme paragraphe.
//!
//! Les fiches parlent des CYP, et elles en parlent bien : « Inhibiteurs
//! puissants du CYP3A4 (kétoconazole, itraconazole, clarithromycine) :
//! exposition majorée et risque de surdosage. » C'est une phrase juste,
//! elle est écrite sept cent trente-huit fois dans ce logiciel, et elle
//! ne répond pas à la question du comptoir, qui est : **cette
//! ordonnance-ci porte-t-elle deux lignes qui se rencontrent sur une
//! enzyme ?**
//!
//! Y répondre demandait d'ouvrir huit fiches et de croiser de tête. Le
//! logiciel avait les deux moitiés et ne les mettait jamais l'une en
//! face de l'autre — c'est mot pour mot ce que `facets.rs` a déjà refusé
//! une fois : la monographie répond « que sais-je de ce médicament », et
//! il manquait l'autre moitié, « quels médicaments ont telle propriété ».
//!
//! ## Ce que cette table ne sait pas
//!
//! Écrit avant le reste, comme dans `vigilance.rs`, parce qu'une table
//! d'interactions qu'on croit complète est plus dangereuse que pas de
//! table du tout.
//!
//! * **Elle ne connaît que sept cytochromes et cinq transporteurs.**
//!   La glycoprotéine P, l'OATP1B1 et la BCRP y sont entrés en 0.358,
//!   l'OCT2 et le MATE1 du rein — la metformine — en 0.362 :
//!   la rosuvastatine et la ciclosporine, la dabigatran et l'amiodarone
//!   sont des interactions majeures qu'aucun cytochrome ne porte, et la
//!   table les taisait. Restent dehors les OAT du rein — le
//!   méthotrexate sous AINS —, les glucuronoconjugaisons, la liaison
//!   aux protéines. Une ordonnance sans croisement sur cette table n'est
//!   pas une ordonnance sans interaction.
//! * **Elle ne connaît pas la pharmacodynamie.** Deux sédatifs, deux
//!   allongeurs du QT, deux néphrotoxiques ne se rencontrent sur aucune
//!   enzyme et se additionnent quand même. C'est `revue.rs` qui regarde
//!   cela.
//! * **Elle ne sait pas la dose, ni la durée, ni le génotype.** Un
//!   inhibiteur pris trois jours n'est pas un inhibiteur pris six mois,
//!   et un métaboliseur lent du CYP2D6 se comporte comme s'il portait un
//!   inhibiteur en permanence.
//! * **Elle ne dit pas ce qu'il faut faire.** Elle nomme une rencontre
//!   et son sens ; la conduite est au RCP et la décision au prescripteur.
//!
//! ## Ce qu'elle tient
//!
//! Cinq règles, une par test :
//!
//! * **Le silence n'est pas une permission.** Une ligne que la table ne
//!   connaît pas est **nommée** — jamais tue. Une lecture qui
//!   n'afficherait que les croisements trouvés se lirait « rien à
//!   signaler » sur une ordonnance de huit lignes dont six sont
//!   inconnues, et c'est ainsi qu'on écrase un comprimé à libération
//!   prolongée : c'est la règle de `crush.rs`, et elle vaut ici autant.
//! * **Pour une prodrogue, l'inhibiteur ne fait pas monter l'effet, il
//!   le fait tomber.** Le clopidogrel, la codéine et le tramadol
//!   n'agissent que par un métabolite : inhiber l'enzyme qui le fabrique
//!   n'accumule pas le produit, elle supprime son effet. Un moteur
//!   naïf annonce « exposition augmentée, risque de surdosage » là où le
//!   patient est en fait **sans antiagrégant**. C'est l'erreur qui coûte
//!   le plus cher sur cette table, et c'est pourquoi elle est un champ du
//!   type et non un commentaire.
//! * **La force d'un substrat n'est pas celle d'un inhibiteur.** Sous un
//!   même mot, deux mesures : pour un acteur, de combien il déplace
//!   l'exposition d'autrui ; pour un substrat, quelle part de sa propre
//!   élimination passe par cette enzyme. C'est le piège du grade de
//!   `facets.rs` — « gravité quand il altère, centralité quand il
//!   traite » — et il est écrit ici pour la même raison.
//! * **La force est celle que la fiche écrit, ou aucune.** « Puissant »,
//!   « modéré », « faible » se recopient ; une fiche qui nomme l'enzyme
//!   sans dire la force donne `None`, et la lecture dit que la force
//!   n'est pas chiffrée. Une force devinée serait une force fausse, et
//!   c'est elle qui décide de l'ordre de lecture. C'est la règle de
//!   `renal.rs` : sans DFG, pas de verdict.
//! * **Ce que la fiche nie n'entre pas dans la table.** La rosuvastatine
//!   n'est pas métabolisée par le CYP3A4, la pravastatine non plus, et
//!   la spiramycine est le macrolide qui n'inhibe pas — les trois le
//!   disent en toutes lettres. Ce sont exactement les trois cas qu'un
//!   moteur d'interactions par classe se trompe, en interdisant la
//!   clarithromycine avec la rosuvastatine et en laissant passer la
//!   simvastatine.
//!
//! Statique, pur, testé. Il ne connaît ni la base ni egui : on lui passe
//! des traitements.

/// Une voie par laquelle deux lignes se rencontrent : un isoenzyme du
/// cytochrome P450, parmi ceux qui décident quelque chose au comptoir,
/// ou l'un des trois transporteurs qui font les interactions que les
/// cytochromes n'expliquent pas.
///
/// Le CYP2E1 n'y est pas : il explique des toxicités — le paracétamol
/// sous alcool — et presque aucune conduite à tenir sur une ordonnance.
///
/// **Les transporteurs se lisent comme les enzymes**, et c'est pourquoi
/// ils sont dans le même type : un inhibiteur de la glycoprotéine P
/// accumule ce qu'elle rejette, un inducteur le fait fondre, exactement
/// comme sur le CYP3A4. Ce sont eux qui répondent à « dabigatran et
/// amiodarone » et à « rosuvastatine et ciclosporine », deux
/// interactions majeures qu'aucun cytochrome ne porte — la table les
/// taisait jusqu'en 0.358.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Enzyme {
    Cyp1a2,
    Cyp2b6,
    Cyp2c8,
    Cyp2c9,
    Cyp2c19,
    Cyp2d6,
    Cyp3a4,
    /// La glycoprotéine P (ABCB1) : l'efflux intestinal, rénal et à la
    /// barrière hémato-encéphalique.
    Pgp,
    /// La captation hépatique des statines, du répaglinide.
    Oatp1b1,
    /// La BCRP (ABCG2), l'autre pompe d'efflux.
    Bcrp,
    /// L'entrée des cations organiques dans la cellule du tubule rénal :
    /// la metformine, la fampridine.
    Oct2,
    /// Leur sortie dans l'urine, de l'autre côté de la même cellule.
    Mate1,
}

impl Enzyme {
    pub const ALL: &'static [Enzyme] = &[
        Enzyme::Cyp1a2,
        Enzyme::Cyp2b6,
        Enzyme::Cyp2c8,
        Enzyme::Cyp2c9,
        Enzyme::Cyp2c19,
        Enzyme::Cyp2d6,
        Enzyme::Cyp3a4,
        Enzyme::Pgp,
        Enzyme::Oatp1b1,
        Enzyme::Bcrp,
        Enzyme::Oct2,
        Enzyme::Mate1,
    ];

    /// Un transporteur, et non un cytochrome : c'est ce que la portée
    /// annonce en tête de la lecture, les deux comptés à part.
    pub fn is_transporter(self) -> bool {
        matches!(
            self,
            Enzyme::Pgp | Enzyme::Oatp1b1 | Enzyme::Bcrp | Enzyme::Oct2 | Enzyme::Mate1
        )
    }

    /// Les façons dont une fiche nomme cette voie, repliées. La
    /// glycoprotéine P s'écrit de trois manières dans les fiches, et
    /// « OATP1B1/1B3 » nomme bien l'OATP1B1 : c'est ce que le test
    /// d'adossement cherche.
    pub fn spellings(self) -> &'static [&'static str] {
        match self {
            Enzyme::Pgp => &["p-gp", "glycoproteine p", "p-glycoproteine"],
            Enzyme::Oatp1b1 => &["oatp1b1"],
            Enzyme::Bcrp => &["bcrp"],
            Enzyme::Oct2 => &["oct2"],
            Enzyme::Mate1 => &["mate1"],
            _ => &[],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Enzyme::Cyp1a2 => "CYP1A2",
            Enzyme::Cyp2b6 => "CYP2B6",
            Enzyme::Cyp2c8 => "CYP2C8",
            Enzyme::Cyp2c9 => "CYP2C9",
            Enzyme::Cyp2c19 => "CYP2C19",
            Enzyme::Cyp2d6 => "CYP2D6",
            Enzyme::Cyp3a4 => "CYP3A4",
            Enzyme::Pgp => "P-gp",
            Enzyme::Oatp1b1 => "OATP1B1",
            Enzyme::Bcrp => "BCRP",
            Enzyme::Oct2 => "OCT2",
            Enzyme::Mate1 => "MATE1",
        }
    }
}

/// Ce qu'une molécule fait de cette enzyme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Elle y passe : c'est son élimination qui en dépend.
    Substrate,
    /// Elle la freine : ce qui y passe s'accumule.
    Inhibitor,
    /// Elle l'accélère : ce qui y passe s'évanouit.
    Inducer,
}

/// Combien — **et de quoi**, selon le rôle. Voir la règle du module :
/// pour un acteur c'est le déplacement qu'il impose, pour un substrat
/// c'est la part de sa propre élimination qui passe par là.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Force {
    Weak,
    Moderate,
    Strong,
}

impl Force {
    /// Comment la force se dit, **selon le rôle** : le même mot ne
    /// nomme pas la même chose des deux côtés, et l'écrire pareil est
    /// précisément ce qui ferait lire « substrat puissant » comme
    /// « inhibiteur puissant ».
    pub fn label(self, role: Role) -> &'static str {
        match (role, self) {
            (Role::Substrate, Force::Strong) => "voie principale",
            (Role::Substrate, Force::Moderate) => "voie partielle",
            (Role::Substrate, Force::Weak) => "voie accessoire",
            (_, Force::Strong) => "puissant",
            (_, Force::Moderate) => "modéré",
            (_, Force::Weak) => "faible",
        }
    }
}

/// Une ligne de la table : ce qu'une molécule fait d'une enzyme.
#[derive(Clone, Copy, Debug)]
pub struct Action {
    pub enzyme: Enzyme,
    pub role: Role,
    /// `None` quand la fiche nomme l'enzyme sans dire la force. Ce
    /// n'est pas « faible » : c'est « non chiffré », et la lecture le
    /// dit.
    pub force: Option<Force>,
    /// **Ce que l'enzyme fabrique est-il l'effet ?** Vrai pour une
    /// prodrogue, et c'est ce champ qui retourne le sens du croisement.
    /// Sans objet pour un inhibiteur ou un inducteur.
    pub prodrug: bool,
}

impl Action {
    const fn new(enzyme: Enzyme, role: Role, force: Option<Force>) -> Action {
        Action {
            enzyme,
            role,
            force,
            prodrug: false,
        }
    }

    /// Un substrat dont le métabolite **est** le principe actif.
    const fn prodrug(enzyme: Enzyme, force: Option<Force>) -> Action {
        Action {
            enzyme,
            role: Role::Substrate,
            force,
            prodrug: true,
        }
    }
}

/// Une molécule et ce qu'elle fait des cytochromes.
pub struct Profile {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes —
    /// repliés par `fuzzy::sort_key`, comme partout ici.
    pub needs: &'static [&'static str],
    /// Ce que la ligne annonce.
    pub label: &'static str,
    /// **Vide veut dire quelque chose** : la molécule est connue, et
    /// elle ne passe par aucune des voies que cette table suit. Ce
    /// n'est pas « on ne sait pas » — c'est le contraire, et c'est
    /// précisément ce qu'on veut lire en cherchant par quoi remplacer
    /// une simvastatine sous clarithromycine. La pravastatine le dit en
    /// toutes lettres dans sa fiche ; la taire la rendrait aussi muette
    /// qu'un produit dont personne n'a rien écrit.
    pub actions: &'static [Action],
    /// La phrase de la fiche d'où la ligne est tirée.
    pub source: &'static str,
}

/// Ce que le croisement fait à la ligne touchée.
///
/// Quatre cas et non deux, parce qu'une prodrogue retourne le sens :
/// voir la règle du module.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shift {
    /// L'exposition monte : le produit s'accumule.
    ExposureUp,
    /// L'exposition baisse : le produit passe sous la dose utile.
    ExposureDown,
    /// **L'effet tombe** : l'enzyme freinée est celle qui fabriquait le
    /// principe actif. Le produit, lui, s'accumule sans agir.
    ActivityDown,
    /// L'effet monte : l'enzyme accélérée fabrique plus de principe
    /// actif.
    ActivityUp,
}

impl Shift {
    /// Le sens d'un croisement : il sort du rôle de l'acteur et de la
    /// nature du substrat, et de rien d'autre.
    fn of(role: Role, prodrug: bool) -> Option<Shift> {
        match (role, prodrug) {
            (Role::Inhibitor, false) => Some(Shift::ExposureUp),
            (Role::Inhibitor, true) => Some(Shift::ActivityDown),
            (Role::Inducer, false) => Some(Shift::ExposureDown),
            (Role::Inducer, true) => Some(Shift::ActivityUp),
            // Un substrat n'agit sur personne.
            (Role::Substrate, _) => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Shift::ExposureUp => "exposition augmentée",
            Shift::ExposureDown => "exposition diminuée",
            Shift::ActivityDown => "effet diminué (prodrogue)",
            Shift::ActivityUp => "effet augmenté (prodrogue)",
        }
    }
}

/// L'ordre de lecture d'un croisement.
///
/// **Ce n'est pas une gravité clinique** : le module ne sait ni la dose,
/// ni la durée, ni le terrain, et un croisement mineur sur une marge
/// thérapeutique étroite passe avant un croisement majeur sur un
/// produit qui pardonne. C'est l'ordre dans lequel on les regarde, et
/// c'est tout ce que le nom promet.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Weight {
    Major,
    Notable,
    Minor,
}

impl Weight {
    /// Le poids se lit sur **les deux** forces : un inhibiteur puissant
    /// sur une voie accessoire déplace peu, et un inhibiteur faible sur
    /// la voie principale déplace beaucoup. Prendre la seule force de
    /// l'acteur classerait le premier au-dessus du second.
    ///
    /// Une force non chiffrée ne tire pas vers le bas : elle laisse le
    /// croisement au milieu, là où on le regardera.
    fn of(actor: Option<Force>, substrate: Option<Force>) -> Weight {
        let rank = |f: Option<Force>| match f {
            Some(Force::Strong) => 3,
            None | Some(Force::Moderate) => 2,
            Some(Force::Weak) => 1,
        };
        match rank(actor) + rank(substrate) {
            6 => Weight::Major,
            5 => Weight::Major,
            2 => Weight::Minor,
            3 => Weight::Minor,
            _ => Weight::Notable,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Weight::Major => "prioritaire",
            Weight::Notable => "à examiner",
            Weight::Minor => "pour information",
        }
    }
}

/// Une rencontre entre deux lignes de l'ordonnance sur une enzyme.
///
/// Il n'y a **pas de champ où écrire une conclusion**, et c'est
/// délibéré : le type est ce qui empêche le module d'en rendre une.
/// C'est la discipline de `vigilance.rs`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Crossing {
    /// La ligne qui agit, telle qu'elle est écrite au dossier.
    pub actor: String,
    /// Ce que la table nomme derrière elle.
    pub actor_label: &'static str,
    /// La ligne dont l'exposition — ou l'effet — bouge.
    pub affected: String,
    pub affected_label: &'static str,
    pub enzyme: Enzyme,
    pub role: Role,
    pub actor_force: Option<Force>,
    pub substrate_force: Option<Force>,
    pub shift: Shift,
    pub weight: Weight,
    /// D'où sortent les deux lignes, dans l'ordre acteur puis touché.
    pub sources: (&'static str, &'static str),
}

/// Ce que la table sait d'une ordonnance.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Reading {
    pub crossings: Vec<Crossing>,
    /// Les lignes dont la table ne sait rien, **nommées**. Voir la
    /// première règle du module.
    pub unknown: Vec<String>,
    /// Les lignes que la table connaît et qui ne passent par aucune de
    /// ses voies.
    ///
    /// **Ce n'est pas la même chose que `unknown`, et c'est tout
    /// l'intérêt** : « on ne sait pas » et « on sait, et il n'y a rien »
    /// se ressemblent sur un écran qui les tairait tous les deux, et ce
    /// sont deux réponses opposées quand on cherche par quoi remplacer
    /// une simvastatine sous clarithromycine.
    pub inert: Vec<String>,
}

/// Ce que la table sait de cette ligne-là.
pub fn of(name: &str, dci: &str, class: &str, tags: &str) -> Option<&'static Profile> {
    let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}"));
    // **Le premier qui répond gagne, et l'ordre de la table décide.**
    // « esomeprazole » contient « omeprazole » une fois replié : sans
    // l'ordre, l'Inexium serait annoncé comme du Mopral. C'est la leçon
    // de `crush.rs`, où « actiskenan » contient « skenan », et elle
    // porte son test.
    // Sans replier une copie de chaque mot cherché : la monographie
    // appelle ceci pour chaque fiche de la base à son ouverture, et une
    // allocation par mot et par fiche en faisait trois cent mille.
    TABLE.iter().find(|p| {
        p.needs
            .iter()
            .any(|n| crate::fuzzy::contains_folded(&hay, n))
    })
}

/// Ce que cette ordonnance croise sur les cytochromes.
///
/// L'ordre est celui du poids, puis celui des noms : deux croisements de
/// même poids ne doivent pas échanger leur place d'une image à l'autre.
pub fn cross(treatments: &[crate::revue::Treatment]) -> Reading {
    let mut known: Vec<(String, &'static Profile)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut inert: Vec<String> = Vec::new();
    for t in treatments {
        let name = t.name.trim().to_owned();
        if name.is_empty() {
            continue;
        }
        // **Une forme locale ne croise rien**, et sa fiche le dit :
        // l'Ikervis est de la ciclosporine en collyre, et « ciclosporine »
        // ne peut pas sortir de la table — le Néoral en vit. Le
        // Kétoderm, lui, avait été réglé en amputant la molécule ;
        // celle-ci ne se laisse pas amputer, et c'est la voie qu'il faut
        // lire. Elle part en **inconnue** plutôt qu'en inerte, comme le
        // Kétoderm : ce module nomme ce sur quoi il ne se prononce pas,
        // et ne délivre pas de certificat de bonne conduite.
        // Sauf le miconazole buccal — voir `classes::local_but_absorbed`.
        if crate::classes::stays_local(t.dci, t.class) {
            unknown.push(name);
            continue;
        }
        match of(t.name, t.dci, t.class, t.tags) {
            // Connue, et sans voie qui compte : elle ne croise rien et
            // ce n'est pas une ignorance. Voir `Reading::inert`.
            Some(p) if p.actions.is_empty() => inert.push(name),
            Some(p) => known.push((name, p)),
            None => unknown.push(name),
        }
    }
    let mut crossings = Vec::new();
    for (i, (actor, ap)) in known.iter().enumerate() {
        for act in ap.actions {
            let Some(shift) = Shift::of(act.role, false) else {
                continue;
            };
            for (j, (affected, bp)) in known.iter().enumerate() {
                // **Une molécule n'agit pas sur elle-même.** Le
                // ticagrélor est substrat et inhibiteur du CYP3A4 ;
                // sans cette ligne il se croiserait lui-même et
                // l'ordonnance porterait un conflit imaginaire.
                if i == j {
                    continue;
                }
                for sub in bp.actions.iter().filter(|s| s.role == Role::Substrate) {
                    if sub.enzyme != act.enzyme {
                        continue;
                    }
                    let shift = Shift::of(act.role, sub.prodrug).unwrap_or(shift);
                    crossings.push(Crossing {
                        actor: actor.clone(),
                        actor_label: ap.label,
                        affected: affected.clone(),
                        affected_label: bp.label,
                        enzyme: act.enzyme,
                        role: act.role,
                        actor_force: act.force,
                        substrate_force: sub.force,
                        shift,
                        weight: Weight::of(act.force, sub.force),
                        sources: (ap.source, bp.source),
                    });
                }
            }
        }
    }
    crossings.sort_by(|a, b| {
        a.weight
            .cmp(&b.weight)
            .then(a.affected.cmp(&b.affected))
            .then(a.actor.cmp(&b.actor))
            .then(a.enzyme.cmp(&b.enzyme))
    });
    Reading {
        crossings,
        unknown,
        inert,
    }
}

/// Les croisements d'une même paire, **réunis**.
///
/// Un inhibiteur puissant du CYP3A4 est presque toujours aussi un
/// inhibiteur de la glycoprotéine P, et une colchicine passe par les
/// deux : la même rencontre s'écrirait deux fois, l'une sous l'autre, et
/// la liste doublerait de longueur pour ne rien dire de plus. On réunit
/// donc ce qui a le même acteur, le même touché et le même sens ; chaque
/// groupe garde toutes ses voies, la plus lourde en tête — la lecture
/// est déjà triée par poids, et le groupe prend la place de sa première
/// ligne.
pub fn grouped(crossings: &[Crossing]) -> Vec<Vec<&Crossing>> {
    let mut groups: Vec<Vec<&Crossing>> = Vec::new();
    for c in crossings {
        match groups
            .iter_mut()
            .find(|g| g[0].actor == c.actor && g[0].affected == c.affected && g[0].shift == c.shift)
        {
            Some(g) => g.push(c),
            None => groups.push(vec![c]),
        }
    }
    groups
}

use Enzyme::{
    Bcrp, Cyp1a2, Cyp2b6, Cyp2c19, Cyp2c8, Cyp2c9, Cyp2d6, Cyp3a4, Mate1, Oatp1b1, Oct2, Pgp,
};
use Force::{Moderate, Strong, Weak};
use Role::{Inducer, Inhibitor, Substrate};

/// Ce que chaque molécule fait des cytochromes.
///
/// **Rien ici n'est inventé** : chaque rôle et chaque force sont ceux
/// que la fiche écrit, et la fiche est citée. Une molécule dont la fiche
/// nomme l'enzyme sans dire ce qu'elle en fait n'y est pas ; une force
/// que la fiche ne qualifie pas reste `None`.
///
/// **L'ordre compte** : le premier motif qui répond gagne, et certains
/// noms se contiennent. L'ésoméprazole passe avant l'oméprazole.
pub const TABLE: &[Profile] = &[
    // ---- Inhibiteurs ----
    Profile {
        needs: &["itraconazole"],
        label: "Itraconazole",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, Some(Strong)),
        ],
        source: "Sporanox : « Inhibiteur puissant du CYP3A4 et de la P-glycoprotéine, et substrat du CYP3A4 » ; la P-glycoprotéine est dans la même phrase que le CYP3A4.",
    },
    Profile {
        needs: &["voriconazole"],
        label: "Voriconazole",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp2c9, Inhibitor, Some(Strong)),
            Action::new(Cyp2c19, Inhibitor, Some(Strong)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
        ],
        source: "Vfend : « Inhibiteur puissant des CYP3A4, 2C9 et 2C19 et substrat du CYP2C19 ».",
    },
    Profile {
        needs: &["posaconazole"],
        label: "Posaconazole",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Noxafil : « Inhibiteur très puissant du CYP3A4, sans effet notable sur les autres cytochromes ; il inhibe aussi la P-glycoprotéine dans une mesure non chiffrée, et en est le substrat ».",
    },
    Profile {
        needs: &["fluconazole"],
        label: "Fluconazole",
        actions: &[
            Action::new(Cyp2c9, Inhibitor, Some(Strong)),
            Action::new(Cyp2c19, Inhibitor, Some(Moderate)),
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
        ],
        source: "Triflucan : « Inhibiteur du CYP2C9 et du CYP2C19, et du CYP3A4 aux doses élevées ».",
    },
    Profile {
        needs: &["miconazole"],
        label: "Miconazole",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp2c9, Inhibitor, Some(Strong)),
        ],
        source: "Daktarin : « Le miconazole, même en gel buccal, est un inhibiteur puissant des CYP3A4 et CYP2C9 après déglutition partielle ».",
    },
    Profile {
        needs: &["clarithromycine"],
        label: "Clarithromycine",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, Some(Strong)),
        ],
        source: "Zeclar : « Inhibiteur puissant du CYP3A4 et de la P-glycoprotéine, ce qui en fait l'un des antibiotiques les plus pourvoyeurs d'interactions » ; la P-glycoprotéine est dans la même phrase que le CYP3A4.",
    },
    Profile {
        needs: &["josamycine"],
        label: "Josamycine",
        actions: &[Action::new(Cyp3a4, Inhibitor, None)],
        source: "Josacine : « Comme les autres macrolides à quatorze et seize chaînons, elle inhibe le CYP3A4, source d'interactions majeures » — la fiche ne qualifie pas la force.",
    },
    Profile {
        needs: &["norvir"],
        label: "Norvir",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Inhibitor, None),
            Action::new(Cyp1a2, Inducer, None),
            Action::new(Cyp2c8, Inducer, None),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Cyp2c19, Inducer, None),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Norvir : « c'est un inhibiteur puissant du CYP3A4 et, dans une moindre mesure, du CYP2D6 » ; « il accélère la glucuronidation et l'oxydation par le CYP1A2, le CYP2C8, le CYP2C9 et le CYP2C19 » ; « Métabolisme hépatique extensif, principalement par le CYP3A4 » ; « et un inhibiteur de la P-glycoprotéine » — sans force.",
    },
    Profile {
        needs: &["ritonavir", "nirmatrelvir"],
        label: "Ritonavir",
        actions: &[Action::new(Cyp3a4, Inhibitor, Some(Strong))],
        source: "Paxlovid : « il est utilisé comme potentialisateur pharmacocinétique, inhibant puissamment le CYP3A4 ».",
    },
    Profile {
        needs: &["verapamil"],
        label: "Vérapamil",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Isoptine : « Le vérapamil est un inhibiteur modéré du CYP3A4 et un inhibiteur de la P-gp » ; « Métabolisme hépatique intense de premier passage par le CYP3A4 » ; Tarka : « Le vérapamil inhibe le CYP3A4 et la P-gp ».",
    },
    Profile {
        needs: &["diltiazem"],
        label: "Diltiazem",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, Some(Weak)),
        ],
        source: "Tildiem : « Inhibiteur modéré du CYP3A4 et faible de la P-glycoprotéine » ; « Métabolisme hépatique important par le CYP3A4 ».",
    },
    Profile {
        needs: &["fluvoxamine"],
        label: "Fluvoxamine",
        actions: &[
            Action::new(Cyp1a2, Inhibitor, Some(Strong)),
            Action::new(Cyp2c19, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Inhibitor, None),
        ],
        source: "Floxyfral : « inhibiteur enzymatique puissant, notamment du CYP1A2 et du CYP2C19 » ; « Inhibition du CYP2C19 et du CYP3A4 ».",
    },
    Profile {
        needs: &["paroxetine"],
        label: "Paroxétine",
        actions: &[
            Action::new(Cyp2d6, Inhibitor, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Strong)),
        ],
        source: "Deroxat : « Inhibiteur puissant du CYP2D6 » ; « Métabolisme hépatique extensif par le CYP2D6, qu'elle sature et inhibe ».",
    },
    Profile {
        needs: &["fluoxetine"],
        label: "Fluoxétine",
        actions: &[
            Action::new(Cyp2d6, Inhibitor, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Strong)),
        ],
        source: "Prozac : « C'est un inhibiteur puissant du CYP2D6 » ; « Métabolisme hépatique important, notamment par le CYP2D6 ».",
    },
    Profile {
        needs: &["sertraline"],
        label: "Sertraline",
        actions: &[Action::new(Cyp2d6, Inhibitor, Some(Moderate))],
        source: "Zoloft : « Inhibiteur modéré du CYP2D6 aux fortes doses ».",
    },
    Profile {
        needs: &["bupropion"],
        label: "Bupropion",
        actions: &[
            Action::new(Cyp2d6, Inhibitor, Some(Strong)),
            Action::new(Cyp2b6, Substrate, Some(Strong)),
        ],
        source: "Zyban : « Inhibition puissante du CYP2D6 par le bupropion » ; « Métabolisme hépatique principalement par le CYP2B6 ».",
    },
    Profile {
        needs: &["terbinafine"],
        label: "Terbinafine",
        actions: &[Action::new(Cyp2d6, Inhibitor, Some(Strong))],
        source: "Lamisil : « Inhibiteur puissant du CYP2D6 ».",
    },
    Profile {
        needs: &["ciprofloxacine"],
        label: "Ciprofloxacine",
        actions: &[Action::new(Cyp1a2, Inhibitor, Some(Strong))],
        source: "Ciflox : « Inhibiteur puissant du CYP1A2 : tizanidine contre-indiquée, théophylline déconseillée ».",
    },
    // **L'ésoméprazole avant l'oméprazole** : le second est contenu dans
    // le premier une fois replié.
    Profile {
        needs: &["esomeprazole"],
        label: "Ésoméprazole",
        actions: &[
            Action::new(Cyp2c19, Inhibitor, Some(Moderate)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
        ],
        source: "Inexium : « l'inhibition du CYP2C19 réduisant l'activation du clopidogrel » ; « Métabolisme hépatique complet par les CYP2C19 et CYP3A4 ».",
    },
    Profile {
        needs: &["omeprazole"],
        label: "Oméprazole",
        actions: &[
            Action::new(Cyp2c19, Inhibitor, Some(Moderate)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
        ],
        source: "Mopral : « l'oméprazole inhibe le CYP2C19 et diminue la formation du métabolite actif » ; « Métabolisme hépatique complet par le CYP2C19 et le CYP3A4 ».",
    },
    // ---- Inducteurs ----
    Profile {
        needs: &["rifampicine"],
        label: "Rifampicine",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, Some(Strong)),
            Action::new(Cyp2c19, Inducer, Some(Strong)),
            Action::new(Cyp1a2, Inducer, Some(Strong)),
            Action::new(Pgp, Inducer, Some(Strong)),
        ],
        source: "Rifadine : « Inducteur enzymatique parmi les plus puissants connus, portant sur les CYP3A4, 2C9, 2C19, 1A2 et sur la P-glycoprotéine » ; Rifinah : « inducteur enzymatique très puissant des CYP3A4, 2C9, 2C19, 1A2 et de la P-glycoprotéine ».",
    },
    Profile {
        needs: &["phenytoine"],
        label: "Phénytoïne",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, Some(Strong)),
            Action::new(Cyp2c19, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Substrate, Some(Strong)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
        ],
        source: "Di-Hydan : « inducteur enzymatique puissant et large, portant sur les CYP3A4, CYP2C9, CYP2C19 » ; « Métabolisme hépatique saturable par les CYP2C9 et CYP2C19 ».",
    },
    Profile {
        needs: &["millepertuis", "hypericum"],
        label: "Millepertuis",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Cyp1a2, Inducer, None),
            Action::new(Pgp, Inducer, Some(Strong)),
        ],
        source: "Millepertuis : « Inducteur enzymatique puissant du CYP3A4 et de la P-gp » ; « l'induction concerne le CYP3A4, le CYP2C9, le CYP1A2 » — la fiche ne qualifie que le premier ; « Inducteur enzymatique puissant du CYP3A4 et de la P-gp ».",
    },
    // ---- Substrats ----
    Profile {
        needs: &["simvastatine"],
        label: "Simvastatine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Zocor : « La simvastatine est un substrat sensible du CYP3A4, ce qui en fait la statine la plus exposée aux interactions ».",
    },
    Profile {
        needs: &["atorvastatine"],
        label: "Atorvastatine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Oatp1b1, Substrate, None),
        ],
        source: "Tahor : « Métabolisme hépatique intense par le CYP3A4 en métabolites actifs » ; « L'atorvastatine est un substrat des transporteurs hépatiques OATP1B1 et OATP1B3 ».",
    },
    // **La rosuvastatine n'est pas un substrat du CYP3A4**, et sa fiche
    // le dit en toutes lettres : c'est le contre-exemple qui empêche de
    // raisonner par classe.
    Profile {
        needs: &["rosuvastatine"],
        label: "Rosuvastatine",
        actions: &[
            Action::new(Cyp2c9, Substrate, Some(Weak)),
            Action::new(Oatp1b1, Substrate, Some(Strong)),
            Action::new(Bcrp, Substrate, None),
        ],
        source: "Crestor : « Contrairement à la simvastatine et à l'atorvastatine, la rosuvastatine n'est pas métabolisée par le CYP3A4 » ; « Métabolisme marginal par le CYP2C9 » ; Crestor : « majoritairement captée par le foie via le transporteur OATP1B1 » ; « substrat des transporteurs OATP1B1 et BCRP ».",
    },
    Profile {
        needs: &["apixaban"],
        label: "Apixaban",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Eliquis : « Métabolisme partiel par le CYP3A4, substrat de la P-gp » ; « substrat de la P-gp ».",
    },
    Profile {
        needs: &["rivaroxaban"],
        label: "Rivaroxaban",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Pgp, Substrate, None),
            Action::new(Bcrp, Substrate, None),
        ],
        source: "Xarelto : « Environ un tiers éliminé par voie rénale sous forme inchangée, le reste métabolisé (CYP3A4, CYP2J2) » ; « substrat de la P-gp et de la BCRP ».",
    },
    Profile {
        needs: &["ticagrelor"],
        label: "Ticagrélor",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inhibitor, None),
            Action::new(Pgp, Substrate, None),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Brilique : « Le ticagrélor est substrat et inhibiteur du CYP3A4 et de la P-gp » ; « Métabolisme hépatique par le CYP3A4 » ; la P-gp est dans la même phrase, sans force.",
    },
    // **Prodrogue** : inhiber le CYP2C19 ne l'accumule pas, cela le
    // désarme.
    Profile {
        needs: &["clopidogrel"],
        label: "Clopidogrel",
        actions: &[
            Action::prodrug(Cyp2c19, Some(Strong)),
            Action::new(Cyp2c8, Inhibitor, None),
        ],
        source: "Plavix : « Prodrogue transformée par les cytochromes hépatiques, principalement le CYP2C19, en un métabolite actif » ; « Le clopidogrel augmente l'exposition au répaglinide par inhibition du CYP2C8 ».",
    },
    Profile {
        needs: &["warfarine"],
        label: "Warfarine",
        actions: &[
            Action::new(Cyp2c9, Substrate, Some(Strong)),
            Action::new(Cyp1a2, Substrate, Some(Moderate)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
        ],
        source: "Coumadine : « l'énantiomère S le plus actif par le CYP2C9, l'énantiomère R par les CYP1A2 et CYP3A4 ».",
    },
    Profile {
        needs: &["acenocoumarol"],
        label: "Acénocoumarol",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Sintrom : « Métabolisme hépatique important, notamment par le CYP2C9 ».",
    },
    // **Prodrogue** aussi : la paroxétine ne majore pas le tramadol,
    // elle le rend inefficace.
    Profile {
        needs: &["tramadol"],
        label: "Tramadol",
        actions: &[
            Action::prodrug(Cyp2d6, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
        ],
        source: "Tramadol : « dont l'activité analgésique repose surtout sur le métabolite O-déméthylé M1 formé par le CYP2D6 » ; « Inhibiteurs du CYP2D6 comme la paroxétine et la fluoxétine : moindre production de M1 et perte d'efficacité ».",
    },
    Profile {
        needs: &["oxycodone"],
        label: "Oxycodone",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Moderate)),
        ],
        source: "Oxycontin : « Métabolisme hépatique par les CYP3A4 et CYP2D6 » ; « Inhibiteurs puissants du CYP3A4 : exposition majorée et risque de surdosage ».",
    },
    Profile {
        needs: &["fentanyl"],
        label: "Fentanyl",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Durogesic : « Métabolisme hépatique par le CYP3A4 en métabolites inactifs » ; « Inhibiteurs puissants du CYP3A4 : concentrations augmentées et risque de dépression respiratoire ».",
    },
    Profile {
        needs: &["alprazolam"],
        label: "Alprazolam",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Xanax : « Métabolisme hépatique par le CYP3A4 » ; « Les inhibiteurs puissants du CYP3A4 augmentent nettement les concentrations et la sédation ».",
    },
    Profile {
        needs: &["triazolam"],
        label: "Triazolam",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Halcion : « Métabolisme hépatique rapide par le CYP3A4 » ; « augmentation majeure de l'exposition et sédation prolongée, association contre-indiquée ».",
    },
    Profile {
        needs: &["bromazepam"],
        label: "Bromazépam",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Moderate))],
        source: "Lexomil : « Métabolisme hépatique, notamment par le CYP3A4 ».",
    },
    Profile {
        needs: &["clobazam"],
        label: "Clobazam",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
        ],
        source: "Urbanyl : « Le clobazam est métabolisé par les CYP3A4 et CYP2C19 » ; « Métabolisme hépatique par le CYP3A4 puis le CYP2C19 en N-desméthylclobazam actif ».",
    },
    Profile {
        needs: &["clonazepam"],
        label: "Clonazépam",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Rivotril : « Métabolisme hépatique par le CYP3A4 ».",
    },
    Profile {
        needs: &["zolpidem"],
        label: "Zolpidem",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Cyp1a2, Substrate, Some(Weak)),
            Action::new(Cyp2c9, Substrate, Some(Weak)),
        ],
        source: "Stilnox : « Métabolisme hépatique par les CYP3A4, CYP1A2 et CYP2C9 en métabolites inactifs ».",
    },
    Profile {
        needs: &["zopiclone"],
        label: "Zopiclone",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Cyp2c8, Substrate, Some(Weak)),
        ],
        source: "Imovane : « Métabolisme hépatique par le CYP3A4 et le CYP2C8 ».",
    },
    Profile {
        needs: &["tacrolimus"],
        label: "Tacrolimus",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Prograf : « Métabolisme hépatique et intestinal extensif par le CYP3A4 et le CYP3A5 » ; « élévation majeure et rapide » sous inhibiteur puissant.",
    },
    Profile {
        needs: &["everolimus"],
        label: "Évérolimus",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Certican : « Métabolisme hépatique et intestinal étendu par le CYP3A4 » ; « élévation majeure des concentrations et toxicité » ; Certican : « avec transport par la glycoprotéine P ».",
    },
    Profile {
        needs: &["sirolimus"],
        label: "Sirolimus",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Rapamune : « Substrat du CYP3A4 et de la glycoprotéine P, largement métabolisé dans l'intestin et le foie » ; la glycoprotéine P, sans force.",
    },
    Profile {
        needs: &["colchicine"],
        label: "Colchicine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Colchicine : « Substrat du CYP3A4 et de la P-gp » ; « concentrations fortement augmentées, réduction de dose impérative » ; Colchimax : « substrat de la glycoprotéine P ».",
    },
    Profile {
        needs: &["domperidone"],
        label: "Dompéridone",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Motilium : « Métabolisme hépatique important par le CYP3A4 » ; « concentrations et risque de torsades de pointes fortement augmentés ».",
    },
    Profile {
        needs: &["escitalopram"],
        label: "Escitalopram",
        actions: &[
            Action::new(Cyp2c19, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Weak)),
            Action::new(Cyp2d6, Substrate, Some(Weak)),
            Action::new(Cyp2d6, Inhibitor, Some(Weak)),
        ],
        source: "Seroplex : « Métabolisme hépatique principalement par le CYP2C19, accessoirement CYP3A4 et CYP2D6 » ; « Inhibiteur faible du CYP2D6 ».",
    },
    Profile {
        needs: &["aripiprazole"],
        label: "Aripiprazole",
        actions: &[
            Action::new(Cyp2d6, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
        ],
        source: "Abilify : « Métabolisme hépatique par les CYP2D6 et CYP3A4 » ; « Les inhibiteurs puissants du CYP2D6 et du CYP3A4 augmentent l'exposition et imposent une réduction de dose ».",
    },
    Profile {
        needs: &["risperidone"],
        label: "Rispéridone",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Risperdal : « Métabolisme hépatique par le CYP2D6 en 9-hydroxyrispéridone, métabolite actif ».",
    },
    Profile {
        needs: &["haloperidol"],
        label: "Halopéridol",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Strong)),
        ],
        source: "Haldol : « Métabolisme hépatique important par les CYP3A4 et CYP2D6 » ; « Les inhibiteurs du CYP2D6 et du CYP3A4 augmentent l'exposition ».",
    },
    // ---- Acteurs ajoutés en 0.200.0 ----
    Profile {
        needs: &["carbamazepine"],
        label: "Carbamazépine",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, Some(Strong)),
            Action::new(Cyp2b6, Inducer, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inducer, None),
        ],
        source: "Tégrétol : « Inducteur enzymatique puissant des CYP3A4, CYP2C9, CYP2B6 » ; « Les inhibiteurs du CYP3A4 augmentent la carbamazépinémie avec risque de surdosage » ; Tégrétol : « Il induit aussi la glycoprotéine P » — sans force.",
    },
    Profile {
        needs: &["oxcarbazepine"],
        label: "Oxcarbazépine",
        actions: &[Action::new(Cyp3a4, Inducer, None)],
        source: "Trileptal : « l'oxcarbazépine induit le CYP3A4 et réduit l'efficacité des estroprogestatifs » — la fiche ne qualifie pas la force.",
    },
    Profile {
        needs: &["enzalutamide", "xtandi"],
        label: "Enzalutamide",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Cyp2c19, Inducer, None),
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Xtandi : « L'enzalutamide est un inducteur puissant du CYP3A4 et un inducteur du CYP2C9 et du CYP2C19 » ; « Substrat du CYP2C8 et du CYP3A4 ».",
    },
    Profile {
        needs: &["gemfibrozil"],
        label: "Gemfibrozil",
        actions: &[
            Action::new(Cyp2c8, Inhibitor, Some(Strong)),
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Lipur : « Le gemfibrozil est un inhibiteur puissant du CYP2C8 et un inhibiteur du transporteur OATP1B1 : l'association au répaglinide est contre-indiquée » — le « puissant » ne porte que sur le CYP2C8.",
    },
    Profile {
        needs: &["aprepitant"],
        label: "Aprépitant",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
            Action::new(Cyp2c9, Inducer, None),
        ],
        source: "Emend : « Inhibition modérée du CYP3A4 » ; « Induction du CYP2C9 : diminution de l'effet des anti-vitamine K ».",
    },
    Profile {
        needs: &["modafinil"],
        label: "Modafinil",
        actions: &[
            Action::new(Cyp3a4, Inducer, None),
            Action::new(Cyp2c19, Inhibitor, None),
        ],
        source: "Modiodal : « Inducteur du CYP3A4 » ; « Inhibiteur du CYP2C19 : concentrations augmentées de diazépam, de phénytoïne, d'oméprazole ».",
    },
    Profile {
        needs: &["mirabegron"],
        label: "Mirabégron",
        actions: &[
            Action::new(Cyp2d6, Inhibitor, Some(Moderate)),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp2d6, Substrate, None),
        ],
        source: "Betmiga : « le mirabégron est un inhibiteur modéré du CYP2D6 » ; « Métabolisme multiple faisant intervenir la glucuronoconjugaison, les estérases, le CYP3A4 et le CYP2D6 ».",
    },
    Profile {
        needs: &["roxithromycine"],
        label: "Roxithromycine",
        actions: &[Action::new(Cyp3a4, Inhibitor, Some(Moderate))],
        source: "Rulid : « Inhibition du CYP3A4 plus modérée qu'avec la clarithromycine mais réelle ».",
    },
    Profile {
        needs: &["pristinamycine"],
        label: "Pristinamycine",
        actions: &[Action::new(Cyp3a4, Inhibitor, None)],
        source: "Pyostacine : « Inhibiteur du CYP3A4 : l'association à l'ergotamine et à la dihydroergotamine est contre-indiquée ».",
    },
    Profile {
        needs: &["isoniazide", "rimifon"],
        label: "Isoniazide",
        actions: &[
            Action::new(Cyp2c19, Inhibitor, None),
            Action::new(Cyp3a4, Inhibitor, None),
        ],
        source: "Rimifon : « Inhibiteur du CYP2C19 et du CYP3A4 : élévation des concentrations de carbamazépine, de phénytoïne, de diazépam ».",
    },
    // **L'hydroquinidine, et non « la quinidine »** : c'est la fiche que
    // le logiciel livre, et un motif « quinidine » l'attraperait sous un
    // autre nom que le sien.
    Profile {
        needs: &["hydroquinidine"],
        label: "Hydroquinidine",
        actions: &[
            Action::new(Cyp2d6, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Hydroquinidine : « Inhibiteur puissant du CYP2D6 : concentrations augmentées des bêta-bloquants métabolisés par cette voie, des antidépresseurs et de la codéine, dont l'efficacité antalgique est abolie ».",
    },
    Profile {
        needs: &["ticlopidine"],
        label: "Ticlopidine",
        actions: &[Action::new(Cyp2c19, Inhibitor, None)],
        source: "Ticlopidine : « La ticlopidine inhibe le CYP2C19 et augmente l'exposition à la phénytoïne et à certaines benzodiazépines ».",
    },
    // **Le rabéprazole freine faiblement, et sa fiche le chiffre** :
    // c'est ce qui le sépare de l'oméprazole devant un clopidogrel.
    Profile {
        needs: &["rabeprazole"],
        label: "Rabéprazole",
        actions: &[
            Action::new(Cyp2c19, Inhibitor, Some(Weak)),
            Action::new(Cyp2c19, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Pariet : « l'inhibition du CYP2C19 par le rabéprazole est faible et l'association est jugée acceptable, contrairement à l'oméprazole et à l'ésoméprazole ».",
    },
    // **Le pantoprazole n'est pas inhibiteur**, et c'est pour cela qu'on
    // le choisit sous clopidogrel. Il reste substrat.
    Profile {
        needs: &["pantoprazole"],
        label: "Pantoprazole",
        actions: &[
            Action::new(Cyp2c19, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Inipomp : « Le pantoprazole est le moins inhibiteur du CYP2C19 parmi les IPP, ce qui fonde sa place chez les patients sous clopidogrel » ; « Métabolisme hépatique par le CYP2C19 puis le CYP3A4 ».",
    },
    Profile {
        needs: &["imatinib"],
        label: "Imatinib",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inhibitor, None),
            Action::new(Cyp2c9, Inhibitor, None),
        ],
        source: "Glivec : « Substrat majeur du CYP3A4 » ; « L'imatinib inhibe le CYP3A4 et le CYP2C9 : majoration de l'effet des AVK ».",
    },
    Profile {
        needs: &["duloxetine"],
        label: "Duloxétine",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Inhibitor, Some(Moderate)),
        ],
        source: "Cymbalta : « Association contre-indiquée aux inhibiteurs puissants du CYP1A2, qui multiplient l'exposition » ; « Inhibiteur modéré du CYP2D6 ».",
    },
    // ---- Substrats ajoutés en 0.200.0 ----
    Profile {
        needs: &["ciclosporine"],
        label: "Ciclosporine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, None),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Néoral : « Métabolisme hépatique et intestinal extensif par le CYP3A4 avec efflux par la glycoprotéine P » ; jus de pamplemousse « formellement interdit » ; « La ciclosporine inhibe elle-même la glycoprotéine P et le transporteur OATP1B1 » — sans force.",
    },
    Profile {
        needs: &["quetiapine"],
        label: "Quétiapine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Xeroquel : « Métabolisme hépatique extensif, principalement par le CYP3A4 » ; « Les inhibiteurs puissants du CYP3A4 sont contre-indiqués ».",
    },
    Profile {
        needs: &["clozapine", "leponex"],
        label: "Clozapine",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Cyp2d6, Substrate, Some(Weak)),
        ],
        source: "Leponex : « Métabolisme hépatique presque complet par les CYP1A2 et CYP3A4, avec contribution du CYP2D6 » ; « Les inhibiteurs du CYP1A2 augmentent fortement la clozapinémie ».",
    },
    Profile {
        needs: &["theophylline", "euphylline"],
        label: "Théophylline",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Weak)),
        ],
        source: "Euphylline : « Métabolisme hépatique majoritaire, principalement par le CYP1A2 avec participation du CYP3A4 ».",
    },
    Profile {
        needs: &["melatonine"],
        label: "Mélatonine",
        actions: &[Action::new(Cyp1a2, Substrate, Some(Strong))],
        source: "Circadin : « La fluvoxamine, inhibiteur puissant du CYP1A2, augmente massivement l'exposition à la mélatonine ».",
    },
    Profile {
        needs: &["ropinirole"],
        label: "Ropinirole",
        actions: &[Action::new(Cyp1a2, Substrate, Some(Strong))],
        source: "Requip : « Métabolisme hépatique important par le CYP1A2 en métabolites inactifs ».",
    },
    Profile {
        needs: &["diazepam"],
        label: "Diazépam",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2c19, Substrate, Some(Strong)),
        ],
        source: "Valium : « Métabolisme hépatique par les CYP3A4 et CYP2C19 en métabolites actifs » ; « Les inhibiteurs du CYP3A4 et du CYP2C19 augmentent nettement l'exposition ».",
    },
    // **Prodrogue** : le losartan n'agit que par son métabolite, et
    // freiner le CYP2C9 ne l'accumule pas, cela le désarme.
    Profile {
        needs: &["losartan"],
        label: "Losartan",
        actions: &[
            Action::prodrug(Cyp2c9, None),
            Action::prodrug(Cyp3a4, None),
        ],
        source: "Cozaar : « Métabolisme hépatique de premier passage par les CYP2C9 et CYP3A4 vers le métabolite actif ».",
    },
    // **Prodrogue** : la paroxétine ne majore pas le tamoxifène, elle
    // le rend inefficace — et c'est un traitement du cancer du sein.
    Profile {
        needs: &["tamoxifene"],
        label: "Tamoxifène",
        actions: &[
            Action::prodrug(Cyp2d6, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Tamoxifène : « L'interaction majeure concerne le CYP2D6, qui transforme le tamoxifène en endoxifène, son métabolite actif » ; « leur association fait chuter les concentrations d'endoxifène et compromet l'efficacité antitumorale ».",
    },
    // **Prodrogue** : sans CYP2D6, pas de morphine, donc pas d'effet.
    Profile {
        needs: &["dihydrocodeine", "dicodin"],
        label: "Dihydrocodéine",
        actions: &[Action::new(Cyp2d6, Substrate, None)],
        source: "Dicodin LP : « substrat de l'enzyme polymorphe CYP2D6, qui la convertit en dihydromorphine » ; la portée clinique du polymorphisme n'est pas établie pour la dihydrocodéine.",
    },
    Profile {
        needs: &["codeine"],
        label: "Codéine",
        // **Seulement le CYP2D6.** La 3A4 déméthyle aussi la codéine,
        // mais la fiche ne le dit pas : le test l'a refusée, et il a eu
        // raison — une table qui en sait plus que la fiche se corrige
        // dans la fiche, pas ici.
        actions: &[Action::prodrug(Cyp2d6, Some(Strong))],
        source: "Néo-Codion : « la codéine, transformée en morphine par le CYP2D6 » ; « Inhibiteurs du CYP2D6 : conversion en morphine réduite, donc perte d'efficacité ».",
    },
    Profile {
        needs: &["dextromethorphane"],
        label: "Dextrométhorphane",
        actions: &[
            Action::new(Cyp2d6, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Tussidane : « principalement O-déméthylation en dextrorphane par le CYP2D6 avec participation du CYP3A4 » ; « Inhibiteurs puissants du CYP2D6 : augmentation importante de l'exposition ».",
    },
    Profile {
        needs: &["metoprolol"],
        label: "Métoprolol",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Lopressor : « Métabolisme hépatique presque complet par le CYP2D6 » ; « Inhibiteurs puissants du CYP2D6 : exposition fortement augmentée, bradycardie ».",
    },
    Profile {
        needs: &["carvedilol"],
        label: "Carvédilol",
        actions: &[
            Action::new(Cyp2d6, Substrate, Some(Strong)),
            Action::new(Cyp2c9, Substrate, None),
        ],
        source: "Kredex : « Métabolisme hépatique important, notamment par le CYP2D6 et le CYP2C9 ».",
    },
    Profile {
        needs: &["nebivolol"],
        label: "Nébivolol",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Temerit : « Métabolisme hépatique important par le CYP2D6, avec un polymorphisme génétique marqué ».",
    },
    Profile {
        needs: &["propafenone"],
        label: "Propafénone",
        actions: &[
            Action::new(Cyp2d6, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Rythmol : « Métabolisme hépatique important, saturable, principalement par le CYP2D6 » ; « Inhibiteurs du CYP2D6 et du CYP3A4 : concentrations fortement augmentées ».",
    },
    Profile {
        needs: &["flecainide"],
        label: "Flécaïnide",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Flécaïne : « Le flécaïnide est métabolisé par le CYP2D6 : la fluoxétine, la paroxétine, la quinidine, le bupropion et la terbinafine augmentent son exposition et exposent au surdosage ».",
    },
    Profile {
        needs: &["atomoxetine"],
        label: "Atomoxétine",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Strattera : « Métabolisme hépatique principalement par le CYP2D6 en 4-hydroxyatomoxétine ».",
    },
    Profile {
        needs: &["dronedarone"],
        label: "Dronédarone",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Multaq : « Métabolisme hépatique important par le CYP3A4 » ; « Les inhibiteurs puissants du CYP3A4 sont contre-indiqués » ; Multaq : « la dronédarone inhibant la P-gp » — sans force.",
    },
    Profile {
        needs: &["amiodarone"],
        label: "Amiodarone",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Cordarone : « Métabolisme hépatique important, notamment par le CYP3A4, en déséthylamiodarone active » ; Cordarone : « L'amiodarone inhibe la P-glycoprotéine » — sans force.",
    },
    Profile {
        needs: &["ivabradine"],
        label: "Ivabradine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Procoralan : « Métabolisme hépatique et intestinal exclusivement par le CYP3A4 » ; « Contre-indication avec les inhibiteurs puissants du CYP3A4 ».",
    },
    Profile {
        needs: &["eplerenone"],
        label: "Éplérénone",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Inspra : « Métabolisme hépatique par le CYP3A4 » ; « Les inhibiteurs puissants du CYP3A4 sont contre-indiqués ».",
    },
    Profile {
        needs: &["repaglinide"],
        label: "Répaglinide",
        actions: &[
            Action::new(Cyp2c8, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Oatp1b1, Substrate, None),
        ],
        source: "Novonorm : « Métabolisme hépatique complet par les CYP2C8 et CYP3A4 » ; « Gemfibrozil : contre-indication absolue, l'inhibition du CYP2C8 multipliant l'exposition avec des hypoglycémies sévères » ; Novonorm : « Captation hépatique par le transporteur OATP1B1 ».",
    },
    Profile {
        needs: &["gliclazide"],
        label: "Gliclazide",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Diamicron : « Métabolisme hépatique extensif, principalement par le CYP2C9 ».",
    },
    Profile {
        needs: &["glimepiride"],
        label: "Glimépiride",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Amarel : « Métabolisme hépatique complet par le CYP2C9 en deux métabolites dont l'un est faiblement actif ».",
    },
    Profile {
        needs: &["celecoxib"],
        label: "Célécoxib",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Celebrex : « Il est métabolisé par le CYP2C9 » ; « fluconazole, qui double l'exposition et impose de commencer à demi-dose ».",
    },
    Profile {
        needs: &["diclofenac"],
        label: "Diclofénac",
        actions: &[
            Action::new(Cyp2c9, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Voltarène : « Métabolisme hépatique par les CYP2C9 et CYP3A4 ».",
    },
    Profile {
        needs: &["ibuprofene"],
        label: "Ibuprofène",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Nurofen : « Métabolisme hépatique par le CYP2C9 puis élimination urinaire des métabolites ».",
    },
    Profile {
        needs: &["montelukast"],
        label: "Montélukast",
        actions: &[
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp2c9, Substrate, None),
        ],
        source: "Singulair : « Métabolisme hépatique important par les CYP2C8, CYP3A4 et CYP2C9 ».",
    },
    Profile {
        needs: &["fluvastatine"],
        label: "Fluvastatine",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Fluvastatine : « Métabolisme hépatique important, essentiellement par le CYP2C9 » ; « la fluconazole et les autres inhibiteurs puissants du CYP2C9 augmentent son exposition ».",
    },
    Profile {
        needs: &["sildenafil"],
        label: "Sildénafil",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2c9, Substrate, None),
        ],
        source: "Viagra : « Métabolisme hépatique par les CYP3A4 et CYP2C9 » ; « Les inhibiteurs puissants du CYP3A4 augmentent nettement l'exposition ».",
    },
    Profile {
        needs: &["tadalafil"],
        label: "Tadalafil",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Cialis : « Métabolisme hépatique prédominant par le CYP3A4 » ; « Les inhibiteurs puissants du CYP3A4 augmentent fortement l'exposition ».",
    },
    Profile {
        needs: &["donepezil"],
        label: "Donépézil",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp2d6, Substrate, None),
        ],
        source: "Aricept : « Métabolisme hépatique par les CYP3A4 et CYP2D6 et glucuronoconjugaison ».",
    },
    Profile {
        needs: &["galantamine"],
        label: "Galantamine",
        actions: &[
            Action::new(Cyp2d6, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Reminyl : « Métabolisme hépatique par les CYP2D6 et CYP3A4 ».",
    },
    Profile {
        needs: &["oxybutynine"],
        label: "Oxybutynine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Ditropan : « Métabolisme hépatique important par le CYP3A4, avec un métabolite actif ».",
    },
    Profile {
        needs: &["solifenacine"],
        label: "Solifénacine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Vesicare : « Métabolisme hépatique principalement par le CYP3A4 » ; les inhibiteurs puissants « imposent de limiter la dose à 5 mg par jour ».",
    },
    Profile {
        needs: &["alfuzosine"],
        label: "Alfuzosine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Xatral : « Métabolisme hépatique important par le CYP3A4 » ; les inhibiteurs puissants « sont contre-indiqués ou déconseillés ».",
    },
    Profile {
        needs: &["amlodipine"],
        label: "Amlodipine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Amlor : « Métabolisme hépatique extensif par le CYP3A4 en métabolites inactifs ».",
    },
    Profile {
        needs: &["hydroxyzine"],
        label: "Hydroxyzine",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Atarax : « Métabolisme hépatique important, notamment par l'alcool déshydrogénase et le CYP3A4 ».",
    },
    Profile {
        needs: &["mefloquine"],
        label: "Méfloquine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Lariam : « Métabolisme hépatique important par le CYP3A4 en métabolites inactifs ».",
    },
    Profile {
        needs: &["ondansetron"],
        label: "Ondansétron",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp1a2, Substrate, None),
            Action::new(Cyp2d6, Substrate, None),
        ],
        source: "Zophren : « Métabolisme hépatique étendu par les CYP3A4, CYP1A2 et CYP2D6 ».",
    },
    // **Avant l'imipramine, et c'est tout l'intérêt de l'ordre.**
    // « imipramine » est une sous-chaîne de « trimipramine » : sans
    // cette ligne-ci en premier, le Surmontil recevait le profil du
    // Tofranil — CYP2D6, 1A2 et 3A4 — alors que sa propre fiche écrit
    // « principalement par le CYP2D6 ». Même piège que
    // « esomeprazole »/« omeprazole », et que « actiskenan »/« skenan »
    // dans `crush.rs`.
    Profile {
        needs: &["trimipramine"],
        label: "Trimipramine",
        actions: &[Action::new(Cyp2d6, Substrate, None)],
        source: "Surmontil : « Métabolisme hépatique extensif, principalement par le CYP2D6, avec formation de métabolites dont la desméthyltrimipramine ».",
    },
    Profile {
        needs: &["imipramine"],
        label: "Imipramine",
        actions: &[
            Action::new(Cyp2d6, Substrate, None),
            Action::new(Cyp1a2, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Tofranil : « Métabolisme hépatique par les CYP2D6, CYP1A2 et CYP3A4 en désipramine active ».",
    },
    Profile {
        needs: &["buspirone"],
        label: "Buspirone",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Buspirone : « Métabolisme hépatique important par le CYP3A4 avec un effet de premier passage majeur » ; les inhibiteurs puissants donnent une « exposition fortement augmentée ».",
    },
    Profile {
        needs: &["buprenorphine", "subutex"],
        label: "Buprénorphine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Subutex : « Métabolisme hépatique par le CYP3A4 en norbuprénorphine » ; « Inhibiteurs puissants du CYP3A4 : exposition augmentée ».",
    },
    Profile {
        needs: &["methadone"],
        label: "Méthadone",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2b6, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, None),
        ],
        source: "Méthadone : « Métabolisme hépatique important, principalement par les CYP3A4, CYP2B6 et CYP2D6 ».",
    },
    // **Aucun cytochrome, et c'est la réponse** qu'on cherche en se
    // demandant par quoi remplacer une simvastatine sous clarithromycine
    // — la fiche l'écrit. Elle fut la ligne « sans voie » de la table
    // jusqu'à ce que les transporteurs y entrent : sa captation par
    // l'OATP1B1 est ce qui la fait croiser la ciclosporine, et la taire
    // la rendait aussi sûre sous ciclosporine que sous clarithromycine.
    Profile {
        needs: &["pravastatine"],
        label: "Pravastatine",
        actions: &[Action::new(Oatp1b1, Substrate, None)],
        source: "Vasten : « elle n'est pas métabolisée par le CYP3A4 » ; « Substrat des transporteurs hépatiques OATP1B1 » ; Elisor : « la pravastatine n'étant pas métabolisée de façon notable par le CYP3A4 : ni le pamplemousse ni les macrolides ni les azolés ne posent le problème observé avec la simvastatine ».",
    },
    Profile {
        needs: &["olanzapine"],
        label: "Olanzapine",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Weak)),
        ],
        source: "Zyprexa : « Métabolisme hépatique par glucuroconjugaison directe et oxydation par le CYP1A2, accessoirement par le CYP2D6 ».",
    },
    Profile {
        needs: &["cabozantinib", "cabometyx"],
        label: "Cabozantinib",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Cabometyx : « Substrat du CYP3A4 : les inhibiteurs puissants (kétoconazole, itraconazole, voriconazole, clarithromycine, ritonavir) et le pamplemousse augmentent l'exposition ».",
    },
    Profile {
        needs: &["acalabrutinib", "calquence"],
        label: "Acalabrutinib",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Mate1, Inhibitor, None),
        ],
        source: "Calquence : « Substrat du CYP3A4 : les inhibiteurs puissants (itraconazole, voriconazole, posaconazole, clarithromycine, ritonavir) sont à éviter » ; « Metformine : surveillance, inhibition de MATE1 ».",
    },
    Profile {
        needs: &["ibrutinib", "imbruvica"],
        label: "Ibrutinib",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Imbruvica : « Substrat majeur du CYP3A4 : les inhibiteurs puissants (kétoconazole, itraconazole, posaconazole, voriconazole, clarithromycine, ritonavir et cobicistat) multiplient l'exposition » ; « Inhibiteur de la P-gp et de la BCRP » — sans force.",
    },
    Profile {
        needs: &["ruxolitinib", "jakavi"],
        label: "Ruxolitinib",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2c9, Substrate, None),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Jakavi : « Substrat majeur du CYP3A4, accessoirement du CYP2C9 » ; « Le ruxolitinib peut inhiber la glycoprotéine P et la BCRP intestinales ».",
    },
    Profile {
        needs: &["eltrombopag", "revolade"],
        label: "Eltrombopag",
        actions: &[
            Action::new(Cyp1a2, Substrate, None),
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Oatp1b1, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Revolade : « Métabolisé par le CYP1A2, le CYP2C8 et la glucuroconjugaison, avec peu de conséquences décrites » ; « Inhibiteur de l'OATP1B1 et de la BCRP » — sans force.",
    },
    Profile {
        needs: &["regorafenib", "stivarga"],
        label: "Régorafénib",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Stivarga : « Substrat du CYP3A4 et de l'UGT1A9 : les inhibiteurs puissants du CYP3A4 (kétoconazole, itraconazole, voriconazole, clarithromycine, ritonavir) et le pamplemousse modifient l'exposition » ; le régorafénib inhibe « la BCRP, ce qui augmente celle de la rosuvastatine ».",
    },
    Profile {
        needs: &["venetoclax", "venclyxto"],
        label: "Vénétoclax",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, Some(Strong)),
            Action::new(Bcrp, Substrate, None),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Venclyxto : « le vénétoclax inhibe lui-même la P-gp et la BCRP » ; « Substrat majeur du CYP3A4 et de la P-gp » ; « association aux inhibiteurs puissants du CYP3A4 contre-indiquée à l'instauration et pendant toute la phase de titration » ; « Substrat majeur du CYP3A4 et de la P-gp » ; « substrat de la P-gp et de la BCRP ».",
    },
    Profile {
        needs: &["anagrelide", "xagrid"],
        label: "Anagrélide",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp1a2, Inhibitor, Some(Weak)),
        ],
        source: "Xagrid : « Métabolisé principalement par le CYP1A2 » ; « L'anagrélide exerce une faible inhibition du CYP1A2, avec un effet possible sur la théophylline ».",
    },
    Profile {
        needs: &["riociguat", "adempas"],
        label: "Riociguat",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Pgp, Substrate, None),
            Action::new(Bcrp, Substrate, None),
        ],
        source: "Adempas : « Métabolisme par plusieurs cytochromes, dont les CYP1A1, CYP3A4, CYP3A5 et CYP2J2 » ; « Inhibiteurs puissants à plusieurs voies, du CYP3A4, de la P-gp et de la BCRP […] : exposition fortement augmentée ».",
    },
    Profile {
        needs: &["macitentan", "opsumit"],
        label: "Macitentan",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2c8, Substrate, Some(Weak)),
            Action::new(Cyp2c9, Substrate, Some(Moderate)),
            Action::new(Cyp2c19, Substrate, Some(Weak)),
        ],
        source: "Opsumit : « le métabolite actif, l'aprocitentan, est formé par dépropylation, principalement par le CYP3A4 et accessoirement par les CYP2C8, CYP2C9 et CYP2C19 ; les autres voies, qui donnent des métabolites inactifs, dépendent surtout du CYP2C9 ».",
    },
    Profile {
        needs: &["ranolazine"],
        label: "Ranolazine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Weak)),
            Action::new(Cyp3a4, Inhibitor, Some(Weak)),
            Action::new(Cyp2d6, Inhibitor, Some(Weak)),
            Action::new(Pgp, Substrate, None),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Oct2, Inhibitor, None),
        ],
        source: "Ranexa : « Métabolisme hépatique rapide et étendu, principalement par le CYP3A4 et accessoirement par le CYP2D6 » ; « La ranolazine est elle-même un inhibiteur faible du CYP3A4 et du CYP2D6, et inhibe la P-gp » ; « substrat […] de la P-gp » ; « inhibe la P-gp » — sans force pour la P-gp ; « Elle augmente l'exposition à la metformine par inhibition de l'OCT2 ».",
    },
    Profile {
        needs: &["disopyramide", "rythmodan"],
        label: "Disopyramide",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Moderate))],
        source: "Rythmodan : « métabolisme hépatique partiel par le CYP3A4 en un métabolite N-désalkylé ».",
    },
    Profile {
        needs: &["aceclofenac", "cartrex"],
        label: "Acéclofénac",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Strong))],
        source: "Cartrex : « Métabolisme hépatique principalement par le CYP2C9 en 4'-hydroxyacéclofénac ».",
    },
    Profile {
        needs: &["guanfacine", "intuniv"],
        label: "Guanfacine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Intuniv : « Métabolisme hépatique principalement par le CYP3A4, d'où l'effet majeur de ses inhibiteurs et de ses inducteurs ».",
    },
    Profile {
        needs: &["siponimod", "mayzent"],
        label: "Siponimod",
        actions: &[
            Action::new(Cyp2c9, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Weak)),
        ],
        source: "Mayzent : « Métabolisme hépatique principalement par le CYP2C9, pour environ quatre cinquièmes ; le reste par d'autres cytochromes, dont le CYP3A4 pour environ 6 %, voies mineures ».",
    },
    Profile {
        needs: &["meloxicam", "mobic"],
        label: "Méloxicam",
        actions: &[
            Action::new(Cyp2c9, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Weak)),
        ],
        source: "Mobic : « Métabolisme hépatique quasi complet, principalement par le CYP2C9 et accessoirement par le CYP3A4 ».",
    },
    Profile {
        needs: &["naloxegol", "moventig"],
        label: "Naloxégol",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Moventig : « Métabolisme hépatique principalement par le CYP3A4 » ; « Inhibiteurs puissants du CYP3A4 : exposition au naloxégol multipliée, association contre-indiquée » ; « substrat de la glycoprotéine P ».",
    },
    Profile {
        needs: &["pimozide"],
        label: "Pimozide",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Moderate)),
        ],
        source: "Orap : « Métabolisme hépatique important par N-désalkylation, principalement par le CYP3A4 et le CYP2D6, en métabolites sans activité antipsychotique ».",
    },
    Profile {
        needs: &["daridorexant", "quviviq"],
        label: "Daridorexant",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inhibitor, Some(Weak)),
            Action::new(Pgp, Inhibitor, Some(Weak)),
        ],
        source: "Quviviq : « Métabolisme hépatique très majoritairement par le CYP3A4 » ; « le daridorexant est lui-même un inhibiteur faible du CYP3A4 et de la glycoprotéine P » ; « Inhibiteurs puissants du CYP3A4, clarithromycine, itraconazole, kétoconazole, ritonavir : exposition fortement augmentée, association contre-indiquée ».",
    },
    Profile {
        needs: &["esketamine", "spravato"],
        label: "Eskétamine",
        actions: &[
            Action::new(Cyp2b6, Substrate, Some(Moderate)),
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
        ],
        source: "Spravato : « Métabolisme hépatique principalement par les CYP2B6 et CYP3A4 en noreskétamine, accessoirement par les CYP2C9 et CYP2C19 ».",
    },
    Profile {
        needs: &["eslicarbazepine", "zebinix"],
        label: "Eslicarbazépine",
        actions: &[
            Action::new(Cyp3a4, Inducer, None),
            Action::new(Cyp2c19, Inhibitor, Some(Weak)),
        ],
        source: "Zebinix : « Induction du CYP3A4 et inhibition faible du CYP2C19, à l'origine de ses interactions ».",
    },
    Profile {
        needs: &["tolterodine", "detrusitol"],
        label: "Toltérodine",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Détrusitol : « Inhibiteurs puissants du CYP3A4 par voie générale […] association déconseillée, l'exposition augmentant chez les métaboliseurs lents du CYP2D6 » ; « La fluoxétine, inhibiteur puissant du CYP2D6, ne donne pas d'interaction cliniquement significative ».",
    },
    Profile {
        needs: &["vardenafil", "levitra"],
        label: "Vardénafil",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Lévitra : « Métabolisme hépatique principalement par le CYP3A4 » ; « le ritonavir multiplie l'exposition par 49 et l'indinavir par 16 ».",
    },
    Profile {
        needs: &["tibolone", "livial"],
        label: "Tibolone",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Livial : « Inducteurs du CYP3A4, barbituriques, carbamazépine, hydantoïnes, rifampicine, et millepertuis : métabolisme accéléré, effet diminué et saignements modifiés ».",
    },
    Profile {
        needs: &["dapoxetine", "priligy"],
        label: "Dapoxétine",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp2d6, Substrate, None),
            Action::new(Cyp2d6, Inhibitor, Some(Weak)),
        ],
        source: "Priligy : « Métabolisme hépatique et rénal par le CYP2D6, le CYP3A4 et la flavine mono-oxygénase FMO1 » ; « Effet inhibiteur faible sur le CYP2D6, l'exposition à la désipramine n'augmentant que de 19 % ».",
    },
    Profile {
        needs: &["relugolix", "ryeqo"],
        label: "Rélugolix + estradiol + noréthistérone",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Ryeqo : « Inducteurs puissants du CYP3A4 ou de la glycoprotéine P […] non recommandés, efficacité diminuée et protection osseuse réduite » ; « Les inhibiteurs du CYP3A4 peuvent augmenter l'estradiol et la noréthistérone » ; « substrat de la glycoprotéine P ».",
    },
    Profile {
        needs: &["dienogest", "visanne"],
        label: "Diénogest",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Visanne : « Le diénogest est métabolisé principalement par le CYP3A4 » ; « le kétoconazole multiplie l'exposition par 2,9 et l'érythromycine par 1,6 ».",
    },
    Profile {
        needs: &["rilpivirine", "edurant"],
        label: "Rilpivirine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Edurant : « Métabolisée principalement par le CYP3A4 » ; les inducteurs du CYP3A sont contre-indiqués par perte d'efficacité ; Edurant : « Dabigatran : prudence, par inhibition de la P-gp intestinale ».",
    },
    Profile {
        needs: &["abrocitinib", "cibinqo"],
        label: "Abrocitinib",
        actions: &[
            Action::new(Cyp2c19, Substrate, None),
            Action::new(Cyp2c9, Substrate, None),
            Action::new(Cyp2c19, Inhibitor, Some(Moderate)),
            Action::new(Cyp1a2, Inhibitor, Some(Weak)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Cibinqo : « Substrat principal des CYP2C19 et CYP2C9 » ; « L'abrocitinib est lui-même un inhibiteur modéré du CYP2C19 » ; « Inhibiteur faible du CYP1A2 » ; « Inhibiteur de la glycoprotéine P : prudence avec le dabigatran et la digoxine ».",
    },
    Profile {
        needs: &["roflumilast", "daxas"],
        label: "Roflumilast",
        actions: &[
            Action::new(Cyp1a2, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Daxas : « Métabolisme hépatique étendu, par les CYP1A2 et CYP3A4, en roflumilast-N-oxyde actif » — la fiche ne qualifie pas la force.",
    },
    Profile {
        needs: &["apalutamide", "erleada"],
        label: "Apalutamide",
        actions: &[
            Action::new(Cyp3a4, Inducer, Some(Strong)),
            Action::new(Cyp2c19, Inducer, Some(Strong)),
            Action::new(Cyp2c9, Inducer, Some(Weak)),
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Pgp, Inducer, Some(Weak)),
            Action::new(Bcrp, Inducer, Some(Weak)),
            Action::new(Oatp1b1, Inducer, Some(Weak)),
        ],
        source: "Erleada : « Inducteur puissant des CYP3A4 et CYP2C19 et inducteur faible du CYP2C9 » ; « Substrat des CYP2C8 et CYP3A4 » ; « inducteur faible de la glycoprotéine P, de la BCRP et de l'OATP1B1 ».",
    },
    Profile {
        needs: &["darolutamide", "nubeqa"],
        label: "Darolutamide",
        actions: &[
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp3a4, Inducer, Some(Weak)),
            Action::new(Pgp, Substrate, None),
            Action::new(Bcrp, Substrate, None),
            Action::new(Bcrp, Inhibitor, None),
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Nubeqa : « Substrat du CYP3A4, de la glycoprotéine P et de la BCRP » ; « Inducteur faible du CYP3A4, sans conséquence clinique attendue » ; « Substrat du CYP3A4, de la glycoprotéine P et de la BCRP » ; « Inhibiteur de la BCRP et des OATP1B1 et OATP1B3 : exposition à la rosuvastatine multipliée par 5 environ ».",
    },
    Profile {
        needs: &["trixeo"],
        label: "Budésonide inhalé (Trixeo)",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Trixeo Aerosphere : « Budésonide métabolisé principalement par le CYP3A4 » ; « Inhibiteurs puissants du CYP3A (itraconazole, kétoconazole, inhibiteurs de protéase du VIH, produits contenant du cobicistat) : exposition au budésonide augmentée ».",
    },
    Profile {
        needs: &["ritlecitinib", "litfulo"],
        label: "Ritlécitinib",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
            Action::new(Cyp1a2, Inhibitor, Some(Moderate)),
        ],
        source: "Litfulo : « Le ritlécitinib est un inhibiteur modéré du CYP3A4 et du CYP1A2 » .",
    },
    Profile {
        needs: &["alitretinoine", "toctino"],
        label: "Alitrétinoïne",
        actions: &[
            Action::new(Cyp2c9, Substrate, None),
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp2c8, Inhibitor, None),
        ],
        source: "Toctino : « L'alitrétinoïne est métabolisée par les CYP2C9, CYP2C8 et CYP3A4 » ; « Elle augmente l'exposition aux substrats du CYP2C8 » .",
    },
    Profile {
        needs: &["rufinamide", "inovelon"],
        label: "Rufinamide",
        actions: &[Action::new(Cyp3a4, Inducer, Some(Weak))],
        source: "Inovelon : « Le rufinamide est un inducteur léger à modéré du CYP3A4 » .",
    },
    Profile {
        needs: &["maprotiline", "ludiomil"],
        label: "Maprotiline",
        actions: &[
            Action::new(Cyp2d6, Substrate, Some(Strong)),
            Action::new(Cyp1a2, Substrate, Some(Weak)),
        ],
        source: "Ludiomil : « Métabolisme hépatique de plus de 95 % de la dose, principalement par le CYP2D6 et accessoirement par le CYP1A2 » ; « exposition augmentée d'environ 270 % chez le métaboliseur lent du CYP2D6 » .",
    },
    Profile {
        needs: &["tetrabenazine", "xenazine"],
        label: "Tétrabénazine",
        actions: &[Action::new(Cyp2d6, Substrate, Some(Strong))],
        source: "Xenazine : « Inhibiteurs puissants du CYP2D6 (fluoxétine, paroxétine, quinidine) : exposition aux métabolites actifs multipliée par 3 à 9 avec la paroxétine » .",
    },
    // **Avant la ligne du Dovato** : ces deux fiches écrivent l'OCT2 que le
    // dolutégravir inhibe — la metformine qui monte —, celles du Dovato
    // et du Juluca ne l'écrivent pas, et la ligne commune le leur
    // prêterait.
    Profile {
        needs: &["tivicay", "triumeq"],
        label: "Dolutégravir (Tivicay, Triumeq)",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Weak)),
            Action::new(Oct2, Inhibitor, None),
        ],
        source: "Tivicay : « Metformine : exposition augmentée par inhibition de l'OCT2, réduction de dose nécessaire » ; Triumeq : « Metformine : concentrations augmentées par inhibition du transporteur OCT2, adaptation de dose nécessaire » — le métabolisme accessoire par le CYP3A4 est celui de la ligne du Dovato.",
    },
    Profile {
        needs: &["dolutégravir", "dovato"],
        label: "Dolutégravir",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Weak))],
        source: "Dovato : « Dolutégravir métabolisé principalement par glucuronoconjugaison via l'UGT1A1, accessoirement par l'UGT1A3, l'UGT1A9 et le CYP3A4 » ; les inducteurs imposent un comprimé de dolutégravir supplémentaire.",
    },
    Profile {
        needs: &["pipéraquine", "eurartesim"],
        label: "Pipéraquine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Cyp3a4, Inhibitor, Some(Weak)),
        ],
        source: "Eurartesim : « La pipéraquine est métabolisée essentiellement par le CYP3A4 » ; « elle est inhibitrice faible du CYP3A4 » ; les inhibiteurs puissants « augmentent jusqu'à deux fois son exposition ».",
    },
    Profile {
        needs: &["elvitégravir", "cobicistat", "genvoya"],
        label: "Elvitégravir + cobicistat",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Inhibitor, Some(Weak)),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Pgp, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Genvoya : « puissant inhibiteur du CYP3A4 » ; « Il inhibe aussi faiblement le CYP2D6 » ; « L'elvitégravir peut induire le CYP2C9 et les UGT » ; « Elvitégravir métabolisé principalement par le CYP3A4 » ; « Il inhibe la P-gp, la BCRP et les OATP1B1/1B3 » — sans force, le « faiblement » de la fiche ne portant que sur le CYP2D6.",
    },
    Profile {
        needs: &["doravirine", "pifeltro"],
        label: "Doravirine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inducer, Some(Weak)),
        ],
        source: "Pifeltro : « Métabolisme oxydatif principalement par le CYP3A4 » ; « La doravirine pourrait être un faible inducteur du CYP3A4 (midazolam diminué de 18 %) ».",
    },
    Profile {
        needs: &["letermovir", "prevymis"],
        label: "Letermovir",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Moderate)),
            Action::new(Cyp2c19, Inducer, None),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Oatp1b1, Inhibitor, None),
            Action::new(Pgp, Inducer, None),
            Action::new(Oatp1b1, Substrate, None),
            Action::new(Pgp, Substrate, None),
            Action::new(Bcrp, Substrate, None),
        ],
        source: "Prevymis : « Inhibiteur modéré du CYP3A4 (midazolam multiplié par 2 à 3) » ; « Inducteur du CYP2C19 et probablement du CYP2C9 : voriconazole diminué » ; « Inhibiteur des OATP1B1/3 » ; « Inducteur de la P-gp intestinale : dabigatran diminué » ; « Captation hépatique par les transporteurs OATP1B1/3 » ; « Substrat de la P-gp et de la BCRP ».",
    },
    Profile {
        needs: &["cabotégravir", "vocabria"],
        label: "Cabotégravir",
        actions: &[],
        source: "Vocabria : « Métabolisé principalement par glucuronoconjugaison via l'UGT1A1, accessoirement l'UGT1A9, sans participation des cytochromes » ; « n'a pas modifié le midazolam ».",
    },
    Profile {
        needs: &["drospirenone"],
        label: "Drospirénone",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Slinda : « avec une part d'oxydation par le CYP3A4 » ; « exposition à la drospirénone augmentée, multipliée par 2,3 à 2,7 sous kétoconazole ».",
    },
    Profile {
        needs: &["zoely"],
        label: "Nomégestrol (Zoely)",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Zoely : « Nomégestrol acétate métabolisé par les cytochromes hépatiques, principalement CYP3A4 et CYP3A5 » ; « Inhibiteurs puissants du CYP3A4 : concentrations de nomégestrol acétate possiblement augmentées ».",
    },
    Profile {
        needs: &["lercanidipine"],
        label: "Lercanidipine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Zanextra : « Lercanidipine métabolisée par le CYP3A4 en métabolites inactifs » ; « contre-indiqués, exposition à la lercanidipine fortement accrue ».",
    },
    Profile {
        needs: &["etrasimod", "velsipity"],
        label: "Étrasimod",
        actions: &[
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Cyp2c9, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Velsipity : « Métabolisme hépatique extensif par le CYP2C8 pour 38 %, le CYP2C9 pour 37 % et le CYP3A4 pour 22 % » ; « comme le fluconazole qui augmente l'exposition de 84 % ».",
    },
    Profile {
        needs: &["manidipine", "iperten"],
        label: "Manidipine",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Iperten : « Métabolisme hépatique intense, vraisemblablement par le CYP3A4 ».",
    },
    Profile {
        needs: &["mizolastine", "mizollen"],
        label: "Mizolastine",
        actions: &[Action::new(Cyp3a4, Substrate, None)],
        source: "Mizollen : « une voie accessoire par le CYP3A4 forme des métabolites hydroxylés » ; « Kétoconazole et érythromycine par voie systémique : concentrations de mizolastine modérément augmentées ».",
    },
    Profile {
        needs: &["naldemedine", "rizmoic"],
        label: "Naldémédine",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Moderate)),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Rizmoic : « Métabolisme principalement par le CYP3A, en pratique le CYP3A4, en nor-naldémédine » ; « exposition augmentée, près de trois fois avec l'itraconazole » ; « substrat de la glycoprotéine P ».",
    },
    Profile {
        needs: &["pitolisant", "wakix"],
        label: "Pitolisant",
        actions: &[
            Action::new(Cyp2d6, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Cyp3a4, Inducer, None),
            Action::new(Cyp2b6, Inducer, None),
        ],
        source: "Wakix : « Métabolisme hépatique par les CYP3A4 et CYP2D6 » ; « induit les CYP3A4, CYP1A2 et CYP2B6 ; des interactions cliniquement pertinentes sont possibles avec les substrats des CYP3A4 et CYP2B6 ».",
    },
    // ---- Transporteurs, ajoutés en 0.358.0 ----
    // Des molécules que la table taisait parce qu'aucun cytochrome ne
    // les porte : c'est la glycoprotéine P ou l'OATP1B1 qui fait leur
    // interaction, et elle est majeure.
    Profile {
        needs: &["dabigatran"],
        label: "Dabigatran",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Pradaxa : « Substrat de la P-gp sans métabolisme par les cytochromes : dronédarone, kétoconazole, itraconazole et ciclosporine sont contre-indiqués ».",
    },
    Profile {
        needs: &["digoxine"],
        label: "Digoxine",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Digoxine : « substrat de la P-glycoprotéine, métabolisme hépatique très faible ».",
    },
    Profile {
        needs: &["edoxaban"],
        label: "Édoxaban",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Lixiana : « substrat de la P-gp, métabolisme par le CYP3A4 minime » ; « Inhibiteurs de la P-gp : réduction de dose à 30 mg ».",
    },
    Profile {
        needs: &["loperamide"],
        label: "Lopéramide",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Imodium : « l'expulsion active par la P-glycoprotéine à la barrière hématoencéphalique » ; « Inhibiteurs de la P-glycoprotéine […] : passage cérébral et exposition systémique augmentés ».",
    },
    Profile {
        needs: &["bilastine"],
        label: "Bilastine",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Inorial : « Substrat de la glycoprotéine P ».",
    },
    Profile {
        needs: &["stromectol"],
        label: "Ivermectine (Stromectol)",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Stromectol : « inhibiteurs puissants de la glycoprotéine P […] qui peuvent augmenter le passage cérébral ».",
    },
    Profile {
        needs: &["pomalidomide"],
        label: "Pomalidomide",
        actions: &[
            Action::new(Cyp1a2, Substrate, None),
            Action::new(Cyp3a4, Substrate, None),
            Action::new(Pgp, Substrate, None),
        ],
        source: "Imnovid : « Substrat du CYP1A2 et du CYP3A4 et de la P-gp ».",
    },
    Profile {
        needs: &["lenalidomide"],
        label: "Lénalidomide",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Revlimid : « il est substrat de la P-gp ».",
    },
    Profile {
        needs: &["nintedanib"],
        label: "Nintédanib",
        actions: &[
            Action::new(Pgp, Substrate, None),
            Action::new(Cyp3a4, Substrate, Some(Weak)),
        ],
        source: "Ofev : « Substrat de la P-glycoprotéine et, accessoirement, du CYP3A4 ».",
    },
    Profile {
        needs: &["fidaxomicine"],
        label: "Fidaxomicine",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Dificlir : « La fidaxomicine est un substrat de la glycoprotéine P » — l'inhibition intestinale qu'elle exerce n'est pas chiffrée par la fiche et n'est pas reprise.",
    },
    Profile {
        needs: &["alafenamide"],
        label: "Ténofovir alafénamide",
        actions: &[
            Action::new(Pgp, Substrate, None),
        ],
        source: "Vemlidy : « Inducteurs puissants de la glycoprotéine P : association contre-indiquée ou déconseillée du fait de la perte d'efficacité » ; « Inhibiteurs de la glycoprotéine P […] : exposition augmentée ».",
    },
    Profile {
        needs: &["tolvaptan"],
        label: "Tolvaptan",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Jinarc : « Métabolisme hépatique quasi exclusif par le CYP3A4 » ; « Le tolvaptan inhibe la glycoprotéine P, majorant l'exposition à la digoxine ».",
    },
    Profile {
        needs: &["osimertinib"],
        label: "Osimertinib",
        actions: &[
            Action::new(Pgp, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Tagrisso : « L'osimertinib est un inhibiteur de la BCRP et de la P-glycoprotéine ».",
    },
    Profile {
        needs: &["olaparib"],
        label: "Olaparib",
        actions: &[
            Action::new(Pgp, Inhibitor, Some(Weak)),
            Action::new(Bcrp, Inhibitor, Some(Weak)),
        ],
        source: "Lynparza : « L'olaparib inhibe faiblement plusieurs cytochromes et transporteurs, dont la P-glycoprotéine et la BCRP ».",
    },
    Profile {
        needs: &["canagliflozine"],
        label: "Canagliflozine",
        actions: &[
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Invokana : « Digoxine : concentrations augmentées par inhibition de la P-gp » — la BCRP, « possiblement », n'est pas reprise.",
    },
    Profile {
        needs: &["tafamidis"],
        label: "Tafamidis",
        actions: &[
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Vyndaqel : « Inhibition de la protéine de transport BCRP : l'exposition à la rosuvastatine est environ doublée ».",
    },
    Profile {
        needs: &["glecaprevir"],
        label: "Glécaprévir + pibrentasvir",
        actions: &[
            Action::new(Pgp, Inhibitor, None),
            Action::new(Oatp1b1, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Maviret : « Dabigatran et digoxine : concentrations augmentées par inhibition de la glycoprotéine P » ; « Le glécaprévir et le pibrentasvir inhibent l'OATP1B1/1B3 et la BCRP ».",
    },
    Profile {
        needs: &["teriflunomide"],
        label: "Tériflunomide",
        actions: &[
            Action::new(Cyp2c8, Inhibitor, None),
            Action::new(Cyp1a2, Inducer, None),
            Action::new(Bcrp, Inhibitor, None),
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Aubagio : « Le tériflunomide inhibe le CYP2C8 […], induit le CYP1A2 […] et inhibe les transporteurs OAT3 et BCRP » ; « Il inhibe aussi l'OATP1B1/1B3 : la dose de rosuvastatine ne dépasse pas 10 mg ».",
    },
    Profile {
        needs: &["leflunomide"],
        label: "Léflunomide",
        actions: &[
            Action::new(Cyp2c8, Inhibitor, None),
            Action::new(Cyp1a2, Inducer, None),
        ],
        source: "Arava : « Le tériflunomide inhibe le CYP2C8 et induit le CYP1A2 » — les transporteurs y sont nommés par leurs substrats, sans rôle écrit, et ne sont pas repris.",
    },
    Profile {
        needs: &["bempedoique"],
        label: "Acide bempédoïque",
        actions: &[
            Action::new(Oatp1b1, Inhibitor, None),
        ],
        source: "Nilemdo : « L'acide bempédoïque inhibe l'OATP1B1 et augmente l'exposition à ces statines ».",
    },
    // ---- Cytokines, ajoutées en 0.359.0 ----
    // **Une levée de répression, lue comme une induction.** L'IL-6 de
    // l'inflammation chronique freine les cytochromes ; la bloquer leur
    // rend leur activité, et ce qui y passe voit son exposition baisser
    // — à l'instauration, puis remonter à l'arrêt. Le sens est celui
    // d'un inducteur, la cause ne l'est pas, et la citation le dit. La
    // force n'est pas qualifiée par les fiches : `None`.
    Profile {
        needs: &["tocilizumab"],
        label: "Tocilizumab",
        actions: &[
            Action::new(Cyp3a4, Inducer, None),
            Action::new(Cyp1a2, Inducer, None),
            Action::new(Cyp2c9, Inducer, None),
            Action::new(Cyp2c19, Inducer, None),
        ],
        source: "RoActemra : « l'interleukine 6 réprimant les cytochromes, son blocage restaure leur activité et diminue les concentrations de ces médicaments » ; « Le CYP3A4 en fait partie : l'exposition à la simvastatine baisse d'environ 57 % une semaine après une dose » — une levée de répression, pas une induction ; « les CYP1A2, CYP2C9 et CYP2C19 aussi, d'où la warfarine et la théophylline à surveiller ».",
    },
    Profile {
        needs: &["sarilumab"],
        label: "Sarilumab",
        actions: &[Action::new(Cyp3a4, Inducer, None)],
        source: "Kevzara : « le blocage de l'interleukine 6 rétablit l'activité des CYP et fait baisser leurs concentrations » ; « Substrats du CYP3A4 comme les contraceptifs oraux ou les statines : exposition diminuée, la simvastatine perdant environ 45 % une semaine après une injection » — une levée de répression, pas une induction.",
    },
    // ---- Transporteurs du rein, ajoutés en 0.362.0 ----
    // L'OCT2 fait entrer les cations organiques dans la cellule du tubule,
    // les MATE les font sortir dans l'urine : la metformine et la
    // fampridine s'accumulent quand on les freine.
    Profile {
        needs: &["metformine"],
        label: "Metformine",
        actions: &[
            Action::new(Oct2, Substrate, None),
            Action::new(Mate1, Substrate, None),
        ],
        source: "Glucophage : « La metformine est un substrat des transporteurs OCT1 et OCT2 : les inhibiteurs de l'OCT2 […] diminuent son élimination rénale et augmentent son exposition » ; « son excrétion urinaire passe aussi par les transporteurs MATE1 et MATE2-K » — et les associations fixes le disent de leur metformine.",
    },
    Profile {
        needs: &["fampridine"],
        label: "Fampridine",
        actions: &[Action::new(Oct2, Substrate, None)],
        source: "Fampyra : « Élimination essentiellement rénale […] par filtration glomérulaire et sécrétion tubulaire active via l'OCT2 » ; « Inhibiteurs de l'OCT2, cimétidine au premier rang […] : augmentation de son exposition, donc du risque convulsif ».",
    },
    Profile {
        needs: &["cimetidine"],
        label: "Cimétidine",
        actions: &[Action::new(Oct2, Inhibitor, None)],
        source: "Cimétidine : « Elle inhibe aussi le transporteur rénal OCT2 : l'élimination de la metformine et de la fampridine diminue » — les cytochromes qu'elle freine ne sont pas nommés par la fiche, et ne sont pas repris.",
    },
    Profile {
        needs: &["bictegravir"],
        label: "Bictégravir",
        actions: &[
            Action::new(Oct2, Inhibitor, None),
            Action::new(Mate1, Inhibitor, None),
        ],
        source: "Biktarvy : « Le bictégravir inhibe les transporteurs OCT2 et MATE1, ce qui augmente la créatininémie et les concentrations de metformine sans traduire de toxicité rénale ».",
    },
    Profile {
        needs: &["tipiracil"],
        label: "Trifluridine + tipiracil",
        actions: &[
            Action::new(Oct2, Substrate, None),
            Action::new(Mate1, Substrate, None),
        ],
        source: "Lonsurf : « le tipiracil [est un substrat] des transporteurs OCT2 et MATE1 : leurs inhibiteurs peuvent en augmenter la concentration ».",
    },
    Profile {
        needs: &["zanubrutinib", "brukinsa"],
        label: "Zanubrutinib",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inducer, Some(Weak)),
            Action::new(Cyp2c19, Inducer, Some(Weak)),
            Action::new(Pgp, Inhibitor, None),
        ],
        source: "Brukinsa : « Substrat majeur du CYP3A4 » ; « Le zanubrutinib est lui-même un inducteur faible du CYP3A4 et du CYP2C19 » ; « Il augmente la concentration maximale de la digoxine, substrat de la P-gp, de 34 % » — sans force pour la P-gp.",
    },
    Profile {
        needs: &["mavacamten", "camzyos"],
        label: "Mavacamten",
        actions: &[
            Action::new(Cyp2c19, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Substrate, None),
        ],
        source: "Camzyos : « Substrat majeur du CYP2C19 et, dans une moindre mesure, du CYP3A4 ; chez le métaboliseur lent du CYP2C19, c'est le CYP3A4 qui l'élimine » — sans force pour le CYP3A4.",
    },
    Profile {
        needs: &["roxadustat", "evrenzo"],
        label: "Roxadustat",
        actions: &[
            Action::new(Cyp2c8, Substrate, None),
            Action::new(Oatp1b1, Inhibitor, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Evrenzo : « Substrat du CYP2C8 et de l'UGT1A9 : le gemfibrozil et le probénécide multiplient son exposition par 2,3 » ; « Inhibiteur de la BCRP et de l'OATP1B1 : exposition à la simvastatine multipliée par 1,8, à la rosuvastatine par 2,9 » — sans force.",
    },
    Profile {
        needs: &["deferasirox", "exjade"],
        label: "Déférasirox",
        actions: &[
            Action::new(Cyp2c8, Inhibitor, Some(Moderate)),
            Action::new(Cyp1a2, Inhibitor, None),
        ],
        source: "Exjade : « Inhibiteur modéré du CYP2C8 : il multiplie par 2,3 l'aire sous la courbe du répaglinide » ; « Inhibiteur du CYP1A2 : exposition à la théophylline augmentée de 84 % » — sans force pour le CYP1A2.",
    },
    Profile {
        needs: &["trametinib", "mekinist"],
        label: "Trametinib",
        actions: &[
            Action::new(Pgp, Substrate, None),
            Action::new(Bcrp, Inhibitor, None),
        ],
        source: "Mekinist : « Substrat de la P-gp in vitro : attention avec les inhibiteurs puissants de la P-gp » ; « Inhibition transitoire de la BCRP intestinale » — sans force.",
    },
];

#[cfg(test)]
mod tests {

    /// **Un profil qu'aucune fiche n'atteint ne se voit jamais.**
    ///
    /// Même filet que dans `renal`, `hepatic` et `gravidity` : la base
    /// livrée est ce contre quoi ces profils ont été écrits, et une
    /// faute de frappe dans un `needs` ne se distingue autrement pas
    /// d'une ligne correcte que la démonstration ne déclenche pas.
    #[test]
    fn every_row_can_fire_on_the_base_as_shipped() {
        for row in TABLE {
            let reachable = row.needs.iter().any(|needle| {
                let needle = crate::fuzzy::sort_key(needle);
                crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::contains_folded(
                            &crate::fuzzy::sort_key(&format!("{name} {dci} {class}")),
                            &needle,
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
        const FLOOR: usize = 197;
        assert!(
            TABLE.len() >= FLOOR,
            "{} molécules aux cytochromes, il y en avait {FLOOR}",
            TABLE.len()
        );
    }

    use super::*;

    fn t<'a>(name: &'a str, dci: &'a str) -> crate::revue::Treatment<'a> {
        crate::revue::Treatment {
            name,
            dci,
            class: "",
            tags: "",
        }
    }

    /// **Le silence n'est pas une permission.**
    ///
    /// Une ligne que la table ne connaît pas est nommée. Une lecture qui
    /// n'afficherait que les croisements trouvés se lirait « rien à
    /// signaler » sur une ordonnance dont six lignes sur huit sont
    /// inconnues — et c'est ainsi qu'on rassure à tort.
    #[test]
    fn a_line_the_table_does_not_know_is_named() {
        let r = cross(&[
            t("Doliprane", "paracétamol"),
            t("Zeclar", "clarithromycine"),
        ]);
        assert_eq!(r.unknown, vec!["Doliprane".to_owned()]);
        // Et une ordonnance dont la table ne connaît rien ne rend pas
        // une lecture vide : elle rend deux noms.
        let r = cross(&[
            t("Doliprane", "paracétamol"),
            t("Spasfon", "phloroglucinol"),
        ]);
        assert!(r.crossings.is_empty());
        assert_eq!(r.unknown.len(), 2);
    }

    /// **Pour une prodrogue, l'inhibiteur ne fait pas monter l'effet, il
    /// le fait tomber.**
    ///
    /// C'est l'erreur qui coûte le plus cher sur cette table. Un moteur
    /// naïf annonce « exposition augmentée » pour l'oméprazole et le
    /// clopidogrel, alors que le patient se retrouve **sans
    /// antiagrégant** — ce qui est l'inverse d'un surdosage, et ce que
    /// la fiche du Plavix dit en toutes lettres.
    #[test]
    fn an_inhibitor_disarms_a_prodrug_instead_of_piling_it_up() {
        let r = cross(&[t("Mopral", "oméprazole"), t("Plavix", "clopidogrel")]);
        let c = r
            .crossings
            .iter()
            .find(|c| c.affected == "Plavix")
            .expect("le croisement existe");
        assert_eq!(c.enzyme, Enzyme::Cyp2c19);
        assert_eq!(c.role, Role::Inhibitor);
        assert_eq!(c.shift, Shift::ActivityDown);
        // Et le même inhibiteur sur un substrat ordinaire fait l'inverse.
        let r = cross(&[t("Mopral", "oméprazole"), t("Seroplex", "escitalopram")]);
        let c = r
            .crossings
            .iter()
            .find(|c| c.affected == "Seroplex" && c.enzyme == Enzyme::Cyp2c19)
            .expect("le croisement existe");
        assert_eq!(c.shift, Shift::ExposureUp);
        // Un inducteur sur une prodrogue relève l'effet, il ne
        // l'efface pas : les quatre cas existent, et ce sont bien
        // quatre.
        let r = cross(&[t("Rifadine", "rifampicine"), t("Plavix", "clopidogrel")]);
        assert!(r
            .crossings
            .iter()
            .any(|c| c.affected == "Plavix" && c.shift == Shift::ActivityUp));
        let r = cross(&[t("Rifadine", "rifampicine"), t("Zocor", "simvastatine")]);
        assert!(r
            .crossings
            .iter()
            .any(|c| c.affected == "Zocor" && c.shift == Shift::ExposureDown));
    }

    /// **Ce que la fiche nie n'entre pas dans la table.**
    ///
    /// La clarithromycine et la simvastatine sont un croisement majeur ;
    /// la clarithromycine et la rosuvastatine n'en sont pas un, parce
    /// que la rosuvastatine n'est pas métabolisée par le CYP3A4 — sa
    /// fiche l'écrit. Un moteur qui raisonne par classe se trompe des
    /// deux côtés : il interdit la seconde association et laisse passer
    /// la première.
    #[test]
    fn what_the_card_denies_is_not_in_the_table() {
        let bad = cross(&[t("Zeclar", "clarithromycine"), t("Zocor", "simvastatine")]);
        assert_eq!(bad.crossings.len(), 1);
        assert_eq!(bad.crossings[0].weight, Weight::Major);
        let fine = cross(&[
            t("Zeclar", "clarithromycine"),
            t("Crestor", "rosuvastatine"),
        ]);
        assert!(
            fine.crossings.is_empty(),
            "la rosuvastatine ne passe pas par le CYP3A4 : {:?}",
            fine.crossings
        );
        // La spiramycine est le macrolide qui n'inhibe pas : elle n'est
        // pas dans la table, donc elle est **inconnue** et non
        // innocentée. La nuance est celle de la première règle.
        let r = cross(&[t("Rovamycine", "spiramycine"), t("Zocor", "simvastatine")]);
        assert!(r.crossings.is_empty());
        assert_eq!(r.unknown, vec!["Rovamycine".to_owned()]);
    }

    /// **« On ne sait pas » et « on sait, et il n'y a rien » sont deux
    /// réponses opposées.**
    ///
    /// C'est la question qu'on pose vraiment sous clarithromycine : par
    /// quoi remplacer la simvastatine ? La pravastatine y répond, et sa
    /// fiche l'écrit — « elle n'est pas métabolisée par le CYP3A4 ».
    /// Elle est **connue** de la table, donc jamais rangée avec les
    /// inconnues ; et depuis les transporteurs elle n'est plus inerte :
    /// sa captation par l'OATP1B1 la fait croiser la ciclosporine, qui
    /// est exactement ce que sa fiche plafonne.
    ///
    /// L'inerte reste une réponse : le cabotégravir, dont la fiche écrit
    /// qu'il ne passe par aucune voie qui compte.
    #[test]
    fn a_molecule_with_no_route_is_not_a_molecule_nobody_knows() {
        let r = cross(&[
            t("Zeclar", "clarithromycine"),
            t("Vasten", "pravastatine"),
            t("Doliprane", "paracétamol"),
        ]);
        assert!(r.crossings.is_empty(), "{:?}", r.crossings);
        assert!(r.inert.is_empty() && !r.unknown.contains(&"Vasten".to_owned()));
        assert_eq!(r.unknown, vec!["Doliprane".to_owned()]);
        // Sous ciclosporine, c'est l'OATP1B1 qui parle.
        let r = cross(&[t("Néoral", "ciclosporine"), t("Vasten", "pravastatine")]);
        assert!(
            r.crossings
                .iter()
                .any(|c| c.affected == "Vasten" && c.enzyme == Oatp1b1),
            "{:?}",
            r.crossings
        );
        let r = cross(&[
            t("Vocabria", "cabotégravir"),
            t("Zeclar", "clarithromycine"),
        ]);
        assert_eq!(r.inert, vec!["Vocabria".to_owned()]);
    }

    /// La colchicine sous clarithromycine se croise sur le CYP3A4 *et*
    /// sur la glycoprotéine P : une rencontre, deux voies, une seule
    /// ligne à lire — la plus lourde en tête.
    #[test]
    fn one_pair_met_on_two_routes_is_read_once() {
        let r = cross(&[
            t("Zeclar", "clarithromycine"),
            t("Colchicine", "colchicine"),
        ]);
        let g = grouped(&r.crossings);
        let mine: Vec<_> = g.iter().filter(|g| g[0].affected == "Colchicine").collect();
        assert_eq!(mine.len(), 1, "{:?}", r.crossings);
        let routes: Vec<Enzyme> = mine[0].iter().map(|c| c.enzyme).collect();
        assert!(
            routes.contains(&Cyp3a4) && routes.contains(&Pgp),
            "{routes:?}"
        );
        assert!(mine[0].windows(2).all(|w| w[0].weight <= w[1].weight));
        // Deux sens opposés ne se réunissent pas.
        assert_eq!(
            grouped(&r.crossings).iter().map(|g| g.len()).sum::<usize>(),
            r.crossings.len()
        );
    }

    /// **Une cytokine bloquée rend les cytochromes.** Le tocilizumab
    /// n'induit rien : il lève la répression que l'IL-6 exerçait, et la
    /// simvastatine perd plus de la moitié de son exposition. Le sens
    /// est celui d'un inducteur, et c'est lui que la lecture annonce.
    #[test]
    fn an_anti_il6_lowers_what_the_cytochromes_carry() {
        let r = cross(&[t("RoActemra", "tocilizumab"), t("Zocor", "simvastatine")]);
        assert!(
            r.crossings.iter().any(|c| c.affected == "Zocor"
                && c.enzyme == Cyp3a4
                && c.shift == Shift::ExposureDown),
            "{:?}",
            r.crossings
        );
        let r = cross(&[t("Kevzara", "sarilumab"), t("Néoral", "ciclosporine")]);
        assert!(r.crossings.iter().any(|c| c.shift == Shift::ExposureDown));
    }

    /// **Ce que les fiches ont appris à dire** : l'atorvastatine passe
    /// par l'OATP1B1 que la ciclosporine et le Maviret bloquent ; le
    /// tocilizumab rend aussi le CYP2C9 de la warfarine.
    #[test]
    fn the_liver_uptake_and_the_il6_reach_further() {
        let meets = |a: (&str, &str), b: (&str, &str), e: Enzyme, sh: Shift| {
            cross(&[t(a.0, a.1), t(b.0, b.1)])
                .crossings
                .iter()
                .any(|c| c.affected == b.0 && c.enzyme == e && c.shift == sh)
        };
        assert!(meets(
            ("Néoral", "ciclosporine"),
            ("Tahor", "atorvastatine"),
            Oatp1b1,
            Shift::ExposureUp
        ));
        assert!(meets(
            ("Maviret", "glécaprévir + pibrentasvir"),
            ("Crestor", "rosuvastatine"),
            Oatp1b1,
            Shift::ExposureUp
        ));
        assert!(meets(
            ("RoActemra", "tocilizumab"),
            ("Coumadine", "warfarine"),
            Cyp2c9,
            Shift::ExposureDown
        ));
    }

    /// **La metformine s'accumule par le rein** : le dolutégravir, le
    /// bictégravir et la cimétidine freinent l'OCT2 ou le MATE1 qui la
    /// font passer dans l'urine. Aucun cytochrome n'y est pour rien.
    #[test]
    fn the_kidney_transporters_carry_metformin() {
        for (actor, dci) in [
            ("Tivicay", "dolutégravir"),
            ("Biktarvy", "bictégravir + emtricitabine + ténofovir"),
            ("Cimétidine", "cimétidine"),
            ("Ranexa", "ranolazine"),
        ] {
            let r = cross(&[t(actor, dci), t("Glucophage", "metformine")]);
            assert!(
                r.crossings.iter().any(|c| c.affected == "Glucophage"
                    && matches!(c.enzyme, Oct2 | Mate1)
                    && c.shift == Shift::ExposureUp),
                "{actor} : {:?}",
                r.crossings
            );
        }
        let r = cross(&[t("Cimétidine", "cimétidine"), t("Fampyra", "fampridine")]);
        assert!(r.crossings.iter().any(|c| c.affected == "Fampyra"));
        // Le Dovato ne l'écrit pas : pas de rencontre sur l'OCT2.
        let r = cross(&[
            t("Dovato", "dolutégravir + lamivudine"),
            t("Glucophage", "metformine"),
        ]);
        assert!(
            !r.crossings.iter().any(|c| c.enzyme == Oct2),
            "{:?}",
            r.crossings
        );
    }

    /// **Les interactions que les cytochromes n'expliquent pas.** Le
    /// dabigatran ne passe par aucun cytochrome : c'est la glycoprotéine
    /// P qui l'amène sous amiodarone et le fait fondre sous rifampicine.
    /// La rosuvastatine non plus : c'est l'OATP1B1 que la ciclosporine
    /// bloque. Avant les transporteurs, la table taisait les deux — pire,
    /// elle offrait la rosuvastatine comme l'alternative sûre.
    #[test]
    fn the_transporters_carry_what_no_cytochrome_does() {
        let on = |a: (&str, &str), b: (&str, &str)| {
            cross(&[t(a.0, a.1), t(b.0, b.1)])
                .crossings
                .into_iter()
                .filter(|c| c.affected == b.0)
                .map(|c| (c.enzyme, c.shift))
                .collect::<Vec<_>>()
        };
        assert!(on(("Cordarone", "amiodarone"), ("Pradaxa", "dabigatran"))
            .contains(&(Pgp, Shift::ExposureUp)));
        assert!(on(("Rifadine", "rifampicine"), ("Pradaxa", "dabigatran"))
            .contains(&(Pgp, Shift::ExposureDown)));
        assert!(on(("Isoptine", "vérapamil"), ("Digoxine", "digoxine"))
            .contains(&(Pgp, Shift::ExposureUp)));
        assert!(on(("Néoral", "ciclosporine"), ("Crestor", "rosuvastatine"))
            .contains(&(Oatp1b1, Shift::ExposureUp)));
        assert!(on(("Lipur", "gemfibrozil"), ("Novonorm", "répaglinide"))
            .contains(&(Oatp1b1, Shift::ExposureUp)));
        // Et la clarithromycine ne touche toujours pas la rosuvastatine :
        // sa fiche ne lui prête aucun rôle sur l'OATP1B1.
        assert!(on(("Zeclar", "clarithromycine"), ("Crestor", "rosuvastatine")).is_empty());
        // Le poids lit les deux forces sur un transporteur comme sur une
        // enzyme : clarithromycine puissante, dabigatran non chiffré — au
        // premier rang ; le diltiazem, que sa fiche dit faible sur la
        // P-gp, au dernier.
        let r = cross(&[t("Zeclar", "clarithromycine"), t("Pradaxa", "dabigatran")]);
        assert!(r
            .crossings
            .iter()
            .any(|c| c.enzyme == Pgp && c.weight == Weight::Major));
        let r = cross(&[t("Tildiem", "diltiazem"), t("Pradaxa", "dabigatran")]);
        assert!(r
            .crossings
            .iter()
            .any(|c| c.enzyme == Pgp && c.weight == Weight::Minor));
    }

    /// **Le choix de l'IPP sous clopidogrel, tel que les fiches
    /// l'écrivent.**
    ///
    /// Les trois se ressemblent sur une ordonnance et ne font pas la
    /// même chose : l'oméprazole et l'ésoméprazole freinent le CYP2C19
    /// qui active le clopidogrel, le rabéprazole le freine faiblement et
    /// sa fiche juge l'association acceptable, le pantoprazole est « le
    /// moins inhibiteur du CYP2C19 parmi les IPP, ce qui fonde sa place
    /// chez les patients sous clopidogrel ». C'est la substitution la
    /// plus fréquente du comptoir, et un moteur par classe la rate.
    #[test]
    fn the_ppi_that_disarms_clopidogrel_is_not_the_one_that_replaces_it() {
        let effet = |ipp: (&str, &str)| {
            cross(&[t(ipp.0, ipp.1), t("Plavix", "clopidogrel")])
                .crossings
                .into_iter()
                .find(|c| c.affected == "Plavix")
                .map(|c| (c.shift, c.actor_force))
        };
        // Celui qu'on évite : l'effet tombe.
        assert_eq!(
            effet(("Mopral", "oméprazole")),
            Some((Shift::ActivityDown, Some(Force::Moderate)))
        );
        assert_eq!(
            effet(("Inexium", "ésoméprazole")),
            Some((Shift::ActivityDown, Some(Force::Moderate)))
        );
        // Celui qu'on tolère : même sens, force faible, et l'écran le
        // dit — le croisement n'est pas caché, il est pesé.
        assert_eq!(
            effet(("Pariet", "rabéprazole")),
            Some((Shift::ActivityDown, Some(Force::Weak)))
        );
        // Et celui qu'on prend : aucun croisement du tout.
        assert_eq!(effet(("Inipomp", "pantoprazole")), None);
    }

    /// **Un kétoconazole local n'est pas un kétoconazole.**
    ///
    /// Sa fiche l'écrit : « Aucune interaction systémique cliniquement
    /// significative n'est attendue avec les formes locales ». Une table
    /// qui raisonnerait sur la molécule mettrait un shampooing en face
    /// d'une simvastatine, ce qui est le genre d'alerte qui apprend à
    /// ignorer les alertes.
    #[test]
    fn a_topical_azole_does_not_cross_anything() {
        let r = cross(&[t("Kétoderm", "kétoconazole"), t("Zocor", "simvastatine")]);
        assert!(r.crossings.is_empty(), "{:?}", r.crossings);
        assert_eq!(r.unknown, vec!["Kétoderm".to_owned()]);
    }

    /// **Une ciclosporine en collyre n'est pas une ciclosporine.**
    ///
    /// Le Kétoderm avait été réglé en retirant le kétoconazole de la
    /// table ; la ciclosporine ne se retire pas, le Néoral en vit. C'est
    /// donc la **voie** qu'on lit, par la classe de la fiche, et
    /// l'Ikervis part en inconnue comme le Kétoderm : ce module nomme ce
    /// sur quoi il ne se prononce pas, et il ne délivre pas de
    /// certificat de bonne conduite.
    #[test]
    fn a_ciclosporin_collyre_crosses_nothing() {
        let ik = crate::revue::Treatment {
            name: "Ikervis",
            dci: "ciclosporine",
            class: "collyre — immunomodulateur (sécheresse oculaire sévère)",
            tags: "",
        };
        let zocor = t("Zocor", "simvastatine");
        let r = cross(&[ik, zocor]);
        assert!(r.crossings.is_empty(), "{:?}", r.crossings);
        assert_eq!(r.unknown, vec!["Ikervis".to_owned()]);
        // Et la ciclosporine générale croise toujours, elle. Face à un
        // **inhibiteur** et non à une autre statine : la ciclosporine
        // est substrat du CYP3A4 et rien d'autre, et deux substrats de
        // la même enzyme ne se font rien l'un à l'autre — c'est tout le
        // modèle de ce module, et la première version de ce test
        // l'avait oublié.
        let r = cross(&[t("Néoral", "ciclosporine"), t("Zeclar", "clarithromycine")]);
        assert!(
            r.crossings.iter().any(|c| c.affected == "Néoral"),
            "la voie générale croise encore : {:?}",
            r.crossings
        );
    }

    /// **Une molécule n'agit pas sur elle-même**, et deux lignes d'une
    /// même molécule ne se croisent pas non plus.
    ///
    /// Le ticagrélor est substrat *et* inhibiteur du CYP3A4 : sans la
    /// règle, il se croiserait lui-même et l'ordonnance porterait un
    /// conflit imaginaire — celui qu'aucun pharmacien ne trouverait en
    /// relisant la fiche.
    #[test]
    fn a_molecule_does_not_cross_itself() {
        let r = cross(&[t("Brilique", "ticagrélor")]);
        assert!(r.crossings.is_empty(), "{:?}", r.crossings);
        // Mais il croise bien ce qui passe par la même enzyme.
        let r = cross(&[t("Brilique", "ticagrélor"), t("Zocor", "simvastatine")]);
        assert!(r.crossings.iter().any(|c| c.affected == "Zocor"));
    }

    /// **Le premier motif qui répond gagne, et l'ordre de la table
    /// décide.**
    ///
    /// « esomeprazole » contient « omeprazole » une fois replié : sans
    /// l'ordre, l'Inexium serait annoncé comme du Mopral. C'est la leçon
    /// de `crush.rs`, où « actiskenan » contient « skenan », et elle
    /// **Aucun mot cherché n'est mangé par un mot placé plus haut.**
    ///
    /// La table est lue dans l'ordre et la première ligne qui répond
    /// gagne : c'est ce qui fait tenir l'ésoméprazole avant
    /// l'oméprazole et la trimipramine avant l'imipramine. Le corollaire
    /// est qu'une ligne dont le mot contient celui d'une ligne
    /// antérieure ne répondra **jamais** — et cela ne se voit pas à
    /// l'écran, puisqu'elle se contente de citer la mauvaise fiche. Le
    /// cas de la trimipramine a vécu des mois ainsi.
    ///
    /// Cette table en compte plus de cent : la prochaine collision ne se
    /// verra pas à l'œil.
    #[test]
    fn no_need_is_eaten_by_one_placed_above_it() {
        for (i, p) in TABLE.iter().enumerate() {
            for n in p.needs {
                let folded = crate::fuzzy::sort_key(n);
                for earlier in TABLE.iter().take(i) {
                    for m in earlier.needs {
                        assert!(
                            !folded.contains(&crate::fuzzy::sort_key(m)),
                            "« {n} » ({}) est mangé par « {m} » ({})",
                            p.label,
                            earlier.label
                        );
                    }
                }
            }
        }
    }

    /// porte son test là-bas comme ici.
    #[test]
    fn a_name_that_contains_another_is_read_for_itself() {
        let p = of("Inexium", "ésoméprazole", "", "").expect("l'ésoméprazole est connu");
        assert_eq!(p.label, "Ésoméprazole");
        let p = of("Mopral", "oméprazole", "", "").expect("l'oméprazole est connu");
        assert_eq!(p.label, "Oméprazole");
    }

    /// **La force d'un substrat n'est pas celle d'un inhibiteur.**
    ///
    /// Le même mot, deux mesures : « puissant » d'un inhibiteur dit de
    /// combien il déplace autrui, « voie principale » d'un substrat dit
    /// quelle part de sa propre élimination passe par là. Les écrire
    /// pareil ferait lire l'un pour l'autre — c'est le piège du grade de
    /// `facets.rs`.
    #[test]
    fn the_same_force_is_not_said_the_same_on_both_sides() {
        assert_eq!(Force::Strong.label(Role::Inhibitor), "puissant");
        assert_eq!(Force::Strong.label(Role::Inducer), "puissant");
        assert_eq!(Force::Strong.label(Role::Substrate), "voie principale");
        assert_ne!(
            Force::Weak.label(Role::Substrate),
            Force::Weak.label(Role::Inhibitor)
        );
    }

    /// Le poids se lit sur **les deux** forces. Un inhibiteur puissant
    /// sur une voie accessoire déplace peu ; un inhibiteur faible sur la
    /// voie principale déplace beaucoup. Ne regarder que l'acteur
    /// classerait le premier au-dessus du second.
    #[test]
    fn the_weight_reads_both_sides() {
        assert_eq!(
            Weight::of(Some(Force::Strong), Some(Force::Strong)),
            Weight::Major
        );
        assert_eq!(
            Weight::of(Some(Force::Strong), Some(Force::Weak)),
            Weight::of(Some(Force::Weak), Some(Force::Strong)),
        );
        assert_eq!(
            Weight::of(Some(Force::Weak), Some(Force::Weak)),
            Weight::Minor
        );
        // Une force non chiffrée ne tire pas vers le bas : elle laisse
        // le croisement là où on le regardera.
        assert_eq!(Weight::of(None, Some(Force::Strong)), Weight::Major);
        assert_eq!(Weight::of(None, None), Weight::Notable);
    }

    /// Chaque molécule de la table est une fiche que le logiciel livre,
    /// **et la fiche nomme l'enzyme**.
    ///
    /// C'est la règle de `facets.rs` : une facette est adossée à ce que
    /// la fiche écrit, et se corrige en corrigeant la fiche. Ici elle
    /// tient à peu de frais, puisque l'enzyme s'écrit de la même façon
    /// des deux côtés.
    #[test]
    fn every_row_is_backed_by_a_card_that_names_the_enzyme() {
        let cards: Vec<(String, String)> = crate::db::STARTER_DETAILS
            .iter()
            .map(|d| {
                (
                    crate::fuzzy::sort_key(d.name),
                    crate::fuzzy::sort_key(
                        &[
                            d.mechanism,
                            d.ddi,
                            d.elimination,
                            d.half_life,
                            d.indications,
                            d.monitoring,
                            d.toxicity,
                        ]
                        .join(" "),
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
        let mut orphans: Vec<String> = Vec::new();
        let mut unbacked: Vec<String> = Vec::new();
        // **Chaque fiche est jugée contre la ligne qui la revendique
        // vraiment**, c'est-à-dire la première qui l'attrape — celle que
        // `of` rendra. Le test ne regardait que la première *fiche*
        // d'une ligne : il validait donc la ligne sur le produit visé et
        // lui prêtait, en silence, tous ceux qu'elle attrape au passage.
        // Les mots cherchés sont des sous-chaînes, et « imipramine » est
        // dans « trimipramine » : le profil du Tofranil — CYP2D6, 1A2 et
        // 3A4 — s'appliquait ainsi au Surmontil, dont la fiche écrit
        // « principalement par le CYP2D6 ». Rien ne le disait, et rien
        // ne pouvait le dire.
        for ((card, hay), (_, dci, class, _)) in drugs.iter().zip(crate::db::STARTER_DRUGS) {
            // **Une forme locale n'est jamais lue par `cross`** : elle
            // part en inconnue avant la table. L'Ikervis est de la
            // ciclosporine en collyre ; exiger que sa fiche nomme la
            // glycoprotéine P serait exiger qu'elle parle d'une voie
            // qu'il n'emprunte pas.
            if crate::classes::stays_local(dci, class) {
                continue;
            }
            let Some(p) = TABLE.iter().find(|p| {
                p.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            }) else {
                continue;
            };
            let Some((_, body)) = cards.iter().find(|(n, _)| n == card) else {
                continue;
            };
            for a in p.actions {
                // **Une énumération partage son préfixe.** Les fiches
                // écrivent « portant sur les CYP3A4, 2C9, 2C19, 1A2 » :
                // l'enzyme y est nommée, mais pas épelée « CYP2C9 ».
                // Chercher la forme longue seule refusait quatre lignes
                // que la fiche adosse parfaitement — le test avait
                // raison de se plaindre, et tort sur la cause.
                let long = crate::fuzzy::sort_key(a.enzyme.label());
                let short = long.trim_start_matches("cyp").to_owned();
                let named = if a.enzyme.is_transporter() {
                    a.enzyme.spellings().iter().any(|w| body.contains(w))
                } else {
                    body.contains(&long) || (body.contains("cyp") && body.contains(&short))
                };
                if !named {
                    unbacked.push(format!("{} / {} → {card}", p.label, a.enzyme.label()));
                }
            }
        }
        // Et aucune ligne ne vise un produit que la base ne livre pas.
        for p in TABLE {
            if !drugs.iter().any(|(_, hay)| {
                p.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            }) {
                orphans.push(p.label.to_owned());
            }
        }
        assert!(
            orphans.is_empty(),
            "molécules sans fiche livrée : {orphans:?}"
        );
        assert!(
            unbacked.is_empty(),
            "rôles que la fiche ne nomme pas : {unbacked:?}"
        );
    }

    /// `Enzyme::ALL` porte bien toutes les enzymes, et la table n'en
    /// nomme aucune qui n'y soit — sans quoi une vue qui parcourt `ALL`
    /// laisserait une enzyme hors de l'écran sans rien dire.
    #[test]
    fn the_enzyme_list_is_complete() {
        for p in TABLE {
            for a in p.actions {
                assert!(
                    Enzyme::ALL.contains(&a.enzyme),
                    "{} : {} manque à ALL",
                    p.label,
                    a.enzyme.label()
                );
            }
        }
        // Et chacune se nomme une seule fois.
        let mut labels: Vec<&str> = Enzyme::ALL.iter().map(|e| e.label()).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before, "deux enzymes portent le même nom");
    }

    /// Chaque ligne cite sa source, et la table n'a pas deux fois la
    /// même molécule : un doublon ferait hériter la seconde ligne de la
    /// première, silencieusement.
    #[test]
    fn every_row_names_itself_once_and_cites_its_card() {
        let mut seen: Vec<&str> = Vec::new();
        for p in TABLE {
            assert!(!p.needs.is_empty(), "{} n'a pas de motif", p.label);
            assert!(p.source.len() > 30, "{} : source trop courte", p.label);
            assert!(!seen.contains(&p.label), "{} est en double", p.label);
            seen.push(p.label);
            // Le libellé d'une ligne ne se retrouve pas par le motif
            // d'une autre : c'est ce qui garantit que l'ordre de la
            // table suffit à les distinguer.
            let found = of(p.label, "", "", "").map(|q| q.label);
            assert_eq!(found, Some(p.label), "{} est lu comme {:?}", p.label, found);
        }
    }
}
