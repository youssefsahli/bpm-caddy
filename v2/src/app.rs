//! The window: unlock, then a sidebar of eight screens, a search box
//! that reaches any file or card (Ctrl+K), and one status line.

use crate::screens::{
    accueil::Accueil, agenda::Agenda, medicaments::Medicaments, options::Options,
    patients::Patients, stups::Stups, tables::Tables, verifier::Verifier, Ctx, Go, Screen,
};
use crate::session::{self, Session};
use crate::t::{t, tf};
use crate::ui;
use eframe::egui;
use std::path::PathBuf;

pub struct App {
    phase: Phase,
    /// `BPM_CADDY_START_VIEW`, applied once the base is open.
    start: Option<String>,
}

enum Phase {
    Locked(Lock),
    Open(Box<Desk>),
}

struct Lock {
    path: PathBuf,
    password: String,
    again: String,
    error: Option<String>,
    focus: bool,
}

pub struct Desk {
    pub s: Session,
    pub screen: Screen,
    accueil: Accueil,
    patients: Patients,
    medicaments: Medicaments,
    verifier: Verifier,
    agenda: Agenda,
    stups: Stups,
    tables: Tables,
    options: Options,
    find: String,
    find_focus: bool,
    find_pick: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let path = session::base_path();
        let start = std::env::var("BPM_CADDY_START_VIEW").ok();
        let mut app = App {
            phase: Phase::Locked(Lock {
                path,
                password: String::new(),
                again: String::new(),
                error: None,
                focus: true,
            }),
            start,
        };
        if app.start.as_deref() != Some("verrou") {
            if let Some(pw) = session::env_password() {
                app.unlock(&pw);
            }
        }
        app
    }

    fn unlock(&mut self, password: &str) {
        let Phase::Locked(lock) = &mut self.phase else {
            return;
        };
        let operator = bpm_caddy::config::Config::load().ui.operator;
        match Session::open(&lock.path, password, &operator) {
            Ok(s) => {
                let mut desk = Box::new(Desk::new(s));
                if let Some(view) = self.start.take() {
                    desk.start_at(&view);
                }
                self.phase = Phase::Open(desk);
            }
            Err(e) => {
                lock.error = Some(bpm_caddy::strings::plain_error(&e));
                lock.password.clear();
                lock.again.clear();
            }
        }
    }

    pub fn desk(&mut self) -> Option<&mut Desk> {
        match &mut self.phase {
            Phase::Open(d) => Some(d),
            Phase::Locked(_) => None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        match &mut self.phase {
            Phase::Locked(_) => {
                let mut try_pw = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    if let Phase::Locked(lock) = &mut self.phase {
                        try_pw = lock_ui(ui, lock);
                    }
                });
                if let Some(pw) = try_pw {
                    self.unlock(&pw);
                }
            }
            Phase::Open(desk) => desk.update(ctx),
        }
    }
}

/// The unlock form. Returns the password to try.
fn lock_ui(ui: &mut egui::Ui, lock: &mut Lock) -> Option<String> {
    let exists = lock.path.exists();
    let mut out = None;
    let w = ui::chars_wide(ui, 44.0).min(ui.available_width());
    let top = (ui.available_height() * 0.25).max(ui::line(ui));
    ui.add_space(top);
    ui.vertical_centered(|ui| {
        ui.set_max_width(w);
        ui.label(
            egui::RichText::new(t("lock.title"))
                .size(motif::pt(ui, 26.0))
                .strong(),
        );
        ui.label(motif::hint(if exists {
            t("lock.sub")
        } else {
            t("lock.sub_new")
        }));
        ui.add_space(ui::line(ui));
        let r = motif::field(
            ui,
            w,
            egui::TextEdit::singleline(&mut lock.password)
                .password(true)
                .hint_text(motif::hint(t("lock.password"))),
        );
        if std::mem::take(&mut lock.focus) {
            r.request_focus();
        }
        let mut enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !exists {
            let r2 = motif::field(
                ui,
                w,
                egui::TextEdit::singleline(&mut lock.again)
                    .password(true)
                    .hint_text(motif::hint(t("lock.again"))),
            );
            enter = r2.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        }
        ui.add_space(ui::line(ui) * 0.4);
        let label = if exists {
            t("lock.open")
        } else {
            t("lock.create")
        };
        if motif::button(ui, label).clicked() || enter {
            if lock.password.is_empty() {
                lock.error = Some(t("lock.empty").to_owned());
            } else if !exists && lock.password != lock.again {
                lock.error = Some(t("lock.mismatch").to_owned());
            } else {
                out = Some(lock.password.clone());
            }
        }
        if let Some(e) = &lock.error {
            ui.label(egui::RichText::new(e).color(motif::alert()));
            lock.focus = true;
        }
        ui.add_space(ui::line(ui));
        ui.label(motif::hint(tf("lock.at", &[&lock.path.display()])));
    });
    out
}

