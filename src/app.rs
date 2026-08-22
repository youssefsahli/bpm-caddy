use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::Config;
use crate::db::{self, Db, Interview, InterviewKind, InterviewState, InterviewSummary, Patient};
use crate::fuzzy;

const TEAM_DOC_TEMPLATE: &str = "\
# Notes d'équipe — BPM-Caddy

Ce panneau est partagé par toute l'équipe : consignes, rappels et suivi des
entretiens en cours. Les modifications sont enregistrées automatiquement.

## Consignes du jour

- (à compléter)

## Patients à recontacter

- (à compléter)

## Rappels de facturation

- Vérifier chaque vendredi les entretiens « Réalisés » non facturés.
";

enum State {
    Locked {
        password: String,
        error: Option<String>,
    },
    Unlocked(Box<Session>),
}

/// The master password can be kept in the OS credential manager
/// (spec 4.2): Windows Credential Manager, macOS Keychain, or the
/// Secret Service on Linux.
fn keyring_entry() -> Option<keyring::Entry> {
    if std::env::var_os("BPM_CADDY_NO_KEYRING").is_some() {
        return None;
    }
    keyring::Entry::new("bpm-caddy", "master-password").ok()
}

#[derive(PartialEq, Clone, Copy)]
enum MainView {
    Search,
    Dashboard,
}

struct Session {
    db: Db,
    patients: Vec<Patient>,
    query: String,
    selected: usize,
    viewing: Option<Patient>,
    viewing_interviews: Vec<Interview>,
    new_patient: Option<NewPatientForm>,
    view: MainView,
    summaries: Vec<InterviewSummary>,
    /// In-progress text of the per-interview date fields, keyed by id.
    date_edits: std::collections::HashMap<i64, String>,
    error: Option<String>,
}

#[derive(Default)]
struct NewPatientForm {
    last_name: String,
    first_name: String,
    birth_date: String,
    error: Option<String>,
}

impl Session {
    fn new(db: Db) -> Result<Self, String> {
        let patients = db.patients()?;
        Ok(Self {
            db,
            patients,
            query: String::new(),
            selected: 0,
            viewing: None,
            viewing_interviews: Vec::new(),
            new_patient: None,
            view: MainView::Search,
            summaries: Vec::new(),
            date_edits: std::collections::HashMap::new(),
            error: None,
        })
    }

    /// Fuzzy-rank patients against the query (best first, capped at 20).
    fn results(&self) -> Vec<&Patient> {
        let mut scored: Vec<(i32, &Patient)> = self
            .patients
            .iter()
            .filter_map(|p| {
                let a = fuzzy::score(&self.query, &format!("{} {}", p.first_name, p.last_name));
                let b = fuzzy::score(&self.query, &format!("{} {}", p.last_name, p.first_name));
                a.max(b).map(|s| (s, p))
            })
            .collect();
        scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
        scored.into_iter().take(20).map(|(_, p)| p).collect()
    }

    fn open_patient(&mut self, patient: Patient) {
        match self.db.interviews_for(patient.id) {
            Ok(list) => self.viewing_interviews = list,
            Err(e) => self.error = Some(e),
        }
        self.viewing = Some(patient);
    }

    fn reload_interviews(&mut self, patient_id: i64) {
        match self.db.interviews_for(patient_id) {
            Ok(list) => self.viewing_interviews = list,
            Err(e) => self.error = Some(e),
        }
    }
}

pub struct App {
    state: State,
    config: Config,
    last_activity: Instant,
    remember_password: bool,
    show_docs: bool,
    doc_text: String,
    doc_dirty: bool,
    doc_last_edit: Instant,
    doc_error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let doc_text = std::fs::read_to_string(config.team_doc_path())
            .unwrap_or_else(|_| TEAM_DOC_TEMPLATE.to_owned());
        let show_docs = config.ui.show_docs_on_start;

