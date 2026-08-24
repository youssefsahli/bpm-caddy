use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::Config;
use crate::db::{
    self, Appointment, Db, Drug, Interview, InterviewKind, InterviewState, InterviewSummary,
    Patient,
};
use crate::fuzzy;
use crate::strings::{tr, trf, trn};

enum State {
    Locked {
        password: String,
        error: Option<String>,
    },
    Unlocked(Box<Session>),
}

/// Run [`daily_backup`] on a background thread with its own connection:
/// `VACUUM INTO` rewrites the whole encrypted file, and doing that
/// synchronously over a network share would freeze the UI at unlock.
fn spawn_daily_backup(db_path: std::path::PathBuf, password: String, keep: usize) {
    if keep == 0 {
        return;
    }
    std::thread::spawn(move || {
        if let Ok(db) = Db::open(&db_path, &password) {
            daily_backup(&db, &db_path, keep);
        }
    });
}

/// One backup per day, in `backups/` next to the database, pruned to
/// the newest `keep` (0 disables). Runs after each successful unlock.
/// Failures only go to stderr: a failed backup must never block the
/// counter workflow, and on a shared drive another PC may have made
/// today's copy already.
fn daily_backup(db: &Db, db_path: &std::path::Path, keep: usize) {
    if keep == 0 {
        return;
    }
    let Ok(today) = db.today_iso() else { return };
    let Some(parent) = db_path.parent() else {
        return;
    };
    let dir = parent.join("backups");
    let target = dir.join(format!("bpm_caddy-{today}.db"));
    if target.exists() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("bpm-caddy : dossier de sauvegarde inaccessible : {e}");
        return;
    }
    if let Err(e) = db.backup_to(&target) {
        eprintln!("bpm-caddy : {e}");
        return;
    }
    // Date-named files sort chronologically: drop the oldest past `keep`.
    let mut backups: Vec<_> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("bpm_caddy-") && n.ends_with(".db"))
                })
                .collect()
        })
        .unwrap_or_default();
    backups.sort();
    while backups.len() > keep {
        let _ = std::fs::remove_file(backups.remove(0));
    }
}

/// Build the billing-reconciliation CSV: BOM + semicolons for French
/// Excel, dates in JJ/MM/AAAA, decimal comma for the fees.
fn interviews_csv(rows: &[db::ExportRow], config: &Config) -> String {
    fn field(s: &str) -> String {
        if s.contains([';', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_owned()
        }
    }
    let mut out = String::from(
        "\u{feff}Patient;Téléphone;Naissance;Type;État;Créé le;RDV;Durée (min);\
         Honoraires (€);Facturé (€)\r\n",
    );
    for r in rows {
        let comma = |v: f64| format!("{v:.2}").replace('.', ",");
        // "Honoraires" is the tariff; "Facturé" only counts once the
        // interview is billed, so summing that column matches the
        // dashboard's billed revenue.
        let fee = config.fee(r.kind);
        let billed = if r.state == InterviewState::Billed {
            fee
        } else {
            0.0
        };
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{}\r\n",
            field(&r.patient_name),
            field(&r.phone),
            db::format_french_date(&r.birth_date),
            r.kind.label(),
            r.state.label(),
            db::format_french_date(&r.created_date),
            r.scheduled_date
                .as_deref()
                .map(db::format_french_date)
                .unwrap_or_default(),
            r.duration_minutes,
            comma(fee),
            comma(billed),
        ));
    }
    out
}

/// Union-merge for the shared team notes: keep our text, append the
/// lines another PC added since the common `base` (lines we deleted
/// ourselves stay deleted). Notes are lists — line granularity is the
/// natural unit, and appending beats losing a colleague's reminder.
fn merge_team_notes(base: &str, ours: &str, theirs: &str) -> String {
    use std::collections::HashSet;
    let ours_set: HashSet<&str> = ours.lines().collect();
    let base_set: HashSet<&str> = base.lines().collect();
    let added: Vec<&str> = theirs
        .lines()
        .filter(|l| !l.trim().is_empty() && !ours_set.contains(l) && !base_set.contains(l))
        .collect();
    if added.is_empty() {
        return ours.to_owned();
    }
    let mut out = ours.trim_end().to_owned();
    out.push('\n');
    for line in added {
        out.push('\n');
        out.push_str(line);
    }
    out.push('\n');
    out
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
    /// The team's drug reference base (F3).
    Drugs,
    /// Upcoming patient appointments, grouped by day (F4).
    Agenda,
}

struct Session {
    db: Db,
    patients: Vec<Patient>,
    /// Precomputed "First Last" / "Last First" search keys, parallel to
    /// `patients` — two string allocations per patient per *load*, not
    /// per frame while typing.
    search_keys: Vec<(String, String)>,
    /// Not-yet-billed interview count per patient id ("n en cours").
    pending: std::collections::HashMap<i64, i64>,
    query: String,
    selected: usize,
    viewing: Option<Patient>,
    viewing_interviews: Vec<Interview>,
    new_patient: Option<PatientForm>,
    /// In-progress correction of the viewed patient's identity.
    edit_patient: Option<PatientForm>,
    /// Two-step delete confirmation for the viewed patient.
    confirm_delete: bool,
    /// Two-step delete confirmation for one interview row (by id).
    confirm_delete_itv: Option<i64>,
    view: MainView,
    summaries: Vec<InterviewSummary>,
    /// Planned interviews for the dashboard's "RDV à venir" list.
    appointments: Vec<Appointment>,
    /// Path of the last CSV export, shown under the export button.
    export_notice: Option<String>,
    /// Today as ISO `YYYY-MM-DD`, to flag overdue appointments.
    today: String,
    /// Tomorrow as ISO `YYYY-MM-DD`, for agenda day labels.
    tomorrow: String,
    /// The 7 dates (Mon..Sun) of the agenda's displayed week.
    agenda_week: Vec<String>,
    /// Week shown in the agenda, relative to the current one.
    agenda_offset: i64,
    /// In-progress text of the per-interview date fields, keyed by id.
    date_edits: std::collections::HashMap<i64, String>,
    /// Discreet mode: revenue amounts stay masked until explicitly
    /// revealed, and re-mask when leaving the dashboard.
    show_amounts: bool,
    /// Last duplicate-check reload of the patient list (quick-create),
    /// throttled so typing doesn't hit the network database per key.
    dup_check: Option<Instant>,
    /// Drug reference base (F3): list, search, and the open card with
    /// its as-loaded baseline for compare-and-set saves.
    drugs: Vec<Drug>,
    drug_query: String,
    drug_selected: usize,
    drug_form: Option<Drug>,
    drug_base: Option<Drug>,
    confirm_delete_drug: bool,
    error: Option<String>,
}

/// Patient identity fields, used by both quick-creation and editing.
/// Quick-creation only asks for the first three (spec 3.1); phone and
/// notes are filled in later from the patient view.
#[derive(Default, Clone)]
struct PatientForm {
    last_name: String,
    first_name: String,
    birth_date: String,
    phone: String,
    notes: String,
    error: Option<String>,
}

impl Session {
    fn new(db: Db) -> Result<Self, String> {
        let patients = db.patients()?;
        let pending = db.pending_counts().unwrap_or_default();
        // First unlock of a fresh base: starter drug cards (names, DCI,
        // textbook antidotes). Non-fatal if it fails.
        let _ = db.seed_drugs_if_empty();
        let mut session = Self {
            db,
            patients: Vec::new(),
            search_keys: Vec::new(),
            pending,
            query: String::new(),
            selected: 0,
            viewing: None,
            viewing_interviews: Vec::new(),
            new_patient: None,
            edit_patient: None,
            confirm_delete: false,
            confirm_delete_itv: None,
            view: MainView::Search,
            summaries: Vec::new(),
            appointments: Vec::new(),
            export_notice: None,
            today: String::new(),
            tomorrow: String::new(),
            agenda_week: Vec::new(),
            agenda_offset: 0,
            date_edits: std::collections::HashMap::new(),
            show_amounts: false,
            dup_check: None,
            drugs: Vec::new(),
            drug_query: String::new(),
            drug_selected: 0,
            drug_form: None,
            drug_base: None,
            confirm_delete_drug: false,
            error: None,
        };
        session.set_patients(patients);
        Ok(session)
    }

    /// Re-point `viewing` at the freshly loaded patient row (or close
    /// the view if the patient no longer exists).
    fn resync_viewing(&mut self) {
        if let Some(id) = self.viewing.as_ref().map(|p| p.id) {
            self.viewing = self.patients.iter().find(|p| p.id == id).cloned();
        }
    }

    /// Commit any parseable in-progress RDV field before the patient
    /// view goes away (view switch, Escape, lock): a date the user
    /// typed but never tabbed out of must not be silently dropped.
    fn flush_date_edits(&mut self) {
        let Some(pid) = self.viewing.as_ref().map(|p| p.id) else {
            self.date_edits.clear();
            return;
        };
        if self.date_edits.is_empty() {
            return;
        }
        let year = self.db.current_year();
        for itv in self.viewing_interviews.clone() {
            let Some(text) = self.date_edits.get(&itv.id) else {
                continue;
            };
            let expected = itv.scheduled_date.as_deref();
            if text.trim().is_empty() {
                if itv.scheduled_date.is_some() {
                    let _ = self.db.set_scheduled_date(itv.id, None, expected);
                }
            } else if let Ok(iso) = db::parse_french_date(text, year, db::YearHint::Future) {
                if expected != Some(iso.as_str()) {
                    let _ = self.db.set_scheduled_date(itv.id, Some(&iso), expected);
                }
            }
        }
        self.date_edits.clear();
        self.reload_interviews(pid);
    }

