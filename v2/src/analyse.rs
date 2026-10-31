//! One reading of a list of treatments against every clinical table.
//!
//! The first window spreads these over six views (rein, foie, grossesse,
//! âge, écrasement, CYP) plus the review. Here they are one list, sorted
//! by how much each line asks of the pharmacist: what stops the
//! dispensing first, what only says « à vérifier » last. Each line keeps
//! its table, its source and the table's own words — this module adds no
//! clinical sentence of its own, only the order and the grouping.
//!
//! Pure: no egui, no clock. The patient's figures come in a [`Context`].

use bpm_caddy::content::Overrides;
use bpm_caddy::db::Drug;
use bpm_caddy::revue::Treatment;
use bpm_caddy::{crush, cyp, elderly, gravidity, hepatic, renal, revue};

/// What the patient's file says, as far as the tables need it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// DFG or clairance, mL/min.
    pub dfg: Option<f64>,
    /// The lab's bound on it (« < », « > »…), or empty.
    pub dfg_bound: String,
    /// The file's biology, for the readings that concern a treatment.
    pub bio: Vec<bpm_caddy::db::BioResult>,
    pub liver: Option<hepatic::Stage>,
    pub age: Option<u32>,
    pub pregnant: bool,
    pub breastfeeding: bool,
    /// The patient cannot swallow whole tablets: read the crushing table.
    pub crush: bool,
}

/// How much a line asks for. Declared from the most to the least urgent,
/// so sorting ascending reads in the right order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gravity {
    /// Contre-indiqué, ne pas écraser, alerte.
    Stop,
    /// Dose à adapter, à éviter.
    Adapt,
    /// À surveiller, sous condition.
    Watch,
    /// The table cannot conclude without a figure or a source.
    Check,
    /// Nothing to change.
    Fine,
}

impl Gravity {
    pub const ALL: [Gravity; 5] = [
        Gravity::Stop,
        Gravity::Adapt,
        Gravity::Watch,
        Gravity::Check,
        Gravity::Fine,
    ];
}

/// Which table a line comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Topic {
    Revue,
    Biologie,
    Rein,
    Foie,
    Grossesse,
    Allaitement,
    Age,
    Ecrasement,
    Interactions,
}

impl Topic {
    pub const ALL: [Topic; 9] = [
        Topic::Revue,
        Topic::Biologie,
        Topic::Rein,
        Topic::Foie,
        Topic::Grossesse,
        Topic::Allaitement,
        Topic::Age,
        Topic::Ecrasement,
        Topic::Interactions,
    ];
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub topic: Topic,
    pub gravity: Gravity,
    /// The treatment(s) concerned, as written on the file.
    pub treatment: String,
    /// The table's verdict, a few words (« Dose à réduire »).
    pub verdict: String,
    /// The table's conduct, one or two sentences.
    pub conduct: String,
    pub source: String,
}

/// The treatments as the tables read them.
pub fn treatments(drugs: &[Drug]) -> Vec<Treatment<'_>> {
    drugs
        .iter()
        .map(|d| Treatment {
            name: &d.name,
            dci: &d.dci,
            class: &d.class,
            tags: &d.tags,
        })
        .collect()
}

