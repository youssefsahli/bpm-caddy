//! Conversion / equivalence tables for the counter (IPP, HBPM,
//! statines, corticoïdes, opioïdes, benzodiazépines).
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
        // The two asked-for families are present.
        assert!(TABLES.iter().any(|t| t.title.starts_with("IPP")));
        assert!(TABLES.iter().any(|t| t.title.starts_with("HBPM")));
    }
}