    /// Replace the patient list and rebuild its search keys. The list is
    /// kept alphabetical (accent-insensitive) so browsing with an empty
    /// query reads naturally.
    fn set_patients(&mut self, mut list: Vec<Patient>) {
        list.sort_by_cached_key(|p| {
            (
                fuzzy::sort_key(&p.last_name),
                fuzzy::sort_key(&p.first_name),
            )
        });
        self.search_keys = list
            .iter()
            .map(|p| {
                (
                    format!("{} {}", p.first_name, p.last_name),
                    format!("{} {}", p.last_name, p.first_name),
                )
            })
            .collect();
        self.patients = list;
    }

    /// Fuzzy-rank patients against the query (best first, capped at 20).
    fn results(&self) -> Vec<&Patient> {
        let mut scored: Vec<(i32, &Patient)> = self
            .patients
            .iter()
            .zip(self.search_keys.iter())
            .filter_map(|(p, (k1, k2))| {
                let a = fuzzy::score(&self.query, k1);
                let b = fuzzy::score(&self.query, k2);
                // Digits find patients by phone number too.
                let c = if p.phone.is_empty() {
                    None
                } else {
                    fuzzy::score(&self.query, &p.phone)
                };
                a.max(b).max(c).map(|s| (s, p))
            })
            .collect();
        scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
        scored.into_iter().take(20).map(|(_, p)| p).collect()
    }

    fn open_patient(&mut self, patient: Patient) {
        self.reload_interviews(patient.id);
        self.edit_patient = None;
        self.confirm_delete = false;
        self.confirm_delete_itv = None;
        self.viewing = Some(patient);
    }

