use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::db::{self, Db, Patient};
use crate::fuzzy;

/// Shared team documentation, editable in the docked pane. The file lives in
/// the application data directory; pointing it at the pharmacy network drive
/// (like the database) will be handled by `config.toml`.
fn team_doc_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bpm-caddy")
        .join("notes_equipe.md")
}

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

struct Session {
    db: Db,
    patients: Vec<Patient>,
    query: String,
    selected: usize,
    viewing: Option<Patient>,
    new_patient: Option<NewPatientForm>,
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
            new_patient: None,
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
}

pub struct App {
    state: State,
    show_docs: bool,
    doc_text: String,
    doc_dirty: bool,
    doc_last_edit: Instant,
    doc_error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let doc_text = std::fs::read_to_string(team_doc_path())
            .unwrap_or_else(|_| TEAM_DOC_TEMPLATE.to_owned());
        Self {
            state: State::Locked {
                password: String::new(),
                error: None,
            },
            show_docs: true,
            doc_text,
            doc_dirty: false,
            doc_last_edit: Instant::now(),
            doc_error: None,
        }
    }

    fn save_doc(&mut self) {
        let path = team_doc_path();
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

        if let Some(pw) = attempt {
            match Db::open(&db::default_path(), &pw).and_then(Session::new) {
                Ok(session) => self.state = State::Unlocked(Box::new(session)),
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

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(patient) = session.viewing.clone() {
                Self::patient_view(ui, ctx, session, &patient);
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
                if !ctx.wants_keyboard_input() {
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
                    session.viewing = Some(results[session.selected].clone());
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
                            session.viewing = Some(p.clone());
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
                            session.viewing = created;
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
    ) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            session.viewing = None;
            return;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if motif::button(ui, "← Retour (Échap)").clicked() {
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
            ui.label("Entretiens : à venir (cycle Identifié → Facturé).");
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_docs = !self.show_docs;
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("BPM-Caddy").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if motif::button(ui, "Documentation (F1)").clicked() {
                        self.show_docs = !self.show_docs;
                    }
                });
            });
            ui.add_space(4.0);
        });

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