/// Every line every relevant table gives, most urgent first.
///
/// Which tables run is decided by the context, never guessed: the liver
/// only with a stage, pregnancy only when the file says so, crushing
/// only when asked. The kidney always runs — a treatment that adapts to
/// the DFG on a file with no DFG is worth saying (« à vérifier »).
pub fn run(drugs: &[Drug], ctx: &Context, over: &Overrides) -> Vec<Line> {
    let list = treatments(drugs);
    let mut out = Vec::new();
    if list.is_empty() {
        return out;
    }

    for p in revue::resolve(revue::review(&list), over) {
        use bpm_caddy::biology::Severity;
        out.push(Line {
            topic: Topic::Revue,
            gravity: match p.severity {
                Severity::Alert => Gravity::Stop,
                Severity::Warn => Gravity::Adapt,
                Severity::Info => Gravity::Watch,
            },
            treatment: p.drugs.join(", "),
            verdict: p.title,
            conduct: p.detail,
            source: String::new(),
        });
    }

    let readings: Vec<bpm_caddy::biology::Reading> = ctx
        .bio
        .iter()
        .filter(|r| !r.code.is_empty())
        .map(|r| bpm_caddy::biology::Reading {
            code: r.code.as_str(),
            value: r.value,
            date: r.taken_on.as_str(),
            bound: bpm_caddy::biology::bound_sign(r.value, &r.remark),
        })
        .collect();
    for f in bpm_caddy::biology::resolve(bpm_caddy::biology::read(&readings, &list), over) {
        use bpm_caddy::biology::Severity;
        out.push(Line {
            topic: Topic::Biologie,
            gravity: match f.severity {
                Severity::Alert => Gravity::Stop,
                Severity::Warn => Gravity::Adapt,
                Severity::Info => Gravity::Watch,
            },
            treatment: bpm_caddy::biology::find(f.code)
                .map(|a| a.label.to_owned())
                .unwrap_or_else(|| f.code.to_owned()),
            verdict: String::new(),
            conduct: f.text,
            source: String::new(),
        });
    }

    for r in renal::resolve(renal::read_bounded(&list, ctx.dfg, &ctx.dfg_bound), over) {
        let (gravity, verdict) = match r.level {
            Some(renal::Level::Contraindicated) => (Gravity::Stop, r.label_of_level()),
            Some(renal::Level::Reduce) => (Gravity::Adapt, r.label_of_level()),
            Some(renal::Level::Watch) => (Gravity::Watch, r.label_of_level()),
            None if ctx.dfg.is_some() && !r.bounded => (Gravity::Fine, String::new()),
            None => (Gravity::Check, String::new()),
        };
        if gravity == Gravity::Fine {
            continue;
        }
        out.push(Line {
            topic: Topic::Rein,
            gravity,
            treatment: r.treatment,
            verdict,
            conduct: r.conduct,
            source: r.source.to_owned(),
        });
    }

    if let Some(stage) = ctx.liver {
        for r in hepatic::resolve(hepatic::read(&list, Some(stage)), over) {
            let gravity = match r.verdict {
                hepatic::Verdict::Adapt(hepatic::Level::Contraindicated) => Gravity::Stop,
                hepatic::Verdict::Adapt(hepatic::Level::Reduce) => Gravity::Adapt,
                hepatic::Verdict::Adapt(hepatic::Level::Watch) => Gravity::Watch,
                hepatic::Verdict::Unknown => Gravity::Check,
                hepatic::Verdict::Nothing => continue,
            };
            out.push(Line {
                topic: Topic::Foie,
                gravity,
                treatment: r.treatment,
                verdict: hepatic_verdict(r.verdict),
                conduct: r.conduct,
                source: r.source.to_owned(),
            });
        }
    }

    if ctx.pregnant || ctx.breastfeeding {
        for r in gravidity::resolve(gravidity::read(&list), over) {
            if ctx.pregnant {
                if let Some(g) = pregnancy_gravity(r.pregnancy) {
                    let conduct = if r.term.is_empty() {
                        r.pregnancy_note.clone()
                    } else {
                        format!("{} — {}", r.term, r.pregnancy_note)
                    };
                    out.push(Line {
                        topic: Topic::Grossesse,
                        gravity: g,
                        treatment: r.treatment.clone(),
                        verdict: r.pregnancy.label().to_owned(),
                        conduct,
                        source: r.source.to_owned(),
                    });
                }
            }
            if ctx.breastfeeding {
                if let Some(g) = pregnancy_gravity(r.breastfeeding) {
                    out.push(Line {
                        topic: Topic::Allaitement,
                        gravity: g,
                        treatment: r.treatment.clone(),
                        verdict: r.breastfeeding.label().to_owned(),
                        conduct: r.breastfeeding_note.clone(),
                        source: r.source.to_owned(),
                    });
                }
            }
        }
    }

    for r in elderly::resolve(elderly::read(&list, ctx.age), over) {
        let gravity = match r.level {
            Some(elderly::Level::Avoid) => Gravity::Adapt,
            Some(elderly::Level::Caution) => Gravity::Watch,
            // No birth date, or not old enough for this line: the table
            // says nothing about this patient.
            None => continue,
        };
        let conduct = if r.instead.is_empty() {
            r.risk
        } else {
            format!("{} {}", r.risk, r.instead)
        };
        out.push(Line {
            topic: Topic::Age,
            gravity,
            treatment: r.treatment,
            verdict: r.level.map(|l| l.label().to_owned()).unwrap_or_default(),
            conduct,
            source: r.source.to_owned(),
        });
    }

    if ctx.crush {
        for r in crush::resolve(crush::read(&list), over) {
            let gravity = match r.verdict {
                crush::Verdict::No => Gravity::Stop,
                crush::Verdict::Conditional => Gravity::Watch,
                crush::Verdict::Unknown => Gravity::Check,
                crush::Verdict::Yes => Gravity::Fine,
            };
            let conduct = if r.instead.is_empty() {
                r.why
            } else {
                format!("{} {}", r.why, r.instead)
            };
            out.push(Line {
                topic: Topic::Ecrasement,
                gravity,
                treatment: r.treatment,
                verdict: r.verdict.label().to_owned(),
                conduct,
                source: r.source.to_owned(),
            });
        }
    }

    let reading = cyp::cross(&list);
    for c in &reading.crossings {
        let gravity = match c.weight {
            cyp::Weight::Major => Gravity::Adapt,
            cyp::Weight::Notable => Gravity::Watch,
            cyp::Weight::Minor => continue,
        };
        out.push(Line {
            topic: Topic::Interactions,
            gravity,
            treatment: format!("{} + {}", c.actor, c.affected),
            verdict: format!("{} ({})", c.shift.label(), c.enzyme.label()),
            conduct: {
                let force = c.actor_force.map(|f| f.label(c.role)).unwrap_or_default();
                let args: [&dyn std::fmt::Display; 5] = [
                    &c.actor,
                    &c.enzyme.label(),
                    &force,
                    &c.affected,
                    &c.weight.label(),
                ];
                match c.role {
                    cyp::Role::Inducer => crate::t::tf("analyse.cyp_induces", &args),
                    _ => crate::t::tf("analyse.cyp_inhibits", &args),
                }
            },
            source: if c.sources.0 == c.sources.1 {
                c.sources.0.to_owned()
            } else {
                format!("{} ; {}", c.sources.0, c.sources.1)
            },
        });
    }

    out.sort_by(|a, b| {
        a.gravity
            .cmp(&b.gravity)
            .then(a.topic.cmp(&b.topic))
            .then(a.treatment.cmp(&b.treatment))
    });
    out
}

