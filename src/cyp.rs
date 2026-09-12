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
//! * **Elle ne connaît que les cytochromes.** La glycoprotéine P,
//!   l'OATP1B1 et la BCRP expliquent des interactions majeures — la
//!   rosuvastatine et la ciclosporine, la dabigatran et l'amiodarone —
//!   et ne sont pas ici. Une ordonnance sans croisement sur cette table
//!   n'est pas une ordonnance sans interaction.
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

/// Un isoenzyme du cytochrome P450, parmi ceux qui décident quelque
/// chose au comptoir.
///
/// Le CYP2E1 n'y est pas : il explique des toxicités — le paracétamol
/// sous alcool — et presque aucune conduite à tenir sur une ordonnance.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Enzyme {
    Cyp1a2,
    Cyp2b6,
    Cyp2c8,
    Cyp2c9,
    Cyp2c19,
    Cyp2d6,
    Cyp3a4,
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
    ];

    pub fn label(self) -> &'static str {
        match self {
            Enzyme::Cyp1a2 => "CYP1A2",
            Enzyme::Cyp2b6 => "CYP2B6",
            Enzyme::Cyp2c8 => "CYP2C8",
            Enzyme::Cyp2c9 => "CYP2C9",
            Enzyme::Cyp2c19 => "CYP2C19",
            Enzyme::Cyp2d6 => "CYP2D6",
            Enzyme::Cyp3a4 => "CYP3A4",
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
            Weight::Major => "à regarder d'abord",
            Weight::Notable => "à regarder",
            Weight::Minor => "pour mémoire",
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
}

/// Ce que la table sait de cette ligne-là.
pub fn of(name: &str, dci: &str, class: &str, tags: &str) -> Option<&'static Profile> {
    let hay = crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}"));
    // **Le premier qui répond gagne, et l'ordre de la table décide.**
    // « esomeprazole » contient « omeprazole » une fois replié : sans
    // l'ordre, l'Inexium serait annoncé comme du Mopral. C'est la leçon
    // de `crush.rs`, où « actiskenan » contient « skenan », et elle
    // porte son test.
    TABLE.iter().find(|p| {
        p.needs
            .iter()
            .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
    })
}

