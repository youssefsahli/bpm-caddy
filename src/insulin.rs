//! Les insulines : ce que fait chacune au cours de la journée, et les
//! trois règles qui donnent une dose.
//!
//! Deux choses que le comptoir ne peut pas lire sur une boîte. La
//! première est la **forme** : une glargine et une NPH portent toutes
//! deux le mot « lente », et l'une a un pic à six heures quand l'autre
//! n'en a pas. C'est ce pic qui fait l'hypoglycémie de fin d'après-midi,
//! et c'est un dessin qui l'explique, pas une phrase. La seconde est
//! l'**arithmétique** de l'adaptation : règle des 500 pour les glucides,
//! règle des 1800 pour la correction, titration de la basale.
//!
//! Pur et testé, sans horloge : le module donne une courbe et des
//! nombres, l'application les dessine. Rien ici ne décide à la place du
//! prescripteur — les règles sont celles des schémas fonctionnels, et
//! elles se lisent contre l'ordonnance, jamais à sa place.

/// How an insulin behaves, in the only two shapes that matter at the
/// counter: one with a peak, one without.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// Rises to a peak, then falls: rapides, humaine, NPH, détémir.
    Peaked,
    /// Rises to a plateau and holds it: glargine, dégludec. There is no
    /// hour of the day when it hits harder — which is the whole point.
    Flat,
    /// Two components in one pen: a rapid peak, then an NPH one.
    Biphasic,
}

/// One insulin's action profile, in minutes.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// The name the box carries, and the DCI under it.
    pub name: &'static str,
    pub dci: &'static str,
    /// "Ultra-rapide", "Lente"… — how the ordonnance names the family.
    pub family: &'static str,
    pub shape: Shape,
    /// When it starts to work.
    pub onset_min: u32,
    /// Where the peak sits, for a peaked insulin. `None` for a flat one.
    pub peak_min: Option<(u32, u32)>,
    /// How long it lasts, in minutes.
    pub duration_min: u32,
    /// What the counter says about it.
    pub note: &'static str,
}

