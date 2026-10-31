//! Vérifier une ordonnance: type the treatments, give what is known of
//! the patient, read every table at once. Starts empty, or filled from a
//! file (« Vérifier » on a patient).

use super::{Ctx, Go};
use crate::analyse::{self, Context, Gravity, Line, Topic};
use crate::t::{t, tf, tn};
use crate::ui;
use bpm_caddy::hepatic::Stage;
use eframe::egui;

#[derive(Default)]
pub struct Verifier {
    drugs: Vec<i64>,
    search: String,
    patient: Option<(i64, String)>,
    dfg: String,
    age: String,
    pregnant: bool,
    breastfeeding: bool,
    crush: bool,
    liver: Option<Stage>,
    bio: Vec<bpm_caddy::db::BioResult>,
    // Cockcroft.
    weight: String,
    creat: String,
    female: bool,
    show_cockcroft: bool,
    hide_checks: bool,
    memo: Option<(u64, Context, Vec<i64>, Vec<Line>)>,
}

impl Verifier {
    /// Fill from a file: its treatments, its latest DFG, its age.
    pub fn load_patient(&mut self, cx: &mut Ctx, id: i64) {
        let Some(p) = cx.s.patient(id).cloned() else {
            return;
        };
        self.drugs =
            cx.s.db
                .drugs_for_patient(id)
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.id)
                .collect();
        self.bio = cx.s.db.bio_results(id).unwrap_or_default();
        self.dfg = analyse::latest_dfg(&self.bio, &cx.s.today)
            .map(|(v, b, _)| format!("{b}{}", dfg_text(v)))
            .unwrap_or_default();
        self.age =
            cx.s.age(&p.birth_date)
                .map(|a| a.to_string())
                .unwrap_or_default();
        self.pregnant = !p.pregnancy_ddr.is_empty();
        self.female = p.sex == "F";
        self.breastfeeding = false;
        self.crush = false;
        self.liver = None;
        self.patient = Some((
            id,
            format!("{} {}", p.last_name.to_uppercase(), p.first_name),
        ));
        self.memo = None;
    }

    pub fn clear(&mut self) {
        *self = Verifier::default();
    }

    fn context(&self) -> Context {
        let (bound, value) = split_bound(&self.dfg);
        Context {
            dfg: value,
            dfg_bound: bound,
            bio: self.bio.clone(),
            liver: self.liver,
            age: self.age.trim().parse().ok().filter(|a| *a < 130),
            pregnant: self.pregnant,
            breastfeeding: self.breastfeeding,
            crush: self.crush,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        // Wide enough: the questions on the left, the answer on the right,
        // both in view. Narrow: one column, the answer under.
        let full = ui.available_rect_before_wrap();
        if full.width() >= ui::chars_wide(ui, 100.0) {
            let gap = ui.spacing().item_spacing.x * 2.0;
            let left_w = (full.width() * 0.42).max(ui::chars_wide(ui, 44.0));
            let left = egui::Rect::from_min_size(full.min, egui::vec2(left_w, full.height()));
            let right =
                egui::Rect::from_min_max(egui::pos2(left.right() + gap, full.top()), full.max);
            motif::inside(ui, left, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("verifier_ask")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.ask(ui, cx));
            });
            motif::inside(ui, right, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("verifier_answer")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.answer(ui, cx));
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("verifier")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.ask(ui, cx);
                    self.answer(ui, cx);
                });
        }
    }

    fn ask(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let sub = self
            .patient
            .as_ref()
            .map(|(_, n)| tf("verifier.for", &[n]))
            .unwrap_or_else(|| t("verifier.sub").to_owned());
        ui::title(ui, t("verifier.title"), Some(&sub));
        ui.horizontal_wrapped(|ui| {
            if let Some((id, _)) = self.patient {
                if motif::button(ui, t("verifier.open_file")).clicked() {
                    cx.go(Go::Patient(id));
                }
            }
            if motif::button(ui, t("verifier.clear")).clicked() {
                self.clear();
            }
        });

        // The patient.
        ui::heading(ui, t("verifier.patient"));
        ui.horizontal_wrapped(|ui| {
            ui.label(t("verifier.age"));
            ui::field(ui, &mut self.age, 5.0, "");
            ui.label(t("verifier.dfg"));
            ui::field(ui, &mut self.dfg, 7.0, t("verifier.dfg_hint"));
            if motif::button(ui, t("verifier.cockcroft")).clicked() {
                self.show_cockcroft = !self.show_cockcroft;
            }
        });
        if self.show_cockcroft {
            self.cockcroft_ui(ui);
        }
        ui.horizontal_wrapped(|ui| {
            motif::checkbox(ui, &mut self.pregnant, t("verifier.pregnant"));
            motif::checkbox(ui, &mut self.breastfeeding, t("verifier.breastfeeding"));
            motif::checkbox(ui, &mut self.crush, t("verifier.crush"));
            liver_select(ui, "verifier_liver", &mut self.liver);
        });

        // The treatments.
        ui::heading(ui, &tn("verifier.treatments", self.drugs.len()));
        let mut drop: Option<usize> = None;
        for (k, id) in self.drugs.iter().enumerate() {
            let Some(d) = cx.s.drug(*id) else { continue };
            ui.horizontal(|ui| {
                if motif::button(ui, t("verifier.remove")).clicked() {
                    drop = Some(k);
                }
                ui.label(egui::RichText::new(&d.name).strong());
                if !d.dci.is_empty() {
                    ui.label(motif::hint(&d.dci));
                }
            });
        }
        if let Some(k) = drop {
            self.drugs.remove(k);
        }
        let r = ui::field(ui, &mut self.search, 30.0, t("verifier.add"));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !self.search.trim().is_empty() {
            let found = super::medicaments::matching(&cx.s.drugs, &self.search, 6, &self.drugs);
            let mut pick = if enter { found.first().copied() } else { None };
            for &i in &found {
                if motif::list_row(ui, egui::RichText::new(&cx.s.drugs[i].name), false).clicked() {
                    pick = Some(i);
                }
            }
            if found.is_empty() {
                ui::empty(ui, t("patients.no_drug_match"));
            }
            if let Some(i) = pick {
                self.drugs.push(cx.s.drugs[i].id);
                self.search.clear();
                r.request_focus();
            }
        }
    }

    fn answer(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        ui::heading(ui, t("verifier.result"));
        if self.drugs.is_empty() {
            ui::empty(ui, t("verifier.empty"));
            return;
        }
        let ctx = self.context();
        let fresh = self
            .memo
            .as_ref()
            .is_some_and(|(g, c, d, _)| *g == cx.s.generation && *c == ctx && *d == self.drugs);
        if !fresh {
            let drugs: Vec<_> = self
                .drugs
                .iter()
                .filter_map(|id| cx.s.drug(*id).cloned())
                .collect();
            let lines = analyse::run(&drugs, &ctx, &cx.s.overrides);
            self.memo = Some((cx.s.generation, ctx, self.drugs.clone(), lines));
        }
        motif::checkbox(ui, &mut self.hide_checks, t("verifier.hide_checks"));
        if let Some((_, _, _, lines)) = &self.memo {
            if self.hide_checks {
                let kept: Vec<Line> = lines
                    .iter()
                    .filter(|l| l.gravity < Gravity::Check)
                    .cloned()
                    .collect();
                show_lines(ui, &kept);
            } else {
                show_lines(ui, lines);
            }
        }
        ui.add_space(ui::line(ui));
        ui.label(motif::hint(t("verifier.disclaimer")));
    }

    fn cockcroft_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(t("verifier.weight"));
            ui::field(ui, &mut self.weight, 5.0, "kg");
            ui.label(t("verifier.creat"));
            ui::field(ui, &mut self.creat, 6.0, "µmol/L");
            motif::checkbox(ui, &mut self.female, t("verifier.female"));
            let num = |s: &str| s.trim().replace(',', ".").parse::<f64>().ok();
            let age = self.age.trim().parse::<u32>().ok();
            let result = match (age, num(&self.weight), num(&self.creat)) {
                (Some(a), Some(w), Some(c)) => analyse::cockcroft(a, w, c, self.female),
                _ => None,
            };
            match result {
                Some(v) => {
                    ui.label(
                        egui::RichText::new(tf("verifier.cockcroft_is", &[&dfg_text(v)])).strong(),
                    );
                    if motif::button(ui, t("verifier.use")).clicked() {
                        self.dfg = dfg_text(v);
                    }
                }
                None => {
                    ui.label(motif::hint(t("verifier.cockcroft_needs")));
                }
            }
        });
    }
}

