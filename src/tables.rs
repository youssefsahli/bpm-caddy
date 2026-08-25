//! Conversion / equivalence and reference tables for the counter: dose
//! equivalences (IPP, statines, corticoïdes, opioïdes,
//! benzodiazépines), dosing references (HBPM, AOD, corticoïdes
//! inhalés, insulines, antalgiques), and the decision aids the acts
//! need (fonction rénale, angine, cystite, contraception, vaccins).
//!
//! The values are the classic published reference equivalences taught
//! in French pharmacy practice. They are deliberately static reference
//! data (like the starter drug list) — each table carries its own
//! caution line, shown on screen and on the printout.

pub struct ConvTable {
    /// Short name for the selector buttons.
    pub short: &'static str,
    pub title: &'static str,
    /// Shown under the table, on screen and in the PDF.
    pub caution: &'static str,
    pub columns: &'static [&'static str],
    pub rows: &'static [&'static [&'static str]],
}

pub const TABLES: &[ConvTable] = &[
    ConvTable {
        short: "IPP",
        title: "IPP — équivalences de doses",
        caution: "Doses AMM françaises usuelles ; adapter à l'indication (RGO, œsophagite, éradication…).",
        columns: &["DCI (spécialité)", "Pleine dose / j", "Demi-dose / j"],
        rows: &[
            &["Oméprazole (Mopral)", "20 mg", "10 mg"],
            &["Ésoméprazole (Inexium)", "40 mg", "20 mg"],
            &["Lansoprazole (Lanzor, Ogast)", "30 mg", "15 mg"],
            &["Pantoprazole (Inipomp, Eupantol)", "40 mg", "20 mg"],
            &["Rabéprazole (Pariet)", "20 mg", "10 mg"],
        ],
    },
    ConvTable {
        short: "HBPM",
        title: "HBPM — posologies usuelles",
        caution: "Posologies usuelles adulte, fonction rénale normale ; vérifier l'indication, le poids et la clairance.",
        columns: &["DCI (spécialité)", "Curatif", "Prophylaxie"],
        rows: &[
            &["Énoxaparine (Lovenox)", "100 UI/kg x2/j", "4 000 UI x1/j"],
            &["Tinzaparine (Innohep)", "175 UI/kg x1/j", "3 500 à 4 500 UI x1/j"],
            &[
                "Daltéparine (Fragmine)",
                "100 UI/kg x2/j (ou 200 UI/kg x1/j)",
                "2 500 à 5 000 UI x1/j",
            ],
            &["Nadroparine (Fraxiparine)", "85 UI/kg x2/j", "2 850 UI x1/j"],
            &[
                "Fondaparinux (Arixtra) — apparenté",
                "7,5 mg x1/j (50 à 100 kg)",
                "2,5 mg x1/j",
            ],
        ],
    },
    ConvTable {
        short: "Statines",
        title: "Statines — doses d'efficacité comparable",
        caution: "Équivalences approximatives sur la baisse du LDL-c ; l'intensité cible dépend du risque cardiovasculaire.",
        columns: &["DCI (spécialité)", "Dose ≈ équivalente"],
        rows: &[
            &["Rosuvastatine (Crestor)", "5 mg"],
            &["Atorvastatine (Tahor)", "10 mg"],
            &["Simvastatine (Zocor)", "20 mg"],
            &["Pravastatine (Elisor, Vasten)", "40 mg"],
            &["Fluvastatine (Lescol, Fractal)", "80 mg"],
        ],
    },
    ConvTable {
        short: "Corticoïdes",
        title: "Corticoïdes — équivalences anti-inflammatoires",
        caution: "Équivalence anti-inflammatoire approximative ; durées d'action et effets minéralocorticoïdes différents.",
        columns: &["DCI", "Dose équivalente", "vs prednisone"],
        rows: &[
            &["Prednisone (Cortancyl)", "5 mg", "référence"],
            &["Prednisolone (Solupred)", "5 mg", "x1"],
            &["Méthylprednisolone (Médrol)", "4 mg", "x1,25"],
            &["Hydrocortisone", "20 mg", "x0,25"],
            &["Cortisone", "25 mg", "x0,2"],
            &["Dexaméthasone", "0,75 mg", "x6,7"],
            &["Bétaméthasone (Célestène)", "0,75 mg", "x6,7"],
        ],
    },
    ConvTable {
        short: "Opioïdes",
        title: "Opioïdes — équianalgésie (réf. morphine orale)",
        caution: "Ratios indicatifs de la littérature ; toute rotation impose une titration individuelle et prudente.",
        columns: &["Opioïde", "Conversion", "Exemple ≈ 60 mg morphine orale / j"],
        rows: &[
            &["Codéine (orale)", "÷ 6", "360 mg/j"],
            &["Tramadol (oral)", "÷ 5", "300 mg/j"],
            &["Oxycodone (orale)", "x 2", "30 mg/j"],
            &["Hydromorphone (orale)", "x 7,5", "8 mg/j"],
            &["Morphine SC", "x 2", "30 mg/j"],
            &["Morphine IV", "x 3", "20 mg/j"],
            &["Fentanyl transdermique", "25 µg/h ≈ 60 mg/j", "patch 25 µg/h"],
        ],
    },
    ConvTable {
        short: "Benzodiazépines",
        title: "Benzodiazépines — équivalences (réf. diazépam 10 mg)",
        caution: "Équivalences approximatives (table d'Ashton) utilisées pour la déprescription ; demi-vies très différentes.",
        columns: &["DCI (spécialité)", "Dose ≈ diazépam 10 mg"],
        rows: &[
            &["Diazépam (Valium)", "10 mg — référence"],
            &["Oxazépam (Séresta)", "30 mg"],
            &["Lorazépam (Temesta)", "1 mg"],
            &["Alprazolam (Xanax)", "0,5 mg"],
            &["Bromazépam (Lexomil)", "6 mg"],
            &["Zolpidem (Stilnox)", "20 mg"],
            &["Zopiclone (Imovane)", "15 mg"],
        ],
    },
    ConvTable {
        short: "AOD",
        title: "AOD — posologies et adaptation rénale",
        caution: "Posologies AMM en FA non valvulaire et MTEV ; vérifier la clairance (Cockcroft), le poids, l'âge et les interactions (P-gp, CYP3A4).",
        columns: &["DCI (spécialité)", "FA non valvulaire", "Dose réduite si", "MTEV"],
        rows: &[
            &[
                "Apixaban (Eliquis)",
                "5 mg x2/j",
                "2,5 mg x2/j si 2 critères : âge ≥ 80 ans, poids ≤ 60 kg, créat. ≥ 133 µmol/L",
                "10 mg x2/j 7 j puis 5 mg x2/j",
            ],
            &[
                "Rivaroxaban (Xarelto)",
                "20 mg x1/j au repas",
                "15 mg x1/j si DFG 15 à 49 mL/min",
                "15 mg x2/j 21 j puis 20 mg x1/j",
            ],
            &[
                "Dabigatran (Pradaxa)",
                "150 mg x2/j",
                "110 mg x2/j si ≥ 80 ans, vérapamil, risque hémorragique",
                "150 mg x2/j après héparine",
            ],
            &[
                "Edoxaban (Lixiana)",
                "60 mg x1/j",
                "30 mg x1/j si DFG 15 à 50, poids ≤ 60 kg ou inhibiteur P-gp",
                "60 mg x1/j après héparine",
            ],
            &[
                "Contre-indication rénale",
                "Dabigatran : DFG < 30 — autres AOD : DFG < 15",
                "AVK si DFG effondré ou valve mécanique",
                "—",
            ],
        ],
    },
    ConvTable {
        short: "Cortico. inhalés",
        title: "Corticoïdes inhalés — paliers de dose (adulte)",
        caution: "Doses quotidiennes totales, adulte, d'après les paliers GINA ; elles dépendent du dispositif — se référer au RCP.",
        columns: &["DCI (exemples)", "Faible", "Moyenne", "Forte"],
        rows: &[
            &[
                "Béclométasone HFA (Bécotide, Foster)",
                "100 à 200 µg",
                "> 200 à 400 µg",
                "> 400 µg",
            ],
            &[
                "Budésonide (Pulmicort, Symbicort)",
                "200 à 400 µg",
                "> 400 à 800 µg",
                "> 800 µg",
            ],
            &[
                "Fluticasone propionate (Flixotide, Seretide)",
                "100 à 250 µg",
                "> 250 à 500 µg",
                "> 500 µg",
            ],
            &["Ciclésonide (Alvesco)", "80 à 160 µg", "> 160 à 320 µg", "> 320 µg"],
            &["Mométasone (Asmanex)", "200 µg", "400 µg", "> 400 µg"],
        ],
    },
    ConvTable {
        short: "Insulines",
        title: "Insulines — profils d'action",
        caution: "Délais indicatifs : ils varient avec la dose, le site d'injection et le patient ; la titration reste individuelle.",
        columns: &["Type (spécialités)", "Début", "Pic", "Durée"],
        rows: &[
            &[
                "Analogue rapide (Humalog, NovoRapid, Apidra)",
                "10 à 20 min",
                "1 à 3 h",
                "3 à 5 h",
            ],
            &["Humaine rapide (Actrapid, Umuline)", "30 à 60 min", "2 à 4 h", "6 à 8 h"],
            &["NPH intermédiaire (Insulatard)", "1 à 2 h", "4 à 8 h", "12 à 16 h"],
            &["Glargine U100 (Lantus, Abasaglar)", "2 à 4 h", "sans pic marqué", "20 à 24 h"],
            &["Glargine U300 (Toujeo)", "~ 6 h", "sans pic", "> 24 h"],
            &["Détémir (Levemir)", "1 à 2 h", "peu marqué", "12 à 20 h"],
            &["Dégludec (Tresiba)", "~ 1 h", "sans pic", "> 42 h"],
        ],
    },
    ConvTable {
        short: "Fonction rénale",
        title: "Fonction rénale — calcul et stades",
        caution: "L'adaptation des doses des AOD, HBPM et metformine se fait sur la clairance de Cockcroft ; les stades servent au suivi néphrologique.",
        columns: &["Repère", "Valeur"],
        rows: &[
            &[
                "Cockcroft & Gault (mL/min)",
                "(140 − âge) x poids (kg) x k / créatinine (µmol/L) — k = 1,23 homme, 1,04 femme",
            ],
            &["Stade G1 — normal", "DFG ≥ 90 mL/min/1,73 m²"],
            &["Stade G2 — légère", "60 à 89"],
            &["Stade G3a — modérée", "45 à 59"],
            &["Stade G3b — modérée à sévère", "30 à 44"],
            &["Stade G4 — sévère", "15 à 29"],
            &["Stade G5 — terminale", "< 15 ou dialyse"],
            &[
                "Metformine",
                "Pleine dose si DFG ≥ 60 ; demi-dose 30 à 59 ; arrêt si < 30 et à l'iode / déshydratation",
            ],
        ],
    },
    ConvTable {
        short: "Angine",
        title: "Angine — score de Mac Isaac et TROD",
        caution: "Le score oriente le TROD chez l'adulte ; chez l'enfant de plus de 3 ans le TROD est réalisé d'emblée. Antibiotique seulement si TROD positif.",
        columns: &["Critère", "Points"],
        rows: &[
            &["Fièvre > 38 °C", "+1"],
            &["Absence de toux", "+1"],
            &["Adénopathies cervicales sensibles", "+1"],
            &["Atteinte amygdalienne (exsudat ou tuméfaction)", "+1"],
            &["Âge 3 à 14 ans", "+1"],
            &["Âge 15 à 44 ans", "0"],
            &["Âge ≥ 45 ans", "−1"],
            &["Conduite à tenir (adulte)", "TROD si score ≥ 2 ; score < 2 : pas de TROD ni d'antibiotique"],
            &["Si TROD positif", "Amoxicilline 6 jours (allergie : céphalosporine ou macrolide)"],
        ],
    },
    ConvTable {
        short: "Cystite",
        title: "Cystite simple — traitements de première intention",
        caution: "Femme non enceinte, sans facteur de risque de complication ; fièvre, douleur lombaire ou grossesse imposent un avis médical.",
        columns: &["Rang", "Traitement", "Durée"],
        rows: &[
            &["1re intention", "Fosfomycine trométamol (Monuril) 3 g", "dose unique"],
            &["2e intention", "Pivmécillinam (Selexid) 400 mg x2/j", "3 à 5 jours"],
            &["3e intention", "Nitrofurantoïne (Furadantine) 100 mg x3/j", "5 jours"],
            &[
                "À éviter",
                "Fluoroquinolones et cotrimoxazole en probabiliste (résistances, effets indésirables)",
                "—",
            ],
            &[
                "Conseils",
                "Boissons abondantes, mictions non retenues, réévaluation à 72 h",
                "—",
            ],
        ],
    },
    ConvTable {
        short: "Contraception",
        title: "Contraception — conduite à tenir en cas d'oubli",
        caution: "Repères usuels ; la contraception d'urgence est d'autant plus efficace qu'elle est prise tôt (lévonorgestrel ≤ 72 h, ulipristal ≤ 120 h).",
        columns: &["Situation", "Conduite à tenir"],
        rows: &[
            &[
                "Œstroprogestatif, oubli < 12 h",
                "Prendre le comprimé oublié immédiatement, poursuivre à l'heure habituelle",
            ],
            &[
                "Œstroprogestatif, oubli > 12 h",
                "Prendre le dernier oubli, poursuivre, préservatif 7 jours ; contraception d'urgence si rapport dans les 5 jours",
            ],
            &[
                "Oubli en 3e semaine",
                "Enchaîner la plaquette suivante sans intervalle libre",
            ],
            &[
                "Microprogestatif désogestrel (Optimizette)",
                "Retard toléré 12 h ; au-delà, préservatif 7 jours",
            ],
            &[
                "Microprogestatif lévonorgestrel (Microval)",
                "Retard toléré 3 h seulement ; au-delà, préservatif 7 jours",
            ],
            &[
                "Vomissements ou diarrhée < 4 h après la prise",
                "Considérer le comprimé comme oublié",
            ],
        ],
    },
    ConvTable {
        short: "Antalgiques",
        title: "Antalgiques non opioïdes — doses adulte usuelles",
        caution: "Doses maximales adulte à fonction rénale et hépatique normales ; réduire chez le sujet âgé, de faible poids ou insuffisant rénal.",
        columns: &["Molécule", "Dose usuelle", "Maximum / 24 h"],
        rows: &[
            &["Paracétamol", "500 mg à 1 g toutes les 6 h", "3 g (4 g sur avis, jamais si < 50 kg)"],
            &["Paracétamol enfant", "15 mg/kg toutes les 6 h", "60 mg/kg"],
            &["Ibuprofène", "200 à 400 mg x3/j", "1 200 mg en automédication"],
            &["Kétoprofène LP", "100 mg x1 à 2/j", "200 mg"],
            &["Diclofénac", "50 mg x2 à 3/j", "150 mg"],
            &["Naproxène", "250 à 500 mg x2/j", "1 100 mg"],
            &["Néfopam (Acupan)", "20 mg x4 à 6/j", "120 mg"],
        ],
    },
    ConvTable {
        short: "Vaccins",
        title: "Vaccination à l'officine — rappels adultes",
        caution: "Repères du calendrier vaccinal, révisé chaque année : vérifier la version en vigueur et le périmètre de vaccination du pharmacien.",
        columns: &["Vaccin", "Rythme adulte"],
        rows: &[
            &[
                "dTP (diphtérie, tétanos, poliomyélite)",
                "25, 45, 65 ans puis tous les 10 ans",
            ],
            &[
                "Coqueluche (dTcaP)",
                "Rappel à 25 ans ; stratégie du cocooning autour du nourrisson",
            ],
            &["Grippe saisonnière", "Chaque automne : ≥ 65 ans, à risque, entourage"],
            &["COVID-19", "Rappel selon les recommandations en vigueur"],
            &["Pneumocoque", "Schéma VPC puis VPP23 chez l'immunodéprimé ou à risque"],
            &["Zona (Shingrix)", "2 doses, à partir de 65 ans"],
            &["HPV", "Rattrapage jusqu'à 19 ans (26 ans HSH)"],
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_well_formed() {
        assert!(TABLES.len() >= 6);
        for t in TABLES {
            assert!(!t.title.is_empty());
            assert!(!t.caution.is_empty());
            assert!(!t.rows.is_empty());
            for row in t.rows {
                assert_eq!(
                    row.len(),
                    t.columns.len(),
                    "row width mismatch in « {} »",
                    t.title
                );
            }
        }
        // The families the acts lean on are present.
        for prefix in [
            "IPP",
            "HBPM",
            "AOD",
            "Insulines",
            "Corticoïdes inhalés",
            "Angine",
            "Cystite",
            "Contraception",
            "Vaccination",
            "Fonction rénale",
        ] {
            assert!(
                TABLES.iter().any(|t| t.title.starts_with(prefix)),
                "table « {prefix} » manquante"
            );
        }
        // Short names label the selector buttons: they must be unique.
        let mut shorts: Vec<&str> = TABLES.iter().map(|t| t.short).collect();
        shorts.sort_unstable();
        let n = shorts.len();
        shorts.dedup();
        assert_eq!(shorts.len(), n);
    }
}