fn pregnancy_gravity(level: gravidity::Level) -> Option<Gravity> {
    match level {
        gravidity::Level::Interdit => Some(Gravity::Stop),
        gravidity::Level::Eviter => Some(Gravity::Adapt),
        gravidity::Level::Prudence => Some(Gravity::Watch),
        gravidity::Level::SansDonnee => Some(Gravity::Check),
        gravidity::Level::Compatible => None,
    }
}

fn hepatic_verdict(v: hepatic::Verdict) -> String {
    match v {
        hepatic::Verdict::Adapt(hepatic::Level::Contraindicated) => renal::Level::Contraindicated,
        hepatic::Verdict::Adapt(hepatic::Level::Reduce) => renal::Level::Reduce,
        hepatic::Verdict::Adapt(hepatic::Level::Watch) => renal::Level::Watch,
        _ => return String::new(),
    }
    .label()
    .to_owned()
}

trait RenalLabel {
    fn label_of_level(&self) -> String;
}

impl RenalLabel for renal::Resolved {
    /// « Dose à réduire — sous 30 mL/min », when the step has a threshold.
    fn label_of_level(&self) -> String {
        let level = self.level.map(|l| l.label()).unwrap_or_default();
        match self.below {
            Some(b) => crate::t::tf("analyse.below", &[&level, &b]),
            None => level.to_owned(),
        }
    }
}

/// The most recent DFG of a file's biology, dated on or before `today`:
/// the state of the kidney now, not the lowest value ever seen. Returns
/// the value, its lab bound and its date.
pub fn latest_dfg(bio: &[bpm_caddy::db::BioResult], today: &str) -> Option<(f64, String, String)> {
    bio.iter()
        .filter(|r| {
            r.code.eq_ignore_ascii_case("DFG")
                && !r.taken_on.trim().is_empty()
                && (today.is_empty() || r.taken_on.as_str() <= today)
        })
        .max_by(|a, b| a.taken_on.cmp(&b.taken_on))
        .map(|r| {
            (
                r.value,
                bpm_caddy::biology::bound_sign(r.value, &r.remark).to_owned(),
                r.taken_on.clone(),
            )
        })
}

/// How many lines of each gravity: the counts on the summary chips.
pub fn counts(lines: &[Line]) -> [usize; 5] {
    let mut out = [0; 5];
    for l in lines {
        out[l.gravity as usize] += 1;
    }
    out
}

