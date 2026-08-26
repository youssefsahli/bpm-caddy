//! Reading a codex formula, and the arithmetic that goes with it.
//!
//! A formula is written the way it is on the fiche de fabrication —
//! « Acide salicylique | 5 g », one ingredient per line, and a
//! « qsp 100 g » for the excipient. What the counter actually needs is
//! rarely the quantity the formula is written for: the prescription
//! says 60 g, the pot holds 30. Everything here is pure and tested, so
//! the rule of three is done once, right, and not in the margin of the
//! ordonnance.

/// One ingredient of a formula, as parsed off its line.
#[derive(Clone, Debug, PartialEq)]
pub struct FormulaLine {
    pub name: String,
    /// The quantity as written: "5 g", "qsp 100 g", or empty when the
    /// line carries no number at all.
    pub written: String,
    /// The number, when there is one.
    pub quantity: Option<f64>,
    /// The unit that followed it ("g", "mL", "%"…).
    pub unit: String,
    /// « qsp » — quantité suffisante pour: this line completes the
    /// preparation up to the total, it is not added to it.
    pub qsp: bool,
}

impl FormulaLine {
    /// The quantity this line becomes when the whole formula is made at
    /// `factor` times its written size. A line without a number keeps
    /// what it says: « une pointe de spatule » does not scale.
    pub fn scaled(&self, factor: f64) -> String {
        match self.quantity {
            Some(q) => {
                let value = format_quantity(q * factor);
                match (self.qsp, self.unit.is_empty()) {
                    (true, true) => format!("qsp {value}"),
                    (true, false) => format!("qsp {value} {}", self.unit),
                    (false, true) => value,
                    (false, false) => format!("{value} {}", self.unit),
                }
            }
            None => self.written.clone(),
        }
    }
}

/// Parse a formula, one ingredient per line. Empty lines are dropped;
/// a line without a `|` is kept as an ingredient with no quantity, so
/// nothing written by the team ever disappears from the sheet.
pub fn parse_formula(text: &str) -> Vec<FormulaLine> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (name, written) = match line.split_once('|') {
                Some((n, q)) => (n.trim(), q.trim()),
                None => (line.trim(), ""),
            };
            let qsp = starts_with_qsp(written);
            let rest = if qsp {
                written
                    .trim_start_matches(|c: char| c.is_alphabetic() || c == '.')
                    .trim()
            } else {
                written
            };
            let (quantity, unit) = match parse_amount(rest) {
                Some((q, u)) => (Some(q), u.to_owned()),
                None => (None, String::new()),
            };
            FormulaLine {
                name: name.to_owned(),
                written: written.to_owned(),
                quantity,
                unit,
                qsp,
            }
        })
        .collect()
}

/// Does this quantity open with a « qsp » (quantité suffisante pour),
/// however it is spelled at the counter?
fn starts_with_qsp(text: &str) -> bool {
    let head: String = text
        .chars()
        .take_while(|c| c.is_alphabetic() || *c == '.')
        .collect::<String>()
        .to_lowercase();
    matches!(head.as_str(), "qsp" | "q.s.p" | "qs")
}

