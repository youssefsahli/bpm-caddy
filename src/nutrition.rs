//! La dénutrition et les compléments nutritionnels oraux (CNO).
//!
//! **Deux jeux de critères, à ne pas confondre.**
//!
//! - **Le diagnostic** suit la HAS : 2019 pour l'adulte de moins de
//!   70 ans, 2021 (avec la FFN) pour 70 ans et plus. Il faut un critère
//!   phénotypique **et** un critère étiologique ; l'albuminémie ne fait
//!   pas le diagnostic, elle en dit la sévérité, et un seul critère de
//!   sévérité l'emporte sur tous les critères modérés.
//! - **La prise en charge des CNO** suit la LPP (arrêté du 7 mai 2019,
//!   mémo de l'Assurance Maladie d'août 2025), dont les critères datent
//!   de la HAS 2007 : IMC ≤ 21, MNA ≤ 17 ou albuminémie < 35 g/L après
//!   70 ans, par exemple. Une personne peut être dénutrie au sens de la
//!   HAS et hors des critères de la LPP, et inversement : la vue montre
//!   les deux réponses côte à côte.
//!
//! Les objectifs d'apport (30 à 40 kcal/kg/j, 1,2 à 1,5 g de protéines
//! /kg/j) sont ceux de la HAS 2007 pour la personne âgée ; il n'y en a
//! pas d'officiel pour l'adulte plus jeune, et le module n'en donne pas.
//! L'objectif des CNO (+400 kcal et/ou +30 g de protéines par jour, le
//! plus souvent 2 unités) vaut pour tout adulte.
//!
//! Pur et sans horloge.

// ---------------------------------------------------------------------
// L'évaluation
// ---------------------------------------------------------------------

/// Ce qui est connu de la personne. Tout est facultatif sauf l'âge, le
/// poids et la taille : un critère dont la donnée manque n'est pas lu.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Assessment {
    pub age: u32,
    /// kg.
    pub weight: f64,
    /// cm.
    pub height: f64,
    /// Poids habituel avant la maladie, kg.
    pub usual_weight: Option<f64>,
    /// Poids il y a un mois, kg.
    pub weight_1m: Option<f64>,
    /// Poids il y a six mois, kg.
    pub weight_6m: Option<f64>,
    /// Albuminémie, g/L (immunonéphélémétrie ou immunoturbidimétrie).
    pub albumin: Option<f64>,
    /// Score MNA sur 30 (critère de la LPP après 70 ans).
    pub mna: Option<f64>,
    /// Réduction quantifiée de la masse ou de la fonction musculaire
    /// (moins de 70 ans), ou sarcopénie confirmée (70 ans et plus).
    pub muscle: bool,
    /// Réduction de la prise alimentaire : de moitié ou plus pendant
    /// plus d'une semaine, ou quelle qu'elle soit pendant plus de deux.
    pub reduced_intake: bool,
    /// Absorption réduite (maldigestion, malabsorption).
    pub malabsorption: bool,
    /// Situation d'agression : pathologie aiguë, chronique évolutive ou
    /// maligne évolutive.
    pub aggression: bool,
}

/// La sévérité d'une dénutrition diagnostiquée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Moderate,
    Severe,
}

/// La réponse : ce que chaque critère a lu, puis la conclusion.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub bmi: Option<f64>,
    /// Pertes de poids en pourcentage : sur un mois, sur six mois, et
    /// par rapport au poids habituel.
    pub loss_1m: Option<f64>,
    pub loss_6m: Option<f64>,
    pub loss_usual: Option<f64>,
    /// Le seuil d'âge de la recommandation appliquée : `true` pour la
    /// HAS 2021 (70 ans et plus).
    pub elderly: bool,
    /// L'IMC ne compte pas chez la personne obèse (IMC ≥ 30) : une
    /// personne obèse peut être dénutrie.
    pub obese: bool,
    pub phenotypic: Vec<&'static str>,
    pub etiologic: Vec<&'static str>,
    /// `None` : pas de dénutrition selon les critères renseignés.
    pub severity: Option<Severity>,
    /// Les critères qui font la sévérité.
    pub severe_because: Vec<&'static str>,
    /// Critères LPP de prise en charge des CNO remplis.
    pub lpp: Vec<&'static str>,
}

fn loss(from: Option<f64>, now: f64) -> Option<f64> {
    let from = from.filter(|w| *w > 0.0)?;
    Some(((from - now) / from * 100.0).max(0.0))
}

