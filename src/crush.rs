//! « Peut-on écraser ? » — la question qu'on pose tous les jours, et
//! dont la réponse est dans la prose de dizaines de fiches.
//!
//! Une infirmière d'EHPAD appelle avec une pilulier et huit lignes ; un
//! aidant demande devant le comptoir si le comprimé passe dans la
//! compote. La réponse existe : elle est écrite quelque part dans le
//! paragraphe « forme » ou « précautions » d'une fiche, et il faut
//! l'ouvrir, la lire et la traduire, une par une. Ce module en fait une
//! table, et la table en fait une feuille.
//!
//! Cinq règles le tiennent, une par test :
//!
//! * **Le silence n'est pas une permission.** Une molécule que la table
//!   ne connaît pas n'a pas de réponse — elle en a une qui dit « à
//!   vérifier », et jamais l'absence de ligne. C'est la règle la plus
//!   importante du module : une liste où ce qu'on n'a pas écrit se lit
//!   « oui » est une liste dangereuse, et c'est exactement ce que
//!   devient une liste dont on ne montre que les interdits.
//! * **La forme décide, pas la molécule.** La morphine s'écrase ou ne
//!   s'écrase pas selon la boîte : Moscontin jamais, Skenan en ouvrant
//!   la gélule. Une table par DCI répondrait faux une fois sur deux.
//! * **Ouvrir une gélule n'est pas écraser un comprimé.** Trois
//!   réponses et non deux : oui, non, et « oui mais » — la gélule
//!   s'ouvre, les microgranules s'avalent et ne se croquent pas. C'est
//!   le cas le plus fréquent en gériatrie, et le réduire à « non »
//!   ferait changer une ordonnance qui n'avait pas besoin de l'être.
//! * **Un « non » sans solution laisse le problème entier.** Chaque
//!   refus dit par quoi remplacer — une forme buvable, une libération
//!   immédiate, un patch — ou dit explicitement qu'il n'y en a pas et
//!   qu'il faut appeler le prescripteur.
//! * **Certains « non » ne protègent pas le patient mais celui qui
//!   écrase.** Un cytotoxique, un tératogène : la poussière est le
//!   danger, et la raison se dit, sans quoi quelqu'un écrasera « juste
//!   cette fois » dans une cuillère.
//!
//! Statique, pur et testé. Il ne connaît ni la base ni egui.

/// Ce que la table répond.
///
/// L'ordre est celui de la gravité pour l'affichage : ce qu'on ne doit
/// pas faire se lit avant ce qu'on peut faire.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Verdict {
    /// Ne jamais écraser ni ouvrir.
    No,
    /// Sous condition — le plus souvent : ouvrir la gélule, avaler les
    /// microgranules sans les croquer.
    Conditional,
    /// Oui, sans conséquence connue.
    Yes,
    /// **La table ne sait pas.** Ce n'est pas « oui » : c'est la
    /// réponse qu'on donne quand on n'a pas ouvert le RCP, et elle
    /// demande de l'ouvrir.
    Unknown,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::No => "Ne pas écraser",
            Verdict::Conditional => "Sous condition",
            Verdict::Yes => "Peut être écrasé",
            Verdict::Unknown => "À vérifier",
        }
    }
}

/// Une présentation et ce qu'on peut en faire.
pub struct Rule {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes.
    /// **Les plus précis d'abord** : « moscontin » avant « morphine ».
    pub needs: &'static [&'static str],
    pub label: &'static str,
    pub verdict: Verdict,
    /// Pourquoi, en une phrase de comptoir.
    pub why: &'static str,
    /// Par quoi remplacer, quand la réponse est non. Vide pour un oui.
    pub instead: &'static str,
    pub source: &'static str,
}

/// La réponse pour un traitement donné.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Answer {
    pub treatment: String,
    pub label: &'static str,
    pub verdict: Verdict,
    pub why: &'static str,
    pub instead: &'static str,
    pub source: &'static str,
}

