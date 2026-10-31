//! Patients: the list on the left, the open file on the right.
//!
//! A file is one scrolling page, in the order the counter asks: who,
//! what they take, what to watch, the biology, the notes. Every value is
//! read from the base and written back compare-and-set.

use super::{Ctx, Go};
use crate::analyse::{self, Context, Line};
use crate::t::{t, tf, tn};
use crate::ui::{self, Typing};
use bpm_caddy::db::{self, BioResult, Drug, Interview, Note, NoteSubject, Patient};
use eframe::egui;
use std::collections::HashMap;

#[derive(Default)]
pub struct Patients {
    filter: String,
    open: Option<i64>,
    file: Option<File>,
    form: Option<Form>,
    add_drug: String,
    note: String,
    dfg: String,
    dfg_day: String,
    typing: Typing,
    /// Per-visit questions the file does not store: they change from
    /// one counter visit to the next.
    crush: bool,
    breastfeeding: bool,
    liver: Option<bpm_caddy::hepatic::Stage>,
    confirm_delete: bool,
    focus_filter: bool,
}

/// The open file, read once per base generation.
struct File {
    id: i64,
    generation: u64,
    patient: Patient,
    drugs: Vec<Drug>,
    dosage: HashMap<i64, String>,
    posology: HashMap<i64, String>,
    bio: Vec<BioResult>,
    notes: Vec<Note>,
    interviews: Vec<Interview>,
    lines: Vec<Line>,
    /// What `lines` was computed for.
    asked: (bool, bool, Option<bpm_caddy::hepatic::Stage>),
}

#[derive(Clone, Default, PartialEq)]
pub struct Form {
    /// `None` for a new file.
    shown: Option<Patient>,
    last: String,
    first: String,
    birth: String,
    sex: String,
    phone: String,
    physician: String,
    notes: String,
    error: Option<String>,
}

impl Form {
    fn of(p: &Patient) -> Self {
        Form {
            shown: Some(p.clone()),
            last: p.last_name.clone(),
            first: p.first_name.clone(),
            birth: if p.birth_date.is_empty() {
                String::new()
            } else {
                db::format_french_date(&p.birth_date)
            },
            sex: p.sex.clone(),
            phone: p.phone.clone(),
            physician: p.physician.clone(),
            notes: p.notes.clone(),
            error: None,
        }
    }
}

impl Patients {
    /// Open a file (from the search, the agenda, a link).
    pub fn open(&mut self, id: i64) {
        if self.open != Some(id) {
            self.open = Some(id);
            self.file = None;
            self.form = None;
            self.confirm_delete = false;
            self.crush = false;
            self.breastfeeding = false;
            self.liver = None;
        }
    }

    pub fn new_file(&mut self) {
        self.form = Some(Form::default());
    }

    pub fn focus_filter(&mut self) {
        self.focus_filter = true;
    }