/// The insulins of the French market, in the order a schema uses them.
///
/// The figures are the usual ones of the RCPs; they move with the dose,
/// the site and the person, and the profile is a shape to reason with,
/// never a promise about tonight.
pub const PROFILES: &[Profile] = &[
    Profile {
        name: "Fiasp",
        dci: "insuline asparte (accélérée)",
        family: "Ultra-rapide",
        shape: Shape::Peaked,
        onset_min: 5,
        peak_min: Some((30, 60)),
        duration_min: 240,
        note: "Se pique au début du repas, ou jusqu'à vingt minutes après : c'est la seule qui rattrape une assiette déjà commencée.",
    },
    Profile {
        name: "NovoRapid",
        dci: "insuline asparte",
        family: "Rapide",
        shape: Shape::Peaked,
        onset_min: 15,
        peak_min: Some((60, 180)),
        duration_min: 300,
        note: "Juste avant le repas. Sauter le repas après l'avoir piquée, c'est l'hypoglycémie une heure plus tard.",
    },
    Profile {
        name: "Humalog",
        dci: "insuline lispro",
        family: "Rapide",
        shape: Shape::Peaked,
        onset_min: 15,
        peak_min: Some((60, 180)),
        duration_min: 300,
        note: "Même profil que l'asparte : le choix se fait sur le stylo et l'habitude, pas sur la cinétique.",
    },
    Profile {
        name: "Apidra",
        dci: "insuline glulisine",
        family: "Rapide",
        shape: Shape::Peaked,
        onset_min: 15,
        peak_min: Some((60, 180)),
        duration_min: 300,
        note: "Rapide comme les deux autres ; à conserver au réfrigérateur avant ouverture, un mois à température ambiante après.",
    },
    Profile {
        name: "Actrapid",
        dci: "insuline humaine",
        family: "Humaine rapide",
        shape: Shape::Peaked,
        onset_min: 30,
        peak_min: Some((120, 240)),
        duration_min: 480,
        note: "Trente minutes avant le repas, pas au moment de manger : c'est l'erreur qui fait monter la glycémie puis la fait tomber trop tard.",
    },
    Profile {
        name: "Insulatard",
        dci: "insuline NPH",
        family: "Intermédiaire",
        shape: Shape::Peaked,
        onset_min: 90,
        peak_min: Some((240, 480)),
        duration_min: 960,
        note: "Suspension trouble : elle se remet en suspension par dix retournements lents avant chaque injection, sinon la dose n'est pas celle qu'on croit. Son pic de fin d'après-midi est la cause classique de l'hypoglycémie de 17 h.",
    },
    Profile {
        name: "Levemir",
        dci: "insuline détémir",
        family: "Lente",
        shape: Shape::Peaked,
        onset_min: 90,
        peak_min: Some((360, 480)),
        duration_min: 1080,
        note: "Un pic discret et une durée qui dépend de la dose : à faible dose elle ne couvre pas 24 heures, d'où les deux injections fréquentes.",
    },
    Profile {
        name: "Lantus",
        dci: "insuline glargine U100",
        family: "Lente",
        shape: Shape::Flat,
        onset_min: 120,
        peak_min: None,
        duration_min: 1440,
        note: "Sans pic, à heure fixe, tous les jours la même. Limpide : elle ne se remet jamais en suspension, et elle ne se mélange à aucune autre insuline dans la seringue.",
    },
    Profile {
        name: "Abasaglar",
        dci: "insuline glargine U100",
        family: "Lente",
        shape: Shape::Flat,
        onset_min: 120,
        peak_min: None,
        duration_min: 1440,
        note: "Biosimilaire de la glargine U100 : même profil, même dose. Le changement de marque se signale au patient — le stylo n'a pas le même aspect.",
    },
    Profile {
        name: "Toujeo",
        dci: "insuline glargine U300",
        family: "Ultralente",
        shape: Shape::Flat,
        onset_min: 360,
        peak_min: None,
        duration_min: 2160,
        note: "Trois cents unités par millilitre : le stylo affiche des unités, pas des millilitres, et une dose de Toujeo ne se transvase jamais dans une seringue à insuline. Le plateau met deux à quatre jours à s'installer après un changement de dose.",
    },
    Profile {
        name: "Tresiba",
        dci: "insuline dégludec",
        family: "Ultralente",
        shape: Shape::Flat,
        onset_min: 60,
        peak_min: None,
        duration_min: 2520,
        note: "Plus de quarante-deux heures : c'est celle qui pardonne un horaire décalé, à condition de garder huit heures entre deux injections. L'équilibre s'établit en trois jours.",
    },
    Profile {
        name: "NovoMix 30",
        dci: "asparte 30 % / asparte protaminée 70 %",
        family: "Prémélangée",
        shape: Shape::Biphasic,
        onset_min: 15,
        peak_min: Some((60, 240)),
        duration_min: 960,
        note: "Deux insulines dans un stylo : un pic de repas et un plateau derrière. Elle se remet en suspension avant chaque injection, et elle impose des repas à heures fixes — c'est son prix.",
    },
];

/// Find a profile by the name on the box, case- and accent-insensitively.
pub fn find(name: &str) -> Option<&'static Profile> {
    let key = crate::fuzzy::sort_key(name);
    PROFILES
        .iter()
        .find(|p| crate::fuzzy::sort_key(p.name) == key)
}

/// The profile that matches a drug card, by its name or its DCI. This is
/// what puts the right curve on the right fiche without a second table
/// mapping one to the other.
pub fn for_card(name: &str, dci: &str) -> Option<&'static Profile> {
    if let Some(p) = find(name) {
        return Some(p);
    }
    let dci_key = crate::fuzzy::sort_key(dci);
    if dci_key.is_empty() {
        return None;
    }
    PROFILES
        .iter()
        .find(|p| crate::fuzzy::sort_key(p.dci) == dci_key)
}

