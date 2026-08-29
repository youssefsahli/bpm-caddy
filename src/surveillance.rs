//! Ce qu'un traitement demande de faire vérifier, et à quel rythme.
//!
//! La biologie répond à « ce chiffre, sous ce traitement, qu'est-ce que
//! ça change ». Il manquait la question d'avant, qui est celle que
//! personne ne pose : **quel chiffre n'a pas été demandé depuis trop
//! longtemps**. Un INR qui alerte est un INR qu'on a fait ; le patient
//! sous AVK dont le dernier INR date de neuf mois ne déclenche aucune
//! règle, parce qu'il n'y a rien à lire.
//!
//! Ce module dit, pour l'ordonnance qu'il reçoit, quels analytes elle
//! réclame, tous les combien, et où en est le dossier. Il ne prescrit
//! rien : les rythmes sont ceux des RCP et des recommandations, et
//! l'espacement réel est décidé par le prescripteur — c'est un
//! aide-mémoire de comptoir, et il le dit.
//!
//! Statique, pur et testé, comme la biologie et la revue.

/// Une surveillance : les mots qui désignent le traitement, l'analyte à
/// demander, le rythme, et la raison.
pub struct Watch {
    /// Cherchés dans le nom, la DCI, la classe et les étiquettes.
    pub needs: &'static [&'static str],
    /// Le code d'un analyte de [`crate::biology::CATALOGUE`].
    pub code: &'static str,
    /// Tous les combien, en mois.
    pub every_months: u32,
    /// Pourquoi on le demande, en une phrase de comptoir.
    pub why: &'static str,
}

/// Où en est le dossier pour un analyte.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// Un résultat existe et il est trop ancien. C'est le seul niveau
    /// qui soit un fait : on a la date, elle est dépassée.
    Overdue,
    /// Rien de noté. Sur une base qui démarre, c'est le cas de presque
    /// tout : une absence de donnée n'est pas une absence d'examen, et
    /// elle ne se dit pas du même ton.
    Never,
    /// Le rythme arrive à terme dans le mois.
    Soon,
    Ok,
}

impl Level {
    fn rank(self) -> u8 {
        match self {
            Level::Overdue => 0,
            Level::Never => 1,
            Level::Soon => 2,
            Level::Ok => 3,
        }
    }
}

/// Une ligne du tableau de surveillance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Due {
    pub code: &'static str,
    /// Le libellé de l'analyte, tel que le catalogue l'écrit.
    pub label: &'static str,
    pub level: Level,
    /// La date du dernier résultat noté, ISO, s'il y en a un.
    pub last: Option<String>,
    /// Depuis combien de mois, quand il y a une date.
    pub months: Option<u32>,
    /// Le rythme retenu : le plus serré de ceux que l'ordonnance
    /// réclame.
    pub every_months: u32,
    /// Les traitements qui le demandent, dans l'ordre de l'ordonnance.
    pub drugs: Vec<String>,
    /// La raison, celle de la règle la plus serrée.
    pub why: &'static str,
}

/// Combien de mois séparent deux dates ISO, ou rien si l'une des deux ne
/// se lit pas.
///
/// Par différence d'années et de mois, avec le jour qui décide du mois
/// entamé — assez juste pour un rythme trimestriel, et cela n'a besoin
/// d'aucune bibliothèque de calendrier.
fn months_between(from: &str, to: &str) -> Option<u32> {
    let read = |iso: &str| -> Option<(i32, i32, i32)> {
        let y = iso.get(..4)?.parse().ok()?;
        let m = iso.get(5..7)?.parse().ok()?;
        let d = iso.get(8..10)?.parse().ok()?;
        Some((y, m, d))
    };
    let (y1, m1, d1) = read(from)?;
    let (y2, m2, d2) = read(to)?;
    let mut months = (y2 - y1) * 12 + (m2 - m1);
    if d2 < d1 {
        months -= 1;
    }
    Some(months.max(0) as u32)
}