    pub fn open_id(&self) -> Option<i64> {
        self.open
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let full = ui.available_rect_before_wrap();
        let list_w = ui::chars_wide(ui, 26.0)
            .min(full.width() * 0.34)
            .max(ui::chars_wide(ui, 18.0).min(full.width() * 0.4));
        let gap = ui.spacing().item_spacing.x * 2.0;
        let left = egui::Rect::from_min_size(full.min, egui::vec2(list_w, full.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.right() + gap, full.top()), full.max);
        motif::inside(ui, left, |ui| self.list(ui, cx));
        motif::inside(ui, right, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("patient_file")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.form.is_some() {
                        self.form_ui(ui, cx);
                    } else if self.open.is_some() {
                        self.file_ui(ui, cx);
                    } else {
                        ui::title(ui, t("patients.title"), None);
                        ui::empty(ui, t("patients.pick"));
                    }
                });
        });
    }

    fn list(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let r = motif::field(
            ui,
            ui.available_width(),
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text(motif::hint(t("patients.filter"))),
        );
        if std::mem::take(&mut self.focus_filter) {
            r.request_focus();
        }
        let rows = matching(&cx.s.patients, &self.filter);
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(&i) = rows.first() {
                let id = cx.s.patients[i].id;
                self.open(id);
            }
        }
        ui.horizontal(|ui| {
            ui.label(motif::hint(tn("patients.count", rows.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if motif::button(ui, t("patients.new")).clicked() {
                    self.new_file();
                }
            });
        });
        let row_h = motif::list_row_height(ui);
        let today = cx.s.today.clone();
        egui::ScrollArea::vertical()
            .id_salt("patient_list")
            .auto_shrink([false, false])
            .show_rows(ui, row_h, rows.len(), |ui, range| {
                for &i in &rows[range] {
                    let p = &cx.s.patients[i];
                    let age = crate::session::age_on(&p.birth_date, &today)
                        .map(|a| tf("patients.age", &[&a]))
                        .unwrap_or_default();
                    let text = format!("{} {}  {age}", p.last_name.to_uppercase(), p.first_name);
                    let selected = self.open == Some(p.id);
                    if motif::list_row(ui, egui::RichText::new(text), selected).clicked() {
                        let id = p.id;
                        self.open(id);
                    }
                }
            });
    }

    fn load(&mut self, cx: &mut Ctx) {
        let Some(id) = self.open else { return };
        let fresh = self
            .file
            .as_ref()
            .is_some_and(|f| f.id == id && f.generation == cx.s.generation);
        if fresh {
            return;
        }
        let Some(patient) = cx.s.patient(id).cloned() else {
            // Deleted elsewhere.
            self.open = None;
            self.file = None;
            return;
        };
        let db = &cx.s.db;
        let mut drugs = db.drugs_for_patient(id).unwrap_or_default();
        drugs.sort_by_cached_key(|d| bpm_caddy::fuzzy::sort_key(&d.name));
        let file = File {
            id,
            generation: cx.s.generation,
            dosage: db
                .patient_dosages(id)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            posology: db
                .patient_posologies(id)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            bio: db.bio_results(id).unwrap_or_default(),
            notes: db.notes_for(NoteSubject::Patient, id).unwrap_or_default(),
            interviews: db.interviews_for(id).unwrap_or_default(),
            drugs,
            patient,
            lines: Vec::new(),
            asked: (false, false, None),
        };
        self.file = Some(file);
        self.reckon(cx);
    }

    /// Recompute the alerts when the file or a counter question changed.
    fn reckon(&mut self, cx: &Ctx) {
        let asked = (self.crush, self.breastfeeding, self.liver);
        let Some(f) = self.file.as_mut() else { return };
        if f.asked == asked && !f.lines.is_empty() {
            return;
        }
        f.asked = asked;
        f.lines = analyse::run(
            &f.drugs,
            &context_of(f, &cx.s.today, asked),
            &cx.s.overrides,
        );
    }

    fn file_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        self.load(cx);
        self.reckon(cx);
        let Some(f) = self.file.as_ref() else {
            ui::empty(ui, t("patients.pick"));
            return;
        };
        let p = f.patient.clone();
        let pid = p.id;

        // Who.
        let age =
            cx.s.age(&p.birth_date)
                .map(|a| tf("patients.age_long", &[&a]))
                .unwrap_or_default();
        let sub = [
            age,
            if p.birth_date.is_empty() {
                String::new()
            } else {
                tf("patients.born", &[&ui::date(&p.birth_date)])
            },
            p.phone.clone(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("  ·  ");
        ui::title(
            ui,
            &format!("{} {}", p.last_name.to_uppercase(), p.first_name),
            Some(&sub),
        );
        ui.horizontal_wrapped(|ui| {
            if motif::button(ui, t("patients.edit")).clicked() {
                self.form = Some(Form::of(&p));
            }
            if motif::button(ui, t("patients.check")).clicked() {
                cx.go(Go::Check(pid));
            }
            if motif::button(ui, t("patients.print_plan")).clicked() {
                match crate::print::plan(cx.s, pid) {
                    Ok(_) => cx.s.say(t("print.opened")),
                    Err(e) => cx.s.fail(e),
                }
            }
            if !p.pregnancy_ddr.is_empty() {
                motif::badge(
                    ui,
                    &tf("patients.pregnant_since", &[&ui::date(&p.pregnancy_ddr)]),
                    Some(motif::Pict::Warn),
                    motif::warn(),
                    false,
                );
            }
        });
        ui::fact(ui, t("patients.physician"), &p.physician);
        if !p.notes.trim().is_empty() {
            ui.add_space(ui::line(ui) * 0.3);
            ui.horizontal_wrapped(|ui| {
                ui::pict(ui, motif::Pict::Warn, motif::warn());
                ui.label(egui::RichText::new(&p.notes).strong());
            });
        }

        self.treatments_ui(ui, cx, pid);
        self.alerts_ui(ui, cx);
        self.bio_ui(ui, cx, pid);
        self.interviews_ui(ui, cx);
        self.notes_ui(ui, cx, pid);
    }

    fn treatments_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, pid: i64) {
        let Some(f) = self.file.as_ref() else { return };
        ui::heading(ui, &tn("patients.treatments", f.drugs.len()));
        if f.drugs.is_empty() {
            ui::empty(ui, t("patients.no_treatment"));
        }
        let dose_w = ui::chars_wide(ui, 10.0);
        let poso_w = ui::chars_wide(ui, 22.0);
        let gap = ui.spacing().item_spacing.x;
        let remove_w = ui::button_width(ui, t("patients.remove"));
        let avail = ui.available_width();
        // One row per treatment when everything fits beside a readable
        // name; otherwise the name on its own line, the fields under it.
        let one_row = avail - dose_w - poso_w - remove_w - gap * 4.0 >= ui::chars_wide(ui, 12.0);
        let (name_w, poso_w) = if one_row {
            (avail - dose_w - poso_w - remove_w - gap * 4.0, poso_w)
        } else {
            (
                avail,
                (avail - dose_w - remove_w - gap * 3.0).max(ui::chars_wide(ui, 8.0)),
            )
        };
        let mut writes: Vec<(i64, bool, String, String)> = Vec::new();
        let mut remove: Option<i64> = None;
        let mut open_drug: Option<i64> = None;
        for d in &f.drugs {
            let dosage = f.dosage.get(&d.id).cloned().unwrap_or_default();
            let posology = f.posology.get(&d.id).cloned().unwrap_or_default();
            if !one_row {
                let r = ui::cell_text(ui, egui::RichText::new(&d.name).strong(), name_w);
                if r.clicked() {
                    open_drug = Some(d.id);
                }
            }
            ui.horizontal(|ui| {
                if one_row {
                    let r = ui::cell_text(ui, egui::RichText::new(&d.name).strong(), name_w);
                    if r.clicked() {
                        open_drug = Some(d.id);
                    }
                }
                if let Some(v) = self.typing.cell(
                    ui,
                    ("dose", pid, d.id),
                    &dosage,
                    dose_w,
                    t("patients.dosage"),
                ) {
                    writes.push((d.id, true, v, dosage.clone()));
                }
                if let Some(v) = self.typing.cell(
                    ui,
                    ("poso", pid, d.id),
                    &posology,
                    poso_w,
                    t("patients.posology"),
                ) {
                    writes.push((d.id, false, v, posology.clone()));
                }
                if motif::button(ui, t("patients.remove")).clicked() {
                    remove = Some(d.id);
                }
            });
        }
        for (did, is_dose, new, old) in writes {
            let done = if is_dose {
                cx.s.db.set_patient_dosage(pid, did, &new, &old)
            } else {
                cx.s.db.set_patient_posology(pid, did, &new, &old)
            };
            match done {
                Ok(true) => cx.s.changed(),
                Ok(false) => cx.s.stale(),
                Err(e) => cx.s.fail(e),
            }
        }
        if let Some(did) = remove {
            self.remove_drug(cx, pid, did);
        }
        if let Some(did) = open_drug {
            cx.go(Go::Drug(did));
        }

        // Add a treatment: type, pick.
        ui.add_space(ui::line(ui) * 0.3);
        let r = ui::field(ui, &mut self.add_drug, 60.0, t("patients.add_drug"));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !self.add_drug.trim().is_empty() {
            let have: Vec<i64> = self
                .file
                .as_ref()
                .map(|f| f.drugs.iter().map(|d| d.id).collect())
                .unwrap_or_default();
            let found = super::medicaments::matching(&cx.s.drugs, &self.add_drug, 6, &have);
            let mut pick = if enter { found.first().copied() } else { None };
            for &i in &found {
                let d = &cx.s.drugs[i];
                let text = if d.dci.is_empty() {
                    d.name.clone()
                } else {
                    format!("{}  ({})", d.name, d.dci)
                };
                if motif::list_row(ui, egui::RichText::new(text), false).clicked() {
                    pick = Some(i);
                }
            }
            if found.is_empty() {
                ui::empty(ui, t("patients.no_drug_match"));
            }
            if let Some(i) = pick {
                let did = cx.s.drugs[i].id;
                match cx.s.db.add_patient_drug(pid, did) {
                    Ok(()) => {
                        self.add_drug.clear();
                        cx.s.changed();
                        r.request_focus();
                    }
                    Err(e) => cx.s.fail(e),
                }
            }
        }
    }

    fn remove_drug(&mut self, cx: &mut Ctx, pid: i64, did: i64) {
        let Some(f) = self.file.as_ref() else { return };
        let poso = f.posology.get(&did).cloned().unwrap_or_default();
        let dose = f.dosage.get(&did).cloned().unwrap_or_default();
        let script =
            cx.s.db
                .patient_prescriptions(pid)
                .unwrap_or_default()
                .into_iter()
                .find(|(d, _)| *d == did)
                .map(|(_, p)| p)
                .unwrap_or_default();
        match cx.s.db.remove_patient_drug(pid, did, &poso, &dose, &script) {
            Ok(true) => {
                cx.s.say(t("patients.removed"));
                cx.s.changed();
            }
            Ok(false) => cx.s.stale(),
            Err(e) => cx.s.fail(e),
        }
    }

    fn alerts_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let Some(f) = self.file.as_ref() else { return };
        if f.drugs.is_empty() {
            return;
        }
        ui::heading(ui, t("patients.alerts"));
        let ctx = context_of(f, &cx.s.today, (self.crush, self.breastfeeding, self.liver));
        ui.horizontal_wrapped(|ui| {
            let dfg = match analyse::latest_dfg(&f.bio, &cx.s.today) {
                Some((v, bound, day)) => tf(
                    "patients.dfg_of",
                    &[
                        &format!("{bound}{}", super::verifier::dfg_text(v)),
                        &ui::date(&day),
                    ],
                ),
                None => t("patients.no_dfg").to_owned(),
            };
            ui.label(motif::hint(dfg));
            if let Some(a) = ctx.age {
                ui.label(motif::hint(tf("patients.age_long", &[&a])));
            }
        });
        ui.horizontal_wrapped(|ui| {
            motif::checkbox(ui, &mut self.crush, t("verifier.crush"));
            motif::checkbox(ui, &mut self.breastfeeding, t("verifier.breastfeeding"));
            super::verifier::liver_select(ui, "patient_liver", &mut self.liver);
        });
        self.reckon(cx);
        if let Some(f) = self.file.as_ref() {
            super::verifier::show_lines(ui, &f.lines);
        }
    }

    fn bio_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, pid: i64) {
        let Some(f) = self.file.as_ref() else { return };
        ui::heading(ui, t("patients.bio"));
        let mut bio = f.bio.clone();
        bio.sort_by(|a, b| b.taken_on.cmp(&a.taken_on));
        for r in bio.iter().take(8) {
            let label = if r.label.is_empty() {
                &r.code
            } else {
                &r.label
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(label).color(motif::text_dim()));
                ui.label(
                    egui::RichText::new(format!(
                        "{}{} {}",
                        bpm_caddy::biology::bound_sign(r.value, &r.remark),
                        super::verifier::dfg_text(r.value),
                        r.unit
                    ))
                    .strong(),
                );
                ui.label(motif::hint(ui::date(&r.taken_on)));
            });
        }
        if bio.is_empty() {
            ui::empty(ui, t("patients.no_bio"));
        }
        ui.horizontal_wrapped(|ui| {
            ui::field(ui, &mut self.dfg, 8.0, t("patients.dfg_value"));
            ui::field(ui, &mut self.dfg_day, 12.0, t("patients.dfg_day"));
            if motif::button(ui, t("patients.dfg_add")).clicked() {
                match parse_dfg(
                    &self.dfg,
                    &self.dfg_day,
                    &cx.s.today,
                    cx.s.db.current_year(),
                ) {
                    Ok((value, day, remark)) => {
                        let a = bpm_caddy::biology::find("DFG");
                        let r = BioResult {
                            code: "DFG".into(),
                            label: a.map(|a| a.label.to_owned()).unwrap_or_default(),
                            unit: a.map(|a| a.unit.to_owned()).unwrap_or_default(),
                            value,
                            taken_on: day,
                            remark,
                            ..Default::default()
                        };
                        match cx.s.db.add_bio_result(pid, &r) {
                            Ok(_) => {
                                self.dfg.clear();
                                self.dfg_day.clear();
                                cx.s.changed();
                            }
                            Err(e) => cx.s.fail(e),
                        }
                    }
                    Err(e) => cx.s.fail(e),
                }
            }
        });
    }

    fn interviews_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let Some(f) = self.file.as_ref() else { return };
        if f.interviews.is_empty() {
            return;
        }
        ui::heading(ui, t("patients.interviews"));
        let today = cx.s.today.clone();
        for i in f.interviews.iter().take(6) {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(i.kind.label()).strong());
                ui.label(i.state.label());
                if let Some(d) = &i.scheduled_date {
                    let ink = if d.as_str() < today.as_str() {
                        motif::text_dim()
                    } else {
                        motif::text()
                    };
                    ui.label(
                        egui::RichText::new(format!("{} {}", ui::date(d), i.scheduled_time))
                            .color(ink),
                    );
                }
            });
        }
    }

    fn notes_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, pid: i64) {
        let Some(f) = self.file.as_ref() else { return };
        ui::heading(ui, t("patients.notes"));
        let notes = f.notes.clone();
        if ui::field_and_button(
            ui,
            &mut self.note,
            t("patients.note_hint"),
            t("patients.note_add"),
        ) {
            let who = cx.s.operator.clone();
            match cx
                .s
                .db
                .add_note(NoteSubject::Patient, pid, &who, self.note.trim())
            {
                Ok(_) => {
                    self.note.clear();
                    cx.s.changed();
                }
                Err(e) => cx.s.fail(e),
            }
        }
        for n in &notes {
            ui.horizontal_wrapped(|ui| {
                ui.label(motif::hint(format!("{} {}", n.stamp(), n.operator)));
                ui.label(&n.body);
            });
        }
    }

    fn form_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let new = form.shown.is_none();
        ui::title(
            ui,
            if new {
                t("patients.new_title")
            } else {
                t("patients.edit_title")
            },
            None,
        );
        let lw = 16.0;
        ui::form_row(ui, t("patients.last"), lw, &mut form.last, 28.0, "");
        ui::form_row(ui, t("patients.first"), lw, &mut form.first, 28.0, "");
        ui::form_row(
            ui,
            t("patients.birth"),
            lw,
            &mut form.birth,
            12.0,
            t("patients.birth_hint"),
        );
        ui.horizontal(|ui| {
            ui::cell_text(
                ui,
                egui::RichText::new(t("patients.sex")).color(motif::text_dim()),
                ui::chars_wide(ui, lw),
            );
            for (key, label) in [
                ("", t("patients.sex_none")),
                ("F", t("patients.sex_f")),
                ("M", t("patients.sex_m")),
            ] {
                if motif::radio(ui, form.sex == key, label).clicked() {
                    form.sex = key.to_owned();
                }
            }
        });
        ui::form_row(ui, t("patients.phone"), lw, &mut form.phone, 18.0, "");
        ui::form_row(
            ui,
            t("patients.physician"),
            lw,
            &mut form.physician,
            28.0,
            "",
        );
        ui::form_row(
            ui,
            t("patients.counter_note"),
            lw,
            &mut form.notes,
            40.0,
            t("patients.counter_note_hint"),
        );
        if let Some(e) = &form.error {
            ui.label(egui::RichText::new(e).color(motif::alert()));
        }
        let mut save = false;
        let mut cancel = false;
        let mut delete = false;
        ui.add_space(ui::line(ui) * 0.5);
        ui.horizontal(|ui| {
            save = motif::button(ui, t("app.save")).clicked();
            cancel = motif::button(ui, t("app.cancel")).clicked();
            if !new {
                ui.add_space(ui::chars_wide(ui, 4.0));
                if self.confirm_delete {
                    ui.label(egui::RichText::new(t("patients.delete_sure")).color(motif::alert()));
                    delete = motif::button(ui, t("patients.delete_yes")).clicked();
                } else if motif::button(ui, t("patients.delete")).clicked() {
                    self.confirm_delete = true;
                }
            }
        });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if cancel {
            self.form = None;
            self.confirm_delete = false;
            return;
        }
        if delete {
            self.delete(cx);
            return;
        }
        if save {
            self.save(cx);
        }
    }

    fn save(&mut self, cx: &mut Ctx) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        if form.last.trim().is_empty() {
            form.error = Some(t("patients.need_name").to_owned());
            return;
        }
        let birth = if form.birth.trim().is_empty() {
            String::new()
        } else {
            match db::parse_french_date(&form.birth, cx.s.db.current_year(), db::YearHint::Past) {
                Ok(d) => d,
                Err(e) => {
                    form.error = Some(e);
                    return;
                }
            }
        };
        let base = match &form.shown {
            Some(p) => p.clone(),
            None => {
                match cx.s.db.add_patient_with_sex(
                    form.last.trim(),
                    form.first.trim(),
                    &birth,
                    &form.sex,
                ) {
                    Ok(id) => match cx.s.db.patients() {
                        Ok(all) => match all.into_iter().find(|p| p.id == id) {
                            Some(p) => p,
                            None => return,
                        },
                        Err(e) => {
                            form.error = Some(e);
                            return;
                        }
                    },
                    Err(e) => {
                        form.error = Some(e);
                        return;
                    }
                }
            }
        };
        let new = Patient {
            last_name: form.last.trim().to_owned(),
            first_name: form.first.trim().to_owned(),
            birth_date: birth,
            sex: form.sex.clone(),
            phone: form.phone.trim().to_owned(),
            physician: form.physician.trim().to_owned(),
            notes: form.notes.trim().to_owned(),
            ..base.clone()
        };
        let id = base.id;
        if new == base {
            self.form = None;
            self.open(id);
            cx.s.changed();
            return;
        }
        match cx.s.db.update_patient(&new, &base) {
            Ok(true) => {
                self.form = None;
                cx.s.say(t("patients.saved"));
                cx.s.changed();
                self.open = None;
                self.open(id);
            }
            Ok(false) => {
                self.form = None;
                cx.s.stale();
            }
            Err(e) => {
                if let Some(form) = self.form.as_mut() {
                    form.error = Some(e);
                }
            }
        }
    }

    fn delete(&mut self, cx: &mut Ctx) {
        let Some(shown) = self.form.as_ref().and_then(|f| f.shown.clone()) else {
            return;
        };
        if cx.s.db.register_cites(shown.id) {
            cx.s.fail(t("patients.delete_register"));
            self.confirm_delete = false;
            return;
        }
        match cx.s.db.delete_patient(&shown) {
            Ok(true) => {
                self.form = None;
                self.open = None;
                self.file = None;
                self.confirm_delete = false;
                cx.s.say(t("patients.deleted"));
                cx.s.changed();
            }
            Ok(false) => cx.s.stale(),
            Err(e) => cx.s.fail(e),
        }
    }
}

