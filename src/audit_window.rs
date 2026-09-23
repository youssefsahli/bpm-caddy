//! `bpm-audit` — la fenêtre d'audit de l'officine.
//!
//! **Un programme à part, sur la même lecture de la base.** Le relevé
//! d'audit, le journal des accès et les compteurs d'usage vivaient dans
//! Options › À propos, c'est-à-dire dans la boîte de réglages d'un poste
//! de comptoir : un endroit où l'on ne cherche pas « qui a ouvert quoi ce
//! mois-ci », et d'où l'on ne peut rien comparer. Ils ont maintenant leur
//! fenêtre, qu'on ouvre depuis l'arrière-boutique, sur la période qu'on
//! choisit.
//!
//! Ce module ne sait rien qu'`audit.rs`, `telemetry.rs` et `db.rs` ne
//! sachent déjà : il **lit** leurs relevés et les dessine. Il n'écrit
//! rien dans la base — une fenêtre d'audit qui modifierait ce qu'elle
//! audite ne prouverait rien. Elle n'ouvre aucun dossier : elle compte
//! des accès, elle n'en fait pas.
//!
//! Les deux règles opposées des deux relevés restent visibles à
//! l'écran : l'usage compte le logiciel et ne nomme personne, les accès
//! nomment la personne et ne disent aucun patient — un numéro de dossier,
//! jamais un nom.

use crate::audit;
use crate::config::Config;
use crate::db::{self, Db};
use crate::strings::{tr, trf, trn};
use eframe::egui;

/// Les périodes qu'on relit, en jours.
pub const PERIODS: [i64; 4] = [7, 30, 90, 365];

/// Ce que la fenêtre a lu, **une fois** par période choisie : des
/// agrégats sur des tables qui grossissent, jamais recalculés à chaque
/// image.
pub struct Reading {
    pub head: audit::Head,
    pub activity: audit::Activity,
    pub access: audit::Summary,
    pub conformity: audit::Conformity,
    pub counters: Vec<(&'static str, u64)>,
    pub span: crate::telemetry::Span,
    /// Le rapport en texte — ce que « Copier » et « Enregistrer »
    /// rendent, et ce que `bpm-caddy audit` imprime : une seule écriture.
    pub report: String,
}

/// Lire la période qui finit aujourd'hui.
pub fn read(db: &Db, cfg: &Config, path: &std::path::Path, days: i64) -> Reading {
    let today = db.today_iso().unwrap_or_default();
    let from = crate::date::add_days(&today, -(days - 1)).unwrap_or_else(|| today.clone());
    let head = audit::Head {
        officine: db
            .officine()
            .map(|o| o.name)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| cfg.pharmacy.name.clone()),
        base: path.display().to_string(),
        made_on: db::format_french_date(&today),
        from: db::format_french_date(&from),
        to: db::format_french_date(&today),
        days: days as usize,
    };
    let activity = db.audit_activity(&from, &today).unwrap_or_default();
    let access = audit::summarize(&db.accesses_since(&from).unwrap_or_default());
    let conformity = db
        .audit_conformity(
            &today,
            i64::from(cfg.stock.count_days),
            cfg.locations.notice_days,
        )
        .unwrap_or_default();
    let mut counts = [0u64; crate::telemetry::Signal::ALL.len()];
    for (key, n) in db.telemetry_totals().unwrap_or_default() {
        if let Some(i) = crate::telemetry::Signal::from_key(&key).and_then(|signal| {
            crate::telemetry::Signal::ALL
                .iter()
                .position(|s| *s == signal)
        }) {
            counts[i] = n;
        }
    }
    let counters = crate::telemetry::Signal::ALL
        .iter()
        .zip(counts)
        .map(|(signal, n)| (signal.label(), n))
        .collect();
    let report = audit::render(&head, &activity, &access, &conformity);
    Reading {
        head,
        activity,
        access,
        conformity,
        counters,
        span: db.telemetry_span(),
        report,
    }
}

enum State {
    Locked {
        password: String,
        error: Option<String>,
    },
    Open {
        db: Box<Db>,
        reading: Box<Reading>,
    },
}

pub struct AuditApp {
    cfg: Config,
    path: std::path::PathBuf,
    days: i64,
    state: State,
    note: Option<(bool, String)>,
    themed: bool,
}