        // Silent unlock when the OS credential manager holds the password.
        let mut state = State::Locked {
            password: String::new(),
            error: None,
        };
        let mut remember_password = false;
        // Demo/e2e hook first, then the OS credential manager.
        let stored_pw = std::env::var("BPM_CADDY_PASSWORD")
            .ok()
            .or_else(|| keyring_entry().and_then(|e| e.get_password().ok()));
        if let Some(pw) = stored_pw {
            match Db::open(&config.db_path(), &pw).and_then(Session::new) {
                Ok(mut session) => {
                    // Demo hook: land on a specific view (screenshots, e2e).
                    match std::env::var("BPM_CADDY_START_VIEW").as_deref() {
                        Ok("dashboard") => {
                            session.summaries =
                                session.db.interview_summaries().unwrap_or_default();
                            session.view = MainView::Dashboard;
                        }
                        Ok("patient") => {
                            if let Some(p) = session.patients.first().cloned() {
                                session.open_patient(p);
                            }
                        }
                        _ => {}
                    }
                    state = State::Unlocked(Box::new(session));
                    remember_password = true;
                }
                Err(e) => {
                    state = State::Locked {
                        password: String::new(),
                        error: Some(format!("Mot de passe mémorisé refusé : {e}")),
                    };
                }
            }
        }