/// « <30 » into (« < », 30).
fn split_bound(s: &str) -> (String, Option<f64>) {
    let s = s.trim();
    let (bound, rest) = match s.chars().next() {
        Some(c @ ('<' | '>' | '≤' | '≥')) => (c.to_string(), s[c.len_utf8()..].trim()),
        _ => (String::new(), s),
    };
    let v = rest
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0);
    (if v.is_some() { bound } else { String::new() }, v)
}

/// A DFG as the screen writes it: whole when it is, to the tenth
/// otherwise — so a 29,6 never reads « 30 » beside « sous 30 ».
pub fn dfg_text(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{v:.0}")
    } else {
        bpm_caddy::strings::decimal(v, 1)
    }
}

/// The liver stage, asked as a one-line choice.
pub fn liver_select(ui: &mut egui::Ui, salt: &str, liver: &mut Option<Stage>) {
    let mut options = vec![(None, t("verifier.liver_none").to_owned())];
    for s in Stage::ALL {
        options.push((Some(*s), tf("verifier.liver_stage", &[&s.short()])));
    }
    let w = motif::select_width(ui, options.iter().map(|(_, l)| l.as_str()));
    motif::select(ui, salt, w, liver, &options);
}

pub fn topic_label(topic: Topic) -> &'static str {
    match topic {
        Topic::Revue => t("topic.revue"),
        Topic::Biologie => t("topic.biologie"),
        Topic::Rein => t("topic.rein"),
        Topic::Foie => t("topic.foie"),
        Topic::Grossesse => t("topic.grossesse"),
        Topic::Allaitement => t("topic.allaitement"),
        Topic::Age => t("topic.age"),
        Topic::Ecrasement => t("topic.ecrasement"),
        Topic::Interactions => t("topic.interactions"),
    }
}