impl AuditApp {
    /// Ouvre la base sans rien demander quand le mot de passe est déjà là
    /// — `BPM_CADDY_PASSWORD` ou le trousseau du système —, et sinon
    /// présente l'écran de déverrouillage.
    pub fn new() -> Self {
        let cfg = Config::load();
        let path = cfg.db_path();
        let mut app = Self {
            cfg,
            path,
            days: 30,
            state: State::Locked {
                password: String::new(),
                error: None,
            },
            note: None,
            themed: false,
        };
        let silent = std::env::var("BPM_CADDY_PASSWORD").ok().or_else(|| {
            if std::env::var("BPM_CADDY_NO_KEYRING").is_ok() {
                None
            } else {
                crate::app::keyring_entry().and_then(|e| e.get_password().ok())
            }
        });
        if let Some(pw) = silent {
            app.unlock(&pw);
        }
        app
    }

    fn unlock(&mut self, password: &str) {
        match Db::open(&self.path, password) {
            Ok(db) => {
                let reading = read(&db, &self.cfg, &self.path, self.days);
                self.state = State::Open {
                    db: Box::new(db),
                    reading: Box::new(reading),
                };
            }
            Err(e) => {
                self.state = State::Locked {
                    password: String::new(),
                    error: Some(audit::unreadable(&e)),
                };
            }
        }
    }

    fn reread(&mut self) {
        if let State::Open { db, reading } = &mut self.state {
            **reading = read(db, &self.cfg, &self.path, self.days);
        }
    }
}

impl Default for AuditApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for AuditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.themed {
            self.cfg.apply_theme();
            motif::apply(ctx);
            motif::apply_scale(ctx, self.cfg.ui.text_scale, self.cfg.density());
            self.themed = true;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(motif::bg()).inner_margin(8.0))
            .show(ctx, |ui| match &mut self.state {
                State::Locked { password, error } => {
                    let mut attempt: Option<String> = None;
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.25);
                        ui.heading(tr("auditw_title"));
                        ui.add_space(8.0);
                        ui.label(tr("auditw_unlock"));
                        let field = motif::field(
                            ui,
                            motif::pt(ui, 11.0) * 24.0,
                            egui::TextEdit::singleline(password).password(true),
                        );
                        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            attempt = Some(password.clone());
                        }
                        if motif::button(ui, tr("auditw_open")).clicked() {
                            attempt = Some(password.clone());
                        }
                        if let Some(e) = error {
                            ui.colored_label(motif::alert(), e.as_str());
                        }
                    });
                    if let Some(pw) = attempt {
                        self.unlock(&pw);
                    }
                }
                State::Open { reading, .. } => {
                    let mut pick: Option<i64> = None;
                    let mut refresh = false;
                    let mut copy = false;
                    let mut save = false;
                    draw(
                        ui,
                        reading,
                        self.days,
                        &self.note,
                        &mut pick,
                        &mut refresh,
                        &mut copy,
                        &mut save,
                    );
                    if copy {
                        ui.ctx().copy_text(reading.report.clone());
                        self.note = Some((false, tr("auditw_copied").to_owned()));
                    }
                    if save {
                        let report = reading.report.clone();
                        if let Some(to) = rfd::FileDialog::new()
                            .set_file_name(format!("audit-{}.txt", self.days))
                            .save_file()
                        {
                            self.note = Some(match std::fs::write(&to, report) {
                                Ok(()) => (false, trf("auditw_saved", to.display())),
                                Err(e) => (true, e.to_string()),
                            });
                        }
                    }
                    if let Some(days) = pick {
                        self.days = days;
                        refresh = true;
                    }
                    if refresh {
                        self.reread();
                    }
                }
            });
    }
}

