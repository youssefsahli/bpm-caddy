//! La console : ce que l'officine peut demander à sa propre base, en
//! quelques lignes, sans attendre qu'on l'écrive dans l'application.
//!
//! Une officine se pose des questions que personne n'a prévues — « quels
//! dossiers prennent une statine sans bilan lipidique depuis dix-huit
//! mois », « combien de fiches de la classe X n'ont pas d'interaction
//! écrite ». Chacune est un écran de plus si elle doit être programmée,
//! et trois lignes si la base se laisse interroger.
//!
//! # Ce qui décide de tout : le script ne peut rien faire sortir
//!
//! Rhai n'a **aucune entrée-sortie** — ni fichier, ni réseau, ni
//! processus —, et rien ici n'en ajoute. Le seul chemin qu'un script a
//! vers le monde est la chaîne qu'il rend, et elle s'affiche dans un
//! volet. C'est la raison du choix de ce moteur : une application qui
//! tient des données de santé chiffrées ne peut pas offrir un langage
//! qui sait ouvrir un fichier.
//!
//! Et il ne peut rien **écrire** non plus. Aucune fonction ne modifie la
//! base : la console lit un instantané, pris avant l'exécution, et le
//! rend en tableaux. Un script qui pourrait écrire au registre des
//! stupéfiants — inaltérable par construction — serait un trou dans le
//! seul endroit de l'application qui n'en a pas.
//!
//! # Et il s'arrête
//!
//! `while true {}` est trois caractères. Le moteur est donc borné en
//! **opérations**, pas en secondes : une horloge rendrait le résultat
//! dépendant de la machine, et un script qui passe sur un poste et
//! échoue sur l'autre est pire qu'un script qui échoue partout.
//!
//! Pur et testé, comme les autres modules de règles : l'instantané est
//! passé en argument, et rien ici ne connaît la base.

use std::sync::{Arc, Mutex};

/// Un dossier, tel qu'un script le voit.
#[derive(Clone, Debug, Default)]
pub struct Patient {
    pub id: i64,
    pub nom: String,
    pub prenom: String,
    /// ISO.
    pub naissance: String,
    /// Les noms des traitements du dossier.
    pub traitements: Vec<String>,
}

/// Une fiche, telle qu'un script la voit.
///
/// **Sans sa prose.** Huit cent cinquante et une monographies entières
/// dans un tableau Rhai, ce sont quelques mégaoctets recopiés à chaque
/// exécution pour répondre à une question qui porte presque toujours sur
/// le nom, la classe ou les étiquettes. La prose se demande fiche par
/// fiche, par [`Snapshot::prose`].
#[derive(Clone, Debug, Default)]
pub struct Drug {
    pub id: i64,
    pub nom: String,
    pub dci: String,
    pub classe: String,
    pub tags: String,
    pub statut: String,
}

/// Un acte, tel qu'un script le voit.
#[derive(Clone, Debug, Default)]
pub struct Act {
    /// Le libellé de la thématique.
    pub theme: String,
    pub etat: String,
    /// `AAAA-MM`.
    pub mois: String,
    pub minutes: i64,
    pub operateur: String,
}

/// Une ligne du registre, telle qu'un script la voit.
#[derive(Clone, Debug, Default)]
pub struct Move {
    pub produit: String,
    pub nature: String,
    /// ISO.
    pub jour: String,
    pub quantite: f64,
    /// Le numéro de dossier, jamais le nom : c'est la règle du registre,
    /// et elle vaut aussi pour ce qu'un script en lit.
    pub dossier: i64,
}

/// Ce que la console donne à lire.
///
/// Pris **avant** l'exécution et jamais pendant : un script qui lirait
/// la base au fil de ses boucles la lirait mille fois, et rien ne
/// garantirait que la moitié de sa réponse parle du même instant que
/// l'autre.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub patients: Vec<Patient>,
    pub drugs: Vec<Drug>,
    pub acts: Vec<Act>,
    pub moves: Vec<Move>,
    /// La prose d'une fiche, par identifiant : ce que `fiche(id)` rend.
    /// L'appelant ne remplit que ce qu'il veut bien laisser lire.
    pub prose: std::collections::HashMap<i64, Vec<(String, String)>>,
}

/// Combien d'opérations un script a le droit de faire.
///
/// Assez pour parcourir toutes les fiches et mille dossiers plusieurs
/// fois ; pas assez pour une boucle infinie. Un compte
/// d'opérations et non un délai : une horloge ferait passer le même
/// script sur un poste et échouer sur l'autre.
const MAX_OPERATIONS: u64 = 20_000_000;

/// Ce que la console rend.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// Ce que le script a écrit, ligne par ligne.
    pub printed: Vec<String>,
    /// La valeur de la dernière expression, quand elle en a une et
    /// qu'elle n'est pas vide.
    pub value: String,
    /// L'erreur, s'il y en a eu une. Le reste est rendu quand même : ce
    /// qu'un script a écrit avant de se tromper est souvent ce qui dit
    /// où il s'est trompé.
    pub error: Option<String>,
}