    fn reload_interviews(&mut self, patient_id: i64) {
        match self.db.interviews_for(patient_id) {
            Ok(list) => {
                self.viewing_interviews = list;
                // A successful reload follows a successful operation:
                // stop showing the previous error.
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        if let Ok(counts) = self.db.pending_counts() {
            self.pending = counts;
        }
    }

    /// Load everything the dashboard shows: summaries, appointments, today.
    fn refresh_dashboard(&mut self) {
        match self.db.interview_summaries() {
            Ok(s) => self.summaries = s,
            Err(e) => self.error = Some(e),
        }
        match self.db.upcoming_appointments() {
            Ok(a) => self.appointments = a,
            Err(e) => self.error = Some(e),
        }
        self.today = self.db.today_iso().unwrap_or_default();
        self.tomorrow = self.db.tomorrow_iso().unwrap_or_default();
        self.agenda_week = self.db.week_dates(self.agenda_offset).unwrap_or_default();
        self.export_notice = None;
    }
}

/// Stable color per act kind, for the agenda's week blocks and legend.
fn kind_color(kind: InterviewKind) -> egui::Color32 {
    match kind {
        InterviewKind::Bpm => egui::Color32::from_rgb(0x3a, 0x54, 0x7e),
        InterviewKind::Aod => egui::Color32::from_rgb(0x2e, 0x6e, 0x4e),
        InterviewKind::Asthme => egui::Color32::from_rgb(0x7e, 0x3a, 0x5e),
        InterviewKind::TrodAngine => egui::Color32::from_rgb(0x8b, 0x5a, 0x1a),
        InterviewKind::TrodCystite => egui::Color32::from_rgb(0x1a, 0x6e, 0x8b),
        InterviewKind::Prevention => egui::Color32::from_rgb(0x5e, 0x7e, 0x3a),
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
    /// Last content known to be on disk — the merge base when another
    /// PC saves the shared notes concurrently.
    doc_base: String,
    /// Whether the notes editor currently has keyboard focus (don't
    /// swap the text under the cursor).
    doc_focused: bool,
    doc_check: Instant,
    /// Multi-PC: periodic re-read of what the current view displays.
    last_refresh: Instant,
    /// Master-password change dialog, when open.
    pw_change: Option<PwChangeForm>,
    /// Operator initials for note stamps (default from config.toml).
    operator: String,
    /// Typst template editor, when open: (source text, status message
    /// where `true` marks an error).
    tpl_editor: Option<(String, Option<(bool, String)>)>,
}

#[derive(Default)]
struct PwChangeForm {
    new1: String,
    new2: String,
    error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let doc_text = std::fs::read_to_string(config.team_doc_path())
            .unwrap_or_else(|_| tr("team_doc_template").to_owned());
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
                    spawn_daily_backup(config.db_path(), pw.clone(), config.database.backups_keep);
                    // Demo hook: land on a specific view (screenshots, e2e).
                    match std::env::var("BPM_CADDY_START_VIEW").as_deref() {
                        Ok("dashboard") => {
                            session.refresh_dashboard();
                            session.view = MainView::Dashboard;
                        }
                        Ok("patient") => {
                            if let Some(p) = session.patients.first().cloned() {
                                session.open_patient(p);
                            }
                        }
                        Ok("drugs") => {
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                            }
                            session.view = MainView::Drugs;
                        }
                        Ok("agenda") => {
                            session.refresh_dashboard();
                            session.view = MainView::Agenda;
                        }
                        Ok("drug_card") => {
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                            }
                            let card = session
                                .drugs
                                .iter()
                                .find(|d| d.name == "Eliquis")
                                .or(session.drugs.first())
                                .cloned();
                            session.drug_base = card.clone();
                            session.drug_form = card;
                            session.view = MainView::Drugs;
                        }
                        _ => {}
                    }
                    state = State::Unlocked(Box::new(session));
                    remember_password = true;
                }
                Err(e) => {
                    state = State::Locked {
                        password: String::new(),
                        error: Some(trf("lock_stored_refused", e)),
                    };
                }
            }
        }

        // Screenshot/e2e hook: open the template editor directly.
        let tpl_editor = if std::env::var("BPM_CADDY_START_VIEW").as_deref() == Ok("template") {
            Some((crate::pdf::default_template().to_owned(), None))
        } else {
            None
        };
        Self {
            state,
            operator: config.ui.operator.clone(),
            config,
            last_activity: Instant::now(),
            remember_password,
            show_docs,
            doc_base: doc_text.clone(),
            doc_text,
            doc_dirty: false,
            doc_last_edit: Instant::now(),
            doc_error: None,
            doc_focused: false,
            doc_check: Instant::now(),
            last_refresh: Instant::now(),
            pw_change: None,
            tpl_editor,
        }
    }

    fn save_doc(&mut self) {
        let path = self.config.team_doc_path();
        // Another PC may have saved since our last sync: merge their
        // additions instead of overwriting the whole file with our copy.
        let mut notice = None;
        if let Ok(disk) = std::fs::read_to_string(&path) {
            if disk != self.doc_base && disk != self.doc_text {
                // Never rewrite the text under a focused cursor: skip
                // this save cycle, the merge happens once focus leaves
                // (the debounce keeps retrying while dirty).
                if self.doc_focused {
                    return;
                }
                self.doc_text = merge_team_notes(&self.doc_base, &self.doc_text, &disk);
                notice = Some(tr("docs_merged").to_owned());
            }
        }
        let result = path
            .parent()
            .map(std::fs::create_dir_all)
            .unwrap_or(Ok(()))
            .and_then(|()| std::fs::write(&path, &self.doc_text));
        match result {
            Ok(()) => {
                self.doc_dirty = false;
                self.doc_base = self.doc_text.clone();
                self.doc_error = notice;
            }
            Err(e) => self.doc_error = Some(trf("docs_save_error", e)),
        }
    }

    fn docs_pane(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("team_docs")
            .resizable(true)
            .default_width(340.0)
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.heading(tr("docs_title"));
                let status = if let Some(err) = &self.doc_error {
                    err.clone()
                } else if self.doc_dirty {
                    tr("docs_saving").to_owned()
                } else {
                    tr("docs_saved").to_owned()
                };
                ui.label(status);
                ui.add_space(4.0);
                // Succinct entries: one click stamps date · operator ·
                // current patient, ready to complete.
                let stamp = if let State::Unlocked(s) = &self.state {
                    Some((
                        s.db.now_stamp().unwrap_or_default(),
                        s.viewing.as_ref().map(|p| p.full_name()),
                    ))
                } else {
                    None
                };
                ui.horizontal(|ui| {
                    ui.label(tr("docs_operator"));
                    ui.add_sized([46.0, 22.0], egui::TextEdit::singleline(&mut self.operator));
                    if let Some((now, patient)) = stamp {
                        if motif::button(ui, tr("docs_stamp"))
                            .on_hover_text(tr("docs_stamp_tooltip"))
                            .clicked()
                        {
                            let mut entry = format!("\n— {now}");
                            if !self.operator.trim().is_empty() {
                                entry.push_str(&format!(" · {}", self.operator.trim()));
                            }
                            if let Some(p) = patient {
                                entry.push_str(&format!(" · {p}"));
                            }
                            entry.push_str(" : ");
                            self.doc_text.push_str(&entry);
                            self.doc_dirty = true;
                            self.doc_last_edit = Instant::now();
                        }
                    }
                });
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
                    self.doc_focused = response.has_focus();
                    if response.changed() {
                        self.doc_dirty = true;
                        self.doc_last_edit = Instant::now();
                    }
                });
            });
    }

    fn unlock_screen(&mut self, ctx: &egui::Context) {
        let db_path = self.config.db_path();
        let mut remember = self.remember_password;
        let State::Locked { password, error } = &mut self.state else {
            return;
        };
        let mut attempt: Option<String> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.heading("BPM-Caddy");
                ui.label(tr("lock_subtitle"));
                ui.add_space(20.0);

                let field = ui.add_sized(
                    [300.0, 30.0],
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .hint_text(tr("lock_password_hint")),
                );
                motif::bevel(ui.painter(), field.rect.expand(2.0), false);
                if !ctx.wants_keyboard_input() {
                    field.request_focus();
                }

                let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(10.0);
                ui.checkbox(&mut remember, tr("lock_remember"));
                if (motif::button(ui, tr("lock_unlock")).clicked() || submitted)
                    && !password.is_empty()
                {
                    attempt = Some(password.clone());
                }
                if let Some(err) = error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                }
                ui.add_space(24.0);
                // Which database this post opens — misconfigured posts
                // (wrong network path) are spotted before unlocking.
                ui.label(
                    egui::RichText::new(trf("lock_db_path", db_path.display()))
                        .size(11.0)
                        .color(motif::BG_DARK),
                );
            });
        });

        self.remember_password = remember;
        if let Some(pw) = attempt {
            match Db::open(&self.config.db_path(), &pw).and_then(Session::new) {
                Ok(session) => {
                    spawn_daily_backup(
                        self.config.db_path(),
                        pw.clone(),
                        self.config.database.backups_keep,
                    );
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
            session.flush_date_edits();
            session.view = MainView::Search;
            session.viewing = None;
            session.show_amounts = false;
        }
        // Escape backs out of the dashboard, like everywhere else.
        if session.view == MainView::Dashboard
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
            && !ctx.wants_keyboard_input()
        {
            session.view = MainView::Search;
            session.show_amounts = false;
        }

        let config = self.config.clone();
        let doc = (
            &mut self.doc_text,
            &mut self.doc_dirty,
            &mut self.doc_last_edit,
        );
        egui::CentralPanel::default().show(ctx, |ui| {
            if session.view == MainView::Dashboard {
                Self::dashboard_view(ui, session, &config);
                return;
            }
            if session.view == MainView::Drugs {
                Self::drugs_view(ui, ctx, session, doc);
                return;
            }
            if session.view == MainView::Agenda {
                Self::agenda_view(ui, ctx, session);
                return;
            }
            if let Some(patient) = session.viewing.clone() {
                Self::patient_view(ui, ctx, session, &patient, &config);
                return;
            }

            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("BPM-Caddy");
                ui.label(tr("app_tagline"));
                ui.add_space(24.0);

                let search = ui.add_sized(
                    [420.0, 32.0],
                    egui::TextEdit::singleline(&mut session.query).hint_text(tr("search_hint")),
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
                ui.add_space(4.0);
                let in_progress: i64 = session.pending.values().sum();
                ui.label(
                    egui::RichText::new(trn(
                        "search_totals",
                        &[&session.patients.len(), &in_progress],
                    ))
                    .size(11.0)
                    .color(motif::BG_DARK),
                );
            });
            ui.add_space(12.0);

            let results: Vec<Patient> = session.results().into_iter().cloned().collect();

            if !results.is_empty() {
                // The list can shrink between frames (background refresh
                // from another post): keep the selection in bounds.
                session.selected = session.selected.min(results.len() - 1);
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
                        let selected = i == session.selected;
                        let name = p.full_name();
                        // Highlight the letters the fuzzy query matched.
                        // If only the "Last First" orientation matched,
                        // the row simply shows without highlights.
                        let indices = fuzzy::score_with_indices(&session.query, &name)
                            .map(|(_, idx)| idx)
                            .unwrap_or_default();
                        let base_color = if selected {
                            egui::Color32::WHITE
                        } else {
                            motif::TEXT
                        };
                        let bg = if selected {
                            motif::ACCENT
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let font = egui::FontId::proportional(15.0);
                        let plain = egui::TextFormat {
                            font_id: font.clone(),
                            color: base_color,
                            background: bg,
                            ..Default::default()
                        };
                        let matched = egui::TextFormat {
                            underline: egui::Stroke::new(1.5_f32, base_color),
                            color: if selected { base_color } else { motif::ACCENT },
                            ..plain.clone()
                        };
                        let mut job = egui::text::LayoutJob::default();
                        for (ci, ch) in name.chars().enumerate() {
                            let fmt = if indices.binary_search(&ci).is_ok() {
                                matched.clone()
                            } else {
                                plain.clone()
                            };
                            job.append(&ch.to_string(), 0.0, fmt);
                        }
                        let pending = session.pending.get(&p.id).copied().unwrap_or(0);
                        let mut rest = trf("search_born", db::format_french_date(&p.birth_date));
                        match pending {
                            0 => {}
                            1 => rest.push_str(tr("search_pending_one")),
                            n => rest.push_str(&trf("search_pending_many", n)),
                        }
                        job.append(&rest, 0.0, plain);
                        let row = ui.add(egui::Label::new(job).sense(egui::Sense::click()));
                        if row.clicked() {
                            session.open_patient(p.clone());
                        }
                    }
                });
            } else if !session.query.trim().is_empty() {
                // Before offering creation, re-read the patient list once:
                // another PC may have just created the very patient being
                // searched for (avoids duplicates on a shared database).
                if session.new_patient.is_none() {
                    // Throttled: search.changed() re-arms this every
                    // keystroke, and each re-read hits the (possibly
                    // network) database — once per 1.5 s is enough.
                    if session
                        .dup_check
                        .is_none_or(|t| t.elapsed() > Duration::from_millis(1500))
                    {
                        session.dup_check = Some(Instant::now());
                        if let Ok(list) = session.db.patients() {
                            session.set_patients(list);
                        }
                        if let Ok(counts) = session.db.pending_counts() {
                            session.pending = counts;
                        }
                    }
                    if !session.results().is_empty() {
                        // The refreshed list matches after all: render it
                        // on the next frame instead of the creation form.
                        ctx.request_repaint();
                        return;
                    }
                }
                // No match: the search transitions into quick creation (spec 3.1).
                let form = session.new_patient.get_or_insert_with(Default::default);
                let mut create = false;
                ui.vertical_centered(|ui| {
                    ui.label(tr("search_no_match"));
                    ui.add_space(8.0);
                    let dim = |t: &str| egui::RichText::new(t).color(motif::BG_DARK);
                    let submitted = egui::Grid::new("new_patient")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(dim(tr("form_last_name")));
                            let a = ui.add_sized(
                                [240.0, 26.0],
                                egui::TextEdit::singleline(&mut form.last_name),
                            );
                            ui.end_row();
                            ui.label(dim(tr("form_first_name")));
                            let b = ui.add_sized(
                                [240.0, 26.0],
                                egui::TextEdit::singleline(&mut form.first_name),
                            );
                            ui.end_row();
                            ui.label(dim(tr("form_birth")));
                            let c = ui.add_sized(
                                [240.0, 26.0],
                                egui::TextEdit::singleline(&mut form.birth_date)
                                    .hint_text(tr("form_birth_hint")),
                            );
                            ui.end_row();
                            // Enter in any field submits (spec 3.1: shortcut
                            // driven — no mouse needed to create a patient).
                            [a, b, c].iter().any(|r| r.lost_focus())
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        })
                        .inner;
                    ui.add_space(8.0);
                    if motif::button(ui, tr("search_create")).clicked() || submitted {
                        create = true;
                    }
                    if let Some(err) = &form.error {
                        ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                    }
                });

                if create {
                    // Birth dates read two-digit years as the past ("49" → 1949).
                    let year = session.db.current_year();
                    let outcome = db::parse_french_date(&form.birth_date, year, db::YearHint::Past)
                        .and_then(|iso| {
                            if form.last_name.trim().is_empty() || form.first_name.trim().is_empty()
                            {
                                return Err(tr("form_names_required").to_owned());
                            }
                            session.db.add_patient(
                                form.last_name.trim(),
                                form.first_name.trim(),
                                &iso,
                            )
                        });
                    match outcome {
                        Ok(new_id) => {
                            match session.db.patients() {
                                Ok(list) => session.set_patients(list),
                                Err(e) => session.error = Some(e),
                            }
                            let created = session.patients.iter().find(|p| p.id == new_id).cloned();
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
        // Escape closes the patient view — but while a text field has
        // focus it only drops that focus (egui's own behavior); acting on
        // both at once would throw away an in-progress date edit.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            session.flush_date_edits();
            session.viewing = None;
            return;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if motif::button(ui, tr("patient_back")).clicked() {
                session.flush_date_edits();
                session.viewing = None;
            }
        });
        ui.add_space(12.0);
        let card = ui.available_rect_before_wrap().shrink(6.0);
        motif::bevel(ui.painter(), card, true);
        let mut start_edit = false;
        let mut save_edit = false;
        let mut cancel_edit = false;
        let mut delete_click = false;
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(patient.full_name());
            ui.label(format!(
                "Né(e) le {}",
                db::format_french_date(&patient.birth_date)
            ));
            if !patient.phone.is_empty() {
                ui.label(trf("patient_phone", &patient.phone));
            }
            if !patient.notes.is_empty() {
                ui.label(egui::RichText::new(patient.notes.as_str()).italics());
            }
            ui.add_space(6.0);
            // Identity corrections and removal (mistaken creation).
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 110.0);
                if session.edit_patient.is_none() && motif::button(ui, tr("patient_edit")).clicked()
                {
                    start_edit = true;
                }
                let del_label = if session.confirm_delete {
                    tr("patient_delete_confirm")
                } else {
                    tr("patient_delete")
                };
                if motif::button(ui, del_label).clicked() {
                    delete_click = true;
                }
            });
            if session.confirm_delete {
                ui.colored_label(
                    egui::Color32::from_rgb(0x8b, 0x1a, 0x1a),
                    tr("patient_delete_warning"),
                );
            }
            if let Some(form) = &mut session.edit_patient {
                ui.add_space(8.0);
                let dim = |t: &str| egui::RichText::new(t).color(motif::BG_DARK);
                egui::Grid::new("edit_patient")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(dim(tr("form_last_name")));
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut form.last_name),
                        );
                        ui.end_row();
                        ui.label(dim(tr("form_first_name")));
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut form.first_name),
                        );
                        ui.end_row();
                        ui.label(dim(tr("form_birth")));
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut form.birth_date)
                                .hint_text(tr("form_birth_hint")),
                        );
                        ui.end_row();
                        ui.label(dim(tr("form_phone")));
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut form.phone)
                                .hint_text(tr("form_phone_hint")),
                        );
                        ui.end_row();
                        ui.label(dim(tr("form_comment")));
                        ui.add_sized(
                            [240.0, 26.0],
                            egui::TextEdit::singleline(&mut form.notes)
                                .hint_text(tr("form_comment_hint")),
                        );
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 110.0);
                    if motif::button(ui, tr("form_save")).clicked() {
                        save_edit = true;
                    }
                    if motif::button(ui, tr("form_cancel")).clicked() {
                        cancel_edit = true;
                    }
                });
                if let Some(err) = &form.error {
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                }
            }
            ui.add_space(16.0);

            // Ctrl+N or the buttons below start a new act (spec 3.1).
            ui.label(tr("patient_new_interview"));
            let ctrl_n = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N));
            let mut new_kind: Option<InterviewKind> = None;
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() / 2.0 - 290.0).max(0.0));
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
                ui.label(tr("patient_no_interviews"));
            }
        });

        if start_edit {
            session.confirm_delete = false;
            session.edit_patient = Some(PatientForm {
                last_name: patient.last_name.clone(),
                first_name: patient.first_name.clone(),
                birth_date: db::format_french_date(&patient.birth_date),
                phone: patient.phone.clone(),
                notes: patient.notes.clone(),
                error: None,
            });
        }
        if cancel_edit {
            session.edit_patient = None;
        }
        if save_edit {
            if let Some(form) = session.edit_patient.clone() {
                let year = session.db.current_year();
                let outcome = db::parse_french_date(&form.birth_date, year, db::YearHint::Past)
                    .and_then(|iso| {
                        if form.last_name.trim().is_empty() || form.first_name.trim().is_empty() {
                            return Err(tr("form_names_required").to_owned());
                        }
                        // CAS against the row as displayed: a colleague's
                        // concurrent correction is never wiped.
                        let applied = session.db.update_patient(
                            patient.id,
                            form.last_name.trim(),
                            form.first_name.trim(),
                            &iso,
                            form.phone.trim(),
                            form.notes.trim(),
                            patient,
                        )?;
                        Ok((iso, applied))
                    });
                match outcome {
                    Ok((iso, true)) => {
                        session.viewing = Some(Patient {
                            id: patient.id,
                            last_name: form.last_name.trim().to_owned(),
                            first_name: form.first_name.trim().to_owned(),
                            birth_date: iso,
                            phone: form.phone.trim().to_owned(),
                            notes: form.notes.trim().to_owned(),
                        });
                        if let Ok(list) = session.db.patients() {
                            session.set_patients(list);
                        }
                        session.edit_patient = None;
                    }
                    Ok((_, false)) => {
                        // Stale: reload the fresh row (header updates),
                        // keep the typed form so nothing is lost.
                        if let Ok(list) = session.db.patients() {
                            session.set_patients(list);
                            session.resync_viewing();
                        }
                        if let Some(form) = &mut session.edit_patient {
                            form.error = Some(tr("patient_stale").to_owned());
                        }
                    }
                    Err(e) => {
                        if let Some(form) = &mut session.edit_patient {
                            form.error = Some(e);
                        }
                    }
                }
            }
        }
        if delete_click {
            if session.confirm_delete {
                match session.db.delete_patient(patient.id) {
                    Ok(()) => {
                        session.confirm_delete = false;
                        session.edit_patient = None;
                        session.viewing = None;
                        session.error = None;
                        session.query.clear();
                        if let Ok(list) = session.db.patients() {
                            session.set_patients(list);
                        }
                        return;
                    }
                    Err(e) => session.error = Some(e),
                }
            } else {
                session.confirm_delete = true;
            }
        }

        let interviews = session.viewing_interviews.clone();
        let mut advance: Option<(i64, db::InterviewState)> = None;
        let mut regress: Option<(i64, db::InterviewState)> = None;
        let mut print_req: Option<(InterviewKind, Option<String>)> = None;
        // (interview id, new minutes, the minutes this PC saw — CAS).
        let mut set_duration: Option<(i64, i64, i64)> = None;
        // (interview id, new date, the date this PC saw — CAS expected).
        let mut set_date: Option<(i64, Option<String>, Option<String>)> = None;
        let mut delete_itv: Option<(i64, db::InterviewState)> = None;
        ui.vertical_centered(|ui| {
            // Long histories must not push the table off the card.
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 20.0)
                .show(ui, |ui| {
                    egui::Grid::new("interviews")
                        .num_columns(8)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            if !interviews.is_empty() {
                                for header in [
                                    tr("itv_header_kind"),
                                    tr("itv_header_created"),
                                    tr("itv_header_state"),
                                    tr("itv_header_advance"),
                                    tr("itv_header_sheet"),
                                    tr("itv_header_duration"),
                                    tr("itv_header_rdv"),
                                    "",
                                ] {
                                    ui.label(
                                        egui::RichText::new(header)
                                            .size(11.0)
                                            .color(motif::BG_DARK),
                                    );
                                }
                                ui.end_row();
                            }
                            for itv in &interviews {
                                ui.label(egui::RichText::new(itv.kind.label()).strong());
                                ui.label(db::format_french_date(
                                    &itv.created_at[..10.min(itv.created_at.len())],
                                ))
                                .on_hover_text(tr("itv_created_tooltip"));
                                ui.label(
                                    egui::RichText::new(itv.state.label())
                                        .color(egui::Color32::WHITE)
                                        .background_color(motif::ACCENT),
                                );
                                ui.horizontal(|ui| {
                                    // A misclicked advance is undone with the small
                                    // "«" button (billing states must be correctable).
                                    if itv.state.prev().is_some() {
                                        let back = motif::button(ui, "«");
                                        if back.on_hover_text(tr("itv_back_tooltip")).clicked() {
                                            regress = Some((itv.id, itv.state));
                                        }
                                    }
                                    if let Some(next) = itv.state.next() {
                                        if motif::button(ui, &trf("itv_advance", next.label()))
                                            .clicked()
                                        {
                                            advance = Some((itv.id, itv.state));
                                        }
                                    } else {
                                        ui.label(tr("itv_done"));
                                    }
                                });
                                if motif::button(ui, tr("itv_pdf"))
                                    .on_hover_text(tr("itv_pdf_tooltip"))
                                    .clicked()
                                {
                                    print_req = Some((itv.kind, itv.scheduled_date.clone()));
                                }
                                let mut minutes = itv.duration_minutes;
                                let drag = ui.add(
                                    egui::DragValue::new(&mut minutes)
                                        .range(0..=480)
                                        .suffix(tr("itv_minutes_suffix")),
                                );
                                if drag.changed() {
                                    set_duration = Some((itv.id, minutes, itv.duration_minutes));
                                }
                                // Planned date: free text, committed when it parses
                                // (or empties) and the field loses focus.
                                let text = session.date_edits.entry(itv.id).or_insert_with(|| {
                                    itv.scheduled_date
                                        .as_deref()
                                        .map(db::format_french_date)
                                        .unwrap_or_default()
                                });
                                let field = ui.add_sized(
                                    [100.0, 22.0],
                                    egui::TextEdit::singleline(text).hint_text(tr("itv_rdv_hint")),
                                );
                                if field.lost_focus() {
                                    let year = session.db.current_year();
                                    if text.trim().is_empty() {
                                        if itv.scheduled_date.is_some() {
                                            set_date =
                                                Some((itv.id, None, itv.scheduled_date.clone()));
                                        }
                                    } else if let Ok(iso) =
                                        // RDV dates are always 20xx ("26" → 2026).
                                        db::parse_french_date(
                                            text,
                                            year,
                                            db::YearHint::Future,
                                        )
                                    {
                                        if itv.scheduled_date.as_deref() != Some(iso.as_str()) {
                                            set_date = Some((
                                                itv.id,
                                                Some(iso),
                                                itv.scheduled_date.clone(),
                                            ));
                                        }
                                    }
                                }
                                // Remove a mistakenly added interview (two clicks).
                                let confirm = session.confirm_delete_itv == Some(itv.id);
                                let del = motif::button(
                                    ui,
                                    if confirm {
                                        tr("itv_delete_confirm")
                                    } else {
                                        tr("itv_delete")
                                    },
                                );
                                if del.on_hover_text(tr("itv_delete_tooltip")).clicked() {
                                    if confirm {
                                        delete_itv = Some((itv.id, itv.state));
                                    } else {
                                        session.confirm_delete_itv = Some(itv.id);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        });
        let stale_msg = tr("itv_stale");
        if let Some((id, state)) = advance {
            match session.db.advance_interview(id, state) {
                Ok(true) => {
                    session.error = None;
                    session.reload_interviews(patient.id);
                }
                Ok(false) => {
                    session.reload_interviews(patient.id);
                    session.error = Some(stale_msg.to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, state)) = regress {
            match session.db.regress_interview(id, state) {
                Ok(true) => {
                    session.error = None;
                    session.reload_interviews(patient.id);
                }
                Ok(false) => {
                    session.reload_interviews(patient.id);
                    session.error = Some(stale_msg.to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, minutes, expected)) = set_duration {
            match session.db.set_duration(id, minutes, expected) {
                Ok(true) => session.reload_interviews(patient.id),
                Ok(false) => {
                    session.reload_interviews(patient.id);
                    session.error = Some(stale_msg.to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, state)) = delete_itv {
            session.confirm_delete_itv = None;
            match session.db.delete_interview(id, state) {
                Ok(true) => {
                    session.error = None;
                    session.date_edits.remove(&id);
                    session.reload_interviews(patient.id);
                }
                Ok(false) => {
                    session.reload_interviews(patient.id);
                    session.error = Some(stale_msg.to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, date, expected)) = set_date {
            match session
                .db
                .set_scheduled_date(id, date.as_deref(), expected.as_deref())
            {
                Ok(true) => {
                    session.date_edits.remove(&id);
                    session.reload_interviews(patient.id);
                }
                Ok(false) => {
                    // The date changed on another post: show the fresh
                    // value instead of writing the stale one back.
                    session.date_edits.remove(&id);
                    session.reload_interviews(patient.id);
                    session.error = Some(stale_msg.to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((kind, scheduled)) = print_req {
            // The sheet is dated with the planned RDV when one is set,
            // today otherwise (sheets are usually printed just before).
            let date = scheduled
                .as_deref()
                .map(db::format_french_date)
                .unwrap_or_else(|| {
                    session
                        .db
                        .today_french()
                        .unwrap_or_else(|_| tr("itv_date_fallback").to_owned())
                });
            if let Err(e) =
                crate::pdf::open_interview_sheet(patient, kind, &date, &config.template_path())
            {
                session.error = Some(e);
            }
        }
        if let Some(err) = &session.error {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
            });
        }
    }

    /// Agenda (F4): the upcoming patient appointments grouped by day,
    /// soonest first, overdue days flagged. Clicking an entry opens the
    /// patient; the list is printable.
    fn agenda_view(ui: &mut egui::Ui, ctx: &egui::Context, session: &mut Session) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            session.view = MainView::Search;
            return;
        }
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 90.0);
                ui.heading(tr("agenda_title"));
                if !session.appointments.is_empty()
                    && motif::button(ui, tr("dash_print"))
                        .on_hover_text(tr("dash_print_tooltip"))
                        .clicked()
                {
                    let today = session
                        .db
                        .today_french()
                        .unwrap_or_else(|_| tr("itv_date_fallback").to_owned());
                    if let Err(e) = crate::pdf::open_appointment_list(&session.appointments, &today)
                    {
                        session.error = Some(e);
                    }
                }
            });
            ui.label(tr("agenda_subtitle"));
            ui.add_space(10.0);
        });

        let red = egui::Color32::from_rgb(0x8b, 0x1a, 0x1a);
        let mut open_id: Option<i64> = None;

        // ---- Week grid (default view): Mon..Sun with colored blocks ----
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 190.0);
                if motif::button(ui, "‹")
                    .on_hover_text(tr("agenda_prev_week"))
                    .clicked()
                {
                    session.agenda_offset -= 1;
                    session.agenda_week = session
                        .db
                        .week_dates(session.agenda_offset)
                        .unwrap_or_default();
                }
                if motif::button(ui, tr("agenda_this_week")).clicked() {
                    session.agenda_offset = 0;
                    session.agenda_week = session
                        .db
                        .week_dates(session.agenda_offset)
                        .unwrap_or_default();
                }
                if motif::button(ui, "›")
                    .on_hover_text(tr("agenda_next_week"))
                    .clicked()
                {
                    session.agenda_offset += 1;
                    session.agenda_week = session
                        .db
                        .week_dates(session.agenda_offset)
                        .unwrap_or_default();
                }
                if let Some(monday) = session.agenda_week.first() {
                    ui.label(trf("agenda_week_of", db::format_french_date(monday)));
                }
            });
        });
        ui.add_space(6.0);
        if session.agenda_week.len() == 7 {
            let grid_w = (ui.available_width() - 24.0).min(940.0);
            let (alloc, _) =
                ui.allocate_exact_size(egui::vec2(grid_w.max(420.0), 230.0), egui::Sense::hover());
            let grid = egui::Rect::from_center_size(
                egui::pos2(ui.max_rect().center().x, alloc.center().y),
                alloc.size(),
            );
            ui.painter().rect_filled(grid, 0.0, motif::TROUGH);
            motif::bevel(ui.painter(), grid, false);
            let inner = grid.shrink(4.0);
            let col_w = inner.width() / 7.0;
            for (i, date) in session.agenda_week.clone().iter().enumerate() {
                let col = egui::Rect::from_min_size(
                    egui::pos2(inner.left() + i as f32 * col_w, inner.top()),
                    egui::vec2(col_w, inner.height()),
                );
                if *date == session.today {
                    ui.painter().rect_filled(col, 0.0, motif::BG_HOVER);
                }
                if i > 0 {
                    ui.painter().line_segment(
                        [col.left_top(), col.left_bottom()],
                        egui::Stroke::new(1.0_f32, motif::BG_DARK),
                    );
                }
                // "Lun 24/08" — weekday short + day/month.
                let day = db::weekday_fr(date).unwrap_or("");
                let short: String = day
                    .chars()
                    .take(3)
                    .enumerate()
                    .map(|(k, c)| {
                        if k == 0 {
                            c.to_uppercase().next().unwrap_or(c)
                        } else {
                            c
                        }
                    })
                    .collect();
                let dm = date
                    .get(8..10)
                    .and_then(|d| date.get(5..7).map(|m| format!("{d}/{m}")));
                ui.painter().text(
                    egui::pos2(col.center().x, col.top() + 12.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{short} {}", dm.unwrap_or_default()),
                    egui::FontId::proportional(12.0),
                    if *date == session.today {
                        motif::ACCENT
                    } else {
                        motif::TEXT
                    },
                );
                // Colored blocks, one per RDV of that day.
                let day_rdvs: Vec<&Appointment> = session
                    .appointments
                    .iter()
                    .filter(|r| r.date == *date)
                    .collect();
                let max_blocks = ((col.height() - 28.0) / 24.0) as usize;
                for (bi, rdv) in day_rdvs.iter().take(max_blocks).enumerate() {
                    let block = egui::Rect::from_min_size(
                        egui::pos2(col.left() + 3.0, col.top() + 26.0 + bi as f32 * 24.0),
                        egui::vec2(col.width() - 6.0, 21.0),
                    );
                    ui.painter().rect_filled(block, 0.0, kind_color(rdv.kind));
                    ui.painter().with_clip_rect(block.shrink(2.0)).text(
                        egui::pos2(block.left() + 4.0, block.center().y),
                        egui::Align2::LEFT_CENTER,
                        &rdv.patient_name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                    let resp =
                        ui.interact(block, ui.id().with(("wkblk", i, bi)), egui::Sense::click());
                    let mut hover = format!("{} ({})", rdv.patient_name, rdv.kind.label());
                    if !rdv.phone.is_empty() {
                        hover.push_str(&format!(" — {}", rdv.phone));
                    }
                    if resp.on_hover_text(hover).clicked() {
                        open_id = Some(rdv.patient_id);
                    }
                }
                if day_rdvs.len() > max_blocks {
                    ui.painter().text(
                        egui::pos2(col.center().x, col.bottom() - 8.0),
                        egui::Align2::CENTER_CENTER,
                        format!("+{}", day_rdvs.len() - max_blocks),
                        egui::FontId::proportional(11.0),
                        motif::TEXT,
                    );
                }
            }
            // Legend: one colored chip per act kind.
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() / 2.0 - 260.0).max(0.0));
                    for kind in InterviewKind::ALL {
                        ui.label(
                            egui::RichText::new(format!("  {}  ", kind.label()))
                                .size(11.0)
                                .color(egui::Color32::WHITE)
                                .background_color(kind_color(kind)),
                        );
                    }
                });
            });
        }
        ui.add_space(10.0);

        if session.appointments.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label(tr("agenda_empty"));
            });
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let mut last_date: Option<&str> = None;
                for rdv in &session.appointments {
                    if last_date != Some(rdv.date.as_str()) {
                        last_date = Some(rdv.date.as_str());
                        ui.add_space(10.0);
                        // "Lundi 24/08/2026 — aujourd'hui"
                        let day = db::weekday_fr(&rdv.date).unwrap_or("");
                        let mut header = format!(
                            "{}{} {}",
                            day.chars()
                                .next()
                                .map(|c| c.to_uppercase().to_string())
                                .unwrap_or_default(),
                            day.chars().skip(1).collect::<String>(),
                            db::format_french_date(&rdv.date)
                        );
                        let overdue = !session.today.is_empty() && rdv.date < session.today;
                        let color = if overdue {
                            header.push_str(tr("agenda_overdue"));
                            red
                        } else if rdv.date == session.today {
                            header.push_str(tr("agenda_today"));
                            motif::ACCENT
                        } else if rdv.date == session.tomorrow {
                            header.push_str(tr("agenda_tomorrow"));
                            motif::ACCENT
                        } else {
                            motif::TEXT
                        };
                        ui.label(egui::RichText::new(header).strong().color(color).size(15.0));
                        ui.add_space(2.0);
                    }
                    let mut row = format!("{}   ({})", rdv.patient_name, rdv.kind.label());
                    if !rdv.phone.is_empty() {
                        row.push_str(&format!("   —  {}", rdv.phone));
                    }
                    let resp = ui.add(
                        egui::Label::new(egui::RichText::new(row).size(14.0))
                            .sense(egui::Sense::click()),
                    );
                    if resp.on_hover_text(tr("dash_open_patient")).clicked() {
                        open_id = Some(rdv.patient_id);
                    }
                }
                ui.add_space(12.0);
            });
        });
        if let Some(id) = open_id {
            if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                session.view = MainView::Search;
                session.open_patient(p);
            }
        }
        if let Some(err) = &session.error {
            ui.vertical_centered(|ui| {
                ui.colored_label(red, err.as_str());
            });
        }
    }

    /// Drug reference base (F3): fuzzy search, quick creation, and a
    /// card editor (dosage, interactions, IUP, antidote, notes) with
    /// compare-and-set saves. `doc` is the team-notes buffer so a card
    /// can be inserted into the notes in one click.
    fn drugs_view(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        doc: (&mut String, &mut bool, &mut Instant),
    ) {
        let (doc_text, doc_dirty, doc_last_edit) = doc;
        // Escape closes the card first, then leaves the view.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            if session.drug_form.is_some() {
                session.drug_form = None;
                session.drug_base = None;
                session.confirm_delete_drug = false;
            } else {
                session.view = MainView::Search;
                return;
            }
        }
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading(tr("drug_title"));
            ui.label(tr("drug_subtitle"));
            ui.add_space(16.0);
        });

        if let Some(form) = &mut session.drug_form {
            // ---- Card editor ----
            let card = ui.available_rect_before_wrap().shrink(6.0);
            motif::bevel(ui.painter(), card, true);
            let mut save = false;
            let mut close = false;
            let mut delete = false;
            let mut insert_note = false;
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                // Identity header: brand name big, DCI underneath.
                ui.heading(if form.name.trim().is_empty() {
                    tr("drug_unnamed")
                } else {
                    form.name.trim()
                });
                if !form.dci.trim().is_empty() {
                    ui.label(
                        egui::RichText::new(form.dci.trim())
                            .italics()
                            .color(motif::BG_DARK),
                    );
                }
                if !form.antidote.trim().is_empty() {
                    ui.label(
                        egui::RichText::new(trf("drug_antidote_banner", form.antidote.trim()))
                            .strong()
                            .color(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a)),
                    );
                }
                ui.add_space(12.0);
                let dim = |t: &str| egui::RichText::new(t).color(motif::BG_DARK);
                egui::Grid::new("drug_card")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(dim(tr("drug_name")));
                        ui.add_sized([360.0, 26.0], egui::TextEdit::singleline(&mut form.name));
                        ui.end_row();
                        ui.label(dim(tr("drug_dci")));
                        ui.add_sized([360.0, 26.0], egui::TextEdit::singleline(&mut form.dci));
                        ui.end_row();
                        ui.label(dim(tr("drug_dosage")));
                        ui.add_sized([360.0, 26.0], egui::TextEdit::singleline(&mut form.dosage));
                        ui.end_row();
                        ui.label(dim(tr("drug_ddi")));
                        ui.add_sized(
                            [360.0, 48.0],
                            egui::TextEdit::multiline(&mut form.ddi).desired_rows(2),
                        );
                        ui.end_row();
                        ui.label(dim(tr("drug_iup")));
                        ui.add_sized([360.0, 26.0], egui::TextEdit::singleline(&mut form.iup));
                        ui.end_row();
                        ui.label(dim(tr("drug_antidote")));
                        ui.add_sized(
                            [360.0, 26.0],
                            egui::TextEdit::singleline(&mut form.antidote),
                        );
                        ui.end_row();
                        ui.label(dim(tr("drug_notes")));
                        ui.add_sized(
                            [360.0, 64.0],
                            egui::TextEdit::multiline(&mut form.notes).desired_rows(3),
                        );
                        ui.end_row();
                    });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 230.0);
                    if motif::button(ui, tr("form_save")).clicked() {
                        save = true;
                    }
                    if motif::button(ui, tr("drug_close")).clicked() {
                        close = true;
                    }
                    if motif::button(ui, tr("drug_to_notes"))
                        .on_hover_text(tr("drug_to_notes_tooltip"))
                        .clicked()
                    {
                        insert_note = true;
                    }
                    let del_label = if session.confirm_delete_drug {
                        tr("patient_delete_confirm")
                    } else {
                        tr("patient_delete")
                    };
                    if motif::button(ui, del_label).clicked() {
                        delete = true;
                    }
                });
                if let Some(err) = &session.error {
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                }
            });

            if insert_note {
                let form = session.drug_form.as_ref().unwrap();
                let mut entry = format!("\n- {}", form.name.trim());
                if !form.dci.trim().is_empty() {
                    entry.push_str(&format!(" ({})", form.dci.trim()));
                }
                if !form.dosage.trim().is_empty() {
                    entry.push_str(&format!(" : {}", form.dosage.trim()));
                }
                doc_text.push_str(&entry);
                *doc_dirty = true;
                *doc_last_edit = Instant::now();
            }
            if save {
                let form = session.drug_form.clone().unwrap();
                if form.name.trim().is_empty() {
                    session.error = Some(tr("drug_name_required").to_owned());
                } else if let Some(base) = session.drug_base.clone() {
                    match session.db.update_drug(&form, &base) {
                        Ok(true) => {
                            session.error = None;
                            session.drug_base = Some(form);
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                            }
                        }
                        Ok(false) => {
                            // Reload the fresh card as the new baseline,
                            // keep the typed values so nothing is lost.
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                                session.drug_base =
                                    session.drugs.iter().find(|d| d.id == form.id).cloned();
                            }
                            session.error = Some(tr("drug_stale").to_owned());
                        }
                        Err(e) => session.error = Some(e),
                    }
                }
            }
            if delete {
                if session.confirm_delete_drug {
                    let form = session.drug_form.clone().unwrap();
                    let name = session
                        .drug_base
                        .as_ref()
                        .map(|b| b.name.clone())
                        .unwrap_or_default();
                    match session.db.delete_drug(form.id, &name) {
                        Ok(true) => {
                            session.error = None;
                            session.drug_form = None;
                            session.drug_base = None;
                            session.confirm_delete_drug = false;
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                            }
                        }
                        Ok(false) => {
                            session.confirm_delete_drug = false;
                            session.error = Some(tr("drug_delete_stale").to_owned());
                        }
                        Err(e) => session.error = Some(e),
                    }
                } else {
                    session.confirm_delete_drug = true;
                }
            }
            if close {
                session.drug_form = None;
                session.drug_base = None;
                session.confirm_delete_drug = false;
                session.error = None;
            }
            return;
        }

        // ---- Search / list ----
        let mut open_drug: Option<Drug> = None;
        ui.vertical_centered(|ui| {
            let search = ui.add_sized(
                [420.0, 32.0],
                egui::TextEdit::singleline(&mut session.drug_query)
                    .hint_text(tr("drug_search_hint")),
            );
            motif::bevel(ui.painter(), search.rect.expand(2.0), false);
            if !ctx.wants_keyboard_input() {
                search.request_focus();
            }
            if search.changed() {
                session.drug_selected = 0;
            }
            ui.add_space(12.0);

            let mut scored: Vec<(i32, &Drug)> = session
                .drugs
                .iter()
                .filter_map(|d| {
                    // Brand name and DCI both match ("elix" or "apixa").
                    let a = fuzzy::score(&session.drug_query, &d.name);
                    let b = if d.dci.is_empty() {
                        None
                    } else {
                        fuzzy::score(&session.drug_query, &d.dci)
                    };
                    a.max(b).map(|s| (s, d))
                })
                .collect();
            scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
            let results: Vec<Drug> = scored
                .into_iter()
                .take(20)
                .map(|(_, d)| d.clone())
                .collect();

            if !results.is_empty() {
                session.drug_selected = session.drug_selected.min(results.len() - 1);
                let (up, down, enter) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::Enter),
                    )
                });
                if down {
                    session.drug_selected = (session.drug_selected + 1).min(results.len() - 1);
                }
                if up {
                    session.drug_selected = session.drug_selected.saturating_sub(1);
                }
                if enter {
                    open_drug = Some(results[session.drug_selected].clone());
                }
                for (i, d) in results.iter().enumerate() {
                    let mut text = d.name.clone();
                    if !d.dci.is_empty() {
                        text.push_str(&format!(" ({})", d.dci));
                    }
                    if !d.dosage.is_empty() {
                        text.push_str(&format!("   ·  {}", d.dosage));
                    }
                    if !d.antidote.is_empty() {
                        text.push_str(&trf("drug_row_antidote", &d.antidote));
                    }
                    let label = egui::RichText::new(text).size(15.0);
                    let label = if i == session.drug_selected {
                        label
                            .color(egui::Color32::WHITE)
                            .background_color(motif::ACCENT)
                    } else {
                        label
                    };
                    let row = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
                    if row.clicked() {
                        open_drug = Some(d.clone());
                    }
                }
            } else if !session.drug_query.trim().is_empty() {
                ui.label(tr("drug_no_match"));
                ui.add_space(8.0);
                let name = session.drug_query.trim().to_owned();
                if motif::button(ui, &trf("drug_create", &name)).clicked() {
                    match session.db.add_drug(&name) {
                        Ok(id) => {
                            session.error = None;
                            session.drug_query.clear();
                            if let Ok(list) = session.db.drugs() {
                                session.drugs = list;
                            }
                            open_drug = session.drugs.iter().find(|d| d.id == id).cloned();
                        }
                        Err(e) => session.error = Some(e),
                    }
                }
            }
            if session.drug_form.is_none() {
                if let Some(err) = &session.error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                }
            }
        });
        if let Some(d) = open_drug {
            session.drug_base = Some(d.clone());
            session.drug_form = Some(d);
            session.confirm_delete_drug = false;
            session.error = None;
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

    /// The "Exporter CSV" button with its status line: writes every
    /// interview (with fees) to `exports/` next to the database and
    /// opens the file, for billing reconciliation with the LGO.
    fn export_controls(ui: &mut egui::Ui, session: &mut Session, config: &Config) {
        ui.vertical_centered(|ui| {
            if motif::button(ui, tr("dash_export")).clicked() {
                match session.db.export_rows() {
                    Ok(rows) => {
                        let csv = interviews_csv(&rows, config);
                        let today = if session.today.is_empty() {
                            session.db.today_iso().unwrap_or_default()
                        } else {
                            session.today.clone()
                        };
                        let dir = config
                            .db_path()
                            .parent()
                            .map(|p| p.join("exports"))
                            .unwrap_or_else(|| std::path::PathBuf::from("exports"));
                        let file = dir.join(format!("entretiens-{today}.csv"));
                        let result = std::fs::create_dir_all(&dir)
                            .and_then(|()| std::fs::write(&file, csv.as_bytes()));
                        match result {
                            Ok(()) => {
                                let _ = open::that_detached(&file);
                                session.export_notice = Some(trf("dash_exported", file.display()));
                            }
                            Err(e) => session.error = Some(trf("dash_export_error", e)),
                        }
                    }
                    Err(e) => session.error = Some(e),
                }
            }
            if let Some(notice) = &session.export_notice {
                ui.label(notice.as_str());
            }
        });
    }

    /// Financial & statistical dashboard (spec 3.3): KPIs, pipeline funnel,
    /// monthly billed vs pending revenue.
    fn dashboard_view(ui: &mut egui::Ui, session: &mut Session, config: &Config) {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.heading(tr("dash_title"));
        });
        ui.add_space(6.0);

        // Discreet finances: amounts stay masked at the counter. The
        // reveal control is deliberately unobtrusive — a small unlabeled
        // square in the corner, raised while masked, sunken while shown.
        let masked = config.ui.discreet_finances && !session.show_amounts;
        if config.ui.discreet_finances {
            let rect = egui::Rect::from_min_size(
                egui::pos2(ui.max_rect().right() - 36.0, ui.max_rect().top() + 4.0),
                egui::vec2(26.0, 18.0),
            );
            let resp = ui.interact(rect, ui.id().with("discreet_toggle"), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, if masked { motif::BG } else { motif::TROUGH });
            motif::bevel(ui.painter(), rect, masked);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "•••",
                egui::FontId::proportional(10.0),
                motif::BG_DARK,
            );
            if resp.clicked() {
                session.show_amounts = !session.show_amounts;
            }
        }

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
            let money = |v: &str| {
                if masked {
                    "•••".to_owned()
                } else {
                    v.to_owned()
                }
            };
            Self::kpi_box(
                ui,
                kpi_w,
                tr("dash_billed"),
                &money(&format!("{billed:.0} €")),
            );
            Self::kpi_box(
                ui,
                kpi_w,
                tr("dash_pending"),
                &money(&format!("{pending:.0} €")),
            );
            Self::kpi_box(
                ui,
                kpi_w,
                tr("dash_billed_count"),
                &billed_count.to_string(),
            );
            Self::kpi_box(ui, kpi_w, tr("dash_hourly"), &money(&roi));
        });
        ui.add_space(18.0);

        // Pipeline funnel: one sunken bar per state.
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(tr("dash_pipeline")).strong());
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
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            let per_kind: Vec<String> = InterviewKind::ALL
                .iter()
                .map(|k| {
                    let n = session.summaries.iter().filter(|s| s.kind == *k).count();
                    format!("{} {}", k.label(), n)
                })
                .collect();
            ui.label(
                egui::RichText::new(trf("dash_per_kind", per_kind.join("   ·   "))).size(12.0),
            );
        });
        ui.add_space(14.0);

        // Upcoming appointments: planned interviews not yet performed,
        // soonest first, overdue ones flagged. Clicking a row opens the
        // patient. Not financial data, so never masked.
        if !session.appointments.is_empty() {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 80.0);
                    ui.label(egui::RichText::new(tr("dash_rdv")).strong());
                    // Paper companion for the counter: the full list with
                    // phone numbers, ready to print.
                    if motif::button(ui, tr("dash_print"))
                        .on_hover_text(tr("dash_print_tooltip"))
                        .clicked()
                    {
                        let today = session
                            .db
                            .today_french()
                            .unwrap_or_else(|_| tr("itv_date_fallback").to_owned());
                        if let Err(e) =
                            crate::pdf::open_appointment_list(&session.appointments, &today)
                        {
                            session.error = Some(e);
                        }
                    }
                });
            });
            ui.add_space(6.0);
            let mut open_id: Option<i64> = None;
            let shown = 8.min(session.appointments.len());
            ui.vertical_centered(|ui| {
                for rdv in &session.appointments[..shown] {
                    let overdue = !session.today.is_empty() && rdv.date < session.today;
                    let today = !session.today.is_empty() && rdv.date == session.today;
                    let phone = if rdv.phone.is_empty() {
                        String::new()
                    } else {
                        format!("   —  {}", rdv.phone)
                    };
                    let text = format!(
                        "{}   {}   ({}){}{}",
                        db::format_french_date(&rdv.date),
                        rdv.patient_name,
                        rdv.kind.label(),
                        phone,
                        if overdue {
                            tr("dash_overdue")
                        } else if today {
                            tr("dash_today")
                        } else {
                            ""
                        }
                    );
                    let label = egui::RichText::new(text).size(14.0);
                    let label = if overdue {
                        label.color(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a))
                    } else if today {
                        label.color(motif::ACCENT).strong()
                    } else {
                        label
                    };
                    let row = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
                    if row.on_hover_text(tr("dash_open_patient")).clicked() {
                        open_id = Some(rdv.patient_id);
                    }
                }
                if session.appointments.len() > shown {
                    ui.label(trf("dash_more", session.appointments.len() - shown));
                }
            });
            if let Some(id) = open_id {
                if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                    session.view = MainView::Search;
                    session.show_amounts = false;
                    session.open_patient(p);
                    return;
                }
            }
            ui.add_space(18.0);
        }

        // Monthly revenue: billed (dark blue) vs pending (grey), last 12 months.
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(tr("dash_monthly")).strong());
        });
        ui.add_space(6.0);
        if masked {
            ui.vertical_centered(|ui| {
                ui.label("• • •");
            });
            ui.add_space(14.0);
            Self::export_controls(ui, session, config);
            return;
        }
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
                ui.label(tr("dash_empty"));
            });
            ui.add_space(14.0);
            Self::export_controls(ui, session, config);
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
            ui.label(tr("dash_legend"));
        });
        ui.add_space(14.0);
        Self::export_controls(ui, session, config);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-lock after inactivity (spec 4.3).
        if ctx.input(|i| !i.events.is_empty() || i.pointer.is_moving()) {
            self.last_activity = Instant::now();
        }
        if let State::Unlocked(session) = &mut self.state {
            let timeout = self.config.database.auto_lock_timeout_minutes;
            if timeout > 0 && self.last_activity.elapsed() > Duration::from_secs(timeout * 60) {
                // Don't lose an RDV date typed but never tabbed out of.
                session.flush_date_edits();
                self.state = State::Locked {
                    password: String::new(),
                    error: None,
                };
            }
            ctx.request_repaint_after(Duration::from_secs(30));
        }

        // Multi-PC: other posts write the same database. Re-read what the
        // current view shows every minute (silently — a transient network
        // hiccup on a background refresh is not worth an error banner).
        if let State::Unlocked(session) = &mut self.state {
            if self.last_refresh.elapsed() > Duration::from_secs(60) {
                self.last_refresh = Instant::now();
                if let Ok(list) = session.db.patients() {
                    session.set_patients(list);
                    // Keep the open patient's header in sync too: another
                    // post may have corrected the identity — or deleted
                    // the patient, in which case the view closes.
                    session.resync_viewing();
                }
                if let Ok(counts) = session.db.pending_counts() {
                    session.pending = counts;
                }
                if let Some(pid) = session.viewing.as_ref().map(|p| p.id) {
                    if let Ok(list) = session.db.interviews_for(pid) {
                        session.viewing_interviews = list;
                    }
                }
                if session.view == MainView::Dashboard {
                    if let Ok(s) = session.db.interview_summaries() {
                        session.summaries = s;
                    }
                    if let Ok(a) = session.db.upcoming_appointments() {
                        session.appointments = a;
                    }
                    if let Ok(t) = session.db.today_iso() {
                        session.today = t;
                    }
                }
                if session.view == MainView::Drugs && session.drug_form.is_none() {
                    if let Ok(list) = session.db.drugs() {
                        session.drugs = list;
                    }
                }
                if session.view == MainView::Agenda {
                    if let Ok(a) = session.db.upcoming_appointments() {
                        session.appointments = a;
                    }
                    if let Ok(t) = session.db.today_iso() {
                        session.today = t;
                    }
                    if let Ok(t) = session.db.tomorrow_iso() {
                        session.tomorrow = t;
                    }
                }
            }
        }

        // Multi-PC: pick up teammates' edits to the shared notes while our
        // copy is clean and the cursor is elsewhere; concurrent edits are
        // merged at save time instead.
        if !self.show_docs {
            self.doc_focused = false;
        }
        if self.show_docs
            && matches!(self.state, State::Unlocked(_))
            && !self.doc_dirty
            && !self.doc_focused
            && self.doc_check.elapsed() > Duration::from_secs(3)
        {
            self.doc_check = Instant::now();
            if let Ok(disk) = std::fs::read_to_string(self.config.team_doc_path()) {
                if disk != self.doc_base {
                    self.doc_base = disk.clone();
                    self.doc_text = disk;
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_docs = !self.show_docs;
        }
        let mut toggle_dashboard = ctx.input(|i| i.key_pressed(egui::Key::F2));
        let mut toggle_drugs = ctx.input(|i| i.key_pressed(egui::Key::F3));
        let mut toggle_agenda = ctx.input(|i| i.key_pressed(egui::Key::F4));

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("BPM-Caddy").strong());
                ui.label(
                    egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(11.0)
                        .color(motif::BG_DARK),
                )
                .on_hover_text(format!(
                    "Base : {}\nConfiguration : {}",
                    self.config.db_path().display(),
                    Config::path().display()
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if motif::button(ui, tr("toolbar_docs")).clicked() {
                        self.show_docs = !self.show_docs;
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, tr("toolbar_dashboard")).clicked()
                    {
                        toggle_dashboard = true;
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, tr("toolbar_drugs")).clicked()
                    {
                        toggle_drugs = true;
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, tr("toolbar_agenda")).clicked()
                    {
                        toggle_agenda = true;
                    }
                    if let State::Unlocked(session) = &mut self.state {
                        if motif::button(ui, tr("toolbar_lock")).clicked() {
                            session.flush_date_edits();
                            self.state = State::Locked {
                                password: String::new(),
                                error: None,
                            };
                        }
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, tr("toolbar_password")).clicked()
                    {
                        self.pw_change = if self.pw_change.is_some() {
                            None
                        } else {
                            Some(PwChangeForm::default())
                        };
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::button(ui, tr("toolbar_template")).clicked()
                    {
                        self.tpl_editor = if self.tpl_editor.is_some() {
                            None
                        } else {
                            let path = self.config.template_path();
                            let text = std::fs::read_to_string(&path)
                                .unwrap_or_else(|_| crate::pdf::default_template().to_owned());
                            Some((text, None))
                        };
                    }
                });
            });
            ui.add_space(4.0);
        });

        if toggle_dashboard {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Search | MainView::Drugs | MainView::Agenda => {
                        session.flush_date_edits();
                        session.refresh_dashboard();
                        MainView::Dashboard
                    }
                    MainView::Dashboard => {
                        session.show_amounts = false;
                        MainView::Search
                    }
                };
            }
        }
        if toggle_agenda {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Agenda => MainView::Search,
                    _ => {
                        session.flush_date_edits();
                        session.show_amounts = false;
                        session.refresh_dashboard();
                        MainView::Agenda
                    }
                };
            }
        }
        if toggle_drugs {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Drugs => MainView::Search,
                    _ => {
                        session.flush_date_edits();
                        session.show_amounts = false;
                        match session.db.drugs() {
                            Ok(list) => session.drugs = list,
                            Err(e) => session.error = Some(e),
                        }
                        MainView::Drugs
                    }
                };
            }
        }

        // Master-password change dialog (spec 4.2: key management).
        if !matches!(self.state, State::Unlocked(_)) {
            self.pw_change = None;
        }
        let mut close_pw = false;
        if let (State::Unlocked(session), Some(form)) = (&mut self.state, &mut self.pw_change) {
            egui::Window::new(tr("pw_title"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, -60.0])
                .show(ctx, |ui| {
                    ui.label(tr("pw_body1"));
                    ui.label(tr("pw_body2"));
                    ui.add_space(8.0);
                    egui::Grid::new("pw_change")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(tr("pw_new"));
                            ui.add_sized(
                                [200.0, 26.0],
                                egui::TextEdit::singleline(&mut form.new1).password(true),
                            );
                            ui.end_row();
                            ui.label(tr("pw_confirm"));
                            ui.add_sized(
                                [200.0, 26.0],
                                egui::TextEdit::singleline(&mut form.new2).password(true),
                            );
                            ui.end_row();
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if motif::button(ui, tr("pw_change")).clicked() {
                            if form.new1.is_empty() {
                                form.error = Some(tr("pw_empty").to_owned());
                            } else if form.new1 != form.new2 {
                                form.error = Some(tr("pw_mismatch").to_owned());
                            } else {
                                match session.db.change_password(&form.new1) {
                                    Ok(()) => {
                                        // Keep the OS credential manager in sync
                                        // when it holds a remembered copy.
                                        if let Some(entry) = keyring_entry() {
                                            if entry.get_password().is_ok() {
                                                let _ = entry.set_password(&form.new1);
                                            }
                                        }
                                        close_pw = true;
                                    }
                                    Err(e) => form.error = Some(e),
                                }
                            }
                        }
                        if motif::button(ui, tr("form_cancel")).clicked() {
                            close_pw = true;
                        }
                    });
                    if let Some(err) = &form.error {
                        ui.colored_label(egui::Color32::from_rgb(0x8b, 0x1a, 0x1a), err.as_str());
                    }
                });
        }
        if close_pw {
            self.pw_change = None;
        }

        // Typst template editor: edit the interview sheet's source, with
        // validation and a live PDF preview. Saved next to config.toml
        // (or at [templates] bpm_template_path when configured).
        if !matches!(self.state, State::Unlocked(_)) {
            self.tpl_editor = None;
        }
        let mut close_tpl = false;
        if let Some((text, message)) = &mut self.tpl_editor {
            let path = self.config.template_path();
            egui::Window::new(tr("tpl_title"))
                .collapsible(false)
                .resizable(true)
                .default_size([680.0, 520.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(trf("tpl_path", path.display()))
                            .size(11.0)
                            .color(motif::BG_DARK),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(380.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 372.0],
                                egui::TextEdit::multiline(text)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor(),
                            );
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if motif::button(ui, tr("form_save")).clicked() {
                            match crate::pdf::check_template(text) {
                                Ok(()) => {
                                    let result = path
                                        .parent()
                                        .map(std::fs::create_dir_all)
                                        .unwrap_or(Ok(()))
                                        .and_then(|()| std::fs::write(&path, text.as_bytes()));
                                    *message = Some(match result {
                                        Ok(()) => (false, trf("tpl_saved", path.display())),
                                        Err(e) => (true, trf("tpl_save_error", e)),
                                    });
                                }
                                Err(e) => *message = Some((true, e)),
                            }
                        }
                        if motif::button(ui, tr("tpl_preview"))
                            .on_hover_text(tr("tpl_preview_tooltip"))
                            .clicked()
                        {
                            if let Err(e) = crate::pdf::preview_template(text) {
                                *message = Some((true, e));
                            }
                        }
                        if motif::button(ui, tr("tpl_reset"))
                            .on_hover_text(tr("tpl_reset_tooltip"))
                            .clicked()
                        {
                            *text = crate::pdf::default_template().to_owned();
                            *message = None;
                        }
                        if motif::button(ui, tr("tpl_close")).clicked() {
                            close_tpl = true;
                        }
                    });
                    if let Some((is_error, msg)) = message {
                        ui.add_space(4.0);
                        let color = if *is_error {
                            egui::Color32::from_rgb(0x8b, 0x1a, 0x1a)
                        } else {
                            motif::ACCENT
                        };
                        ui.colored_label(color, msg.as_str());
                    }
                });
        }
        if close_tpl {
            self.tpl_editor = None;
        }

        // The docs pane may hold patient-adjacent notes: never show it on
        // the lock screen.
        if self.show_docs && matches!(self.state, State::Unlocked(_)) {
            self.docs_pane(ctx);
        }

        // Debounced auto-save runs even when the pane is hidden.
        if self.doc_dirty && self.doc_last_edit.elapsed() > Duration::from_millis(1200) {
            self.save_doc();
        }
        if self.doc_dirty {
            ctx.request_repaint_after(Duration::from_millis(300));
        }

        match self.state {
            State::Locked { .. } => self.unlock_screen(ctx),
            State::Unlocked(_) => self.main_screen(ctx),
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.doc_dirty {
            // No cursor to protect on the way out: allow the merge.
            self.doc_focused = false;
            self.save_doc();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::merge_team_notes;
    use super::{interviews_csv, Config};
    use crate::db::{ExportRow, InterviewKind, InterviewState};

    #[test]
    fn csv_export_is_french_excel_friendly() {
        let rows = [ExportRow {
            patient_name: "Jean; \"Le Grand\" Dupont".to_owned(),
            phone: "06 12 34 56 78".to_owned(),
            birth_date: "1958-07-03".to_owned(),
            kind: InterviewKind::Bpm,
            state: InterviewState::Billed,
            created_date: "2026-08-23".to_owned(),
            scheduled_date: Some("2026-09-01".to_owned()),
            duration_minutes: 45,
        }];
        let csv = interviews_csv(&rows, &Config::default());
        // BOM so Excel decodes UTF-8 accents, semicolons, CRLF.
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("Patient;Téléphone;Naissance;Type"));
        // The tricky name is quoted with doubled inner quotes.
        assert!(csv
            .contains("\"Jean; \"\"Le Grand\"\" Dupont\";06 12 34 56 78;03/07/1958;BPM;Facturé;"));
        // Billed row: tariff and billed columns both carry the fee.
        assert!(csv.contains("23/08/2026;01/09/2026;45;60,00;60,00\r\n"));
        // Unbilled row: the "Facturé" column stays at zero.
        let mut pending = rows[0].clone();
        pending.state = InterviewState::Performed;
        let csv = interviews_csv(&[pending], &Config::default());
        assert!(csv.contains(";60,00;0,00\r\n"));
    }

    #[test]
    fn concurrent_note_edits_are_merged() {
        let base = "# Notes\n\n- a\n- b\n";
        let ours = "# Notes\n\n- a\n- b\n- ajout local\n";
        let theirs = "# Notes\n\n- a\n- b\n- ajout autre poste\n";
        let merged = merge_team_notes(base, ours, theirs);
        assert!(merged.contains("- ajout local"));
        assert!(merged.contains("- ajout autre poste"));
    }

    #[test]
    fn our_deletions_are_not_resurrected() {
        let base = "- a\n- b\n";
        let ours = "- b\n";
        // The other PC did not touch the file since base.
        assert_eq!(merge_team_notes(base, ours, base), ours);
    }

    #[test]
    fn identical_notes_merge_to_themselves() {
        let base = "# Notes\n\n- a\n";
        assert_eq!(merge_team_notes(base, base, base), base);
    }

    #[test]
    fn daily_backup_creates_once_and_prunes() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-dbak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("live.db");
        let db = crate::db::Db::open(&db_path, "secret").unwrap();
        db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();

        // 16 stale dated snapshots already exist on the shared drive.
        let bdir = dir.join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        for day in 1..=16 {
            std::fs::write(bdir.join(format!("bpm_caddy-2000-01-{day:02}.db")), b"old").unwrap();
        }

        super::daily_backup(&db, &db_path, 14);
        let today = db.today_iso().unwrap();
        let todays = bdir.join(format!("bpm_caddy-{today}.db"));
        assert!(todays.exists());
        // Pruned to the 14 newest; the oldest fakes are gone, today's kept.
        let count = std::fs::read_dir(&bdir).unwrap().count();
        assert_eq!(count, 14);
        assert!(!bdir.join("bpm_caddy-2000-01-01.db").exists());

        // Running again the same day is a no-op (no duplicate, no churn).
        let before = std::fs::metadata(&todays).unwrap().modified().unwrap();
        super::daily_backup(&db, &db_path, 14);
        assert_eq!(
            std::fs::metadata(&todays).unwrap().modified().unwrap(),
            before
        );

        // keep = 0 disables backups entirely.
        std::fs::remove_dir_all(&bdir).unwrap();
        super::daily_backup(&db, &db_path, 0);
        assert!(!bdir.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
