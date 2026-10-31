//! Médicaments: the cards, searchable by name, DCI, class or tag; the
//! open card reads top-down in the order the counter asks.

use super::{Ctx, Go};
use crate::analyse::{self, Context};
use crate::t::{t, tn};
use crate::ui;
use bpm_caddy::db::{self, Drug};
use eframe::egui;

/// The card's sections, in reading order: the counter's questions first
/// (how much, what forms, a missed dose, when to send to the doctor),
/// the pharmacology after. `(column, label key)`.
const SECTIONS: [(&str, &str); 19] = [
    ("dosage", "fiche.dosage"),
    ("forms", "fiche.forms"),
    ("missed_dose", "fiche.missed_dose"),
    ("red_flags", "fiche.red_flags"),
    ("indications", "fiche.indications"),
    ("contraindications", "fiche.contraindications"),
    ("ddi", "fiche.ddi"),
    ("adverse", "fiche.adverse"),
    ("monitoring", "fiche.monitoring"),
    ("renal", "fiche.renal"),
    ("pregnancy", "fiche.pregnancy"),
    ("iup", "fiche.iup"),
    ("toxicity", "fiche.toxicity"),
    ("antidote", "fiche.antidote"),
    ("mechanism", "fiche.mechanism"),
    ("half_life", "fiche.half_life"),
    ("elimination", "fiche.elimination"),
    ("notes", "fiche.notes"),
    ("sources", "fiche.sources"),
];

/// The one-line fields of the head of the card.
const HEAD: [(&str, &str); 5] = [
    ("name", "fiche.name"),
    ("dci", "fiche.dci"),
    ("class", "fiche.class"),
    ("status", "fiche.status"),
    ("tags", "fiche.tags"),
];

fn label(key: &str) -> &'static str {
    // Keys are listed here so the text table's test sees each one.
    match key {
        "fiche.dosage" => t("fiche.dosage"),
        "fiche.forms" => t("fiche.forms"),
        "fiche.missed_dose" => t("fiche.missed_dose"),
        "fiche.red_flags" => t("fiche.red_flags"),
        "fiche.indications" => t("fiche.indications"),
        "fiche.contraindications" => t("fiche.contraindications"),
        "fiche.ddi" => t("fiche.ddi"),
        "fiche.adverse" => t("fiche.adverse"),
        "fiche.monitoring" => t("fiche.monitoring"),
        "fiche.renal" => t("fiche.renal"),
        "fiche.pregnancy" => t("fiche.pregnancy"),
        "fiche.iup" => t("fiche.iup"),
        "fiche.toxicity" => t("fiche.toxicity"),
        "fiche.antidote" => t("fiche.antidote"),
        "fiche.mechanism" => t("fiche.mechanism"),
        "fiche.half_life" => t("fiche.half_life"),
        "fiche.elimination" => t("fiche.elimination"),
        "fiche.notes" => t("fiche.notes"),
        "fiche.sources" => t("fiche.sources"),
        "fiche.name" => t("fiche.name"),
        "fiche.dci" => t("fiche.dci"),
        "fiche.class" => t("fiche.class"),
        "fiche.status" => t("fiche.status"),
        "fiche.tags" => t("fiche.tags"),
        _ => "",
    }
}

/// A file on the open card: its id and its name as listed.
type OnIt = (i64, String);

#[derive(Default)]
pub struct Medicaments {
    filter: String,
    open: Option<i64>,
    edit: Option<(Drug, Drug)>,
    new_name: String,
    confirm_delete: bool,
    focus_filter: bool,
    /// The patients on the open card, and the generation they were read at.
    on_it: Option<(i64, u64, Vec<OnIt>)>,
    tables: Option<(i64, u64, Vec<analyse::Line>)>,
}

impl Medicaments {
    pub fn open(&mut self, id: i64) {
        if self.open != Some(id) {
            self.open = Some(id);
            self.edit = None;
            self.confirm_delete = false;
        }
    }