/// Ce que cette ordonnance réclame, et où en est le dossier.
///
/// Deux traitements qui demandent le même analyte font une ligne, au
/// rythme du plus serré des deux : c'est une prise de sang, pas deux.
pub fn due(
    treatments: &[crate::revue::Treatment],
    readings: &[crate::biology::Reading],
    today: &str,
) -> Vec<Due> {
    // Chaque traitement replié une fois, avec le nom qu'on affichera.
    let folded: Vec<(String, String)> = treatments
        .iter()
        .map(|t| {
            (
                t.name.trim().to_owned(),
                crate::fuzzy::sort_key(&format!("{} {} {} {}", t.name, t.dci, t.class, t.tags)),
            )
        })
        .collect();
    // La date la plus récente notée pour chaque analyte. Lue dans
    // l'ordre reçu et non dans une table de hachage : deux résultats du
    // même jour ne doivent pas départager la ligne au hasard.
    let mut latest: Vec<(&str, &str)> = Vec::new();
    for r in readings {
        if r.code.trim().is_empty() || r.date.trim().is_empty() {
            continue;
        }
        match latest.iter_mut().find(|(c, _)| *c == r.code) {
            Some((_, kept)) if r.date > *kept => *kept = r.date,
            Some(_) => {}
            None => latest.push((r.code, r.date)),
        }
    }

    let mut out: Vec<Due> = Vec::new();
    for watch in WATCHES {
        let asked: Vec<String> = folded
            .iter()
            .filter(|(_, hay)| {
                watch
                    .needs
                    .iter()
                    .any(|n| crate::fuzzy::contains_folded(hay, n))
            })
            .map(|(name, _)| name.clone())
            .collect();
        if asked.is_empty() {
            continue;
        }
        match out.iter_mut().find(|d| d.code == watch.code) {
            Some(d) => {
                for name in asked {
                    if !d.drugs.contains(&name) {
                        d.drugs.push(name);
                    }
                }
                // Le rythme le plus serré gagne, et la raison suit le
                // rythme : c'est celle qui explique pourquoi si souvent.
                if watch.every_months < d.every_months {
                    d.every_months = watch.every_months;
                    d.why = watch.why;
                }
            }
            None => out.push(Due {
                code: watch.code,
                label: crate::biology::find(watch.code)
                    .map(|a| a.label)
                    .unwrap_or(watch.code),
                level: Level::Never,
                last: None,
                months: None,
                every_months: watch.every_months,
                drugs: asked,
                why: watch.why,
            }),
        }
    }

    for d in &mut out {
        let last = latest
            .iter()
            .find(|(c, _)| *c == d.code)
            .map(|(_, day)| *day);
        d.last = last.map(|s| s.to_owned());
        d.months = last.and_then(|day| months_between(day, today));
        d.level = match d.months {
            None => Level::Never,
            Some(m) if m >= d.every_months => Level::Overdue,
            // « Bientôt » un mois avant l'échéance, et seulement à
            // partir du trimestre : au mois, la granularité est trop
            // grosse pour prévenir — un INR fait il y a cinq jours est
            // à jour, pas « bientôt à refaire ».
            Some(m) if d.every_months >= 3 && m + 1 >= d.every_months => Level::Soon,
            Some(_) => Level::Ok,
        };
    }
    // Le plus fort en tête ; à niveau égal, le plus en retard ; puis
    // l'ordre du catalogue, pour que la liste soit stable.
    out.sort_by(|a, b| {
        a.level
            .rank()
            .cmp(&b.level.rank())
            .then_with(|| b.months.unwrap_or(0).cmp(&a.months.unwrap_or(0)))
            .then_with(|| position(a.code).cmp(&position(b.code)))
    });
    out
}

fn position(code: &str) -> usize {
    crate::biology::CATALOGUE
        .iter()
        .position(|a| a.code == code)
        .unwrap_or(usize::MAX)
}