/// Exécuter un script contre un instantané.
///
/// Ne panique jamais : une erreur de syntaxe, un dépassement de compte,
/// un accès hors bornes se rendent comme du texte. La console est un
/// endroit où l'on se trompe, c'est même sa raison d'être.
pub fn run(source: &str, data: &Snapshot) -> Outcome {
    let printed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let mut engine = rhai::Engine::new();
    // Les bornes. `set_max_operations` est ce qui arrête `while true {}` ;
    // les autres arrêtent les explosions de mémoire, qui sont l'autre
    // façon de bloquer une application avec trois caractères.
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(1_000_000);
    engine.set_max_array_size(1_000_000);
    engine.set_max_map_size(1_000_000);
    // Ce qu'un script écrit va dans le volet et nulle part ailleurs.
    {
        let sink = Arc::clone(&printed);
        engine.on_print(move |text| {
            if let Ok(mut lines) = sink.lock() {
                lines.push(text.to_owned());
            }
        });
    }
    // `debug` aussi : sans cela il écrit sur la sortie standard, qu'une
    // application de bureau n'a pas.
    {
        let sink = Arc::clone(&printed);
        engine.on_debug(move |text, _, pos| {
            if let Ok(mut lines) = sink.lock() {
                lines.push(format!("{pos:?} {text}"));
            }
        });
    }

    // --- Ce que le script peut lire ---------------------------------
    //
    // Des tableaux de cartes, et non des types propres : une carte se
    // parcourt, se filtre et s'imprime avec le langage tel qu'il est,
    // sans que personne ait à apprendre une bibliothèque.
    let patients = data.patients.clone();
    engine.register_fn("patients", move || {
        patients
            .iter()
            .map(|p| {
                let mut m = rhai::Map::new();
                m.insert("id".into(), rhai::Dynamic::from(p.id));
                m.insert("nom".into(), p.nom.clone().into());
                m.insert("prenom".into(), p.prenom.clone().into());
                m.insert("naissance".into(), p.naissance.clone().into());
                m.insert(
                    "traitements".into(),
                    p.traitements
                        .iter()
                        .map(|t| rhai::Dynamic::from(t.clone()))
                        .collect::<rhai::Array>()
                        .into(),
                );
                rhai::Dynamic::from(m)
            })
            .collect::<rhai::Array>()
    });
    let drugs = data.drugs.clone();
    engine.register_fn("medicaments", move || {
        drugs
            .iter()
            .map(|d| {
                let mut m = rhai::Map::new();
                m.insert("id".into(), rhai::Dynamic::from(d.id));
                m.insert("nom".into(), d.nom.clone().into());
                m.insert("dci".into(), d.dci.clone().into());
                m.insert("classe".into(), d.classe.clone().into());
                m.insert("tags".into(), d.tags.clone().into());
                m.insert("statut".into(), d.statut.clone().into());
                rhai::Dynamic::from(m)
            })
            .collect::<rhai::Array>()
    });
    let acts = data.acts.clone();
    engine.register_fn("entretiens", move || {
        acts.iter()
            .map(|a| {
                let mut m = rhai::Map::new();
                m.insert("theme".into(), a.theme.clone().into());
                m.insert("etat".into(), a.etat.clone().into());
                m.insert("mois".into(), a.mois.clone().into());
                m.insert("minutes".into(), rhai::Dynamic::from(a.minutes));
                m.insert("operateur".into(), a.operateur.clone().into());
                rhai::Dynamic::from(m)
            })
            .collect::<rhai::Array>()
    });
    let moves = data.moves.clone();
    engine.register_fn("registre", move || {
        moves
            .iter()
            .map(|l| {
                let mut m = rhai::Map::new();
                m.insert("produit".into(), l.produit.clone().into());
                m.insert("nature".into(), l.nature.clone().into());
                m.insert("jour".into(), l.jour.clone().into());
                m.insert("quantite".into(), rhai::Dynamic::from(l.quantite));
                m.insert("dossier".into(), rhai::Dynamic::from(l.dossier));
                rhai::Dynamic::from(m)
            })
            .collect::<rhai::Array>()
    });
    // La prose d'une fiche, demandée une par une : la charger pour
    // toutes recopierait quelques mégaoctets à chaque exécution, pour
    // une question qui porte presque toujours sur le nom.
    let prose = data.prose.clone();
    engine.register_fn("fiche", move |id: i64| {
        let mut m = rhai::Map::new();
        for (field, text) in prose.get(&id).into_iter().flatten() {
            m.insert(field.as_str().into(), text.clone().into());
        }
        m
    });

    let result = engine.eval::<rhai::Dynamic>(source);
    let printed = printed
        .lock()
        .map(|l| l.clone())
        .unwrap_or_else(|e| e.into_inner().clone());
    match result {
        Ok(value) => Outcome {
            printed,
            value: if value.is_unit() {
                String::new()
            } else {
                value.to_string()
            },
            error: None,
        },
        Err(e) => Outcome {
            printed,
            value: String::new(),
            error: Some(e.to_string()),
        },
    }
}