/// Read "100 g", "0,5 mL", "12.5 %" — the number, then whatever unit
/// followed it. The French decimal comma is accepted, since that is how
/// a formula is written here.
pub fn parse_amount(text: &str) -> Option<(f64, &str)> {
    let text = text.trim();
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if c.is_ascii_digit() || ((c == ',' || c == '.') && i > 0) {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let value: f64 = text[..end].replace(',', ".").parse().ok()?;
    Some((value, text[end..].trim()))
}

/// A quantity as a pharmacist writes it: French decimal comma, no
/// trailing zeros, and three decimals at most — below that the balance
/// is the limit, not the arithmetic.
pub fn format_quantity(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut text = format!("{rounded:.3}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text.replace('.', ",")
}

/// The factor to apply to a formula written for `yield_amount` in order
/// to make `target`. `None` when either side cannot be read, or when
/// the units disagree — a formula for 100 g does not scale to 60 mL,
/// and guessing there would be worse than saying nothing.
pub fn scale_factor(yield_amount: &str, target: &str) -> Option<f64> {
    let (base, base_unit) = parse_amount(yield_amount)?;
    let (want, want_unit) = parse_amount(target)?;
    if base <= 0.0 || want <= 0.0 {
        return None;
    }
    if !base_unit.eq_ignore_ascii_case(want_unit) {
        return None;
    }
    Some(want / base)
}

/// The mass of active ingredient in `total` of a preparation titrated
/// at `percent` — the m/m percentage a dermatological formula is
/// written in.
pub fn mass_for_percent(percent: f64, total: f64) -> f64 {
    percent / 100.0 * total
}

/// The percentage `mass` represents in `total`.
pub fn percent_for_mass(mass: f64, total: f64) -> Option<f64> {
    if total <= 0.0 {
        return None;
    }
    Some(mass / total * 100.0)
}

/// C1·V1 = C2·V2: how much of the concentrated solution to take in
/// order to obtain `volume` at `wanted`. `None` when the dilution is
/// impossible — one cannot dilute upwards.
pub fn dilution_take(strong: f64, wanted: f64, volume: f64) -> Option<f64> {
    if strong <= 0.0 || wanted <= 0.0 || volume <= 0.0 || wanted > strong {
        return None;
    }
    Some(wanted * volume / strong)
}

/// The apparent volume of an empty gelatin capsule, by size — what
/// decides how much excipient a batch of capsules takes.
pub const CAPSULE_VOLUMES: &[(&str, f64)] = &[
    ("0", 0.68),
    ("1", 0.50),
    ("2", 0.37),
    ("3", 0.30),
    ("4", 0.21),
];

/// The active ingredient to weigh for a batch of capsules: the unit
/// dose, the number of capsules, and the overage the officine adds for
/// what stays in the mortar and the gélulier.
pub fn capsule_batch_mass(dose_mg: f64, count: f64, overage_percent: f64) -> f64 {
    dose_mg * count * (1.0 + overage_percent / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_formula_line_is_read_as_written() {
        let lines = parse_formula(
            "Acide salicylique | 5 g\nVaseline blanche | qsp 100 g\nEssence | une goutte\n\n",
        );
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].name, "Acide salicylique");
        assert_eq!(lines[0].quantity, Some(5.0));
        assert_eq!(lines[0].unit, "g");
        assert!(!lines[0].qsp);
        // The excipient completes the preparation, it is not added.
        assert!(lines[1].qsp);
        assert_eq!(lines[1].quantity, Some(100.0));
        // A line with no number keeps what it says.
        assert_eq!(lines[2].quantity, None);
        assert_eq!(lines[2].scaled(0.5), "une goutte");
    }

    #[test]
    fn the_rule_of_three_is_done_once_and_right() {
        // 60 g of a formula written for 100 g.
        let factor = scale_factor("100 g", "60 g").unwrap();
        let lines = parse_formula("Acide salicylique | 5 g\nVaseline blanche | qsp 100 g");
        assert_eq!(lines[0].scaled(factor), "3 g");
        assert_eq!(lines[1].scaled(factor), "qsp 60 g");
        // The balance stops at the milligram, and so does the number.
        let small = scale_factor("100 g", "33 g").unwrap();
        assert_eq!(lines[0].scaled(small), "1,65 g");
        let tiny = scale_factor("100 g", "12,5 g").unwrap();
        assert_eq!(lines[0].scaled(tiny), "0,625 g");
        // Units that disagree are not scaled at all.
        assert_eq!(scale_factor("100 g", "60 mL"), None);
        assert_eq!(scale_factor("", "60 g"), None);
        assert_eq!(scale_factor("100 g", "0 g"), None);
    }

    #[test]
    fn the_titre_and_the_dilution() {
        assert_eq!(mass_for_percent(5.0, 60.0), 3.0);
        assert_eq!(percent_for_mass(3.0, 60.0), Some(5.0));
        assert_eq!(percent_for_mass(3.0, 0.0), None);
        // 250 mL at 0,05 % from a 0,5 % solution: 25 mL, and the rest
        // is water.
        assert_eq!(dilution_take(0.5, 0.05, 250.0), Some(25.0));
        // One does not dilute upwards.
        assert_eq!(dilution_take(0.05, 0.5, 250.0), None);
    }

    #[test]
    fn a_batch_of_capsules_carries_its_overage() {
        // 30 capsules of 12,5 mg, plus 10 % for what stays behind.
        assert!((capsule_batch_mass(12.5, 30.0, 10.0) - 412.5).abs() < 1e-6);
        assert_eq!(capsule_batch_mass(12.5, 30.0, 0.0), 375.0);
    }

    #[test]
    fn quantities_are_written_the_french_way() {
        assert_eq!(format_quantity(3.0), "3");
        assert_eq!(format_quantity(1.6666), "1,667");
        assert_eq!(format_quantity(0.5), "0,5");
        assert_eq!(format_quantity(100.0), "100");
    }
}
