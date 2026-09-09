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
/// Assez pour parcourir huit cent cinquante fiches et mille dossiers
/// plusieurs fois ; pas assez pour une boucle infinie. Un compte
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
    // La prose d'une fiche, demandée une par une : la charger pour les
    // huit cent cinquante recopierait quelques mégaoctets à chaque
    // exécution pour une question qui porte presque toujours sur le nom.
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
        "Ce qui est sorti du registre",
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
}
