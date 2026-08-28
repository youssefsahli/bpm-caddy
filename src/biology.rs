//! The biology a pharmacist reads at the counter: what is normal, what
//! is not, and what it changes for the treatments the patient is on.
//!
//! Static, pure and tested, like the calendrier vaccinal: the analytes
//! with their usual adult intervals, the thresholds that stop being a
//! deviation and start being an emergency, and the rules that tie a
//! value to a treatment — a kaliémie at 5,8 is a number, a kaliémie at
//! 5,8 under IEC and spironolactone is a phone call.
//!
//! The intervals are the usual adult ones and vary from one laboratory
//! to the next: what the report itself states always wins. Nothing here
//! decides anything — it says what is worth looking at.

/// How far from usual a value is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// Under the critical threshold: not a deviation, an emergency.
    CriticalLow,
    Low,
    Normal,
    High,
    CriticalHigh,
    /// The analyte has no interval to be read against (INR, HbA1c: a
    /// target, not a range), or the value could not be read.
    Unknown,
}

impl Level {
    /// Whether this level is worth saying out loud.
    pub fn notable(self) -> bool {
        !matches!(self, Level::Normal | Level::Unknown)
    }

    pub fn critical(self) -> bool {
        matches!(self, Level::CriticalLow | Level::CriticalHigh)
    }
}

/// One analyte, as it is written on a French laboratory report.
pub struct Analyte {
    /// Short stable code, stored with the reading.
    pub code: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    /// The usual adult interval. `None` on either side when the
    /// analyte is read against a target instead — an INR is not
    /// "normal", it is in its zone or out of it.
    pub low: Option<f64>,
    pub high: Option<f64>,
    /// Beyond these, the value is an emergency rather than a deviation.
    pub critical_low: Option<f64>,
    pub critical_high: Option<f64>,
    /// What the number means at the counter, in one or two sentences.
    pub note: &'static str,
}

/// Where a value falls, given the analyte's intervals.
pub fn level(a: &Analyte, value: f64) -> Level {
    if let Some(c) = a.critical_low {
        if value <= c {
            return Level::CriticalLow;
        }
    }
    if let Some(c) = a.critical_high {
        if value >= c {
            return Level::CriticalHigh;
        }
    }
    match (a.low, a.high) {
        (Some(l), _) if value < l => Level::Low,
        (_, Some(h)) if value > h => Level::High,
        (None, None) => Level::Unknown,
        _ => Level::Normal,
    }
}

/// The interval as it reads on screen ("3,5 – 5,0 mmol/L"), or an empty
/// string for an analyte that has none.
pub fn interval_text(a: &Analyte) -> String {
    let fr = |v: f64| crate::codex::format_quantity(v);
    match (a.low, a.high) {
        (Some(l), Some(h)) => format!("{} – {} {}", fr(l), fr(h), a.unit),
        (Some(l), None) => format!("≥ {} {}", fr(l), a.unit),
        (None, Some(h)) => format!("≤ {} {}", fr(h), a.unit),
        (None, None) => String::new(),
    }
}

/// The analyte behind a stored code.
pub fn find(code: &str) -> Option<&'static Analyte> {
    CATALOGUE.iter().find(|a| a.code == code)
}

/// The analytes matching what is being typed, best first.
pub fn search(query: &str) -> Vec<&'static Analyte> {
    if query.trim().is_empty() {
        return CATALOGUE.iter().collect();
    }
    let mut scored: Vec<(i32, &'static Analyte)> = CATALOGUE
        .iter()
        .filter_map(|a| {
            let by_label = crate::fuzzy::score(query, a.label);
            let by_code = crate::fuzzy::score(query, a.code).map(|s| s + 20);
            by_label.max(by_code).map(|s| (s, a))
        })
        .collect();
    scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
    scored.into_iter().map(|(_, a)| a).collect()
}

/// One reading, reduced to what the reading rules need.
pub struct Reading<'a> {
    pub code: &'a str,
    pub value: f64,
    /// ISO `YYYY-MM-DD`, possibly empty.
    pub date: &'a str,
}

/// How loudly a finding asks to be acted on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Severity {
    /// Worth knowing.
    Info,
    /// Worth a look, and often a call to the prescriber.
    Warn,
    /// Do not dispense without an answer.
    Alert,
}

/// One thing worth saying about a patient's biology.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    /// The analyte it is about.
    pub code: &'static str,
    pub text: String,
}

/// Which side of a threshold triggers a rule.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Above,
    Below,
}

/// A rule tying a value to the treatments it concerns.
struct Rule {
    code: &'static str,
    side: Side,
    threshold: f64,
    /// Words looked for in the patient's treatments — a brand, a DCI, a
    /// class or a tag. Empty means the rule holds whatever the patient
    /// takes.
    needs: &'static [&'static str],
    severity: Severity,
    text: &'static str,
}

/// Read a patient's biology against the treatments the file knows.
///
/// `treatments` is whatever describes them — brand, DCI, class, tags —
/// in any case: the rules match on words, so a card tagged « AINS »
/// answers a rule about AINS without the code needing to know the brand.
///
/// Only the most recent reading of each analyte is read: a kaliémie
/// corrected last month is not an alert today.
pub fn read(readings: &[Reading], treatments: &[String]) -> Vec<Finding> {
    let haystack: Vec<String> = treatments
        .iter()
        .map(|t| crate::fuzzy::sort_key(t))
        .collect();
    let takes = |needle: &str| {
        let needle = crate::fuzzy::sort_key(needle);
        haystack.iter().any(|t| t.contains(&needle))
    };
    let mut latest: std::collections::HashMap<&str, &Reading> = std::collections::HashMap::new();
    for r in readings {
        latest
            .entry(r.code)
            .and_modify(|kept| {
                if r.date >= kept.date {
                    *kept = r;
                }
            })
            .or_insert(r);
    }
    let mut out = Vec::new();
    // First what the value says on its own, then what it says about a
    // treatment: the second is the one that changes a dispensation.
    for (code, r) in &latest {
        let Some(a) = find(code) else {
            continue;
        };
        let lv = level(a, r.value);
        if lv.notable() {
            out.push(Finding {
                severity: if lv.critical() {
                    Severity::Alert
                } else {
                    Severity::Info
                },
                code: a.code,
                text: format!(
                    "{} {} {} — {} ({}). {}",
                    a.label,
                    crate::codex::format_quantity(r.value),
                    a.unit,
                    level_word(lv),
                    interval_text(a),
                    a.note
                ),
            });
        }
    }
    for rule in RULES {
        let Some(r) = latest.get(rule.code) else {
            continue;
        };
        let hit = match rule.side {
            Side::Above => r.value >= rule.threshold,
            Side::Below => r.value <= rule.threshold,
        };
        if !hit {
            continue;
        }
        if !rule.needs.is_empty() && !rule.needs.iter().any(|n| takes(n)) {
            continue;
        }
        out.push(Finding {
            severity: rule.severity,
            code: rule.code,
            text: rule.text.to_owned(),
        });
    }
    // Loudest first; inside one severity, the order of the catalogue.
    out.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| position(a.code).cmp(&position(b.code)))
    });
    out
}

fn position(code: &str) -> usize {
    CATALOGUE
        .iter()
        .position(|a| a.code == code)
        .unwrap_or(usize::MAX)
}

/// The French word for a level, as it is shown beside the value.
pub fn level_word(level: Level) -> &'static str {
    match level {
        Level::CriticalLow => "effondré",
        Level::Low => "bas",
        Level::Normal => "normal",
        Level::High => "élevé",
        Level::CriticalHigh => "très élevé",
        Level::Unknown => "à interpréter",
    }
}