/// L'indice de masse corporelle, kg/m².
pub fn bmi(weight: f64, height_cm: f64) -> Option<f64> {
    (weight > 0.0 && height_cm > 0.0).then(|| weight / (height_cm / 100.0).powi(2))
}

/// Lire une évaluation.
pub fn assess(a: &Assessment) -> Reading {
    let elderly = a.age >= 70;
    let bmi = bmi(a.weight, a.height);
    let obese = bmi.is_some_and(|b| b >= 30.0);
    let loss_1m = loss(a.weight_1m, a.weight);
    let loss_6m = loss(a.weight_6m, a.weight);
    let loss_usual = loss(a.usual_weight, a.weight);

    let mut phenotypic = Vec::new();
    if loss_1m.is_some_and(|l| l >= 5.0) {
        phenotypic.push("Perte de poids d'au moins 5 % en 1 mois");
    }
    if loss_6m.is_some_and(|l| l >= 10.0) {
        phenotypic.push("Perte de poids d'au moins 10 % en 6 mois");
    }
    if loss_usual.is_some_and(|l| l >= 10.0) {
        phenotypic.push("Perte de poids d'au moins 10 % par rapport au poids habituel");
    }
    if !obese {
        match bmi {
            Some(b) if elderly && b < 22.0 => phenotypic.push("IMC inférieur à 22 kg/m²"),
            Some(b) if !elderly && b < 18.5 => phenotypic.push("IMC inférieur à 18,5 kg/m²"),
            _ => {}
        }
    }
    if a.muscle {
        phenotypic.push(if elderly {
            "Sarcopénie confirmée"
        } else {
            "Réduction quantifiée de la masse ou de la fonction musculaire"
        });
    }

    let mut etiologic = Vec::new();
    if a.reduced_intake {
        etiologic.push("Réduction de la prise alimentaire");
    }
    if a.malabsorption {
        etiologic.push("Absorption réduite");
    }
    if a.aggression {
        etiologic.push("Situation d'agression");
    }

    let diagnosed = !phenotypic.is_empty() && !etiologic.is_empty();
    let mut severe_because = Vec::new();
    if diagnosed {
        if loss_1m.is_some_and(|l| l >= 10.0) {
            severe_because.push("Perte de poids d'au moins 10 % en 1 mois");
        }
        if loss_6m.is_some_and(|l| l >= 15.0) {
            severe_because.push("Perte de poids d'au moins 15 % en 6 mois");
        }
        if loss_usual.is_some_and(|l| l >= 15.0) {
            severe_because.push("Perte de poids d'au moins 15 % par rapport au poids habituel");
        }
        if !obese {
            match bmi {
                Some(b) if elderly && b < 20.0 => severe_because.push("IMC inférieur à 20 kg/m²"),
                Some(b) if !elderly && b <= 17.0 => {
                    severe_because.push("IMC inférieur ou égal à 17 kg/m²")
                }
                _ => {}
            }
        }
        // Les deux recommandations ne posent pas la borne au même
        // endroit : ≤ 30 g/L avant 70 ans (HAS 2019), < 30 g/L après
        // (fiche outil HAS 2021).
        match a.albumin {
            Some(x) if elderly && x < 30.0 => {
                severe_because.push("Albuminémie inférieure à 30 g/L")
            }
            Some(x) if !elderly && x <= 30.0 => {
                severe_because.push("Albuminémie inférieure ou égale à 30 g/L")
            }
            _ => {}
        }
    }
    let severity = diagnosed.then_some({
        if severe_because.is_empty() {
            Severity::Moderate
        } else {
            Severity::Severe
        }
    });

    // La LPP : ses propres critères, sans critère étiologique.
    let mut lpp = Vec::new();
    if loss_1m.is_some_and(|l| l >= 5.0) {
        lpp.push("Perte de poids d'au moins 5 % en 1 mois");
    }
    if loss_6m.is_some_and(|l| l >= 10.0) {
        lpp.push("Perte de poids d'au moins 10 % en 6 mois");
    }
    if elderly {
        if bmi.is_some_and(|b| b <= 21.0) {
            lpp.push("IMC inférieur ou égal à 21 kg/m²");
        }
        if a.mna.is_some_and(|m| m <= 17.0) {
            lpp.push("MNA inférieur ou égal à 17/30");
        }
        if a.albumin.is_some_and(|x| x < 35.0) {
            lpp.push("Albuminémie inférieure à 35 g/L");
        }
    } else if bmi.is_some_and(|b| b <= 18.5) {
        lpp.push("IMC inférieur ou égal à 18,5 kg/m² (hors maigreur constitutionnelle)");
    }

    Reading {
        bmi,
        loss_1m,
        loss_6m,
        loss_usual,
        elderly,
        obese,
        phenotypic,
        etiologic,
        severity,
        severe_because,
        lpp,
    }
}

