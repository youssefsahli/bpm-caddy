//! Agenda: one week, a column of days, each day its rendez-vous and its
//! events. Booking is one row of fields under the week.

use super::{Ctx, Go};
use crate::t::{t, tf};
use crate::ui;
use bpm_caddy::db::{self, Appointment, Event, EventCategory, InterviewKind};
use eframe::egui;

#[derive(Default)]
pub struct Agenda {
    /// Weeks from the current one.
    offset: i64,
    memo: Option<(u64, i64, String, Week)>,
    // Booking a patient.
    who: String,
    picked: Option<i64>,
    kind: Option<InterviewKind>,
    day: String,
    time: String,
    // An event.
    ev_title: String,
    ev_day: String,
    ev_time: String,
    ev_cat: Option<EventCategory>,
    error: Option<String>,
}

#[derive(Clone, Default)]
pub struct Week {
    pub days: Vec<String>,
    pub appointments: Vec<Appointment>,
    pub events: Vec<Event>,
}

/// The seven ISO days of the week holding `today`, shifted by `offset`
/// weeks; Monday first.
pub fn week_of(today: &str, offset: i64) -> Vec<String> {
    let Some(wd) = bpm_caddy::date::weekday(today) else {
        return Vec::new();
    };
    let monday = bpm_caddy::date::add_days(today, 1 - wd + 7 * offset).unwrap_or_default();
    (0..7)
        .filter_map(|k| bpm_caddy::date::add_days(&monday, k))
        .collect()
}

/// What the base holds for those days.
pub fn load_week(db: &db::Db, days: &[String]) -> Week {
    let (Some(first), Some(last)) = (days.first(), days.last()) else {
        return Week::default();
    };
    let appointments = db
        .upcoming_appointments()
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a.date.as_str() >= first.as_str() && a.date.as_str() <= last.as_str())
        .collect();
    Week {
        days: days.to_vec(),
        appointments,
        events: db.events_between(first, last).unwrap_or_default(),
    }
}