/// The file's figures as the tables read them.
fn context_of(
    f: &File,
    today: &str,
    (crush, breastfeeding, liver): (bool, bool, Option<bpm_caddy::hepatic::Stage>),
) -> Context {
    let dfg = analyse::latest_dfg(&f.bio, today);
    Context {
        dfg: dfg.as_ref().map(|d| d.0),
        dfg_bound: dfg.map(|d| d.1).unwrap_or_default(),
        bio: f.bio.clone(),
        liver,
        age: crate::session::age_on(&f.patient.birth_date, today),
        pregnant: !f.patient.pregnancy_ddr.is_empty(),
        breastfeeding,
        crush,
    }
}

/// Indices of the files matching `filter`, best first; every file, in
/// alphabetical order, when the filter is empty.
pub fn matching(patients: &[Patient], filter: &str) -> Vec<usize> {
    let q = filter.trim();
    if q.is_empty() {
        return (0..patients.len()).collect();
    }
    let mut scored: Vec<(i32, usize)> = patients
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let hay = format!(
                "{} {} {} {}",
                p.last_name,
                p.first_name,
                db::format_french_date(&p.birth_date),
                p.phone
            );
            bpm_caddy::fuzzy::score(q, &hay).map(|s| (s, i))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, i)| i).collect()
}