/// La fenêtre ouverte : l'en-tête et ses gestes, puis quatre volets.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ui: &mut egui::Ui,
    r: &Reading,
    days: i64,
    note: &Option<(bool, String)>,
    pick: &mut Option<i64>,
    refresh: &mut bool,
    copy: &mut bool,
    save: &mut bool,
) {
    let body = motif::visible_rect(ui);
    let row = motif::button_height(ui);
    let gap = ui.spacing().item_spacing.y;
    // L'en-tête : le titre, la période, les gestes — enveloppé, et
    // compté sur ce qu'il dessine plutôt que sur une hauteur écrite.
    let head_h = row * 3.0 + gap * 3.0;
    let rows = motif::split_rows(body, &[head_h, 0.0], 8.0);
    motif::inside(ui, rows[0], |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading(trf("auditw_heading", &r.head.officine));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(tr("auditw_period"))
                    .size(motif::pt(ui, 11.0))
                    .color(motif::text_dim()),
            );
            for d in PERIODS {
                if motif::toggle(ui, &trn("auditw_days", &[&d]), d == days).clicked() {
                    *pick = Some(d);
                }
            }
            if motif::button(ui, tr("auditw_refresh")).clicked() {
                *refresh = true;
            }
            if motif::button(ui, tr("auditw_copy")).clicked() {
                *copy = true;
            }
            if motif::button(ui, tr("auditw_save")).clicked() {
                *save = true;
            }
        });
        let said = match note {
            Some((_, n)) => n.clone(),
            None => trf(
                "auditw_span",
                format!("{} — {} · {}", r.head.from, r.head.to, r.head.base),
            ),
        };
        ui.label(egui::RichText::new(said).size(motif::pt(ui, 10.5)).color(
            if note.as_ref().is_some_and(|(bad, _)| *bad) {
                motif::alert()
            } else {
                motif::text_faint()
            },
        ));
    });
    let grid = motif::split_rows(rows[1], &[0.0, 0.0], 8.0);
    let top = motif::split_columns(grid[0], 2, 8.0);
    let bottom = motif::split_columns(grid[1], 2, 8.0);
    pane(ui, top[0], tr("auditw_activity"), "auditw_activity", |ui| {
        activity(ui, &r.activity)
    });
    pane(ui, top[1], tr("auditw_access"), "auditw_access", |ui| {
        access(ui, &r.access)
    });
    pane(ui, bottom[0], tr("auditw_usage"), "auditw_usage", |ui| {
        usage(ui, &r.counters, &r.span)
    });
    pane(ui, bottom[1], tr("auditw_checks"), "auditw_checks", |ui| {
        checks(ui, &r.conformity)
    });
}

/// Un volet qui défile, barre visible : ce qu'il y a dessous est la
/// suite du relevé, et une barre flottante le cacherait.
fn pane(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    salt: &str,
    add: impl FnOnce(&mut egui::Ui),
) {
    motif::panel(ui, rect, Some(title), |ui| {
        ui.spacing_mut().scroll.floating = false;
        egui::ScrollArea::vertical()
            .id_salt(salt)
            .auto_shrink([false, false])
            .show(ui, add);
    });
}

/// Une ligne « libellé … nombre ».
fn count_row(ui: &mut egui::Ui, label: &str, n: String) {
    ui.label(label);
    ui.label(egui::RichText::new(n).strong());
    ui.end_row();
}

/// Ce qu'un volet écrit quand il ne trouve rien — en toutes lettres : un
/// blanc se lit comme une panne.
fn nothing(ui: &mut egui::Ui, key: &'static str) {
    ui.label(
        egui::RichText::new(tr(key))
            .size(motif::pt(ui, 11.0))
            .color(motif::text_dim()),
    );
}

fn activity(ui: &mut egui::Ui, a: &audit::Activity) {
    if a.acts.is_empty() && a.register_lines == 0 && a.till_evenings == 0 {
        nothing(ui, "auditw_activity_none");
        return;
    }
    egui::Grid::new("auditw_acts")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (kind, n) in &a.acts {
                count_row(ui, kind, n.to_string());
            }
            count_row(ui, tr("auditw_register"), a.register_lines.to_string());
            count_row(ui, tr("auditw_till"), a.till_evenings.to_string());
            let gap = match a.till_gap_cents {
                Some(g) => trf(
                    "auditw_till_gap",
                    format!(
                        "{}{} € ({})",
                        if g > 0 { "+" } else { "" },
                        crate::caisse::euros(g),
                        trn("auditw_evenings", &[&a.till_expected_evenings])
                    ),
                ),
                None => tr("auditw_till_no_gap").to_owned(),
            };
            ui.label(tr("auditw_till_gap_label"));
            ui.label(gap);
            ui.end_row();
        });
    if !a.by_operator.is_empty() {
        ui.add_space(6.0);
        motif::section(ui, tr("auditw_by_operator"));
        egui::Grid::new("auditw_acts_who")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (who, n) in &a.by_operator {
                    let who = if who.trim().is_empty() { "—" } else { who };
                    count_row(ui, who, n.to_string());
                }
            });
    }
}