        Self {
            state,
            config,
            last_activity: Instant::now(),
            remember_password,
            show_docs,
            doc_text,
            doc_dirty: false,
            doc_last_edit: Instant::now(),
            doc_error: None,
        }
    }

    fn save_doc(&mut self) {
        let path = self.config.team_doc_path();
        let result = path
            .parent()
            .map(std::fs::create_dir_all)
            .unwrap_or(Ok(()))
            .and_then(|()| std::fs::write(&path, &self.doc_text));
        match result {
            Ok(()) => {
                self.doc_dirty = false;
                self.doc_error = None;
            }
            Err(e) => self.doc_error = Some(format!("Enregistrement impossible : {e}")),
        }
    }

    fn docs_pane(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("team_docs")
            .resizable(true)
            .default_width(340.0)
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.heading("Documentation d'équipe");
                let status = if let Some(err) = &self.doc_error {
                    err.clone()
                } else if self.doc_dirty {
                    "Modifications en cours…".to_owned()
                } else {
                    "Enregistré".to_owned()
                };
                ui.label(status);
                ui.add_space(4.0);

                let editor_rect = ui.available_rect_before_wrap().shrink(2.0);
                motif::bevel(ui.painter(), editor_rect, false);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response = ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut self.doc_text)
                            .font(egui::TextStyle::Monospace)
                            .frame(false),
                    );
                    if response.changed() {
                        self.doc_dirty = true;
                        self.doc_last_edit = Instant::now();
                    }
                });
            });

        // Debounced auto-save: write once the user pauses typing.
        if self.doc_dirty && self.doc_last_edit.elapsed() > Duration::from_millis(1200) {
            self.save_doc();
        }
        if self.doc_dirty {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }

    fn unlock_screen(&mut self, ctx: &egui::Context) {
        let mut remember = self.remember_password;
        let State::Locked { password, error } = &mut self.state else {
            return;
        };
        let mut attempt: Option<String> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.heading("BPM-Caddy");
                ui.label("Base patients chiffrée — saisissez le mot de passe maître");
                ui.add_space(20.0);

                let field = ui.add_sized(
                    [300.0, 30.0],
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .hint_text("Mot de passe"),
                );
                motif::bevel(ui.painter(), field.rect.expand(2.0), false);
                if !ctx.wants_keyboard_input() {
                    field.request_focus();
                }

                let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(10.0);
                ui.checkbox(&mut remember, "Mémoriser dans le trousseau du système");
                if (motif::button(ui, "Déverrouiller").clicked() || submitted)
                    && !password.is_empty()
                {
                    attempt = Some(password.clone());
                }
                if let Some(err) = error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                }
            });
        });

        self.remember_password = remember;
        if let Some(pw) = attempt {
            match Db::open(&self.config.db_path(), &pw).and_then(Session::new) {
                Ok(session) => {
                    self.state = State::Unlocked(Box::new(session));
                    if let Some(entry) = keyring_entry() {
                        if self.remember_password {
                            let _ = entry.set_password(&pw);
                        } else {
                            // Unchecked: make sure no stale copy remains.
                            let _ = entry.delete_credential();
                        }
                    }
                }
                Err(e) => {
                    let State::Locked { password, error } = &mut self.state else {
                        return;
                    };
                    password.clear();
                    *error = Some(e);
                }
            }
        }
    }

    fn main_screen(&mut self, ctx: &egui::Context) {
        let State::Unlocked(session) = &mut self.state else {
            return;
        };

        // Ctrl+F returns to the search from anywhere (spec 3.1).
        let focus_search = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F));
        if focus_search {
            session.view = MainView::Search;
            session.viewing = None;
        }

        let config = self.config.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            if session.view == MainView::Dashboard {
                Self::dashboard_view(ui, session, &config);
                return;
            }
            if let Some(patient) = session.viewing.clone() {
                Self::patient_view(ui, ctx, session, &patient, &config);
                return;
            }

            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("BPM-Caddy");
                ui.label("Suivi des entretiens pharmaceutiques");
                ui.add_space(24.0);

                let search = ui.add_sized(
                    [420.0, 32.0],
                    egui::TextEdit::singleline(&mut session.query)
                        .hint_text("Rechercher un patient (recherche floue)…"),
                );
                motif::bevel(ui.painter(), search.rect.expand(2.0), false);
                // Search is the default view: keep the bar focused.
                if focus_search || !ctx.wants_keyboard_input() {
                    search.request_focus();
                }
                if search.changed() {
                    session.selected = 0;
                    session.new_patient = None;
                }
            });
            ui.add_space(12.0);

            let results: Vec<Patient> = session.results().into_iter().cloned().collect();

            if !results.is_empty() {
                let (up, down, enter) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::Enter),
                    )
                });
                if down {
                    session.selected = (session.selected + 1).min(results.len() - 1);
                }
                if up {
                    session.selected = session.selected.saturating_sub(1);
                }
                if enter {
                    session.open_patient(results[session.selected].clone());
                }

                ui.vertical_centered(|ui| {
                    for (i, p) in results.iter().enumerate() {
                        let text = format!(
                            "{}   (né(e) le {})",
                            p.full_name(),
                            db::format_french_date(&p.birth_date)
                        );
                        let selected = i == session.selected;
                        let label = egui::RichText::new(text).size(15.0);
                        let label = if selected {
                            label
                                .color(egui::Color32::WHITE)
                                .background_color(motif::ACCENT)
                        } else {
                            label
                        };
                        let row = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
                        if row.clicked() {
                            session.open_patient(p.clone());
                        }
                    }
                });
            } else if !session.query.trim().is_empty() {
                // No match: the search transitions into quick creation (spec 3.1).
                let form = session.new_patient.get_or_insert_with(Default::default);
                let mut create = false;
                ui.vertical_centered(|ui| {
                    ui.label("Aucun patient trouvé — création rapide :");
                    ui.add_space(8.0);
                    egui::Grid::new("new_patient")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Nom");
                            ui.add_sized(
                                [220.0, 26.0],
                                egui::TextEdit::singleline(&mut form.last_name),
                            );
                            ui.end_row();
                            ui.label("Prénom");
                            ui.add_sized(
                                [220.0, 26.0],
                                egui::TextEdit::singleline(&mut form.first_name),
                            );
                            ui.end_row();
                            ui.label("Naissance");
                            ui.add_sized(
                                [220.0, 26.0],
                                egui::TextEdit::singleline(&mut form.birth_date)
                                    .hint_text("JJ/MM/AAAA"),
                            );
                            ui.end_row();
                        });
                    ui.add_space(8.0);
                    if motif::button(ui, "Créer le patient").clicked() {
                        create = true;
                    }
                    if let Some(err) = &form.error {
                        ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                    }
                });

                if create {
                    let outcome = db::parse_french_date(&form.birth_date).and_then(|iso| {
                        if form.last_name.trim().is_empty() || form.first_name.trim().is_empty() {
                            return Err("Nom et prénom sont obligatoires.".to_owned());
                        }
                        session
                            .db
                            .add_patient(form.last_name.trim(), form.first_name.trim(), &iso)
                    });
                    match outcome {
                        Ok(_) => {
                            match session.db.patients() {
                                Ok(list) => session.patients = list,
                                Err(e) => session.error = Some(e),
                            }
                            let created = session.patients.last().cloned();
                            session.query.clear();
                            session.new_patient = None;
                            if let Some(p) = created {
                                session.open_patient(p);
                            }
                        }
                        Err(e) => form.error = Some(e),
                    }
                }
            }

            if let Some(err) = &session.error {
                ui.vertical_centered(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                });
            }
        });
    }

    fn patient_view(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        patient: &Patient,
        config: &Config,
    ) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            session.viewing = None;
            return;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if motif::button(ui, "Retour (Échap)").clicked() {
                session.viewing = None;
            }
        });
        ui.add_space(12.0);
        let card = ui.available_rect_before_wrap().shrink(6.0);
        motif::bevel(ui.painter(), card, true);
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(patient.full_name());
            ui.label(format!(
                "Né(e) le {}",
                db::format_french_date(&patient.birth_date)
            ));
            ui.add_space(16.0);

            // Ctrl+N or the buttons below start a new interview (spec 3.1).
            ui.label("Nouvel entretien (Ctrl+N) :");
            let ctrl_n = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N));
            let mut new_kind: Option<InterviewKind> = None;
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 130.0);
                for kind in InterviewKind::ALL {
                    if motif::button(ui, kind.label()).clicked() {
                        new_kind = Some(kind);
                    }
                }
            });
            if ctrl_n && new_kind.is_none() {
                new_kind = Some(InterviewKind::Bpm);
            }
            if let Some(kind) = new_kind {
                match session.db.add_interview(patient.id, kind) {
                    Ok(_) => session.reload_interviews(patient.id),
                    Err(e) => session.error = Some(e),
                }
            }

            ui.add_space(16.0);
            if session.viewing_interviews.is_empty() {
                ui.label("Aucun entretien pour ce patient.");
            }
        });

        let interviews = session.viewing_interviews.clone();
        let mut advance: Option<(i64, db::InterviewState)> = None;
        let mut print_kind: Option<InterviewKind> = None;
        let mut set_duration: Option<(i64, i64)> = None;
        let mut set_date: Option<(i64, Option<String>)> = None;
        ui.vertical_centered(|ui| {
            egui::Grid::new("interviews")
                .num_columns(7)
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    for itv in &interviews {
                        ui.label(egui::RichText::new(itv.kind.label()).strong());
                        ui.label(format!(
                            "créé le {}",
                            &itv.created_at[..10.min(itv.created_at.len())]
                        ));
                        ui.label(
                            egui::RichText::new(itv.state.label())
                                .color(egui::Color32::WHITE)
                                .background_color(motif::ACCENT),
                        );
                        if let Some(next) = itv.state.next() {
                            if motif::button(ui, &format!("» {}", next.label())).clicked() {
                                advance = Some((itv.id, itv.state));
                            }
                        } else {
                            ui.label("Terminé");
                        }
                        if motif::button(ui, "Fiche PDF").clicked() {
                            print_kind = Some(itv.kind);
                        }
                        let mut minutes = itv.duration_minutes;
                        let drag = ui.add(
                            egui::DragValue::new(&mut minutes)
                                .range(0..=480)
                                .suffix(" min"),
                        );
                        if drag.changed() {
                            set_duration = Some((itv.id, minutes));
                        }
                        // Planned date: free text, committed when it parses
                        // (or empties) and the field loses focus.
                        let text = session.date_edits.entry(itv.id).or_insert_with(|| {
                            itv.scheduled_date
                                .as_deref()
                                .map(db::format_french_date)
                                .unwrap_or_default()
                        });
                        let field = ui.add(
                            egui::TextEdit::singleline(text)
                                .hint_text("RDV JJ/MM/AAAA")
                                .desired_width(120.0),
                        );
                        if field.lost_focus() {
                            if text.trim().is_empty() {
                                if itv.scheduled_date.is_some() {
                                    set_date = Some((itv.id, None));
                                }
                            } else if let Ok(iso) = db::parse_french_date(text) {
                                if itv.scheduled_date.as_deref() != Some(iso.as_str()) {
                                    set_date = Some((itv.id, Some(iso)));
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        });
        if let Some((id, state)) = advance {
            match session.db.advance_interview(id, state) {
                Ok(()) => session.reload_interviews(patient.id),
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, minutes)) = set_duration {
            match session.db.set_duration(id, minutes) {
                Ok(()) => session.reload_interviews(patient.id),
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, date)) = set_date {
            match session.db.set_scheduled_date(id, date.as_deref()) {
                Ok(()) => {
                    session.date_edits.remove(&id);
                    session.reload_interviews(patient.id);
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some(kind) = print_kind {
            let today = session
                .db
                .today_french()
                .unwrap_or_else(|_| "__ / __ / ____".to_owned());
            if let Err(e) = crate::pdf::open_interview_sheet(
                patient,
                kind,
                &today,
                config.templates.bpm_template_path.as_deref(),
            ) {
                session.error = Some(e);
            }
        }
        if let Some(err) = &session.error {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
            });
        }
    }

    /// A raised KPI box: title on top, big value below.
    fn kpi_box(ui: &mut egui::Ui, width: f32, title: &str, value: &str) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 74.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, motif::BG);
        motif::bevel(ui.painter(), rect, true);
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 18.0),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(13.0),
            motif::TEXT,
        );
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 48.0),
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::proportional(22.0),
            motif::ACCENT,
        );
    }

    /// Financial & statistical dashboard (spec 3.3): KPIs, pipeline funnel,
    /// monthly billed vs pending revenue.
    fn dashboard_view(ui: &mut egui::Ui, session: &mut Session, config: &Config) {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.heading("Tableau de bord");
        });
        ui.add_space(10.0);

        let billed: f64 = session
            .summaries
            .iter()
            .filter(|s| s.state == InterviewState::Billed)
            .map(|s| config.fee(s.kind))
            .sum();
        let pending: f64 = session
            .summaries
            .iter()
            .filter(|s| s.state != InterviewState::Billed)
            .map(|s| config.fee(s.kind))
            .sum();
        let billed_count = session
            .summaries
            .iter()
            .filter(|s| s.state == InterviewState::Billed)
            .count();
        // Hourly ROI (spec 3.3): billed revenue over the time actually spent.
        let billed_minutes: i64 = session
            .summaries
            .iter()
            .filter(|s| s.state == InterviewState::Billed)
            .map(|s| s.duration_minutes)
            .sum();
        let roi = if billed_minutes > 0 {
            format!("{:.0} €/h", billed / (billed_minutes as f64 / 60.0))
        } else {
            "— €/h".to_owned()
        };

        let kpi_w = ((ui.available_width() - 70.0) / 4.0).clamp(110.0, 190.0);
        ui.horizontal(|ui| {
            let total = kpi_w * 4.0 + 30.0;
            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
            Self::kpi_box(ui, kpi_w, "CA facturé", &format!("{billed:.0} €"));
            Self::kpi_box(ui, kpi_w, "CA en attente", &format!("{pending:.0} €"));
            Self::kpi_box(ui, kpi_w, "Entretiens facturés", &billed_count.to_string());
            Self::kpi_box(ui, kpi_w, "Taux horaire", &roi);
        });
        ui.add_space(18.0);

        // Pipeline funnel: one sunken bar per state.
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Pipeline des entretiens").strong());
        });
        ui.add_space(6.0);
        let max_count = InterviewState::ALL
            .iter()
            .map(|st| session.summaries.iter().filter(|s| s.state == *st).count())
            .max()
            .unwrap_or(0)
            .max(1);
        for st in InterviewState::ALL {
            let count = session.summaries.iter().filter(|s| s.state == st).count();
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 250.0);
                ui.add_sized([110.0, 20.0], egui::Label::new(st.label()));
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(300.0, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, motif::TROUGH);
                motif::bevel(ui.painter(), rect, false);
                let mut fill = rect.shrink(3.0);
                fill.set_width(fill.width() * (count as f32 / max_count as f32));
                if count > 0 {
                    ui.painter().rect_filled(fill, 0.0, motif::ACCENT);
                }
                ui.label(count.to_string());
            });
        }
        ui.add_space(18.0);

        // Monthly revenue: billed (dark blue) vs pending (grey), last 12 months.
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("CA mensuel — facturé vs en attente").strong());
        });
        ui.add_space(6.0);
        let mut months: Vec<String> = session
            .summaries
            .iter()
            .map(|s| {
                if s.state == InterviewState::Billed {
                    s.updated_month.clone()
                } else {
                    s.created_month.clone()
                }
            })
            .collect();
        months.sort();
        months.dedup();
        let months: Vec<String> = months.into_iter().rev().take(12).rev().collect();

        if months.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label("Aucun entretien enregistré pour l'instant.");
            });
            return;
        }

        let per_month: Vec<(String, f64, f64)> = months
            .iter()
            .map(|m| {
                let b: f64 = session
                    .summaries
                    .iter()
                    .filter(|s| s.state == InterviewState::Billed && &s.updated_month == m)
                    .map(|s| config.fee(s.kind))
                    .sum();
                let p: f64 = session
                    .summaries
                    .iter()
                    .filter(|s| s.state != InterviewState::Billed && &s.created_month == m)
                    .map(|s| config.fee(s.kind))
                    .sum();
                (m.clone(), b, p)
            })
            .collect();
        let max_val = per_month
            .iter()
            .map(|(_, b, p)| b.max(*p))
            .fold(1.0_f64, f64::max);

        let chart_w = (per_month.len() as f32 * 70.0).min(ui.available_width() - 40.0);
        let (chart, _) =
            ui.allocate_exact_size(egui::vec2(chart_w.max(140.0), 190.0), egui::Sense::hover());
        let chart = egui::Rect::from_center_size(
            egui::pos2(ui.max_rect().center().x, chart.center().y),
            chart.size(),
        );
        ui.painter().rect_filled(chart, 0.0, motif::TROUGH);
        motif::bevel(ui.painter(), chart, false);
        let plot = chart.shrink(10.0);
        let slot = plot.width() / per_month.len() as f32;
        for (i, (month, b, p)) in per_month.iter().enumerate() {
            let x0 = plot.left() + i as f32 * slot;
            let bar_w = (slot - 18.0).clamp(6.0, 24.0);
            let scale = |v: f64| (v / max_val) as f32 * (plot.height() - 26.0);
            let billed_rect = egui::Rect::from_min_max(
                egui::pos2(
                    x0 + slot / 2.0 - bar_w - 1.0,
                    plot.bottom() - 16.0 - scale(*b),
                ),
                egui::pos2(x0 + slot / 2.0 - 1.0, plot.bottom() - 16.0),
            );
            let pending_rect = egui::Rect::from_min_max(
                egui::pos2(x0 + slot / 2.0 + 1.0, plot.bottom() - 16.0 - scale(*p)),
                egui::pos2(x0 + slot / 2.0 + bar_w + 1.0, plot.bottom() - 16.0),
            );
            ui.painter().rect_filled(billed_rect, 0.0, motif::ACCENT);
            ui.painter().rect_filled(pending_rect, 0.0, motif::BG_DARK);
            // "2026-08" → "08/26"
            let label = match (month.get(5..7), month.get(2..4)) {
                (Some(mm), Some(yy)) => format!("{mm}/{yy}"),
                _ => month.clone(),
            };
            ui.painter().text(
                egui::pos2(x0 + slot / 2.0, plot.bottom() - 6.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                motif::TEXT,
            );
        }
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            ui.label("■ facturé (bleu)   ■ en attente (gris)");
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-lock after inactivity (spec 4.3).
        if ctx.input(|i| !i.events.is_empty() || i.pointer.is_moving()) {
            self.last_activity = Instant::now();
        }
        if let State::Unlocked(_) = self.state {
            let timeout = self.config.database.auto_lock_timeout_minutes;
            if timeout > 0 && self.last_activity.elapsed() > Duration::from_secs(timeout * 60) {
                self.state = State::Locked {
                    password: String::new(),
                    error: None,
                };
            }
            ctx.request_repaint_after(Duration::from_secs(30));
        }

        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_docs = !self.show_docs;
        }
        let mut toggle_dashboard = ctx.input(|i| i.key_pressed(egui::Key::F2));

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("BPM-Caddy").strong());
                ui.label(
                    egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(11.0)
                        .color(motif::BG_DARK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if motif::button(ui, "Documentation (F1)").clicked() {
                        self.show_docs = !self.show_docs;
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, "Tableau de bord (F2)").clicked()
                    {
                        toggle_dashboard = true;
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, "Verrouiller").clicked()
                    {
                        self.state = State::Locked {
                            password: String::new(),
                            error: None,
                        };
                    }
                });
            });
            ui.add_space(4.0);
        });

        if toggle_dashboard {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Search => {
                        match session.db.interview_summaries() {
                            Ok(s) => session.summaries = s,
                            Err(e) => session.error = Some(e),
                        }
                        MainView::Dashboard
                    }
                    MainView::Dashboard => MainView::Search,
                };
            }
        }

        if self.show_docs {
            self.docs_pane(ctx);
        }

        match self.state {
            State::Locked { .. } => self.unlock_screen(ctx),
            State::Unlocked(_) => self.main_screen(ctx),
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.doc_dirty {
            self.save_doc();
        }
    }
}