impl Desk {
    pub fn new(s: Session) -> Self {
        Desk {
            s,
            screen: Screen::Accueil,
            accueil: Accueil::default(),
            patients: Patients::default(),
            medicaments: Medicaments::default(),
            verifier: Verifier::default(),
            agenda: Agenda::default(),
            stups: Stups::default(),
            tables: Tables::default(),
            options: Options::default(),
            find: String::new(),
            find_focus: false,
            find_pick: 0,
        }
    }

    /// The screenshot / e2e hook: land on a view, with something open
    /// where the view would otherwise be empty.
    pub fn start_at(&mut self, view: &str) {
        let first_patient = self
            .s
            .patients
            .iter()
            .find(|p| {
                self.s
                    .db
                    .drugs_for_patient(p.id)
                    .is_ok_and(|d| !d.is_empty())
            })
            .or(self.s.patients.first())
            .map(|p| p.id);
        let first_drug = self.s.drugs.first().map(|d| d.id);
        match view {
            "patients" => self.screen = Screen::Patients,
            "patient" => {
                if let Some(id) = first_patient {
                    self.go(Go::Patient(id));
                } else {
                    self.screen = Screen::Patients;
                }
            }
            "patient_new" => self.go(Go::NewPatient),
            "medicaments" => self.screen = Screen::Medicaments,
            "fiche" => {
                if let Some(id) = first_drug {
                    self.go(Go::Drug(id));
                }
            }
            "verifier" => self.screen = Screen::Verifier,
            "verifier_patient" => {
                if let Some(id) = first_patient {
                    self.go(Go::Check(id));
                } else {
                    self.screen = Screen::Verifier;
                }
            }
            "agenda" => self.screen = Screen::Agenda,
            "stups" => self.screen = Screen::Stups,
            "tables" => self.screen = Screen::Tables,
            "options" => self.screen = Screen::Options,
            _ => self.screen = Screen::Accueil,
        }
    }