/// A typed DFG: « 28 », « <15 », « 42,5 », and its day (empty = today).
/// Returns the value, the ISO day and the remark that carries the bound.
pub fn parse_dfg(
    value: &str,
    day: &str,
    today: &str,
    year: u32,
) -> Result<(f64, String, String), String> {
    let v = value.trim();
    let (bound, rest) = match v.chars().next() {
        Some(c @ ('<' | '>')) => (c.to_string(), v[1..].trim()),
        _ => (String::new(), v),
    };
    let number: f64 = rest
        .replace(',', ".")
        .parse()
        .map_err(|_| t("patients.dfg_bad").to_owned())?;
    if !(number.is_finite() && (0.0..=300.0).contains(&number)) {
        return Err(t("patients.dfg_bad").to_owned());
    }
    let day = if day.trim().is_empty() {
        today.to_owned()
    } else {
        db::parse_french_date(day, year, db::YearHint::Past)?
    };
    if day.as_str() > today {
        return Err(t("patients.dfg_future").to_owned());
    }
    let remark = if bound.is_empty() {
        String::new()
    } else {
        format!("{bound}{rest}")
    };
    Ok((number, day, remark))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(last: &str, first: &str) -> Patient {
        Patient {
            last_name: last.into(),
            first_name: first.into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_filter_finds_by_name_without_accents() {
        let all = [p("Lefèvre", "Hélène"), p("Dupont", "Jean")];
        assert_eq!(matching(&all, ""), vec![0, 1]);
        assert_eq!(matching(&all, "lefevre"), vec![0]);
        assert_eq!(matching(&all, "dup"), vec![1]);
    }

    #[test]
    fn a_typed_dfg_keeps_its_bound_and_refuses_nonsense() {
        let (v, d, r) = parse_dfg("<15", "", "2026-10-31", 2026).unwrap();
        assert_eq!((v, d.as_str(), r.as_str()), (15.0, "2026-10-31", "<15"));
        assert_eq!(
            bpm_caddy::biology::bound_sign(v, &r),
            "<",
            "the remark must read back as a bound"
        );
        let (v, _, r) = parse_dfg("42,5", "01/10/2026", "2026-10-31", 2026).unwrap();
        assert_eq!((v, r.as_str()), (42.5, ""));
        assert!(parse_dfg("abc", "", "2026-10-31", 2026).is_err());
        assert!(parse_dfg("40", "01/12/2026", "2026-10-31", 2026).is_err());
    }

    #[test]
    fn a_file_saves_and_its_treatments_come_back() {
        let (mut s, dir) = crate::session::tests::scratch("pat");
        let mut go = None;
        let mut screen = Patients::default();
        screen.new_file();
        if let Some(f) = screen.form.as_mut() {
            f.last = "Durand".into();
            f.first = "Anne".into();
            f.birth = "030758".into();
        }
        {
            let mut cx = Ctx {
                s: &mut s,
                go: &mut go,
            };
            screen.save(&mut cx);
        }
        let saved = s.patients.iter().find(|p| p.last_name == "Durand").unwrap();
        assert_eq!(saved.birth_date, "1958-07-03");
        assert_eq!(screen.open, Some(saved.id));
        let pid = saved.id;
        let did = s.drugs[0].id;
        s.db.add_patient_drug(pid, did).unwrap();
        s.changed();
        {
            let mut cx = Ctx {
                s: &mut s,
                go: &mut go,
            };
            screen.load(&mut cx);
        }
        assert_eq!(screen.file.as_ref().unwrap().drugs.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