/// The analytes an officine actually reads, with the usual adult
/// intervals. A laboratory's own interval always wins over these.
pub const CATALOGUE: &[Analyte] = &[
    Analyte {
        code: "DFG",
        label: "Débit de filtration glomérulaire",
        unit: "mL/min",
        low: Some(60.0),
        high: None,
        critical_low: Some(15.0),
        critical_high: None,
        note: "C'est le chiffre qui commande l'adaptation des doses : au-dessous de 60 mL/min une bonne part de l'ordonnance se relit, au-dessous de 30 plusieurs molécules sortent.",
    },
    Analyte {
        code: "CREAT",
        label: "Créatininémie",
        unit: "µmol/L",
        low: Some(45.0),
        high: Some(110.0),
        critical_low: None,
        critical_high: Some(200.0),
        note: "Chez la femme l'intervalle usuel est plus bas (45 à 90 µmol/L). Une créatinine normale chez une personne âgée et maigre peut cacher une clairance basse : c'est le DFG qui décide.",
    },
    Analyte {
        code: "K",
        label: "Kaliémie",
        unit: "mmol/L",
        low: Some(3.5),
        high: Some(5.0),
        critical_low: Some(3.0),
        critical_high: Some(6.0),
        note: "Le potassium se surveille par ce qui l'entoure : IEC, sartans, spironolactone et AINS le montent, diurétiques de l'anse et thiazidiques le descendent.",
    },
    Analyte {
        code: "NA",
        label: "Natrémie",
        unit: "mmol/L",
        low: Some(135.0),
        high: Some(145.0),
        critical_low: Some(125.0),
        critical_high: Some(155.0),
        note: "Une hyponatrémie d'apparition récente chez une personne âgée est médicamenteuse jusqu'à preuve du contraire : diurétiques, ISRS, carbamazépine.",
    },
    Analyte {
        code: "INR",
        label: "INR",
        unit: "",
        low: None,
        high: None,
        critical_low: None,
        critical_high: Some(5.0),
        note: "La cible usuelle est 2 à 3, et 2,5 à 3,5 pour certaines valves mécaniques : c'est l'ordonnance qui la fixe, pas un intervalle de laboratoire.",
    },
    Analyte {
        code: "HB",
        label: "Hémoglobine",
        unit: "g/dL",
        low: Some(12.0),
        high: Some(17.0),
        critical_low: Some(8.0),
        critical_high: None,
        note: "Anémie au-dessous de 13 g/dL chez l'homme et de 12 chez la femme. Sous anticoagulant, une hémoglobine qui baisse est un saignement jusqu'à preuve du contraire.",
    },
    Analyte {
        code: "PLQ",
        label: "Plaquettes",
        unit: "G/L",
        low: Some(150.0),
        high: Some(400.0),
        critical_low: Some(50.0),
        critical_high: None,
        note: "Sous héparine, une chute des plaquettes entre le 5e et le 21e jour fait suspecter une thrombopénie induite par l'héparine : c'est une urgence, et l'héparine s'arrête.",
    },
    Analyte {
        code: "PNN",
        label: "Polynucléaires neutrophiles",
        unit: "G/L",
        low: Some(1.5),
        high: Some(7.0),
        critical_low: Some(0.5),
        critical_high: None,
        note: "Au-dessous de 0,5 G/L c'est une agranulocytose : toute fièvre devient une urgence, et les molécules qui l'induisent s'arrêtent immédiatement.",
    },
    Analyte {
        code: "ALAT",
        label: "ALAT (transaminases)",
        unit: "UI/L",
        low: None,
        high: Some(40.0),
        critical_low: None,
        critical_high: Some(120.0),
        note: "Trois fois la limite supérieure est le seuil qui fait discuter l'arrêt d'une statine, d'un antituberculeux ou du méthotrexate.",
    },
    Analyte {
        code: "GGT",
        label: "Gamma-GT",
        unit: "UI/L",
        low: None,
        high: Some(55.0),
        critical_low: None,
        critical_high: None,
        note: "Sensible mais peu spécifique : alcool, surpoids, inducteurs enzymatiques. Isolée, elle ne fait pas arrêter un traitement.",
    },
    Analyte {
        code: "CPK",
        label: "Créatine-phosphokinase",
        unit: "UI/L",
        low: None,
        high: Some(200.0),
        critical_low: None,
        critical_high: Some(1000.0),
        note: "Sous statine, une douleur musculaire diffuse avec CPK au-delà de cinq fois la normale fait arrêter ; au-delà de dix fois, c'est une rhabdomyolyse.",
    },
    Analyte {
        code: "TSH",
        label: "TSH",
        unit: "mUI/L",
        low: Some(0.4),
        high: Some(4.0),
        critical_low: None,
        critical_high: Some(10.0),
        note: "Sous lévothyroxine, la TSH se contrôle six à huit semaines après tout changement de dose ou de marque, jamais avant : elle réagit lentement.",
    },
    Analyte {
        code: "HBA1C",
        label: "Hémoglobine glyquée",
        unit: "%",
        low: None,
        high: Some(7.0),
        critical_low: None,
        critical_high: Some(10.0),
        note: "La cible est individuelle : 7 % pour la plupart, 8 % ou plus chez le sujet âgé fragile où l'hypoglycémie coûte plus cher que l'hyperglycémie.",
    },
    Analyte {
        code: "GLY",
        label: "Glycémie à jeun",
        unit: "g/L",
        low: Some(0.7),
        high: Some(1.1),
        critical_low: Some(0.5),
        critical_high: Some(3.0),
        note: "Le diabète se définit à partir de 1,26 g/L à jeun, vérifié deux fois. Au-dessous de 0,7 g/L, chercher un sulfamide ou une insuline.",
    },
    Analyte {
        code: "LDL",
        label: "LDL-cholestérol",
        unit: "g/L",
        low: None,
        high: None,
        critical_low: None,
        critical_high: None,
        note: "Une cible, pas une norme : moins de 0,55 g/L à très haut risque cardiovasculaire, 0,70 à haut risque, 1,00 à risque modéré.",
    },
    Analyte {
        code: "TG",
        label: "Triglycérides",
        unit: "g/L",
        low: None,
        high: Some(1.5),
        critical_low: None,
        critical_high: Some(5.0),
        note: "Au-delà de 5 g/L le risque devient celui de la pancréatite aiguë, et l'alcool comme les sucres rapides pèsent autant que le traitement.",
    },
    Analyte {
        code: "URIC",
        label: "Uricémie",
        unit: "µmol/L",
        low: Some(180.0),
        high: Some(420.0),
        critical_low: None,
        critical_high: None,
        note: "Sous traitement hypo-uricémiant, la cible est inférieure à 360 µmol/L, et inférieure à 300 en présence de tophus.",
    },
    Analyte {
        code: "CRP",
        label: "Protéine C-réactive",
        unit: "mg/L",
        low: None,
        high: Some(5.0),
        critical_low: None,
        critical_high: Some(100.0),
        note: "Marqueur d'inflammation, pas de diagnostic. Elle monte en quelques heures et redescend en quelques jours.",
    },
    Analyte {
        code: "CA",
        label: "Calcémie",
        unit: "mmol/L",
        low: Some(2.2),
        high: Some(2.6),
        critical_low: Some(1.8),
        critical_high: Some(3.0),
        note: "À interpréter avec l'albumine : une hypoalbuminémie abaisse la calcémie totale sans toucher au calcium ionisé.",
    },
    Analyte {
        code: "MG",
        label: "Magnésémie",
        unit: "mmol/L",
        low: Some(0.7),
        high: Some(1.0),
        critical_low: Some(0.5),
        critical_high: None,
        note: "Une hypokaliémie qui ne se corrige pas est souvent une hypomagnésémie non traitée. Les IPP au long cours et les diurétiques l'entretiennent.",
    },
    Analyte {
        code: "FERR",
        label: "Ferritine",
        unit: "µg/L",
        low: Some(15.0),
        high: Some(300.0),
        critical_low: None,
        critical_high: None,
        note: "Une ferritine basse signe une carence martiale ; normale ou haute, elle ne l'exclut pas en cas d'inflammation — regarder la CRP à côté.",
    },
    Analyte {
        code: "VITD",
        label: "Vitamine D (25-OH)",
        unit: "ng/mL",
        low: Some(30.0),
        high: Some(100.0),
        critical_low: None,
        critical_high: Some(150.0),
        note: "Carence au-dessous de 20 ng/mL, insuffisance entre 20 et 30. Au-delà de 150, le risque d'hypercalcémie devient réel.",
    },
    Analyte {
        code: "ALB",
        label: "Albuminémie",
        unit: "g/L",
        low: Some(35.0),
        high: Some(50.0),
        critical_low: Some(25.0),
        critical_high: None,
        note: "Elle change la fraction libre des médicaments fortement liés — AVK, phénytoïne — sans que le dosage total ne bouge.",
    },
    Analyte {
        code: "DIGOX",
        label: "Digoxinémie",
        unit: "ng/mL",
        low: Some(0.5),
        high: Some(0.9),
        critical_low: None,
        critical_high: Some(2.0),
        note: "Prélèvement au moins six heures après la prise, sinon le chiffre ne veut rien dire. La marge est étroite et l'hypokaliémie majore la toxicité à concentration constante.",
    },
    Analyte {
        code: "LITH",
        label: "Lithémie",
        unit: "mmol/L",
        low: Some(0.6),
        high: Some(0.8),
        critical_low: None,
        critical_high: Some(1.2),
        note: "Prélèvement douze heures après la dernière prise. Déshydratation, régime sans sel, AINS, IEC et diurétiques font monter la lithémie sans changer la dose.",
    },
    Analyte {
        code: "ASAT",
        label: "ASAT (transaminases)",
        unit: "UI/L",
        low: None,
        high: Some(40.0),
        critical_low: None,
        critical_high: Some(120.0),
        note: "Moins spécifique du foie que l'ALAT : elle monte aussi avec le muscle. ASAT franchement supérieures aux ALAT chez un buveur, l'inverse dans les hépatites médicamenteuses.",
    },
    Analyte {
        code: "PAL",
        label: "Phosphatases alcalines",
        unit: "UI/L",
        low: Some(30.0),
        high: Some(120.0),
        critical_low: None,
        critical_high: None,
        note: "Élevées avec la bilirubine et les GGT, elles orientent vers la cholestase : c'est le profil de l'amoxicilline-acide clavulanique et de plusieurs antifongiques.",
    },
    Analyte {
        code: "BILI",
        label: "Bilirubine totale",
        unit: "µmol/L",
        low: None,
        high: Some(17.0),
        critical_low: None,
        critical_high: Some(50.0),
        note: "Un ictère apparaît vers 50 µmol/L. Isolée et modérée, elle est souvent constitutionnelle (Gilbert) ; accompagnée des PAL, elle se surveille.",
    },
    Analyte {
        code: "VGM",
        label: "Volume globulaire moyen",
        unit: "fL",
        low: Some(80.0),
        high: Some(100.0),
        critical_low: None,
        critical_high: None,
        note: "Bas, il oriente vers la carence martiale ; haut, vers l'alcool, la carence en B12 ou en folates, et vers la metformine au long cours.",
    },
    Analyte {
        code: "B12",
        label: "Vitamine B12",
        unit: "pmol/L",
        low: Some(150.0),
        high: Some(600.0),
        critical_low: None,
        critical_high: None,
        note: "La metformine et les IPP au long cours l'abaissent. Une carence installée donne des fourmillements avant de donner une anémie.",
    },
    Analyte {
        code: "B9",
        label: "Folates sériques",
        unit: "nmol/L",
        low: Some(7.0),
        high: None,
        critical_low: None,
        critical_high: None,
        note: "Basse sous méthotrexate, sous antiépileptique inducteur et chez le buveur. Une supplémentation avant conception se compte en semaines, pas en jours.",
    },
    Analyte {
        code: "LIP",
        label: "Lipasémie",
        unit: "UI/L",
        low: None,
        high: Some(60.0),
        critical_low: None,
        critical_high: Some(180.0),
        note: "Au-delà de trois fois la normale avec une douleur abdominale, c'est une pancréatite aiguë. Les analogues du GLP-1 et les gliptines en sont une cause rapportée.",
    },
    Analyte {
        code: "PHOS",
        label: "Phosphorémie",
        unit: "mmol/L",
        low: Some(0.8),
        high: Some(1.45),
        critical_low: None,
        critical_high: Some(2.0),
        note: "Elle monte quand le rein s'altère, et c'est elle qui fait prescrire un chélateur, à prendre au moment du repas pour servir à quelque chose.",
    },
    Analyte {
        code: "RAC",
        label: "Rapport albuminurie/créatininurie",
        unit: "mg/mmol",
        low: None,
        high: Some(3.0),
        critical_low: None,
        critical_high: Some(30.0),
        note: "Le marqueur d'atteinte rénale qui bouge le plus tôt chez le diabétique et l'hypertendu, bien avant le DFG.",
    },
    Analyte {
        code: "HDL",
        label: "HDL-cholestérol",
        unit: "g/L",
        low: Some(0.40),
        high: None,
        critical_low: None,
        critical_high: None,
        note: "Le « bon » cholestérol, et le seul dont on souhaite qu'il monte. Aucun traitement ne le cible utilement : c'est le LDL qui se traite, et le HDL bas se corrige par l'activité physique, l'arrêt du tabac et la perte de poids.",
    },
    Analyte {
        code: "HCO3",
        label: "Bicarbonates (réserve alcaline)",
        unit: "mmol/L",
        low: Some(22.0),
        high: Some(29.0),
        critical_low: Some(15.0),
        critical_high: None,
        note: "L'acidose se lit ici avant de se voir. Une réserve alcaline qui s'effondre chez un patient sous metformine, avec des vomissements ou une diarrhée, est l'alerte de l'acidose lactique.",
    },
    Analyte {
        code: "BNP",
        label: "NT-proBNP",
        unit: "pg/mL",
        low: None,
        high: Some(125.0),
        critical_low: None,
        critical_high: Some(2000.0),
        note: "Marqueur de la surcharge du cœur. Normal, il écarte l'insuffisance cardiaque ; élevé, il la suit. Il monte aussi avec l'âge et l'insuffisance rénale, et baisse chez l'obèse.",
    },
    Analyte {
        code: "PSA",
        label: "PSA (antigène prostatique)",
        unit: "ng/mL",
        low: None,
        high: Some(4.0),
        critical_low: None,
        critical_high: None,
        note: "Le seuil dépend de l'âge, et c'est l'évolution d'un dosage à l'autre qui compte plus que le chiffre. Un toucher rectal, un vélo ou une infection urinaire récents le font monter.",
    },
    Analyte {
        code: "CST",
        label: "Coefficient de saturation de la transferrine",
        unit: "%",
        low: Some(20.0),
        high: Some(40.0),
        critical_low: None,
        critical_high: Some(60.0),
        note: "Il dit si le fer circulant est disponible, là où la ferritine dit la réserve. Bas avec une ferritine basse : carence vraie. Bas avec une ferritine haute : inflammation, et le fer oral n'y fera rien.",
    },
    Analyte {
        code: "T4L",
        label: "T4 libre",
        unit: "pmol/L",
        low: Some(9.0),
        high: Some(19.0),
        critical_low: None,
        critical_high: None,
        note: "Elle tranche là où la TSH hésite : dans les premières semaines d'un traitement thyroïdien, la TSH retarde de six semaines et la T4 libre répond tout de suite.",
    },
    Analyte {
        code: "PNE",
        label: "Polynucléaires éosinophiles",
        unit: "G/L",
        low: None,
        high: Some(0.5),
        critical_low: None,
        critical_high: Some(1.5),
        note: "Au-dessus de 0,5 G/L : allergie, parasitose, ou médicament. Une éosinophilie qui apparaît deux à six semaines après un nouveau traitement, avec une éruption et de la fièvre, fait chercher un DRESS.",
    },
    Analyte {
        code: "LYMPHO",
        label: "Lymphocytes",
        unit: "G/L",
        low: Some(1.0),
        high: Some(4.0),
        critical_low: Some(0.5),
        critical_high: None,
        note: "La lymphopénie est ce que surveillent les traitements de fond de la sclérose en plaques et plusieurs immunosuppresseurs : sous 0,5 G/L le risque infectieux, dont la LEMP, n'est plus théorique.",
    },
    Analyte {
        code: "ANTIXA",
        label: "Activité anti-Xa",
        unit: "UI/mL",
        low: None,
        high: None,
        critical_low: None,
        critical_high: Some(1.5),
        note: "La seule mesure de l'effet d'une héparine de bas poids moléculaire. Elle ne se fait pas en routine : elle sert quand le rein est mauvais, le poids extrême, ou qu'un saignement pose la question de l'accumulation. Le prélèvement se fait quatre heures après l'injection, et le résultat ne veut rien dire à un autre moment.",
    },
    Analyte {
        code: "UREE",
        label: "Urée",
        unit: "mmol/L",
        low: Some(2.5),
        high: Some(7.5),
        critical_low: None,
        critical_high: Some(20.0),
        note: "Elle monte avant la créatinine quand le rein manque d'eau : une urée haute avec une créatinine encore normale est le premier signe d'une déshydratation, et c'est là qu'on agit. Elle monte aussi sous corticoïde, après un saignement digestif et avec un régime très riche en protéines, sans que le rein soit en cause.",
    },
    Analyte {
        code: "TP",
        label: "Taux de prothrombine",
        unit: "%",
        low: Some(70.0),
        high: Some(100.0),
        critical_low: Some(20.0),
        critical_high: None,
        note: "C'est l'autre face de l'INR, et c'est celle que les laboratoires français impriment : le TP baisse quand l'INR monte. Sous AVK on lit l'INR et rien d'autre ; hors AVK, un TP bas explore le foie et la vitamine K.",
    },
    Analyte {
        code: "CT",
        label: "Cholestérol total",
        unit: "g/L",
        low: None,
        high: Some(2.0),
        critical_low: None,
        critical_high: None,
        note: "Le chiffre que le patient retient et le moins utile des quatre : c'est le LDL qui porte la cible et le risque, et un cholestérol total « normal » avec un HDL effondré n'est pas rassurant. Sert surtout au calcul du LDL et au rapport CT/HDL.",
    },
    Analyte {
        code: "GB",
        label: "Leucocytes",
        unit: "G/L",
        low: Some(4.0),
        high: Some(10.0),
        critical_low: Some(2.0),
        critical_high: Some(30.0),
        note: "Le total ne dit rien tout seul : c'est la formule qui parle, et surtout les polynucléaires neutrophiles. Une hyperleucocytose sous corticoïde est un effet du corticoïde et non une infection ; une leucopénie fébrile est une urgence.",
    },
    Analyte {
        code: "HTE",
        label: "Hématocrite",
        unit: "%",
        low: Some(37.0),
        high: Some(50.0),
        critical_low: Some(25.0),
        critical_high: Some(60.0),
        note: "Il suit l'hémoglobine et se lit avec elle. Un hématocrite haut est le plus souvent une hémoconcentration — déshydratation, diurétique — avant d'être une polyglobulie, mais la testostérone et le tabac en fabriquent de vraies.",
    },
    Analyte {
        code: "PTH",
        label: "Parathormone",
        unit: "ng/L",
        low: Some(15.0),
        high: Some(65.0),
        critical_low: None,
        critical_high: None,
        note: "Elle se lit toujours avec la calcémie et la vitamine D, jamais seule : haute avec une calcémie basse ou normale, c'est une carence en vitamine D ou une insuffisance rénale qui la stimule ; haute avec une calcémie haute, c'est la glande elle-même.",
    },
];