/// The reading: counts first, then each line under its gravity.
pub fn show_lines(ui: &mut egui::Ui, lines: &[Line]) {
    let counts = analyse::counts(lines);
    if lines.is_empty() {
        ui.horizontal(|ui| {
            ui::pict(ui, motif::Pict::Check, motif::text_dim());
            ui.label(t("verifier.nothing"));
        });
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for g in Gravity::ALL {
            if counts[g as usize] > 0 {
                ui::gravity_chip(ui, g, Some(counts[g as usize]));
            }
        }
    });
    ui.add_space(ui::line(ui) * 0.3);
    for l in lines {
        ui.horizontal_wrapped(|ui| {
            ui::pict(ui, ui::gravity_pict(l.gravity), ui::gravity_ink(l.gravity));
            ui.label(
                egui::RichText::new(topic_label(l.topic))
                    .color(motif::text_dim())
                    .size(motif::pt(ui, 11.0)),
            );
            ui.label(egui::RichText::new(&l.treatment).strong());
            if !l.verdict.is_empty() {
                ui.label(egui::RichText::new(&l.verdict).color(ui::gravity_ink(l.gravity)));
            }
        });
        if !l.conduct.is_empty() {
            ui.indent(("conduct", &l.treatment, l.topic), |ui| {
                ui.label(&l.conduct);
                if !l.source.is_empty() {
                    ui.label(motif::hint(&l.source).size(motif::pt(ui, 10.0)));
                }
            });
        }
        ui.add_space(ui::line(ui) * 0.25);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_is_kept_only_with_a_number() {
        assert_eq!(split_bound("<30"), ("<".into(), Some(30.0)));
        assert_eq!(split_bound(" 42,5 "), (String::new(), Some(42.5)));
        assert_eq!(split_bound("<"), (String::new(), None));
        assert_eq!(split_bound(""), (String::new(), None));
    }

    #[test]
    fn dfg_text_never_rounds_across_a_threshold() {
        assert_eq!(dfg_text(30.0), "30");
        assert_eq!(dfg_text(29.6), "29,6");
    }

    #[test]
    fn every_topic_has_words() {
        for topic in Topic::ALL {
            assert!(!topic_label(topic).contains('.'), "{topic:?}");
        }
    }
}