/// Une fonction que la console offre, décrite **là où elle est
/// enregistrée**.
///
/// Une seconde liste, tenue à la main dans l'écran d'aide, finirait par
/// décrire une console qui n'existe plus : c'est la description que
/// personne ne relit qui aurait tort, et elle aurait tort en silence.
/// Un test lit le texte de ce module, y cherche les fonctions
/// réellement enregistrées, et refuse qu'il en manque une ici — ou
/// l'inverse.
pub struct Call {
    /// Comme on l'écrit : « patients() », « fiche(id) ». Le nom que le
    /// moteur connaît est ce qui précède la parenthèse.
    pub call: &'static str,
    /// Ce qu'elle rend, en une ligne.
    pub returns: &'static str,
    /// Les clés des cartes rendues, et ce que chacune porte.
    pub fields: &'static [(&'static str, &'static str)],
    /// Ce qu'il faut savoir avant de s'en servir, ou rien.
    pub note: &'static str,
    /// Un script court qui marche tel quel — c'est lui que l'explorateur
    /// pose dans la console, et un test l'exécute.
    pub example: &'static str,
}

/// Tout ce qu'un script peut lire. Quatre tableaux et une prose.
pub const API: &[Call] = &[
    Call {
        call: "patients()",
        returns: "Les dossiers de la base, en tableau de cartes.",
        fields: &[
            ("id", "le numéro de dossier"),
            ("nom", "le nom de famille"),
            ("prenom", "le prénom"),
            ("naissance", "la date de naissance, AAAA-MM-JJ"),
            ("traitements", "les noms des traitements, en tableau"),
        ],
        note: "",
        example: "for p in patients() {\n    \
                  if p.traitements.len >= 5 { print(`${p.nom} ${p.prenom}`); }\n\
                  }\n",
    },
    Call {
        call: "medicaments()",
        returns: "Les fiches de la base, en tableau de cartes.",
        fields: &[
            ("id", "le numéro de fiche"),
            ("nom", "le nom commercial"),
            ("dci", "la dénomination commune"),
            ("classe", "la classe thérapeutique, telle qu'elle est écrite"),
            ("tags", "les étiquettes de la fiche"),
            ("statut", "commercialisé, arrêté…"),
        ],
        note: "Sans la prose : les monographies entières seraient \
               quelques mégaoctets recopiés à chaque exécution. La prose \
               se demande fiche par fiche, par `fiche(id)`.",
        example: "let sans = [];\n\
                  for d in medicaments() { if d.classe == \"\" { sans.push(d.nom); } }\n\
                  `${sans.len} fiche(s) sans classe`\n",
    },
    Call {
        call: "entretiens()",
        returns: "Les actes du suivi, en tableau de cartes.",
        fields: &[
            ("theme", "la thématique, telle qu'elle s'affiche"),
            ("etat", "l'état de l'acte"),
            ("mois", "le mois de création, AAAA-MM"),
            ("minutes", "la durée saisie, en minutes"),
            ("operateur", "les initiales de qui l'a fait"),
        ],
        note: "",
        example: "let par = #{};\n\
                  for a in entretiens() {\n    \
                      let qui = if a.operateur == \"\" { \"(non signé)\" } else { a.operateur };\n    \
                      par[qui] = if qui in par { par[qui] + 1 } else { 1 };\n\
                  }\n\
                  for qui in par.keys() { print(`${qui} : ${par[qui]}`); }\n",
    },
    Call {
        call: "registre()",
        returns: "Les lignes du registre des stupéfiants, les plus récentes.",
        fields: &[
            ("produit", "le libellé de la présentation"),
            ("nature", "entrée, sortie, inventaire, annulation…"),
            ("jour", "le jour de la ligne, AAAA-MM-JJ"),
            ("quantite", "la quantité de la ligne"),
            ("dossier", "le numéro de dossier, jamais le nom"),
        ],
        note: "Une ligne de registre porte le numéro de dossier et non le \
               nom : un registre s'imprime et se laisse sur un comptoir.",
        example: "let par = #{};\n\
                  for l in registre() {\n    \
                      if l.nature != \"SORTIE\" { continue; }\n    \
                      let eu = if l.produit in par { par[l.produit] } else { 0 };\n    \
                      par[l.produit] = eu + l.quantite;\n\
                  }\n\
                  for p in par.keys() { print(`${p} : ${par[p]}`); }\n",
    },
    Call {
        call: "fiche(id)",
        returns: "La prose d'une fiche : une carte de sections.",
        fields: &[],
        note: "Les clés sont celles des sections de la monographie, \
               listées sous « Les sections d'une fiche ». Une section \
               vide n'est pas dans la carte.",
        example: "for d in medicaments() {\n    \
                      let f = fiche(d.id);\n    \
                      if !(\"mono_f_ddi\" in f) { print(d.nom); }\n\
                  }\n",
    },
];