/// Le rythme, écrit comme on le dit.
pub fn rhythm_text(months: u32) -> String {
    match months {
        0 | 1 => "tous les mois".to_owned(),
        3 => "tous les trois mois".to_owned(),
        6 => "tous les six mois".to_owned(),
        12 => "une fois par an".to_owned(),
        24 => "tous les deux ans".to_owned(),
        n if n % 12 == 0 => format!("tous les {} ans", n / 12),
        n => format!("tous les {n} mois"),
    }
}

/// Ce que les traitements courants demandent de surveiller.
///
/// Les rythmes sont ceux des RCP et des recommandations usuelles, et
/// c'est le prescripteur qui décide de l'espacement réel : un patient
/// stable depuis dix ans n'est pas un patient qu'on vient d'instaurer.
/// Ce qu'un comptoir apporte, c'est de voir qu'un chiffre n'a pas été
/// demandé depuis trop longtemps — la règle qui alerte ne peut rien dire
/// d'un examen qui n'a pas été fait.
pub const WATCHES: &[Watch] = &[
    // --- Anticoagulants et antiagrégants ---
    Watch {
        needs: &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        code: "INR",
        every_months: 1,
        why: "Sous AVK l'INR se contrôle au moins une fois par mois une fois la dose stable, et à chaque changement de traitement, de régime ou d'état.",
    },
    Watch {
        needs: &["AOD", "apixaban", "rivaroxaban", "dabigatran", "édoxaban"],
        code: "DFG",
        every_months: 12,
        why: "La dose d'un AOD se décide sur la clairance : au moins une fois par an, tous les six mois au-delà de 75 ans, en cas de poids faible ou de clairance sous 60, et à chaque épisode aigu — fièvre, diarrhée, canicule.",
    },
    Watch {
        needs: &["héparine", "HBPM", "énoxaparine", "tinzaparine", "daltéparine", "fondaparinux"],
        code: "PLQ",
        every_months: 1,
        why: "La thrombopénie induite par l'héparine survient entre le cinquième et le vingt et unième jour : la numération plaquettaire est ce qui la trouve, et elle se surveille pendant tout le traitement.",
    },
    Watch {
        // The unfractionated one only: under a low-molecular-weight
        // heparin the TCA says nothing, and the anti-Xa is the test.
        needs: &["héparine sodique", "héparine non fractionnée", "calciparine"],
        code: "TCA",
        every_months: 1,
        why: "Sous héparine non fractionnée, c'est le TCA qui règle la dose : rapport de 1,5 à 2,5 fois le témoin, contrôlé 4 à 6 h après le début ou tout changement, puis chaque jour. Sous HBPM il ne veut rien dire.",
    },
    Watch {
        needs: &["HBPM", "énoxaparine", "tinzaparine", "daltéparine"],
        code: "DFG",
        every_months: 12,
        why: "Les héparines de bas poids moléculaire s'accumulent quand le rein filtre mal : au-dessous de 30 mL/min les doses curatives sont contre-indiquées.",
    },
    // --- Rein, tension, cœur ---
    Watch {
        needs: &["IEC", "sartan", "ARA2"],
        code: "K",
        every_months: 12,
        why: "Un bloqueur du système rénine-angiotensine monte le potassium : kaliémie et créatinine une à deux semaines après l'instauration ou toute majoration, puis au moins une fois par an.",
    },
    Watch {
        needs: &["IEC", "sartan", "ARA2"],
        code: "DFG",
        every_months: 12,
        why: "La fonction rénale se contrôle après l'instauration puis annuellement : une hausse de la créatinine de plus de 30 % fait rediscuter le traitement.",
    },
    Watch {
        needs: &["spironolactone", "éplérénone", "anti-aldostérone"],
        code: "K",
        every_months: 6,
        why: "L'anti-aldostérone est le médicament qui fait le plus d'hyperkaliémies graves : kaliémie et créatinine à une semaine, à un mois, puis tous les trois à six mois.",
    },
    Watch {
        needs: &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide", "bumétanide"],
        code: "NA",
        every_months: 6,
        why: "Le thiazidique fait l'hyponatrémie du sujet âgé, et elle est silencieuse jusqu'à la chute ou la confusion. Natrémie, kaliémie et créatinine ensemble.",
    },
    Watch {
        needs: &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide", "bumétanide"],
        code: "K",
        every_months: 6,
        why: "Les diurétiques de l'anse et thiazidiques descendent le potassium, et l'hypokaliémie fait le trouble du rythme — d'autant plus sous digoxine ou sous allongeur du QT.",
    },
    Watch {
        needs: &["digoxine", "digitalique"],
        code: "DIGOX",
        every_months: 6,
        why: "Marge thérapeutique étroite : la digoxinémie se prélève au moins six heures après la prise, et se lit avec la kaliémie et la fonction rénale.",
    },
    Watch {
        needs: &["amiodarone"],
        code: "TSH",
        every_months: 6,
        why: "L'amiodarone contient de l'iode et dérègle la thyroïde dans les deux sens, parfois des mois après l'arrêt : TSH avant l'instauration puis tous les six mois, et six mois après l'arrêt.",
    },
    Watch {
        needs: &["amiodarone"],
        code: "ALAT",
        every_months: 6,
        why: "Hépatotoxicité de l'amiodarone : transaminases avant l'instauration puis tous les six mois.",
    },
    Watch {
        needs: &["sacubitril", "valsartan sacubitril", "Entresto"],
        code: "K",
        every_months: 6,
        why: "Même surveillance qu'un bloqueur du système rénine-angiotensine, et le passage depuis un IEC demande trente-six heures d'arrêt.",
    },
    // --- Lipides, diabète ---
    Watch {
        needs: &["statine", "atorvastatine", "rosuvastatine", "simvastatine", "pravastatine"],
        code: "LDL",
        every_months: 12,
        why: "Le LDL est la cible du traitement : bilan lipidique deux à trois mois après l'instauration ou tout changement de dose, puis une fois par an quand la cible est atteinte.",
    },
    Watch {
        needs: &["statine", "fibrate", "fénofibrate", "ézétimibe"],
        code: "ALAT",
        every_months: 12,
        why: "Transaminases avant l'instauration et à trois mois ; ensuite seulement si la dose change ou si un symptôme apparaît. Un dosage systématique de CPK n'a d'intérêt que devant des douleurs musculaires.",
    },
    Watch {
        needs: &["metformine"],
        code: "DFG",
        every_months: 12,
        why: "La metformine se contre-indique sous 30 mL/min et se réduit de moitié entre 30 et 45 : la clairance au moins une fois par an, plus souvent chez le sujet âgé.",
    },
    Watch {
        needs: &["antidiabétique", "metformine", "insuline", "gliclazide", "glimépiride", "sitagliptine", "dapagliflozine", "empagliflozine", "sémaglutide", "dulaglutide"],
        code: "HBA1C",
        every_months: 3,
        why: "L'HbA1c reflète les trois derniers mois : tous les trois mois tant que la cible n'est pas tenue, tous les six mois ensuite. La cible se personnalise — 7 % n'est pas la cible de tout le monde.",
    },
    Watch {
        needs: &["dapagliflozine", "empagliflozine", "canagliflozine", "gliflozine"],
        code: "DFG",
        every_months: 12,
        why: "Une baisse de la filtration dans les premières semaines est attendue et réversible ; c'est ensuite que la clairance se surveille, et le traitement se suspend en cas de déshydratation.",
    },
    // --- Thyroïde ---
    Watch {
        needs: &["lévothyroxine", "lévothyrox", "hormone thyroïdienne", "L-thyroxine"],
        code: "TSH",
        every_months: 12,
        why: "TSH six à huit semaines après toute modification de dose ou tout changement de spécialité, puis une fois par an quand l'équilibre tient.",
    },
    Watch {
        needs: &["carbimazole", "thiamazole", "propylthiouracile", "antithyroïdien"],
        code: "GB",
        every_months: 3,
        why: "Agranulocytose : toute fièvre ou angine sous antithyroïdien impose une numération en urgence et l'arrêt en attendant. La surveillance programmée ne remplace pas cette consigne, elle l'accompagne.",
    },
    Watch {
        needs: &["carbimazole", "thiamazole", "propylthiouracile", "antithyroïdien"],
        code: "TSH",
        every_months: 3,
        why: "L'équilibre se cherche : TSH et T4 libre toutes les quatre à six semaines au début, puis tous les trois mois.",
    },
    // --- Psychiatrie, neurologie ---
    Watch {
        needs: &["lithium"],
        code: "LITH",
        every_months: 3,
        why: "Marge thérapeutique étroite : lithémie douze heures après la dernière prise, tous les trois mois une fois la dose stable, et à chaque changement — déshydratation, AINS, diurétique, IEC.",
    },
    Watch {
        needs: &["lithium"],
        code: "TSH",
        every_months: 12,
        why: "Le lithium fait l'hypothyroïdie et l'hyperparathyroïdie : TSH et calcémie une fois par an.",
    },
    Watch {
        needs: &["lithium"],
        code: "DFG",
        every_months: 12,
        why: "Le lithium s'élimine par le rein et l'abîme à long terme : la clairance une fois par an, et toute baisse fait remonter la lithémie sans changement de dose.",
    },
    Watch {
        needs: &["clozapine"],
        code: "PNN",
        every_months: 1,
        why: "Agranulocytose : numération hebdomadaire pendant dix-huit semaines, puis mensuelle pendant toute la durée du traitement et quatre semaines après l'arrêt. C'est une condition de délivrance.",
    },
    Watch {
        needs: &["antipsychotique", "neuroleptique", "olanzapine", "rispéridone", "quétiapine", "aripiprazole", "clozapine"],
        code: "GLY",
        every_months: 12,
        why: "Syndrome métabolique : glycémie à jeun et bilan lipidique à trois mois de l'instauration puis une fois par an, avec le poids et le tour de taille.",
    },
    Watch {
        needs: &["valproate", "valproïque", "Dépakine"],
        code: "ALAT",
        every_months: 12,
        why: "Hépatotoxicité, surtout dans les six premiers mois : transaminases et numération avant l'instauration, puis à six mois et une fois par an.",
    },
    Watch {
        needs: &["carbamazépine", "oxcarbazépine"],
        code: "NA",
        every_months: 12,
        why: "Hyponatrémie par SIADH, fréquente et silencieuse chez le sujet âgé : natrémie à l'instauration puis une fois par an.",
    },
    Watch {
        needs: &["ISRS", "sertraline", "escitalopram", "citalopram", "fluoxétine", "paroxétine", "venlafaxine", "duloxétine"],
        code: "NA",
        every_months: 12,
        why: "L'hyponatrémie sous antidépresseur sérotoninergique apparaît dans les premières semaines et se voit surtout chez le sujet âgé sous diurétique : natrémie à un mois de l'instauration.",
    },
    // --- Immunosuppresseurs et anti-inflammatoires ---
    Watch {
        needs: &["méthotrexate"],
        code: "ALAT",
        every_months: 3,
        why: "Numération, transaminases et créatinine tous les mois pendant trois mois, puis tous les trois mois. L'acide folique se prend à distance de la prise hebdomadaire, jamais le même jour.",
    },
    Watch {
        needs: &["méthotrexate", "azathioprine", "mycophénolate", "léflunomide"],
        code: "GB",
        every_months: 3,
        why: "Toxicité médullaire : numération formule sanguine tous les trois mois, et sans attendre devant une fièvre, une angine ou un saignement.",
    },
    Watch {
        needs: &["ciclosporine", "tacrolimus", "évérolimus", "sirolimus"],
        code: "DFG",
        every_months: 3,
        why: "Néphrotoxicité de la classe : créatinine, clairance, kaliémie et magnésémie régulièrement, et le taux résiduel décide de la dose.",
    },
    Watch {
        needs: &["anti-TNF", "adalimumab", "étanercept", "infliximab", "tocilizumab", "sécukinumab"],
        code: "ALAT",
        every_months: 6,
        why: "Numération et transaminases tous les trois à six mois. Toute fièvre sous biothérapie fait suspendre l'injection et consulter.",
    },
    Watch {
        needs: &["corticoïde", "prednisone", "prednisolone", "corticothérapie"],
        code: "GLY",
        every_months: 6,
        why: "Une corticothérapie prolongée déséquilibre le diabète et en révèle : glycémie et kaliémie tous les trois à six mois, avec la tension et le poids.",
    },
    Watch {
        needs: &["AINS", "ibuprofène", "diclofénac", "naproxène", "kétoprofène"],
        code: "DFG",
        every_months: 6,
        why: "Un AINS au long cours abîme le rein, d'autant plus avec un IEC et un diurétique : la clairance tous les six mois, et pas d'AINS sous 60 mL/min sans avis.",
    },
    // --- Métabolisme, os, digestif ---
    Watch {
        needs: &["allopurinol", "fébuxostat", "hypo-uricémiant"],
        code: "URIC",
        every_months: 6,
        why: "La cible est une uricémie sous 60 mg/L (360 µmol/L), sous 50 en cas de tophus : c'est elle qui dit si la dose suffit, et un traitement à dose fixe sans dosage ne sert à rien.",
    },
    Watch {
        needs: &["bisphosphonate", "alendronate", "risédronate", "zolédronique", "dénosumab"],
        code: "CA",
        every_months: 12,
        why: "La calcémie et la vitamine D se corrigent **avant** l'injection et jamais après : sous dénosumab l'hypocalcémie peut être sévère, surtout si la clairance est basse.",
    },
    Watch {
        needs: &["bisphosphonate", "dénosumab", "vitamine D", "cholécalciférol", "calcifédiol"],
        code: "VITD",
        every_months: 12,
        why: "Une carence en vitamine D entretient l'hyperparathyroïdie et fait échouer le traitement de l'os : elle se corrige avant, et se recontrôle une fois par an.",
    },
    Watch {
        needs: &["IPP", "oméprazole", "pantoprazole", "ésoméprazole", "lansoprazole"],
        code: "MG",
        every_months: 12,
        why: "Hypomagnésémie des IPP au long cours, après un an ou plus : elle empêche de corriger une hypokaliémie et donne crampes, tremblements et troubles du rythme. Elle justifie surtout de rediscuter l'indication.",
    },
    Watch {
        needs: &["metformine", "IPP", "oméprazole", "pantoprazole"],
        code: "B12",
        every_months: 24,
        why: "Metformine et IPP au long cours font la carence en vitamine B12, qui se voit d'abord sur le VGM et donne une neuropathie que l'on met sur le compte du diabète.",
    },
    Watch {
        needs: &["fer", "sulfate ferreux", "fumarate ferreux", "ascorbate ferreux"],
        code: "FERR",
        every_months: 3,
        why: "Un traitement martial se juge sur la ferritine et l'hémoglobine à trois mois : reconstituer la réserve demande trois à six mois après la normalisation de l'hémoglobine.",
    },
    Watch {
        needs: &["chélateur", "sévélamer", "carbonate de lanthane", "acétate de calcium"],
        code: "PHOS",
        every_months: 3,
        why: "Phosphore, calcium et parathormone se lisent ensemble, jamais séparément — et le chélateur se prend au milieu du repas : à jeun, il ne chélate rien.",
    },
    // --- Divers ---
    Watch {
        needs: &["testostérone", "androgène"],
        code: "HTE",
        every_months: 6,
        why: "La testostérone monte l'hématocrite et avec lui le risque thrombotique : hématocrite et PSA avant l'instauration, à trois et six mois, puis une fois par an.",
    },
    Watch {
        needs: &["érythropoïétine", "époétine", "darbépoétine", "agent stimulant l'érythropoïèse"],
        code: "HB",
        every_months: 3,
        why: "La cible d'hémoglobine ne se dépasse pas : au-delà de 12 g/dL le risque thrombotique augmente sans bénéfice. Hémoglobine et statut martial régulièrement.",
    },
    Watch {
        needs: &["isotrétinoïne", "acitrétine", "rétinoïde"],
        code: "TG",
        every_months: 1,
        why: "Triglycérides et transaminases avant le traitement, à un mois, puis tous les trois mois — et le test de grossesse mensuel, qui est une condition de délivrance.",
    },
    Watch {
        needs: &["colchicine"],
        code: "DFG",
        every_months: 12,
        why: "La colchicine s'accumule quand le rein filtre mal, et sa marge est étroite : la dose se réduit selon la clairance et l'association aux inhibiteurs du CYP3A4 est à vérifier.",
    },
    Watch {
        needs: &["hypolipémiant", "statine", "ézétimibe", "fibrate", "bempédoïque", "évolocumab", "alirocumab"],
        code: "CT",
        every_months: 12,
        why: "Le bilan lipidique complet une fois par an quand la cible est tenue : c'est aussi ce qui dit si le traitement est pris.",
    },
    Watch {
        needs: &["néphroprotection", "diabète", "metformine", "IEC", "sartan"],
        code: "RAC",
        every_months: 12,
        why: "L'albuminurie bouge des années avant le DFG chez le diabétique et l'hypertendu : c'est le marqueur qui permet d'agir tant qu'il reste quelque chose à protéger.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biology::Reading;
    use crate::revue::Treatment;

    fn treat(name: &'static str, dci: &'static str, class: &'static str) -> Treatment<'static> {
        Treatment {
            name,
            dci,
            class,
            tags: "",
        }
    }

    #[test]
    fn months_are_counted_by_the_calendar_and_not_by_the_day() {
        assert_eq!(months_between("2026-01-15", "2026-08-29"), Some(7));
        // Le jour décide du mois entamé.
        assert_eq!(months_between("2026-01-15", "2026-02-14"), Some(0));
        assert_eq!(months_between("2026-01-15", "2026-02-15"), Some(1));
        // Par-dessus le changement d'année.
        assert_eq!(months_between("2025-11-30", "2026-08-29"), Some(8));
        // Une date à venir ne compte pas en négatif.
        assert_eq!(months_between("2027-01-01", "2026-08-29"), Some(0));
        assert_eq!(months_between("", "2026-08-29"), None);
        assert_eq!(months_between("2026-8-1", "2026-08-29"), None);
    }

    #[test]
    fn an_ordonnance_says_what_it_owes_and_the_file_says_since_when() {
        let treatments = [treat("Previscan", "fluindione", "AVK")];
        // Rien de noté : la ligne existe, et elle le dit sans crier.
        let none = due(&treatments, &[], "2026-08-29");
        let inr = none
            .iter()
            .find(|d| d.code == "INR")
            .expect("l'AVK demande un INR");
        assert_eq!(inr.level, Level::Never);
        assert_eq!(inr.every_months, 1);
        assert_eq!(inr.drugs, vec!["Previscan".to_owned()]);
        assert_eq!(inr.label, "INR");

        // Un INR de la semaine dernière : à jour.
        let fresh = [Reading {
            code: "INR",
            value: 2.4,
            date: "2026-08-22",
        }];
        let now = due(&treatments, &fresh, "2026-08-29");
        assert_eq!(now[0].level, Level::Ok);
        assert_eq!(now[0].months, Some(0));
        assert_eq!(now[0].last.as_deref(), Some("2026-08-22"));

        // Un INR de neuf mois : c'est le seul niveau qui soit un fait.
        let old = [Reading {
            code: "INR",
            value: 2.4,
            date: "2025-11-20",
        }];
        let late = due(&treatments, &old, "2026-08-29");
        assert_eq!(late[0].level, Level::Overdue);
        assert_eq!(late[0].months, Some(9));
    }

    #[test]
    fn two_treatments_asking_for_the_same_analyte_make_one_line() {
        // Une prise de sang, pas deux — et au rythme du plus serré.
        let treatments = [
            treat("Coversyl", "périndopril", "IEC"),
            treat("Aldactone", "spironolactone", "anti-aldostérone"),
        ];
        let out = due(&treatments, &[], "2026-08-29");
        let k: Vec<&Due> = out.iter().filter(|d| d.code == "K").collect();
        assert_eq!(k.len(), 1, "la kaliémie ne se demande qu'une fois");
        assert_eq!(k[0].every_months, 6, "le rythme le plus serré gagne");
        assert_eq!(k[0].drugs.len(), 2);
        assert!(k[0].why.contains("anti-aldostérone"));
    }

    #[test]
    fn the_loudest_comes_first_and_the_order_never_moves() {
        let treatments = [
            treat("Previscan", "fluindione", "AVK"),
            treat("Lévothyrox", "lévothyroxine", "hormone thyroïdienne"),
            treat("Coversyl", "périndopril", "IEC"),
        ];
        let readings = [
            // À jour.
            Reading {
                code: "INR",
                value: 2.4,
                date: "2026-08-22",
            },
            // Très en retard.
            Reading {
                code: "TSH",
                value: 2.1,
                date: "2023-01-10",
            },
        ];
        let out = due(&treatments, &readings, "2026-08-29");
        assert_eq!(out[0].code, "TSH");
        assert_eq!(out[0].level, Level::Overdue);
        assert_eq!(out.last().map(|d| d.level), Some(Level::Ok));
        // Deux appels de suite rendent exactement la même chose : le
        // tableau se réaffiche à chaque image.
        for _ in 0..10 {
            assert_eq!(due(&treatments, &readings, "2026-08-29"), out);
        }
    }

    #[test]
    fn nothing_is_owed_by_an_ordonnance_nobody_watches() {
        let out = due(&[treat("Dexeryl", "", "émollient")], &[], "2026-08-29");
        assert!(out.is_empty());
        assert!(due(&[], &[], "2026-08-29").is_empty());
    }

    #[test]
    fn every_watch_names_an_analyte_of_the_catalogue_and_says_why() {
        for w in WATCHES {
            assert!(
                crate::biology::find(w.code).is_some(),
                "surveillance sur un analyte inconnu : {}",
                w.code
            );
            assert!(!w.needs.is_empty(), "surveillance sans traitement");
            // Une phrase, pas une étiquette : le rythme se trouve dans un
            // RCP, la raison est ce qu'on dit au comptoir.
            assert!(
                w.why.len() > 60,
                "surveillance {} : la raison tient en trop peu de mots",
                w.code
            );
            assert!(w.every_months > 0 && w.every_months <= 24);
        }
    }

    /// A watch naming a treatment no card carries can never fire.
    #[test]
    fn every_watch_can_fire_on_the_base_as_shipped() {
        for w in WATCHES {
            let reachable = w.needs.iter().any(|needle| {
                let needle = crate::fuzzy::sort_key(needle);
                crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::sort_key(&format!("{name} {dci} {class}")).contains(&needle)
                    })
            });
            assert!(
                reachable,
                "surveillance {} : aucun médicament de la base de départ ne correspond à {:?}",
                w.code, w.needs
            );
        }
    }

    /// The catalogue is a ratchet: a watch withdrawn is a check nobody
    /// does any more, and it goes without anyone seeing it.
    #[test]
    fn the_watch_count_only_grows() {
        assert!(
            WATCHES.len() >= 47,
            "{} surveillances : le compte ne baisse pas",
            WATCHES.len()
        );
    }

    #[test]
    fn a_rhythm_is_written_the_way_it_is_said() {
        assert_eq!(rhythm_text(1), "tous les mois");
        assert_eq!(rhythm_text(3), "tous les trois mois");
        assert_eq!(rhythm_text(12), "une fois par an");
        assert_eq!(rhythm_text(24), "tous les deux ans");
        assert_eq!(rhythm_text(36), "tous les 3 ans");
        assert_eq!(rhythm_text(4), "tous les 4 mois");
    }
}
