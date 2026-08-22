#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
            .with_title("BPM-Caddy"),
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy",
        options,
        Box::new(|cc| {
            motif::apply(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

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

struct App {
    search_query: String,
    show_docs: bool,
    doc_text: String,
    doc_dirty: bool,
    doc_last_edit: Instant,
    doc_error: Option<String>,
}

impl App {
    fn new() -> Self {
        let doc_text = std::fs::read_to_string(team_doc_path())
            .unwrap_or_else(|_| TEAM_DOC_TEMPLATE.to_owned());
        Self {
            search_query: String::new(),
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.heading("BPM-Caddy");
                ui.label("Suivi des entretiens pharmaceutiques");
                ui.add_space(24.0);

                let search = ui.add_sized(
                    [420.0, 32.0],
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Rechercher un patient (recherche floue)…"),
                );
                motif::bevel(ui.painter(), search.rect.expand(2.0), false);
                // Search is the default view: keep the bar focused on launch.
                if !ctx.wants_keyboard_input() {
                    search.request_focus();
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.doc_dirty {
            self.save_doc();
        }
    }
}