/// Ce que la console **ne peut pas** faire, dit à qui l'ouvre.
///
/// Écrit ici, à côté des bornes qui l'imposent : une phrase rangée dans
/// l'écran d'aide survivrait au jour où le moteur changerait.
pub const LIMITS: &[&str] = &[
    "Un script ne peut rien écrire : la console lit un instantané pris \
     avant l'exécution, et aucune fonction ne modifie la base.",
    "Un script n'a ni fichier, ni réseau, ni processus. Le seul chemin \
     qu'il a vers le monde est le texte qu'il imprime dans le volet.",
    "Un script s'arrête : le moteur est borné en nombre d'opérations, et \
     non en secondes — une horloge ferait passer sur un poste ce qui \
     échoue sur l'autre.",
    "`print(...)` écrit une ligne dans le volet de sortie ; la dernière \
     expression du script y est rendue aussi.",
];

/// Les exemples livrés : ce qu'on ouvre pour comprendre ce que la
/// console sait faire.
///
/// Livrés et non semés : ce sont des exemples, pas du contenu de base.
/// Ils s'ouvrent, se modifient, et ce qu'on en fait s'enregistre sous un
/// autre nom — comme une formule du codex qu'on reprend.
pub const EXAMPLES: &[(&str, &str)] = &[
    (
        "Dossiers éligibles au BPM",
        "// Les dossiers qui portent assez de traitements pour un bilan\n\
         // partagé de médication.\n\
         let n = 0;\n\
         for p in patients() {\n    \
             if p.traitements.len >= 5 {\n        \
                 print(`${p.nom} ${p.prenom} — ${p.traitements.len} traitements`);\n        \
                 n += 1;\n    \
             }\n\
         }\n\
         `${n} dossier(s)`\n",
    ),
    (
        "Fiches sans classe",
        "// Ce qu'il reste à ranger dans le référentiel.\n\
         let sans = [];\n\
         for d in medicaments() {\n    \
             if d.classe == \"\" { sans.push(d.nom); }\n\
         }\n\
         print(`${sans.len} fiche(s) sans classe`);\n\
         for nom in sans { print(nom); }\n",
    ),
    (
        "Entretiens par opérateur",
        "// Qui a fait quoi, tous mois confondus.\n\
         let par = #{};\n\
         for a in entretiens() {\n    \
             let qui = if a.operateur == \"\" { \"(non signé)\" } else { a.operateur };\n    \
             par[qui] = if qui in par { par[qui] + 1 } else { 1 };\n\
         }\n\
         for qui in par.keys() { print(`${qui} : ${par[qui]}`); }\n",
    ),
    (
        "Délivrances par produit",
        "// Les délivrances de stupéfiants, produit par produit.\n\
         let par = #{};\n\
         for l in registre() {\n    \
             if l.nature == \"SORTIE\" {\n        \
                 par[l.produit] = if l.produit in par { par[l.produit] + l.quantite } else { l.quantite };\n    \
             }\n\
         }\n\
         for p in par.keys() { print(`${p} : ${par[p]}`); }\n",
    ),
];

/// Ce qu'une portion de source **est**, pour qui la colore.
///
/// Cinq natures et une neutre : c'est exactement [`motif::CODE_RAMP`],
/// dans le même ordre, et c'est voulu — une teinte de plus est une
/// ligne de plus dans la rampe nommée, jamais une couleur écrite au
/// point où l'on peint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ink {
    Comment,
    Text,
    Number,
    Keyword,
    /// Un appel que le moteur connaît vraiment — lu dans [`API`], et
    /// jamais dans une seconde liste qui vieillirait à côté.
    Known,
    /// Tout le reste : les identifiants, les opérateurs, les espaces.
    Plain,
}

/// Les mots que le langage se réserve.
///
/// Ceux de Rhai, et rien d'autre : un mot colorié qui n'est pas un
/// mot-clé enseigne une syntaxe fausse à qui apprend le langage dans
/// cette console.
pub const KEYWORDS: &[&str] = &[
    "let", "const", "if", "else", "switch", "do", "while", "until", "loop", "for", "in", "break",
    "continue", "return", "fn", "private", "throw", "try", "catch", "import", "export", "as",
    "true", "false", "this", "global",
];