/// Relative activity at `minutes` after the injection, from 0 to 1,
/// normalised so that each curve peaks at 1.
///
/// Peak-normalised and not area-normalised on purpose: the question the
/// drawing answers is *when* an insulin works, not how much total insulin
/// is on board. Two curves of the same height are directly comparable in
/// time, which is what makes the NPH's afternoon peak visible beside the
/// glargine's flat line.
pub fn activity(p: &Profile, minutes: f64) -> f64 {
    let onset = p.onset_min as f64;
    let end = p.duration_min as f64;
    if minutes <= onset || minutes >= end {
        return 0.0;
    }
    match p.shape {
        Shape::Flat => {
            // Up over two hours, hold, then down over the last three:
            // a plateau with shoulders, which is what a peakless
            // insulin's curve actually looks like.
            let rise = (onset + 120.0).min(onset + (end - onset) * 0.25);
            let fall = (end - 180.0).max(rise);
            if minutes < rise {
                smoothstep((minutes - onset) / (rise - onset))
            } else if minutes <= fall {
                1.0
            } else {
                smoothstep(1.0 - (minutes - fall) / (end - fall))
            }
        }
        Shape::Peaked => {
            let peak = peak_centre(p);
            if minutes <= peak {
                smoothstep((minutes - onset) / (peak - onset))
            } else {
                smoothstep(1.0 - (minutes - peak) / (end - peak))
            }
        }
        Shape::Biphasic => {
            // The rapid component, then the NPH one behind it. The sum
            // is capped at 1: the point is the two humps, not a total.
            let rapid = Profile {
                shape: Shape::Peaked,
                onset_min: p.onset_min,
                peak_min: Some((60, 120)),
                duration_min: 300,
                ..*p
            };
            let slow = Profile {
                shape: Shape::Peaked,
                onset_min: 90,
                peak_min: Some((240, 480)),
                duration_min: p.duration_min,
                ..*p
            };
            (activity(&rapid, minutes) * 0.85 + activity(&slow, minutes) * 0.7).min(1.0)
        }
    }
}

/// The middle of the peak window, in minutes. A flat insulin has none,
/// and the caller must not draw a marker for it.
pub fn peak_centre(p: &Profile) -> f64 {
    match p.peak_min {
        Some((a, b)) => (a as f64 + b as f64) / 2.0,
        None => (p.onset_min as f64 + p.duration_min as f64) / 2.0,
    }
}

/// A smooth 0→1 ramp, clamped. Cheaper than a gamma curve and honest
/// about being a shape rather than a pharmacokinetic model.
fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The three numbers a functional schema is adjusted with, from the
/// total daily dose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    /// Grams of carbohydrate covered by one unit — the « règle des 500 »
    /// (450 for human insulin, which is slower and covers less).
    pub carb_ratio: f64,
    /// How far one unit lowers the blood glucose, in g/L — the « règle
    /// des 1800 », converted to the unit a French lab reports.
    ///
    /// The conversion is the trap: 1800/DTQ is in **mg/dL**, and
    /// 100 mg/dL is 1 g/L, so the figure in g/L is `18/DTQ` and not
    /// `1,8/DTQ`. A factor of ten here is a correction dose ten times
    /// too large.
    pub sensitivity_gl: f64,
    /// The same, in mmol/L — the « règle des 100 ».
    pub sensitivity_mmol: f64,
}

/// The rules for a total daily dose. Returns `None` at or below zero:
/// there is no ratio to compute from no insulin, and a division that
/// quietly returns infinity is worse than a blank.
pub fn rules(total_daily_units: f64, human_insulin: bool) -> Option<Rules> {
    if total_daily_units <= 0.0 {
        return None;
    }
    let numerator = if human_insulin { 450.0 } else { 500.0 };
    Some(Rules {
        carb_ratio: numerator / total_daily_units,
        sensitivity_gl: 18.0 / total_daily_units,
        sensitivity_mmol: 100.0 / total_daily_units,
    })
}

/// The correction dose: how far above target, divided by what one unit
/// moves. Negative when the reading is already under target — and that
/// is a number the caller must show as « rien à corriger », never as a
/// dose to subtract from the meal bolus without a prescriber saying so.
pub fn correction_units(measured_gl: f64, target_gl: f64, sensitivity_gl: f64) -> Option<f64> {
    if sensitivity_gl <= 0.0 {
        return None;
    }
    Some((measured_gl - target_gl) / sensitivity_gl)
}

