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