/// Clairance de la créatinine, Cockcroft et Gault, mL/min — the formula
/// the shipped renal table prints: (140 − âge) × poids × k / créatinine
/// (µmol/L), k = 1,23 for a man and 1,04 for a woman. `None` on figures
/// outside what the formula was ever meant for.
pub fn cockcroft(age: u32, weight_kg: f64, creat_umol: f64, female: bool) -> Option<f64> {
    if !(1..140).contains(&age)
        || !(weight_kg.is_finite() && weight_kg > 0.0 && weight_kg < 400.0)
        || !(creat_umol.is_finite() && creat_umol > 0.0)
    {
        return None;
    }
    let k = if female { 1.04 } else { 1.23 };
    Some((140.0 - age as f64) * weight_kg * k / creat_umol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drug(name: &str, dci: &str, class: &str) -> Drug {
        Drug {
            name: name.to_owned(),
            dci: dci.to_owned(),
            class: class.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(run(&[], &Context::default(), &Overrides::default()).is_empty());
    }

    #[test]
    fn metformin_under_a_low_dfg_stops_before_it_watches() {
        let drugs = [drug("Metformine 1000", "metformine", "biguanide")];
        let ctx = Context {
            dfg: Some(20.0),
            ..Default::default()
        };
        let lines = run(&drugs, &ctx, &Overrides::default());
        let renal: Vec<_> = lines.iter().filter(|l| l.topic == Topic::Rein).collect();
        assert!(!renal.is_empty(), "{lines:?}");
        assert_eq!(renal[0].gravity, Gravity::Stop, "{renal:?}");
        assert!(!renal[0].source.is_empty());
    }

    #[test]
    fn without_a_dfg_the_kidney_says_check_not_fine() {
        let drugs = [drug("Metformine 1000", "metformine", "biguanide")];
        let lines = run(&drugs, &Context::default(), &Overrides::default());
        assert!(lines
            .iter()
            .any(|l| l.topic == Topic::Rein && l.gravity == Gravity::Check));
    }

    #[test]
    fn tables_that_are_not_asked_stay_silent() {
        let drugs = [
            drug("Metformine 1000", "metformine", "biguanide"),
            drug("Ibuprofène 400", "ibuprofène", "AINS"),
        ];
        let lines = run(&drugs, &Context::default(), &Overrides::default());
        for topic in [
            Topic::Foie,
            Topic::Grossesse,
            Topic::Allaitement,
            Topic::Ecrasement,
        ] {
            assert!(lines.iter().all(|l| l.topic != topic), "{topic:?}");
        }
    }

    #[test]
    fn pregnancy_reads_nsaids() {
        let drugs = [drug("Ibuprofène 400", "ibuprofène", "AINS")];
        let ctx = Context {
            pregnant: true,
            ..Default::default()
        };
        let lines = run(&drugs, &ctx, &Overrides::default());
        assert!(
            lines.iter().any(|l| l.topic == Topic::Grossesse),
            "{lines:?}"
        );
    }

    #[test]
    fn crushing_answers_every_line_when_asked() {
        let drugs = [
            drug("Paracétamol 500", "paracétamol", "antalgique"),
            drug("Xyzzy LP", "inconnue", ""),
        ];
        let ctx = Context {
            crush: true,
            ..Default::default()
        };
        let lines = run(&drugs, &ctx, &Overrides::default());
        let n = lines
            .iter()
            .filter(|l| l.topic == Topic::Ecrasement)
            .count();
        assert_eq!(n, 2, "toute ligne reçoit une réponse : {lines:?}");
    }

    #[test]
    fn lines_come_most_urgent_first() {
        let drugs = [
            drug("Metformine 1000", "metformine", "biguanide"),
            drug("Simvastatine 40", "simvastatine", "statine"),
            drug("Clarithromycine 500", "clarithromycine", "macrolide"),
            drug("Ibuprofène 400", "ibuprofène", "AINS"),
        ];
        let ctx = Context {
            dfg: Some(25.0),
            age: Some(82),
            pregnant: true,
            crush: true,
            ..Default::default()
        };
        let lines = run(&drugs, &ctx, &Overrides::default());
        assert!(lines.windows(2).all(|w| w[0].gravity <= w[1].gravity));
        let c = counts(&lines);
        assert_eq!(c.iter().sum::<usize>(), lines.len());
        assert!(
            lines.iter().any(|l| l.topic == Topic::Interactions),
            "simvastatine sous clarithromycine : {lines:?}"
        );
    }

    #[test]
    fn the_latest_dated_dfg_wins_and_the_future_does_not_count() {
        let r = |v: f64, d: &str| bpm_caddy::db::BioResult {
            code: "DFG".into(),
            value: v,
            taken_on: d.into(),
            ..Default::default()
        };
        let bio = [
            r(80.0, "2020-01-01"),
            r(28.0, "2026-09-01"),
            r(90.0, "2027-01-01"),
        ];
        assert_eq!(latest_dfg(&bio, "2026-10-01").unwrap().0, 28.0);
        assert!(latest_dfg(&[], "2026-10-01").is_none());
    }

    #[test]
    fn cockcroft_matches_the_printed_formula() {
        // (140 − 80) × 60 × 1,23 / 100 = 44,28
        let v = cockcroft(80, 60.0, 100.0, false).unwrap();
        assert!((v - 44.28).abs() < 0.01);
        let f = cockcroft(80, 60.0, 100.0, true).unwrap();
        assert!(f < v);
        assert_eq!(cockcroft(0, 60.0, 100.0, false), None);
        assert_eq!(cockcroft(80, 0.0, 100.0, false), None);
        assert_eq!(cockcroft(80, 60.0, f64::NAN, false), None);
    }
}