/// What a value changes for the treatments the patient is actually on.
///
/// The `needs` words are matched inside the treatments' names, DCI,
/// classes and tags, accent- and case-insensitively.
const RULES: &[Rule] = &[
    Rule {
        code: "K",
        side: Side::Above,
        threshold: 5.0,
        needs: &["IEC", "sartan", "spironolactone", "éplérénone", "ARA2"],
        severity: Severity::Alert,
        text: "Kaliémie élevée sous bloqueur du système rénine-angiotensine ou anti-aldostérone : ne pas renouveler sans avis, et se méfier des sels de régime enrichis en potassium et des AINS ajoutés.",
    },
    Rule {
        code: "K",
        side: Side::Below,
        threshold: 3.5,
        needs: &["digoxine", "digitalique"],
        severity: Severity::Alert,
        text: "Hypokaliémie sous digoxine : la toxicité digitalique apparaît à digoxinémie inchangée. Corriger le potassium avant de discuter la dose.",
    },
    Rule {
        code: "K",
        side: Side::Below,
        threshold: 3.5,
        needs: &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
        severity: Severity::Warn,
        text: "Hypokaliémie sous diurétique : contrôler aussi la magnésémie, une hypomagnésémie empêche la correction du potassium.",
    },
    Rule {
        code: "DFG",
        side: Side::Below,
        threshold: 30.0,
        needs: &["AOD", "apixaban", "rivaroxaban", "édoxaban", "dabigatran"],
        severity: Severity::Alert,
        text: "DFG inférieur à 30 mL/min sous anticoagulant oral direct : la dose se réduit ou la molécule se change selon laquelle — le dabigatran est contre-indiqué au-dessous de 30.",
    },
    Rule {
        code: "DFG",
        side: Side::Below,
        threshold: 30.0,
        needs: &["metformine", "biguanide"],
        severity: Severity::Alert,
        text: "DFG inférieur à 30 mL/min sous metformine : contre-indication, en raison du risque d'acidose lactique. Entre 30 et 45, la dose est réduite de moitié.",
    },
    Rule {
        code: "DFG",
        side: Side::Below,
        threshold: 60.0,
        needs: &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène"],
        severity: Severity::Warn,
        text: "AINS et DFG inférieur à 60 mL/min : association à éviter, surtout avec un IEC ou un sartan et un diurétique — c'est la triade qui fait l'insuffisance rénale aiguë.",
    },
    Rule {
        code: "DFG",
        side: Side::Below,
        threshold: 30.0,
        needs: &[],
        severity: Severity::Warn,
        text: "DFG inférieur à 30 mL/min : relire toute l'ordonnance, doses comprises. Beaucoup de molécules courantes s'adaptent ou s'arrêtent à ce niveau.",
    },
    Rule {
        code: "INR",
        side: Side::Above,
        threshold: 5.0,
        needs: &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        severity: Severity::Alert,
        text: "INR supérieur à 5 sous AVK : conduite à tenir immédiate selon le chiffre et le saignement, avis du prescripteur le jour même. Chercher l'interaction récente — antibiotique, antifongique, amiodarone.",
    },
    Rule {
        code: "INR",
        side: Side::Below,
        threshold: 1.5,
        needs: &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        severity: Severity::Warn,
        text: "INR inférieur à 1,5 sous AVK : sous-dosage, le risque est thrombotique. Vérifier l'observance, un changement de régime alimentaire ou un inducteur récent.",
    },
    Rule {
        code: "CPK",
        side: Side::Above,
        threshold: 1000.0,
        needs: &["statine", "fibrate", "atorvastatine", "simvastatine", "rosuvastatine"],
        severity: Severity::Alert,
        text: "CPK au-delà de cinq fois la normale sous statine ou fibrate : arrêt et avis, d'autant plus si les urines sont foncées ou la douleur musculaire diffuse.",
    },
    Rule {
        code: "ALAT",
        side: Side::Above,
        threshold: 120.0,
        needs: &["statine", "méthotrexate", "isoniazide", "amiodarone"],
        severity: Severity::Alert,
        text: "Transaminases au-delà de trois fois la normale sous un traitement hépatotoxique : arrêt à discuter avec le prescripteur, contrôle rapproché.",
    },
    Rule {
        code: "PLQ",
        side: Side::Below,
        threshold: 100.0,
        needs: &["héparine", "énoxaparine", "tinzaparine", "fondaparinux"],
        severity: Severity::Alert,
        text: "Thrombopénie sous héparine : entre le 5e et le 21e jour, suspicion de thrombopénie induite par l'héparine — arrêt immédiat et avis, sans attendre la confirmation biologique.",
    },
    Rule {
        code: "PNN",
        side: Side::Below,
        threshold: 1.5,
        needs: &["clozapine", "carbimazole", "méthotrexate", "immunosuppresseur", "anticancéreux"],
        severity: Severity::Alert,
        text: "Neutropénie sous une molécule qui en induit : toute fièvre impose une consultation en urgence, et la délivrance ne se fait pas sans le contrôle hématologique prévu.",
    },
    Rule {
        code: "NA",
        side: Side::Below,
        threshold: 130.0,
        needs: &["ISRS", "sertraline", "paroxétine", "citalopram", "diurétique", "carbamazépine"],
        severity: Severity::Warn,
        text: "Hyponatrémie sous ISRS, diurétique ou carbamazépine : cause médicamenteuse fréquente chez la personne âgée, à signaler au prescripteur.",
    },
    Rule {
        code: "TSH",
        side: Side::Above,
        threshold: 4.0,
        needs: &["lévothyroxine", "Levothyrox", "L-Thyroxine"],
        severity: Severity::Warn,
        text: "TSH élevée sous lévothyroxine : sous-dosage, mauvaise observance, ou prise trop rapprochée du calcium, du fer, d'un IPP ou du café — le comprimé se prend à jeun, à distance.",
    },
    Rule {
        code: "TSH",
        side: Side::Below,
        threshold: 0.4,
        needs: &["lévothyroxine", "Levothyrox", "L-Thyroxine"],
        severity: Severity::Warn,
        text: "TSH basse sous lévothyroxine : surdosage, avec un risque de fibrillation atriale et de perte osseuse chez la personne âgée.",
    },
    Rule {
        code: "DIGOX",
        side: Side::Above,
        threshold: 2.0,
        needs: &[],
        severity: Severity::Alert,
        text: "Digoxinémie en zone toxique : anorexie, nausées et troubles visuels sont les premiers signes. Vérifier l'heure du prélèvement, la fonction rénale et la kaliémie.",
    },
    Rule {
        code: "LITH",
        side: Side::Above,
        threshold: 1.2,
        needs: &[],
        severity: Severity::Alert,
        text: "Lithémie au-dessus de la zone thérapeutique : tremblement, diarrhée et somnolence signent le surdosage. Chercher une déshydratation, un AINS, un IEC ou un diurétique récemment ajouté.",
    },
    Rule {
        code: "HBA1C",
        side: Side::Below,
        threshold: 6.5,
        needs: &["sulfamide", "glicazide", "glimépiride", "insuline"],
        severity: Severity::Warn,
        text: "HbA1c basse sous sulfamide ou insuline chez un patient âgé : le sur-traitement expose à l'hypoglycémie, qui coûte plus cher ici que quelques dixièmes d'HbA1c.",
    },
    Rule {
        code: "URIC",
        side: Side::Above,
        threshold: 360.0,
        needs: &["allopurinol", "fébuxostat"],
        severity: Severity::Info,
        text: "Uricémie au-dessus de la cible sous hypo-uricémiant : la dose se titre jusqu'à passer sous 360 µmol/L, et sous 300 en présence de tophus.",
    },
    Rule {
        code: "VGM",
        side: Side::Above,
        threshold: 100.0,
        needs: &["metformine", "biguanide"],
        severity: Severity::Warn,
        text: "Macrocytose sous metformine : penser à la carence en vitamine B12, que la metformine induit au long cours. Un dosage de B12 tranche, et la supplémentation ne fait pas arrêter la metformine.",
    },
    Rule {
        code: "B12",
        side: Side::Below,
        threshold: 150.0,
        needs: &["metformine", "biguanide", "IPP", "oméprazole", "pantoprazole", "ésoméprazole"],
        severity: Severity::Warn,
        text: "Vitamine B12 basse sous metformine ou sous IPP au long cours : les deux en sont des causes reconnues. Supplémenter, et réévaluer l'indication de l'IPP.",
    },
    Rule {
        code: "PAL",
        side: Side::Above,
        threshold: 240.0,
        needs: &["amoxicilline", "clavulanique", "antifongique", "statine", "macrolide"],
        severity: Severity::Alert,
        text: "Cholestase sous un médicament qui en donne : l'amoxicilline-acide clavulanique est la première cause médicamenteuse en ville. Arrêt à discuter le jour même, et contrôle rapproché.",
    },
    Rule {
        code: "LIP",
        side: Side::Above,
        threshold: 180.0,
        needs: &["GLP-1", "gliptine", "DPP-4", "sémaglutide", "liraglutide", "dulaglutide"],
        severity: Severity::Alert,
        text: "Lipase au-delà de trois fois la normale sous incrétinomimétique : suspicion de pancréatite. Arrêt immédiat et avis, d'autant plus si la douleur abdominale irradie dans le dos.",
    },
    Rule {
        code: "PHOS",
        side: Side::Above,
        threshold: 1.45,
        needs: &["chélateur du phosphore", "sevelamer", "carbonate de calcium"],
        severity: Severity::Info,
        text: "Phosphorémie encore haute sous chélateur : le comprimé se prend au milieu du repas, pas avant ni après — pris à distance, il ne chélate rien.",
    },
    Rule {
        code: "B9",
        side: Side::Below,
        threshold: 7.0,
        needs: &["méthotrexate", "antiépileptique", "phénytoïne", "carbamazépine"],
        severity: Severity::Warn,
        text: "Folates bas sous méthotrexate ou sous antiépileptique inducteur : la supplémentation est la règle sous méthotrexate, et elle se donne à distance de la prise hebdomadaire.",
    },
    Rule {
        code: "HB",
        side: Side::Below,
        threshold: 10.0,
        needs: &["AOD", "AVK", "héparine", "antiagrégant", "aspirine", "clopidogrel"],
        severity: Severity::Alert,
        text: "Anémie sous antithrombotique : chercher un saignement, digestif en premier lieu, avant de conclure à une carence.",
    },
    Rule {
        code: "MG",
        side: Side::Below,
        threshold: 0.7,
        needs: &["IPP", "oméprazole", "ésoméprazole", "pantoprazole"],
        severity: Severity::Warn,
        text: "Hypomagnésémie sous IPP : effet de classe des traitements prolongés, souvent découvert sur des crampes, une fatigue ou une hypokaliémie qui ne se corrige pas. Doser la magnésémie, supplémenter, et surtout réévaluer l'indication de l'IPP — la magnésémie ne remonte durablement qu'à son arrêt.",
    },
    Rule {
        code: "MG",
        side: Side::Below,
        threshold: 0.7,
        needs: &["digoxine", "amiodarone", "sotalol", "citalopram", "hydroxyzine"],
        severity: Severity::Alert,
        text: "Hypomagnésémie sous une molécule qui allonge le QT ou sous digoxine : c'est le terrain de la torsade de pointes. Corriger le magnésium et le potassium ensemble, avant toute discussion de dose.",
    },
    Rule {
        code: "CREAT",
        side: Side::Above,
        threshold: 110.0,
        needs: &["diurétique", "furosémide", "IEC", "sartan", "AINS"],
        severity: Severity::Warn,
        text: "Créatinine élevée sous diurétique, IEC, sartan ou AINS : demander le DFG, qui est le chiffre qui commande. Chercher ce qui a déshydraté — canicule, gastro-entérite, diurétique majoré : c'est la triade qui fait l'insuffisance rénale aiguë, et elle se prévient en suspendant quelques jours.",
    },
    Rule {
        code: "GLY",
        side: Side::Below,
        threshold: 0.7,
        needs: &["insuline", "sulfamide", "glimépiride", "gliclazide", "répaglinide"],
        severity: Severity::Alert,
        text: "Glycémie basse sous insuline, sulfamide ou glinide : chercher le repas sauté, l'effort, l'alcool ou l'erreur de dose, et prévenir le prescripteur. Chez la personne âgée, l'hypoglycémie se manifeste par une chute ou une confusion, pas par des sueurs.",
    },
    Rule {
        code: "GLY",
        side: Side::Above,
        threshold: 1.26,
        needs: &["corticoïde", "prednisone", "prednisolone", "antipsychotique"],
        severity: Severity::Warn,
        text: "Hyperglycémie sous corticoïde ou antipsychotique : le diabète cortico-induit monte surtout l'après-midi et le soir, et se dépiste sur une glycémie post-prandiale plutôt qu'à jeun. Surveiller pendant toute la corticothérapie, et prévenir le patient déjà diabétique que ses doses vont bouger.",
    },
    Rule {
        code: "LDL",
        side: Side::Above,
        threshold: 1.0,
        needs: &["statine", "ézétimibe", "atorvastatine", "rosuvastatine"],
        severity: Severity::Warn,
        text: "LDL au-dessus de 1 g/L sous statine : la cible dépend du risque — 0,55 g/L après un infarctus, 0,70 en haut risque. Avant de parler d'intensification, vérifier l'observance réelle, l'horaire de prise et ce qui a été arrêté sur une douleur musculaire jamais reparlée.",
    },
    Rule {
        code: "TG",
        side: Side::Above,
        threshold: 5.0,
        needs: &[],
        severity: Severity::Alert,
        text: "Triglycérides au-delà de 5 g/L : risque de pancréatite aiguë, qui devient net au-delà de 10. Vérifier d'abord que le prélèvement était à jeun, puis avis pour un fibrate, un sevrage de l'alcool et une prise en charge diététique.",
    },
    Rule {
        code: "TG",
        side: Side::Above,
        threshold: 2.0,
        needs: &["isotrétinoïne", "antipsychotique", "corticoïde", "œstrogène"],
        severity: Severity::Warn,
        text: "Triglycérides élevés sous isotrétinoïne, antipsychotique, corticoïde ou œstrogène : la molécule y contribue et le bilan lipidique fait partie de sa surveillance. Sous isotrétinoïne, un contrôle s'impose avant de renouveler.",
    },
    Rule {
        code: "CRP",
        side: Side::Above,
        threshold: 100.0,
        needs: &[
            "immunosuppresseur",
            "anti-TNF",
            "corticoïde",
            "méthotrexate",
            "azathioprine",
        ],
        severity: Severity::Alert,
        text: "CRP franchement élevée sous immunosuppresseur, biothérapie ou corticoïde : une infection se cherche le jour même, et le traitement de fond se suspend le temps de la trancher. Le corticoïde masque la fièvre — l'absence de température ne rassure pas.",
    },
    Rule {
        code: "CA",
        side: Side::Above,
        threshold: 2.6,
        needs: &["vitamine D", "cholécalciférol", "calcium", "hydrochlorothiazide"],
        severity: Severity::Warn,
        text: "Hypercalcémie sous vitamine D, calcium ou thiazidique : suspendre la supplémentation, faire boire, et contrôler. Nausées, constipation, soif et confusion sont les signes que le chiffre est déjà haut.",
    },
    Rule {
        code: "CA",
        side: Side::Below,
        threshold: 2.2,
        needs: &["bisphosphonate", "alendronate", "dénosumab", "acide zolédronique"],
        severity: Severity::Alert,
        text: "Hypocalcémie sous bisphosphonate ou dénosumab : la calcémie et la vitamine D se corrigent avant l'injection, jamais après. Sous dénosumab l'hypocalcémie peut être sévère, surtout si le DFG est bas.",
    },
    Rule {
        code: "VITD",
        side: Side::Below,
        threshold: 20.0,
        needs: &["bisphosphonate", "dénosumab", "alendronate", "calcium"],
        severity: Severity::Warn,
        text: "Vitamine D effondrée sous traitement de l'ostéoporose : le traitement perd son efficacité et expose à l'hypocalcémie. Charge de correction puis entretien, et contrôle avant la prochaine injection.",
    },
    Rule {
        code: "FERR",
        side: Side::Below,
        threshold: 15.0,
        needs: &["AOD", "AVK", "antiagrégant", "aspirine", "AINS"],
        severity: Severity::Alert,
        text: "Carence martiale sous antithrombotique ou AINS : c'est un saignement digestif occulte jusqu'à preuve du contraire. La supplémentation ne dispense pas de chercher la cause, et l'exploration se demande avant de renouveler.",
    },
    Rule {
        code: "FERR",
        side: Side::Below,
        threshold: 15.0,
        needs: &["Tardyferon", "ferreux", "fumarate"],
        severity: Severity::Warn,
        text: "Ferritine toujours basse sous fer oral : reprendre la prise avant tout. Le fer s'absorbe à jeun, jamais avec le thé, le café, le calcium ou un IPP, et un comprimé un jour sur deux est mieux absorbé que deux le même jour.",
    },
    Rule {
        code: "ALB",
        side: Side::Below,
        threshold: 30.0,
        needs: &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        severity: Severity::Warn,
        text: "Hypoalbuminémie sous AVK : la fraction libre augmente et l'INR devient instable à dose inchangée. Contrôles rapprochés, et se méfier de tout ajout qui déplace la liaison protéique.",
    },
    Rule {
        code: "ASAT",
        side: Side::Above,
        threshold: 120.0,
        needs: &["statine", "fibrate", "atorvastatine", "simvastatine"],
        severity: Severity::Warn,
        text: "ASAT élevées sous statine ou fibrate : si elles dépassent les ALAT, penser au muscle avant le foie et demander les CPK. Une douleur musculaire diffuse avec des urines foncées ne s'explore pas au comptoir.",
    },
    Rule {
        code: "BILI",
        side: Side::Above,
        threshold: 50.0,
        needs: &[
            "amiodarone",
            "antifongique",
            "statine",
            "azathioprine",
            "amoxicilline",
        ],
        severity: Severity::Alert,
        text: "Hyperbilirubinémie sous un traitement hépatotoxique : suspendre et avis le jour même. Un ictère, des urines foncées ou des selles décolorées ne s'attendent pas — l'association amoxicilline-acide clavulanique est une cause classique et retardée.",
    },
    Rule {
        code: "RAC",
        side: Side::Above,
        threshold: 3.0,
        needs: &["IEC", "sartan", "gliflozine", "dapagliflozine", "empagliflozine"],
        severity: Severity::Warn,
        text: "Albuminurie sous IEC, sartan ou gliflozine : ces molécules sont le traitement, pas la cause. Une créatinine qui monte de moins de 30 % à l'instauration est attendue et ne fait pas arrêter ; c'est au-delà qu'on rappelle le prescripteur.",
    },
    Rule {
        code: "RAC",
        side: Side::Above,
        threshold: 30.0,
        needs: &[],
        severity: Severity::Warn,
        text: "Albuminurie franche : la néphroprotection se discute même sans diabète — bloqueur du système rénine-angiotensine, gliflozine, tension à la cible. Et l'ordonnance se relit du point de vue du rein, AINS en tête.",
    },
    Rule {
        code: "GGT",
        side: Side::Above,
        threshold: 110.0,
        needs: &["carbamazépine", "phénytoïne", "corticoïde", "AVK"],
        severity: Severity::Info,
        text: "GGT isolément élevée sous inducteur enzymatique : c'est attendu et ce n'est pas une hépatite. Ce qui compte, c'est ce que l'induction fait au reste de l'ordonnance — AVK, contraception, immunosuppresseur.",
    },
    Rule {
        code: "HDL",
        side: Side::Below,
        threshold: 0.40,
        needs: &["statine", "fibrate", "ézétimibe"],
        severity: Severity::Info,
        text: "HDL bas sous hypolipémiant : ce n'est pas la cible du traitement et aucune molécule ne le remonte utilement. Le LDL reste l'objectif ; le HDL se corrige par l'activité physique, l'arrêt du tabac et la perte de poids.",
    },
    Rule {
        code: "HCO3",
        side: Side::Below,
        threshold: 20.0,
        needs: &["metformine", "biguanide"],
        severity: Severity::Alert,
        text: "Réserve alcaline effondrée sous metformine : c'est le tableau biologique de l'acidose lactique. Suspendre et faire évaluer le jour même, d'autant plus s'il y a des vomissements, une diarrhée, des crampes ou une respiration ample.",
    },
    Rule {
        code: "HCO3",
        side: Side::Below,
        threshold: 22.0,
        needs: &[],
        severity: Severity::Warn,
        text: "Acidose métabolique débutante : chercher une insuffisance rénale, une diarrhée prolongée, un diabète déséquilibré ou un médicament — acétazolamide, topiramate, metformine.",
    },
    Rule {
        code: "BNP",
        side: Side::Above,
        threshold: 2000.0,
        needs: &["furosémide", "diurétique de l'anse", "bumétanide"],
        severity: Severity::Alert,
        text: "NT-proBNP très élevé sous diurétique de l'anse : décompensation cardiaque probable. Peser le patient, chercher l'essoufflement au moindre effort et les jambes gonflées, et faire évaluer sans attendre le prochain rendez-vous.",
    },
    Rule {
        code: "BNP",
        side: Side::Above,
        threshold: 125.0,
        needs: &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène"],
        severity: Severity::Warn,
        text: "NT-proBNP élevé chez un patient qui prend un AINS : l'AINS retient le sel et l'eau et décompense une insuffisance cardiaque. C'est la ligne à retirer, y compris si elle vient de l'automédication.",
    },
    Rule {
        code: "PSA",
        side: Side::Above,
        threshold: 4.0,
        needs: &["finastéride", "dutastéride", "5-alpha-réductase"],
        severity: Severity::Alert,
        text: "PSA au-dessus du seuil sous inhibiteur de la 5-alpha-réductase : ces molécules divisent le PSA par deux environ après six mois. Un chiffre déjà « normal » doit donc être doublé pour être interprété, et celui-ci, tel quel, est franchement anormal — le signaler.",
    },
    Rule {
        code: "CST",
        side: Side::Below,
        threshold: 20.0,
        needs: &["fer", "sulfate ferreux", "fumarate ferreux"],
        severity: Severity::Warn,
        text: "Saturation de la transferrine encore basse sous fer oral : soit le traitement n'est pas pris, soit il est mal absorbé. Vérifier la prise à jeun, à distance du thé, du café, du calcium et des IPP — un IPP à côté d'un fer oral annule une bonne partie du traitement.",
    },
    Rule {
        code: "CST",
        side: Side::Above,
        threshold: 60.0,
        needs: &[],
        severity: Severity::Alert,
        text: "Saturation au-delà de 60 % : surcharge en fer jusqu'à preuve du contraire — hémochromatose, transfusions répétées, supplémentation prolongée sans carence. Toute supplémentation martiale s'arrête et le bilan se fait.",
    },
    Rule {
        code: "T4L",
        side: Side::Above,
        threshold: 19.0,
        needs: &["lévothyroxine", "hormone thyroïdienne"],
        severity: Severity::Warn,
        text: "T4 libre au-dessus de l'intervalle sous lévothyroxine : surdosage. Palpitations, tremblements, insomnie et perte de poids le confirment ; chez la personne âgée le premier signe est souvent une fibrillation atriale, et chez l'ostéoporotique une perte osseuse silencieuse.",
    },
    Rule {
        code: "T4L",
        side: Side::Below,
        threshold: 9.0,
        needs: &["carbimazole", "thiamazole", "antithyroïdien", "propylthiouracile"],
        severity: Severity::Warn,
        text: "T4 libre basse sous antithyroïdien : la dose dépasse la cible. La TSH ne suit qu'au bout de six semaines et ne sert à rien pour cet ajustement — c'est la T4 libre qui guide la baisse.",
    },
    Rule {
        code: "PNE",
        side: Side::Above,
        threshold: 1.5,
        needs: &[],
        severity: Severity::Alert,
        text: "Éosinophilie franche : chercher un médicament introduit dans les deux à six semaines précédentes. Avec une éruption, de la fièvre et des transaminases hautes, c'est un DRESS — le médicament s'arrête et le patient est vu le jour même.",
    },
    Rule {
        code: "PNE",
        side: Side::Above,
        threshold: 0.5,
        needs: &["antibiotique", "antiépileptique", "allopurinol", "sulfamide"],
        severity: Severity::Warn,
        text: "Éosinophilie modérée sous une classe connue pour l'hypersensibilité retardée : la surveiller, et demander au patient s'il a une éruption, de la fièvre ou des ganglions. Ce sont les trois questions qui font la différence entre une anomalie et un DRESS qui commence.",
    },
    Rule {
        code: "LYMPHO",
        side: Side::Below,
        threshold: 0.5,
        needs: &["fingolimod", "diméthyle", "tériflunomide", "SEP", "immunomodulateur", "modulateur S1P"],
        severity: Severity::Alert,
        text: "Lymphopénie sévère sous traitement de fond de la sclérose en plaques : c'est le seuil auquel le traitement se suspend et où la LEMP cesse d'être théorique. Ne pas renouveler sans l'avis du neurologue.",
    },
    Rule {
        code: "LYMPHO",
        side: Side::Below,
        threshold: 1.0,
        needs: &["immunosuppresseur", "corticoïde", "méthotrexate", "azathioprine", "mycophénolate"],
        severity: Severity::Warn,
        text: "Lymphopénie sous immunosuppresseur : le risque est infectieux, et il est d'autant plus discret que le traitement masque la fièvre. Toute fièvre, toute toux qui dure, tout zona se signalent sans attendre.",
    },
    Rule {
        code: "ANTIXA",
        side: Side::Above,
        threshold: 1.5,
        needs: &["HBPM", "énoxaparine", "tinzaparine", "daltéparine", "héparine"],
        severity: Severity::Alert,
        text: "Activité anti-Xa au-dessus de la zone attendue au pic : accumulation. Chercher l'insuffisance rénale, qui en est la cause habituelle, et ne pas renouveler la dose sans avis. Vérifier aussi que le prélèvement a bien été fait quatre heures après l'injection, sans quoi il ne veut rien dire.",
    },
    Rule {
        code: "HB",
        side: Side::Below,
        threshold: 11.0,
        needs: &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "aspirine", "antiagrégant", "anticoagulant"],
        severity: Severity::Alert,
        text: "Anémie chez un patient qui prend un AINS, un antiagrégant ou un anticoagulant : c'est un saignement digestif jusqu'à preuve du contraire, et il est souvent indolore. Chercher les selles noires, et faire évaluer sans attendre le prochain bilan.",
    },
    Rule {
        code: "HB",
        side: Side::Below,
        threshold: 11.0,
        needs: &["IPP", "oméprazole", "pantoprazole", "ésoméprazole", "metformine"],
        severity: Severity::Warn,
        text: "Anémie sous IPP ou metformine au long cours : les deux gênent l'absorption de la vitamine B12, et l'IPP celle du fer. Un VGM élevé oriente vers la B12, un VGM bas vers le fer — le bilan tranche, et la carence se corrige.",
    },
    Rule {
        code: "PLQ",
        side: Side::Below,
        threshold: 100.0,
        needs: &["valproate", "dépakine", "dépakote", "linézolide", "interféron"],
        severity: Severity::Warn,
        text: "Thrombopénie sous valproate ou linézolide : elle est dose-dépendante pour le premier, et impose une NFS hebdomadaire pour le second. Signaler tout saignement de gencives, tout hématome spontané et toute pétéchie.",
    },
    Rule {
        code: "PNN",
        side: Side::Below,
        threshold: 1.5,
        needs: &["carbimazole", "thiamazole", "antithyroïdien", "propylthiouracile"],
        severity: Severity::Alert,
        text: "Neutropénie sous antithyroïdien : l'agranulocytose est l'accident de cette classe, et elle se manifeste par une fièvre avec angine. La consigne au patient est claire — toute fièvre fait arrêter le traitement et faire une NFS le jour même, sans attendre un rendez-vous.",
    },
    Rule {
        code: "ALAT",
        side: Side::Above,
        threshold: 150.0,
        needs: &["statine", "amiodarone", "méthotrexate", "isoniazide", "kétoconazole", "amoxicilline", "clavulanique"],
        severity: Severity::Alert,
        text: "Transaminases au-delà de trois fois la normale sous un médicament hépatotoxique : arrêter et faire évaluer. Sous amoxicilline-clavulanate, l'atteinte est cholestatique et peut apparaître après la fin du traitement — elle contre-indique l'association à vie, mais pas l'amoxicilline seule.",
    },
    Rule {
        code: "GGT",
        side: Side::Above,
        threshold: 150.0,
        needs: &["carbamazépine", "phénobarbital", "phénytoïne", "rifampicine", "millepertuis"],
        severity: Severity::Info,
        text: "Gamma-GT élevées sous inducteur enzymatique : c'est l'induction elle-même, pas une souffrance du foie, tant que les transaminases restent normales. En revanche, cet inducteur abaisse la concentration de tout ce qui l'accompagne — l'ordonnance se relit entière.",
    },
    Rule {
        code: "CREAT",
        side: Side::Above,
        threshold: 110.0,
        needs: &["triméthoprime", "cotrimoxazole", "bactrim", "dolutégravir", "cimétidine"],
        severity: Severity::Info,
        text: "Créatinine en hausse sous triméthoprime, dolutégravir ou cimétidine : ces molécules bloquent la sécrétion tubulaire de la créatinine sans altérer le rein. La hausse est de 10 à 20 %, apparaît en quelques jours et se stabilise — ce n'est pas une insuffisance rénale, et l'arrêter serait une erreur.",
    },
    Rule {
        code: "NA",
        side: Side::Below,
        threshold: 130.0,
        needs: &["ISRS", "IRSNa", "sertraline", "citalopram", "escitalopram", "paroxétine", "venlafaxine", "carbamazépine", "oxcarbazépine", "diurétique", "hydrochlorothiazide", "indapamide"],
        severity: Severity::Alert,
        text: "Hyponatrémie sous ISRS, carbamazépine ou thiazidique : c'est un SIADH médicamenteux, fréquent chez la personne âgée dans les premières semaines. Confusion, chutes et nausées en sont les signes, et ils passent pour de la vieillesse. Ne pas renouveler sans avis.",
    },
    Rule {
        code: "URIC",
        side: Side::Above,
        threshold: 420.0,
        needs: &["diurétique", "hydrochlorothiazide", "indapamide", "furosémide", "aspirine"],
        severity: Severity::Warn,
        text: "Hyperuricémie sous diurétique : le thiazidique et le diurétique de l'anse font monter l'acide urique et déclenchent des crises de goutte. Chez un patient goutteux, cela se discute avec le prescripteur — un autre antihypertenseur existe presque toujours.",
    },
    Rule {
        code: "CA",
        side: Side::Above,
        threshold: 2.60,
        needs: &["vitamine D", "cholécalciférol", "calcifédiol", "calcium", "thiazidique", "hydrochlorothiazide"],
        severity: Severity::Alert,
        text: "Hypercalcémie sous vitamine D, calcium ou thiazidique : suspendre la supplémentation et faire évaluer. Soif, nausées, urines abondantes, constipation et confusion sont les signes, et ils s'installent lentement — d'où le retard au diagnostic.",
    },
    Rule {
        code: "MG",
        side: Side::Below,
        threshold: 0.70,
        needs: &["IPP", "oméprazole", "pantoprazole", "ésoméprazole", "lansoprazole"],
        severity: Severity::Warn,
        text: "Hypomagnésémie sous IPP au long cours : elle apparaît après des mois ou des années, entretient une hypokaliémie qui ne se corrige pas, et donne crampes, tétanie et troubles du rythme. C'est aussi un argument pour réévaluer l'indication de l'IPP.",
    },
    Rule {
        code: "TSH",
        side: Side::Above,
        threshold: 10.0,
        needs: &["amiodarone", "lithium", "interféron"],
        severity: Severity::Warn,
        text: "Hypothyroïdie sous amiodarone ou lithium : les deux la provoquent, et l'amiodarone peut aussi faire l'inverse. La molécule ne s'arrête pas pour autant — c'est la thyroïde qu'on substitue, et la TSH qui se surveille tous les six mois.",
    },
    Rule {
        code: "HBA1C",
        side: Side::Below,
        threshold: 6.5,
        needs: &["insuline", "sulfamide hypoglycémiant", "gliclazide", "glimépiride", "répaglinide"],
        severity: Severity::Warn,
        text: "HbA1c basse sous insuline ou sulfamide chez un patient âgé : ce n'est pas un bon résultat, c'est un risque d'hypoglycémie. La cible se relâche après 75 ans et davantage encore en cas de fragilité — le sur-traitement du diabète de la personne âgée est aussi dangereux que le sous-traitement.",
    },
    Rule {
        code: "LDL",
        side: Side::Above,
        threshold: 1.0,
        needs: &["statine", "ézétimibe", "anti-PCSK9"],
        severity: Severity::Warn,
        text: "LDL au-dessus de la cible sous hypolipémiant : avant de monter la dose, vérifier que le traitement est pris — l'inobservance est la première cause d'échec, et les douleurs musculaires attribuées à la statine en sont le motif le plus fréquent. En prévention secondaire, la cible est plus basse encore.",
    },
    Rule {
        code: "CRP",
        side: Side::Above,
        threshold: 50.0,
        needs: &["corticoïde", "immunosuppresseur", "anti-TNF", "méthotrexate", "prednisone"],
        severity: Severity::Alert,
        text: "CRP franchement élevée chez un patient immunodéprimé : l'infection est à chercher activement, et le traitement masque la fièvre et la douleur qui la signaleraient. Un patient sous anti-TNF ou corticoïde avec une CRP à 50 se voit le jour même.",
    },
    Rule {
        code: "ALB",
        side: Side::Below,
        threshold: 30.0,
        needs: &["AVK", "warfarine", "fluindione", "phénytoïne", "furosémide"],
        severity: Severity::Warn,
        text: "Hypoalbuminémie sous médicament fortement lié aux protéines : la fraction libre — celle qui agit — augmente à concentration totale inchangée. Un INR qui s'emballe chez un patient dénutri vient souvent de là, et la dose se revoit avec le prescripteur.",
    },
    Rule {
        code: "UREE",
        side: Side::Above,
        threshold: 10.0,
        needs: &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide", "IEC", "sartan"],
        severity: Severity::Warn,
        text: "Urée élevée sous diurétique ou bloqueur du système rénine-angiotensine, créatinine encore acceptable : c'est le rein qui manque d'eau, pas encore le rein qui se dégrade. Chercher ce qui a fait perdre du volume — chaleur, diarrhée, diurétique majoré — et faire réévaluer avant que la créatinine ne suive.",
    },
    Rule {
        code: "UREE",
        side: Side::Above,
        threshold: 10.0,
        needs: &["AINS", "ibuprofène", "diclofénac", "anticoagulant", "AVK", "AOD", "antiagrégant"],
        severity: Severity::Alert,
        text: "Urée élevée avec une créatinine peu modifiée chez un patient sous AINS, anticoagulant ou antiagrégant : c'est aussi la signature d'un saignement digestif, le sang digéré fabriquant de l'urée. Chercher les selles noires et une anémie sur la même prise de sang, et faire évaluer sans attendre.",
    },
    Rule {
        code: "TP",
        side: Side::Below,
        threshold: 60.0,
        needs: &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        severity: Severity::Info,
        text: "Sous AVK, le TP n'est pas le chiffre à lire : c'est l'INR, et lui seul, qui dit si l'anticoagulation est dans sa zone. Un TP bas est attendu et ne se corrige pas pour lui-même — vérifier l'INR de la même prise de sang avant toute conclusion.",
    },
    Rule {
        code: "TP",
        side: Side::Below,
        threshold: 60.0,
        needs: &["paracétamol", "amiodarone", "méthotrexate", "isoniazide", "antifongique azolé", "statine"],
        severity: Severity::Alert,
        text: "TP bas chez un patient qui ne prend pas d'AVK : le foie fabrique les facteurs de coagulation, et un TP qui chute sous un médicament hépatotoxique est un signe de gravité, plus parlant que les transaminases. Faire évaluer sans attendre, et vérifier la dose cumulée de paracétamol.",
    },
    Rule {
        code: "GB",
        side: Side::Above,
        threshold: 12.0,
        needs: &["corticoïde", "prednisone", "prednisolone", "méthylprednisolone"],
        severity: Severity::Info,
        text: "Hyperleucocytose sous corticoïde : la molécule démargine les polynucléaires et fait monter le chiffre sans la moindre infection. C'est une cause classique d'antibiothérapie inutile. La formule, la CRP et surtout l'état du patient tranchent — et à l'inverse, le corticoïde masque la fièvre d'une vraie infection.",
    },
    Rule {
        code: "GB",
        side: Side::Below,
        threshold: 3.0,
        needs: &["clozapine", "carbimazole", "thiamazole", "antithyroïdien", "méthotrexate", "sulfasalazine", "colchicine"],
        severity: Severity::Alert,
        text: "Leucopénie sous une molécule qui donne des agranulocytoses : la consigne est la même pour toutes, et elle se donne au comptoir avant la première boîte — toute fièvre, toute angine, tout aphte fait arrêter le traitement et faire une numération le jour même, sans attendre un rendez-vous.",
    },
    Rule {
        code: "HTE",
        side: Side::Above,
        threshold: 52.0,
        needs: &["testostérone", "androgène", "érythropoïétine", "epoétine", "darbépoétine"],
        severity: Severity::Warn,
        text: "Hématocrite élevé sous testostérone ou agent stimulant l'érythropoïèse : c'est l'effet indésirable qui compte pour ces deux classes, parce qu'il fait le risque thrombotique. Au-delà de 54 %, la dose se réduit ou le traitement se suspend — c'est une décision du prescripteur, et elle ne se reporte pas au prochain bilan.",
    },
    Rule {
        code: "PTH",
        side: Side::Above,
        threshold: 65.0,
        needs: &["chélateur du phosphore", "calcitriol", "alfacalcidol", "cinacalcet", "vitamine D"],
        severity: Severity::Info,
        text: "Parathormone élevée dans l'insuffisance rénale chronique : c'est l'hyperparathyroïdie secondaire, et son traitement se lit à trois chiffres, jamais à un seul — phosphore, calcium et PTH ensemble. Rappeler que le chélateur du phosphore se prend au milieu du repas et pas à distance : pris à jeun, il ne chélate rien.",
    },
    Rule {
        code: "CT",
        side: Side::Above,
        threshold: 3.0,
        needs: &["statine", "ézétimibe", "anti-PCSK9", "fibrate", "acide bempédoïque"],
        severity: Severity::Warn,
        text: "Cholestérol total au-delà de 3 g/L malgré un hypolipémiant : vérifier d'abord que le traitement est pris — l'inobservance explique la majorité des échecs, et les douleurs musculaires attribuées à la statine en sont le motif le plus fréquent. Un chiffre aussi haut, surtout avant 40 ans ou avec un accident cardiovasculaire précoce dans la famille, fait évoquer une hypercholestérolémie familiale, qui se dépiste chez les apparentés au premier degré. C'est le LDL qui porte la cible : le total ne sert qu'à alerter.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_lands_where_it_should() {
        let k = find("K").unwrap();
        assert_eq!(level(k, 4.2), Level::Normal);
        assert_eq!(level(k, 3.2), Level::Low);
        assert_eq!(level(k, 5.4), Level::High);
        assert_eq!(level(k, 6.1), Level::CriticalHigh);
        assert_eq!(level(k, 2.9), Level::CriticalLow);
        // An analyte read against a target has no "normal".
        let ldl = find("LDL").unwrap();
        assert_eq!(level(ldl, 1.2), Level::Unknown);
        assert_eq!(interval_text(ldl), "");
        assert_eq!(interval_text(k), "3,5 – 5 mmol/L");
    }

    #[test]
    fn the_rules_need_the_treatment_to_fire() {
        let readings = [Reading {
            code: "K",
            value: 5.6,
            date: "2026-08-20",
        }];
        // On its own the value is reported, but nothing is alerted
        // about a treatment the patient does not take.
        let alone = read(&readings, &[]);
        assert!(alone.iter().all(|f| f.severity != Severity::Alert));
        assert!(alone.iter().any(|f| f.text.contains("Kaliémie")));
        // With a sartan on the file, it becomes an alert — and the
        // match is on the class, whatever the brand is called.
        let treated = read(&readings, &["Coversyl".to_owned(), "IEC".to_owned()]);
        assert!(treated.iter().any(|f| f.severity == Severity::Alert));
        // Loudest first.
        assert_eq!(treated[0].severity, Severity::Alert);
    }

    #[test]
    fn only_the_latest_reading_of_an_analyte_is_read() {
        // A kaliémie corrected since is not an alert today.
        let readings = [
            Reading {
                code: "K",
                value: 6.2,
                date: "2026-01-05",
            },
            Reading {
                code: "K",
                value: 4.1,
                date: "2026-08-20",
            },
        ];
        let found = read(&readings, &["IEC".to_owned()]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn the_kidney_rules_read_the_treatments() {
        let readings = [Reading {
            code: "DFG",
            value: 26.0,
            date: "2026-08-20",
        }];
        let found = read(&readings, &["Eliquis".to_owned(), "apixaban".to_owned()]);
        assert!(found
            .iter()
            .any(|f| f.text.contains("anticoagulant oral direct")));
        // The general rule fires for everyone, treatment or not.
        let anyone = read(&readings, &[]);
        assert!(anyone
            .iter()
            .any(|f| f.text.contains("relire toute l'ordonnance")));
    }

    #[test]
    fn every_rule_names_an_analyte_of_the_catalogue() {
        for rule in RULES {
            assert!(
                find(rule.code).is_some(),
                "règle sur un analyte inconnu : {}",
                rule.code
            );
            assert!(!rule.text.trim().is_empty());
        }
        for a in CATALOGUE {
            assert!(!a.note.trim().is_empty(), "{} sans commentaire", a.code);
            assert!(!a.label.trim().is_empty());
        }
    }

    /// A rule that names a treatment no card carries can never fire.
    /// The shipped base is what these rules were written against, so
    /// each of them must match something in it.
    #[test]
    fn every_rule_can_fire_on_the_base_as_shipped() {
        for rule in RULES {
            if rule.needs.is_empty() {
                continue;
            }
            let reachable = rule.needs.iter().any(|needle| {
                let needle = crate::fuzzy::sort_key(needle);
                crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::sort_key(&format!("{name} {dci} {class}")).contains(&needle)
                    })
            });
            assert!(
                reachable,
                "règle sur {} : aucun médicament de la base de départ ne correspond à {:?}",
                rule.code, rule.needs
            );
        }
    }

    /// An analyte with no rule is a number the application displays and
    /// says nothing about — the laboratory already does that, and does
    /// it better. What an officine adds is what the value changes for
    /// the treatments in front of it, so every analyte carries at least
    /// one rule. Adding an analyte means writing that rule too.
    #[test]
    fn every_analyte_says_something_about_a_treatment() {
        // Both catalogues only ever grow: an analyte withdrawn is a
        // value the counter can no longer read, and a rule withdrawn is
        // a reading nobody does any more.
        assert!(
            CATALOGUE.len() >= 49,
            "{} analytes, il y en avait quarante-neuf",
            CATALOGUE.len()
        );
        assert!(
            RULES.len() >= 87,
            "{} règles de biologie, il y en avait quatre-vingt-sept",
            RULES.len()
        );
        let mut codes: Vec<&str> = CATALOGUE.iter().map(|a| a.code).collect();
        codes.sort_unstable();
        let seen = codes.len();
        codes.dedup();
        assert_eq!(seen, codes.len(), "deux analytes portent le même code");
        for a in CATALOGUE {
            assert!(
                RULES.iter().any(|r| r.code == a.code),
                "{} ({}) : aucun règle ne lit cette valeur — un analyte sans règle n'est qu'un \
                 chiffre recopié",
                a.code,
                a.label
            );
        }
    }

    #[test]
    fn the_search_finds_an_analyte_by_code_or_by_name() {
        assert_eq!(search("kalie")[0].code, "K");
        assert_eq!(search("DFG")[0].code, "DFG");
        assert_eq!(search("")[0].code, CATALOGUE[0].code);
    }
}