/// Les objectifs d'apport de la HAS 2007 pour la personne âgée, en
/// kcal/j et en g de protéines/j : `(kcal bas, kcal haut, prot bas,
/// prot haut)`. `None` avant 70 ans : il n'y a pas de repère officiel.
pub fn targets(age: u32, weight: f64) -> Option<(f64, f64, f64, f64)> {
    (age >= 70 && weight > 0.0).then_some((
        30.0 * weight,
        40.0 * weight,
        1.2 * weight,
        1.5 * weight,
    ))
}

/// L'apport des CNO visé par jour : +400 kcal et/ou +30 g de protéines.
pub const CNO_KCAL: f64 = 400.0;
pub const CNO_PROTEIN: f64 = 30.0;

// ---------------------------------------------------------------------
// Les produits
// ---------------------------------------------------------------------

/// Un CNO tel que l'officine le garde. Seedé une fois depuis
/// [`STARTER_CNO`], puis à l'équipe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub maker: String,
    /// Boisson lactée, jus de fruits, crème, potage, poudre…
    pub form: String,
    /// Le contenu d'une unité, en ml ou en g.
    pub portion: f64,
    /// `ml` ou `g`.
    pub unit: String,
    pub kcal: f64,
    pub protein: f64,
    /// Ce qui distingue le produit : sans lactose, fibres, IDDSI…
    pub features: String,
    /// Précautions du fabricant : âge minimal, galactosémie, insuffisance
    /// rénale…
    pub caution: String,
    pub source: String,
}

/// La catégorie de la nomenclature LPP de 2019, lue sur la densité pour
/// 100 ml ou 100 g. `None` quand le produit n'en atteint aucune — ce
/// qui ne dit pas qu'il n'est pas inscrit : la LPP fait foi.
pub fn category(p: &Product) -> Option<&'static str> {
    if p.portion <= 0.0 {
        return None;
    }
    let kcal = p.kcal / p.portion; // par ml ou g
    let prot = p.protein / p.portion * 100.0; // par 100 ml ou g
    Some(if prot >= 14.0 && kcal >= 2.25 && p.unit == "ml" {
        "D — hyperprotidique hyperénergétique concentré"
    } else if prot >= 10.0 && kcal >= 1.8 {
        "C — hyperprotidique hyperénergétique"
    } else if prot >= 7.0 && kcal >= 1.5 {
        "B/C — hyperprotidique hyperénergétique"
    } else if prot >= 7.0 && kcal >= 1.0 {
        "B — hyperprotidique normoénergétique"
    } else if (4.5..7.0).contains(&prot) && kcal >= 1.5 {
        "A — normoprotidique hyperénergétique"
    } else {
        return None;
    })
}

/// Une ligne d'un plan : un produit et le nombre d'unités par jour.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanLine {
    pub product: Product,
    pub units: f64,
}

/// Ce qu'un plan apporte par jour : kcal, protéines.
pub fn plan_totals(plan: &[PlanLine]) -> (f64, f64) {
    plan.iter().fold((0.0, 0.0), |(k, p), l| {
        (
            k + l.product.kcal * l.units,
            p + l.product.protein * l.units,
        )
    })
}