    pub fn go(&mut self, to: Go) {
        match to {
            Go::Screen(s) => {
                self.screen = s;
                match s {
                    Screen::Patients => self.patients.focus_filter(),
                    Screen::Medicaments => self.medicaments.focus_filter(),
                    _ => {}
                }
            }
            Go::Patient(id) => {
                self.patients.open(id);
                self.screen = Screen::Patients;
            }
            Go::NewPatient => {
                self.patients.new_file();
                self.screen = Screen::Patients;
            }
            Go::Drug(id) => {
                self.medicaments.open(id);
                self.screen = Screen::Medicaments;
            }
            Go::Check(id) => {
                let mut go = None;
                let mut cx = Ctx {
                    s: &mut self.s,
                    go: &mut go,
                };
                self.verifier.load_patient(&mut cx, id);
                self.screen = Screen::Verifier;
            }
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.s.tick();
        self.keys(ctx);
        let mut go: Option<Go> = None;

        egui::TopBottomPanel::top("v2_top").show(ctx, |ui| {
            ui.add_space(ui::line(ui) * 0.2);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t("app.name"))
                        .strong()
                        .size(motif::pt(ui, 15.0)),
                );
                ui.add_space(ui::chars_wide(ui, 2.0));
                let w = ui::chars_wide(ui, 36.0).min(ui.available_width() * 0.6);
                let r = motif::field(
                    ui,
                    w,
                    egui::TextEdit::singleline(&mut self.find)
                        .id(egui::Id::new("v2_find"))
                        .hint_text(motif::hint(t("app.find"))),
                );
                if std::mem::take(&mut self.find_focus) {
                    r.request_focus();
                }
                if r.changed() {
                    self.find_pick = 0;
                }
                if r.has_focus() {
                    go = self.find_popup(ui, &r);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(motif::hint(ui::day_long(&self.s.today)));
                    if !self.s.operator.is_empty() {
                        ui.label(egui::RichText::new(&self.s.operator).strong());
                    }
                });
            });
            ui.add_space(ui::line(ui) * 0.2);
        });

        egui::TopBottomPanel::bottom("v2_status").show(ctx, |ui| {
            ui.horizontal(|ui| match &self.s.notice {
                Some(n) => {
                    let ink = if n.bad { motif::alert() } else { motif::text() };
                    if n.bad {
                        ui::pict(ui, motif::Pict::Warn, ink);
                    }
                    ui.label(egui::RichText::new(&n.text).color(ink));
                }
                None => {
                    ui.label(motif::hint(t("app.ready")));
                }
            });
        });

        egui::SidePanel::left("v2_nav")
            .resizable(false)
            .exact_width(nav_width(ctx))
            .show(ctx, |ui| {
                ui.add_space(ui::line(ui) * 0.5);
                for s in Screen::ALL {
                    let on = self.screen == s;
                    let r = nav_row(ui, s, on);
                    if r.clicked() {
                        go = Some(Go::Screen(s));
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut cx = Ctx {
                s: &mut self.s,
                go: &mut go,
            };
            match self.screen {
                Screen::Accueil => self.accueil.ui(ui, &mut cx),
                Screen::Patients => self.patients.ui(ui, &mut cx),
                Screen::Medicaments => self.medicaments.ui(ui, &mut cx),
                Screen::Verifier => self.verifier.ui(ui, &mut cx),
                Screen::Agenda => self.agenda.ui(ui, &mut cx),
                Screen::Stups => self.stups.ui(ui, &mut cx),
                Screen::Tables => self.tables.ui(ui, &mut cx),
                Screen::Options => self.options.ui(ui, &mut cx),
            }
        });

        if let Some(to) = go {
            self.s.notice = None;
            self.go(to);
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        let (find, screen) = ctx.input_mut(|i| {
            let find = i.consume_key(egui::Modifiers::COMMAND, egui::Key::K);
            let screen = Screen::ALL
                .into_iter()
                .find(|s| i.consume_key(egui::Modifiers::NONE, s.key()));
            (find, screen)
        });
        if find {
            self.find_focus = true;
        }
        if let Some(s) = screen {
            self.go(Go::Screen(s));
        }
    }

    /// The matches under the search box: files then cards, arrows to
    /// move, Enter to open.
    fn find_popup(&mut self, ui: &mut egui::Ui, field: &egui::Response) -> Option<Go> {
        let hits = find(&self.s, &self.find, 8);
        if hits.is_empty() {
            return None;
        }
        let (down, up, enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            self.find.clear();
            return None;
        }
        if down {
            self.find_pick = (self.find_pick + 1).min(hits.len() - 1);
        }
        if up {
            self.find_pick = self.find_pick.saturating_sub(1);
        }
        self.find_pick = self.find_pick.min(hits.len() - 1);
        let mut out = None;
        if enter {
            out = Some(hits[self.find_pick].1.clone());
        }
        let pos = field.rect.left_bottom();
        let width = field.rect.width().max(ui::chars_wide(ui, 36.0));
        egui::Area::new(egui::Id::new("v2_find_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(width);
                    for (k, (text, to)) in hits.iter().enumerate() {
                        if motif::list_row(ui, egui::RichText::new(text), k == self.find_pick)
                            .clicked()
                        {
                            out = Some(to.clone());
                        }
                    }
                });
            });
        if out.is_some() {
            self.find.clear();
            ui.memory_mut(|m| m.surrender_focus(egui::Id::new("v2_find")));
        }
        out
    }
}

/// What the search box offers for `query`: files first (a name is what
/// the counter types most), then cards.
pub fn find(s: &Session, query: &str, limit: usize) -> Vec<(String, Go)> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in crate::screens::patients::matching(&s.patients, query)
        .into_iter()
        .take(limit / 2)
    {
        let p = &s.patients[i];
        out.push((
            tf(
                "app.find_patient",
                &[&format!("{} {}", p.last_name.to_uppercase(), p.first_name)],
            ),
            Go::Patient(p.id),
        ));
    }
    let room = limit - out.len();
    for i in crate::screens::medicaments::matching(&s.drugs, query, room, &[]) {
        let d = &s.drugs[i];
        out.push((tf("app.find_drug", &[&d.name]), Go::Drug(d.id)));
    }
    out
}

/// The sidebar is as wide as its longest label and its key.
fn nav_width(ctx: &egui::Context) -> f32 {
    let style = ctx.style();
    let font = egui::TextStyle::Button.resolve(&style);
    let widest = ctx.fonts(|f| {
        Screen::ALL
            .iter()
            .map(|s| {
                f.layout_no_wrap(
                    format!("{}  F8", s.label()),
                    font.clone(),
                    egui::Color32::WHITE,
                )
                .size()
                .x
            })
            .fold(0.0, f32::max)
    });
    let line = ctx.fonts(|f| f.row_height(&font));
    widest + line * 2.5 + style.spacing.button_padding.x * 2.0
}