    pub fn focus_filter(&mut self) {
        self.focus_filter = true;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let full = ui.available_rect_before_wrap();
        let list_w = ui::chars_wide(ui, 30.0).min(full.width() * 0.4);
        let gap = ui.spacing().item_spacing.x * 2.0;
        let left = egui::Rect::from_min_size(full.min, egui::vec2(list_w, full.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.right() + gap, full.top()), full.max);
        motif::inside(ui, left, |ui| self.list(ui, cx));
        motif::inside(ui, right, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("drug_card")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.edit.is_some() {
                        self.edit_ui(ui, cx);
                    } else if let Some(id) = self.open {
                        self.card(ui, cx, id);
                    } else {
                        ui::title(ui, t("medicaments.title"), None);
                        ui::empty(ui, t("medicaments.pick"));
                        self.new_ui(ui, cx);
                    }
                });
        });
    }

    fn list(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let r = ui::field(ui, &mut self.filter, 40.0, t("medicaments.filter"));
        if std::mem::take(&mut self.focus_filter) {
            r.request_focus();
        }
        let rows = matching(&cx.s.drugs, &self.filter, usize::MAX, &[]);
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(&i) = rows.first() {
                let id = cx.s.drugs[i].id;
                self.open(id);
            }
        }
        ui.label(motif::hint(tn("medicaments.count", rows.len())));
        let row_h = motif::list_row_height(ui);
        egui::ScrollArea::vertical()
            .id_salt("drug_list")
            .auto_shrink([false, false])
            .show_rows(ui, row_h, rows.len(), |ui, range| {
                for &i in &rows[range] {
                    let d = &cx.s.drugs[i];
                    let mut text = egui::RichText::new(&d.name);
                    if db::DrugStatus::parse(&d.status)
                        .is_some_and(|s| s != db::DrugStatus::Marketed)
                    {
                        text = text.color(motif::warn());
                    }
                    if motif::list_row(ui, text, self.open == Some(d.id)).clicked() {
                        let id = d.id;
                        self.open(id);
                    }
                }
            });
    }

    fn new_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        ui::heading(ui, t("medicaments.new"));
        ui.horizontal(|ui| {
            ui::field(ui, &mut self.new_name, 28.0, t("medicaments.new_hint"));
            if motif::button(ui, t("medicaments.create")).clicked()
                && !self.new_name.trim().is_empty()
            {
                match cx.s.db.add_drug(self.new_name.trim()) {
                    Ok(id) => {
                        self.new_name.clear();
                        cx.s.changed();
                        self.open(id);
                        if let Some(d) = cx.s.drug(id).cloned() {
                            self.edit = Some((d.clone(), d));
                        }
                    }
                    Err(e) => cx.s.fail(e),
                }
            }
        });
    }

    fn card(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, id: i64) {
        let Some(d) = cx.s.drug(id).cloned() else {
            self.open = None;
            return;
        };
        let sub = [d.dci.as_str(), d.class.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("  ·  ");
        ui::title(ui, &d.name, Some(&sub));
        ui.horizontal_wrapped(|ui| {
            if motif::button(ui, t("medicaments.edit")).clicked() {
                self.edit = Some((d.clone(), d.clone()));
            }
            if motif::button(ui, t("medicaments.print")).clicked() {
                match crate::print::monograph(cx.s, id) {
                    Ok(_) => cx.s.say(t("print.opened")),
                    Err(e) => cx.s.fail(e),
                }
            }
            if let Some(status) = db::DrugStatus::parse(&d.status) {
                if status != db::DrugStatus::Marketed {
                    motif::badge(ui, &d.status, Some(motif::Pict::Warn), motif::warn(), false);
                }
            }
            for tag in d.tags.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                motif::badge(ui, tag, None, motif::bg_light(), false);
            }
        });

        let mut any = false;
        for (col, key) in SECTIONS {
            let text = db::drug_field(&d, col).unwrap_or_default();
            if text.trim().is_empty() {
                continue;
            }
            any = true;
            ui::heading(ui, label(key));
            if col == "sources" {
                for (n, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
                    ui.label(motif::hint(format!("{}. {}", n + 1, line.trim())));
                }
            } else {
                ui.label(text);
            }
        }
        if !any {
            ui::empty(ui, t("medicaments.blank"));
        }

        // What the tables say of this molecule, without a patient.
        let fresh = self
            .tables
            .as_ref()
            .is_some_and(|(i, g, _)| *i == id && *g == cx.s.generation);
        if !fresh {
            let ctx = Context {
                pregnant: true,
                breastfeeding: true,
                crush: true,
                dfg: Some(f64::MAX),
                ..Default::default()
            };
            let lines = analyse::run(std::slice::from_ref(&d), &ctx, &cx.s.overrides)
                .into_iter()
                .filter(|l| l.topic != analyse::Topic::Rein)
                .collect();
            self.tables = Some((id, cx.s.generation, lines));
        }
        if let Some((_, _, lines)) = &self.tables {
            if !lines.is_empty() {
                ui::heading(ui, t("medicaments.tables"));
                super::verifier::show_lines(ui, lines);
            }
        }

        // Who takes it.
        let fresh = self
            .on_it
            .as_ref()
            .is_some_and(|(i, g, _)| *i == id && *g == cx.s.generation);
        if !fresh {
            let who =
                cx.s.db
                    .patients_for_drug(id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        (
                            p.id,
                            format!("{} {}", p.last_name.to_uppercase(), p.first_name),
                        )
                    })
                    .collect();
            self.on_it = Some((id, cx.s.generation, who));
        }
        if let Some((_, _, who)) = &self.on_it {
            ui::heading(ui, &tn("medicaments.patients", who.len()));
            let mut go = None;
            ui.horizontal_wrapped(|ui| {
                for (pid, name) in who {
                    if motif::button(ui, name).clicked() {
                        go = Some(*pid);
                    }
                }
            });
            if let Some(pid) = go {
                cx.go(Go::Patient(pid));
            }
        }
    }

    fn edit_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let Some((shown, draft)) = self.edit.as_mut() else {
            return;
        };
        ui::title(ui, t("medicaments.edit_title"), Some(&shown.name));
        let lw = 18.0;
        for (col, key) in HEAD {
            let mut v = db::drug_field(draft, col).unwrap_or_default().to_owned();
            if ui::form_row(ui, label(key), lw, &mut v, 40.0, "").changed() {
                db::set_drug_field(draft, col, v);
            }
        }
        let w = ui.available_width();
        for (col, key) in SECTIONS {
            ui::heading(ui, label(key));
            let mut v = db::drug_field(draft, col).unwrap_or_default().to_owned();
            let rows = v.lines().count().clamp(2, 12) as f32;
            let h = ui::line(ui) * (rows + 0.8);
            if motif::area(
                ui,
                egui::vec2(w, h),
                egui::TextEdit::multiline(&mut v).id_salt(("drug_edit", col)),
            )
            .changed()
            {
                db::set_drug_field(draft, col, v);
            }
        }
        let mut save = false;
        let mut cancel = false;
        let mut delete = false;
        ui.add_space(ui::line(ui) * 0.5);
        ui.horizontal(|ui| {
            save = motif::button(ui, t("app.save")).clicked();
            cancel = motif::button(ui, t("app.cancel")).clicked();
            ui.add_space(ui::chars_wide(ui, 4.0));
            if self.confirm_delete {
                ui.label(egui::RichText::new(t("medicaments.delete_sure")).color(motif::alert()));
                delete = motif::button(ui, t("medicaments.delete_yes")).clicked();
            } else if motif::button(ui, t("medicaments.delete")).clicked() {
                self.confirm_delete = true;
            }
        });
        if cancel {
            self.edit = None;
            self.confirm_delete = false;
            return;
        }
        if delete {
            let Some((shown, _)) = self.edit.take() else {
                return;
            };
            match cx.s.db.delete_drug(shown.id, &shown.name) {
                Ok(true) => {
                    self.open = None;
                    cx.s.say(t("medicaments.deleted"));
                    cx.s.changed();
                }
                Ok(false) => cx.s.stale(),
                Err(e) => cx.s.fail(e),
            }
            self.confirm_delete = false;
            return;
        }
        if save {
            let Some((shown, draft)) = self.edit.take() else {
                return;
            };
            if draft.name.trim().is_empty() {
                cx.s.fail(t("medicaments.need_name"));
                self.edit = Some((shown, draft));
                return;
            }
            if draft == shown {
                return;
            }
            match cx.s.db.update_drug(&draft, &shown) {
                Ok(true) => {
                    cx.s.say(t("medicaments.saved"));
                    cx.s.changed();
                }
                Ok(false) => cx.s.stale(),
                Err(e) => cx.s.fail(e),
            }
        }
    }
}