/// Découper une source en portions colorées.
///
/// **Le découpage couvre tout, dans l'ordre, une fois.** C'est la règle
/// dont tout le reste dépend : la galée est reconstruite portion par
/// portion, donc un octet oublié est un caractère qui disparaît de
/// l'écran, et un octet compté deux fois est un caractère qui se
/// dédouble — dans le script qu'on est en train d'écrire, ce qui est la
/// pire chose qu'un colorateur puisse faire. `a_reading_loses_not_one_byte`
/// le vérifie sur tout ce qui est livré.
///
/// Les bornes sont en octets et tombent toujours sur une frontière de
/// caractère : les commentaires de cette base sont en français, et
/// couper « é » en deux fait paniquer le découpage d'une chaîne.
pub fn colour(source: &str) -> Vec<(usize, usize, Ink)> {
    let b = source.as_bytes();
    let mut out: Vec<(usize, usize, Ink)> = Vec::new();
    let mut i = 0;
    // Une portion neutre en attente : on ne la ferme qu'au moment où
    // quelque chose d'autre commence, sinon la liste compterait une
    // entrée par espace.
    let mut plain_from = 0;
    let flush = |out: &mut Vec<(usize, usize, Ink)>, from: usize, to: usize| {
        if to > from {
            out.push((from, to, Ink::Plain));
        }
    };
    while i < b.len() {
        let c = b[i];
        // --- Un commentaire, jusqu'au bout de la ligne ou du bloc ---
        if c == b'/' && i + 1 < b.len() && (b[i + 1] == b'/' || b[i + 1] == b'*') {
            flush(&mut out, plain_from, i);
            let start = i;
            if b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            } else {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                // Un bloc que rien ne ferme va jusqu'au bout : c'est ce
                // que le moteur en fera aussi, et le montrer autrement
                // cacherait la faute.
                i = (i + 2).min(b.len());
            }
            out.push((start, i, Ink::Comment));
            plain_from = i;
            continue;
        }
        // --- Une chaîne, entre guillemets ou en accent grave --------
        if c == b'"' || c == b'`' || c == b'\'' {
            flush(&mut out, plain_from, i);
            let quote = c;
            let start = i;
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    // **Et la borne retombe sur une frontière de
                    // caractère.** Un échappement suivi d'une lettre
                    // accentuée — `"\\é"` — fait sauter deux *octets*,
                    // c'est-à-dire au milieu du « é » : la portion
                    // découpée là fait paniquer le tranchage d'une
                    // chaîne, dans un chemin de dessin, sur ce que
                    // quelqu'un est en train de taper.
                    i = (i + 2).min(b.len());
                    while i < b.len() && !source.is_char_boundary(i) {
                        i += 1;
                    }
                    continue;
                }
                if b[i] == quote {
                    i += 1;
                    break;
                }
                // Une chaîne entre guillemets ne passe pas la ligne, et
                // un caractère entre apostrophes non plus : sans cela un
                // guillemet oublié — ou une apostrophe égarée — colore
                // tout le reste du script. Seul l'accent grave le fait,
                // parce qu'une chaîne interpolée de Rhai s'écrit
                // réellement sur plusieurs lignes.
                if quote != b'`' && b[i] == b'\n' {
                    break;
                }
                i += 1;
            }
            out.push((start, i, Ink::Text));
            plain_from = i;
            continue;
        }
        // --- Un nombre ---------------------------------------------
        if c.is_ascii_digit() && !starts_in_word(b, i) {
            flush(&mut out, plain_from, i);
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'_') {
                i += 1;
            }
            out.push((start, i, Ink::Number));
            plain_from = i;
            continue;
        }
        // --- Un mot : mot-clé, appel connu, ou rien de particulier --
        if c == b'_' || c.is_ascii_alphabetic() {
            let start = i;
            while i < b.len() && (b[i] == b'_' || b[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            let word = &source[start..i];
            let ink = if KEYWORDS.contains(&word) {
                Ink::Keyword
            } else if is_known_call(word) {
                Ink::Known
            } else {
                Ink::Plain
            };
            if ink != Ink::Plain {
                flush(&mut out, plain_from, start);
                out.push((start, i, ink));
                plain_from = i;
            }
            continue;
        }
        i += 1;
    }
    flush(&mut out, plain_from, b.len());
    out
}

/// Un chiffre collé à la fin d'un mot n'est pas un nombre : `p2` est un
/// nom de variable, et le colorer par morceaux fait clignoter la moitié
/// des scripts.
fn starts_in_word(b: &[u8], i: usize) -> bool {
    i > 0 && (b[i - 1] == b'_' || b[i - 1].is_ascii_alphanumeric())
}

/// Ce que le moteur enregistre vraiment, lu dans [`API`].
///
/// Le nom est ce qui précède la parenthèse : la table écrit
/// « patients() », le moteur connaît `patients`. Une seconde liste de
/// noms vieillirait à côté de la première, et c'est la copie qui se
/// tromperait — la règle que ce dépôt applique déjà au manuel.
fn is_known_call(word: &str) -> bool {
    API.iter().any(|c| c.call.split('(').next() == Some(word)) || word == "print" || word == "debug"
}

/// Ce que l'on peut proposer à qui est en train de taper un mot.
///
/// Rend la portion du mot commencé et les propositions, ou rien du
/// tout : **on ne propose pas sur le vide**. Une liste qui s'ouvre dès
/// qu'on pose le curseur cache le code qu'on lit et se referme à la
/// touche suivante ; ce qu'on veut est une liste qui répond à un début
/// de mot.
///
/// Le curseur est donné en octets, et il faut qu'il tombe sur une
/// frontière de caractère : c'est l'appelant qui le convertit depuis le
/// compte de caractères d'egui, une fois, plutôt que deux conversions
/// qui finiraient par diverger.
pub fn suggest(source: &str, caret: usize) -> Option<(usize, usize, Vec<&'static str>)> {
    let caret = caret.min(source.len());
    if !source.is_char_boundary(caret) {
        return None;
    }
    let b = source.as_bytes();
    let mut start = caret;
    while start > 0 && (b[start - 1] == b'_' || b[start - 1].is_ascii_alphanumeric()) {
        start -= 1;
    }
    // Un mot qui commence par un chiffre est un nombre, pas un nom.
    if start == caret || b[start].is_ascii_digit() {
        return None;
    }
    let typed = &source[start..caret];
    // Déjà complet : proposer à quelqu'un le mot qu'il vient de finir
    // d'écrire est une liste d'une ligne qu'il faut fermer.
    let mut found: Vec<&'static str> = Vec::new();
    for call in API {
        let name = call.call;
        if name.starts_with(typed) && name.len() > typed.len() {
            found.push(name);
        }
    }
    for word in KEYWORDS {
        if word.starts_with(typed) && word.len() > typed.len() {
            found.push(word);
        }
    }
    if found.is_empty() {
        None
    } else {
        Some((start, caret, found))
    }
}