impl Agenda {
    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let fresh = self.memo.as_ref().is_some_and(|(g, o, d, _)| {
            *g == cx.s.generation && *o == self.offset && *d == cx.s.today
        });
        if !fresh {
            let days = week_of(&cx.s.today, self.offset);
            let w = load_week(&cx.s.db, &days);
            self.memo = Some((cx.s.generation, self.offset, cx.s.today.clone(), w));
        }
        let Some((_, _, _, week)) = self.memo.clone() else {
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt("agenda")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let sub = match (week.days.first(), week.days.last()) {
                    (Some(a), Some(b)) => tf("agenda.week", &[&ui::date(a), &ui::date(b)]),
                    _ => String::new(),
                };
                ui::title(ui, t("agenda.title"), Some(&sub));
                ui.horizontal(|ui| {
                    if motif::button(ui, t("agenda.prev")).clicked() {
                        self.offset -= 1;
                    }
                    if motif::button_enabled(ui, t("agenda.this_week"), self.offset != 0).clicked()
                    {
                        self.offset = 0;
                    }
                    if motif::button(ui, t("agenda.next")).clicked() {
                        self.offset += 1;
                    }
                });
                self.week_ui(ui, cx, &week);
                self.book_ui(ui, cx);
                self.event_ui(ui, cx);
            });
    }

    fn week_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx, week: &Week) {
        let mut open = None;
        let mut delete: Option<(i64, String)> = None;
        for day in &week.days {
            let past = day.as_str() < cx.s.today.as_str();
            let empty = !week.appointments.iter().any(|a| &a.date == day)
                && !week.events.iter().any(|e| &e.day == day);
            let ink = if *day == cx.s.today {
                motif::accent()
            } else if past || empty {
                motif::text_dim()
            } else {
                motif::text()
            };
            motif::section_ink(ui, &ui::day_long(day), ink);
            let mut rows: Vec<(String, Row)> = Vec::new();
            for a in week.appointments.iter().filter(|a| &a.date == day) {
                rows.push((a.time.clone(), Row::Rdv(a)));
            }
            for e in week.events.iter().filter(|e| &e.day == day) {
                rows.push((e.time.clone(), Row::Event(e)));
            }
            // Timed first, by hour; the untimed after.
            rows.sort_by(|a, b| (a.0.is_empty(), &a.0).cmp(&(b.0.is_empty(), &b.0)));
            for (time, row) in rows {
                ui.horizontal_wrapped(|ui| {
                    let when = if time.is_empty() {
                        t("agenda.no_time").to_owned()
                    } else {
                        time.clone()
                    };
                    ui.add_sized(
                        [ui::chars_wide(ui, 9.0), ui::line(ui)],
                        egui::Label::new(egui::RichText::new(when).color(motif::text_dim())),
                    );
                    match row {
                        Row::Rdv(a) => {
                            let r = ui.add(
                                egui::Label::new(egui::RichText::new(&a.patient_name).strong())
                                    .sense(egui::Sense::click()),
                            );
                            if r.clicked() {
                                open = Some(a.patient_id);
                            }
                            ui.label(a.kind.label());
                            if a.remote {
                                ui.label(motif::hint(t("agenda.remote")));
                            }
                            if !a.phone.is_empty() {
                                ui.label(motif::hint(&a.phone));
                            }
                        }
                        Row::Event(e) => {
                            ui.label(egui::RichText::new(&e.title).strong());
                            ui.label(motif::hint(e.category.label()));
                            if e.repeat_days == 0 && motif::button(ui, t("agenda.delete")).clicked()
                            {
                                delete = Some((e.source_id, e.title.clone()));
                            }
                        }
                    }
                });
            }
        }
        if let Some(pid) = open {
            cx.go(Go::Patient(pid));
        }
        if let Some((id, title)) = delete {
            match cx.s.db.delete_event(id, &title) {
                Ok(true) => cx.s.changed(),
                Ok(false) => cx.s.stale(),
                Err(e) => cx.s.fail(e),
            }
        }
    }

    fn book_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        ui::heading(ui, t("agenda.book"));
        ui.horizontal_wrapped(|ui| {
            let r = ui::field(ui, &mut self.who, 22.0, t("agenda.who"));
            if r.changed() {
                self.picked = None;
            }
            let mut options = vec![(None, t("agenda.kind").to_owned())];
            options.extend(
                InterviewKind::ALL
                    .iter()
                    .map(|k| (Some(*k), k.label().to_owned())),
            );
            let w = motif::select_width(ui, options.iter().map(|(_, l)| l.as_str()));
            motif::select(ui, "agenda_kind", w, &mut self.kind, &options);
            ui::field(ui, &mut self.day, 11.0, t("agenda.day_hint"));
            ui::field(ui, &mut self.time, 6.0, t("agenda.time_hint"));
            if motif::button(ui, t("agenda.book_do")).clicked() {
                self.book(cx);
            }
        });
        if self.picked.is_none() && !self.who.trim().is_empty() {
            let found = super::patients::matching(&cx.s.patients, &self.who);
            for &i in found.iter().take(5) {
                let p = &cx.s.patients[i];
                let text = format!(
                    "{} {}  {}",
                    p.last_name.to_uppercase(),
                    p.first_name,
                    ui::date(&p.birth_date)
                );
                if motif::list_row(ui, egui::RichText::new(text), false).clicked() {
                    self.picked = Some(p.id);
                    self.who = format!("{} {}", p.last_name.to_uppercase(), p.first_name);
                }
            }
        }
        if let Some(e) = &self.error {
            ui.label(egui::RichText::new(e).color(motif::alert()));
        }
    }

    fn book(&mut self, cx: &mut Ctx) {
        self.error = None;
        let Some(pid) = self.picked else {
            self.error = Some(t("agenda.need_patient").to_owned());
            return;
        };
        let Some(kind) = self.kind else {
            self.error = Some(t("agenda.need_kind").to_owned());
            return;
        };
        let day =
            match db::parse_french_date(&self.day, cx.s.db.current_year(), db::YearHint::Future) {
                Ok(d) => d,
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            };
        let time = match normalise_time(&self.time) {
            Some(t) => t,
            None => {
                self.error = Some(t("agenda.bad_time").to_owned());
                return;
            }
        };
        let who = cx.s.operator.clone();
        let done =
            cx.s.db
                .add_interview_by(pid, kind, "", &who)
                .and_then(|id| {
                    cx.s.db.set_scheduled_date(id, Some(&day), None)?;
                    if !time.is_empty() {
                        cx.s.db.set_scheduled_time(id, &time, "")?;
                    }
                    Ok(())
                });
        match done {
            Ok(()) => {
                cx.s.say(tf("agenda.booked", &[&self.who, &ui::date(&day)]));
                self.who.clear();
                self.picked = None;
                self.time.clear();
                cx.s.changed();
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn event_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        ui::heading(ui, t("agenda.event"));
        ui.horizontal_wrapped(|ui| {
            ui::field(ui, &mut self.ev_title, 24.0, t("agenda.event_title"));
            let mut options = vec![(None, t("agenda.category").to_owned())];
            options.extend(
                EventCategory::ALL
                    .iter()
                    .map(|c| (Some(*c), c.label().to_owned())),
            );
            let w = motif::select_width(ui, options.iter().map(|(_, l)| l.as_str()));
            motif::select(ui, "agenda_cat", w, &mut self.ev_cat, &options);
            ui::field(ui, &mut self.ev_day, 11.0, t("agenda.day_hint"));
            ui::field(ui, &mut self.ev_time, 6.0, t("agenda.time_hint"));
            if motif::button(ui, t("agenda.event_add")).clicked() {
                self.add_event(cx);
            }
        });
    }

    fn add_event(&mut self, cx: &mut Ctx) {
        self.error = None;
        if self.ev_title.trim().is_empty() {
            self.error = Some(t("agenda.need_title").to_owned());
            return;
        }
        let day = if self.ev_day.trim().is_empty() {
            cx.s.today.clone()
        } else {
            match db::parse_french_date(&self.ev_day, cx.s.db.current_year(), db::YearHint::Future)
            {
                Ok(d) => d,
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        };
        let Some(time) = normalise_time(&self.ev_time) else {
            self.error = Some(t("agenda.bad_time").to_owned());
            return;
        };
        let cat = self.ev_cat.unwrap_or(EventCategory::Autre);
        match cx
            .s
            .db
            .add_event_span(&day, &time, "", self.ev_title.trim(), cat, "", "")
        {
            Ok(_) => {
                self.ev_title.clear();
                self.ev_time.clear();
                cx.s.changed();
            }
            Err(e) => self.error = Some(e),
        }
    }
}

enum Row<'a> {
    Rdv(&'a Appointment),
    Event(&'a Event),
}

/// « 9 », « 9h », « 9h30 », « 09:30 », « 930 » into `HH:MM`; empty stays
/// empty (no fixed hour). `None` when it is not an hour.
pub fn normalise_time(s: &str) -> Option<String> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Some(String::new());
    }
    let (h, m) = if let Some((h, m)) = s.split_once(['h', ':']) {
        (h.trim().to_owned(), m.trim().to_owned())
    } else if s.len() >= 3 && s.chars().all(|c| c.is_ascii_digit()) {
        let (h, m) = s.split_at(s.len() - 2);
        (h.to_owned(), m.to_owned())
    } else {
        (s.clone(), String::new())
    };
    let h: u32 = h.parse().ok()?;
    let m: u32 = if m.is_empty() { 0 } else { m.parse().ok()? };
    (h < 24 && m < 60).then(|| format!("{h:02}:{m:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_week_starts_on_monday() {
        // 2026-10-31 is a Saturday.
        let w = week_of("2026-10-31", 0);
        assert_eq!(w.len(), 7);
        assert_eq!(w[0], "2026-10-26");
        assert_eq!(w[6], "2026-11-01");
        assert_eq!(week_of("2026-10-31", 1)[0], "2026-11-02");
        assert_eq!(week_of("2026-10-26", 0)[0], "2026-10-26");
    }

    #[test]
    fn hours_are_typed_the_way_people_say_them() {
        assert_eq!(normalise_time("9").as_deref(), Some("09:00"));
        assert_eq!(normalise_time("9h30").as_deref(), Some("09:30"));
        assert_eq!(normalise_time("14:05").as_deref(), Some("14:05"));
        assert_eq!(normalise_time("930").as_deref(), Some("09:30"));
        assert_eq!(normalise_time("1415").as_deref(), Some("14:15"));
        assert_eq!(normalise_time("").as_deref(), Some(""));
        assert_eq!(normalise_time("25h"), None);
        assert_eq!(normalise_time("midi"), None);
    }

    #[test]
    fn a_booking_shows_in_its_week() {
        let (mut s, dir) = crate::session::tests::scratch("agenda");
        let pid =
            s.db.add_patient_with_sex("Durand", "Anne", "1950-01-01", "")
                .unwrap();
        s.changed();
        let mut a = Agenda {
            picked: Some(pid),
            who: "DURAND Anne".into(),
            kind: Some(InterviewKind::Bpm),
            day: bpm_caddy::db::format_french_date(&s.today),
            time: "10h".into(),
            ..Default::default()
        };
        let mut go = None;
        let mut cx = Ctx {
            s: &mut s,
            go: &mut go,
        };
        a.book(&mut cx);
        assert_eq!(a.error, None);
        let days = week_of(&cx.s.today, 0);
        let w = load_week(&cx.s.db, &days);
        assert_eq!(w.appointments.len(), 1);
        assert_eq!(w.appointments[0].time, "10:00");
        let _ = std::fs::remove_dir_all(dir);
    }
}