/// Indices of the cards matching `query` (name, DCI, class, tags), best
/// first, at most `limit`, leaving out the ids in `except`. Every card,
/// alphabetically, when the query is empty.
pub fn matching(drugs: &[Drug], query: &str, limit: usize, except: &[i64]) -> Vec<usize> {
    let q = query.trim();
    if q.is_empty() {
        return (0..drugs.len())
            .filter(|&i| !except.contains(&drugs[i].id))
            .take(limit)
            .collect();
    }
    let mut scored: Vec<(i32, usize)> = drugs
        .iter()
        .enumerate()
        .filter(|(_, d)| !except.contains(&d.id))
        .filter_map(|(i, d)| {
            // The name weighs most: « doli » should open Doliprane before
            // a card that merely lists paracétamol in its class.
            let name = bpm_caddy::fuzzy::score(q, &d.name).map(|s| s + 1000);
            let rest = bpm_caddy::fuzzy::score(q, &format!("{} {} {}", d.dci, d.class, d.tags));
            name.max(rest).map(|s| (s, i))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(limit).map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(id: i64, name: &str, dci: &str) -> Drug {
        Drug {
            id,
            name: name.into(),
            dci: dci.into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_name_wins_over_the_dci() {
        let all = [
            d(1, "Efferalgan", "paracétamol"),
            d(2, "Doliprane", "paracétamol"),
        ];
        assert_eq!(matching(&all, "doli", 10, &[]), vec![1]);
        let both = matching(&all, "paracetamol", 10, &[]);
        assert_eq!(both.len(), 2);
        assert_eq!(matching(&all, "paracetamol", 10, &[2]), vec![0]);
        assert_eq!(matching(&all, "", 1, &[]), vec![0]);
    }

    #[test]
    fn every_section_is_a_real_column_with_words() {
        let drug = Drug::default();
        for (col, key) in SECTIONS.iter().chain(HEAD.iter()) {
            assert!(db::drug_field(&drug, col).is_some(), "{col}");
            assert!(!label(key).is_empty(), "{key}");
            assert_ne!(label(key), *key, "{key} absent du fichier");
        }
    }
}