fn access(ui: &mut egui::Ui, s: &audit::Summary) {
    if s.lines == 0 {
        nothing(ui, "auditw_access_none");
        return;
    }
    ui.label(
        egui::RichText::new(trf(
            "auditw_access_span",
            format!(
                "{} — {} · {}",
                db::format_french_date(&s.first),
                db::format_french_date(&s.last),
                trn("auditw_files", &[&s.files])
            ),
        ))
        .size(motif::pt(ui, 11.0))
        .color(motif::text_dim()),
    );
    egui::Grid::new("auditw_access_acts")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (act, n) in &s.by_act {
                count_row(ui, act.label(), n.to_string());
            }
        });
    ui.add_space(6.0);
    motif::section(ui, tr("auditw_by_operator"));
    egui::Grid::new("auditw_access_who")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (who, n) in &s.by_operator {
                count_row(ui, who, n.to_string());
            }
        });
}

fn usage(ui: &mut egui::Ui, counters: &[(&'static str, u64)], span: &crate::telemetry::Span) {
    if span.days == 0 {
        nothing(ui, "auditw_usage_none");
        return;
    }
    ui.label(
        egui::RichText::new(trf(
            "auditw_usage_span",
            format!(
                "{} — {} ({})",
                db::format_french_date(&span.first),
                db::format_french_date(&span.last),
                trn("auditw_days", &[&span.days])
            ),
        ))
        .size(motif::pt(ui, 11.0))
        .color(motif::text_dim()),
    );
    egui::Grid::new("auditw_usage_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (label, n) in counters {
                count_row(ui, label, n.to_string());
            }
        });
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(tr("auditw_usage_rule"))
            .size(motif::pt(ui, 10.5))
            .color(motif::text_faint()),
    );
}

fn checks(ui: &mut egui::Ui, c: &audit::Conformity) {
    egui::Grid::new("auditw_checks_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            count_row(
                ui,
                tr("auditw_uncounted"),
                c.uncounted_stups.len().to_string(),
            );
            count_row(ui, tr("auditw_rentals"), c.overdue_rentals.to_string());
            count_row(ui, tr("auditw_rewrites"), c.outdated_rewrites.to_string());
            count_row(ui, tr("auditw_no_dci"), c.cards_without_dci.to_string());
            count_row(ui, tr("auditw_no_class"), c.cards_without_class.to_string());
            count_row(
                ui,
                tr("auditw_unknown_classes"),
                c.unknown_classes.len().to_string(),
            );
        });
    if !c.uncounted_stups.is_empty() {
        ui.add_space(6.0);
        motif::section(ui, tr("auditw_uncounted"));
        for (name, since) in &c.uncounted_stups {
            let said = match since {
                Some(d) => trn("auditw_since_days", &[d]),
                None => tr("auditw_never_counted").to_owned(),
            };
            ui.label(format!("{name} — {said}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La fenêtre se dessine sur une base vide sans rien laisser en
    /// blanc** : chaque volet qui ne trouve rien le dit en toutes
    /// lettres, et rien ne panique — ni à l'échelle 1, ni à 1,6.
    #[test]
    fn the_audit_window_draws_an_empty_base_in_words() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-auditw-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = crate::db::Swept(dir.clone());
        let path = dir.join("auditw.db");
        let db = Db::open(&path, "secret").unwrap();
        let cfg = Config::default();
        let reading = read(&db, &cfg, &path, 30);
        assert!(reading.report.contains(&reading.head.from));
        for scale in [1.0, 1.6] {
            let ctx = egui::Context::default();
            motif::apply(&ctx);
            motif::apply_scale(&ctx, scale, motif::Density::Comfortable);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1024.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (mut pick, mut refresh, mut copy, mut save) =
                            (None, false, false, false);
                        draw(
                            ui,
                            &reading,
                            30,
                            &None,
                            &mut pick,
                            &mut refresh,
                            &mut copy,
                            &mut save,
                        );
                    });
                },
            );
        }
    }
}