/// Ce qu'on écrit vraiment quand on accepte une proposition.
///
/// **Une parenthèse vide s'écrit en entier, une parenthèse qui attend
/// quelque chose s'ouvre et s'arrête.** `patients()` est complet tel
/// quel ; écrire `fiche(id)` poserait le mot `id` dans le script, qui
/// n'est pas une variable de celui qui tape — il faudrait l'effacer
/// avant d'écrire le numéro, c'est-à-dire défaire ce que la complétion
/// vient de faire.
pub fn insertion(candidate: &str) -> String {
    match candidate.split_once('(') {
        Some((name, rest)) if rest.trim_end_matches(')').is_empty() => format!("{name}()"),
        Some((name, _)) => format!("{name}("),
        None => candidate.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Snapshot {
        Snapshot {
            patients: vec![
                Patient {
                    id: 1,
                    nom: "Dupont".into(),
                    prenom: "Jean".into(),
                    naissance: "1958-07-03".into(),
                    traitements: vec!["Eliquis".into(), "Coversyl".into()],
                },
                Patient {
                    id: 2,
                    nom: "Martin".into(),
                    prenom: "Claire".into(),
                    naissance: "1949-02-11".into(),
                    traitements: vec![
                        "Coversyl".into(),
                        "Lasilix".into(),
                        "Advil".into(),
                        "Aricept".into(),
                        "Ditropan".into(),
                    ],
                },
            ],
            drugs: vec![
                Drug {
                    id: 10,
                    nom: "Eliquis".into(),
                    dci: "apixaban".into(),
                    classe: "AOD".into(),
                    ..Drug::default()
                },
                Drug {
                    id: 11,
                    nom: "Sans classe".into(),
                    ..Drug::default()
                },
            ],
            acts: vec![
                Act {
                    theme: "BPM".into(),
                    etat: "BILLED".into(),
                    mois: "2026-08".into(),
                    minutes: 45,
                    operateur: "CL".into(),
                },
                Act {
                    theme: "AOD".into(),
                    etat: "PERFORMED".into(),
                    mois: "2026-09".into(),
                    minutes: 20,
                    operateur: "CL".into(),
                },
            ],
            moves: vec![Move {
                produit: "Skenan LP 30 mg".into(),
                nature: "SORTIE".into(),
                jour: "2026-08-12".into(),
                quantite: 14.0,
                dossier: 1,
            }],
            prose: [(
                10_i64,
                vec![("indications".to_owned(), "FA non valvulaire".to_owned())],
            )]
            .into_iter()
            .collect(),
        }
    }

    /// La console lit la base et rend ce qu'elle a écrit.
    #[test]
    fn a_script_reads_the_base_and_says_what_it_found() {
        let out = run(
            "let n = 0;\n\
             for p in patients() { if p.traitements.len >= 5 { print(p.nom); n += 1; } }\n\
             n",
            &sample(),
        );
        assert_eq!(out.error, None, "{out:?}");
        assert_eq!(out.printed, vec!["Martin".to_owned()]);
        assert_eq!(out.value, "1");
        // Les quatre tables sont là, et la prose se demande fiche par
        // fiche.
        let out = run(
            "[medicaments().len, entretiens().len, registre().len, fiche(10).indications]",
            &sample(),
        );
        assert_eq!(out.error, None, "{out:?}");
        assert_eq!(out.value, "[2, 2, 1, \"FA non valvulaire\"]");
        // Une fiche dont on n'a pas donné la prose rend une carte vide,
        // et non une erreur : une question posée sur ce qu'on n'a pas
        // n'est pas une faute de syntaxe.
        // (`len()` et non `.len` : une carte Rhai a une méthode, un
        // tableau a une propriété.)
        assert_eq!(run("fiche(999).len()", &sample()).value, "0");
    }

    /// **Le script s'arrête.** `while true {}` fait trois caractères, et
    /// une application de comptoir ne se rouvre pas d'un clic.
    ///
    /// Borné en **opérations** et non en secondes : une horloge ferait
    /// passer le même script sur un poste et échouer sur l'autre, ce qui
    /// est la pire façon d'échouer.
    #[test]
    fn a_runaway_script_stops_instead_of_taking_the_counter_with_it() {
        let out = run("while true { }", &sample());
        let err = out.error.expect("une boucle infinie s'arrête");
        assert!(
            err.to_lowercase().contains("operation"),
            "l'erreur dit pourquoi : {err}"
        );
        // Et ce qui a été écrit avant l'erreur est rendu quand même :
        // c'est souvent ce qui dit où le script s'est trompé.
        let out = run("print(\"avant\"); let x = 1/0;", &sample());
        assert!(out.error.is_some());
        assert_eq!(out.printed, vec!["avant".to_owned()]);
        // Une faute de frappe est une erreur, pas une panique.
        assert!(run("cette ligne n'est pas du rhai", &sample())
            .error
            .is_some());
        assert!(run("", &sample()).error.is_none());
    }

    /// **Rien ne sort et rien ne s'écrit.**
    ///
    /// Le moteur n'a aucune entrée-sortie et rien ici n'en ajoute : le
    /// seul chemin d'un script vers le monde est ce qu'il imprime. Ce
    /// test tient la porte fermée — le jour où l'on enregistrerait une
    /// fonction de trop, il le dirait.
    #[test]
    fn a_script_can_neither_read_a_file_nor_write_to_the_base() {
        for attempt in [
            "open(\"/etc/passwd\")",
            "read_file(\"/etc/passwd\")",
            "system(\"ls\")",
            "exec(\"ls\")",
            "http(\"https://example.org\")",
            // Et rien qui écrive : la console lit un instantané, et le
            // registre des stupéfiants est inaltérable par construction.
            "ajouter_patient(\"X\")",
            "supprimer_fiche(1)",
            "ecrire_registre(1, 1.0)",
        ] {
            let out = run(attempt, &sample());
            assert!(
                out.error.is_some(),
                "« {attempt} » ne doit pas être une fonction de la console"
            );
        }
        // Modifier le tableau rendu ne modifie pas l'instantané : c'est
        // une copie, et l'appel suivant repart de la base.
        let out = run("let a = patients(); a.clear(); patients().len", &sample());
        assert_eq!(out.value, "2");
    }

    /// Les exemples livrés s'exécutent, tous, sans erreur.
    ///
    /// C'est le seul « contenu » de ce module, et un exemple qui ne
    /// tourne pas apprend le contraire de ce qu'il montre. Ils sont
    /// éprouvés sur un instantané où chaque table porte quelque chose,
    /// parce qu'un exemple qui ne se casse que sur une base pleine se
    /// casserait chez l'officine et nulle part ici.
    /// **Toute fonction offerte est décrite, et toute description
    /// désigne une fonction offerte.**
    ///
    /// La description vit à côté de l'enregistrement, et ce test est ce
    /// qui l'y tient : il lit le texte de ce module, y cherche les
    /// `register_fn` et compare. Sans lui, ajouter une fonction sans la
    /// décrire donnerait une console qui sait faire une chose que
    /// l'explorateur ignore, et retirer une fonction décrite donnerait
    /// un exemple qui ne s'exécute plus — les deux en silence.
    #[test]
    fn every_call_the_console_offers_is_described() {
        let source = include_str!("script.rs");
        // Assemblé, comme ailleurs dans la maison : un test qui se lit
        // lui-même trouverait d'abord son propre repère.
        let marker = concat!("register_", "fn(\"");
        let mut registered: Vec<&str> = Vec::new();
        let mut rest = source;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at + marker.len()..];
            if let Some(end) = rest.find('"') {
                registered.push(&rest[..end]);
            }
        }
        registered.sort_unstable();
        let mut described: Vec<&str> = API
            .iter()
            .map(|c| c.call.split('(').next().unwrap_or(c.call))
            .collect();
        described.sort_unstable();
        assert_eq!(
            described, registered,
            "les fonctions décrites et celles qu'on enregistre"
        );
        assert!(!registered.is_empty(), "le repère lui-même doit marcher");

        // Chaque description porte de quoi s'en servir, et son exemple
        // s'exécute : un exemple qui échoue est pire que pas d'exemple,
        // puisqu'on l'essaie avant de lire.
        let data = sample();
        for c in API {
            assert!(!c.returns.is_empty(), "{}", c.call);
            assert!(!c.example.trim().is_empty(), "{}", c.call);
            let out = run(c.example, &data);
            assert!(out.error.is_none(), "{} : {:?}", c.call, out.error);
        }
        assert!(!LIMITS.is_empty());
    }

    #[test]
    fn every_shipped_example_runs() {
        for (name, source) in EXAMPLES {
            let out = run(source, &sample());
            assert_eq!(out.error, None, "« {name} » : {out:?}");
        }
        // Et sur une base vide, qui est l'état d'un premier lancement.
        for (name, source) in EXAMPLES {
            let out = run(source, &Snapshot::default());
            assert_eq!(out.error, None, "« {name} » à vide : {out:?}");
        }
    }

    /// **Une lecture ne perd pas un octet.** La galée est reconstruite
    /// portion par portion : un octet oublié est un caractère qui
    /// disparaît du script qu'on est en train d'écrire, un octet compté
    /// deux fois un caractère qui se dédouble. Vérifié sur tout ce qui
    /// est livré — les exemples et les exemples de l'API —, plus les
    /// formes qui se trompent, qui sont celles qu'on tape le plus.
    #[test]
    fn a_reading_loses_not_one_byte() {
        let mut sources: Vec<String> = EXAMPLES.iter().map(|(_, s)| (*s).to_owned()).collect();
        sources.extend(API.iter().map(|c| c.example.to_owned()));
        sources.extend(
            [
                "",
                "let x = 1;",
                "// un commentaire accentué : périmé, déjà réglé\n",
                "/* jamais fermé",
                "`une chaîne ${interpolée}`",
                "\"un guillemet oublié",
                "let p2 = 12; let x3 = 0.5;",
                "print(\"éàü\"); // et après",
                "let n = 0; // l'apostrophe égarée",
                // Un échappement devant une lettre accentuée : deux
                // octets sautés tombent au milieu du « é ».
                "print(\"\\é\"); let n = 1;",
            ]
            .iter()
            .map(|s| (*s).to_owned()),
        );
        for src in &sources {
            let spans = colour(src);
            let mut at = 0;
            let mut rebuilt = String::new();
            for (from, to, _) in &spans {
                assert_eq!(*from, at, "un trou ou un recouvrement dans « {src} »");
                assert!(from <= to && *to <= src.len(), "des bornes folles");
                assert!(src.is_char_boundary(*from) && src.is_char_boundary(*to));
                rebuilt.push_str(&src[*from..*to]);
                at = *to;
            }
            assert_eq!(at, src.len(), "la fin manque dans « {src} »");
            assert_eq!(&rebuilt, src);
        }
    }

    /// **Une apostrophe égarée ne colore pas la suite du script.**
    ///
    /// Un caractère entre apostrophes ne passe pas la ligne, pas plus
    /// qu'une chaîne entre guillemets : sans cette borne, le « l' » d'un
    /// commentaire mal placé ou d'une chaîne mal fermée peignait tout ce
    /// qui suit en couleur de chaîne — et le reste du script devenait
    /// illisible pour une apostrophe. Seul l'accent grave franchit la
    /// ligne, parce qu'une chaîne interpolée de Rhai s'écrit réellement
    /// sur plusieurs.
    #[test]
    fn a_stray_quote_stops_at_the_end_of_its_line() {
        let src = "let s = \"pas fermée\nlet n = 1;\n";
        let spans = colour(src);
        // La ligne suivante retrouve ses couleurs : « let » y est un
        // mot-clé et « 1 » un nombre.
        assert!(spans
            .iter()
            .any(|(f, t, ink)| *ink == Ink::Keyword && &src[*f..*t] == "let" && *f > 10));
        assert!(spans.iter().any(|(_, _, ink)| *ink == Ink::Number));
        // L'accent grave, lui, tient sur plusieurs lignes.
        let multi = "let s = `une\nchaîne`;\n";
        assert!(colour(multi)
            .iter()
            .any(|(f, t, ink)| *ink == Ink::Text && multi[*f..*t].chars().count() > 10));
    }

    /// **On ne colore en appel que ce que le moteur connaît.** Un nom
    /// colorié qui n'existe pas enseigne une API fausse à qui apprend le
    /// langage dans cette console, et c'est justement là qu'on
    /// l'apprend.
    #[test]
    fn only_a_call_the_engine_answers_is_painted_as_one() {
        let known = "for p in patients() { print(p.nom); }";
        assert!(colour(known)
            .iter()
            .any(|(f, t, ink)| *ink == Ink::Known && &known[*f..*t] == "patients"));
        let made_up = "fournisseurs()";
        assert!(!colour(made_up).iter().any(|(_, _, ink)| *ink == Ink::Known));
        // Et un mot-clé reste un mot-clé, pas un appel.
        assert!(colour("let x = 1;")
            .iter()
            .any(|(f, t, ink)| *ink == Ink::Keyword && &"let x = 1;"[*f..*t] == "let"));
    }

    /// **On ne propose rien sur le vide.** Une liste qui s'ouvre dès
    /// qu'on pose le curseur cache le code qu'on lit ; ce qu'on veut est
    /// une liste qui répond à un début de mot — et qui se tait quand le
    /// mot est fini.
    #[test]
    fn nothing_is_suggested_until_a_word_is_begun() {
        assert_eq!(suggest("", 0), None);
        assert_eq!(suggest("for p in ", 9), None);
        assert_eq!(suggest("let x = 12", 10), None);
        let (from, to, found) = suggest("pat", 3).expect("« pat » commence un mot");
        assert_eq!((from, to), (0, 3));
        assert!(found.contains(&"patients()"));
        // Un mot déjà entier ne se propose pas lui-même.
        assert_eq!(suggest("let", 3), None);
        // Et la proposition s'écrit sans poser de mot qui n'est pas à
        // celui qui tape.
        assert_eq!(insertion("patients()"), "patients()");
        assert_eq!(insertion("fiche(id)"), "fiche(");
        assert_eq!(insertion("let"), "let");
    }
}