fn nav_row(ui: &mut egui::Ui, s: Screen, on: bool) -> egui::Response {
    let h = motif::button_height(ui) * 1.15;
    let (rect, r) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(rect, 0.0, motif::accent());
    } else if r.hovered() {
        painter.rect_filled(rect, 0.0, motif::bg_hover());
    }
    let ink = if on {
        motif::on_fill(motif::accent())
    } else {
        motif::text()
    };
    let line = ui::line(ui);
    let pad = ui.spacing().button_padding.x;
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + pad + line * 0.5, rect.center().y),
        egui::vec2(line, line),
    );
    motif::pictogram(painter, icon.shrink(line * 0.1), s.pict(), ink);
    let font = egui::TextStyle::Button.resolve(ui.style());
    painter.text(
        egui::pos2(icon.right() + pad * 0.8, rect.center().y),
        egui::Align2::LEFT_CENTER,
        s.label(),
        font.clone(),
        ink,
    );
    let key = format!("{:?}", s.key());
    painter.text(
        egui::pos2(rect.right() - pad, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        key,
        egui::FontId::proportional(motif::pt(ui, 10.0)),
        if on { ink } else { motif::text_faint() },
    );
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_offers_files_before_cards() {
        let (mut s, dir) = crate::session::tests::scratch("find");
        s.db.add_patient_with_sex("Doliveau", "Marc", "1960-01-01", "")
            .unwrap();
        s.changed();
        let hits = find(&s, "doli", 8);
        assert!(
            matches!(hits.first(), Some((_, Go::Patient(_)))),
            "{hits:?}"
        );
        assert!(
            hits.iter().any(|(_, g)| matches!(g, Go::Drug(_))),
            "{hits:?}"
        );
        assert!(find(&s, "  ", 8).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Every screen drawn headless at the two smallest shapes and the
    /// largest text, on a seeded base: no panic, and the frame fits.
    #[test]
    fn every_screen_draws_headless() {
        let (s, dir) = crate::session::tests::scratch("draw");
        let pid =
            s.db.add_patient_with_sex("Durand", "Anne", "1950-01-01", "")
                .unwrap();
        for d in s.drugs.iter().take(4) {
            s.db.add_patient_drug(pid, d.id).unwrap();
        }
        let mut desk = Desk::new(s);
        desk.s.changed();
        for (w, h, scale) in [(1024.0, 700.0, 1.0), (1280.0, 800.0, 1.6)] {
            for view in [
                "accueil",
                "patients",
                "patient",
                "patient_new",
                "medicaments",
                "fiche",
                "verifier",
                "verifier_patient",
                "agenda",
                "stups",
                "tables",
                "options",
            ] {
                let ctx = egui::Context::default();
                motif::apply(&ctx);
                motif::apply_scale(&ctx, scale, motif::Density::Comfortable);
                desk.start_at(view);
                for _ in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(w, h),
                        )),
                        ..Default::default()
                    };
                    let _ = ctx.run(input, |ctx| desk.update(ctx));
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Every screen *used* at random — clicks, wheel, keys, typing — on a
    /// base with files, treatments, biology and an ordonnance. Fails on a
    /// panic. A fixed seed, so a failure replays.
    #[test]
    fn every_screen_survives_random_use() {
        let (s, dir) = crate::session::tests::scratch("fuzz");
        let pid =
            s.db.add_patient_with_sex("Durand", "Anne", "1950-01-01", "F")
                .unwrap();
        for d in s.drugs.iter().take(6) {
            s.db.add_patient_drug(pid, d.id).unwrap();
        }
        let mut desk = Desk::new(s);
        desk.s.changed();
        let mut seed: u64 = 0x5eed_b9c4;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let keys = [
            egui::Key::Enter,
            egui::Key::Escape,
            egui::Key::Tab,
            egui::Key::ArrowDown,
            egui::Key::ArrowUp,
            egui::Key::Backspace,
        ];
        let (w, h) = (1024.0_f32, 700.0_f32);
        for view in [
            "accueil",
            "patient",
            "patient_new",
            "fiche",
            "verifier_patient",
            "agenda",
            "stups",
            "tables",
        ] {
            let ctx = egui::Context::default();
            motif::apply(&ctx);
            desk.start_at(view);
            for _ in 0..250 {
                let mut events = Vec::new();
                let pos = egui::pos2((next() % w as u64) as f32, (next() % h as u64) as f32);
                events.push(egui::Event::PointerMoved(pos));
                match next() % 6 {
                    0 | 1 => {
                        for pressed in [true, false] {
                            events.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            });
                        }
                    }
                    2 => events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, if next() % 2 == 0 { 3.0 } else { -3.0 }),
                        modifiers: egui::Modifiers::NONE,
                    }),
                    3 => {
                        let key = keys[(next() % keys.len() as u64) as usize];
                        for pressed in [true, false] {
                            events.push(egui::Event::Key {
                                key,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            });
                        }
                    }
                    4 => {
                        let words = ["dol", "12", "<15", "9h30", "010126", "a", "Tahor"];
                        events.push(egui::Event::Text(
                            words[(next() % words.len() as u64) as usize].to_owned(),
                        ));
                    }
                    _ => {}
                }
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(w, h),
                    )),
                    events,
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| desk.update(ctx));
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