/// Ce que cette ordonnance croise sur les cytochromes.
///
/// L'ordre est celui du poids, puis celui des noms : deux croisements de
/// même poids ne doivent pas échanger leur place d'une image à l'autre.
pub fn cross(treatments: &[crate::revue::Treatment]) -> Reading {
    let mut known: Vec<(String, &'static Profile)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for t in treatments {
        let name = t.name.trim().to_owned();
        if name.is_empty() {
            continue;
        }
        match of(t.name, t.dci, t.class, t.tags) {
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
    Reading { crossings, unknown }
}

use Enzyme::{Cyp1a2, Cyp2b6, Cyp2c19, Cyp2c8, Cyp2c9, Cyp2d6, Cyp3a4};
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
        ],
        source: "Sporanox : « Inhibiteur puissant du CYP3A4 et de la P-glycoprotéine, et substrat du CYP3A4 ».",
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
        actions: &[Action::new(Cyp3a4, Inhibitor, Some(Strong))],
        source: "Noxafil : « Inhibiteur très puissant du CYP3A4 et de la P-glycoprotéine, sans effet notable sur les autres cytochromes ».",
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
        ],
        source: "Zeclar : « Inhibiteur puissant du CYP3A4 et de la P-glycoprotéine, ce qui en fait l'un des antibiotiques les plus pourvoyeurs d'interactions ».",
    },
    Profile {
        needs: &["josamycine"],
        label: "Josamycine",
        actions: &[Action::new(Cyp3a4, Inhibitor, None)],
        source: "Josacine : « Comme les autres macrolides à quatorze et seize chaînons, elle inhibe le CYP3A4, source d'interactions majeures » — la fiche ne qualifie pas la force.",
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
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
        ],
        source: "Isoptine : « Le vérapamil est un inhibiteur puissant du CYP3A4 et de la P-gp » ; « Métabolisme hépatique intense de premier passage par le CYP3A4 ».",
    },
    Profile {
        needs: &["diltiazem"],
        label: "Diltiazem",
        actions: &[
            Action::new(Cyp3a4, Inhibitor, Some(Strong)),
            Action::new(Cyp3a4, Substrate, Some(Strong)),
        ],
        source: "Tildiem : « Inhibiteur puissant du CYP3A4 et de la P-glycoprotéine » ; « Métabolisme hépatique important par le CYP3A4 ».",
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
        ],
        source: "Rifadine : « Inducteur enzymatique parmi les plus puissants connus, portant sur les CYP3A4, 2C9, 2C19, 1A2 et sur la P-glycoprotéine ».",
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
        ],
        source: "Millepertuis : « Inducteur enzymatique puissant du CYP3A4 et de la P-gp » ; « l'induction concerne le CYP3A4, le CYP2C9, le CYP1A2 » — la fiche ne qualifie que le premier.",
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
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Tahor : « Métabolisme hépatique intense par le CYP3A4 en métabolites actifs ».",
    },
    // **La rosuvastatine n'est pas un substrat du CYP3A4**, et sa fiche
    // le dit en toutes lettres : c'est le contre-exemple qui empêche de
    // raisonner par classe.
    Profile {
        needs: &["rosuvastatine"],
        label: "Rosuvastatine",
        actions: &[Action::new(Cyp2c9, Substrate, Some(Weak))],
        source: "Crestor : « Contrairement à la simvastatine et à l'atorvastatine, la rosuvastatine n'est pas métabolisée par le CYP3A4 » ; « Métabolisme marginal par le CYP2C9 ».",
    },
    Profile {
        needs: &["apixaban"],
        label: "Apixaban",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Moderate))],
        source: "Eliquis : « Métabolisme partiel par le CYP3A4, substrat de la P-gp ».",
    },
    Profile {
        needs: &["rivaroxaban"],
        label: "Rivaroxaban",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Moderate))],
        source: "Xarelto : « Environ un tiers éliminé par voie rénale sous forme inchangée, le reste métabolisé (CYP3A4, CYP2J2) ».",
    },
    Profile {
        needs: &["ticagrelor"],
        label: "Ticagrélor",
        actions: &[
            Action::new(Cyp3a4, Substrate, Some(Strong)),
            Action::new(Cyp3a4, Inhibitor, None),
        ],
        source: "Brilique : « Le ticagrélor est substrat et inhibiteur du CYP3A4 et de la P-gp » ; « Métabolisme hépatique par le CYP3A4 ».",
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
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Certican : « Métabolisme hépatique et intestinal étendu par le CYP3A4 » ; « élévation majeure des concentrations et toxicité ».",
    },
    Profile {
        needs: &["sirolimus"],
        label: "Sirolimus",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Rapamune : « Substrat du CYP3A4 et de la glycoprotéine P, largement métabolisé dans l'intestin et le foie ».",
    },
    Profile {
        needs: &["colchicine"],
        label: "Colchicine",
        actions: &[Action::new(Cyp3a4, Substrate, Some(Strong))],
        source: "Colchicine : « Substrat du CYP3A4 et de la P-gp » ; « concentrations fortement augmentées, réduction de dose impérative ».",
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
    Profile {
        needs: &["olanzapine"],
        label: "Olanzapine",
        actions: &[
            Action::new(Cyp1a2, Substrate, Some(Strong)),
            Action::new(Cyp2d6, Substrate, Some(Weak)),
        ],
        source: "Zyprexa : « Métabolisme hépatique par glucuroconjugaison directe et oxydation par le CYP1A2, accessoirement par le CYP2D6 ».",
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
            .map(|(name, dci, class, tags)| {
                (
                    crate::fuzzy::sort_key(name),
                    crate::fuzzy::sort_key(&format!("{name} {dci} {class} {tags}")),
                )
            })
            .collect();
        let mut orphans: Vec<String> = Vec::new();
        let mut unbacked: Vec<String> = Vec::new();
        for p in TABLE {
            // La fiche qui porte cette molécule.
            let Some((card, _)) = drugs.iter().find(|(_, hay)| {
                p.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            }) else {
                orphans.push(p.label.to_owned());
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
                let named = body.contains(&long) || (body.contains("cyp") && body.contains(&short));
                if !named {
                    unbacked.push(format!("{} / {}", p.label, a.enzyme.label()));
                }
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
            assert!(!p.actions.is_empty(), "{} ne fait rien", p.label);
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
