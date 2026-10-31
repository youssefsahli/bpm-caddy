//! Accueil: today. The rendez-vous of the day, what the team left for
//! each other, and the three things the counter starts from.

use super::{Ctx, Go, Screen};
use crate::t::{t, tn};
use crate::ui;
use bpm_caddy::db::{Appointment, Event, Note, NoteSubject};
use eframe::egui;

#[derive(Default)]
pub struct Accueil {
    memo: Option<(u64, String, Today)>,
    handover: String,
}

#[derive(Clone, Default)]
pub struct Today {
    pub appointments: Vec<Appointment>,
    /// Rendez-vous of past days still not held.
    pub overdue: usize,
    pub events: Vec<Event>,
    pub handover: Vec<Note>,
    pub low_stups: usize,
}

pub fn load_today(db: &bpm_caddy::db::Db, today: &str) -> Today {
    let all = db.upcoming_appointments().unwrap_or_default();
    let overdue = all.iter().filter(|a| a.date.as_str() < today).count();
    let appointments = all.into_iter().filter(|a| a.date == today).collect();
    let standing = db.stup_summary().unwrap_or_default();
    Today {
        appointments,
        overdue,
        events: db.events_between(today, today).unwrap_or_default(),
        handover: db.transmissions_for_day(today).unwrap_or_default(),
        low_stups: super::stups::low(&standing).len(),
    }
}

impl Accueil {
    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let fresh = self
            .memo
            .as_ref()
            .is_some_and(|(g, d, _)| *g == cx.s.generation && *d == cx.s.today);
        if !fresh {
            let today = load_today(&cx.s.db, &cx.s.today);
            self.memo = Some((cx.s.generation, cx.s.today.clone(), today));
        }
        let Some((_, _, today)) = self.memo.clone() else {
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt("accueil")
            .auto_shrink([false, false])
            .show(ui, |ui| self.body(ui, cx, &today));
    }

    fn body(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, today: &Today) {
        ui::title(ui, &ui::day_long(&cx.s.today), Some(t("accueil.sub")));
        ui.horizontal_wrapped(|ui| {
            if motif::icon_button(ui, Some(motif::Pict::Register), t("accueil.find_patient"))
                .clicked()
            {
                cx.go(Go::Screen(Screen::Patients));
            }
            if motif::icon_button(ui, Some(motif::Pict::Pen), t("accueil.new_patient")).clicked() {
                cx.go(Go::NewPatient);
            }
            if motif::icon_button(ui, Some(motif::Pict::Check), t("accueil.check")).clicked() {
                cx.go(Go::Screen(Screen::Verifier));
            }
            if motif::icon_button(ui, Some(motif::Pict::Pill), t("accueil.drug")).clicked() {
                cx.go(Go::Screen(Screen::Medicaments));
            }
        });
        if today.low_stups > 0 || today.overdue > 0 {
            ui.add_space(ui::line(ui) * 0.3);
            ui.horizontal_wrapped(|ui| {
                if today.low_stups > 0
                    && motif::badge(
                        ui,
                        &tn("accueil.low_stups", today.low_stups),
                        Some(motif::Pict::Warn),
                        motif::warn(),
                        false,
                    )
                    .clicked()
                {
                    cx.go(Go::Screen(Screen::Stups));
                }
                if today.overdue > 0
                    && motif::badge(
                        ui,
                        &tn("accueil.overdue", today.overdue),
                        Some(motif::Pict::Pending),
                        motif::bg_light(),
                        false,
                    )
                    .clicked()
                {
                    cx.go(Go::Screen(Screen::Agenda));
                }
            });
        }

        ui::heading(ui, &tn("accueil.rdv", today.appointments.len()));
        if today.appointments.is_empty() && today.events.is_empty() {
            ui::empty(ui, t("accueil.no_rdv"));
        }
        let mut open = None;
        for a in &today.appointments {
            ui.horizontal_wrapped(|ui| {
                let when = if a.time.is_empty() {
                    t("agenda.no_time").to_owned()
                } else {
                    a.time.clone()
                };
                ui.add_sized(
                    [ui::chars_wide(ui, 9.0), ui::line(ui)],
                    egui::Label::new(egui::RichText::new(when).color(motif::text_dim())),
                );
                if motif::button(ui, &a.patient_name).clicked() {
                    open = Some(a.patient_id);
                }
                ui.label(a.kind.label());
                if !a.operator.is_empty() {
                    ui.label(motif::hint(&a.operator));
                }
            });
        }
        for e in &today.events {
            ui.horizontal_wrapped(|ui| {
                let when = if e.time.is_empty() {
                    t("agenda.no_time").to_owned()
                } else {
                    e.time.clone()
                };
                ui.add_sized(
                    [ui::chars_wide(ui, 9.0), ui::line(ui)],
                    egui::Label::new(egui::RichText::new(when).color(motif::text_dim())),
                );
                ui.label(egui::RichText::new(&e.title).strong());
                ui.label(motif::hint(e.category.label()));
            });
        }
        if let Some(pid) = open {
            cx.go(Go::Patient(pid));
        }

        ui::heading(ui, t("accueil.handover"));
        if ui::field_and_button(
            ui,
            &mut self.handover,
            t("accueil.handover_hint"),
            t("accueil.handover_add"),
        ) {
            let who = cx.s.operator.clone();
            match cx
                .s
                .db
                .add_note(NoteSubject::Transmission, 0, &who, self.handover.trim())
            {
                Ok(_) => {
                    self.handover.clear();
                    cx.s.changed();
                }
                Err(e) => cx.s.fail(e),
            }
        }
        if today.handover.is_empty() {
            ui::empty(ui, t("accueil.no_handover"));
        }
        for n in &today.handover {
            ui.horizontal_wrapped(|ui| {
                ui.label(motif::hint(format!("{} {}", n.stamp(), n.operator)));
                ui.label(&n.body);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handover_note_shows_today() {
        let (s, dir) = crate::session::tests::scratch("accueil");
        s.db.add_note(NoteSubject::Transmission, 0, "AB", "Commande en retard.")
            .unwrap();
        let today = load_today(&s.db, &s.today);
        assert_eq!(today.handover.len(), 1);
        assert_eq!(today.appointments.len(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