/// Ce qu'on peut faire de chaque ligne de cette ordonnance.
///
/// **Toute ligne reçoit une réponse**, y compris « à vérifier » : une
/// feuille qui ne montrerait que les interdits se lirait comme une
/// autorisation pour tout le reste, et c'est ainsi qu'on écrase un
/// comprimé à libération prolongée.
///
/// L'ordre est celui de la gravité, puis celui du nom, pour qu'une
/// feuille imprimée deux fois soit deux fois la même.
pub fn read(treatments: &[crate::revue::Treatment]) -> Vec<Answer> {
    let mut out: Vec<Answer> = treatments
        .iter()
        .map(|t| {
            let hay =
                crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags));
            let hit = TABLE.iter().find(|r| {
                r.needs
                    .iter()
                    .any(|n| hay.contains(&crate::fuzzy::sort_key(n)))
            });
            match hit {
                Some(r) => Answer {
                    treatment: t.name.trim().to_owned(),
                    label: r.label,
                    verdict: r.verdict,
                    why: r.why,
                    instead: r.instead,
                    source: r.source,
                },
                None => Answer {
                    treatment: t.name.trim().to_owned(),
                    label: "",
                    verdict: Verdict::Unknown,
                    why: "La table ne connaît pas cette présentation. Lire le RCP avant d'écraser : une forme à libération prolongée ou gastro-résistante ne se voit pas au nom.",
                    instead: "",
                    source: "",
                },
            }
        })
        .collect();
    out.sort_by(|a, b| {
        a.verdict
            .cmp(&b.verdict)
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

/// Ce que le comptoir peut ou ne peut pas faire d'une présentation.
///
/// **Les entrées les plus précises d'abord** : la table est lue dans
/// l'ordre et la première qui accroche gagne, si bien que « moscontin »
/// doit précéder « morphine ». C'est un test qui le tient, parce que
/// c'est une erreur qu'on ne voit pas en relisant.
///
/// Rien ici n'est déduit : chaque ligne vient du RCP de la présentation
/// ou de la liste nationale des médicaments écrasables.
pub const TABLE: &[Rule] = &[
    // --- Les morphiniques, où la boîte décide de tout ----------------
    Rule {
        needs: &["moscontin"],
        label: "Moscontin",
        verdict: Verdict::No,
        why: "Comprimé à libération prolongée à matrice : écrasé, il délivre en une fois la dose de douze heures. C'est un surdosage morphinique.",
        instead: "Skenan LP, dont la gélule s'ouvre, ou une solution buvable de morphine à libération immédiate — à dose recalculée par le prescripteur.",
        source: "RCP Moscontin ; liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["actiskenan", "oramorph", "sevredol"],
        label: "Morphine à libération immédiate",
        verdict: Verdict::Conditional,
        why: "La gélule d'Actiskenan s'ouvre ; Oramorph est déjà buvable ; le comprimé de Sevredol s'écrase. Ces trois-là sont à libération immédiate, à la différence du Skenan, dont le nom est voisin.",
        instead: "",
        source: "RCP des présentations ; liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["skenan"],
        label: "Skenan LP",
        verdict: Verdict::Conditional,
        why: "La gélule s'ouvre et les microgranules s'avalent tels quels, dans une cuillerée de compote. Ils ne se croquent ni ne s'écrasent : c'est leur enrobage qui fait la libération prolongée.",
        instead: "",
        source: "RCP Skenan ; liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["oxycontin"],
        label: "Oxycontin LP",
        verdict: Verdict::No,
        why: "Libération prolongée : écrasé, il délivre douze heures d'oxycodone en une prise.",
        instead: "Oxynorm en gélule ouvrable ou en solution buvable, à dose recalculée par le prescripteur.",
        source: "RCP Oxycontin",
    },
    Rule {
        needs: &["oxynorm"],
        label: "Oxynorm",
        verdict: Verdict::Conditional,
        why: "La gélule s'ouvre ; la solution buvable évite la question.",
        instead: "",
        source: "RCP Oxynorm",
    },
    // --- Les formes que leur mécanisme interdit d'ouvrir --------------
    Rule {
        needs: &["concerta"],
        label: "Concerta LP",
        verdict: Verdict::No,
        why: "Pompe osmotique : le comprimé libère par un orifice laser et l'enveloppe se retrouve intacte dans les selles, ce qui est normal. Écrasé, il n'a plus de mécanisme du tout.",
        instead: "Une autre forme de méthylphénidate, à voir avec le prescripteur — les formes LP ne sont pas interchangeables dose pour dose.",
        source: "RCP Concerta",
    },
    Rule {
        needs: &["chronadalate", "adalate"],
        label: "Nifédipine LP",
        verdict: Verdict::No,
        why: "Libération prolongée : écrasée, elle donne une chute tensionnelle brutale.",
        instead: "Un autre inhibiteur calcique en forme écrasable, sur avis du prescripteur.",
        source: "RCP Chronadalate",
    },
    Rule {
        needs: &["depakine chrono", "micropakine"],
        label: "Valproate à libération prolongée",
        verdict: Verdict::Conditional,
        why: "Le comprimé de Dépakine Chrono se coupe à la barre de sécabilité mais ne s'écrase pas. Micropakine est un granulé qui se verse dans un aliment froid et ne se croque pas.",
        instead: "Micropakine LP, ou la solution buvable pour les doses qui s'y prêtent.",
        source: "RCP Dépakine Chrono et Micropakine",
    },
    Rule {
        needs: &["tegretol lp", "carbamazepine lp"],
        label: "Carbamazépine LP",
        verdict: Verdict::No,
        why: "Libération prolongée : écrasée, elle fait un pic puis un creux, et cette molécule-là se règle au taux plasmatique.",
        instead: "La suspension buvable, à répartir selon le prescripteur.",
        source: "RCP Tégrétol LP",
    },
    Rule {
        needs: &["sinemet lp", "modopar lp", "levodopa lp"],
        label: "Lévodopa à libération prolongée",
        verdict: Verdict::No,
        why: "Libération prolongée : la fluctuation motrice de la maladie de Parkinson se règle à l'heure près, et écraser la forme LP la fait perdre.",
        instead: "Modopar dispersible ou la forme à libération immédiate, à répartir par le prescripteur.",
        source: "RCP Sinemet LP et Modopar LP",
    },
    // --- Les enrobages gastro-résistants -----------------------------
    Rule {
        needs: &["inexium", "esomeprazole", "omeprazole", "lansoprazole", "pantoprazole", "rabeprazole"],
        label: "Inhibiteurs de la pompe à protons",
        verdict: Verdict::Conditional,
        why: "Les microgranules sont gastro-résistants : la gélule s'ouvre et se verse dans un aliment acide — une compote de pommes —, mais ils ne se croquent pas. Le comprimé gastro-résistant, lui, ne s'écrase pas.",
        instead: "La forme orodispersible quand elle existe, ou la voie injectable à l'hôpital.",
        source: "RCP des IPP ; liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["kardegic"],
        label: "Kardégic",
        verdict: Verdict::Yes,
        why: "Poudre en sachet à dissoudre : la question ne se pose pas.",
        instead: "",
        source: "RCP Kardégic",
    },
    Rule {
        needs: &["aspirine protect", "acide acetylsalicylique gastro"],
        label: "Aspirine gastro-résistante",
        verdict: Verdict::No,
        why: "L'enrobage protège l'estomac ; écrasé, il ne protège plus rien.",
        instead: "Kardégic en sachet, à dose équivalente.",
        source: "RCP des formes gastro-résistantes",
    },
    // --- Ce qui protège celui qui écrase, et non le patient ----------
    Rule {
        needs: &["methotrexate", "novatrex", "imeth"],
        label: "Méthotrexate",
        verdict: Verdict::No,
        why: "Cytotoxique : la poussière expose la personne qui écrase, pas le patient qui avale. C'est une manipulation à éviter au domicile comme en établissement.",
        instead: "La solution buvable ou la forme injectable, prescrites comme telles.",
        source: "RCP méthotrexate ; recommandations de manipulation des cytotoxiques",
    },
    Rule {
        needs: &["finasteride", "dutasteride", "chibro-proscar", "avodart"],
        label: "Finastéride et dutastéride",
        verdict: Verdict::No,
        why: "Tératogènes par contact : une femme enceinte ou en âge de l'être ne doit pas manipuler le comprimé écrasé ni la gélule ouverte. Le risque est pour qui prépare.",
        instead: "Aucune forme écrasable ; en parler au prescripteur si la déglutition ne permet plus la prise.",
        source: "RCP finastéride et dutastéride",
    },
    Rule {
        needs: &["mycophenolate", "cellcept", "myfortic"],
        label: "Mycophénolate",
        verdict: Verdict::No,
        why: "Tératogène et cytotoxique : ni écraser ni ouvrir, et éviter d'inhaler la poudre.",
        instead: "La suspension buvable, prescrite comme telle.",
        source: "RCP mycophénolate",
    },
    // --- Les cas où l'on croit que non, et où c'est oui ---------------
    Rule {
        needs: &["eliquis", "apixaban"],
        label: "Apixaban",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase et se met dans l'eau ou la compote ; à prendre aussitôt. Le RCP le prévoit, sonde gastrique comprise.",
        instead: "",
        source: "RCP apixaban",
    },
    Rule {
        needs: &["xarelto", "rivaroxaban"],
        label: "Rivaroxaban",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase et se prend dans un peu d'eau ou de compote, et toujours AVEC UN ALIMENT : l'absorption des dosages à 15 et 20 mg en dépend.",
        instead: "",
        source: "RCP rivaroxaban",
    },
    Rule {
        needs: &["pradaxa", "dabigatran"],
        label: "Dabigatran",
        verdict: Verdict::No,
        why: "La gélule ne s'ouvre jamais : sans son enveloppe, la biodisponibilité monte de trois quarts et le risque hémorragique avec elle.",
        instead: "Aucune forme ouvrable ; c'est un changement d'anticoagulant à discuter avec le prescripteur.",
        source: "RCP dabigatran",
    },
    Rule {
        needs: &["levothyrox", "levothyroxine", "l-thyroxine", "euthyrox"],
        label: "Lévothyroxine",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase. La solution buvable existe et évite les variations de dose d'un écrasement mal fait — sur une molécule qui se règle à quelques microgrammes.",
        instead: "",
        source: "RCP lévothyroxine",
    },
    Rule {
        needs: &["metformine lp", "glucophage lp"],
        label: "Metformine LP",
        verdict: Verdict::No,
        why: "Libération prolongée : écrasée, elle donne d'un coup ce qu'elle devait rendre en une journée, et les troubles digestifs suivent.",
        instead: "La metformine à libération immédiate, répartie sur les repas.",
        source: "RCP metformine LP",
    },
    Rule {
        needs: &["metformine", "glucophage", "stagid"],
        label: "Metformine",
        verdict: Verdict::Yes,
        why: "La forme à libération immédiate s'écrase. Le goût est amer : un aliment sucré aide.",
        instead: "",
        source: "RCP metformine ; liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["alendronate", "fosamax", "risedronate", "actonel"],
        label: "Bisphosphonates oraux",
        verdict: Verdict::No,
        why: "Très irritants pour l'œsophage : le comprimé se prend entier, à jeun, avec un grand verre d'eau et debout une demi-heure. Écrasé, il fait une ulcération.",
        instead: "La forme injectable, ou un espacement décidé par le prescripteur.",
        source: "RCP des bisphosphonates oraux",
    },
    Rule {
        needs: &["aricept", "donepezil"],
        label: "Donépézil",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase ; la forme orodispersible existe et se pose sur la langue.",
        instead: "",
        source: "RCP donépézil",
    },
    Rule {
        needs: &["orodispersible", "lyoc", "effervescent"],
        label: "Formes orodispersibles et effervescentes",
        verdict: Verdict::Yes,
        why: "Elles sont faites pour cela : sur la langue ou dans un verre d'eau, sans être écrasées.",
        instead: "",
        source: "RCP des présentations",
    },
    Rule {
        needs: &["doliprane", "paracetamol", "dafalgan", "efferalgan"],
        label: "Paracétamol",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase, la forme effervescente se dissout, la gélule s'ouvre. Attention aux formes effervescentes chez l'insuffisant cardiaque : elles apportent du sodium.",
        instead: "",
        source: "RCP paracétamol",
    },
    Rule {
        needs: &["plavix", "clopidogrel"],
        label: "Clopidogrel",
        verdict: Verdict::Yes,
        why: "Le comprimé s'écrase et se prend aussitôt, dans un peu d'eau. Il n'est ni gastro-résistant ni à libération prolongée, malgré ce que sa taille laisse croire.",
        instead: "",
        source: "Liste nationale des médicaments écrasables",
    },
    Rule {
        needs: &["bisoprolol", "ramipril", "perindopril", "coversyl", "amlodipine", "furosemide", "lasilix"],
        label: "Antihypertenseurs courants à libération immédiate",
        verdict: Verdict::Yes,
        why: "Ces comprimés-là s'écrasent. Vérifier tout de même la boîte : la plupart existent aussi en forme à libération prolongée, qui, elle, ne s'écrase pas.",
        instead: "",
        source: "Liste nationale des médicaments écrasables",
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

    /// **Ce que la table écrit est dessiné tel quel.** Le panneau la
    /// peint avec `RichText`, qui n'interprète aucun balisage : une
    /// astérisque écrite pour insister sort à l'écran comme une
    /// astérisque, et « **à partir de 24 SA** » se lit avec ses quatre
    /// étoiles. Trouvé sur une capture, corrigé ici pour de bon.
    #[test]
    fn the_table_writes_no_markup() {
        for r in TABLE {
            for text in [r.why, r.instead, r.label, r.source] {
                assert!(
                    !text.contains("**") && !text.contains("`"),
                    "{} : « {text} » porte du balisage",
                    r.label
                );
            }
        }
    }

    /// **Le silence n'est pas une permission**, et c'est la règle la
    /// plus importante du module.
    ///
    /// Une molécule que la table ne connaît pas reçoit une réponse — « à
    /// vérifier » — et non pas rien. Une feuille qui ne montrerait que
    /// les interdits se lirait comme une autorisation pour tout le
    /// reste, et c'est exactement ainsi qu'on écrase un comprimé à
    /// libération prolongée.
    #[test]
    fn what_the_table_does_not_know_is_never_a_yes() {
        let found = read(&[treat("Zoltruc 40 mg LP")]);
        assert_eq!(found.len(), 1, "toute ligne reçoit une réponse");
        assert_eq!(found[0].verdict, Verdict::Unknown);
        assert_ne!(found[0].verdict, Verdict::Yes);
        assert!(found[0].why.contains("RCP"), "elle dit où chercher");
        // Et « à vérifier » se lit autrement que « oui » : le libellé
        // est ce que la feuille imprime.
        assert_eq!(Verdict::Unknown.label(), "À vérifier");
    }

    /// **La forme décide, pas la molécule.** La morphine s'écrase ou ne
    /// s'écrase pas selon la boîte, et une table par DCI répondrait faux
    /// une fois sur deux.
    #[test]
    fn the_presentation_decides_and_not_the_molecule() {
        let found = read(&[treat("Moscontin 30 mg"), treat("Skenan LP 30 mg")]);
        let by = |n: &str| {
            found
                .iter()
                .find(|a| a.treatment.starts_with(n))
                .unwrap()
                .clone()
        };
        assert_eq!(by("Moscontin").verdict, Verdict::No);
        assert_eq!(by("Skenan").verdict, Verdict::Conditional);
        // Et la metformine, dont la forme LP doit gagner sur la forme
        // simple bien que « metformine » soit contenu dans les deux.
        assert_eq!(read(&[treat("Metformine LP 1000")])[0].verdict, Verdict::No);
        assert_eq!(read(&[treat("Metformine 1000")])[0].verdict, Verdict::Yes);
    }

    /// **Un « non » sans solution laisse le problème entier.** Chaque
    /// refus dit par quoi remplacer, ou dit qu'il n'y a rien et qu'il
    /// faut appeler le prescripteur.
    #[test]
    fn every_refusal_says_what_to_do_instead() {
        for r in TABLE.iter().filter(|r| r.verdict == Verdict::No) {
            assert!(
                !r.instead.trim().is_empty(),
                "{} : un « non » sans solution",
                r.label
            );
        }
        // Et un « oui » ne propose rien : il n'y a rien à proposer.
        for r in TABLE.iter().filter(|r| r.verdict == Verdict::Yes) {
            assert!(
                r.instead.trim().is_empty(),
                "{} : un « oui » n'a pas de remplaçant",
                r.label
            );
        }
    }

    /// **Les entrées les plus précises d'abord.** La table est lue dans
    /// l'ordre et la première qui accroche gagne : « moscontin » doit
    /// précéder « morphine », « metformine lp » précéder
    /// « metformine ». C'est une erreur qu'on ne voit pas en relisant,
    /// donc c'est un test.
    #[test]
    fn a_more_precise_row_never_sits_behind_a_broader_one() {
        for (i, r) in TABLE.iter().enumerate() {
            for n in r.needs {
                let folded = crate::fuzzy::sort_key(n);
                // Une entrée précédente qui accrocherait déjà ce mot
                // mangerait cette ligne-ci.
                for earlier in TABLE.iter().take(i) {
                    for m in earlier.needs {
                        assert!(
                            !folded.contains(&crate::fuzzy::sort_key(m)),
                            "« {n} » ({}) est mangé par « {m} » ({})",
                            r.label,
                            earlier.label
                        );
                    }
                }
            }
        }
    }

    /// La table est une règle : des mots à chercher, un libellé, une
    /// raison en toutes lettres et une source. Une raison vide ferait
    /// une feuille où « ne pas écraser » n'a pas d'explication, et une
    /// consigne sans raison ne se respecte pas longtemps.
    #[test]
    fn every_row_says_why_and_where_it_comes_from() {
        for r in TABLE {
            assert!(!r.needs.is_empty(), "{} sans mot à chercher", r.label);
            assert!(!r.label.trim().is_empty());
            assert!(!r.source.trim().is_empty(), "{} sans source", r.label);
            assert!(
                r.why.trim().len() > 30,
                "{} : la raison tient en trop peu de mots",
                r.label
            );
            assert_ne!(
                r.verdict,
                Verdict::Unknown,
                "{} : hors de la table, la réponse est « à vérifier »",
                r.label
            );
            for n in r.needs {
                assert_eq!(*n, n.trim(), "{} : « {n} » a une espace en trop", r.label);
                assert_eq!(
                    *n,
                    n.to_lowercase(),
                    "{} : « {n} » n'est pas replié",
                    r.label
                );
            }
        }
    }

    /// L'ordre est celui de la gravité, puis du nom : ce qu'on ne doit
    /// pas faire se lit avant ce qu'on peut faire, et une feuille
    /// imprimée deux fois est deux fois la même.
    #[test]
    fn the_sheet_reads_the_refusals_first() {
        let found = read(&[
            treat("Doliprane"),
            treat("Moscontin"),
            treat("Zoltruc"),
            treat("Skenan"),
        ]);
        let verdicts: Vec<Verdict> = found.iter().map(|a| a.verdict).collect();
        assert_eq!(
            verdicts,
            vec![
                Verdict::No,
                Verdict::Conditional,
                Verdict::Yes,
                Verdict::Unknown
            ]
        );
    }
}
