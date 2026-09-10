//! L'adaptation rénale, comme règle et non comme paragraphe.
//!
//! `biology.rs` connaît le DFG et sait dire « ce chiffre-là, sous ce
//! traitement-là, veut dire ceci ». `surveillance.rs` sait dire « ce
//! chiffre n'a pas été demandé depuis trop longtemps ». Il manquait la
//! troisième question, qui est celle qu'on pose vraiment au comptoir :
//! **ce dossier porte un DFG à 28 ; que devient chaque ligne de son
//! ordonnance ?**
//!
//! Aujourd'hui le pharmacien lit deux paragraphes — le « rein » de la
//! fiche, le chiffre du laboratoire — et rapproche les deux de tête,
//! ligne par ligne, sur une ordonnance qui en compte huit. C'est
//! l'erreur de comptoir la plus fréquente, et l'application avait déjà
//! les deux moitiés sans jamais les mettre l'une en face de l'autre.
//!
//! Quatre règles le tiennent, une par test :
//!
//! * **Sans DFG, pas de verdict.** Le module dit alors ce que
//!   l'ordonnance porte de dépendant du rein, et **que le chiffre
//!   manque** — jamais ce qu'il faudrait faire. Un logiciel qui
//!   annoncerait « contre-indiqué » sans avoir vu de clairance dirait
//!   une chose qu'il ne sait pas. C'est la règle de l'écart de caisse
//!   sans recette attendue, et celle du creux sans horaires déclarés.
//! * **Le palier atteint est le plus bas des paliers franchis.** Une
//!   molécule qui se réduit sous 60 et se contre-indique sous 30, lue à
//!   28, est contre-indiquée : prendre le premier palier de la liste
//!   dirait « réduire la dose » d'un traitement qu'il faut arrêter.
//! * **Un seuil vient du RCP, jamais d'une interpolation.** Le module
//!   ne calcule pas une dose à partir d'une clairance : il répète le
//!   palier écrit, et la dose dépend aussi de l'indication, du poids et
//!   de l'âge.
//! * **La conduite est celle du résumé des caractéristiques, la
//!   décision est celle du prescripteur.** Le module ne modifie rien,
//!   ne propose pas d'ordonnance, et chaque ligne porte sa source.
//!
//! Statique, pur et testé, comme `biology`, `revue` et `surveillance`.
//! Il ne connaît ni la base ni egui : on lui passe des traitements et
//! un chiffre.

/// Ce qu'un palier demande. L'ordre est celui de la gravité, et c'est
/// lui qui trie l'affichage : une contre-indication ne se lit pas après
/// une surveillance.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    /// Le traitement ne doit pas être poursuivi à ce niveau.
    Contraindicated,
    /// La dose se réduit, selon ce que dit la conduite.
    Reduce,
    /// Ni l'un ni l'autre : à surveiller de plus près.
    Watch,
}

impl Level {
    /// La clé écrite nulle part — ce module ne stocke rien — mais qui
    /// nomme le niveau dans les tests et les libellés.
    pub fn label(self) -> &'static str {
        match self {
            Level::Contraindicated => "Contre-indiqué",
            Level::Reduce => "Dose à réduire",
            Level::Watch => "À surveiller",
        }
    }
}

/// Un palier : au-dessous de ce DFG, le traitement devient ceci.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    /// En mL/min. Le palier s'applique **strictement au-dessous**.
    pub below: u16,
    pub level: Level,
    /// Ce que le RCP dit, en une phrase de comptoir. Jamais une dose en
    /// milligrammes : elle dépend aussi de l'indication.
    pub conduct: &'static str,
}

/// Une molécule ou une classe, et ce que le rein lui fait.
pub struct Adaptation {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes —
    /// repliés par `fuzzy::sort_key`, comme partout ici.
    pub needs: &'static [&'static str],
    /// Ce que la ligne annonce : « Metformine », « AINS ».
    pub label: &'static str,
    /// **Du seuil le plus haut au plus bas.** L'ordre est celui de la
    /// lecture ; le calcul, lui, ne s'y fie pas (voir [`read`]).
    pub steps: &'static [Step],
    /// D'où vient le seuil.
    pub source: &'static str,
}