/// La forme écrite en base d'un plan : `id×unités` séparés par des
/// points-virgules.
pub fn encode_plan(plan: &[(i64, f64)]) -> String {
    // Des espaces entre les lignes : le plan voyage dans un champ de
    // `counsel_records`, dont les points-virgules séparent les champs.
    plan.iter()
        .map(|(id, u)| format!("{id}x{u}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn decode_plan(text: &str) -> Vec<(i64, f64)> {
    text.split([';', ' '])
        .filter_map(|p| {
            let (id, u) = p.split_once('x')?;
            Some((id.trim().parse().ok()?, u.trim().parse().ok()?))
        })
        .collect()
}

/// Les produits livrés : relevés sur les pages des fabricants
/// (octobre 2026). Chaque chiffre est celui de la page citée en source ;
/// l'équipe corrige ce qui change, et une fiche réécrite n'est jamais
/// ressemée par-dessus.
pub struct StarterCno {
    pub name: &'static str,
    pub maker: &'static str,
    pub form: &'static str,
    pub portion: f64,
    pub unit: &'static str,
    pub kcal: f64,
    pub protein: f64,
    pub features: &'static str,
    pub caution: &'static str,
    pub source: &'static str,
}

const NESTLE: &str = "Nestlé Health Science — nestlehealthscience.fr, gamme Clinutren";
const NUTRICIA: &str = "Nutricia — nutricia.fr, gamme Fortimel";
const FRESENIUS: &str = "Fresenius Kabi — fresubin.com/fr, gamme Fresubin";
const DELICAL: &str = "Lactalis Nutrition Santé — delical.fr, produits professionnels";

pub const STARTER_CNO: &[StarterCno] = &[
    StarterCno {
        name: "Clinutren Boisson 2kcal",
        maker: "Nestlé Health Science",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 400.0,
        protein: 20.0,
        features: "Six saveurs.",
        caution: "Ne convient pas aux enfants de moins de 3 ans.",
        source: NESTLE,
    },
    StarterCno {
        name: "Clinutren Ultra",
        maker: "Nestlé Health Science",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 450.0,
        protein: 32.0,
        features: "Concentré : 2,25 kcal/ml et 16 g de protéines pour 100 ml.",
        caution: "Ne convient pas aux enfants de moins de 14 ans. Précaution en cas d'insuffisance rénale et sous anti-vitamine K.",
        source: NESTLE,
    },
    StarterCno {
        name: "Clinutren Renutryl Booster",
        maker: "Nestlé Health Science",
        form: "Boisson lactée",
        portion: 300.0,
        unit: "ml",
        kcal: 600.0,
        protein: 30.0,
        features: "Sans lactose, sans gluten. Une bouteille par jour.",
        caution: "Précaution en cas d'insuffisance rénale.",
        source: NESTLE,
    },
    StarterCno {
        name: "Clinutren Dessert 2kcal",
        maker: "Nestlé Health Science",
        form: "Crème",
        portion: 200.0,
        unit: "g",
        kcal: 400.0,
        protein: 20.0,
        features: "Texture IDDSI 3 à 4. Index glycémique bas selon le fabricant, sans dispenser du contrôle glycémique.",
        caution: "",
        source: NESTLE,
    },
    StarterCno {
        name: "Clinutren Fruit",
        maker: "Nestlé Health Science",
        form: "Jus de fruits",
        portion: 200.0,
        unit: "ml",
        kcal: 300.0,
        protein: 8.0,
        features: "Sans lipides. Une à trois unités par jour.",
        caution: "",
        source: NESTLE,
    },
    StarterCno {
        name: "Clinutren Velouté",
        maker: "Nestlé Health Science",
        form: "Potage",
        portion: 200.0,
        unit: "ml",
        kcal: 360.0,
        protein: 20.0,
        features: "Se consomme chaud : micro-ondes, sans faire bouillir ; ne pas réchauffer une seconde fois.",
        caution: "Ne convient pas aux enfants de moins de 10 ans.",
        source: NESTLE,
    },
    StarterCno {
        name: "Fortimel Creme Protein 2 kcal",
        maker: "Nutricia",
        form: "Crème",
        portion: 200.0,
        unit: "g",
        kcal: 400.0,
        protein: 20.0,
        features: "Lactose 0,6 g par pot.",
        caution: "Ne convient pas en cas de galactosémie.",
        source: NUTRICIA,
    },
    StarterCno {
        name: "Fortimel Compact Protein",
        maker: "Nutricia",
        form: "Boisson lactée",
        portion: 125.0,
        unit: "ml",
        kcal: 306.0,
        protein: 18.0,
        features: "Petit volume : 2,4 kcal/ml. Une à trois unités par jour.",
        caution: "Ne convient pas aux enfants de moins de 6 ans ni en cas de galactosémie. Précaution en cas d'insuffisance rénale.",
        source: NUTRICIA,
    },
    StarterCno {
        name: "Fortimel Multifibre 1.5 kcal",
        maker: "Nutricia",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 308.0,
        protein: 12.0,
        features: "Avec fibres. Peut se consommer chaud, sans ébullition.",
        caution: "",
        source: NUTRICIA,
    },
    StarterCno {
        name: "Fresubin 2 kcal Drink",
        maker: "Fresenius Kabi",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 400.0,
        protein: 20.0,
        features: "Sans gluten, sans lactose. Sept arômes, dont neutre.",
        caution: "Ne convient pas en cas de galactosémie.",
        source: FRESENIUS,
    },
    StarterCno {
        name: "Fresubin 2 kcal Crème",
        maker: "Fresenius Kabi",
        form: "Crème",
        portion: 200.0,
        unit: "g",
        kcal: 400.0,
        protein: 20.0,
        features: "Sans lactose. Adaptée aux troubles de la déglutition selon le fabricant.",
        caution: "",
        source: FRESENIUS,
    },
    StarterCno {
        name: "Fresubin DB 2 kcal Drink",
        maker: "Fresenius Kabi",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 400.0,
        protein: 20.0,
        features: "6 g de fibres par unité. Conçu pour les troubles du métabolisme glucidique.",
        caution: "",
        source: FRESENIUS,
    },
    StarterCno {
        name: "Fresubin Jucy Drink",
        maker: "Fresenius Kabi",
        form: "Jus de fruits",
        portion: 200.0,
        unit: "ml",
        kcal: 300.0,
        protein: 8.0,
        features: "Sans lactose. Intérêt en cas de malabsorption des lipides.",
        caution: "",
        source: FRESENIUS,
    },
    StarterCno {
        name: "Fresubin Plant Based Drink",
        maker: "Fresenius Kabi",
        form: "Boisson végétale",
        portion: 200.0,
        unit: "ml",
        kcal: 300.0,
        protein: 15.0,
        features: "Protéines de soja, sans protéines de lait.",
        caution: "Contient du soja.",
        source: FRESENIUS,
    },
    StarterCno {
        name: "Delical Boisson HP HC",
        maker: "Lactalis Nutrition Santé",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 360.0,
        protein: 20.0,
        features: "Texture IDDSI 1. Certaines saveurs se consomment tièdes, sans faire bouillir.",
        caution: "Contient du lactose.",
        source: DELICAL,
    },
    StarterCno {
        name: "Delical Boisson Concentrée HP HC",
        maker: "Lactalis Nutrition Santé",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 452.0,
        protein: 29.0,
        features: "Sans lactose, sans résidu.",
        caution: "Prescription à adapter en cas d'insuffisance rénale ou hépatique.",
        source: DELICAL,
    },
    StarterCno {
        name: "Delical Boisson HP HC sans sucres",
        maker: "Lactalis Nutrition Santé",
        form: "Boisson lactée",
        portion: 200.0,
        unit: "ml",
        kcal: 361.0,
        protein: 20.0,
        features: "7 g de fibres par unité. Adaptée à un régime de restriction en sucres.",
        caution: "",
        source: DELICAL,
    },
    StarterCno {
        name: "Delical Nutra'Pote",
        maker: "Lactalis Nutrition Santé",
        form: "Compote",
        portion: 200.0,
        unit: "g",
        kcal: 280.0,
        protein: 8.2,
        features: "Texture IDDSI 3. 6 g de fibres par pot.",
        caution: "",
        source: DELICAL,
    },
];

/// Les produits livrés, comme lignes à semer.
pub fn starter_products() -> Vec<Product> {
    STARTER_CNO
        .iter()
        .map(|s| Product {
            id: 0,
            name: s.name.to_owned(),
            maker: s.maker.to_owned(),
            form: s.form.to_owned(),
            portion: s.portion,
            unit: s.unit.to_owned(),
            kcal: s.kcal,
            protein: s.protein,
            features: s.features.to_owned(),
            caution: s.caution.to_owned(),
            source: s.source.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(age: u32) -> Assessment {
        Assessment {
            age,
            weight: 60.0,
            height: 170.0,
            ..Default::default()
        }
    }

    #[test]
    fn a_phenotypic_criterion_alone_is_not_a_diagnosis() {
        let mut a = base(50);
        a.weight_1m = Some(64.0); // 6,25 %
        let r = assess(&a);
        assert_eq!(r.phenotypic.len(), 1);
        assert_eq!(r.severity, None, "il faut aussi un critère étiologique");
        a.reduced_intake = true;
        assert_eq!(assess(&a).severity, Some(Severity::Moderate));
    }

    #[test]
    fn one_severe_criterion_outweighs_the_moderate_ones() {
        let mut a = base(50);
        a.weight_1m = Some(64.0);
        a.aggression = true;
        a.albumin = Some(30.0);
        let r = assess(&a);
        assert_eq!(r.severity, Some(Severity::Severe));
        assert_eq!(
            r.severe_because,
            ["Albuminémie inférieure ou égale à 30 g/L"]
        );
        // Après 70 ans, la borne est stricte (fiche outil HAS 2021).
        let mut b = base(80);
        b.weight_1m = Some(64.0);
        b.aggression = true;
        b.albumin = Some(30.0);
        assert_eq!(assess(&b).severity, Some(Severity::Moderate));
    }

    #[test]
    fn the_bmi_threshold_moves_at_seventy() {
        // 60 kg, 170 cm : IMC 20,8.
        let mut a = base(50);
        a.reduced_intake = true;
        assert_eq!(assess(&a).severity, None, "20,8 ne dit rien avant 70 ans");
        let mut b = base(75);
        b.reduced_intake = true;
        let r = assess(&b);
        assert_eq!(r.severity, Some(Severity::Moderate), "< 22 après 70 ans");
        assert!(r.lpp.contains(&"IMC inférieur ou égal à 21 kg/m²"));
    }

    #[test]
    fn an_obese_patient_is_read_on_weight_loss_not_on_bmi() {
        let mut a = Assessment {
            age: 75,
            weight: 95.0,
            height: 165.0,
            reduced_intake: true,
            ..Default::default()
        };
        let r = assess(&a);
        assert!(r.obese);
        assert_eq!(r.severity, None);
        a.weight_6m = Some(110.0); // 13,6 %
        assert_eq!(assess(&a).severity, Some(Severity::Moderate));
    }

    #[test]
    fn the_lpp_reads_its_own_criteria() {
        let mut a = base(75);
        a.weight = 70.0; // IMC 24,2
        assert!(assess(&a).lpp.is_empty());
        a.mna = Some(16.5);
        assert_eq!(assess(&a).lpp, ["MNA inférieur ou égal à 17/30"]);
        // Avant 70 ans, ni le MNA ni l'albuminémie ne comptent.
        let mut b = base(60);
        b.weight = 70.0;
        b.mna = Some(10.0);
        b.albumin = Some(25.0);
        assert!(assess(&b).lpp.is_empty());
    }

    #[test]
    fn the_targets_exist_only_where_the_has_wrote_them() {
        assert_eq!(targets(65, 60.0), None);
        assert_eq!(targets(80, 60.0), Some((1800.0, 2400.0, 72.0, 90.0)));
    }

    #[test]
    fn the_shipped_products_land_in_the_category_their_label_claims() {
        let ps = starter_products();
        let by = |n: &str| ps.iter().find(|p| p.name == n).unwrap();
        assert_eq!(category(by("Clinutren Ultra")).map(|c| &c[..1]), Some("D"));
        assert_eq!(
            category(by("Clinutren Boisson 2kcal")).map(|c| &c[..1]),
            Some("C")
        );
        assert_eq!(
            category(by("Clinutren Fruit")),
            None,
            "jus : pas hyperprotidique"
        );
        for p in &ps {
            assert!(
                p.kcal > 0.0 && p.protein > 0.0 && p.portion > 0.0,
                "{}",
                p.name
            );
            assert!(p.unit == "ml" || p.unit == "g", "{}", p.name);
            assert!(!p.source.is_empty(), "{}", p.name);
        }
        let mut names: Vec<&str> = ps.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ps.len(), "noms uniques");
    }

    #[test]
    fn a_plan_adds_up_and_travels_through_its_text() {
        let ps = starter_products();
        let plan = vec![
            PlanLine {
                product: ps[0].clone(),
                units: 2.0,
            },
            PlanLine {
                product: ps[4].clone(),
                units: 1.0,
            },
        ];
        assert_eq!(plan_totals(&plan), (1100.0, 48.0));
        let text = encode_plan(&[(3, 2.0), (7, 0.5)]);
        assert_eq!(decode_plan(&text), vec![(3, 2.0), (7, 0.5)]);
        // Le plan passe tel quel par le format des fiches de dossier.
        let data = crate::db::CounselRecord::encode(&[("plan", &text)]);
        let rec = crate::db::CounselRecord {
            data,
            ..Default::default()
        };
        assert_eq!(decode_plan(rec.field("plan")), vec![(3, 2.0), (7, 0.5)]);
    }
}