/// The meal bolus: grams of carbohydrate divided by the ratio.
pub fn meal_units(carbs_g: f64, carb_ratio: f64) -> Option<f64> {
    if carb_ratio <= 0.0 {
        return None;
    }
    Some(carbs_g / carb_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curve_starts_at_the_onset_and_ends_at_the_duration() {
        for p in PROFILES {
            assert_eq!(activity(p, 0.0), 0.0, "{} avant l'injection", p.name);
            assert_eq!(
                activity(p, p.onset_min as f64),
                0.0,
                "{} au début d'action",
                p.name
            );
            assert_eq!(
                activity(p, p.duration_min as f64),
                0.0,
                "{} à la fin",
                p.name
            );
            assert_eq!(
                activity(p, p.duration_min as f64 + 60.0),
                0.0,
                "{} après la fin",
                p.name
            );
            // And it is a fraction all the way through.
            for step in 0..=48 {
                let t = p.duration_min as f64 * step as f64 / 48.0;
                let a = activity(p, t);
                assert!((0.0..=1.0).contains(&a), "{} à {t} min : {a}", p.name);
            }
        }
    }

    /// The whole reason for the drawing: a peaked insulin has an hour
    /// when it hits hardest, and a flat one does not.
    #[test]
    fn a_peaked_insulin_peaks_and_a_flat_one_does_not() {
        let nph = find("Insulatard").unwrap();
        let at_peak = activity(nph, peak_centre(nph));
        assert!(at_peak > 0.95, "la NPH doit culminer à son pic : {at_peak}");
        // Two hours after the injection it is still climbing; at its
        // peak it is far above that.
        assert!(activity(nph, 120.0) < at_peak * 0.75);

        let glargine = find("Lantus").unwrap();
        // Across the whole plateau it stays flat — no hour of the day
        // when it hits harder.
        let samples: Vec<f64> = (6..=18)
            .map(|h| activity(glargine, h as f64 * 60.0))
            .collect();
        let lo = samples.iter().cloned().fold(f64::MAX, f64::min);
        let hi = samples.iter().cloned().fold(f64::MIN, f64::max);
        assert!(hi - lo < 0.05, "la glargine ne doit pas piquer : {lo}–{hi}");
        assert!(lo > 0.9);
    }

    /// A premix is two insulins in one pen, and the drawing has to show
    /// both humps or it is telling the patient something false.
    #[test]
    fn a_premix_shows_two_humps() {
        let mix = find("NovoMix 30").unwrap();
        let early = activity(mix, 90.0);
        let dip = activity(mix, 300.0);
        let late = activity(mix, 400.0);
        assert!(early > 0.5, "le pic de repas manque : {early}");
        assert!(dip < early, "pas de creux entre les deux : {dip}");
        assert!(late > dip, "la seconde bosse manque : {late} vs {dip}");
    }

    #[test]
    fn the_ultralentes_outlast_the_day() {
        for name in ["Toujeo", "Tresiba"] {
            let p = find(name).unwrap();
            assert!(
                activity(p, 24.0 * 60.0) > 0.0,
                "{name} doit encore agir à 24 h"
            );
        }
        // And a rapid one is long gone.
        assert_eq!(activity(find("NovoRapid").unwrap(), 24.0 * 60.0), 0.0);
    }

    #[test]
    fn the_rules_come_out_of_the_total_daily_dose() {
        let r = rules(50.0, false).unwrap();
        assert!((r.carb_ratio - 10.0).abs() < 1e-9);
        // 1800/50 = 36 mg/dL, which is 0,36 g/L — not 0,036.
        assert!(
            (r.sensitivity_gl - 0.36).abs() < 1e-9,
            "{}",
            r.sensitivity_gl
        );
        assert!((r.sensitivity_mmol - 2.0).abs() < 1e-9);
        // And the two units must agree: 1 mmol/L of glucose is
        // 0,18 g/L. A conversion that drifts is how a dose goes wrong.
        assert!(
            (r.sensitivity_gl - r.sensitivity_mmol * 0.18).abs() < 1e-9,
            "g/L et mmol/L ne disent pas la même chose"
        );
        // Human insulin covers less per unit.
        let h = rules(50.0, true).unwrap();
        assert!(h.carb_ratio < r.carb_ratio);
        // No insulin, no ratio — never an infinity dressed up as a dose.
        assert!(rules(0.0, false).is_none());
        assert!(rules(-3.0, false).is_none());
    }

    /// The two sensitivity figures are the same fact in two units, and
    /// they must stay so at every dose: this is the check that would
    /// have caught the g/L conversion being ten times too small.
    #[test]
    fn the_two_sensitivity_units_never_drift() {
        for dtq in [10.0, 24.0, 37.5, 50.0, 80.0, 120.0] {
            let r = rules(dtq, false).unwrap();
            assert!(
                (r.sensitivity_gl - r.sensitivity_mmol * 0.18).abs() < 1e-9,
                "DTQ {dtq} : {} g/L contre {} mmol/L",
                r.sensitivity_gl,
                r.sensitivity_mmol
            );
            // And it lands in the range a sensitivity factor actually
            // has: a tenth of a g/L to a bit over one.
            assert!(
                (0.1..=2.0).contains(&r.sensitivity_gl),
                "DTQ {dtq} : {} g/L est hors de toute réalité",
                r.sensitivity_gl
            );
        }
    }

    #[test]
    fn the_correction_and_the_meal_bolus_are_plain_arithmetic() {
        let r = rules(50.0, false).unwrap();
        // 2,40 g/L for a target of 1,20 with 0,36 g/L per unit.
        let units = correction_units(2.4, 1.2, r.sensitivity_gl).unwrap();
        assert!((units - 3.3333).abs() < 0.001, "{units}");
        // Already under target: the number is negative, and the caller
        // has to say « rien à corriger » rather than subtract it.
        assert!(correction_units(0.9, 1.2, r.sensitivity_gl).unwrap() < 0.0);
        assert!(correction_units(2.4, 1.2, 0.0).is_none());
        // 60 g of carbohydrate at 1 unit for 10 g.
        assert!((meal_units(60.0, r.carb_ratio).unwrap() - 6.0).abs() < 1e-9);
        assert!(meal_units(60.0, 0.0).is_none());
    }

    #[test]
    fn a_profile_is_found_by_its_box_or_by_its_dci() {
        assert_eq!(find("lantus").unwrap().name, "Lantus");
        assert_eq!(find("LANTUS").unwrap().name, "Lantus");
        assert!(find("Doliprane").is_none());
        // A card whose name is not in the table is matched on its DCI:
        // that is how a generic lands on the right curve.
        assert_eq!(
            for_card("Insuline glargine Biogaran", "insuline glargine U100")
                .unwrap()
                .name,
            "Lantus"
        );
        assert!(for_card("Doliprane", "paracétamol").is_none());
        assert!(for_card("Inconnu", "").is_none());
    }

    #[test]
    fn every_profile_is_coherent_and_says_something() {
        for p in PROFILES {
            assert!(p.onset_min < p.duration_min, "{} : durée < début", p.name);
            assert!(!p.note.trim().is_empty(), "{} sans commentaire", p.name);
            assert!(!p.family.trim().is_empty());
            assert!(!p.dci.trim().is_empty());
            match (p.shape, p.peak_min) {
                (Shape::Flat, Some(_)) => panic!("{} : sans pic mais avec un pic", p.name),
                (Shape::Peaked | Shape::Biphasic, None) => {
                    panic!("{} : à pic mais sans pic", p.name)
                }
                _ => {}
            }
            if let Some((a, b)) = p.peak_min {
                assert!(a <= b, "{} : fenêtre de pic à l'envers", p.name);
                assert!(a > p.onset_min, "{} : pic avant le début", p.name);
                assert!(b < p.duration_min, "{} : pic après la fin", p.name);
            }
        }
        // No two entries under the same name, or `find` would be a lie.
        let mut names: Vec<&str> = PROFILES.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let seen = names.len();
        names.dedup();
        assert_eq!(seen, names.len());
    }

    /// Every profile must reach a card of the shipped base, or it is a
    /// curve nobody will ever see.
    #[test]
    fn every_profile_reaches_a_card_or_is_a_named_addition() {
        // These are on the French market but not (yet) in the starter
        // base; they are named so that adding the card is a decision.
        const NOT_IN_BASE: &[&str] = &["Fiasp", "Actrapid", "NovoMix 30"];
        for p in PROFILES {
            if NOT_IN_BASE.contains(&p.name) {
                continue;
            }
            let reached = crate::db::STARTER_DRUGS
                .iter()
                .any(|(name, dci, _, _)| for_card(name, dci).is_some_and(|f| f.name == p.name));
            assert!(reached, "profil sans fiche : {}", p.name);
        }
    }
}