/// Ce que le module rend pour une ligne d'ordonnance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    /// Le nom tel qu'il est écrit au dossier.
    pub treatment: String,
    /// Ce que la table nomme.
    pub label: &'static str,
    /// `None` quand aucun DFG n'est connu : la ligne dit alors que le
    /// traitement dépend du rein et que le chiffre manque. **Ce n'est
    /// pas un verdict**, et le type est ce qui l'empêche d'en devenir un.
    pub level: Option<Level>,
    /// Le seuil franchi, quand il y en a un.
    pub below: Option<u16>,
    pub conduct: &'static str,
    pub source: &'static str,
}

impl Finding {
    /// La ligne porte un verdict.
    pub fn decided(&self) -> bool {
        self.level.is_some()
    }
}

/// Ce que le rein fait à cette ordonnance, à ce DFG.
///
/// `dfg` est en mL/min. `None` — aucune clairance au dossier — n'est
/// **pas** zéro : le module rend alors les traitements concernés sans
/// verdict, et c'est à la vue de dire que le chiffre manque.
///
/// L'ordre est celui de la gravité, puis celui du dossier : deux lignes
/// de même niveau ne doivent pas échanger leur place d'une image à
/// l'autre.
pub fn read(treatments: &[crate::revue::Treatment], dfg: Option<f64>) -> Vec<Finding> {
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
            // **Le palier atteint est le plus bas des paliers
            // franchis.** Pas le premier de la liste : une molécule qui
            // se réduit sous 60 et se contre-indique sous 30, lue à 28,
            // est contre-indiquée. Le premier de la liste dirait
            // « réduire la dose » d'un traitement qu'il faut arrêter.
            let hit = dfg.and_then(|v| {
                a.steps
                    .iter()
                    .filter(|s| v < f64::from(s.below))
                    .min_by_key(|s| s.below)
            });
            // Sans DFG, la ligne existe et ne conclut pas. Avec un DFG
            // au-dessus de tous les seuils, il n'y a rien à dire du
            // tout — et le silence est la bonne réponse.
            if dfg.is_some() && hit.is_none() {
                continue;
            }
            out.push(Finding {
                treatment: t.name.trim().to_owned(),
                label: a.label,
                level: hit.map(|s| s.level),
                below: hit.map(|s| s.below),
                conduct: hit.map_or(
                    "Ce traitement s'adapte à la fonction rénale ; aucun DFG n'est noté au dossier.",
                    |s| s.conduct,
                ),
                source: a.source,
            });
            break;
        }
    }
    out.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then(a.below.cmp(&b.below))
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

/// Combien de lignes de cette ordonnance ne peuvent pas être jugées
/// faute de chiffre.
pub fn undecided(findings: &[Finding]) -> usize {
    findings.iter().filter(|f| !f.decided()).count()
}

/// Ce que le rein change, molécule par molécule.
///
/// **Rien ici n'est inventé** : chaque seuil est celui du résumé des
/// caractéristiques du produit ou de la recommandation citée, et les
/// molécules retenues sont celles dont le seuil est net et l'usage
/// courant au comptoir. Une molécule dont le RCP dit « prudence » sans
/// chiffre n'y est pas : une ligne sans seuil ne serait pas une règle,
/// et la fiche dit déjà « prudence » dans sa prose.
///
/// Les paliers sont écrits du plus haut au plus bas, pour la lecture.
pub const TABLE: &[Adaptation] = &[
    Adaptation {
        needs: &["metformine", "biguanide", "glucophage", "stagid"],
        label: "Metformine",
        steps: &[
            Step {
                below: 60,
                level: Level::Watch,
                conduct: "Entre 45 et 60 : poursuivre, contrôler le DFG au moins deux fois par an.",
            },
            Step {
                below: 45,
                level: Level::Reduce,
                conduct: "Entre 30 et 45 : dose maximale réduite de moitié, pas d'instauration à ce niveau, DFG tous les trois à six mois.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication, risque d'acidose lactique. À suspendre aussi devant toute déshydratation ou avant un produit de contraste iodé.",
            },
        ],
        source: "RCP metformine ; HAS, diabète de type 2",
    },
    Adaptation {
        needs: &["dabigatran", "pradaxa"],
        label: "Dabigatran",
        steps: &[
            Step {
                below: 50,
                level: Level::Reduce,
                conduct: "Entre 30 et 50 : dose réduite selon l'âge et l'indication, et le risque hémorragique se réévalue.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication. C'est l'AOD le plus dépendant du rein — 80 % d'élimination rénale.",
            },
        ],
        source: "RCP dabigatran",
    },
    Adaptation {
        needs: &["rivaroxaban", "xarelto"],
        label: "Rivaroxaban",
        steps: &[
            Step {
                below: 50,
                level: Level::Reduce,
                conduct: "Entre 15 et 49 : dose réduite dans la fibrillation auriculaire.",
            },
            Step {
                below: 15,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 15 : non recommandé, données absentes.",
            },
        ],
        source: "RCP rivaroxaban",
    },
    Adaptation {
        needs: &["apixaban", "eliquis"],
        label: "Apixaban",
        steps: &[
            Step {
                below: 30,
                level: Level::Reduce,
                conduct: "Entre 15 et 29 : dose réduite. Le critère de réduction habituel reste celui des trois facteurs — âge, poids, créatinine.",
            },
            Step {
                below: 15,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 15 : non recommandé, données absentes.",
            },
        ],
        source: "RCP apixaban",
    },
    Adaptation {
        needs: &["edoxaban", "lixiana"],
        label: "Édoxaban",
        steps: &[
            Step {
                below: 50,
                level: Level::Reduce,
                conduct: "Entre 15 et 50 : dose réduite.",
            },
            Step {
                below: 15,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 15 : non recommandé.",
            },
        ],
        source: "RCP édoxaban",
    },
    Adaptation {
        needs: &[
            "ains",
            "ibuprofene",
            "diclofenac",
            "ketoprofene",
            "naproxene",
            "celecoxib",
            "piroxicam",
        ],
        label: "AINS",
        steps: &[
            Step {
                below: 60,
                level: Level::Watch,
                conduct: "Au-dessous de 60 : à éviter, surtout avec un IEC ou un sartan et un diurétique — c'est la triade qui fait l'insuffisance rénale aiguë. Jamais au long cours sans avis.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication.",
            },
        ],
        source: "RCP des AINS ; ANSM, triade néfaste",
    },
    Adaptation {
        needs: &["nitrofurantoine", "furadantine"],
        label: "Nitrofurantoïne",
        steps: &[Step {
            below: 45,
            level: Level::Contraindicated,
            conduct: "Au-dessous de 45 : contre-indication. Elle n'atteint plus l'urine à concentration utile, et sa toxicité, elle, reste.",
        }],
        source: "RCP nitrofurantoïne",
    },
    Adaptation {
        needs: &["colchicine", "colchimax"],
        label: "Colchicine",
        steps: &[
            Step {
                below: 60,
                level: Level::Reduce,
                conduct: "Au-dessous de 60 : dose réduite. Marge thérapeutique étroite — la diarrhée est le premier signe de surdosage et impose l'arrêt.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication.",
            },
        ],
        source: "RCP colchicine ; ANSM, mise au point 2016",
    },
    Adaptation {
        needs: &["methotrexate", "novatrex", "imeth"],
        label: "Méthotrexate",
        steps: &[
            Step {
                below: 60,
                level: Level::Reduce,
                conduct: "Au-dessous de 60 : dose réduite et surveillance rapprochée ; l'élimination est rénale et le risque est hématologique.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication.",
            },
        ],
        source: "RCP méthotrexate",
    },
    Adaptation {
        needs: &["spironolactone", "aldactone", "eplerenone", "inspra"],
        label: "Anti-aldostérone",
        steps: &[
            Step {
                below: 60,
                level: Level::Watch,
                conduct: "Au-dessous de 60 : kaliémie et créatinine à une semaine de toute instauration ou augmentation.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication, risque d'hyperkaliémie.",
            },
        ],
        source: "RCP spironolactone et éplérénone",
    },
    Adaptation {
        needs: &["allopurinol", "zyloric"],
        label: "Allopurinol",
        steps: &[Step {
            below: 60,
            level: Level::Reduce,
            conduct: "Au-dessous de 60 : dose adaptée à la clairance et titration lente. C'est une dose initiale trop forte qui fait les toxidermies graves.",
        }],
        source: "RCP allopurinol ; EULAR, goutte",
    },
    Adaptation {
        needs: &["alendronate", "risedronate", "acide zoledronique", "bisphosphonate"],
        label: "Bisphosphonates",
        steps: &[Step {
            below: 35,
            level: Level::Contraindicated,
            conduct: "Au-dessous de 35 : contre-indication (30 pour certains). Vérifier aussi la calcémie et la vitamine D avant toute reprise.",
        }],
        source: "RCP des bisphosphonates oraux",
    },
    Adaptation {
        needs: &["gabapentine", "neurontin", "pregabaline", "lyrica"],
        label: "Gabapentinoïdes",
        steps: &[Step {
            below: 60,
            level: Level::Reduce,
            conduct: "Au-dessous de 60 : dose adaptée à la clairance. L'élimination est rénale et pure ; l'accumulation donne somnolence, confusion et chutes.",
        }],
        source: "RCP gabapentine et prégabaline",
    },
    Adaptation {
        needs: &["baclofene", "liorésal"],
        label: "Baclofène",
        steps: &[Step {
            below: 60,
            level: Level::Reduce,
            conduct: "Au-dessous de 60 : dose réduite. L'encéphalopathie par accumulation est décrite dès une insuffisance rénale modérée.",
        }],
        source: "RCP baclofène ; ANSM",
    },
    Adaptation {
        needs: &["digoxine", "hemigoxine", "digitalique"],
        label: "Digoxine",
        steps: &[Step {
            below: 60,
            level: Level::Reduce,
            conduct: "Au-dessous de 60 : dose réduite et digoxinémie. Marge thérapeutique étroite, et l'hypokaliémie majore la toxicité à digoxinémie inchangée.",
        }],
        source: "RCP digoxine",
    },
    Adaptation {
        needs: &["valaciclovir", "aciclovir", "zelitrex", "zovirax"],
        label: "Aciclovir et valaciclovir",
        steps: &[Step {
            below: 50,
            level: Level::Reduce,
            conduct: "Au-dessous de 50 : dose et espacement adaptés, et hydratation. L'accumulation donne confusion et hallucinations, surtout chez la personne âgée.",
        }],
        source: "RCP aciclovir et valaciclovir",
    },
    Adaptation {
        needs: &["fenofibrate", "lipanthyl", "bezafibrate", "fibrate"],
        label: "Fibrates",
        steps: &[
            Step {
                below: 60,
                level: Level::Reduce,
                conduct: "Au-dessous de 60 : dose réduite.",
            },
            Step {
                below: 30,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 30 : contre-indication, risque de rhabdomyolyse.",
            },
        ],
        source: "RCP fénofibrate",
    },
    Adaptation {
        needs: &["rosuvastatine", "crestor"],
        label: "Rosuvastatine",
        steps: &[Step {
            below: 30,
            level: Level::Contraindicated,
            conduct: "Au-dessous de 30 : contre-indication. Les autres statines s'utilisent, à dose prudente.",
        }],
        source: "RCP rosuvastatine",
    },
    Adaptation {
        needs: &["cotrimoxazole", "bactrim", "sulfamethoxazole"],
        label: "Cotrimoxazole",
        steps: &[
            Step {
                below: 30,
                level: Level::Reduce,
                conduct: "Entre 15 et 30 : dose réduite de moitié, et kaliémie — il fait monter le potassium et la créatinine.",
            },
            Step {
                below: 15,
                level: Level::Contraindicated,
                conduct: "Au-dessous de 15 : contre-indication.",
            },
        ],
        source: "RCP cotrimoxazole",
    },
    Adaptation {
        needs: &["morphine", "skenan", "actiskenan", "oramorph", "moscontin"],
        label: "Morphine",
        steps: &[Step {
            below: 30,
            level: Level::Reduce,
            conduct: "Au-dessous de 30 : dose réduite et intervalle allongé. Ce sont les métabolites actifs qui s'accumulent, pas la morphine — la sédation vient à retard.",
        }],
        source: "RCP morphine ; SFAP, douleur et insuffisance rénale",
    },
    Adaptation {
        needs: &["tramadol", "contramal", "topalgic", "ixprim"],
        label: "Tramadol",
        steps: &[Step {
            below: 30,
            level: Level::Reduce,
            conduct: "Au-dessous de 30 : intervalle allongé à douze heures, formes à libération prolongée déconseillées.",
        }],
        source: "RCP tramadol",
    },
    Adaptation {
        needs: &["atenolol", "tenormine", "sotalol", "sotalex"],
        label: "Aténolol et sotalol",
        steps: &[Step {
            below: 60,
            level: Level::Reduce,
            conduct: "Au-dessous de 60 : dose réduite — ces deux bêtabloquants-là s'éliminent par le rein, au contraire du bisoprolol ou du métoprolol.",
        }],
        source: "RCP aténolol et sotalol",
    },
    Adaptation {
        needs: &["amoxicilline", "clamoxyl", "augmentin"],
        label: "Amoxicilline",
        steps: &[Step {
            below: 30,
            level: Level::Reduce,
            conduct: "Au-dessous de 30 : dose et intervalle adaptés. Fortes doses prolongées : risque de cristallurie et de neurotoxicité.",
        }],
        source: "RCP amoxicilline",
    },
    Adaptation {
        needs: &["lithium", "teralithe"],
        label: "Lithium",
        steps: &[Step {
            below: 60,
            level: Level::Watch,
            conduct: "Au-dessous de 60 : lithiémie rapprochée. Toute déshydratation, tout AINS et tout IEC fait monter la lithiémie — la marge est étroite.",
        }],
        source: "RCP lithium",
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

    /// **Sans DFG, pas de verdict.**
    ///
    /// La ligne existe — elle dit que le traitement dépend du rein et
    /// que le chiffre manque — et elle ne conclut pas. Le type est ce
    /// qui l'empêche : `level` vaut `None`, et il n'y a nulle part où
    /// écrire « contre-indiqué » sans avoir vu de clairance.
    #[test]
    fn without_a_clearance_there_is_no_verdict() {
        let ordo = [treat("Metformine"), treat("Doliprane")];
        let found = read(&ordo, None);
        assert_eq!(found.len(), 1, "seule la metformine dépend du rein");
        assert_eq!(found[0].level, None);
        assert_eq!(found[0].below, None);
        assert!(!found[0].decided());
        assert_eq!(undecided(&found), 1);
        // Et la phrase dit le manque plutôt que la conduite.
        assert!(found[0].conduct.contains("aucun DFG"));
    }

    /// **Le palier atteint est le plus bas des paliers franchis.**
    ///
    /// La metformine se surveille sous 60, se réduit sous 45 et se
    /// contre-indique sous 30. Lue à 28, elle est contre-indiquée :
    /// prendre le premier palier de la liste dirait « réduire la dose »
    /// d'un traitement qu'il faut arrêter.
    #[test]
    fn the_lowest_step_crossed_is_the_one_that_speaks() {
        let ordo = [treat("Metformine")];
        let at = |v: f64| read(&ordo, Some(v))[0].clone();
        assert_eq!(at(28.0).level, Some(Level::Contraindicated));
        assert_eq!(at(28.0).below, Some(30));
        assert_eq!(at(38.0).level, Some(Level::Reduce));
        assert_eq!(at(38.0).below, Some(45));
        assert_eq!(at(52.0).level, Some(Level::Watch));
        assert_eq!(at(52.0).below, Some(60));
        // Le seuil s'applique **strictement au-dessous** : à 30 pile, on
        // n'est pas contre-indiqué.
        assert_eq!(at(30.0).level, Some(Level::Reduce));
    }

    /// Un DFG au-dessus de tous les seuils ne dit rien du tout, et le
    /// silence est la bonne réponse : une ligne « rien à signaler » par
    /// traitement ferait huit lignes à lire pour n'apprendre rien.
    #[test]
    fn a_normal_clearance_says_nothing() {
        let ordo = [treat("Metformine"), treat("Eliquis")];
        assert!(read(&ordo, Some(92.0)).is_empty());
        // Et un traitement que la table ne connaît pas ne dit jamais
        // rien, quel que soit le chiffre.
        assert!(read(&[treat("Doliprane")], Some(12.0)).is_empty());
    }

    /// L'ordre est celui de la gravité, puis du seuil, puis du nom :
    /// une contre-indication ne se lit pas après une surveillance, et
    /// deux lignes de même niveau ne changent pas de place d'une image
    /// à l'autre.
    #[test]
    fn the_worst_is_read_first_and_the_order_is_total() {
        let ordo = [
            treat("Lithium"),
            treat("Ibuprofène"),
            treat("Metformine"),
            treat("Gabapentine"),
        ];
        let found = read(&ordo, Some(28.0));
        let levels: Vec<Level> = found.iter().filter_map(|f| f.level).collect();
        assert_eq!(
            levels,
            vec![
                Level::Contraindicated,
                Level::Contraindicated,
                Level::Reduce,
                Level::Watch
            ]
        );
        // Les deux contre-indications sont triées par seuil puis par
        // nom : même entrée, même sortie, à chaque image.
        let names: Vec<&str> = found.iter().map(|f| f.treatment.as_str()).collect();
        assert_eq!(names[0], "Ibuprofène", "seuil 30, avant metformine à 30");
        assert!(names.contains(&"Metformine"));
    }

    /// Une ligne se reconnaît par la DCI et par la classe autant que
    /// par le nom : une officine écrit « Xarelto », une autre
    /// « rivaroxaban », et la fiche porte les deux.
    #[test]
    fn a_treatment_is_recognised_by_any_of_its_names() {
        let brand = crate::revue::Treatment {
            name: "Xarelto",
            dci: "",
            class: "",
            tags: "",
        };
        let by_dci = crate::revue::Treatment {
            name: "Générique",
            dci: "rivaroxaban",
            class: "",
            tags: "",
        };
        for t in [brand, by_dci] {
            let found = read(&[t], Some(12.0));
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].level, Some(Level::Contraindicated));
        }
    }

    /// **La table est une règle, pas de la prose.** Chaque entrée porte
    /// des mots à chercher, un libellé, au moins un palier et une
    /// source ; les paliers sont écrits du plus haut au plus bas et ne
    /// se répètent pas ; et une conduite ne contient jamais de dose en
    /// milligrammes — elle dépend aussi de l'indication, du poids et de
    /// l'âge, et un chiffre écrit ici serait lu comme une prescription.
    #[test]
    fn every_row_of_the_table_is_a_rule_with_a_source() {
        for a in TABLE {
            assert!(!a.needs.is_empty(), "{} sans mot à chercher", a.label);
            assert!(!a.label.trim().is_empty());
            assert!(!a.steps.is_empty(), "{} sans palier", a.label);
            assert!(!a.source.trim().is_empty(), "{} sans source", a.label);
            let mut seen: Option<u16> = None;
            for s in a.steps {
                assert!(
                    s.below > 0 && s.below <= 90,
                    "{} : seuil hors bornes",
                    a.label
                );
                if let Some(prev) = seen {
                    assert!(
                        s.below < prev,
                        "{} : les paliers vont du plus haut au plus bas",
                        a.label
                    );
                }
                seen = Some(s.below);
                assert!(
                    !s.conduct.trim().is_empty(),
                    "{} : palier sans conduite",
                    a.label
                );
                assert!(
                    !s.conduct.contains(" mg"),
                    "{} : une conduite ne porte pas de dose en milligrammes",
                    a.label
                );
            }
            // Les mots cherchés sont repliés comme le sera l'ordonnance.
            for n in a.needs {
                assert_eq!(*n, n.trim(), "{} : « {n} » a une espace en trop", a.label);
            }
        }
    }

    /// Deux entrées ne doivent pas se disputer la même ligne : la
    /// première gagne, et si deux se recouvrent c'est un choix qu'il
    /// faut faire dans la table plutôt que de laisser à l'ordre.
    #[test]
    fn no_two_rows_claim_the_same_treatment() {
        for a in TABLE {
            for n in a.needs {
                let claimers: Vec<&str> = TABLE
                    .iter()
                    .filter(|o| {
                        o.needs
                            .iter()
                            .any(|m| crate::fuzzy::sort_key(n).contains(&crate::fuzzy::sort_key(m)))
                    })
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
}
