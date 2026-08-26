use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::{ActFees, Config, RuleEnforcement};
use crate::db::{
    self, Appointment, Db, Drug, Interview, InterviewKind, InterviewState, InterviewSummary, Note,
    NoteSubject, Patient,
};
use crate::fuzzy;
use crate::strings::{tr, trf, trn};
use crate::vaccines;

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
        "\u{feff}Patient;Téléphone;Naissance;Type;Code acte;Année;Étape;Thème;État;Créé le;RDV;\
         Durée (min);À distance;Changement de traitement;Situation;Honoraires (€);\
         Prise en charge (%);Facturé (€)\r\n",
    );
    for r in rows {
        let comma = |v: f64| format!("{v:.2}").replace('.', ",");
        // "Honoraires" is the tariff; "Facturé" only counts once the
        // interview is billed, so summing that column matches the
        // dashboard's billed revenue.
        let fee = config.act_total(r.kind, r.fee_year, r.fee_rank, r.remote);
        let billed = if r.state == InterviewState::Billed {
            fee
        } else {
            0.0
        };
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\r\n",
            field(&r.patient_name),
            field(&r.phone),
            db::format_french_date(&r.birth_date),
            r.kind.label(),
            r.kind.act_code(r.fee_year).unwrap_or(""),
            r.fee_year + 1,
            field(
                r.kind
                    .step_label(r.fee_year, r.fee_rank)
                    .unwrap_or(&rank_label(r.fee_rank))
            ),
            field(&r.theme),
            r.state.label(),
            db::format_french_date(&r.created_date),
            r.scheduled_date
                .as_deref()
                .map(db::format_french_date)
                .unwrap_or_default(),
            r.duration_minutes,
            if r.remote { db::REMOTE_CODE } else { "" },
            if r.treatment_change && r.kind.allows_treatment_change() {
                tr("csv_yes")
            } else {
                ""
            },
            field(
                db::situation_label(&r.situation)
                    .map(tr)
                    .unwrap_or(&r.situation)
            ),
            comma(fee),
            r.kind.coverage_rate(),
            comma(billed),
        ));
    }
    out
}

/// The acts still to invoice — performed but not yet billed — turned
/// into the lines of the printable recap.
fn billing_lines(rows: &[db::ExportRow], config: &Config) -> Vec<crate::pdf::BillingLine> {
    rows.iter()
        .filter(|r| r.state == InterviewState::Performed)
        .map(|r| crate::pdf::BillingLine {
            date: r
                .scheduled_date
                .clone()
                .unwrap_or_else(|| r.created_date.clone()),
            patient: r.patient_name.clone(),
            kind: r.kind.label().to_owned(),
            code: r.kind.act_code(r.fee_year).unwrap_or("—").to_owned(),
            step: r
                .kind
                .step_label(r.fee_year, r.fee_rank)
                .map(|s| s.to_owned())
                .unwrap_or_else(|| rank_label(r.fee_rank)),
            situation: db::situation_label(&r.situation)
                .map(|k| tr(k).to_owned())
                .unwrap_or_else(|| r.situation.clone()),
            remote: r.remote,
            coverage: r.kind.coverage_rate(),
            fee: config.act_total(r.kind, r.fee_year, r.fee_rank, r.remote),
        })
        .collect()
}

/// A multiline field that keeps the height it is given. `TextEdit`
/// grows with its content, so a field holding a full monograph section
/// overran the rows under it and drew over their labels; the text
/// scrolls inside a fixed sunken box instead.
fn field_box(ui: &mut egui::Ui, id: &str, width: f32, height: f32, text: &mut String) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    motif::bevel(ui.painter(), rect, false);
    let inner = rect.shrink(3.0);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(inner.height())
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.add(
                egui::TextEdit::multiline(text)
                    .desired_width(inner.width() - 8.0)
                    .frame(false),
            );
        });
}

/// The clinical half of the editable drug card.
fn drug_form_clinical(ui: &mut egui::Ui, form: &mut Drug) {
    let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
    motif::section(ui, tr("drug_sec_clinical"));
    ui.add_space(4.0);
    let w = (ui.available_width() - 118.0).max(140.0);
    egui::Grid::new("drug_card")
        .num_columns(2)
        .min_col_width(90.0)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label(dim(tr("drug_name")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.name));
            ui.end_row();
            ui.label(dim(tr("drug_dci")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.dci));
            ui.end_row();
            ui.label(dim(tr("drug_class")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.class));
            ui.end_row();
            ui.label(dim(tr("drug_sec_indications")));
            field_box(ui, "fld_indications", w, 84.0, &mut form.indications);
            ui.end_row();
            ui.label(dim(tr("drug_sec_mechanism")));
            field_box(ui, "fld_mechanism", w, 84.0, &mut form.mechanism);
            ui.end_row();
            ui.label(dim(tr("drug_dosage")));
            field_box(ui, "fld_dosage", w, 84.0, &mut form.dosage);
            ui.end_row();
            ui.label(dim(tr("drug_sec_ci")));
            field_box(
                ui,
                "fld_contraindications",
                w,
                84.0,
                &mut form.contraindications,
            );
            ui.end_row();
            ui.label(dim(tr("drug_ddi")));
            field_box(ui, "fld_ddi", w, 84.0, &mut form.ddi);
            ui.end_row();
            ui.label(dim(tr("drug_sec_adverse")));
            field_box(ui, "fld_adverse", w, 84.0, &mut form.adverse);
            ui.end_row();
            ui.label(dim(tr("drug_sec_monitoring")));
            field_box(ui, "fld_monitoring", w, 84.0, &mut form.monitoring);
            ui.end_row();
            ui.label(dim(tr("drug_iup")));
            field_box(ui, "fld_iup", w, 150.0, &mut form.iup);
            ui.end_row();
            ui.label(dim(tr("drug_antidote")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.antidote));
            ui.end_row();
            ui.label(dim(tr("drug_notes")));
            field_box(ui, "fld_notes", w, 64.0, &mut form.notes);
            ui.end_row();
        });
}

/// The pharmacokinetic half.
fn drug_form_pk(ui: &mut egui::Ui, form: &mut Drug) {
    let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
    motif::section(ui, tr("drug_sec_pk"));
    ui.add_space(4.0);
    let w = (ui.available_width() - 138.0).max(130.0);
    egui::Grid::new("drug_pk")
        .num_columns(2)
        .min_col_width(110.0)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label(dim(tr("drug_half_life")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.half_life));
            ui.end_row();
            ui.label(dim(tr("drug_auc")));
            field_box(ui, "fld_auc", w, 64.0, &mut form.auc);
            ui.end_row();
            ui.label(dim(tr("drug_elimination")));
            field_box(ui, "fld_elimination", w, 76.0, &mut form.elimination);
            ui.end_row();
            ui.label(dim(tr("drug_renal")));
            field_box(ui, "fld_renal", w, 84.0, &mut form.renal);
            ui.end_row();
            ui.label(dim(tr("drug_pregnancy")));
            field_box(ui, "fld_pregnancy", w, 76.0, &mut form.pregnancy);
            ui.end_row();
            ui.label(dim(tr("tables_sources")))
                .on_hover_text(tr("drug_sources_hint"));
            field_box(ui, "fld_sources", w, 76.0, &mut form.sources);
            ui.end_row();
            ui.label(dim(tr("drug_forms")));
            field_box(ui, "fld_forms", w, 64.0, &mut form.forms);
            ui.end_row();
            ui.label(dim(tr("drug_status")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.status));
            ui.end_row();
            ui.label(dim(tr("drug_tags")))
                .on_hover_text(tr("drug_tags_hint"));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.tags));
            ui.end_row();
            ui.label(dim(tr("drug_sec_smr")));
            ui.add_sized([w, 26.0], egui::TextEdit::singleline(&mut form.smr));
            ui.end_row();
            ui.label(dim(tr("drug_sec_toxicity")));
            field_box(ui, "fld_toxicity", w, 76.0, &mut form.toxicity);
            ui.end_row();
        });
}

/// Shorten `text` until it fits `width` at `size`, ending in an
/// ellipsis. A hard clip cut names mid-letter and gave no sign that
/// anything was missing.
fn elide(ui: &egui::Ui, text: &str, width: f32, size: f32) -> String {
    let font = egui::FontId::proportional(size);
    let measure = |t: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(t.to_owned(), font.clone(), motif::TEXT)
                .size()
                .x
        })
    };
    if measure(text) <= width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut keep = chars.len();
    while keep > 1 {
        keep -= 1;
        let candidate: String = chars[..keep]
            .iter()
            .collect::<String>()
            .trim_end()
            .to_owned()
            + "…";
        if measure(&candidate) <= width {
            return candidate;
        }
    }
    "…".to_owned()
}

/// The uppercase heading of a monograph section, with its hairline.
fn mono_heading(ui: &mut egui::Ui, width: f32, title: &str) {
    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .size(11.0)
            .strong()
            .color(motif::INK_LIGHT),
    );
    let rule = ui.cursor().top() + 2.0;
    ui.painter().hline(
        ui.cursor().left()..=(ui.cursor().left() + width),
        rule,
        egui::Stroke::new(0.8_f32, motif::INK_LIGHT),
    );
    ui.add_space(5.0);
}

/// One section of the printed-looking monograph: the heading, then the
/// text wrapped to the sheet's width. Empty sections are skipped.
fn mono_section(ui: &mut egui::Ui, width: f32, title: &str, body: &str) {
    let body = body.trim();
    if body.is_empty() {
        return;
    }
    mono_heading(ui, width, title);
    // Blank lines in the stored text separate paragraphs.
    for para in body.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        ui.scope(|ui| {
            ui.set_max_width(width);
            let mut job = rich_text(para, 13.0, motif::INK);
            job.wrap.max_width = width;
            ui.add(egui::Label::new(job).wrap());
        });
        ui.add_space(3.0);
    }
}

/// The drug card as a printed monograph on a sheet of paper: identity,
/// then every filled section in reading order, the pharmacokinetics as
/// a short definition list, and the numbered sources at the foot.
fn drug_monograph(ui: &mut egui::Ui, d: &Drug, class_note: &str, posologies: &[db::Posologie]) {
    // Measured against the visible slice: a sheet centred on a width
    // the panel claimed but does not have loses its right margin — and
    // with it the right-hand column of the posology table.
    let avail = motif::visible_rect(ui);
    let sheet_w = avail.width().min(860.0);
    let pad = 34.0;
    let width = sheet_w - 2.0 * pad;
    let bg = ui.painter().add(egui::Shape::Noop);
    let content = egui::Rect::from_min_size(
        egui::pos2(avail.center().x - sheet_w / 2.0 + pad, avail.top() + pad),
        egui::vec2(width, avail.height().max(1.0)),
    );
    let used = ui
        .allocate_new_ui(egui::UiBuilder::new().max_rect(content), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(d.name.trim().to_uppercase())
                        .size(19.0)
                        .strong()
                        .color(motif::INK),
                );
                let mut sub = d.dci.trim().to_owned();
                if !d.class.trim().is_empty() {
                    if !sub.is_empty() {
                        sub.push_str(" — ");
                    }
                    sub.push_str(d.class.trim());
                }
                if !sub.is_empty() {
                    ui.label(
                        egui::RichText::new(sub)
                            .size(13.0)
                            .italics()
                            .color(motif::INK_LIGHT),
                    );
                }
                if !d.antidote.trim().is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(trf("drug_antidote_banner", d.antidote.trim()))
                            .size(12.0)
                            .strong()
                            .color(motif::ALERT),
                    );
                }
                if !d.status.trim().is_empty() {
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(format!("  {}  ", d.status.trim()))
                            .size(11.0)
                            .strong()
                            .color(egui::Color32::WHITE)
                            .background_color(status_color(&d.status)),
                    );
                }
                let tags: Vec<&str> = d
                    .tags
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .collect();
                if !tags.is_empty() {
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(tags.join("  ·  "))
                            .size(10.0)
                            .color(motif::INK_LIGHT),
                    );
                }
            });
            ui.add_space(6.0);
            let top = ui.cursor().top();
            ui.painter().hline(
                content.left()..=content.right(),
                top,
                egui::Stroke::new(1.2_f32, motif::INK),
            );
            for (title, body) in [
                (tr("drug_sec_indications"), d.indications.as_str()),
                (tr("drug_sec_mechanism"), d.mechanism.as_str()),
                (tr("drug_dosage"), d.dosage.as_str()),
                (tr("drug_sec_ci"), d.contraindications.as_str()),
                (tr("drug_ddi"), d.ddi.as_str()),
                (tr("drug_sec_adverse"), d.adverse.as_str()),
                (tr("drug_sec_toxicity"), d.toxicity.as_str()),
                (tr("drug_sec_monitoring"), d.monitoring.as_str()),
                (tr("drug_iup"), d.iup.as_str()),
                (tr("drug_sec_smr"), d.smr.as_str()),
            ] {
                mono_section(ui, width, title, body);
            }
            // Posologies by indication: the mainstream ones and the
            // less obvious, each with what changes it.
            if !posologies.is_empty() {
                mono_heading(ui, width, tr("drug_sec_poso"));
                egui::Grid::new(("mono_poso", d.id))
                    .num_columns(2)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        for p in posologies {
                            ui.scope(|ui| {
                                ui.set_max_width(width * 0.38);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&p.indication)
                                            .size(12.5)
                                            .strong()
                                            .color(motif::INK),
                                    )
                                    .wrap(),
                                );
                            });
                            ui.vertical(|ui| {
                                ui.set_max_width(width * 0.58);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&p.posologie)
                                            .size(12.5)
                                            .color(motif::INK),
                                    )
                                    .wrap(),
                                );
                                if !p.remarque.trim().is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&p.remarque)
                                                .size(11.0)
                                                .italics()
                                                .color(motif::INK_LIGHT),
                                        )
                                        .wrap(),
                                    );
                                }
                            });
                            ui.end_row();
                        }
                    });
            }
            // Pharmacokinetics as a compact definition list.
            let pk = [
                (tr("drug_forms"), d.forms.as_str()),
                (tr("drug_half_life"), d.half_life.as_str()),
                (tr("drug_auc"), d.auc.as_str()),
                (tr("drug_elimination"), d.elimination.as_str()),
                (tr("drug_renal"), d.renal.as_str()),
                (tr("drug_pregnancy"), d.pregnancy.as_str()),
            ];
            if pk.iter().any(|(_, v)| !v.trim().is_empty()) {
                mono_heading(ui, width, tr("drug_sec_pk"));
                // The half-life as a shape, not only as a sentence:
                // how much is left a day after the last dose is the
                // question behind "puis-je opérer / relayer / arrêter",
                // and reading it off "≈ 12 heures" is arithmetic the
                // counter should not have to do.
                if let Some(hl) = parse_hours(&d.half_life) {
                    if hl > 0.0 {
                        let span = (hl * 5.0).clamp(6.0, 240.0);
                        let steps = 60;
                        let curve: Vec<f64> = (0..=steps)
                            .map(|i| {
                                let t = span * i as f64 / steps as f64;
                                100.0 * 0.5_f64.powf(t / hl)
                            })
                            .collect();
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(width.min(320.0), 44.0),
                            egui::Sense::hover(),
                        );
                        motif::chart::sparkline(ui, rect.shrink(2.0), &curve, motif::ACCENT);
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(0.8_f32, motif::INK_LIGHT),
                        );
                        let caption = trn(
                            "drug_decay_caption",
                            &[
                                &format!("{:.0}", span),
                                &format!("{:.0}", 100.0 * 0.5_f64.powf(24.0 / hl)),
                            ],
                        );
                        ui.label(
                            egui::RichText::new(caption)
                                .size(10.0)
                                .color(motif::INK_LIGHT),
                        );
                        resp.on_hover_text(trf("drug_decay_tooltip", format!("{hl:.1}")));
                        ui.add_space(4.0);
                    }
                }
                egui::Grid::new(("mono_pk", d.id))
                    .num_columns(2)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        for (label, value) in pk {
                            if value.trim().is_empty() {
                                continue;
                            }
                            ui.scope(|ui| {
                                ui.set_max_width(130.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(label)
                                            .size(12.0)
                                            .color(motif::INK_LIGHT),
                                    )
                                    .wrap(),
                                );
                            });
                            ui.scope(|ui| {
                                ui.set_max_width(width - 150.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(value.trim())
                                            .size(13.0)
                                            .color(motif::INK),
                                    )
                                    .wrap(),
                                );
                            });
                            ui.end_row();
                        }
                    });
            }
            mono_section(ui, width, tr("drug_notes"), &d.notes);
            mono_section(
                ui,
                width,
                &trf("drug_class_note", d.class.trim()),
                class_note,
            );
            let sources: Vec<&str> = d
                .sources
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            if !sources.is_empty() {
                ui.add_space(12.0);
                let y = ui.cursor().top();
                ui.painter().hline(
                    content.left()..=content.right(),
                    y,
                    egui::Stroke::new(0.8_f32, motif::INK_LIGHT),
                );
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(tr("tables_sources"))
                        .size(11.0)
                        .strong()
                        .color(motif::INK_LIGHT),
                );
                for (i, src) in sources.iter().enumerate() {
                    ui.scope(|ui| {
                        ui.set_max_width(width);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{}. {}", i + 1, src))
                                    .size(11.0)
                                    .color(motif::INK_LIGHT),
                            )
                            .wrap(),
                        );
                    });
                }
            }
            ui.add_space(4.0);
        })
        .response
        .rect;
    let sheet_rect = egui::Rect::from_min_size(
        egui::pos2(avail.center().x - sheet_w / 2.0, avail.top()),
        egui::vec2(sheet_w, used.height() + 2.0 * pad),
    );
    // The sheet is painted behind the text, once its height is known.
    ui.painter().set(
        bg,
        egui::Shape::Vec(vec![
            egui::Shape::rect_filled(
                sheet_rect.translate(egui::vec2(4.0, 4.0)),
                0.0,
                motif::BG_DARK,
            ),
            egui::Shape::rect_filled(sheet_rect, 0.0, motif::PAPER),
            egui::Shape::rect_stroke(
                sheet_rect,
                0.0,
                egui::Stroke::new(1.0_f32, motif::INK_LIGHT),
            ),
        ]),
    );
    let below = (sheet_rect.bottom() - ui.cursor().top()).max(0.0) + 12.0;
    ui.add_space(below);
}

/// French name of an act's rank inside its année d'accompagnement.
fn rank_label(rank: usize) -> String {
    match rank {
        0 => tr("rank_initial").to_owned(),
        1 => tr("rank_suivi_1").to_owned(),
        n => trf("rank_suivi_n", n),
    }
}

/// A compact dated-notes journal: sunken scroll list ("24/08 14:32 ·
/// CL" then the body, with a two-step "×"), and an add row below.
/// Returns (body to add, note id to delete).
/// The height [`notes_box`] needs under its well for the "add" row.
/// Derived from the style so a bigger text scale does not push the
/// button through the bottom of its panel.
fn notes_box_reserve(ui: &egui::Ui) -> f32 {
    // The field, the button's own padding, the gap the box leaves under
    // its well, and the row spacing either side of it.
    let button =
        ui.text_style_height(&egui::TextStyle::Button) + ui.spacing().button_padding.y * 2.0 + 2.0;
    ui.spacing().interact_size.y.max(button) + ui.spacing().item_spacing.y + 10.0
}

fn notes_box(
    ui: &mut egui::Ui,
    id_salt: &str,
    notes: &[Note],
    text: &mut String,
    confirm: &mut Option<i64>,
    height: f32,
    with_add: bool,
) -> (Option<String>, Option<i64>) {
    let mut add: Option<String> = None;
    let mut delete: Option<i64> = None;
    let w = ui.available_width();
    let top = ui.cursor().top();
    // `height` is what the well would like; what it gets is what is
    // left after the "add" row drawn under it. Callers used to subtract
    // that themselves and got it wrong in both directions — grey under
    // a short box, a clipped button under a tall one.
    let budget = ui.max_rect().bottom() - top;
    let reserve = if with_add { notes_box_reserve(ui) } else { 0.0 };
    let height = height.min(budget - reserve).max(28.0);
    let rect =
        egui::Rect::from_min_size(egui::pos2(ui.cursor().left(), top), egui::vec2(w, height));
    ui.painter().rect_filled(rect, 0.0, motif::TROUGH);
    motif::bevel(ui.painter(), rect, false);
    // Clipped to the well it sits in. `allocate_new_ui` only sets a max
    // rect, and egui paints past that; an over-full journal therefore
    // spilled its last entries under the frame and pushed the "add" row
    // through the bottom edge of the panel.
    motif::inside(ui, rect.shrink(5.0), |ui| {
        egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .max_height(rect.height() - 10.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                if notes.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("notes_empty"))
                            .size(11.0)
                            .color(motif::TEXT_DIM),
                    );
                }
                for n in notes {
                    ui.horizontal(|ui| {
                        let head = if n.operator.is_empty() {
                            n.stamp()
                        } else {
                            format!("{} · {}", n.stamp(), n.operator)
                        };
                        // Stamped in the operator's own colour, so a
                        // journal can be scanned by who wrote what.
                        ui.label(
                            egui::RichText::new(head)
                                .size(11.0)
                                .color(operator_color(&n.operator)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if *confirm == Some(n.id) {
                                tr("itv_delete_confirm")
                            } else {
                                tr("itv_delete")
                            };
                            let x = ui.add(
                                egui::Label::new(egui::RichText::new(label).size(11.0))
                                    .sense(egui::Sense::click()),
                            );
                            if x.on_hover_text(tr("notes_delete_tooltip")).clicked() {
                                if *confirm == Some(n.id) {
                                    delete = Some(n.id);
                                    *confirm = None;
                                } else {
                                    *confirm = Some(n.id);
                                }
                            }
                        });
                    });
                    ui.add(egui::Label::new(rich_text(&n.body, 13.0, motif::TEXT)).wrap());
                    ui.add_space(3.0);
                }
            });
    });
    ui.add_space(6.0);
    if with_add {
        ui.horizontal(|ui| {
            let field_w = (ui.available_width() - 100.0).max(120.0);
            ui.add_sized(
                [field_w, 24.0],
                egui::TextEdit::singleline(text).hint_text(tr("notes_add_hint")),
            )
            .on_hover_text(tr("notes_markup_hint"));
            if motif::button(ui, tr("notes_add")).clicked() && !text.trim().is_empty() {
                add = Some(text.trim().to_owned());
            }
        });
    }
    (add, delete)
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
    /// The end-of-day transmission logbook (F5).
    Transmissions,
    /// The vaccination world map and its country groups (F7).
    VaccineMap,
}

/// One item open in the workspace notebook.
///
/// A tab is a *bookmark of a destination*, not a copy of the view's
/// state: the session keeps one live view, and activating a tab points
/// it back at that destination. Two patients, or a patient and a drug
/// card, therefore stay one click apart all day — the counter's real
/// working pattern, and the thing a single-view app made impossible.
#[derive(Clone, PartialEq, Eq, Debug)]
enum WorkTab {
    Dashboard,
    Search,
    Agenda,
    Carnet,
    /// The vaccination map.
    Map,
    /// The drug base's list (no card open).
    Drugs,
    Patient(i64),
    Drug(i64),
}

impl WorkTab {
    /// Does this tab survive a "close"? The four standing views are
    /// fixtures of the workspace; only opened files can be dismissed.
    fn closable(&self) -> bool {
        matches!(self, WorkTab::Patient(_) | WorkTab::Drug(_))
    }
}

/// Which half of an open patient file is on screen.
///
/// The acts and the carnet both want the whole work area — ten columns
/// of buttons on one side, a dose table on the other — so they take
/// turns behind a notebook strip rather than share a split.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum PatientTab {
    #[default]
    Acts,
    Vaccins,
}

/// The ordonnance being composed after a positive TROD.
///
/// It is keyed by the interview so that closing the patient file — or
/// another post correcting the act — cannot leave a prescription open
/// against the wrong record.
struct OrdonnanceBox {
    interview: i64,
    kind: InterviewKind,
    choice: crate::ordonnance::Choice,
}

/// What the world map colours its tiles by.
///
/// One map, several readings: the group a country belongs to answers
/// "where am I", and each risk answers one counter question outright.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum MapLens {
    #[default]
    Group,
    YellowFever,
    Malaria,
    Meningo,
    HepatitisA,
    Rabies,
    JapaneseEnceph,
}

impl MapLens {
    const ALL: [MapLens; 7] = [
        Self::Group,
        Self::YellowFever,
        Self::Malaria,
        Self::Meningo,
        Self::HepatitisA,
        Self::Rabies,
        Self::JapaneseEnceph,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Group => tr("map_lens_group"),
            Self::YellowFever => tr("map_lens_yf"),
            Self::Malaria => tr("map_lens_palu"),
            Self::Meningo => tr("map_lens_meningo"),
            Self::HepatitisA => tr("map_lens_hepa"),
            Self::Rabies => tr("map_lens_rage"),
            Self::JapaneseEnceph => tr("map_lens_ej"),
        }
    }
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
    /// Quick act picker (Ctrl+N): open flag and the theme it will stamp
    /// on the created interview.
    act_picker: bool,
    act_theme: String,
    /// A new act refused by the yearly-quota rule: (kind, message),
    /// with an explicit override offered.
    /// (kind, thematic, French notice) of an act the yearly quota
    /// blocked, kept so the explicit override creates the same act.
    rule_block: Option<(InterviewKind, String, String)>,
    /// The viewed patient's current treatments (from the drug base).
    patient_treats: Vec<Drug>,
    /// The viewed patient's dated notes, newest first.
    patient_notes: Vec<Note>,
    /// Which half of the patient file is on screen: the acts, or the
    /// carnet de vaccination.
    patient_tab: PatientTab,
    /// The viewed patient's carnet, newest dose first, and the
    /// destinations recorded on the file.
    vaccinations: Vec<db::Vaccination>,
    travels: Vec<db::Travel>,
    /// The line being written at the foot of the carnet: the dose, plus
    /// the date typed in French (parsed on save, like every other date
    /// field) and the catalogue entry it was picked from.
    vacc_new: db::Vaccination,
    vacc_new_date: String,
    vacc_new_pick: usize,
    /// The line being corrected, with the label and date the row was
    /// displayed with — the compare-and-set baseline.
    vacc_edit: Option<db::Vaccination>,
    vacc_edit_date: String,
    vacc_edit_base: (String, String),
    /// Two-step delete confirmation for one carnet line.
    vacc_confirm: Option<i64>,
    /// In-progress country search of the travel panel.
    travel_query: String,
    /// The open ordonnance box, after a positive TROD.
    ordonnance: Option<OrdonnanceBox>,
    /// The open drug card's dated notes, newest first.
    drug_notes: Vec<Note>,
    /// Transmission logbook: the shown day, its entries, and the days
    /// that have entries (for navigation).
    trans_day: String,
    trans_notes: Vec<Note>,
    trans_days: Vec<String>,
    /// Input buffer of the visible notes box (views are exclusive).
    note_text: String,
    /// Two-step delete confirmation for one note.
    note_confirm: Option<i64>,
    /// In-progress text of the treatment picker.
    treat_query: String,
    view: MainView,
    summaries: Vec<InterviewSummary>,
    /// Planned interviews for the dashboard's "RDV à venir" list.
    appointments: Vec<Appointment>,
    /// Dashboard: patients whose file moved most recently, and the
    /// notes written today (day notes and transmissions).
    recent: Vec<(Patient, String)>,
    today_notes: Vec<Note>,
    /// Path of the last CSV export, shown under the export button.
    export_notice: Option<String>,
    /// Today as ISO `YYYY-MM-DD`, to flag overdue appointments.
    today: String,
    /// Tomorrow as ISO `YYYY-MM-DD`, for agenda day labels.
    tomorrow: String,
    /// The 7 dates (Mon..Sun) of the agenda's displayed week.
    agenda_week: Vec<String>,
    /// Which agenda layout is shown.
    agenda_mode: AgendaMode,
    /// Month grid: on/off, the offset in months and its 42-day grid.
    agenda_month: bool,
    agenda_month_offset: i64,
    agenda_month_days: Vec<String>,
    /// The day the agenda panel details (ISO), its events and notes.
    agenda_day: String,
    events: Vec<db::Event>,
    day_notes: Vec<Note>,
    day_note_text: String,
    day_note_confirm: Option<i64>,
    event_title: String,
    event_time: String,
    event_category: db::EventCategory,
    /// How often a new entry repeats, in days (0 = once).
    event_repeat: i64,
    /// Agenda filter: the act kinds shown (all when empty), and the
    /// rendez-vous whose hour or date is being changed.
    agenda_filter: std::collections::HashSet<InterviewKind>,
    rdv_time_edit: Option<(i64, String)>,
    rdv_move_edit: Option<(i64, String)>,
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
    /// A card opens as a monograph to read; "Modifier" switches to the
    /// editable form.
    drug_reading: bool,
    /// The note shared by every card of the open card's class, and its
    /// editing buffer.
    class_note: String,
    class_note_edit: Option<String>,
    /// The open card's posologies, and the row being written.
    posologies: Vec<db::Posologie>,
    poso_new: (String, String, String),
    poso_edit: Option<db::Posologie>,
    drug_base: Option<Drug>,
    confirm_delete_drug: bool,
    /// Patients currently on the drug whose card is open.
    drug_patients: Vec<Patient>,
    /// Conversion tables browser (inside the drug view).
    show_tables: bool,
    /// The convention's cycle length in months, from the options: it
    /// drives both the quota rule and the fee ranks.
    cycle_months: u32,
    /// Substitution protocols: the list, the open one with its steps,
    /// what is being written, and the walk-through position.
    show_protocols: bool,
    protocols: Vec<db::Protocol>,
    protocol_open: Option<db::Protocol>,
    protocol_nodes: Vec<db::ProtocolNode>,
    protocol_new_title: String,
    /// Editing buffer for the open protocol's title and subject: the
    /// fields were re-cloned each frame, so typing went nowhere.
    protocol_header: Option<(String, String)>,
    protocol_node_edit: Option<(i64, db::NodeKind, String)>,
    protocol_walk: Option<i64>,
    /// Team edits of the shown table, keyed by (row, col), plus the
    /// cell being edited and the last change (for the undo).
    table_cells: std::collections::HashMap<(usize, usize), String>,
    table_edit: Option<(usize, usize, String)>,
    table_undo: Option<(usize, usize, String)>,
    /// The calculation panel: which tool, and its inputs.
    calc_open: bool,
    calc_weight: f64,
    calc_age: f64,
    calc_creat: f64,
    calc_female: bool,
    calc_per_kg: f64,
    calc_takes: u32,
    calc_half_life: f64,
    calc_interval: f64,
    table_selected: usize,
    /// The vaccination map: what colours the tiles, the country the
    /// detail panel is showing, and the search box above it.
    map_lens: MapLens,
    map_country: Option<&'static str>,
    map_query: String,
    /// The workspace notebook: what the operator has opened, in the
    /// order they opened it. The *active* tab is never stored — it is
    /// derived from the live view each frame (see [`Session::current_tab`]),
    /// so navigating by any other route (a dashboard row, a search
    /// result, Escape) can never leave the strip pointing elsewhere.
    tabs: Vec<WorkTab>,
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
    physician: String,
    email: String,
    address: String,
    /// ALD, AT/MP, maternité — what the billing must take into account.
    situation: String,
    /// Numéro d'immatriculation and régime, for the bulletin d'adhésion.
    nir: String,
    regime: String,
    error: Option<String>,
}

impl Session {
    fn new(db: Db, cycle_months: u32) -> Result<Self, String> {
        let patients = db.patients()?;
        let pending = db.pending_counts().unwrap_or_default();
        // First unlock of a fresh base: starter drug cards (names, DCI,
        // textbook antidotes). Non-fatal if it fails.
        let _ = db.seed_drugs_if_empty();
        let drugs = db.drugs().unwrap_or_default();
        let protocols = db.protocols().unwrap_or_default();
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
            act_picker: false,
            act_theme: String::new(),
            rule_block: None,
            patient_treats: Vec::new(),
            patient_notes: Vec::new(),
            patient_tab: PatientTab::default(),
            vaccinations: Vec::new(),
            travels: Vec::new(),
            vacc_new: db::Vaccination::default(),
            vacc_new_date: String::new(),
            vacc_new_pick: 0,
            vacc_edit: None,
            vacc_edit_date: String::new(),
            vacc_edit_base: (String::new(), String::new()),
            vacc_confirm: None,
            travel_query: String::new(),
            ordonnance: None,
            drug_notes: Vec::new(),
            trans_day: String::new(),
            trans_notes: Vec::new(),
            trans_days: Vec::new(),
            note_text: String::new(),
            note_confirm: None,
            treat_query: String::new(),
            view: MainView::Search,
            summaries: Vec::new(),
            appointments: Vec::new(),
            recent: Vec::new(),
            today_notes: Vec::new(),
            export_notice: None,
            today: String::new(),
            tomorrow: String::new(),
            agenda_week: Vec::new(),
            agenda_mode: AgendaMode::Week,
            agenda_month: false,
            agenda_month_offset: 0,
            agenda_month_days: Vec::new(),
            agenda_day: String::new(),
            events: Vec::new(),
            day_notes: Vec::new(),
            day_note_text: String::new(),
            day_note_confirm: None,
            event_title: String::new(),
            event_time: String::new(),
            event_category: db::EventCategory::Formation,
            event_repeat: 0,
            agenda_filter: std::collections::HashSet::new(),
            rdv_time_edit: None,
            rdv_move_edit: None,
            agenda_offset: 0,
            date_edits: std::collections::HashMap::new(),
            show_amounts: false,
            dup_check: None,
            drugs,
            drug_query: String::new(),
            drug_selected: 0,
            drug_form: None,
            drug_reading: true,
            class_note: String::new(),
            class_note_edit: None,
            posologies: Vec::new(),
            poso_new: (String::new(), String::new(), String::new()),
            poso_edit: None,
            drug_base: None,
            confirm_delete_drug: false,
            drug_patients: Vec::new(),
            show_tables: false,
            cycle_months: cycle_months.max(1),
            show_protocols: false,
            protocols,
            protocol_open: None,
            protocol_nodes: Vec::new(),
            protocol_new_title: String::new(),
            protocol_header: None,
            protocol_node_edit: None,
            protocol_walk: None,
            table_cells: std::collections::HashMap::new(),
            table_edit: None,
            table_undo: None,
            calc_open: false,
            calc_weight: 70.0,
            calc_age: 75.0,
            calc_creat: 90.0,
            calc_female: false,
            calc_per_kg: 15.0,
            calc_takes: 3,
            calc_half_life: 12.0,
            calc_interval: 12.0,
            table_selected: 0,
            map_lens: MapLens::default(),
            map_country: None,
            map_query: String::new(),
            // The five standing views are always in the strip, in a
            // fixed order, so their position never moves under the
            // pointer; opened files are appended after them.
            tabs: vec![
                WorkTab::Dashboard,
                WorkTab::Search,
                WorkTab::Drugs,
                WorkTab::Agenda,
                WorkTab::Carnet,
                WorkTab::Map,
            ],
            error: None,
        };
        session.set_patients(patients);
        // The search view opens on the day's own panels, so the figures
        // they show have to be loaded before the first frame.
        session.refresh_dashboard();
        Ok(session)
    }

    /// Which tab the live view corresponds to right now.
    fn current_tab(&self) -> WorkTab {
        match self.view {
            MainView::Dashboard => WorkTab::Dashboard,
            MainView::Agenda => WorkTab::Agenda,
            MainView::Transmissions => WorkTab::Carnet,
            MainView::VaccineMap => WorkTab::Map,
            MainView::Drugs => match &self.drug_form {
                Some(d) => WorkTab::Drug(d.id),
                None => WorkTab::Drugs,
            },
            MainView::Search => match &self.viewing {
                Some(p) => WorkTab::Patient(p.id),
                None => WorkTab::Search,
            },
        }
    }

    /// Record the live view in the notebook, appending a tab for a file
    /// opened for the first time. Called once per frame: whatever route
    /// the operator took to get here, the strip ends up showing it.
    fn note_tab(&mut self) {
        let cur = self.current_tab();
        if !self.tabs.contains(&cur) {
            self.tabs.push(cur);
        }
    }

    /// Point the live view at `tab`. Reloads whatever that destination
    /// needs, exactly as opening it the long way would.
    fn activate_tab(&mut self, tab: &WorkTab) {
        self.flush_date_edits();
        self.show_amounts = false;
        match tab {
            WorkTab::Dashboard => {
                self.view = MainView::Dashboard;
                self.refresh_dashboard();
            }
            WorkTab::Search => {
                self.view = MainView::Search;
                self.viewing = None;
                // The home panels show the day: they need the same
                // figures the dashboard does.
                self.refresh_dashboard();
            }
            WorkTab::Agenda => {
                self.view = MainView::Agenda;
                self.refresh_dashboard();
            }
            WorkTab::Carnet => {
                self.view = MainView::Transmissions;
                self.trans_day = String::new();
                self.load_transmissions();
            }
            WorkTab::Map => {
                self.view = MainView::VaccineMap;
            }
            WorkTab::Drugs => {
                self.view = MainView::Drugs;
                self.drug_form = None;
                self.drug_base = None;
                if let Ok(list) = self.db.drugs() {
                    self.drugs = list;
                }
            }
            WorkTab::Patient(id) => {
                // The list is the authority on identity: another post
                // may have corrected the name since the tab was opened.
                if let Some(p) = self.patients.iter().find(|p| p.id == *id).cloned() {
                    self.view = MainView::Search;
                    self.open_patient(p);
                } else {
                    // Deleted elsewhere: drop the stale tab rather than
                    // showing an empty file.
                    self.close_tab(&WorkTab::Patient(*id));
                }
            }
            WorkTab::Drug(id) => {
                if let Some(d) = self.drugs.iter().find(|d| d.id == *id).cloned() {
                    self.view = MainView::Drugs;
                    self.open_drug_card(d);
                } else {
                    self.close_tab(&WorkTab::Drug(*id));
                }
            }
        }
    }

    /// Close `tab`, falling back to the neighbour on its left when it
    /// was the one on screen.
    fn close_tab(&mut self, tab: &WorkTab) {
        let Some(i) = self.tabs.iter().position(|t| t == tab) else {
            return;
        };
        if !tab.closable() {
            return;
        }
        let was_active = &self.current_tab() == tab;
        self.tabs.remove(i);
        if was_active {
            let fallback = self
                .tabs
                .get(i.saturating_sub(1))
                .cloned()
                .unwrap_or(WorkTab::Search);
            self.activate_tab(&fallback);
        }
    }

    /// Move `delta` tabs along the strip (Ctrl+Tab and its shifted twin).
    fn cycle_tab(&mut self, delta: i64) {
        if self.tabs.is_empty() {
            return;
        }
        let cur = self.current_tab();
        let at = self.tabs.iter().position(|t| *t == cur).unwrap_or(0) as i64;
        let n = self.tabs.len() as i64;
        let next = self.tabs[(at + delta).rem_euclid(n) as usize].clone();
        self.activate_tab(&next);
    }

    /// The drug base filtered by `drug_query`, best match first.
    ///
    /// Brand name and DCI both match ("elix" or "apixa"); the class and
    /// the tags widen the net ("statine", "marge étroite"), scored below
    /// an identity match.
    fn drug_results(&self, limit: usize) -> Vec<Drug> {
        let mut scored: Vec<(i32, &Drug)> = self
            .drugs
            .iter()
            .filter_map(|d| {
                let a = fuzzy::score(&self.drug_query, &d.name);
                let b = if d.dci.is_empty() {
                    None
                } else {
                    fuzzy::score(&self.drug_query, &d.dci)
                };
                let side = [d.class.as_str(), d.tags.as_str()]
                    .into_iter()
                    .filter(|t| !t.is_empty())
                    .filter_map(|t| fuzzy::score(&self.drug_query, t))
                    .max()
                    .map(|s| s - 40);
                a.max(b).max(side).map(|s| (s, d))
            })
            .collect();
        scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, d)| d.clone())
            .collect()
    }

    /// The label a tab shows: the view's name, or the file's.
    fn tab_label(&self, tab: &WorkTab) -> String {
        match tab {
            WorkTab::Dashboard => tr("tab_dashboard").to_owned(),
            WorkTab::Search => tr("tab_search").to_owned(),
            WorkTab::Agenda => tr("tab_agenda").to_owned(),
            WorkTab::Carnet => tr("tab_carnet").to_owned(),
            WorkTab::Map => tr("tab_map").to_owned(),
            WorkTab::Drugs => tr("tab_drugs").to_owned(),
            WorkTab::Patient(id) => self
                .patients
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.full_name())
                .unwrap_or_else(|| tr("tab_missing").to_owned()),
            WorkTab::Drug(id) => self
                .drugs
                .iter()
                .find(|d| d.id == *id)
                .map(|d| d.name.trim().to_owned())
                .unwrap_or_else(|| tr("tab_missing").to_owned()),
        }
    }

    /// Open a drug card: load its baseline for CAS and the patients
    /// currently on it (recall / alert lookup).
    fn open_drug_card(&mut self, d: Drug) {
        self.drug_patients = self.db.patients_for_drug(d.id).unwrap_or_default();
        self.drug_notes = self
            .db
            .notes_for(NoteSubject::Drug, d.id)
            .unwrap_or_default();
        self.note_text.clear();
        self.note_confirm = None;
        self.class_note = self.db.class_note(&d.class).unwrap_or_default();
        self.posologies = self.db.posologies(d.id).unwrap_or_default();
        self.poso_new = (String::new(), String::new(), String::new());
        self.poso_edit = None;
        self.drug_base = Some(d.clone());
        self.drug_form = Some(d);
        self.drug_reading = true;
        self.class_note_edit = None;
        self.confirm_delete_drug = false;
    }

    /// Load (or reload) the transmission logbook for `trans_day`.
    /// Reload what the agenda's day panel shows: the day's entries and
    /// its notes. Called on selection, on refresh and after a write.
    fn load_day(&mut self) {
        if self.agenda_day.is_empty() {
            self.agenda_day = self.today.clone();
        }
        let day = self.agenda_day.clone();
        self.events = self.db.events_between(&day, &day).unwrap_or_default();
        self.day_notes = self
            .db
            .notes_for(NoteSubject::Day, db::day_subject_id(&day))
            .unwrap_or_default();
        self.day_note_confirm = None;
    }

    /// Reload the events shown on the week or month grid.
    fn load_grid_events(&mut self) -> Vec<db::Event> {
        let range = if self.agenda_month {
            (
                self.agenda_month_days.first().cloned(),
                self.agenda_month_days.last().cloned(),
            )
        } else {
            (
                self.agenda_week.first().cloned(),
                self.agenda_week.last().cloned(),
            )
        };
        match range {
            (Some(from), Some(to)) => self.db.events_between(&from, &to).unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn load_transmissions(&mut self) {
        if self.trans_day.is_empty() {
            self.trans_day = self.db.today_iso().unwrap_or_default();
        }
        self.trans_notes = self
            .db
            .transmissions_for_day(&self.trans_day)
            .unwrap_or_default();
        self.trans_days = self.db.transmission_days().unwrap_or_default();
        self.today = self.db.today_iso().unwrap_or_default();
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
        self.patient_treats = self.db.drugs_for_patient(patient.id).unwrap_or_default();
        self.patient_notes = self
            .db
            .notes_for(NoteSubject::Patient, patient.id)
            .unwrap_or_default();
        self.note_text.clear();
        self.note_confirm = None;
        self.treat_query.clear();
        self.edit_patient = None;
        self.confirm_delete = false;
        self.confirm_delete_itv = None;
        self.rule_block = None;
        self.load_carnet(patient.id);
        self.viewing = Some(patient);
    }

    /// (Re)read the patient's carnet and destinations. Called on open
    /// and after every write, so a compare-and-set refusal can put the
    /// colleague's version back on screen.
    fn load_carnet(&mut self, patient_id: i64) {
        self.vaccinations = self.db.vaccinations(patient_id).unwrap_or_default();
        self.travels = self.db.travels(patient_id).unwrap_or_default();
        self.vacc_edit = None;
        self.vacc_edit_date.clear();
        self.vacc_confirm = None;
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
        let months = self.cycle_months;
        match self.db.interview_summaries(months) {
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
        self.recent = self.db.recent_patients(6).unwrap_or_default();
        self.protocols = self.db.protocols().unwrap_or_default();
        // What the team wrote today: the day's notes, then the day's
        // transmissions — the two journals a morning starts with.
        let mut notes = self
            .db
            .notes_for(NoteSubject::Day, db::day_subject_id(&self.today))
            .unwrap_or_default();
        notes.extend(
            self.db
                .transmissions_for_day(&self.today)
                .unwrap_or_default(),
        );
        self.today_notes = notes;
        self.export_notice = None;
    }
}

/// The team's free text accepts a light markup, the one people already
/// type: `*gras*`, `_italique_` and `=surligné=`. It is rendered where
/// the text is read — monograph sections, note journals — while the
/// editors stay plain text.
fn rich_text(text: &str, size: f32, color: egui::Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let mut buf = String::new();
    let (mut bold, mut italic, mut mark) = (false, false, false);
    let push = |job: &mut egui::text::LayoutJob,
                buf: &mut String,
                bold: bool,
                italic: bool,
                mark: bool| {
        if buf.is_empty() {
            return;
        }
        let mut format = egui::TextFormat {
            font_id: egui::FontId::proportional(size),
            color,
            italics: italic,
            ..Default::default()
        };
        if bold {
            // The bundled family has no bold face: a darker ink and the
            // background are what carry the emphasis.
            format.color = egui::Color32::BLACK;
        }
        if mark {
            format.background = motif::BG_LIGHT;
        }
        job.append(buf, 0.0, format);
        buf.clear();
    };
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '=' => {
                // A marker glued to a word toggles; a lone one is text.
                let toggles = !buf.ends_with(' ') || chars.peek().is_some_and(|n| *n != ' ');
                if !toggles {
                    buf.push(c);
                    continue;
                }
                push(&mut job, &mut buf, bold, italic, mark);
                match c {
                    '*' => bold = !bold,
                    '_' => italic = !italic,
                    _ => mark = !mark,
                }
            }
            _ => buf.push(c),
        }
    }
    push(&mut job, &mut buf, bold, italic, mark);
    job
}

/// Percent-encode a query for a URL handed to the browser.
fn urlencode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Read a number of hours out of a free-text half-life ("≈ 12 heures",
/// "5 à 13 h"): the first number, or the middle of a range.
fn parse_hours(text: &str) -> Option<f64> {
    let cleaned = text.replace(',', ".");
    let mut nums = Vec::new();
    let mut cur = String::new();
    for c in cleaned.chars() {
        if c.is_ascii_digit() || (c == '.' && !cur.is_empty()) {
            cur.push(c);
        } else {
            if let Ok(v) = cur.parse::<f64>() {
                nums.push(v);
            }
            cur.clear();
        }
    }
    if let Ok(v) = cur.parse::<f64>() {
        nums.push(v);
    }
    let lower = crate::fuzzy::sort_key(text);
    // The unit is read from whole words: "min" inside "administration"
    // used to turn 5 hours into 5 minutes.
    let word = |w: &str| {
        lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|token| token == w)
    };
    let factor = if word("jour") || word("jours") || word("semaine") || word("semaines") {
        if word("semaine") || word("semaines") {
            24.0 * 7.0
        } else {
            24.0
        }
    } else if word("min") || word("mn") || word("minute") || word("minutes") {
        1.0 / 60.0
    } else {
        1.0
    };
    match nums.len() {
        0 => None,
        1 => Some(nums[0] * factor),
        _ => Some((nums[0] + nums[1]) / 2.0 * factor),
    }
}

/// A stable colour per operator's initials, so a journal can be scanned
/// by who wrote what. Derived from the text, no configuration needed.
fn operator_color(operator: &str) -> egui::Color32 {
    const PALETTE: [egui::Color32; 6] = [
        egui::Color32::from_rgb(0x3a, 0x54, 0x7e),
        egui::Color32::from_rgb(0x2e, 0x6e, 0x4e),
        egui::Color32::from_rgb(0x7e, 0x3a, 0x5e),
        egui::Color32::from_rgb(0x8b, 0x5a, 0x1a),
        egui::Color32::from_rgb(0x1a, 0x6e, 0x8b),
        egui::Color32::from_rgb(0x5e, 0x3a, 0x7e),
    ];
    let key = operator.trim();
    if key.is_empty() {
        return motif::BG_DARK;
    }
    let sum: u32 = key.bytes().map(u32::from).sum();
    PALETTE[(sum as usize) % PALETTE.len()]
}

/// The three ways the agenda draws time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AgendaMode {
    /// One hour column for the selected day.
    Day,
    /// Monday to Sunday, one column per day.
    Week,
    /// The month as a grid of chips.
    Month,
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
        InterviewKind::Avk => egui::Color32::from_rgb(0x6e, 0x2e, 0x2e),
        InterviewKind::AnticancereuxLc => egui::Color32::from_rgb(0x5e, 0x3a, 0x7e),
        InterviewKind::AnticancereuxAutres => egui::Color32::from_rgb(0x7e, 0x4a, 0x2e),
        InterviewKind::Vaccination => egui::Color32::from_rgb(0x2e, 0x6e, 0x6e),
    }
}

/// The badge colour of a drug's administrative status: a rupture or a
/// withdrawal must be seen before the card is read.
fn status_color(status: &str) -> egui::Color32 {
    match db::DrugStatus::parse(status) {
        Some(db::DrugStatus::Withdrawn) => motif::ALERT,
        Some(db::DrugStatus::Shortage) => egui::Color32::from_rgb(0x8b, 0x5a, 0x1a),
        Some(db::DrugStatus::OffLabel) => egui::Color32::from_rgb(0x5e, 0x3a, 0x7e),
        Some(db::DrugStatus::Marketed) => egui::Color32::from_rgb(0x2e, 0x6e, 0x4e),
        None => motif::BG_DARK,
    }
}

/// Rank every interview of one patient inside its yearly cycle, keyed
/// by interview id — the fee slot each act falls into.
fn interview_ranks(
    interviews: &[db::Interview],
    months: u32,
) -> std::collections::HashMap<i64, (usize, usize)> {
    let mut by_kind: std::collections::HashMap<InterviewKind, Vec<(i64, String, bool)>> =
        std::collections::HashMap::new();
    for itv in interviews {
        let date = itv.created_at[..10.min(itv.created_at.len())].to_owned();
        by_kind.entry(itv.kind).or_default().push((
            itv.id,
            date,
            itv.treatment_change && itv.kind.allows_treatment_change(),
        ));
    }
    let mut out = std::collections::HashMap::new();
    for (kind, mut rows) in by_kind {
        // The table is newest-first; cycles are computed oldest-first.
        rows.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        let dates: Vec<String> = rows.iter().map(|(_, d, _)| d.clone()).collect();
        let restarts: Vec<bool> = rows.iter().map(|(_, _, r)| *r).collect();
        let positions = db::cycle_positions_for(kind, &dates, &restarts, months);
        for ((id, _, _), position) in rows.iter().zip(positions) {
            out.insert(*id, position);
        }
    }
    out
}

/// The memo's derogation conditions, checked against what the patient's
/// history actually holds. Returns `None` when the marked entretien
/// satisfies them, or what is still missing: how many entretiens short
/// the sequence before the change is, and the one after it.
fn treatment_change_shortfall(
    interviews: &[db::Interview],
    marked: &db::Interview,
    months: u32,
) -> Option<(usize, usize)> {
    if !marked.treatment_change || !marked.kind.allows_treatment_change() {
        return None;
    }
    let mut same: Vec<&db::Interview> = interviews
        .iter()
        .filter(|i| i.kind == marked.kind)
        .collect();
    same.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    let ranks = interview_ranks(interviews, months);
    let pos = same.iter().position(|i| i.id == marked.id)?;
    // The sequence the change closes runs back to the previous entretien
    // of rank 0; the one it opens runs up to the next.
    let mut before = 0usize;
    let mut closed_year = 0usize;
    for itv in same[..pos].iter().rev() {
        before += 1;
        if let Some((y, r)) = ranks.get(&itv.id).copied() {
            closed_year = y;
            if r == 0 {
                break;
            }
        }
    }
    let after = 1 + same[pos + 1..]
        .iter()
        .take_while(|i| ranks.get(&i.id).map(|(_, r)| *r) != Some(0))
        .count();
    // The memo splits on the year the change happens in — the year of
    // the sequence it closes, not the one it opens.
    let (need_before, need_after) = marked.kind.treatment_change_minimums(closed_year == 0);
    if before >= need_before && after >= need_after {
        None
    } else {
        Some((
            need_before.saturating_sub(before),
            need_after.saturating_sub(after),
        ))
    }
}

/// Quick act picker: the nine acts with digit shortcuts and the theme
/// the new act will carry. Returns the chosen kind (and closes) when a
/// row is clicked or its digit is pressed.
fn act_picker_window(ctx: &egui::Context, session: &mut Session) -> Option<InterviewKind> {
    // One digit per act, in the order the rows are drawn: 1-9 then 0
    // for the tenth. The list must cover `InterviewKind::ALL` — it held
    // nine keys for ten acts, and the loop below indexed past its end
    // on the first frame the picker was open, so Ctrl+N crashed.
    const DIGITS: [egui::Key; InterviewKind::ALL.len()] = [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num3,
        egui::Key::Num4,
        egui::Key::Num5,
        egui::Key::Num6,
        egui::Key::Num7,
        egui::Key::Num8,
        egui::Key::Num9,
        egui::Key::Num0,
    ];
    let mut chosen: Option<InterviewKind> = None;
    let mut close = false;
    egui::Window::new(tr("act_picker_title"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(tr("act_picker_hint"))
                    .size(11.0)
                    .color(motif::TEXT_DIM),
            );
            ui.add_space(6.0);
            egui::Grid::new("act_picker")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    for (i, kind) in InterviewKind::ALL.into_iter().enumerate() {
                        if ui
                            .add_sized(
                                [190.0, 24.0],
                                egui::Button::new(format!("{}  ·  {}", (i + 1) % 10, kind.label()))
                                    .fill(motif::BG),
                            )
                            .clicked()
                        {
                            chosen = Some(kind);
                        }
                        ui.label(
                            egui::RichText::new("     ")
                                .background_color(kind_color(kind))
                                .size(11.0),
                        );
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            theme_combo(ui, "act_picker_theme", &mut session.act_theme);
            ui.add_space(8.0);
            if motif::button(ui, tr("tpl_close")).clicked() {
                close = true;
            }
        });
    // The picker is not modal — the table behind it still takes text.
    // Only claim the digits when nothing else wants the keyboard, or
    // typing a duration would create acts behind the dialog.
    if !ctx.wants_keyboard_input() {
        for (i, kind) in InterviewKind::ALL.into_iter().enumerate() {
            if ctx.input(|inp| inp.key_pressed(DIGITS[i])) {
                chosen = Some(kind);
            }
        }
    }
    if close {
        session.act_picker = false;
        // An armed theme belongs to the act being picked; dropping the
        // picker drops it too, so it cannot leak onto a later act.
        session.act_theme.clear();
    }
    if chosen.is_some() {
        session.act_picker = false;
    }
    chosen
}

/// Thematic drop-down: the standard list plus "no theme" and whatever
/// free text the row already carries. Returns true when changed.
fn theme_combo(ui: &mut egui::Ui, id_salt: &str, theme: &mut String) -> bool {
    let mut changed = false;
    let shown = if theme.is_empty() {
        tr("itv_theme_none").to_owned()
    } else {
        theme.clone()
    };
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(egui::RichText::new(shown).size(12.0))
        .width(190.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(theme.is_empty(), tr("itv_theme_none"))
                .clicked()
            {
                theme.clear();
                changed = true;
            }
            for t in db::THEMES {
                if ui.selectable_label(theme == t, *t).clicked() {
                    *theme = (*t).to_owned();
                    changed = true;
                }
            }
        });
    changed
}

pub struct App {
    state: State,
    config: Config,
    last_activity: Instant,
    remember_password: bool,
    show_docs: bool,
    /// The left navigator dock: the list the active view is browsing
    /// (patients, drugs, the month), always in reach instead of taking
    /// the whole screen every time you need the next file (F6).
    show_nav: bool,
    /// The keyboard reference (F12), open or not.
    show_keys: bool,
    /// Ctrl+F asked for the navigator's search field; consumed by the
    /// dock on the next frame it draws.
    focus_nav: bool,
    /// Where the workspace was left: window size and dock widths.
    layout: crate::config::Layout,
    /// What is actually on disk, and when the two last diverged. The
    /// record is written on a debounce rather than only on the way out:
    /// `on_exit` never runs if the session is killed or the machine is
    /// switched off at the counter, which is how a shared post usually
    /// ends its day.
    layout_saved: crate::config::Layout,
    layout_changed: Instant,
    /// Which content the right pane shows: "docs", "carnet", "notes".
    side_pane: String,
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
    /// The (scale, density) pair currently applied to the egui style.
    applied_look: Option<(i32, motif::Density)>,
    /// Master-password change dialog, when open.
    pw_change: Option<PwChangeForm>,
    /// Operator initials for note stamps (default from config.toml).
    operator: String,
    /// The operator's personal notes (loaded for `op_notes_for`).
    op_notes: Vec<Note>,
    op_notes_for: Option<String>,
    op_note_text: String,
    op_note_confirm: Option<i64>,
    /// Typst template editor, when open.
    tpl_editor: Option<TplEditor>,
    /// Global options editor, when open.
    options: Option<OptionsEditor>,
}

/// In-app editor for `config.toml`.
/// The Options dialog's pages. A single scroll held every one of them,
/// five screens deep, so a fee matrix and an auto-lock timeout were the
/// same distance from the top.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OptionsPage {
    Pharmacy,
    Ui,
    Database,
    Fees,
    Rules,
}

impl OptionsPage {
    const ALL: [OptionsPage; 5] = [
        Self::Pharmacy,
        Self::Ui,
        Self::Database,
        Self::Fees,
        Self::Rules,
    ];
    /// The tab's own short label: the section headings inside run to a
    /// full sentence ("Règles (actes / année d'accompagnement…)"), which
    /// is a heading, not a tab.
    fn label(self) -> &'static str {
        match self {
            Self::Pharmacy => tr("opts_tab_pharmacy"),
            Self::Ui => tr("opts_tab_ui"),
            Self::Database => tr("opts_tab_db"),
            Self::Fees => tr("opts_tab_fees"),
            Self::Rules => tr("opts_tab_rules"),
        }
    }
}

struct OptionsEditor {
    page: OptionsPage,
    /// Two-step guard on the destructive reset button.
    confirm_reset: bool,
    cfg: Config,
    /// Text buffer for `[database] path` ("" = default location).
    db_path_text: String,
    /// Status line; `true` marks an error.
    message: Option<(bool, String)>,
}

struct TplEditor {
    target: TplTarget,
    text: String,
    /// Status line; `true` marks an error.
    message: Option<(bool, String)>,
}

/// Which Typst template the editor is showing.
#[derive(Clone, Copy, PartialEq)]
enum TplTarget {
    /// The interview sheet ("Fiche PDF").
    Fiche,
    /// The CR letter to the médecin traitant.
    Courrier,
    /// The carnet de transmissions page.
    Carnet,
    /// The ordonnance printed after a positive TROD.
    Ordonnance,
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
        let show_nav = config.ui.show_nav_on_start;

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
            match Db::open(&config.db_path(), &pw)
                .and_then(|db| Session::new(db, config.rules.cycle_months))
            {
                Ok(mut session) => {
                    spawn_daily_backup(config.db_path(), pw.clone(), config.database.backups_keep);
                    // Demo hook: land on a specific view (screenshots, e2e).
                    match std::env::var("BPM_CADDY_START_VIEW").as_deref() {
                        Ok("dashboard") => {
                            session.refresh_dashboard();
                            session.view = MainView::Dashboard;
                        }
                        // Landing on the quick picker needs the patient
                        // under it: same branch, one flag more.
                        Ok(v @ ("patient" | "act_picker")) => {
                            session.act_picker = v == "act_picker";
                            // Prefer the fullest record for screenshots.
                            let pick = session
                                .patients
                                .iter()
                                .find(|p| !p.email.is_empty())
                                .or(session.patients.first())
                                .cloned();
                            if let Some(p) = pick {
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
                        // The agenda's three modes are session state, so
                        // a screenshot of the day plan or the month grid
                        // needs its own hook.
                        Ok("agenda_day") => {
                            session.refresh_dashboard();
                            session.agenda_mode = AgendaMode::Day;
                            session.view = MainView::Agenda;
                        }
                        Ok("agenda_month") => {
                            session.refresh_dashboard();
                            session.agenda_mode = AgendaMode::Month;
                            session.agenda_month = true;
                            session.agenda_month_days =
                                session.db.month_grid(0).unwrap_or_default();
                            session.view = MainView::Agenda;
                        }
                        Ok(v @ ("protocols" | "protocol_open")) => {
                            session.show_protocols = true;
                            session.protocols = session.db.protocols().unwrap_or_default();
                            if v == "protocol_open" {
                                if let Some(p) = session.protocols.first().cloned() {
                                    session.protocol_nodes =
                                        session.db.protocol_nodes(p.id).unwrap_or_default();
                                    session.protocol_open = Some(p);
                                }
                            }
                            session.view = MainView::Drugs;
                        }
                        Ok("calc") => {
                            session.show_tables = true;
                            session.calc_open = true;
                            session.view = MainView::Drugs;
                        }
                        Ok("tables") => {
                            session.show_tables = true;
                            session.view = MainView::Drugs;
                        }
                        Ok("carnet") => {
                            session.load_transmissions();
                            session.view = MainView::Transmissions;
                        }
                        Ok("vaccine_map") => {
                            session.map_country = Some("ML");
                            session.view = MainView::VaccineMap;
                        }
                        // The carnet de vaccination is the patient
                        // file's second tab: open a patient, then turn
                        // to it.
                        // Land on a positive TROD with its ordonnance
                        // box open (screenshots, smoke).
                        Ok("ordonnance") => {
                            // The demo's TROD sits on one patient in
                            // particular: open files until one has it.
                            for candidate in session.patients.clone() {
                                session.open_patient(candidate);
                                if session
                                    .viewing_interviews
                                    .iter()
                                    .any(|i| crate::ordonnance::is_trod(i.kind))
                                {
                                    break;
                                }
                            }
                            if let Some(itv) = session
                                .viewing_interviews
                                .iter()
                                .find(|i| crate::ordonnance::is_trod(i.kind))
                                .cloned()
                            {
                                let _ = session.db.set_trod_result(
                                    itv.id,
                                    crate::ordonnance::POSITIF,
                                    &itv.trod_result,
                                );
                                if let Some(pid) = session.viewing.as_ref().map(|p| p.id) {
                                    session.reload_interviews(pid);
                                }
                                session.ordonnance = Some(OrdonnanceBox {
                                    interview: itv.id,
                                    kind: itv.kind,
                                    choice: crate::ordonnance::Choice::default(),
                                });
                            }
                        }
                        Ok("vaccins") => {
                            let pick = session
                                .patients
                                .iter()
                                .find(|p| !p.email.is_empty())
                                .or(session.patients.first())
                                .cloned();
                            if let Some(p) = pick {
                                session.open_patient(p);
                            }
                            session.patient_tab = PatientTab::Vaccins;
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
                            if let Some(d) = card {
                                session.open_drug_card(d);
                            }
                            // Land straight on the editable form, so the
                            // screenshots cover it too.
                            if std::env::var("BPM_CADDY_DRUG_EDIT").is_ok() {
                                session.drug_reading = false;
                            }
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

        // Screenshot/e2e hooks: open a dialog directly.
        let start_view = std::env::var("BPM_CADDY_START_VIEW").unwrap_or_default();
        let tpl_editor = if start_view == "template" {
            Some(TplEditor {
                target: TplTarget::Fiche,
                text: crate::pdf::default_template().to_owned(),
                message: None,
            })
        } else {
            None
        };
        let options = if start_view == "options" {
            Some(OptionsEditor {
                page: OptionsPage::Pharmacy,
                cfg: config.clone(),
                db_path_text: String::new(),
                message: None,
                confirm_reset: false,
            })
        } else {
            None
        };
        let side_pane = config.ui.side_pane.clone();
        Self {
            state,
            operator: config.ui.operator.clone(),
            config,
            last_activity: Instant::now(),
            remember_password,
            show_docs,
            show_nav,
            show_keys: start_view == "keys",
            focus_nav: false,
            layout: crate::config::Layout::load(),
            layout_saved: crate::config::Layout::load(),
            layout_changed: Instant::now(),
            side_pane,
            doc_base: doc_text.clone(),
            doc_text,
            doc_dirty: false,
            doc_last_edit: Instant::now(),
            doc_error: None,
            doc_focused: false,
            doc_check: Instant::now(),
            last_refresh: Instant::now(),
            applied_look: None,
            pw_change: None,
            tpl_editor,
            options,
            op_notes: Vec::new(),
            op_notes_for: None,
            op_note_text: String::new(),
            op_note_confirm: None,
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

    /// The pane showing the day's carnet: read the entries, add one.
    fn side_carnet(&mut self, ui: &mut egui::Ui) {
        let op = self.operator.trim().to_owned();
        let State::Unlocked(session) = &mut self.state else {
            return;
        };
        if session.trans_day.is_empty() {
            session.load_transmissions();
        }
        let title = db::weekday_fr(&session.trans_day)
            .map(|d| {
                format!(
                    "{}{} {}",
                    d.chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_default(),
                    d.chars().skip(1).collect::<String>(),
                    db::format_french_date(&session.trans_day)
                )
            })
            .unwrap_or_default();
        ui.label(egui::RichText::new(title).strong());
        ui.add_space(4.0);
        let is_today = session.trans_day == session.today;
        let (add, delete) = notes_box(
            ui,
            "side_carnet",
            &session.trans_notes,
            &mut session.note_text,
            &mut session.note_confirm,
            ui.available_height() - 60.0,
            is_today,
        );
        if let Some(body) = add {
            if let Err(e) = session
                .db
                .add_note(NoteSubject::Transmission, 0, &op, &body)
            {
                session.error = Some(e);
            }
            session.note_text.clear();
            session.load_transmissions();
        }
        if let Some(id) = delete {
            let _ = session.db.delete_note(id);
            session.load_transmissions();
        }
    }

    /// The pane showing only the operator's personal notes, with room.
    fn side_operator_notes(&mut self, ui: &mut egui::Ui) {
        let op = self.operator.trim().to_owned();
        if op.is_empty() {
            ui.label(
                egui::RichText::new(tr("op_notes_missing"))
                    .size(11.0)
                    .color(motif::TEXT_DIM),
            );
            return;
        }
        ui.label(egui::RichText::new(trf("op_notes_section", &op)).strong());
        ui.add_space(4.0);
        let (add, delete) = notes_box(
            ui,
            "side_op_notes",
            &self.op_notes,
            &mut self.op_note_text,
            &mut self.op_note_confirm,
            ui.available_height() - 60.0,
            true,
        );
        if let State::Unlocked(session) = &self.state {
            let mut changed = false;
            if let Some(body) = add {
                if session
                    .db
                    .add_note(NoteSubject::Operator, 0, &op, &body)
                    .is_ok()
                {
                    changed = true;
                }
                self.op_note_text.clear();
            }
            if let Some(id) = delete {
                let _ = session.db.delete_note(id);
                changed = true;
            }
            if changed {
                self.op_notes = session.db.notes_for_operator(&op).unwrap_or_default();
            }
        }
    }

    /// The left navigator dock: whatever list the active view is
    /// browsing, kept beside the work instead of replacing it.
    ///
    /// Before this, reaching the next patient meant leaving the open one
    /// (Escape, retype, re-open); the file being read and the list it
    /// came from were the same screen. They are now two.
    fn nav_dock(&mut self, ctx: &egui::Context) {
        let focus = std::mem::take(&mut self.focus_nav);
        // A dock is a share of the window, not a fixed slab: at 1024 px
        // a 232 px navigator and a 340 px notes pane left the work
        // itself 430 px, narrower than either of them. Once it has been
        // dragged, though, the width the operator chose wins.
        let screen = ctx.screen_rect().width();
        let default_w = if self.layout.nav_width >= 130.0 {
            self.layout.nav_width
        } else {
            (screen * 0.15).clamp(150.0, 232.0)
        };
        let mut width = 0.0_f32;
        {
            let State::Unlocked(session) = &mut self.state else {
                return;
            };
            let view = session.view;
            egui::SidePanel::left("navigator")
                .resizable(true)
                .default_width(default_w)
                .min_width(130.0)
                .max_width((screen * 0.3).clamp(180.0, 420.0))
                .show(ctx, |ui| {
                    width = ui.max_rect().width() + 16.0;
                    ui.add_space(6.0);
                    match view {
                        MainView::Drugs => Self::nav_drugs(ui, session, focus),
                        MainView::Agenda => Self::nav_agenda(ui, session),
                        MainView::Transmissions => Self::nav_carnet(ui, session),
                        MainView::VaccineMap => Self::nav_map(ui, session),
                        MainView::Dashboard | MainView::Search => {
                            Self::nav_patients(ui, session, focus)
                        }
                    }
                });
        }
        if width >= 130.0 {
            self.layout.nav_width = width;
        }
    }

    /// The dock's search box: a sunken full-width field with a caption.
    fn nav_search(ui: &mut egui::Ui, hint: &str, text: &mut String) -> egui::Response {
        let resp = ui.add_sized(
            [ui.available_width(), 26.0],
            egui::TextEdit::singleline(text).hint_text(hint),
        );
        motif::bevel(ui.painter(), resp.rect.expand(2.0), false);
        ui.add_space(6.0);
        resp
    }

    /// The well every dock list sits in: it takes the rest of the pane,
    /// so the list grows with the window instead of with the content.
    fn nav_list<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) {
        let rect = ui.available_rect_before_wrap();
        if rect.height() < 20.0 {
            return;
        }
        let inner = motif::well(ui, rect);
        motif::inside(ui, inner, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("nav_list")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    add(ui);
                });
        });
    }

    /// Patients: the search that used to own the whole screen, docked.
    fn nav_patients(ui: &mut egui::Ui, session: &mut Session, focus: bool) {
        motif::section(ui, tr("nav_patients"));
        ui.add_space(4.0);
        let field = Self::nav_search(ui, tr("nav_search_hint"), &mut session.query);
        if focus {
            field.request_focus();
        }
        if field.changed() {
            session.selected = 0;
            session.new_patient = None;
        }
        let results: Vec<Patient> = session.results().into_iter().cloned().collect();
        let open = session.viewing.as_ref().map(|p| p.id);
        let mut clicked: Option<Patient> = None;
        // The dock is the search now, so it answers to the keys the
        // search always did: type, arrow down, Enter. Only while its own
        // field has focus, so the arrows still drive the agenda and the
        // act table everywhere else.
        if field.has_focus() && !results.is_empty() {
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
                clicked = Some(results[session.selected].clone());
            }
        }
        let cursor = field.has_focus().then_some(session.selected);
        ui.label(
            egui::RichText::new(trf("nav_count", results.len()))
                .size(10.5)
                .color(motif::TEXT_FAINT),
        );
        ui.add_space(3.0);
        Self::nav_list(ui, |ui| {
            for (i, p) in results.iter().enumerate() {
                let pending = session.pending.get(&p.id).copied().unwrap_or(0);
                let mut text = p.full_name();
                if pending > 0 {
                    text.push_str(&format!("   ({pending})"));
                }
                // The open file stays marked; the keyboard cursor marks
                // where Enter would go, which is not always the same row.
                let selected = cursor == Some(i) || (cursor.is_none() && open == Some(p.id));
                let row = motif::list_row(ui, egui::RichText::new(text), selected)
                    .on_hover_text(db::format_french_date(&p.birth_date));
                if cursor == Some(i) {
                    row.scroll_to_me(None);
                }
                if row.clicked() {
                    clicked = Some(p.clone());
                }
            }
        });
        if let Some(p) = clicked {
            session.view = MainView::Search;
            session.show_amounts = false;
            session.open_patient(p);
        }
    }

    /// Drugs: the reference base's index, so a card can be compared with
    /// the next one without going back through the search screen.
    fn nav_drugs(ui: &mut egui::Ui, session: &mut Session, focus: bool) {
        motif::section(ui, tr("nav_drugs"));
        ui.add_space(4.0);
        let field = Self::nav_search(ui, tr("drug_search_hint"), &mut session.drug_query);
        if focus {
            field.request_focus();
        }
        if field.changed() {
            session.drug_selected = 0;
        }
        let results = session.drug_results(60);
        let open = session.drug_form.as_ref().map(|d| d.id);
        let mut clicked: Option<Drug> = None;
        // Same keys as the patient dock: type, arrow down, Enter.
        if field.has_focus() && !results.is_empty() {
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
                clicked = Some(results[session.drug_selected].clone());
            }
        }
        let cursor = field.has_focus().then_some(session.drug_selected);
        ui.label(
            egui::RichText::new(trf("nav_count", results.len()))
                .size(10.5)
                .color(motif::TEXT_FAINT),
        );
        ui.add_space(3.0);
        Self::nav_list(ui, |ui| {
            for (i, d) in results.iter().enumerate() {
                let mut hover = d.dci.clone();
                if !d.class.is_empty() {
                    if !hover.is_empty() {
                        hover.push_str(" — ");
                    }
                    hover.push_str(&d.class);
                }
                let selected = cursor == Some(i) || (cursor.is_none() && open == Some(d.id));
                let row = motif::list_row(ui, egui::RichText::new(d.name.trim()), selected);
                let row = if hover.is_empty() {
                    row
                } else {
                    row.on_hover_text(hover)
                };
                if cursor == Some(i) {
                    row.scroll_to_me(None);
                }
                if row.clicked() {
                    clicked = Some(d.clone());
                }
            }
        });
        if let Some(d) = clicked {
            session.open_drug_card(d);
            session.error = None;
        }
    }

    /// Agenda: a month at a glance, and the queue of rendez-vous.
    ///
    /// The dock browses; the view works. Picking a day here moves the
    /// calendar without leaving whatever mode it is in.
    fn nav_agenda(ui: &mut egui::Ui, session: &mut Session) {
        motif::section(ui, tr("nav_month"));
        ui.add_space(4.0);
        if session.agenda_month_days.is_empty() {
            session.agenda_month_days = session
                .db
                .month_grid(session.agenda_month_offset)
                .unwrap_or_default();
        }
        ui.horizontal(|ui| {
            if motif::button(ui, "‹").clicked() {
                session.agenda_month_offset -= 1;
                session.agenda_month_days = session
                    .db
                    .month_grid(session.agenda_month_offset)
                    .unwrap_or_default();
            }
            if motif::button(ui, tr("agenda_this_week")).clicked() {
                session.agenda_month_offset = 0;
                session.agenda_month_days = session.db.month_grid(0).unwrap_or_default();
            }
            if motif::button(ui, "›").clicked() {
                session.agenda_month_offset += 1;
                session.agenda_month_days = session
                    .db
                    .month_grid(session.agenda_month_offset)
                    .unwrap_or_default();
            }
        });
        ui.add_space(4.0);
        // Six rows of seven: the day's number, tinted by how loaded it
        // is, with today ringed and the selected day filled.
        let days = session.agenda_month_days.clone();
        let mut pick: Option<String> = None;
        if days.len() >= 28 {
            let w = ui.available_width();
            let cell = (w / 7.0).floor().max(14.0);
            let rows = days.len().div_ceil(7);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(cell * 7.0, cell * rows as f32 + 14.0),
                egui::Sense::hover(),
            );
            for (i, label) in ["L", "M", "M", "J", "V", "S", "D"].iter().enumerate() {
                ui.painter().text(
                    egui::pos2(rect.left() + (i as f32 + 0.5) * cell, rect.top() + 6.0),
                    egui::Align2::CENTER_CENTER,
                    *label,
                    egui::FontId::proportional(9.5),
                    motif::TEXT_FAINT,
                );
            }
            for (i, day) in days.iter().enumerate() {
                let r = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + (i % 7) as f32 * cell,
                        rect.top() + 14.0 + (i / 7) as f32 * cell,
                    ),
                    egui::vec2(cell - 1.0, cell - 1.0),
                );
                let load = session
                    .appointments
                    .iter()
                    .filter(|a| &a.date == day)
                    .count();
                let selected = *day == session.agenda_day;
                if selected {
                    ui.painter().rect_filled(r, 0.0, motif::ACCENT);
                } else if load > 0 {
                    ui.painter().rect_filled(
                        r,
                        0.0,
                        motif::ACCENT.gamma_multiply((0.18 + 0.14 * load as f32).min(0.6)),
                    );
                }
                if *day == session.today {
                    ui.painter()
                        .rect_stroke(r, 0.0, egui::Stroke::new(1.5_f32, motif::ALERT));
                }
                let num = day.get(8..10).unwrap_or("").trim_start_matches('0');
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    num,
                    egui::FontId::proportional(10.5),
                    if selected {
                        egui::Color32::WHITE
                    } else {
                        motif::TEXT
                    },
                );
                let resp = ui.interact(r, ui.id().with(("navday", i)), egui::Sense::click());
                if resp
                    .on_hover_text(trn(
                        "dash_load_tooltip",
                        &[&db::format_french_date(day), &load],
                    ))
                    .clicked()
                {
                    pick = Some(day.clone());
                }
            }
        }
        if let Some(day) = pick {
            session.agenda_day = day;
            session.load_day();
        }

        ui.add_space(10.0);
        motif::section(ui, tr("nav_next"));
        ui.add_space(4.0);
        let today = session.today.clone();
        let mut open_id: Option<i64> = None;
        Self::nav_list(ui, |ui| {
            for rdv in session.appointments.iter().take(60) {
                let overdue = !today.is_empty() && rdv.date < today;
                let text = format!(
                    "{}  {}",
                    db::format_french_date(&rdv.date),
                    rdv.patient_name
                );
                let label = if overdue {
                    egui::RichText::new(text).color(motif::ALERT)
                } else {
                    egui::RichText::new(text)
                };
                if motif::list_row(ui, label, false)
                    .on_hover_text(rdv.kind.label())
                    .clicked()
                {
                    open_id = Some(rdv.patient_id);
                }
            }
        });
        if let Some(id) = open_id {
            if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                session.view = MainView::Search;
                session.open_patient(p);
            }
        }
    }

    /// Carnet: the days that carry entries.
    fn nav_carnet(ui: &mut egui::Ui, session: &mut Session) {
        motif::section(ui, tr("nav_days"));
        ui.add_space(4.0);
        let days = session.trans_days.clone();
        let current = session.trans_day.clone();
        let mut pick: Option<String> = None;
        Self::nav_list(ui, |ui| {
            for day in &days {
                if motif::list_row(
                    ui,
                    egui::RichText::new(db::format_french_date(day)),
                    *day == current,
                )
                .clicked()
                {
                    pick = Some(day.clone());
                }
            }
        });
        if let Some(day) = pick {
            session.trans_day = day;
            session.load_transmissions();
        }
    }

    /// Map: the country list, grouped, with the search box above it.
    /// Picking a name here and clicking a tile do the same thing.
    fn nav_map(ui: &mut egui::Ui, session: &mut Session) {
        let resp = Self::nav_search(ui, tr("map_search_hint"), &mut session.map_query);
        let _ = resp;
        let query = session.map_query.clone();
        let selected = session.map_country;
        let mut pick: Option<&'static str> = None;
        let matches = vaccines::search(&query);
        if !query.trim().is_empty() {
            ui.label(
                egui::RichText::new(trf("nav_count", matches.len()))
                    .size(11.0)
                    .color(motif::TEXT_DIM),
            );
            ui.add_space(4.0);
        }
        Self::nav_list(ui, |ui| {
            if !query.trim().is_empty() {
                for country in matches {
                    if motif::list_row(
                        ui,
                        egui::RichText::new(country.name),
                        selected == Some(country.code),
                    )
                    .clicked()
                    {
                        pick = Some(country.code);
                    }
                }
                return;
            }
            for region in vaccines::Region::ALL {
                motif::section(ui, region.label());
                for country in vaccines::COUNTRIES.iter().filter(|c| c.region == region) {
                    if motif::list_row(
                        ui,
                        egui::RichText::new(country.name).size(12.0),
                        selected == Some(country.code),
                    )
                    .clicked()
                    {
                        pick = Some(country.code);
                    }
                }
                ui.add_space(4.0);
            }
        });
        if let Some(code) = pick {
            session.map_country = Some(code);
        }
    }

    fn docs_pane(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect().width();
        let default_w = if self.layout.docs_width >= 200.0 {
            self.layout.docs_width
        } else {
            (screen * 0.21).clamp(220.0, 340.0)
        };
        let mut width = 0.0_f32;
        egui::SidePanel::right("team_docs")
            .resizable(true)
            .default_width(default_w)
            .min_width(200.0)
            // A side panel that grows past the width it reserved leaves
            // the central view laid out wider than it is visible, and
            // everything on its right edge is clipped away — buttons
            // included. Cap it, and let the content inside wrap.
            .max_width((screen * 0.36).clamp(260.0, 520.0))
            .show(ctx, |ui| {
                width = ui.max_rect().width() + 16.0;
                ui.add_space(6.0);
                // One pane, three contents: the shared documentation,
                // the day's carnet, or the operator's own notes.
                ui.horizontal_wrapped(|ui| {
                    for (value, label) in [
                        ("docs", tr("side_pane_docs")),
                        ("carnet", tr("side_pane_carnet")),
                        ("notes", tr("side_pane_notes")),
                    ] {
                        if ui
                            .selectable_label(self.side_pane == value, label)
                            .on_hover_text(tr("side_pane_switch"))
                            .clicked()
                        {
                            self.side_pane = value.to_owned();
                            if value == "carnet" {
                                if let State::Unlocked(session) = &mut self.state {
                                    session.trans_day.clear();
                                    session.load_transmissions();
                                }
                            }
                        }
                    }
                });
                ui.add_space(4.0);
                if self.side_pane == "carnet" {
                    self.side_carnet(ui);
                    return;
                }
                if self.side_pane == "notes" {
                    self.side_operator_notes(ui);
                    return;
                }
                let status = if let Some(err) = &self.doc_error {
                    err.clone()
                } else if self.doc_dirty {
                    tr("docs_saving").to_owned()
                } else {
                    tr("docs_saved").to_owned()
                };
                // Succinct entries: one click stamps date · operator ·
                // current patient. The timestamp is only queried on
                // click, never per frame.
                let unlocked = matches!(self.state, State::Unlocked(_));
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(status).size(11.0).color(
                        if self.doc_error.is_some() {
                            motif::ALERT
                        } else {
                            motif::TEXT_DIM
                        },
                    ));
                    ui.add_space(4.0);
                    ui.add_sized([46.0, 22.0], egui::TextEdit::singleline(&mut self.operator))
                        .on_hover_text(tr("docs_operator"));
                    if unlocked
                        && motif::button(ui, tr("docs_stamp"))
                            .on_hover_text(tr("docs_stamp_tooltip"))
                            .clicked()
                    {
                        let (now, patient) = match &self.state {
                            State::Unlocked(s) => (
                                s.db.now_stamp().unwrap_or_default(),
                                s.viewing.as_ref().map(|p| p.full_name()),
                            ),
                            State::Locked { .. } => (String::new(), None),
                        };
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
                });
                ui.add_space(4.0);

                // The operator's own journal only claims room when there
                // is an operator to claim it: 185 px were reserved for
                // it whether or not the field was filled in.
                let op = self.operator.trim().to_owned();
                let reserve = if op.is_empty() { 34.0 } else { 185.0 };
                let mut editor_rect = ui.available_rect_before_wrap().shrink(2.0);
                editor_rect
                    .set_bottom((editor_rect.bottom() - reserve).max(editor_rect.top() + 60.0));
                motif::bevel(ui.painter(), editor_rect, false);
                egui::ScrollArea::vertical()
                    .id_salt("team_doc")
                    .max_height(editor_rect.height())
                    .show(ui, |ui| {
                        let response = ui.add_sized(
                            [ui.available_width(), editor_rect.height() - 8.0],
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

                // Personal notes of the operator (private journal).
                ui.add_space(10.0);
                if let State::Unlocked(session) = &self.state {
                    if self.op_notes_for.as_deref() != Some(op.as_str()) {
                        self.op_notes = if op.is_empty() {
                            Vec::new()
                        } else {
                            session.db.notes_for_operator(&op).unwrap_or_default()
                        };
                        self.op_notes_for = Some(op.clone());
                    }
                }
                if op.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("op_notes_missing"))
                            .size(11.0)
                            .color(motif::TEXT_DIM),
                    );
                } else if matches!(self.state, State::Unlocked(_)) {
                    motif::section(ui, &trf("op_notes_section", &op));
                    ui.add_space(4.0);
                    let (add, delete) = notes_box(
                        ui,
                        "op_notes",
                        &self.op_notes,
                        &mut self.op_note_text,
                        &mut self.op_note_confirm,
                        84.0,
                        true,
                    );
                    if let State::Unlocked(session) = &self.state {
                        let mut changed = false;
                        if let Some(body) = add {
                            if session
                                .db
                                .add_note(NoteSubject::Operator, 0, &op, &body)
                                .is_ok()
                            {
                                changed = true;
                            }
                            self.op_note_text.clear();
                        }
                        if let Some(id) = delete {
                            let _ = session.db.delete_note(id);
                            changed = true;
                        }
                        if changed {
                            self.op_notes = session.db.notes_for_operator(&op).unwrap_or_default();
                        }
                    }
                }
            });
        if width >= 200.0 {
            self.layout.docs_width = width;
        }
    }

    /// The keyboard reference (F12).
    ///
    /// The app is driven from the keyboard — that is the point of it at
    /// a counter — and until now the only way to learn a key was to be
    /// told. Every shortcut it answers to, on one page, grouped by what
    /// it acts on.
    fn keys_window(&mut self, ctx: &egui::Context) {
        // (key, what it does). An empty key starts a new group.
        let rows: [(&str, &str); 23] = [
            ("", tr("keys_group_workspace")),
            ("F1", tr("toolbar_docs_tooltip")),
            ("F6", tr("toolbar_nav_tooltip")),
            ("F12", tr("keys_this")),
            ("Ctrl+Tab", tr("keys_next_tab")),
            ("Ctrl+Shift+Tab", tr("keys_prev_tab")),
            ("Ctrl+W", tr("keys_close_tab")),
            ("", tr("keys_group_views")),
            ("F2", tr("tab_dashboard")),
            ("F3", tr("tab_drugs")),
            ("F4", tr("tab_agenda")),
            ("F5", tr("tab_carnet")),
            ("F7", tr("tab_map")),
            ("Ctrl+F", tr("keys_search")),
            ("Échap", tr("keys_back")),
            ("", tr("keys_group_work")),
            ("↑ ↓", tr("keys_updown")),
            ("Entrée", tr("keys_enter")),
            ("Ctrl+N", tr("keys_new_act")),
            ("1 … 9, 0", tr("keys_act_digit")),
            ("← →", tr("keys_arrows")),
            ("", tr("keys_group_dates")),
            ("230826 · 2308", tr("keys_dates")),
        ];
        let mut open = true;
        egui::Window::new(tr("keys_title"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                egui::Grid::new("keys")
                    .num_columns(2)
                    .spacing([18.0, 3.0])
                    .show(ui, |ui| {
                        for (key, what) in rows {
                            if key.is_empty() {
                                // A group heading spanning both columns.
                                ui.label("");
                                ui.label(
                                    egui::RichText::new(what)
                                        .size(11.0)
                                        .strong()
                                        .color(motif::TEXT_DIM),
                                );
                                ui.end_row();
                                continue;
                            }
                            // The key itself as a keycap: a raised bevel
                            // in the monospace face, so the eye can scan
                            // the left column for the one it wants.
                            let font = egui::FontId::monospace(11.5);
                            let galley = ui.painter().layout_no_wrap(
                                key.to_owned(),
                                font.clone(),
                                motif::TEXT,
                            );
                            let (cap, _) = ui.allocate_exact_size(
                                egui::vec2(galley.size().x + 14.0, galley.size().y + 8.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(cap, 0.0, motif::BG);
                            motif::bevel(ui.painter(), cap, true);
                            ui.painter().galley(
                                cap.center() - galley.size() / 2.0,
                                galley,
                                motif::TEXT,
                            );
                            ui.label(egui::RichText::new(what).size(12.0));
                            ui.end_row();
                        }
                    });
            });
        if !open {
            self.show_keys = false;
        }
    }

    fn unlock_screen(&mut self, ctx: &egui::Context) {
        let db_path = self.config.db_path();
        let mut remember = self.remember_password;
        let State::Locked { password, error } = &mut self.state else {
            return;
        };
        let mut attempt: Option<String> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // A raised Motif dialog box, the way an X login screen has
            // always looked, rather than four lines of text floating on
            // a field of grey the size of the window.
            let screen = ui.max_rect();
            let w = 420.0_f32.min(screen.width() - 40.0);
            // Sized from the type scale rather than in fixed pixels: at
            // 1.4x the unlock button fell out through the bottom edge.
            let unit = ui.text_style_height(&egui::TextStyle::Body) / 14.0;
            let h = (if error.is_some() { 246.0 } else { 196.0 }) * unit;
            let box_rect = egui::Rect::from_center_size(
                egui::pos2(screen.center().x, screen.center().y - 40.0),
                egui::vec2(w, h),
            );
            // The hard Motif shadow, then the panel itself.
            ui.painter().rect_filled(
                box_rect.translate(egui::vec2(5.0, 5.0)),
                0.0,
                motif::BG_DARK,
            );
            motif::panel(ui, box_rect, None, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0 * unit;
                ui.vertical_centered(|ui| {
                    // The app's own icon, painted: a raised bevel square
                    // with a sunken accent centre.
                    let (mark, _) =
                        ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
                    ui.painter().rect_filled(mark, 0.0, motif::BG);
                    motif::bevel(ui.painter(), mark, true);
                    let inner = mark.shrink(9.0);
                    ui.painter().rect_filled(inner, 0.0, motif::ACCENT);
                    motif::bevel(ui.painter(), inner, false);
                    ui.add_space(4.0);
                    ui.heading("BPM-Caddy");
                    ui.label(
                        egui::RichText::new(tr("lock_subtitle"))
                            .size(11.5)
                            .color(motif::TEXT_DIM),
                    );
                    ui.add_space(14.0);

                    let field = ui.add_sized(
                        [(ui.available_width() - 20.0).clamp(60.0, 320.0), 30.0],
                        egui::TextEdit::singleline(password)
                            .password(true)
                            .hint_text(tr("lock_password_hint")),
                    );
                    motif::bevel(ui.painter(), field.rect.expand(2.0), false);
                    // The lock screen holds a single field, so any Enter
                    // press submits. The previous focus-based idiom
                    // silently failed here: pressing Enter makes the
                    // field surrender focus, and the re-focus below then
                    // made `lost_focus()` false again, so Enter did
                    // nothing.
                    let submitted = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if !ctx.wants_keyboard_input() {
                        field.request_focus();
                    }

                    ui.add_space(10.0);
                    ui.checkbox(&mut remember, tr("lock_remember"));
                    ui.add_space(6.0);
                    if (motif::button(ui, tr("lock_unlock")).clicked() || submitted)
                        && !password.is_empty()
                    {
                        attempt = Some(password.clone());
                    }
                    if let Some(err) = error {
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(err.as_str())
                                    .size(11.5)
                                    .color(motif::ALERT),
                            )
                            .wrap(),
                        );
                    }
                });
            });
            // Which database this post opens — misconfigured posts
            // (wrong network path) are spotted before unlocking. Under
            // the box, where it belongs: it is about the machine, not
            // about the password.
            ui.painter().text(
                egui::pos2(screen.center().x, box_rect.bottom() + 22.0),
                egui::Align2::CENTER_CENTER,
                elide(
                    ui,
                    &trf("lock_db_path", db_path.display()),
                    screen.width() - 40.0,
                    11.0,
                ),
                egui::FontId::proportional(11.0),
                motif::TEXT_DIM,
            );
        });

        self.remember_password = remember;
        if let Some(pw) = attempt {
            match Db::open(&self.config.db_path(), &pw)
                .and_then(|db| Session::new(db, self.config.rules.cycle_months))
            {
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
        let show_nav = self.show_nav;
        let State::Unlocked(session) = &mut self.state else {
            return;
        };

        // Ctrl+F returns to the search from anywhere (spec 3.1). With
        // the navigator open the search is right there, so the key just
        // puts the cursor in it — closing the open file to reach a
        // search bar was the old shape of the app, not this one.
        let focus_search = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F));
        if focus_search && show_nav {
            self.focus_nav = true;
        } else if focus_search {
            session.flush_date_edits();
            session.view = MainView::Search;
            session.viewing = None;
            session.show_amounts = false;
        }
        let focus_search = focus_search && !show_nav;
        // Escape backs out of the dashboard, like everywhere else.
        if session.view == MainView::Dashboard
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
            && !ctx.wants_keyboard_input()
        {
            session.view = MainView::Search;
            session.show_amounts = false;
        }

        let config = self.config.clone();
        let operator = self.operator.clone();
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
                Self::drugs_view(ui, ctx, session, doc, &operator);
                return;
            }
            if session.view == MainView::Agenda {
                Self::agenda_view(ui, ctx, session, &operator, &config);
                return;
            }
            if session.view == MainView::Transmissions {
                Self::transmissions_view(ui, ctx, session, &operator, &config);
                return;
            }
            if session.view == MainView::VaccineMap {
                Self::vaccine_map_view(ui, session);
                return;
            }
            if let Some(patient) = session.viewing.clone() {
                Self::patient_view(ui, ctx, session, &patient, &config, &operator);
                return;
            }

            let idle = session.query.trim().is_empty();
            motif::page(ui, 720.0, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(if idle { 22.0 } else { 12.0 });
                    ui.heading("BPM-Caddy");
                    ui.label(tr("app_tagline"));
                });
                ui.add_space(14.0);
                let search = ui.add_sized(
                    [ui.available_width(), 32.0],
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
            });
            ui.add_space(12.0);

            let results: Vec<Patient> = session.results().into_iter().cloned().collect();

            // An empty query matches everyone, and that list is already
            // in the left dock: the middle of the screen is better spent
            // on the day itself.
            if !results.is_empty() && !idle {
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

                // Sunken Motif list box, centered, with full-width rows.
                let avail = motif::visible_rect(ui);
                let w = avail.width().min(720.0);
                let h = (avail.height() - 14.0).max(140.0);
                let box_rect = egui::Rect::from_min_size(
                    egui::pos2(avail.center().x - w / 2.0, avail.top()),
                    egui::vec2(w, h),
                );
                ui.painter().rect_filled(box_rect, 0.0, motif::TROUGH);
                motif::bevel(ui.painter(), box_rect, false);
                let builder = egui::UiBuilder::new().max_rect(box_rect.shrink(4.0));
                ui.allocate_new_ui(builder, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, p) in results.iter().enumerate() {
                            let selected = i == session.selected;
                            let name = p.full_name();
                            // Highlight the letters the fuzzy query
                            // matched (none when only the "Last First"
                            // orientation matched).
                            let indices = fuzzy::score_with_indices(&session.query, &name)
                                .map(|(_, idx)| idx)
                                .unwrap_or_default();
                            let base_color = if selected {
                                egui::Color32::WHITE
                            } else {
                                motif::TEXT
                            };
                            let font = egui::FontId::proportional(14.0);
                            let plain = egui::TextFormat {
                                font_id: font.clone(),
                                color: base_color,
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
                            let mut rest =
                                trf("search_born", db::format_french_date(&p.birth_date));
                            match pending {
                                0 => {}
                                1 => rest.push_str(tr("search_pending_one")),
                                n => rest.push_str(&trf("search_pending_many", n)),
                            }
                            job.append(&rest, 0.0, plain);
                            if motif::list_row_job(ui, job, selected).clicked() {
                                session.open_patient(p.clone());
                            }
                        }
                    });
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
                    let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
                    let submitted = egui::Grid::new("new_patient")
                        .min_col_width(110.0)
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
                        ui.colored_label(motif::ALERT, err.as_str());
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

            if idle {
                Self::home_panels(ui, session);
            }
            if let Some(err) = &session.error {
                ui.vertical_centered(|ui| {
                    ui.colored_label(motif::ALERT, err.as_str());
                });
            }
        });
    }

    /// What the counter needs before anyone has typed anything: the
    /// day's rendez-vous, the files the team touched last, and what was
    /// written today. The search screen used to be a box on an empty
    /// grey field the size of the window.
    fn home_panels(ui: &mut egui::Ui, session: &mut Session) {
        let body = motif::visible_rect(ui).shrink2(egui::vec2(10.0, 0.0));
        if body.height() < 120.0 {
            return;
        }
        let mut open_patient: Option<i64> = None;
        let mut open_recent: Option<Patient> = None;
        // The day and the recent files share the top row; the notes run
        // the full width underneath, because they are prose.
        let split = body.width() >= 760.0;
        let rects: [egui::Rect; 3] = if split {
            let rows = motif::split_rows(body, &[body.height() * 0.55, 0.0], 8.0);
            let top = motif::split_columns(rows[0], 2, 8.0);
            [top[0], top[1], rows[1]]
        } else {
            let rows = motif::split_rows(body, &[0.0, 0.0, 0.0], 8.0);
            [rows[0], rows[1], rows[2]]
        };
        let titles = [tr("home_today"), tr("dash_recent"), tr("dash_today_notes")];
        for (i, (title, rect)) in titles.iter().zip(rects).enumerate() {
            motif::panel(ui, rect, Some(title), |ui| {
                let body = ui.max_rect();
                match i {
                    0 => open_patient = Self::home_today(ui, session, body),
                    1 => open_recent = Self::dash_recent(ui, session, body),
                    _ => Self::dash_today_notes(ui, session, body),
                }
            });
        }
        ui.allocate_space(egui::vec2(body.width(), body.height()));
        if let Some(p) = open_recent {
            session.open_patient(p);
        } else if let Some(id) = open_patient {
            if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                session.open_patient(p);
            }
        }
    }

    /// Today's rendez-vous, and what is already overdue.
    fn home_today(ui: &mut egui::Ui, session: &Session, rect: egui::Rect) -> Option<i64> {
        let today: Vec<&Appointment> = session
            .appointments
            .iter()
            .filter(|a| !session.today.is_empty() && a.date <= session.today)
            .collect();
        if today.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("home_today_empty"),
                egui::FontId::proportional(12.0),
                motif::TEXT_DIM,
            );
            return None;
        }
        let mut open = None;
        let rows: Vec<(String, bool, i64)> = today
            .iter()
            .map(|a| {
                let overdue = a.date < session.today;
                let hour = if a.time.is_empty() {
                    String::new()
                } else {
                    format!("{}  ", a.time)
                };
                let mut text = format!("{hour}{}  ({})", a.patient_name, a.kind.label());
                if overdue {
                    text = format!("{}  {}", db::format_french_date(&a.date), text);
                }
                (text, overdue, a.patient_id)
            })
            .collect();
        motif::inside(ui, rect, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("home_today")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for (text, overdue, id) in &rows {
                        let label = if *overdue {
                            egui::RichText::new(text).color(motif::ALERT)
                        } else {
                            egui::RichText::new(text).color(motif::ACCENT).strong()
                        };
                        if motif::list_row(ui, label, false)
                            .on_hover_text(tr("dash_open_patient"))
                            .clicked()
                        {
                            open = Some(*id);
                        }
                    }
                });
        });
        open
    }

    #[allow(clippy::too_many_arguments)]
    fn patient_view(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        patient: &Patient,
        config: &Config,
        operator: &str,
    ) {
        // Escape closes the patient view — but while a text field has
        // focus it only drops that focus (egui's own behavior); acting on
        // both at once would throw away an in-progress date edit.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            // The quick picker and the ordonnance box sit on top:
            // Escape dismisses whichever is open first, and leaves the
            // patient view where it was. Closing both at once would
            // throw away a prescription being composed.
            if session.act_picker {
                session.act_picker = false;
                session.act_theme.clear();
            } else if session.ordonnance.is_some() {
                session.ordonnance = None;
            } else {
                session.flush_date_edits();
                session.viewing = None;
                return;
            }
        }

        // An identity band across the top, then the two things the
        // file is open for side by side: the acts table and the
        // journal. It used to be one centred column three screens tall,
        // so the table — the reason to open a fiche at all — began
        // below the fold on every window.
        let body = motif::visible_rect(ui).shrink(6.0);
        // The band is as tall as its content: the act buttons wrap, and
        // an open correction form is much taller than a header.
        let band_h = Self::patient_band_height(ui, session, patient);
        let rows = motif::split_rows(body, &[band_h, 0.0], 8.0);
        motif::panel(ui, rows[0], None, |ui| {
            let inner = ui.max_rect();
            egui::ScrollArea::vertical()
                .id_salt("patient_band")
                .show(ui, |ui| {
                    ui.set_max_width(inner.width());
                    Self::patient_identity_pane(ui, ctx, session, patient, config, operator);
                });
        });
        // The patient may have just been closed or deleted by the band.
        if session.viewing.as_ref().map(|p| p.id) != Some(patient.id) {
            return;
        }
        // The file has two halves — the acts and the carnet de
        // vaccination — and each wants the whole work area. They take
        // turns behind a notebook strip instead of sharing a split.
        let strip = motif::split_rows(rows[1], &[28.0, 0.0], 4.0);
        let active = match session.patient_tab {
            PatientTab::Acts => 0,
            PatientTab::Vaccins => 1,
        };
        motif::inside(ui, strip[0], |ui| {
            let tabs = [
                motif::Tab::new(tr("patient_tab_acts")),
                motif::Tab::new(tr("patient_tab_vaccins")),
            ];
            if let Some(motif::TabAction::Select(i)) = motif::tab_strip(ui, &tabs, active) {
                session.patient_tab = if i == 0 {
                    PatientTab::Acts
                } else {
                    PatientTab::Vaccins
                };
            }
        });
        let work = strip[1];
        // The ordonnance box floats over whichever half is on screen.
        Self::ordonnance_box(ctx, session, patient, config);
        if session.patient_tab == PatientTab::Vaccins {
            Self::patient_vaccins_pane(ui, session, patient, operator, work);
            return;
        }
        // The acts table has ten columns, most of them buttons: it wants
        // about a thousand pixels. The journal only gets a column of its
        // own once that is satisfied — below that it goes underneath,
        // and the table keeps the full width.
        if work.width() >= 1320.0 {
            let notes_w = (work.width() * 0.28).clamp(260.0, 400.0);
            let acts = egui::Rect::from_min_size(
                work.min,
                egui::vec2(work.width() - notes_w - 8.0, work.height()),
            );
            let notes =
                egui::Rect::from_min_max(egui::pos2(acts.right() + 8.0, work.top()), work.max);
            motif::panel(ui, acts, Some(tr("itv_section")), |ui| {
                Self::patient_acts_pane(ui, session, patient, config);
            });
            motif::panel(ui, notes, Some(tr("notes_section")), |ui| {
                Self::patient_notes_pane(ui, session, patient, operator);
            });
        } else {
            // Narrow: the journal goes under the table, still in view.
            let notes_h = (work.height() * 0.34).clamp(170.0, 280.0);
            let stack = motif::split_rows(work, &[0.0, notes_h], 8.0);
            motif::panel(ui, stack[0], Some(tr("itv_section")), |ui| {
                Self::patient_acts_pane(ui, session, patient, config);
            });
            motif::panel(ui, stack[1], Some(tr("notes_section")), |ui| {
                Self::patient_notes_pane(ui, session, patient, operator);
            });
        }
    }

    /// The ordonnance box: what a positive TROD allows, chosen line by
    /// line and printed.
    ///
    /// Every posology arrives pre-filled from the molecule and stays a
    /// plain text field — the protocol proposes, the pharmacist writes.
    fn ordonnance_box(
        ctx: &egui::Context,
        session: &mut Session,
        patient: &Patient,
        config: &Config,
    ) {
        let Some(open) = &session.ordonnance else {
            return;
        };
        let kind = open.kind;
        let interview = open.interview;
        // The act may have been deleted, or its result corrected on
        // another post, since the box was opened.
        let still_positive = session
            .viewing_interviews
            .iter()
            .any(|i| i.id == interview && i.trod_result == crate::ordonnance::POSITIF);
        let Some(protocol) = crate::ordonnance::protocol(kind) else {
            session.ordonnance = None;
            return;
        };
        if !still_positive {
            session.ordonnance = None;
            return;
        }
        // Read once per frame, before the window borrows the session:
        // the base is small and this is a modal the operator opened on
        // purpose, so a query per frame is cheaper than a cache that
        // could go stale against another post's edit.
        let adjuvant_tag = config.ordonnance.adjuvant_tag.clone();
        let adjuvants: Vec<(Drug, Vec<db::Posologie>)> = session
            .db
            .drugs_with_tag(&adjuvant_tag)
            .unwrap_or_default()
            .into_iter()
            .map(|d| {
                let posologies = session.db.posologies(d.id).unwrap_or_default();
                (d, posologies)
            })
            .collect();
        let mut close = false;
        let mut print = false;
        egui::Window::new(trf("ord_title", protocol.indication))
            .collapsible(false)
            .resizable(true)
            .default_size([680.0, 640.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let Some(open) = &mut session.ordonnance else {
                    return;
                };
                let choice = &mut open.choice;
                // The body takes whatever the window leaves after the
                // warning and the two buttons: a fixed height scrolled
                // the toggles out of sight on the default window.
                let body_h = (ui.available_height() - 84.0).max(160.0);
                egui::ScrollArea::vertical()
                    .id_salt("ord_body")
                    .max_height(body_h)
                    .show(ui, |ui| {
                        motif::section(ui, tr("ord_atb_section"));
                        ui.add_space(4.0);
                        for (i, atb) in protocol.antibiotics.iter().enumerate() {
                            let picked = choice.antibiotic == Some(i);
                            let row = ui.horizontal_wrapped(|ui| {
                                if motif::toggle(ui, atb.name, picked).clicked() {
                                    if picked {
                                        choice.antibiotic = None;
                                    } else {
                                        choice.antibiotic = Some(i);
                                        // Pre-fill the usual posology;
                                        // it stays editable.
                                        choice.posology = atb
                                            .posologies
                                            .first()
                                            .map(|p| (*p).to_owned())
                                            .unwrap_or_default();
                                    }
                                }
                                ui.label(
                                    egui::RichText::new(atb.situation)
                                        .size(11.0)
                                        .color(motif::TEXT_DIM),
                                );
                            });
                            let _ = row;
                            if picked {
                                // The alternatives, when the molecule
                                // has more than one usual schema.
                                if atb.posologies.len() > 1 {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add_space(12.0);
                                        for p in atb.posologies {
                                            if motif::toggle(ui, p, choice.posology == *p).clicked()
                                            {
                                                choice.posology = (*p).to_owned();
                                            }
                                        }
                                    });
                                }
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(tr("ord_posology"))
                                            .size(11.0)
                                            .color(motif::TEXT_DIM),
                                    );
                                    ui.add_sized(
                                        [ui.available_width().max(120.0), 24.0],
                                        egui::TextEdit::singleline(&mut choice.posology)
                                            .hint_text(tr("ord_posology_hint")),
                                    );
                                });
                                if !atb.caution.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add_space(12.0);
                                        ui.label(
                                            egui::RichText::new(atb.caution)
                                                .size(11.0)
                                                .italics()
                                                .color(motif::ALERT),
                                        );
                                    });
                                }
                            }
                            ui.add_space(3.0);
                        }

                        ui.add_space(8.0);
                        motif::section(ui, tr("ord_probio_section"));
                        ui.add_space(4.0);
                        // The adjuvants are drug cards carrying the
                        // configured tag: adding one to the list means
                        // adding a card to the base, not editing the app.
                        if adjuvants.is_empty() {
                            ui.label(
                                egui::RichText::new(trf("ord_probio_empty", &adjuvant_tag))
                                    .size(11.0)
                                    .color(motif::TEXT_DIM),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            if motif::toggle(ui, tr("ord_probio_none"), choice.adjuvant.is_none())
                                .clicked()
                            {
                                choice.adjuvant = None;
                                choice.adjuvant_posology.clear();
                            }
                            for (drug, posologies) in &adjuvants {
                                let picked = choice.adjuvant.as_deref() == Some(drug.name.as_str());
                                let resp = motif::toggle(ui, &drug.name, picked);
                                let resp = if drug.dci.trim().is_empty() {
                                    resp
                                } else {
                                    resp.on_hover_text(&drug.dci)
                                };
                                if resp.clicked() {
                                    choice.adjuvant = Some(drug.name.clone());
                                    // The team's own first posology line
                                    // for that card, when it wrote one.
                                    choice.adjuvant_posology = posologies
                                        .first()
                                        .map(|p| p.posologie.clone())
                                        .unwrap_or_default();
                                }
                            }
                        });
                        if let Some((_, posologies)) = adjuvants
                            .iter()
                            .find(|(d, _)| choice.adjuvant.as_deref() == Some(d.name.as_str()))
                        {
                            if posologies.len() > 1 {
                                ui.horizontal_wrapped(|ui| {
                                    ui.add_space(12.0);
                                    for p in posologies {
                                        let on = choice.adjuvant_posology == p.posologie;
                                        let label = if p.indication.trim().is_empty() {
                                            p.posologie.clone()
                                        } else {
                                            format!("{} — {}", p.indication, p.posologie)
                                        };
                                        if motif::toggle(ui, &label, on).clicked() {
                                            choice.adjuvant_posology = p.posologie.clone();
                                        }
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.add_sized(
                                    [ui.available_width().max(120.0), 24.0],
                                    egui::TextEdit::singleline(&mut choice.adjuvant_posology)
                                        .hint_text(tr("ord_posology_hint")),
                                );
                            });
                        }

                        ui.add_space(8.0);
                        motif::section(ui, tr("ord_advice_section"));
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if motif::toggle(ui, tr("ord_conseils"), choice.conseils).clicked() {
                                choice.conseils = !choice.conseils;
                            }
                            if motif::toggle(ui, tr("ord_temps"), choice.temps_de_prise).clicked() {
                                choice.temps_de_prise = !choice.temps_de_prise;
                            }
                        });

                        ui.add_space(8.0);
                        motif::section(ui, tr("ord_extra_section"));
                        ui.add_space(4.0);
                        ui.add_sized(
                            [ui.available_width(), 60.0],
                            egui::TextEdit::multiline(&mut choice.extra)
                                .hint_text(tr("ord_extra_hint")),
                        );
                    });

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(tr("ord_warning"))
                        .size(10.5)
                        .italics()
                        .color(motif::TEXT_FAINT),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if motif::button(ui, tr("ord_print")).clicked() {
                        print = true;
                    }
                    if motif::button(ui, tr("ord_close")).clicked() {
                        close = true;
                    }
                });
            });

        // Escape is handled by `patient_view`, which owns the key for
        // the whole file and dismisses this box before the file itself.
        if print {
            let choice = session
                .ordonnance
                .as_ref()
                .map(|o| o.choice.clone())
                .unwrap_or_default();
            let lines = choice.lines(protocol);
            if lines.is_empty() {
                session.error = Some(tr("ord_empty").to_owned());
            } else {
                let today = session
                    .db
                    .today_french()
                    .unwrap_or_else(|_| tr("itv_date_fallback").to_owned());
                let advice = choice.advice(protocol);
                match crate::pdf::open_ordonnance(
                    patient,
                    &config.pharmacy,
                    protocol.indication,
                    &today,
                    &lines,
                    &advice,
                    &config.ordonnance_template_path(),
                ) {
                    Ok(_) => {
                        session.error = None;
                        close = true;
                    }
                    Err(e) => session.error = Some(e),
                }
            }
        }
        if close {
            session.ordonnance = None;
        }
    }

    /// A date typed into the carnet, read the way the counter types it.
    ///
    /// The `Future` hint is the right one for the common case — a dose
    /// given today or this year, typed `2308` or `230826`. But a dose
    /// can never be *in* the future, so a reading that lands there is
    /// retried as a past date ("230850" is 1950, not 2050) and only
    /// kept when the past reading makes no sense either.
    fn parse_carnet_date(text: &str, year: u32, today: &str) -> Result<String, String> {
        let iso = db::parse_french_date(text, year, db::YearHint::Future)?;
        if iso.as_str() <= today {
            return Ok(iso);
        }
        Ok(db::parse_french_date(text, year, db::YearHint::Past).unwrap_or(iso))
    }

    /// Widths of the boxes on the "new dose" line. They are named
    /// because the band's height is measured from them: a field and its
    /// measurement drifting apart is how a row gets clipped.
    const FORM_PICK_W: f32 = 232.0;
    const FORM_LABEL_W: f32 = 160.0;
    const FORM_DOSE_W: f32 = 116.0;
    const FORM_DATE_W: f32 = 96.0;
    const FORM_LOT_W: f32 = 90.0;
    const FORM_SITE_W: f32 = 96.0;

    /// The patient file's second half: the carnet de vaccination, what
    /// the calendar still owes, and the destinations on the file.
    fn patient_vaccins_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        operator: &str,
        work: egui::Rect,
    ) {
        // The carnet is a seven-column table; the two reading panels
        // only get a column of their own once it is satisfied.
        let wide = work.width() >= 1240.0;
        let (carnet, side) = if wide {
            let side_w = (work.width() * 0.32).clamp(300.0, 440.0);
            (
                egui::Rect::from_min_max(
                    work.min,
                    egui::pos2(work.right() - side_w - 8.0, work.bottom()),
                ),
                egui::Rect::from_min_max(egui::pos2(work.right() - side_w, work.top()), work.max),
            )
        } else {
            // The carnet keeps a floor: at 1024x700 with both docks out
            // there is barely 300 px here, and a band taking its share
            // of that left the table showing its header and nothing
            // else. The band scrolls inside whatever is left instead.
            let band = (work.height() * 0.40)
                .clamp(150.0, 320.0)
                .min((work.height() - 200.0).max(110.0));
            let stack = motif::split_rows(work, &[0.0, band], 8.0);
            (stack[0], stack[1])
        };
        let (due, travel) = if wide {
            let rows = motif::split_rows(side, &[0.0, 0.0], 8.0);
            (rows[0], rows[1])
        } else {
            let cols = motif::split_columns(side, 2, 8.0);
            (cols[0], cols[1])
        };
        Self::carnet_pane(ui, session, patient, operator, carnet);
        Self::vacc_due_pane(ui, session, patient, due);
        Self::vacc_travel_pane(ui, session, patient, travel);
    }

    /// The carnet itself: one line per dose, correctable in place, with
    /// the line being written kept out of the scroll so it is always
    /// under the hand.
    fn carnet_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        operator: &str,
        rect: egui::Rect,
    ) {
        let lines = session.vaccinations.clone();
        let editing = session.vacc_edit.as_ref().map(|v| v.id);
        let confirm = session.vacc_confirm;
        let today = session.today.clone();
        let year = session.db.current_year();
        let mut start_edit: Option<db::Vaccination> = None;
        let mut save_edit = false;
        let mut cancel_edit = false;
        let mut delete: Option<(i64, String)> = None;
        let mut add = false;
        let mut print = false;

        motif::panel(ui, rect, Some(tr("vacc_section")), |ui| {
            let inner = ui.max_rect();
            // The line being written wraps onto two rows on a narrow
            // file. Its fields are fixed-width boxes, not buttons, so
            // the band is measured from those widths — measuring the
            // hint text instead read one row where two are drawn, and
            // clipped the source note away.
            let free_label = if session.vacc_new_pick == 0 {
                Self::FORM_LABEL_W
            } else {
                0.0
            };
            let widths = [
                Self::FORM_PICK_W,
                free_label,
                Self::FORM_DOSE_W,
                Self::FORM_DATE_W,
                Self::FORM_LOT_W,
                Self::FORM_SITE_W,
                Self::button_width(ui, tr("vacc_add")),
                Self::button_width(ui, tr("vacc_print")),
            ];
            let form_rows = Self::wrapped_rows_of(
                ui,
                inner.width() - 16.0,
                widths.into_iter().filter(|w| *w > 0.0),
            );
            // Plus the source line under it, which is part of the claim
            // the panel makes and must not be clipped away.
            let form_h = ((ui.spacing().interact_size.y + ui.spacing().item_spacing.y) * form_rows
                + 34.0)
                .min(inner.height() * 0.42);
            let parts = motif::split_rows(inner, &[0.0, form_h], 6.0);
            let table = motif::well(ui, parts[0]);
            motif::inside(ui, table, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("carnet_rows")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if lines.is_empty() {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(tr("vacc_empty"))
                                    .size(11.5)
                                    .color(motif::TEXT_DIM),
                            );
                            return;
                        }
                        let dim =
                            |t: &str| egui::RichText::new(t).size(11.0).color(motif::TEXT_DIM);
                        // The text columns share what the two buttons
                        // leave: fixed widths pushed « Par » off the
                        // table as soon as a dock was open.
                        let w = (ui.available_width() - 175.0).max(300.0);
                        egui::Grid::new("carnet_grid")
                            .num_columns(7)
                            .spacing([6.0, 5.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(dim(tr("vacc_col_vaccine")));
                                ui.label(dim(tr("vacc_col_dose")));
                                ui.label(dim(tr("vacc_col_date")));
                                ui.label(dim(tr("vacc_col_lot")));
                                ui.label(dim(tr("vacc_col_site")));
                                ui.label(dim(tr("vacc_col_operator")));
                                ui.label("");
                                ui.end_row();
                                for line in &lines {
                                    if editing == Some(line.id) {
                                        let e = session.vacc_edit.as_mut().unwrap();
                                        ui.add_sized(
                                            [w * 0.32, 22.0],
                                            egui::TextEdit::singleline(&mut e.label),
                                        );
                                        ui.add_sized(
                                            [w * 0.14, 22.0],
                                            egui::TextEdit::singleline(&mut e.dose),
                                        );
                                        ui.add_sized(
                                            [w * 0.14, 22.0],
                                            egui::TextEdit::singleline(&mut session.vacc_edit_date),
                                        );
                                        ui.add_sized(
                                            [w * 0.14, 22.0],
                                            egui::TextEdit::singleline(&mut e.lot),
                                        );
                                        ui.add_sized(
                                            [w * 0.16, 22.0],
                                            egui::TextEdit::singleline(&mut e.site),
                                        );
                                        ui.add_sized(
                                            [w * 0.10, 22.0],
                                            egui::TextEdit::singleline(&mut e.operator),
                                        );
                                        ui.horizontal(|ui| {
                                            if motif::button(ui, tr("form_save")).clicked() {
                                                save_edit = true;
                                            }
                                            if motif::button(ui, tr("form_cancel")).clicked() {
                                                cancel_edit = true;
                                            }
                                        });
                                        ui.end_row();
                                        continue;
                                    }
                                    ui.label(egui::RichText::new(&line.label).size(12.0));
                                    ui.label(egui::RichText::new(&line.dose).size(12.0));
                                    ui.label(
                                        egui::RichText::new(if line.given_on.is_empty() {
                                            tr("vacc_no_date").to_owned()
                                        } else {
                                            db::format_french_date(&line.given_on)
                                        })
                                        .size(12.0),
                                    );
                                    ui.label(egui::RichText::new(&line.lot).size(11.5));
                                    ui.label(egui::RichText::new(&line.site).size(11.5));
                                    ui.label(
                                        egui::RichText::new(&line.operator)
                                            .size(11.5)
                                            .color(operator_color(&line.operator)),
                                    );
                                    ui.horizontal(|ui| {
                                        if motif::button(ui, tr("drug_edit")).clicked() {
                                            start_edit = Some(line.clone());
                                        }
                                        let label = if confirm == Some(line.id) {
                                            tr("itv_delete_confirm")
                                        } else {
                                            tr("itv_delete")
                                        };
                                        if motif::button(ui, label).clicked() {
                                            if confirm == Some(line.id) {
                                                delete = Some((line.id, line.label.clone()));
                                            } else {
                                                session.vacc_confirm = Some(line.id);
                                            }
                                        }
                                    });
                                    ui.end_row();
                                    // A next dose owed, or a remark,
                                    // shows under the line rather than
                                    // in columns of their own: both are
                                    // filled on one line in ten.
                                    if !line.next_due.is_empty() || !line.remark.is_empty() {
                                        let mut foot = String::new();
                                        if !line.next_due.is_empty() {
                                            foot.push_str(&trf(
                                                "vacc_next_prefix",
                                                db::format_french_date(&line.next_due),
                                            ));
                                        }
                                        if !line.remark.is_empty() {
                                            if !foot.is_empty() {
                                                foot.push_str(" — ");
                                            }
                                            foot.push_str(&line.remark);
                                        }
                                        ui.label("");
                                        ui.label(
                                            egui::RichText::new(foot)
                                                .size(10.5)
                                                .italics()
                                                .color(motif::TEXT_FAINT),
                                        );
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });
            // --- The line being written ---
            motif::inside(ui, parts[1], |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("carnet_form")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let picked = session.vacc_new_pick;
                            let shown = if picked == 0 {
                                tr("vacc_free_label").to_owned()
                            } else {
                                vaccines::CATALOGUE[picked - 1].label.to_owned()
                            };
                            egui::ComboBox::from_id_salt("vacc_pick")
                                .selected_text(egui::RichText::new(shown).size(11.5))
                                .width(Self::FORM_PICK_W - 16.0)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(picked == 0, tr("vacc_free_label"))
                                        .clicked()
                                    {
                                        session.vacc_new_pick = 0;
                                    }
                                    for (i, v) in vaccines::CATALOGUE.iter().enumerate() {
                                        if ui
                                            .selectable_label(picked == i + 1, v.label)
                                            .on_hover_text(v.schedule)
                                            .clicked()
                                        {
                                            session.vacc_new_pick = i + 1;
                                        }
                                    }
                                });
                            if session.vacc_new_pick == 0 {
                                ui.add_sized(
                                    [Self::FORM_LABEL_W, 22.0],
                                    egui::TextEdit::singleline(&mut session.vacc_new.label)
                                        .hint_text(tr("vacc_label_hint")),
                                );
                            }
                            ui.add_sized(
                                [Self::FORM_DOSE_W, 22.0],
                                egui::TextEdit::singleline(&mut session.vacc_new.dose)
                                    .hint_text(tr("vacc_dose_hint")),
                            );
                            ui.add_sized(
                                [Self::FORM_DATE_W, 22.0],
                                egui::TextEdit::singleline(&mut session.vacc_new_date)
                                    .hint_text(tr("vacc_date_hint")),
                            );
                            ui.add_sized(
                                [Self::FORM_LOT_W, 22.0],
                                egui::TextEdit::singleline(&mut session.vacc_new.lot)
                                    .hint_text(tr("vacc_lot_hint")),
                            );
                            ui.add_sized(
                                [Self::FORM_SITE_W, 22.0],
                                egui::TextEdit::singleline(&mut session.vacc_new.site)
                                    .hint_text(tr("vacc_site_hint")),
                            );
                            if motif::button(ui, tr("vacc_add")).clicked() {
                                add = true;
                            }
                            if motif::button(ui, tr("vacc_print"))
                                .on_hover_text(tr("vacc_print_tooltip"))
                                .clicked()
                            {
                                print = true;
                            }
                        });
                        ui.label(
                            egui::RichText::new(tr("vacc_source"))
                                .size(10.0)
                                .italics()
                                .color(motif::TEXT_FAINT),
                        );
                    });
            });
        });

        // --- What the buttons asked for -------------------------------
        if let Some(line) = start_edit {
            session.vacc_edit_base = (line.label.clone(), line.given_on.clone());
            session.vacc_edit_date = if line.given_on.is_empty() {
                String::new()
            } else {
                db::format_french_date(&line.given_on)
            };
            session.vacc_edit = Some(line);
            session.vacc_confirm = None;
        }
        if cancel_edit {
            session.vacc_edit = None;
            session.vacc_edit_date.clear();
        }
        if save_edit {
            if let Some(mut line) = session.vacc_edit.clone() {
                let parsed = if session.vacc_edit_date.trim().is_empty() {
                    Ok(String::new())
                } else {
                    Self::parse_carnet_date(&session.vacc_edit_date, year, &today)
                };
                match parsed {
                    Ok(iso) => {
                        line.given_on = iso;
                        let (label, date) = session.vacc_edit_base.clone();
                        match session.db.update_vaccination(line.id, &line, &label, &date) {
                            Ok(true) => {
                                session.error = None;
                                session.load_carnet(patient.id);
                            }
                            Ok(false) => {
                                session.error = Some(tr("vacc_stale").to_owned());
                                session.load_carnet(patient.id);
                            }
                            Err(e) => session.error = Some(e),
                        }
                    }
                    Err(e) => session.error = Some(e),
                }
            }
        }
        if let Some((id, label)) = delete {
            match session.db.delete_vaccination(id, &label) {
                Ok(true) => {
                    session.error = None;
                    session.load_carnet(patient.id);
                }
                Ok(false) => {
                    session.error = Some(tr("vacc_stale").to_owned());
                    session.load_carnet(patient.id);
                }
                Err(e) => session.error = Some(e),
            }
        }
        if add {
            let mut line = session.vacc_new.clone();
            if session.vacc_new_pick > 0 {
                let v = &vaccines::CATALOGUE[session.vacc_new_pick - 1];
                line.code = v.code.to_owned();
                line.label = v.label.to_owned();
            } else {
                line.code.clear();
                line.label = line.label.trim().to_owned();
            }
            // An empty date means "today": recording the dose you have
            // just given is the common case, and typing the date again
            // is the step an operator skips.
            let parsed = if session.vacc_new_date.trim().is_empty() {
                Ok(today.clone())
            } else {
                Self::parse_carnet_date(&session.vacc_new_date, year, &today)
            };
            if line.label.is_empty() {
                session.error = Some(tr("vacc_needs_label").to_owned());
            } else {
                match parsed {
                    Ok(iso) => {
                        line.given_on = iso;
                        if line.operator.trim().is_empty() {
                            line.operator = operator.to_owned();
                        }
                        match session.db.add_vaccination(patient.id, &line) {
                            Ok(_) => {
                                session.error = None;
                                session.vacc_new = db::Vaccination::default();
                                session.vacc_new_date.clear();
                                session.vacc_new_pick = 0;
                                session.load_carnet(patient.id);
                            }
                            Err(e) => session.error = Some(e),
                        }
                    }
                    Err(e) => session.error = Some(e),
                }
            }
        }
        if print {
            let lines = session.vaccinations.clone();
            if let Err(e) = crate::pdf::open_vaccination_carnet(patient, &lines) {
                session.error = Some(e);
            }
        }
    }

    /// What the calendrier vaccinal still owes this patient, read
    /// against the doses in the carnet.
    fn vacc_due_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        rect: egui::Rect,
    ) {
        let age = db::age_on(&patient.birth_date, &session.today);
        let birth_year = patient.birth_date.get(..4).and_then(|y| y.parse().ok());
        let doses: Vec<vaccines::Dose> = session
            .vaccinations
            .iter()
            .map(|v| vaccines::Dose {
                code: v.code.as_str(),
                date: v.given_on.as_str(),
            })
            .collect();
        let lines = vaccines::due_lines(age, birth_year, &session.today, &doses);
        // Clicking a line owed loads that vaccine into the form at the
        // foot of the carnet: the panel says what to do, and the click
        // is the doing.
        let mut pick: Option<&'static str> = None;
        motif::panel(ui, rect, Some(tr("vacc_due_section")), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("vacc_due")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if lines.is_empty() {
                        ui.label(
                            egui::RichText::new(tr("vacc_due_empty"))
                                .size(11.5)
                                .color(motif::TEXT_DIM),
                        );
                        return;
                    }
                    for line in &lines {
                        let (tag, color) = match line.level {
                            vaccines::DueLevel::Ok => (tr("vacc_due_ok"), motif::TEXT_FAINT),
                            vaccines::DueLevel::Due => (tr("vacc_due_todo"), motif::ALERT),
                            vaccines::DueLevel::Ask => (
                                tr("vacc_due_ask"),
                                egui::Color32::from_rgb(0x7a, 0x5c, 0x1f),
                            ),
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("  {tag}  "))
                                    .size(10.0)
                                    .strong()
                                    .color(egui::Color32::WHITE)
                                    .background_color(color),
                            );
                            if ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(line.label).strong().size(12.0),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text(tr("vacc_due_click"))
                                .clicked()
                            {
                                pick = Some(line.code);
                            }
                        });
                        ui.label(
                            egui::RichText::new(&line.detail)
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        ui.add_space(5.0);
                    }
                });
        });
        if let Some(code) = pick {
            session.vacc_new_pick = vaccines::CATALOGUE
                .iter()
                .position(|v| v.code == code)
                .map(|i| i + 1)
                .unwrap_or(0);
        }
    }

    /// Destinations on the file, and what each one adds to the carnet.
    ///
    /// A recommendation is ticked off against the doses already
    /// recorded: what stays « manquant » is the conversation to have.
    fn vacc_travel_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        rect: egui::Rect,
    ) {
        let travels = session.travels.clone();
        let held: std::collections::HashSet<String> = session
            .vaccinations
            .iter()
            .filter(|v| !v.code.is_empty())
            .map(|v| v.code.clone())
            .collect();
        let mut remove: Option<String> = None;
        let mut add: Option<&'static str> = None;
        motif::panel(ui, rect, Some(tr("vacc_travel_section")), |ui| {
            ui.add_sized(
                [ui.available_width().min(260.0), 22.0],
                egui::TextEdit::singleline(&mut session.travel_query)
                    .hint_text(tr("vacc_travel_add_hint")),
            );
            let query = session.travel_query.clone();
            if !query.trim().is_empty() {
                for country in vaccines::search(&query).into_iter().take(6) {
                    if motif::list_row(ui, egui::RichText::new(country.name).size(12.0), false)
                        .clicked()
                    {
                        add = Some(country.code);
                    }
                }
                ui.add_space(4.0);
            }
            egui::ScrollArea::vertical()
                .id_salt("vacc_travel")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if travels.is_empty() {
                        ui.label(
                            egui::RichText::new(tr("vacc_travel_empty"))
                                .size(11.5)
                                .color(motif::TEXT_DIM),
                        );
                        return;
                    }
                    for travel in &travels {
                        let Some(country) = vaccines::country(&travel.country) else {
                            continue;
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(country.name).strong().size(12.0));
                            if motif::button(ui, tr("vacc_travel_remove")).clicked() {
                                remove = Some(travel.country.clone());
                            }
                        });
                        if !travel.depart_on.is_empty() {
                            ui.label(
                                egui::RichText::new(trf(
                                    "vacc_travel_depart",
                                    db::format_french_date(&travel.depart_on),
                                ))
                                .size(10.5)
                                .color(motif::TEXT_FAINT),
                            );
                        }
                        let row = |ui: &mut egui::Ui, label: &str, done: bool| {
                            let (tag, color) = if done {
                                (tr("vacc_travel_done"), motif::TEXT_FAINT)
                            } else {
                                (tr("vacc_travel_missing"), motif::ALERT)
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("  {tag}  "))
                                        .size(10.0)
                                        .strong()
                                        .color(egui::Color32::WHITE)
                                        .background_color(color),
                                );
                                ui.label(egui::RichText::new(label).size(11.5));
                            });
                        };
                        // Yellow fever first: it is the one that has to
                        // be done in an approved centre, and the one a
                        // border can turn a traveller back for.
                        if country.yf.needed() {
                            row(ui, tr("map_reco_yf"), held.contains("FJ"));
                        }
                        for reco in country.recos() {
                            row(ui, reco.label, held.contains(reco.code));
                        }
                        ui.add_space(6.0);
                    }
                });
        });
        if let Some(code) = add {
            session.travel_query.clear();
            match session.db.add_travel(patient.id, code, "") {
                Ok(()) => session.load_carnet(patient.id),
                Err(e) => session.error = Some(e),
            }
        }
        if let Some(code) = remove {
            match session.db.remove_travel(patient.id, &code) {
                Ok(_) => session.load_carnet(patient.id),
                Err(e) => session.error = Some(e),
            }
        }
    }

    /// « Modifier » and « Supprimer… », with the two-step confirmation.
    fn patient_actions(
        ui: &mut egui::Ui,
        session: &Session,
        start_edit: &mut bool,
        delete_click: &mut bool,
    ) {
        let del_label = if session.confirm_delete {
            tr("patient_delete_confirm")
        } else {
            tr("patient_delete")
        };
        if motif::button(ui, del_label).clicked() {
            *delete_click = true;
        }
        if session.edit_patient.is_none() && motif::button(ui, tr("patient_edit")).clicked() {
            *start_edit = true;
        }
    }

    /// The width a Motif button with this label occupies.
    fn button_width(ui: &egui::Ui, label: &str) -> f32 {
        let font = egui::TextStyle::Button.resolve(ui.style());
        ui.fonts(|f| {
            f.layout_no_wrap(label.to_owned(), font.clone(), motif::TEXT)
                .size()
                .x
        }) + ui.spacing().button_padding.x * 2.0
            + 8.0
    }

    /// How many lines a wrapped row of items of these widths takes at
    /// `width`. Bands are carved rectangles, so their height has to be
    /// known before the content is drawn — measured, not guessed.
    fn wrapped_rows_of(ui: &egui::Ui, width: f32, widths: impl Iterator<Item = f32>) -> f32 {
        let gap = ui.spacing().item_spacing.x;
        let mut x = 0.0_f32;
        let mut lines = 1.0_f32;
        for w in widths {
            if x + w > width && x > 0.0 {
                lines += 1.0;
                x = 0.0;
            }
            x += w + gap;
        }
        lines
    }

    /// [`wrapped_rows_of`] for a row that is all buttons.
    fn wrapped_rows<'a>(ui: &egui::Ui, width: f32, labels: impl Iterator<Item = &'a str>) -> f32 {
        Self::wrapped_rows_of(ui, width, labels.map(|l| Self::button_width(ui, l)))
    }

    /// How tall the identity band needs to be: a header and one or two
    /// wrapped button rows normally, much more with the correction form
    /// open. Measured rather than guessed, so nothing is ever clipped.
    fn patient_band_height(ui: &egui::Ui, session: &Session, patient: &Patient) -> f32 {
        let w = motif::visible_rect(ui).width() - 40.0;
        // The act buttons are the part that wraps.
        let lines = Self::wrapped_rows(ui, w, InterviewKind::ALL.iter().map(|k| k.label()));
        let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y + 8.0;
        // Header (name, birth, contact, address, comment) + treatments +
        // "nouvel entretien" + the wrapped act rows + the eligibility note.
        let mut h = 96.0 + row * (2.0 + lines);
        if !patient.address.is_empty() {
            h += 18.0;
        }
        if !patient.notes.is_empty() {
            h += 20.0;
        }
        if session.edit_patient.is_some() {
            h += 9.0 * 34.0 + 40.0;
        }
        if session.rule_block.is_some() {
            h += 46.0;
        }
        if session.patient_treats.len() < db::BPM_MIN_TREATMENTS {
            h += 34.0;
        }
        if !session.treat_query.trim().is_empty() {
            h += row;
        }
        // Whatever the band would like, the acts and the journal keep
        // their half of the file: the band scrolls instead.
        let avail = motif::visible_rect(ui).height();
        h.min((avail * 0.45).max(avail - 340.0))
    }

    /// Who the patient is: identity, corrections, treatments, and the
    /// buttons that start an act.
    fn patient_identity_pane(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        patient: &Patient,
        config: &Config,
        operator: &str,
    ) {
        let _ = operator;
        let mut start_edit = false;
        let mut save_edit = false;
        let mut cancel_edit = false;
        let mut delete_click = false;
        let mut back = false;
        let cramped = motif::visible_rect(ui).width() < 620.0;
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if motif::button(ui, tr("patient_back")).clicked() {
                session.flush_date_edits();
                session.viewing = None;
                back = true;
            }
            ui.add_space(6.0);
            ui.heading(patient.full_name());
            ui.label(
                egui::RichText::new(trf(
                    "patient_born",
                    db::format_french_date(&patient.birth_date),
                ))
                .color(motif::TEXT_DIM),
            );
            // The file's own actions live on its header line, hard
            // right — they act on the patient, not on the acts below.
            // Unless the name already fills the line, in which case they
            // drop underneath rather than printing over it.
            if !cramped {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    Self::patient_actions(ui, session, &mut start_edit, &mut delete_click);
                });
            }
        });
        if cramped {
            ui.horizontal(|ui| {
                Self::patient_actions(ui, session, &mut start_edit, &mut delete_click);
            });
        }
        {
            // Everything else about the patient on one quiet line under
            // the name: contact, situation, address, comment. Wrapped,
            // so a long address pushes a line instead of being cut.
            let mut bits: Vec<String> = Vec::new();
            if !patient.phone.is_empty() {
                bits.push(trf("patient_phone", &patient.phone));
            }
            if !patient.physician.is_empty() {
                bits.push(trf("patient_physician", &patient.physician));
            }
            if !patient.email.is_empty() {
                bits.push(patient.email.clone());
            }
            // The memo requires the situation to be carried onto the
            // billing, so it belongs on the header.
            if let Some(key) = db::situation_label(&patient.situation) {
                if !patient.situation.is_empty() {
                    bits.push(trf("patient_situation", tr(key)));
                }
            }
            if !patient.address.is_empty() {
                bits.push(patient.address.clone());
            }
            if !patient.notes.is_empty() {
                bits.push(patient.notes.clone());
            }
            if !bits.is_empty() {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(bits.join("   ·   "))
                            .size(12.0)
                            .color(motif::TEXT_DIM),
                    )
                    .wrap(),
                );
            }
        }
        if back {
            return;
        }
        if session.confirm_delete {
            ui.colored_label(motif::ALERT, tr("patient_delete_warning"));
        }
        if let Some(form) = &mut session.edit_patient {
            ui.add_space(8.0);
            let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
            egui::Grid::new("edit_patient")
                .min_col_width(110.0)
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
                    ui.label(dim(tr("form_physician")));
                    ui.add_sized(
                        [240.0, 26.0],
                        egui::TextEdit::singleline(&mut form.physician)
                            .hint_text(tr("form_physician_hint")),
                    );
                    ui.end_row();
                    ui.label(dim(tr("form_email")));
                    ui.add_sized([240.0, 26.0], egui::TextEdit::singleline(&mut form.email));
                    ui.end_row();
                    ui.label(dim(tr("form_address")));
                    ui.add_sized([240.0, 26.0], egui::TextEdit::singleline(&mut form.address));
                    ui.end_row();
                    // Both are for the bulletin d'adhésion, and both are
                    // optional: left empty, the printed form keeps its
                    // dotted rule for the patient's carte Vitale.
                    ui.label(dim(tr("form_nir")));
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [200.0, 26.0],
                            egui::TextEdit::singleline(&mut form.nir)
                                .hint_text(tr("form_nir_hint")),
                        );
                        ui.label(dim(tr("form_regime")));
                        ui.add_sized(
                            [56.0, 26.0],
                            egui::TextEdit::singleline(&mut form.regime)
                                .hint_text(tr("form_regime_hint")),
                        );
                    });
                    ui.end_row();
                    ui.label(dim(tr("form_situation")));
                    ui.horizontal(|ui| {
                        for (code, key) in db::SITUATIONS {
                            let btn = motif::toggle(ui, tr(key), form.situation == *code);
                            if btn.clicked() {
                                form.situation = (*code).to_owned();
                            }
                        }
                        ui.label(dim(tr("form_situation_hint")));
                    });
                    ui.end_row();
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if motif::button(ui, tr("form_save")).clicked() {
                    save_edit = true;
                }
                if motif::button(ui, tr("form_cancel")).clicked() {
                    cancel_edit = true;
                }
            });
            if let Some(err) = &form.error {
                ui.colored_label(motif::ALERT, err.as_str());
            }
        }
        ui.add_space(16.0);

        // Current treatments, linked to the drug base: chips open the
        // drug card, "×" unlinks, the small picker adds by fuzzy name.
        {
            let mut remove_treat: Option<i64> = None;
            let mut add_treat: Option<i64> = None;
            let mut open_card: Option<Drug> = None;
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr("treat_label")).color(motif::TEXT_DIM));
                for t in &session.patient_treats {
                    let chip = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("  {}  ", t.name))
                                .size(12.0)
                                .color(egui::Color32::WHITE)
                                .background_color(motif::ACCENT),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if chip.on_hover_text(tr("treat_open_tooltip")).clicked() {
                        open_card = Some(t.clone());
                    }
                    let x = ui.add(
                        egui::Label::new(egui::RichText::new("×").size(12.0))
                            .sense(egui::Sense::click()),
                    );
                    if x.on_hover_text(tr("treat_remove_tooltip")).clicked() {
                        remove_treat = Some(t.id);
                    }
                }
                ui.add_sized(
                    [140.0, 20.0],
                    egui::TextEdit::singleline(&mut session.treat_query)
                        .hint_text(tr("treat_add_hint")),
                );
            });
            if !session.treat_query.trim().is_empty() {
                let q = session.treat_query.clone();
                let mut scored: Vec<(i32, &Drug)> = session
                    .drugs
                    .iter()
                    .filter(|d| session.patient_treats.iter().all(|t| t.id != d.id))
                    .filter_map(|d| {
                        let a = fuzzy::score(&q, &d.name);
                        let b = if d.dci.is_empty() {
                            None
                        } else {
                            fuzzy::score(&q, &d.dci)
                        };
                        a.max(b).map(|s| (s, d))
                    })
                    .collect();
                scored.sort_by_key(|&(s, _)| std::cmp::Reverse(s));
                ui.horizontal(|ui| {
                    for (_, d) in scored.into_iter().take(4) {
                        let sug = ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("+ {}", d.name)).size(12.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if sug.clicked() {
                            add_treat = Some(d.id);
                        }
                    }
                });
            }
            if let Some(id) = remove_treat {
                if let Err(e) = session.db.remove_patient_drug(patient.id, id) {
                    session.error = Some(e);
                }
                session.patient_treats =
                    session.db.drugs_for_patient(patient.id).unwrap_or_default();
            }
            if let Some(id) = add_treat {
                if let Err(e) = session.db.add_patient_drug(patient.id, id) {
                    session.error = Some(e);
                }
                session.treat_query.clear();
                session.patient_treats =
                    session.db.drugs_for_patient(patient.id).unwrap_or_default();
            }
            if let Some(d) = open_card {
                session.open_drug_card(d);
                session.view = MainView::Drugs;
            }
        }
        ui.add_space(10.0);

        // Ctrl+N opens the quick picker; the buttons below start an
        // act directly (spec 3.1).
        ui.horizontal(|ui| {
            ui.label(tr("patient_new_interview"));
            if motif::button(ui, tr("act_picker_open"))
                .on_hover_text(tr("act_picker_tooltip"))
                .clicked()
            {
                session.act_picker = true;
            }
        });
        let ctrl_n = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N));
        // (kind, thematic): the direct buttons create a themeless
        // act, the picker attaches the theme chosen with it.
        let mut new_act: Option<(InterviewKind, String)> = None;
        ui.horizontal_wrapped(|ui| {
            for kind in InterviewKind::ALL {
                if motif::button(ui, kind.label()).clicked() {
                    new_act = Some((kind, String::new()));
                }
            }
        });
        if ctrl_n {
            session.act_picker = true;
        }
        if session.act_picker {
            if let Some(kind) = act_picker_window(ctx, session) {
                new_act = Some((kind, std::mem::take(&mut session.act_theme)));
            }
        }
        if let Some((kind, theme)) = new_act {
            // Convention rule: N acts per année d'accompagnement,
            // next cycle at least 12 months after the first act.
            let months = config.rules.cycle_months.max(1);
            // How many acts the year allows: the convention's
            // sequence for an accompaniment theme, the officine's
            // own quota for the rest.
            let dates_so_far = session
                .db
                .interview_dates_for(patient.id, kind)
                .unwrap_or_default();
            let year_now = db::cycle_positions(&dates_so_far, months)
                .last()
                .map(|(y, _)| *y)
                .unwrap_or(0);
            // « Autres anticancéreux » may close a sequence early,
            // so a completed one simply opens the next: no quota to
            // enforce there.
            let per_year = if kind.sequence_may_finish_early() {
                0
            } else {
                config.sequence_len(kind, year_now) as u32
            };
            let blocked = if per_year > 0 {
                let dates = session
                    .db
                    .interview_dates_for(patient.id, kind)
                    .unwrap_or_default();
                let today = session.db.today_iso().unwrap_or_default();
                db::rule_next_allowed(&dates, &today, per_year, months)
            } else {
                None
            };
            match blocked {
                // "Informer" states the rule but never stops the
                // act; "avertir" asks for a confirmation; "refuser"
                // declines, with no override button.
                Some(_) if config.rules.enforcement == RuleEnforcement::Inform => {
                    session.rule_block = None;
                    match session.db.add_interview_themed(patient.id, kind, &theme) {
                        Ok(_) => {
                            session.reload_interviews(patient.id);
                            session.error = Some(trn("rule_informed", &[&kind.label(), &per_year]));
                        }
                        Err(e) => session.error = Some(e),
                    }
                }
                Some(next) => {
                    session.rule_block = Some((
                        kind,
                        theme,
                        trn(
                            "rule_blocked",
                            &[&kind.label(), &per_year, &db::format_french_date(&next)],
                        ),
                    ));
                }
                None => {
                    session.rule_block = None;
                    match session.db.add_interview_themed(patient.id, kind, &theme) {
                        Ok(_) => session.reload_interviews(patient.id),
                        Err(e) => session.error = Some(e),
                    }
                }
            }
        }
        if let Some((kind, theme, msg)) = session.rule_block.clone() {
            ui.add_space(4.0);
            ui.colored_label(motif::ALERT, msg.as_str());
            if config.rules.enforcement == RuleEnforcement::Block {
                ui.label(
                    egui::RichText::new(tr("rule_blocked_hard"))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
            } else if motif::button(ui, tr("rule_override")).clicked() {
                session.rule_block = None;
                match session.db.add_interview_themed(patient.id, kind, &theme) {
                    Ok(_) => session.reload_interviews(patient.id),
                    Err(e) => session.error = Some(e),
                }
            }
        }

        // Eligibility, as the memo states it: the bilan partagé de
        // médication is for the patient on at least five treatments
        // for six months or more. The linked treatments are what the
        // app knows, so this informs rather than blocks.
        if session.patient_treats.len() < db::BPM_MIN_TREATMENTS {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(trn(
                    "rule_bpm_eligibility",
                    &[&db::BPM_MIN_TREATMENTS, &session.patient_treats.len()],
                ))
                .size(11.0)
                .color(motif::TEXT_DIM),
            );
        }

        if start_edit {
            session.confirm_delete = false;
            session.edit_patient = Some(PatientForm {
                last_name: patient.last_name.clone(),
                first_name: patient.first_name.clone(),
                birth_date: db::format_french_date(&patient.birth_date),
                phone: patient.phone.clone(),
                notes: patient.notes.clone(),
                physician: patient.physician.clone(),
                email: patient.email.clone(),
                address: patient.address.clone(),
                situation: patient.situation.clone(),
                nir: patient.nir.clone(),
                regime: patient.regime.clone(),
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
                        let updated = Patient {
                            id: patient.id,
                            last_name: form.last_name.trim().to_owned(),
                            first_name: form.first_name.trim().to_owned(),
                            birth_date: iso,
                            phone: form.phone.trim().to_owned(),
                            notes: form.notes.trim().to_owned(),
                            physician: form.physician.trim().to_owned(),
                            email: form.email.trim().to_owned(),
                            address: form.address.trim().to_owned(),
                            situation: form.situation.trim().to_owned(),
                            nir: form.nir.trim().to_owned(),
                            regime: form.regime.trim().to_owned(),
                        };
                        // CAS against the row as displayed: a colleague's
                        // concurrent correction is never wiped.
                        let applied = session.db.update_patient(&updated, patient)?;
                        Ok((updated, applied))
                    });
                match outcome {
                    Ok((updated, true)) => {
                        session.viewing = Some(updated);
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
                        // `viewing` is now None: patient_view checks it
                        // and skips the acts and journal panes.
                    }
                    Err(e) => session.error = Some(e),
                }
            } else {
                session.confirm_delete = true;
            }
        }
    }

    /// The acts table: every entretien of this file, its state, its
    /// fee rank, its RDV.
    fn patient_acts_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        config: &Config,
    ) {
        let interviews = session.viewing_interviews.clone();
        let mut advance: Option<(i64, db::InterviewState)> = None;
        let mut regress: Option<(i64, db::InterviewState)> = None;
        // (kind, planned date, thematic) of the row whose PDF was asked.
        let mut print_req: Option<(InterviewKind, Option<String>, String)> = None;
        let mut cr_req: Option<(InterviewKind, Option<String>, String)> = None;
        // The act's bulletin d'adhésion, for the themes that have one.
        let mut bulletin_req: Option<InterviewKind> = None;
        // (interview id, what the TROD read, the value this PC saw — CAS).
        let mut set_trod: Option<(i64, String, String)> = None;
        // The act whose ordonnance box is being opened.
        let mut open_ordonnance: Option<(i64, InterviewKind)> = None;
        // (interview id, new minutes, the minutes this PC saw — CAS).
        let mut set_duration: Option<(i64, i64, i64)> = None;
        // (interview id, new date, the date this PC saw — CAS expected).
        let mut set_date: Option<(i64, Option<String>, Option<String>)> = None;
        let mut delete_itv: Option<(i64, db::InterviewState)> = None;
        // (interview id, new theme, the theme this PC saw — CAS).
        let mut set_theme: Option<(i64, String, String)> = None;
        // (interview id, new hour, the hour this PC saw — CAS).
        let mut set_hour: Option<(i64, String, String)> = None;
        let mut set_remote: Option<(i64, bool, bool)> = None;
        let mut set_change: Option<(i64, bool, bool)> = None;
        // Rank of each act inside its yearly cycle, per kind — this is
        // what selects the fee slot (initial / 1er / 2e suivi).
        let ranks = interview_ranks(&interviews, config.rules.cycle_months.max(1));
        // Where each accompaniment stands, above the rows it summarises.
        Self::patient_sequences(ui, &interviews, &ranks, config, session.show_amounts);
        // The table is wide by nature — ten columns, most of them
        // buttons. It scrolls both ways rather than losing its right
        // hand columns silently to whatever width the pane happens to
        // have.
        egui::ScrollArea::both()
            .id_salt("interviews")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("interviews")
                    .num_columns(10)
                    .spacing([8.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        if !interviews.is_empty() {
                            for header in [
                                tr("itv_header_kind"),
                                tr("itv_header_act"),
                                tr("itv_header_theme"),
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
                                        .color(motif::TEXT_DIM),
                                );
                            }
                            ui.end_row();
                        }
                        for itv in &interviews {
                            let (year, rank) = ranks.get(&itv.id).copied().unwrap_or((0, 0));
                            // The theme, and nothing else: the act
                            // code and the two flags have a column
                            // of their own, so the row keeps one
                            // line and the columns stay aligned.
                            ui.label(egui::RichText::new(itv.kind.label()).strong());
                            // The convention's act code, the step it
                            // pays, and the two flags that change
                            // what is billed.
                            ui.horizontal(|ui| {
                                let step_name = itv
                                    .kind
                                    .step_label(year, rank)
                                    .map(|s| s.to_owned())
                                    .unwrap_or_else(|| rank_label(rank));
                                let code = itv.kind.act_code(year).unwrap_or("—");
                                ui.label(
                                    egui::RichText::new(format!("{code} · {}", rank + 1))
                                        .size(11.0)
                                        .strong()
                                        .color(motif::ACCENT),
                                )
                                .on_hover_text(format!(
                                    "{step_name}\n{}",
                                    trn(
                                        "itv_fee_tooltip",
                                        &[
                                            &format!(
                                                "{:.2} €",
                                                config.act_total(itv.kind, year, rank, itv.remote)
                                            ),
                                            &(year + 1),
                                            &itv.kind.coverage_rate(),
                                        ],
                                    )
                                ));
                                // Held remotely: the convention bills
                                // TPH on top of the act code.
                                if itv.kind.is_accompaniment()
                                    && motif::toggle(ui, db::REMOTE_CODE, itv.remote)
                                        .on_hover_text(trf(
                                            "itv_remote_tooltip",
                                            format!("{:.2} €", config.billing.teleconsultation),
                                        ))
                                        .clicked()
                                {
                                    set_remote = Some((itv.id, !itv.remote, itv.remote));
                                }
                                // Anticancéreux only: the memo keeps
                                // the treatment-change derogation
                                // for those two themes alone.
                                if itv.kind.allows_treatment_change() {
                                    if motif::toggle(
                                        ui,
                                        tr("itv_change_short_label"),
                                        itv.treatment_change,
                                    )
                                    .on_hover_text(tr("itv_change_tooltip"))
                                    .clicked()
                                    {
                                        set_change = Some((
                                            itv.id,
                                            !itv.treatment_change,
                                            itv.treatment_change,
                                        ));
                                    }
                                    // The derogation has conditions;
                                    // say which one is not met yet
                                    // rather than billing blind.
                                    if let Some((before, after)) = treatment_change_shortfall(
                                        &interviews,
                                        itv,
                                        config.rules.cycle_months.max(1),
                                    ) {
                                        ui.label(
                                            egui::RichText::new("!").strong().color(motif::ALERT),
                                        )
                                        .on_hover_text(trn("itv_change_short", &[&before, &after]));
                                    }
                                }
                            });
                            let mut theme = itv.theme.clone();
                            if theme_combo(ui, &format!("theme{}", itv.id), &mut theme) {
                                set_theme = Some((itv.id, theme, itv.theme.clone()));
                            }
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
                            ui.horizontal(|ui| {
                                if motif::button(ui, tr("itv_pdf"))
                                    .on_hover_text(tr("itv_pdf_tooltip"))
                                    .clicked()
                                {
                                    print_req = Some((
                                        itv.kind,
                                        itv.scheduled_date.clone(),
                                        itv.theme.clone(),
                                    ));
                                }
                                if motif::button(ui, tr("itv_cr"))
                                    .on_hover_text(tr("itv_cr_tooltip"))
                                    .clicked()
                                {
                                    cr_req = Some((
                                        itv.kind,
                                        itv.scheduled_date.clone(),
                                        itv.theme.clone(),
                                    ));
                                }
                                // Only the themes under the accompaniment
                                // convention have an adhésion to sign.
                                if crate::bulletin::has_bulletin(itv.kind)
                                    && motif::button(ui, tr("itv_bulletin"))
                                        .on_hover_text(tr("itv_bulletin_tooltip"))
                                        .clicked()
                                {
                                    bulletin_req = Some(itv.kind);
                                }
                                // A TROD is read once, and what it read
                                // decides whether there is anything to
                                // dispense at all.
                                if crate::ordonnance::is_trod(itv.kind) {
                                    let result = itv.trod_result.clone();
                                    for (value, label) in [
                                        (crate::ordonnance::POSITIF, tr("trod_positive")),
                                        (crate::ordonnance::NEGATIF, tr("trod_negative")),
                                    ] {
                                        let on = result == value;
                                        if motif::toggle(ui, label, on).clicked() {
                                            // Clicking the current answer
                                            // clears it: a test read by
                                            // mistake can be un-read.
                                            let next = if on { "" } else { value };
                                            set_trod =
                                                Some((itv.id, next.to_owned(), result.clone()));
                                        }
                                    }
                                    if result == crate::ordonnance::POSITIF
                                        && motif::button(ui, tr("trod_ordonnance"))
                                            .on_hover_text(tr("trod_ordonnance_tooltip"))
                                            .clicked()
                                    {
                                        open_ordonnance = Some((itv.id, itv.kind));
                                    }
                                }
                            });
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
                            // The hour sits with its date; it only
                            // means something once one is set.
                            if itv.scheduled_date.is_some() {
                                let mut hour = itv.scheduled_time.clone();
                                let h = ui.add_sized(
                                    [52.0, 22.0],
                                    egui::TextEdit::singleline(&mut hour)
                                        .hint_text(tr("agenda_hour_hint")),
                                );
                                if h.lost_focus() && hour != itv.scheduled_time {
                                    let parsed = if hour.trim().is_empty() {
                                        Some(String::new())
                                    } else {
                                        db::parse_hour(&hour)
                                    };
                                    if let Some(value) = parsed {
                                        set_hour =
                                            Some((itv.id, value, itv.scheduled_time.clone()));
                                    }
                                }
                            }
                            if field.lost_focus() {
                                let year = session.db.current_year();
                                if text.trim().is_empty() {
                                    if itv.scheduled_date.is_some() {
                                        set_date = Some((itv.id, None, itv.scheduled_date.clone()));
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
                                        set_date =
                                            Some((itv.id, Some(iso), itv.scheduled_date.clone()));
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
        let stale_msg = tr("itv_stale");
        if let Some((id, changed, expected)) = set_change {
            match session.db.set_treatment_change(id, changed, expected) {
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
        if let Some((id, remote, expected)) = set_remote {
            match session.db.set_remote(id, remote, expected) {
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
        if let Some((id, hour, expected)) = set_hour {
            match session.db.set_scheduled_time(id, &hour, &expected) {
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
        if let Some((id, theme, expected)) = set_theme {
            match session.db.set_theme(id, &theme, &expected) {
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
        if let Some((kind, scheduled, theme)) = print_req {
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
            if let Err(e) = crate::pdf::open_interview_sheet(
                patient,
                kind,
                &date,
                &theme,
                &config.template_path(),
            ) {
                session.error = Some(e);
            }
        }
        if let Some((kind, scheduled, theme)) = cr_req {
            // The CR letter to the médecin traitant, with the patient's
            // known treatments; dated like the interview sheet.
            let date = scheduled
                .as_deref()
                .map(db::format_french_date)
                .unwrap_or_else(|| {
                    session
                        .db
                        .today_french()
                        .unwrap_or_else(|_| tr("itv_date_fallback").to_owned())
                });
            if let Err(e) = crate::pdf::open_cr_letter(
                patient,
                kind,
                &date,
                &theme,
                &session.patient_treats,
                &config.pharmacy,
                &config.cr_template_path(),
            ) {
                session.error = Some(e);
            }
        }
        if let Some(kind) = bulletin_req {
            if let Err(e) = crate::pdf::open_bulletin(kind, patient, &config.pharmacy) {
                session.error = Some(e);
            }
        }
        if let Some((id, result, expected)) = set_trod {
            match session.db.set_trod_result(id, &result, &expected) {
                Ok(true) => session.reload_interviews(patient.id),
                Ok(false) => {
                    session.error = Some(tr("trod_stale").to_owned());
                    session.reload_interviews(patient.id);
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, kind)) = open_ordonnance {
            session.ordonnance = Some(OrdonnanceBox {
                interview: id,
                kind,
                choice: crate::ordonnance::Choice::default(),
            });
        }
        if session.viewing_interviews.is_empty() {
            ui.label(tr("patient_no_interviews"));
        }
        if let Some(err) = &session.error {
            ui.colored_label(motif::ALERT, err.as_str());
        }
    }

    /// Where each of the patient's accompaniments stands: one row per
    /// act kind, with the current année d'accompagnement and its
    /// sequence as filled squares.
    ///
    /// The table below says what was done; this says what is left to do
    /// and what is still billable, which is the question the counter
    /// actually asks and had to answer by counting rows.
    fn patient_sequences(
        ui: &mut egui::Ui,
        interviews: &[Interview],
        ranks: &std::collections::HashMap<i64, (usize, usize)>,
        config: &Config,
        show_amounts: bool,
    ) {
        // For each kind, the newest year reached and how many acts of
        // that year are on file.
        let mut by_kind: Vec<(InterviewKind, usize, usize)> = Vec::new();
        for kind in InterviewKind::ALL {
            let mut year = 0_usize;
            let mut seen = false;
            for itv in interviews.iter().filter(|i| i.kind == kind) {
                if let Some((y, _)) = ranks.get(&itv.id) {
                    year = year.max(*y);
                    seen = true;
                }
            }
            if !seen {
                continue;
            }
            let done = interviews
                .iter()
                .filter(|i| i.kind == kind && ranks.get(&i.id).is_some_and(|(y, _)| *y == year))
                .count();
            by_kind.push((kind, year, done));
        }
        if by_kind.is_empty() {
            return;
        }
        let row_h = 18.0;
        // Wide enough for the longest sequence on file, and no wider:
        // the count sits against its pips, not at the far right of a
        // strip the width of the pane.
        let widest = by_kind
            .iter()
            .map(|(k, y, _)| config.sequence_len(*k, *y))
            .max()
            .unwrap_or(1)
            .max(1);
        let width = (250.0 + 15.0 * widest as f32 + 56.0).min(ui.available_width());
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, by_kind.len() as f32 * row_h + 4.0),
            egui::Sense::hover(),
        );
        for (i, (kind, year, done)) in by_kind.iter().enumerate() {
            let line = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.top() + i as f32 * row_h),
                egui::vec2(rect.width(), row_h),
            );
            let swatch = egui::Rect::from_min_size(
                egui::pos2(line.left(), line.center().y - 5.0),
                egui::vec2(4.0, 10.0),
            );
            ui.painter().rect_filled(swatch, 0.0, kind_color(*kind));
            ui.painter().text(
                egui::pos2(line.left() + 12.0, line.center().y),
                egui::Align2::LEFT_CENTER,
                elide(ui, kind.label(), 150.0, 11.5),
                egui::FontId::proportional(11.5),
                motif::TEXT,
            );
            ui.painter().text(
                egui::pos2(line.left() + 170.0, line.center().y),
                egui::Align2::LEFT_CENTER,
                trf("seq_year", year + 1),
                egui::FontId::proportional(11.0),
                motif::TEXT_DIM,
            );
            let total = config.sequence_len(*kind, *year);
            if total == 0 {
                // No quota configured for this kind: nothing to fill in.
                continue;
            }
            let pips = egui::Rect::from_min_max(
                egui::pos2(line.left() + 250.0, line.top() + 3.0),
                egui::pos2(
                    (line.left() + 250.0 + 15.0 * total as f32).min(line.right() - 50.0),
                    line.bottom() - 3.0,
                ),
            );
            if pips.width() > 8.0 {
                motif::chart::pips(ui, pips, *done, total, motif::ACCENT);
            }
            ui.painter().text(
                egui::pos2(pips.right() + 10.0, line.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{done}/{total}"),
                egui::FontId::proportional(11.0),
                if *done >= total {
                    motif::TEXT_DIM
                } else {
                    motif::TEXT
                },
            );
        }
        // What the file has brought in, and what is still owed on it.
        // The dashboard totals the officine; this totals the patient,
        // which is the figure the accompaniment is actually judged on.
        let fee = |itv: &Interview| {
            let (year, rank) = ranks.get(&itv.id).copied().unwrap_or((0, 0));
            config.act_total(itv.kind, year, rank, itv.remote)
        };
        let billed: f64 = interviews
            .iter()
            .filter(|i| i.state == InterviewState::Billed)
            .map(fee)
            .sum();
        let pending: f64 = interviews
            .iter()
            .filter(|i| i.state != InterviewState::Billed)
            .map(fee)
            .sum();
        if billed > 0.0 || pending > 0.0 {
            let money = |v: f64| {
                if config.ui.discreet_finances && !show_amounts {
                    "•••".to_owned()
                } else {
                    format!("{v:.0}")
                }
            };
            ui.label(
                egui::RichText::new(trn("seq_totals", &[&money(billed), &money(pending)]))
                    .size(11.0)
                    .color(motif::TEXT_DIM),
            );
        }
        ui.add_space(6.0);
    }

    /// The patient's dated notes journal.
    fn patient_notes_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        patient: &Patient,
        operator: &str,
    ) {
        {
            // The journal fills its pane: it used to be a 96 px box
            // whatever room it had, with grey underneath it.
            let height = 420.0;
            let (add, delete) = notes_box(
                ui,
                "patient_notes",
                &session.patient_notes,
                &mut session.note_text,
                &mut session.note_confirm,
                height,
                true,
            );
            if let Some(body) = add {
                if let Err(e) =
                    session
                        .db
                        .add_note(NoteSubject::Patient, patient.id, operator, &body)
                {
                    session.error = Some(e);
                }
                session.note_text.clear();
                session.patient_notes = session
                    .db
                    .notes_for(NoteSubject::Patient, patient.id)
                    .unwrap_or_default();
            }
            if let Some(id) = delete {
                if let Err(e) = session.db.delete_note(id) {
                    session.error = Some(e);
                }
                session.patient_notes = session
                    .db
                    .notes_for(NoteSubject::Patient, patient.id)
                    .unwrap_or_default();
            }
        }
    }

    /// A distinct colour per region.
    ///
    /// There are seventeen groups and eight chart series, so the series
    /// are walked three times, lighter then darker: two regions never
    /// share a swatch, which is the whole point of the group lens.
    fn region_color(i: usize) -> egui::Color32 {
        let base = motif::chart::SERIES[i % motif::chart::SERIES.len()];
        match i / motif::chart::SERIES.len() {
            0 => base,
            1 => base.gamma_multiply(1.6),
            _ => base.gamma_multiply(0.55),
        }
    }

    /// The colour a country's tile takes under the current lens, and
    /// the legend entry that explains it.
    ///
    /// Every lens is ordinal — "nothing to do" reads pale, "act on
    /// this" reads dark — so the map can be read at a glance without
    /// consulting the legend twice.
    fn map_tint(lens: MapLens, c: &vaccines::Country) -> egui::Color32 {
        use vaccines::{Palu, Yf};
        let step = |level: u8| match level {
            0 => motif::TROUGH,
            1 => egui::Color32::from_rgb(0x7a, 0x8c, 0x6e),
            2 => egui::Color32::from_rgb(0x9a, 0x7e, 0x33),
            _ => motif::ALERT,
        };
        match lens {
            MapLens::Group => {
                let i = vaccines::Region::ALL
                    .iter()
                    .position(|r| *r == c.region)
                    .unwrap_or(0);
                Self::region_color(i)
            }
            MapLens::YellowFever => step(match c.yf {
                Yf::No => 0,
                Yf::RequiredFromEndemic => 1,
                Yf::Recommended => 2,
                Yf::Required => 3,
            }),
            MapLens::Malaria => step(match c.palu {
                Palu::No => 0,
                Palu::Limited => 1,
                Palu::Present => 2,
                Palu::High => 3,
            }),
            MapLens::Meningo => step(if c.reco & vaccines::reco::MENINGO != 0 {
                3
            } else {
                0
            }),
            MapLens::HepatitisA => step(if c.reco & vaccines::reco::HEP_A != 0 {
                2
            } else {
                0
            }),
            MapLens::Rabies => step(if c.reco & vaccines::reco::RAGE != 0 {
                2
            } else {
                0
            }),
            MapLens::JapaneseEnceph => step(if c.reco & vaccines::reco::ENCEPH_JAP != 0 {
                2
            } else {
                0
            }),
        }
    }

    /// What the legend under the map says for the current lens.
    fn map_legend(lens: MapLens) -> Vec<(&'static str, egui::Color32)> {
        let shade = |l: u8| match l {
            0 => motif::TROUGH,
            1 => egui::Color32::from_rgb(0x7a, 0x8c, 0x6e),
            2 => egui::Color32::from_rgb(0x9a, 0x7e, 0x33),
            _ => motif::ALERT,
        };
        match lens {
            MapLens::Group => vaccines::Region::ALL
                .iter()
                .enumerate()
                .map(|(i, r)| (r.label(), Self::region_color(i)))
                .collect(),
            MapLens::YellowFever => vec![
                (tr("map_yf_no"), shade(0)),
                (tr("map_yf_from"), shade(1)),
                (tr("map_yf_reco"), shade(2)),
                (tr("map_yf_req"), shade(3)),
            ],
            MapLens::Malaria => vec![
                (tr("map_palu_no"), shade(0)),
                (tr("map_palu_limited"), shade(1)),
                (tr("map_palu_present"), shade(2)),
                (tr("map_palu_high"), shade(3)),
            ],
            _ => vec![
                (tr("map_flag_no"), shade(0)),
                (tr("map_flag_yes"), shade(2)),
            ],
        }
    }

    /// The world as a cartogram: one square per country, grouped into
    /// blocks laid out roughly where they belong. Hovering a square
    /// gives the country's group and what a traveller owes for it;
    /// clicking pins it in the detail panel.
    fn vaccine_map_view(ui: &mut egui::Ui, session: &mut Session) {
        let body = motif::visible_rect(ui).shrink(6.0);
        // The lens buttons wrap: measure the band before carving it.
        let lens_h = {
            let lines = Self::wrapped_rows(
                ui,
                body.width() - 40.0,
                MapLens::ALL.iter().map(|l| l.label()),
            );
            let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
            // 44 px of panel chrome: the inset title, its rule, and the
            // padding above and below. Leaving it out cost the band a
            // row, and the last lens with it.
            (44.0 + row * lines).min(body.height() * 0.35)
        };
        let rows = motif::split_rows(body, &[lens_h, 0.0], 8.0);
        motif::panel(ui, rows[0], Some(tr("map_lens_title")), |ui| {
            // Capped: on a short window the band scrolls past its share
            // rather than hiding a lens behind its own bottom edge.
            egui::ScrollArea::vertical()
                .id_salt("map_lenses")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for lens in MapLens::ALL {
                            if motif::toggle(ui, lens.label(), session.map_lens == lens).clicked() {
                                session.map_lens = lens;
                            }
                        }
                    });
                });
        });
        // Wide enough for a column beside the map, or a band under it.
        let work = rows[1];
        let wide = work.width() >= 1080.0;
        let (map_rect, detail_rect) = if wide {
            let side = (work.width() * 0.30).clamp(300.0, 420.0);
            (
                egui::Rect::from_min_max(
                    work.min,
                    egui::pos2(work.right() - side - 8.0, work.bottom()),
                ),
                egui::Rect::from_min_max(egui::pos2(work.right() - side, work.top()), work.max),
            )
        } else {
            let band = (work.height() * 0.38).clamp(160.0, 300.0);
            let stack = motif::split_rows(work, &[0.0, band], 8.0);
            (stack[0], stack[1])
        };

        let mut pick: Option<&'static str> = None;
        motif::panel(ui, map_rect, Some(tr("map_title")), |ui| {
            let inner = ui.max_rect();
            // The legend takes the foot of the panel; the grid gets the
            // rest, and its tiles are sized so the whole world fits.
            let legend = Self::map_legend(session.map_lens);
            let legend_h = if session.map_lens == MapLens::Group {
                (inner.height() * 0.24).clamp(52.0, 96.0)
            } else {
                26.0
            };
            let parts = motif::split_rows(inner, &[0.0, legend_h], 6.0);
            let grid = motif::well(ui, parts[0]);
            let (cols, grid_rows) = vaccines::COUNTRIES.iter().fold((1, 1), |(w, h), c| {
                let (x, y) = c.tile();
                (w.max(x + 1), h.max(y + 1))
            });
            let tile = (grid.width() / cols as f32)
                .min(grid.height() / grid_rows as f32)
                .max(6.0);
            let origin = egui::pos2(
                grid.center().x - tile * cols as f32 / 2.0,
                grid.center().y - tile * grid_rows as f32 / 2.0,
            );
            let font = egui::FontId::proportional((tile * 0.42).clamp(6.0, 11.0));
            for country in vaccines::COUNTRIES {
                let (x, y) = country.tile();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(origin.x + x as f32 * tile, origin.y + y as f32 * tile),
                    egui::vec2(tile - 1.0, tile - 1.0),
                );
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let resp = ui.interact(
                    rect,
                    ui.id().with(("map_tile", country.code)),
                    egui::Sense::click(),
                );
                let mut fill = Self::map_tint(session.map_lens, country);
                if resp.hovered() {
                    fill = fill.gamma_multiply(1.35);
                }
                ui.painter().rect_filled(rect, 0.0, fill);
                if session.map_country == Some(country.code) {
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(2.0_f32, motif::BG_LIGHT),
                    );
                } else {
                    ui.painter()
                        .rect_stroke(rect, 0.0, egui::Stroke::new(0.5_f32, motif::BG_DARK));
                }
                if tile >= 15.0 {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        country.code,
                        font.clone(),
                        egui::Color32::WHITE,
                    );
                }
                if resp.clicked() {
                    pick = Some(country.code);
                }
                resp.on_hover_ui(|ui| {
                    ui.set_max_width(320.0);
                    ui.label(egui::RichText::new(country.name).strong());
                    ui.label(
                        egui::RichText::new(country.region.label())
                            .size(11.0)
                            .color(motif::TEXT_DIM),
                    );
                    ui.label(
                        egui::RichText::new(trf("map_yf_line", country.yf.label())).size(11.5),
                    );
                    ui.label(
                        egui::RichText::new(trf("map_palu_line", country.palu.label())).size(11.5),
                    );
                    let recos: Vec<&str> = country.recos().map(|r| r.label).collect();
                    ui.label(
                        egui::RichText::new(if recos.is_empty() {
                            tr("map_reco_none").to_owned()
                        } else {
                            trf("map_reco_line", recos.join(", "))
                        })
                        .size(11.5),
                    );
                });
            }
            motif::inside(ui, parts[1], |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("map_legend")
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (label, color) in &legend {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(11.0, 11.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 0.0, *color);
                                ui.label(
                                    egui::RichText::new(*label)
                                        .size(10.5)
                                        .color(motif::TEXT_DIM),
                                );
                                ui.add_space(6.0);
                            }
                        });
                    });
            });
        });
        if let Some(code) = pick {
            session.map_country = Some(code);
        }
        Self::map_detail_pane(ui, session, detail_rect);
    }

    /// The pinned country: its group, what it asks of a traveller, and
    /// — when a patient file is open — the button that records it as a
    /// destination on that file.
    fn map_detail_pane(ui: &mut egui::Ui, session: &mut Session, rect: egui::Rect) {
        let country = session.map_country.and_then(vaccines::country);
        let title = country.map(|c| c.name).unwrap_or(tr("map_detail_title"));
        let open_patient = session.viewing.as_ref().map(|p| (p.id, p.full_name()));
        let mut add_travel: Option<(i64, &'static str)> = None;
        motif::panel(ui, rect, Some(title), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("map_detail")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let Some(c) = country else {
                        ui.label(
                            egui::RichText::new(tr("map_detail_empty"))
                                .size(11.5)
                                .color(motif::TEXT_DIM),
                        );
                        return;
                    };
                    ui.label(egui::RichText::new(trf("map_group_line", c.region.label())).strong());
                    ui.add_space(4.0);
                    ui.label(trf("map_yf_line", c.yf.label()));
                    ui.label(trf("map_palu_line", c.palu.label()));
                    ui.add_space(8.0);
                    motif::section(ui, tr("map_reco_section"));
                    ui.add_space(4.0);
                    let mut any = false;
                    for reco in c.recos() {
                        any = true;
                        ui.label(egui::RichText::new(reco.label).strong().size(12.0));
                        ui.label(
                            egui::RichText::new(reco.detail)
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        ui.add_space(4.0);
                    }
                    if c.yf.needed() {
                        any = true;
                        ui.label(
                            egui::RichText::new(tr("map_reco_yf"))
                                .strong()
                                .size(12.0)
                                .color(motif::ALERT),
                        );
                        ui.label(
                            egui::RichText::new(tr("map_reco_yf_detail"))
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        ui.add_space(4.0);
                    }
                    if !any {
                        ui.label(
                            egui::RichText::new(tr("map_reco_calendar_only"))
                                .size(11.5)
                                .color(motif::TEXT_DIM),
                        );
                    }
                    ui.add_space(8.0);
                    if let Some((id, name)) = &open_patient {
                        if motif::button(ui, &trf("map_add_travel", name))
                            .on_hover_text(tr("map_add_travel_tooltip"))
                            .clicked()
                        {
                            add_travel = Some((*id, c.code));
                        }
                        ui.add_space(6.0);
                    }
                    ui.label(
                        egui::RichText::new(tr("vacc_source"))
                            .size(10.5)
                            .italics()
                            .color(motif::TEXT_FAINT),
                    );
                });
        });
        if let Some((pid, code)) = add_travel {
            match session.db.add_travel(pid, code, "") {
                Ok(()) => session.load_carnet(pid),
                Err(e) => session.error = Some(e),
            }
        }
    }

    /// The end-of-day transmission logbook (F5): one page per day,
    /// entries stamped time · operator, browsable day by day and
    /// printable; writing is only possible on today's page.
    fn transmissions_view(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        operator: &str,
        config: &Config,
    ) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            session.view = MainView::Search;
            return;
        }
        motif::page(ui, 900.0, |ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.heading(tr("trans_title"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !session.trans_notes.is_empty()
                        && motif::button(ui, tr("dash_print"))
                            .on_hover_text(tr("trans_print_tooltip"))
                            .clicked()
                    {
                        let day = &session.trans_day;
                        let title = format!(
                            "{} {}",
                            db::weekday_fr(day).unwrap_or(""),
                            db::format_french_date(day)
                        );
                        if let Err(e) = crate::pdf::open_transmission_day(
                            &title,
                            &session.trans_notes,
                            &config.carnet_template_path(),
                        ) {
                            session.error = Some(e);
                        }
                    }
                });
            });
            ui.label(tr("trans_subtitle"));
            ui.add_space(10.0);

            // Day navigation: previous day with entries (calendar
            // fallback), today, next day with entries.
            let mut goto: Option<String> = None;
            ui.horizontal(|ui| {
                if motif::button(ui, "‹")
                    .on_hover_text(tr("trans_prev"))
                    .clicked()
                {
                    let prev = session
                        .trans_days
                        .iter()
                        .find(|d| **d < session.trans_day)
                        .cloned();
                    goto = prev.or_else(|| session.db.date_offset(&session.trans_day, -1).ok());
                }
                if motif::button(ui, tr("agenda_this_week")).clicked() {
                    goto = session.db.today_iso().ok();
                }
                if motif::button(ui, "›")
                    .on_hover_text(tr("trans_next"))
                    .clicked()
                {
                    let next = session
                        .trans_days
                        .iter()
                        .rev()
                        .find(|d| **d > session.trans_day)
                        .cloned();
                    goto = next.or_else(|| {
                        if session.trans_day < session.today {
                            Some(session.today.clone())
                        } else {
                            None
                        }
                    });
                }
                let day = db::weekday_fr(&session.trans_day).unwrap_or("");
                let cap: String = day
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_uppercase().next().unwrap_or(c)
                        } else {
                            c
                        }
                    })
                    .collect();
                let mut title = format!("{cap} {}", db::format_french_date(&session.trans_day));
                if session.trans_day == session.today {
                    title.push_str(tr("dash_today"));
                }
                ui.label(egui::RichText::new(title).strong());
            });
            if let Some(day) = goto {
                session.trans_day = day;
                session.trans_notes = session
                    .db
                    .transmissions_for_day(&session.trans_day)
                    .unwrap_or_default();
                session.note_confirm = None;
            }
            ui.add_space(8.0);

            let is_today = session.trans_day == session.today;
            let h = (ui.available_height() - 60.0).max(180.0);
            let (add, delete) = notes_box(
                ui,
                "transmissions",
                &session.trans_notes,
                &mut session.note_text,
                &mut session.note_confirm,
                h,
                is_today,
            );
            if !is_today {
                ui.label(
                    egui::RichText::new(tr("trans_readonly"))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
            }
            if let Some(body) = add {
                if let Err(e) = session
                    .db
                    .add_note(NoteSubject::Transmission, 0, operator, &body)
                {
                    session.error = Some(e);
                }
                session.note_text.clear();
                session.load_transmissions();
            }
            if let Some(id) = delete {
                if let Err(e) = session.db.delete_note(id) {
                    session.error = Some(e);
                }
                session.load_transmissions();
            }
            if let Some(err) = &session.error {
                ui.colored_label(motif::ALERT, err.as_str());
            }
        });
    }

    /// Agenda (F4): the upcoming patient appointments grouped by day,
    /// soonest first, overdue days flagged. Clicking an entry opens the
    /// patient; the list is printable.
    /// The selected day as an hour column: the opening hours down the
    /// left, each rendez-vous and entry placed on its own line. What has
    /// no hour is listed under the plan, so nothing is hidden.
    fn agenda_day_plan(
        ui: &mut egui::Ui,
        session: &mut Session,
        events: &[db::Event],
        config: &Config,
        open_id: &mut Option<i64>,
    ) {
        let day = session.agenda_day.clone();
        let start = config.ui.day_start_hour.min(23);
        let end = config.ui.day_end_hour.clamp(start + 1, 24);
        let hours = (end - start) as f32;
        // No day navigation of its own any more: the agenda's control
        // band drives all three modes from one set of buttons, and two
        // « ‹ Aujourd'hui › » rows on the same screen was one too many.

        // The hour column fills the pane it is given, down to a legible
        // row: a fixed 34 px row left the plan floating in the top half
        // of a panel and scrolling in a short one.
        let avail = motif::visible_rect(ui);
        let w = (avail.width() - 16.0).max(300.0);
        let row_h = ((avail.height() - 12.0) / hours).clamp(20.0, 44.0);
        let (alloc, _) =
            ui.allocate_exact_size(egui::vec2(w, hours * row_h + 8.0), egui::Sense::hover());
        let plan =
            egui::Rect::from_min_size(egui::pos2(avail.left() + 8.0, alloc.top()), alloc.size());
        ui.painter().rect_filled(plan, 0.0, motif::TROUGH);
        motif::bevel(ui.painter(), plan, false);
        let inner = plan.shrink(4.0);
        let gutter = 54.0;
        // Hour lines and their labels.
        for i in 0..=(end - start) {
            let y = inner.top() + i as f32 * row_h;
            ui.painter().line_segment(
                [egui::pos2(inner.left(), y), egui::pos2(inner.right(), y)],
                egui::Stroke::new(0.5_f32, motif::BG_DARK),
            );
            if i < end - start {
                ui.painter().text(
                    egui::pos2(inner.left() + 6.0, y + row_h / 2.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{:02} h", start + i),
                    egui::FontId::proportional(11.0),
                    motif::TEXT_DIM,
                );
            }
        }
        ui.painter().line_segment(
            [
                egui::pos2(inner.left() + gutter, inner.top()),
                egui::pos2(inner.left() + gutter, inner.bottom()),
            ],
            egui::Stroke::new(1.0_f32, motif::BG_DARK),
        );
        // Where an entry sits: its hour, minutes to the fraction of the
        // row. Several entries in the same hour share the width.
        let place = |time: &str| -> Option<f32> {
            let t = db::parse_hour(time)?;
            let (h, m) = t.split_once(':')?;
            let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
            if h < start || h >= end {
                return None;
            }
            Some((h - start) as f32 * row_h + (m as f32 / 60.0) * row_h)
        };
        let mut untimed: Vec<String> = Vec::new();
        let mut slots: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
        let draw = |ui: &mut egui::Ui,
                    time: &str,
                    label: String,
                    color: egui::Color32,
                    hover: String,
                    patient: Option<i64>,
                    untimed: &mut Vec<String>,
                    slots: &mut std::collections::HashMap<i64, usize>,
                    open_id: &mut Option<i64>| {
            let Some(offset) = place(time) else {
                untimed.push(label);
                return;
            };
            let hour_key = (offset / row_h) as i64;
            let column = slots.entry(hour_key).or_insert(0);
            let index = *column;
            *column += 1;
            let width = (inner.width() - gutter - 8.0) / 2.0;
            let block = egui::Rect::from_min_size(
                egui::pos2(
                    inner.left() + gutter + 4.0 + (index % 2) as f32 * width,
                    inner.top() + offset + 2.0,
                ),
                egui::vec2(width - 4.0, row_h - 6.0),
            );
            ui.painter().rect_filled(block, 0.0, color);
            ui.painter().with_clip_rect(block.shrink(2.0)).text(
                egui::pos2(block.left() + 5.0, block.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(11.5),
                egui::Color32::WHITE,
            );
            let resp = ui.interact(
                block,
                ui.id().with(("dayblk", hour_key, index)),
                egui::Sense::click(),
            );
            if resp.on_hover_text(hover).clicked() {
                if let Some(id) = patient {
                    *open_id = Some(id);
                }
            }
        };
        for rdv in session
            .appointments
            .iter()
            .filter(|r| r.date == day)
            .cloned()
            .collect::<Vec<_>>()
        {
            let label = format!("{} {}", rdv.time, rdv.patient_name);
            let hover = format!("{} — {}", rdv.patient_name, rdv.kind.label());
            draw(
                ui,
                &rdv.time,
                label,
                kind_color(rdv.kind),
                hover,
                Some(rdv.patient_id),
                &mut untimed,
                &mut slots,
                open_id,
            );
        }
        for ev in events.iter().filter(|e| e.day == day) {
            let label = format!("{} {}", ev.time, ev.title);
            let hover = format!("{} — {}", ev.category.label(), ev.title);
            draw(
                ui,
                &ev.time,
                label,
                motif::BG_DARK,
                hover,
                None,
                &mut untimed,
                &mut slots,
                open_id,
            );
        }
        let below = (plan.bottom() - ui.cursor().top()).max(0.0) + 8.0;
        ui.add_space(below);
        if !untimed.is_empty() {
            motif::column(ui, 940.0, |ui| {
                ui.label(
                    egui::RichText::new(tr("agenda_untimed"))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
                for label in &untimed {
                    ui.label(egui::RichText::new(label.trim()).size(12.0));
                }
            });
            ui.add_space(6.0);
        }
    }

    /// The month as a Monday-aligned grid: each cell carries the day
    /// number, a coloured dot per act and a grey one per other entry.
    /// Clicking a cell details that day below.
    fn agenda_month_grid(
        ui: &mut egui::Ui,
        session: &mut Session,
        events: &[db::Event],
        pick_day: &mut Option<String>,
        _open_id: &mut Option<i64>,
    ) {
        // The month has no navigation of its own: the agenda's control
        // band drives all three modes from one set of buttons.
        if session.agenda_month_days.is_empty() {
            session.agenda_month_days = session
                .db
                .month_grid(session.agenda_month_offset)
                .unwrap_or_default();
        }
        let days = session.agenda_month_days.clone();
        let rows = days.len().div_ceil(7).max(1);
        // Six weeks fill the pane: a fixed 62 px cell left a third of
        // the panel grey under the last row.
        let avail = motif::visible_rect(ui);
        let grid_w = (avail.width() - 16.0).max(360.0);
        let cell_h = ((avail.height() - 30.0) / rows as f32).clamp(44.0, 96.0);
        let (alloc, _) = ui.allocate_exact_size(
            egui::vec2(grid_w, rows as f32 * cell_h + 22.0),
            egui::Sense::hover(),
        );
        let grid =
            egui::Rect::from_min_size(egui::pos2(avail.left() + 8.0, alloc.top()), alloc.size());
        ui.painter().rect_filled(grid, 0.0, motif::TROUGH);
        motif::bevel(ui.painter(), grid, false);
        let inner = grid.shrink(4.0);
        let col_w = inner.width() / 7.0;
        for (i, head) in ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"]
            .into_iter()
            .enumerate()
        {
            ui.painter().text(
                egui::pos2(inner.left() + (i as f32 + 0.5) * col_w, inner.top() + 9.0),
                egui::Align2::CENTER_CENTER,
                head,
                egui::FontId::proportional(11.0),
                motif::TEXT_DIM,
            );
        }
        let month = session
            .db
            .month_of(session.agenda_month_offset)
            .unwrap_or_default();
        for (idx, date) in days.iter().enumerate() {
            let (r, c) = (idx / 7, idx % 7);
            let cell = egui::Rect::from_min_size(
                egui::pos2(
                    inner.left() + c as f32 * col_w,
                    inner.top() + 20.0 + r as f32 * cell_h,
                ),
                egui::vec2(col_w, cell_h),
            );
            let in_month = date.starts_with(&month);
            if *date == session.today {
                ui.painter()
                    .rect_filled(cell.shrink(1.0), 0.0, motif::BG_HOVER);
            } else if !in_month {
                ui.painter()
                    .rect_filled(cell.shrink(1.0), 0.0, motif::TROUGH);
            } else {
                ui.painter().rect_filled(cell.shrink(1.0), 0.0, motif::BG);
            }
            if *date == session.agenda_day {
                ui.painter().rect_stroke(
                    cell.shrink(1.0),
                    0.0,
                    egui::Stroke::new(1.5_f32, motif::ACCENT),
                );
            }
            ui.painter().text(
                egui::pos2(cell.left() + 6.0, cell.top() + 10.0),
                egui::Align2::LEFT_CENTER,
                date.get(8..10).unwrap_or("").trim_start_matches('0'),
                egui::FontId::proportional(12.0),
                if in_month {
                    motif::TEXT
                } else {
                    motif::TEXT_FAINT
                },
            );
            // One chip per act, then the other entries, clipped to the cell.
            let mut x = cell.left() + 5.0;
            let mut y = cell.top() + 22.0;
            let mut chip = |color: egui::Color32, painter: &egui::Painter| {
                if x + 12.0 > cell.right() - 4.0 {
                    x = cell.left() + 5.0;
                    y += 12.0;
                }
                if y + 8.0 < cell.bottom() {
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(9.0, 7.0)),
                        0.0,
                        color,
                    );
                }
                x += 12.0;
            };
            for rdv in session.appointments.iter().filter(|r| r.date == *date) {
                chip(kind_color(rdv.kind), ui.painter());
            }
            for _ in events.iter().filter(|e| e.day == *date) {
                chip(motif::BG_DARK, ui.painter());
            }
            let resp = ui.interact(cell, ui.id().with(("mcell", idx)), egui::Sense::click());
            let n_rdv = session
                .appointments
                .iter()
                .filter(|r| r.date == *date)
                .count();
            let n_ev = events.iter().filter(|e| e.day == *date).count();
            if resp
                .on_hover_text(trn(
                    "agenda_day_summary",
                    &[&db::format_french_date(date), &n_rdv, &n_ev],
                ))
                .clicked()
            {
                *pick_day = Some(date.clone());
            }
        }
        ui.add_space(8.0);
    }

    /// The selected day: its acts, its other entries, and its notes.
    fn agenda_day_panel(
        ui: &mut egui::Ui,
        session: &mut Session,
        operator: &str,
        open_id: &mut Option<i64>,
    ) {
        let day = session.agenda_day.clone();
        let mut delete_event: Option<(i64, String)> = None;
        let mut add_event = false;
        // (id, typed text, the value this PC displayed) for the two
        // compare-and-set writes the panel can make.
        let mut set_time: Option<(i64, String, String)> = None;
        let mut move_rdv: Option<(i64, String, String)> = None;
        motif::page(ui, 940.0, |ui| {
            let rdvs: Vec<Appointment> = session
                .appointments
                .iter()
                .filter(|r| r.date == day)
                .cloned()
                .collect();
            if rdvs.is_empty() && session.events.is_empty() {
                ui.label(
                    egui::RichText::new(tr("agenda_day_empty"))
                        .size(12.0)
                        .color(motif::TEXT_DIM),
                );
            }
            for rdv in &rdvs {
                ui.horizontal(|ui| {
                    // The hour, typed the fast way: 9, 9h30, 930, 09:30.
                    let editing = session
                        .rdv_time_edit
                        .as_ref()
                        .is_some_and(|(id, _)| *id == rdv.id);
                    if editing {
                        let (_, text) = session.rdv_time_edit.as_mut().unwrap();
                        let field = ui.add_sized(
                            [56.0, 22.0],
                            egui::TextEdit::singleline(text).hint_text(tr("agenda_hour_hint")),
                        );
                        if field.lost_focus() {
                            set_time = Some((rdv.id, text.clone(), rdv.time.clone()));
                        } else {
                            field.request_focus();
                        }
                    } else {
                        let shown = if rdv.time.is_empty() {
                            tr("agenda_no_hour").to_owned()
                        } else {
                            rdv.time.clone()
                        };
                        if ui
                            .add_sized(
                                [56.0, 20.0],
                                egui::Button::new(egui::RichText::new(shown).size(12.0).strong())
                                    .fill(motif::BG),
                            )
                            .on_hover_text(tr("agenda_hour_tooltip"))
                            .clicked()
                        {
                            session.rdv_time_edit = Some((rdv.id, rdv.time.clone()));
                        }
                    }
                    ui.label(
                        egui::RichText::new(format!("  {}  ", rdv.kind.label()))
                            .size(11.0)
                            .color(egui::Color32::WHITE)
                            .background_color(kind_color(rdv.kind)),
                    );
                    if ui
                        .selectable_label(false, &rdv.patient_name)
                        .on_hover_text(tr("dash_open_patient"))
                        .clicked()
                    {
                        *open_id = Some(rdv.patient_id);
                    }
                    if !rdv.phone.is_empty() {
                        ui.label(
                            egui::RichText::new(&rdv.phone)
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                    }
                    // Moving a rendez-vous without opening the record.
                    let moving = session
                        .rdv_move_edit
                        .as_ref()
                        .is_some_and(|(id, _)| *id == rdv.id);
                    if moving {
                        let (_, text) = session.rdv_move_edit.as_mut().unwrap();
                        let field = ui.add_sized(
                            [96.0, 22.0],
                            egui::TextEdit::singleline(text).hint_text(tr("itv_rdv_hint")),
                        );
                        if field.lost_focus() {
                            move_rdv = Some((rdv.id, text.clone(), rdv.date.clone()));
                        } else {
                            field.request_focus();
                        }
                    } else if motif::button(ui, tr("agenda_move"))
                        .on_hover_text(tr("agenda_move_tooltip"))
                        .clicked()
                    {
                        session.rdv_move_edit = Some((rdv.id, db::format_french_date(&rdv.date)));
                    }
                });
            }
            for ev in session.events.clone() {
                ui.horizontal(|ui| {
                    if ev.time.is_empty() {
                        ui.add_space(48.0);
                    } else {
                        ui.label(egui::RichText::new(&ev.time).size(12.0).strong());
                    }
                    ui.label(
                        egui::RichText::new(format!("  {}  ", ev.category.label()))
                            .size(11.0)
                            .color(egui::Color32::WHITE)
                            .background_color(motif::BG_DARK),
                    );
                    ui.label(&ev.title);
                    if ev.repeat_days > 0 {
                        ui.label(
                            egui::RichText::new(trf("agenda_repeat_mark", ev.repeat_days))
                                .size(10.0)
                                .color(motif::TEXT_DIM),
                        );
                    }
                    if motif::button(ui, tr("itv_delete"))
                        .on_hover_text(if ev.repeat_days > 0 {
                            tr("agenda_event_delete_series")
                        } else {
                            tr("agenda_event_delete")
                        })
                        .clicked()
                    {
                        delete_event = Some((ev.source_id, ev.title.clone()));
                    }
                });
            }
            ui.add_space(6.0);
            // The category, the hour and the repeat are fixed-size
            // controls; the title takes whatever the panel has left, so
            // the row fits a docked column as well as a full window.
            // Docked, the form breaks into short rows; given a full
            // window it stays the single line it always was. Either way
            // nothing runs off the right edge, which is what a fixed
            // 250 px title field did as soon as the panel was a column.
            let avail = ui.available_width();
            let narrow = avail < 560.0;
            let mut entered = false;
            let category = |ui: &mut egui::Ui, session: &mut Session| {
                egui::ComboBox::from_id_salt("event_cat")
                    .selected_text(session.event_category.label())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for c in db::EventCategory::ALL {
                            ui.selectable_value(&mut session.event_category, c, c.label());
                        }
                    });
                ui.add_sized(
                    [52.0, 24.0],
                    egui::TextEdit::singleline(&mut session.event_time)
                        .hint_text(tr("agenda_hour_hint")),
                );
            };
            let title = |ui: &mut egui::Ui, session: &mut Session, w: f32| -> egui::Response {
                ui.add_sized(
                    [w, 24.0],
                    egui::TextEdit::singleline(&mut session.event_title)
                        .hint_text(tr("agenda_event_hint")),
                )
            };
            let repeat = |ui: &mut egui::Ui, session: &mut Session| {
                // Every week, every fortnight, every month or once.
                let label = match session.event_repeat {
                    7 => tr("agenda_repeat_week"),
                    14 => tr("agenda_repeat_fortnight"),
                    28 => tr("agenda_repeat_month"),
                    _ => tr("agenda_repeat_once"),
                };
                egui::ComboBox::from_id_salt("event_repeat")
                    .selected_text(label)
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for (days, label) in [
                            (0, tr("agenda_repeat_once")),
                            (7, tr("agenda_repeat_week")),
                            (14, tr("agenda_repeat_fortnight")),
                            (28, tr("agenda_repeat_month")),
                        ] {
                            ui.selectable_value(&mut session.event_repeat, days, label);
                        }
                    });
            };
            if narrow {
                ui.horizontal(|ui| category(ui, session));
                let field = title(ui, session, (avail - 12.0).max(120.0));
                entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    repeat(ui, session);
                    if (motif::button(ui, tr("agenda_event_add")).clicked() || entered)
                        && !session.event_title.trim().is_empty()
                    {
                        add_event = true;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    category(ui, session);
                    let field = title(ui, session, (avail - 360.0).clamp(140.0, 420.0));
                    entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    repeat(ui, session);
                    if (motif::button(ui, tr("agenda_event_add")).clicked() || entered)
                        && !session.event_title.trim().is_empty()
                    {
                        add_event = true;
                    }
                });
            }
            // The day's own notes, same journal widget as everywhere.
            ui.add_space(8.0);
            motif::section(ui, tr("agenda_day_notes"));
            ui.add_space(4.0);
        });
        let (note_add, note_delete) = notes_box(
            ui,
            "day_notes",
            &session.day_notes,
            &mut session.day_note_text,
            &mut session.day_note_confirm,
            76.0,
            true,
        );
        if add_event {
            let title = session.event_title.trim().to_owned();
            let time = db::parse_hour(&session.event_time).unwrap_or_default();
            match session.db.add_event(
                &day,
                &time,
                &title,
                session.event_category,
                session.event_repeat,
                "",
            ) {
                Ok(_) => {
                    session.event_title.clear();
                    session.event_time.clear();
                    session.load_day();
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, typed, expected)) = set_time {
            // An empty field clears the hour; anything unreadable is
            // refused, leaving the hour as it was.
            let time = if typed.trim().is_empty() {
                Some(String::new())
            } else {
                db::parse_hour(&typed)
            };
            if let Some(time) = time {
                match session.db.set_scheduled_time(id, &time, &expected) {
                    Ok(true) => {}
                    Ok(false) => session.error = Some(tr("itv_stale").to_owned()),
                    Err(e) => session.error = Some(e),
                }
                session.refresh_dashboard();
            }
            session.rdv_time_edit = None;
        }
        if let Some((id, typed, expected)) = move_rdv {
            let year = session.db.current_year();
            match db::parse_french_date(&typed, year, db::YearHint::Future) {
                Ok(iso) => match session
                    .db
                    .set_scheduled_date(id, Some(&iso), Some(&expected))
                {
                    Ok(true) => {
                        session.refresh_dashboard();
                        session.load_day();
                    }
                    Ok(false) => session.error = Some(tr("itv_stale").to_owned()),
                    Err(e) => session.error = Some(e),
                },
                Err(e) => session.error = Some(e),
            }
            session.rdv_move_edit = None;
        }
        if let Some((id, title)) = delete_event {
            match session.db.delete_event(id, &title) {
                Ok(true) => session.load_day(),
                Ok(false) => {
                    session.load_day();
                    session.error = Some(tr("agenda_event_stale").to_owned());
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some(body) = note_add {
            if let Err(e) =
                session
                    .db
                    .add_note(NoteSubject::Day, db::day_subject_id(&day), operator, &body)
            {
                session.error = Some(e);
            }
            session.day_note_text.clear();
            session.load_day();
        }
        if let Some(id) = note_delete {
            if let Err(e) = session.db.delete_note(id) {
                session.error = Some(e);
            }
            session.load_day();
        }
    }

    fn agenda_view(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &mut Session,
        operator: &str,
        config: &Config,
    ) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            session.view = MainView::Search;
            return;
        }
        // Left and right arrows move the week or the month shown.
        if !ctx.wants_keyboard_input() {
            let step = ctx.input(|i| {
                i.key_pressed(egui::Key::ArrowRight) as i64
                    - i.key_pressed(egui::Key::ArrowLeft) as i64
            });
            if step != 0 {
                if session.agenda_mode == AgendaMode::Day {
                    let day = session.agenda_day.clone();
                    if let Ok(next) = session.db.date_offset(&day, step) {
                        session.agenda_day = next;
                        session.load_day();
                    }
                } else if session.agenda_month {
                    session.agenda_month_offset += step;
                    session.agenda_month_days = session
                        .db
                        .month_grid(session.agenda_month_offset)
                        .unwrap_or_default();
                } else {
                    session.agenda_offset += step;
                    session.agenda_week = session
                        .db
                        .week_dates(session.agenda_offset)
                        .unwrap_or_default();
                }
            }
        }
        let mut print_week = false;
        let red = motif::ALERT;
        let mut open_id: Option<i64> = None;
        // The filter applies to every part of the view at once.
        if !session.agenda_filter.is_empty() {
            let keep = session.agenda_filter.clone();
            session.appointments.retain(|r| keep.contains(&r.kind));
        }
        // What has slipped past its date and is still waiting: the
        // agenda says so before anything else.
        let overdue: Vec<Appointment> = session
            .appointments
            .iter()
            .filter(|r| r.date < session.today)
            .cloned()
            .collect();
        // The grid's entries that are not acts (formation, réunion…).
        let grid_events = session.load_grid_events();
        if session.agenda_day.is_empty() {
            session.agenda_day = session.today.clone();
            session.load_day();
        }

        // A control band across the top, then the calendar and the
        // selected day side by side. The five sections used to run down
        // one 900 px column, which left the week grid 150 px tall and
        // pushed the day's own plan below the fold.
        let body = motif::visible_rect(ui).shrink2(egui::vec2(4.0, 0.0));
        // Both rows of the band wrap: measure them rather than assume
        // one line each, which cut the last act kinds off the bottom.
        let filter_lines = Self::wrapped_rows(
            ui,
            body.width() - 60.0,
            std::iter::once(tr("agenda_filter_all"))
                .chain(InterviewKind::ALL.iter().map(|k| k.label())),
        );
        let control_lines = Self::wrapped_rows(
            ui,
            body.width() - 20.0,
            [
                tr("agenda_mode_day"),
                tr("agenda_mode_week"),
                tr("agenda_mode_month"),
                "‹",
                tr("agenda_this_week"),
                "›",
                tr("agenda_week_of"),
                tr("dash_print"),
                tr("agenda_print_week"),
            ]
            .into_iter(),
        );
        let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y + 8.0;
        let want =
            row * (control_lines + filter_lines) + if overdue.is_empty() { 6.0 } else { row + 6.0 };
        // However much the band would like, the calendar keeps most of
        // the screen: past the cap the band scrolls instead.
        let band_h = want.min((body.height() * 0.4).max(120.0));
        let rows = motif::split_rows(body, &[band_h, 0.0], 6.0);
        // Set inside the band's closure, applied after it: the closure
        // already holds `session` uniquely.
        let mut banner_open: Option<i64> = None;
        motif::panel(ui, rows[0], None, |ui| {
            let inner = ui.max_rect();
            egui::ScrollArea::vertical()
                .id_salt("agenda_band")
                .show(ui, |ui| {
                    ui.set_max_width(inner.width());
                    ui.horizontal_wrapped(|ui| {
                        for (mode, label) in [
                            (AgendaMode::Day, tr("agenda_mode_day")),
                            (AgendaMode::Week, tr("agenda_mode_week")),
                            (AgendaMode::Month, tr("agenda_mode_month")),
                        ] {
                            let btn = motif::toggle(ui, label, session.agenda_mode == mode);
                            if btn.clicked() {
                                session.agenda_mode = mode;
                                session.agenda_month = mode == AgendaMode::Month;
                                if mode == AgendaMode::Month && session.agenda_month_days.is_empty()
                                {
                                    session.agenda_month_days = session
                                        .db
                                        .month_grid(session.agenda_month_offset)
                                        .unwrap_or_default();
                                }
                            }
                        }
                        ui.add_space(10.0);
                        Self::agenda_nav_buttons(ui, session);
                        {
                            if motif::button(ui, tr("agenda_print_week"))
                                .on_hover_text(tr("agenda_print_week_tooltip"))
                                .clicked()
                            {
                                print_week = true;
                            }
                            if !session.appointments.is_empty()
                                && motif::button(ui, tr("dash_print"))
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
                        }
                    });
                    ui.add_space(4.0);
                    // Filter by act kind: an empty set shows everything, so the
                    // agenda opens complete and narrows only on demand.
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(tr("agenda_filter"))
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        if motif::toggle(
                            ui,
                            tr("agenda_filter_all"),
                            session.agenda_filter.is_empty(),
                        )
                        .on_hover_text(tr("agenda_filter_tooltip"))
                        .clicked()
                        {
                            session.agenda_filter.clear();
                        }
                        for kind in InterviewKind::ALL {
                            let on = session.agenda_filter.contains(&kind);
                            // The same sunken-when-on idiom as everywhere else,
                            // with the act's own colour as a marker so the
                            // filter row and the grid read as one thing.
                            let resp = motif::toggle(ui, kind.label(), on);
                            let dot = egui::Rect::from_center_size(
                                egui::pos2(resp.rect.left() + 6.0, resp.rect.center().y),
                                egui::vec2(4.0, resp.rect.height() - 10.0),
                            );
                            ui.painter().rect_filled(dot, 0.0, kind_color(kind));
                            if resp.clicked() {
                                if on {
                                    session.agenda_filter.remove(&kind);
                                } else {
                                    session.agenda_filter.insert(kind);
                                }
                            }
                        }
                    });
                    if !overdue.is_empty() {
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(trn(
                                        "agenda_overdue_banner",
                                        &[
                                            &overdue.len(),
                                            &db::format_french_date(&overdue[0].date),
                                        ],
                                    ))
                                    .strong()
                                    .color(motif::ALERT),
                                );
                                for rdv in overdue.iter().take(4) {
                                    if ui
                                        .selectable_label(
                                            false,
                                            egui::RichText::new(&rdv.patient_name)
                                                .color(motif::ALERT),
                                        )
                                        .on_hover_text(tr("agenda_overdue_tooltip"))
                                        .clicked()
                                    {
                                        banner_open = Some(rdv.patient_id);
                                    }
                                }
                                if overdue.len() > 4 {
                                    ui.label(
                                        egui::RichText::new(trf("dash_more", overdue.len() - 4))
                                            .size(11.0)
                                            .color(motif::ALERT),
                                    );
                                }
                            });
                        }
                    }
                });
        });

        open_id = open_id.or(banner_open);

        let work = rows[1];
        // The day panel is a fixed column of forms; the calendar takes
        // everything else.
        let wide = work.width() >= 860.0;
        let (cal, day) = if wide {
            let day_w = (work.width() * 0.3).clamp(300.0, 420.0);
            (
                egui::Rect::from_min_max(
                    work.min,
                    egui::pos2(work.right() - day_w - 8.0, work.bottom()),
                ),
                egui::Rect::from_min_max(egui::pos2(work.right() - day_w, work.top()), work.max),
            )
        } else {
            let day_h = (work.height() * 0.42).clamp(150.0, 300.0);
            let stack = motif::split_rows(work, &[0.0, day_h], 8.0);
            (stack[0], stack[1])
        };

        let mut pick_day: Option<String> = None;
        let cal_title = match session.agenda_mode {
            AgendaMode::Day => tr("agenda_mode_day"),
            AgendaMode::Week => tr("agenda_mode_week"),
            AgendaMode::Month => tr("agenda_mode_month"),
        };
        motif::panel(ui, cal, Some(cal_title), |ui| {
            let rect = ui.max_rect();
            match session.agenda_mode {
                AgendaMode::Day => {
                    motif::inside(ui, rect, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("agenda_day_plan")
                            .show(ui, |ui| {
                                Self::agenda_day_plan(
                                    ui,
                                    session,
                                    &grid_events,
                                    config,
                                    &mut open_id,
                                );
                            });
                    });
                }
                AgendaMode::Month => {
                    motif::inside(ui, rect, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("agenda_month")
                            .show(ui, |ui| {
                                Self::agenda_month_grid(
                                    ui,
                                    session,
                                    &grid_events,
                                    &mut pick_day,
                                    &mut open_id,
                                );
                            });
                    });
                }
                AgendaMode::Week => {
                    Self::agenda_week_grid(
                        ui,
                        session,
                        &grid_events,
                        rect,
                        &mut pick_day,
                        &mut open_id,
                    );
                }
            }
        });

        // The selected day, and — when there is room for it — what is
        // coming after. On a narrow window the queue is dropped: the
        // left dock already lists it, and the calendar needs the height
        // more than a second copy of it does.
        let day_rows = if wide {
            motif::split_rows(day, &[0.0, (day.height() * 0.38).clamp(110.0, 260.0)], 6.0)
        } else {
            vec![day, egui::Rect::NOTHING]
        };
        motif::panel(
            ui,
            day_rows[0],
            Some(&trf(
                "agenda_day_title",
                db::format_french_date(&session.agenda_day),
            )),
            |ui| {
                let rect = ui.max_rect();
                motif::inside(ui, rect, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("agenda_day_panel")
                        .show(ui, |ui| {
                            Self::agenda_day_panel(ui, session, operator, &mut open_id);
                        });
                });
            },
        );
        if wide {
            motif::panel(ui, day_rows[1], Some(tr("nav_next")), |ui| {
                Self::agenda_upcoming(ui, session, &mut open_id);
            });
        }

        if let Some(day) = pick_day.take() {
            session.agenda_day = day;
            session.load_day();
        }
        if print_week {
            let week = if session.agenda_month {
                session.agenda_month_days.clone()
            } else {
                session.agenda_week.clone()
            };
            // The month grid prints as its first seven days: the week
            // plan is a week, whichever view asked for it.
            let week: Vec<String> = week.into_iter().take(7).collect();
            if let Err(e) = crate::pdf::open_week_plan(
                &week,
                &session.appointments,
                &grid_events,
                &session.today,
            ) {
                session.error = Some(e);
            }
        }
        if let Some(id) = open_id {
            if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                session.view = MainView::Search;
                session.open_patient(p);
            }
        }
        if let Some(err) = &session.error {
            ui.colored_label(red, err.as_str());
        }
    }

    /// « ‹ · Aujourd'hui · › » plus the label of what is shown. One set
    /// of controls for all three modes, instead of one per mode.
    fn agenda_nav_buttons(ui: &mut egui::Ui, session: &mut Session) {
        let step = |session: &mut Session, delta: i64| match session.agenda_mode {
            AgendaMode::Day => {
                let day = session.agenda_day.clone();
                if let Some(next) = db::add_days(&day, delta) {
                    session.agenda_day = next;
                    session.load_day();
                }
            }
            AgendaMode::Month => {
                session.agenda_month_offset += delta;
                session.agenda_month_days = session
                    .db
                    .month_grid(session.agenda_month_offset)
                    .unwrap_or_default();
            }
            AgendaMode::Week => {
                session.agenda_offset += delta;
                session.agenda_week = session
                    .db
                    .week_dates(session.agenda_offset)
                    .unwrap_or_default();
            }
        };
        if motif::button(ui, "‹")
            .on_hover_text(tr("agenda_prev_week"))
            .clicked()
        {
            step(session, -1);
        }
        if motif::button(ui, tr("agenda_this_week")).clicked() {
            match session.agenda_mode {
                AgendaMode::Day => {
                    session.agenda_day = session.today.clone();
                    session.load_day();
                }
                AgendaMode::Month => {
                    session.agenda_month_offset = 0;
                    session.agenda_month_days = session.db.month_grid(0).unwrap_or_default();
                }
                AgendaMode::Week => {
                    session.agenda_offset = 0;
                    session.agenda_week = session.db.week_dates(0).unwrap_or_default();
                }
            }
        }
        if motif::button(ui, "›")
            .on_hover_text(tr("agenda_next_week"))
            .clicked()
        {
            step(session, 1);
        }
        let label = match session.agenda_mode {
            AgendaMode::Day => db::format_french_date(&session.agenda_day),
            AgendaMode::Month => session
                .db
                .month_of(session.agenda_month_offset)
                .map(|m| trf("agenda_month_of", db::month_name_fr(&m)))
                .unwrap_or_default(),
            AgendaMode::Week => session
                .agenda_week
                .first()
                .map(|m| trf("agenda_week_of", db::format_french_date(m)))
                .unwrap_or_default(),
        };
        ui.label(egui::RichText::new(label).color(motif::TEXT_DIM));
    }

    /// Every planned rendez-vous still to come, day by day.
    fn agenda_upcoming(ui: &mut egui::Ui, session: &Session, open_id: &mut Option<i64>) {
        if session.appointments.is_empty() {
            ui.label(
                egui::RichText::new(tr("dash_rdv_empty"))
                    .size(11.5)
                    .color(motif::TEXT_DIM),
            );
            return;
        }
        let rect = ui.available_rect_before_wrap();
        if rect.height() < 20.0 {
            return;
        }
        let inner = motif::well(ui, rect);
        motif::inside(ui, inner, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("agenda_upcoming")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    let mut last: Option<&str> = None;
                    for rdv in &session.appointments {
                        if last != Some(rdv.date.as_str()) {
                            last = Some(rdv.date.as_str());
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
                                motif::ALERT
                            } else if rdv.date == session.today {
                                header.push_str(tr("agenda_today"));
                                motif::ACCENT
                            } else if rdv.date == session.tomorrow {
                                header.push_str(tr("agenda_tomorrow"));
                                motif::ACCENT
                            } else {
                                motif::TEXT_DIM
                            };
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(header).strong().size(11.5).color(color));
                        }
                        let hour = if rdv.time.is_empty() {
                            String::new()
                        } else {
                            format!("{}  ", rdv.time)
                        };
                        let row = format!("{hour}{}  ({})", rdv.patient_name, rdv.kind.label());
                        if motif::list_row(ui, egui::RichText::new(row), false)
                            .on_hover_text(tr("dash_open_patient"))
                            .clicked()
                        {
                            *open_id = Some(rdv.patient_id);
                        }
                    }
                });
        });
    }

    /// The week grid: Mon..Sun as columns of coloured blocks, filling
    /// `rect`. Clicking a block opens the patient, clicking a column
    /// header details that day.
    fn agenda_week_grid(
        ui: &mut egui::Ui,
        session: &mut Session,
        grid_events: &[db::Event],
        rect: egui::Rect,
        pick_day: &mut Option<String>,
        open_id: &mut Option<i64>,
    ) {
        if session.agenda_week.len() != 7 || rect.height() < 60.0 {
            return;
        }
        {
            // The grid takes the height it is given rather than the
            // height its busiest day happens to need: a week was 150 px
            // tall on a 1000 px screen, hiding entries behind a "+N"
            // with the room to show them sitting empty underneath.
            // The legend has nine chips: at a narrow grid width it needs
            // two lines, and a fixed one line dropped the last kinds.
            let legend_rows = {
                let font = egui::FontId::proportional(11.0);
                let mut x = 0.0_f32;
                let mut lines = 1.0_f32;
                for kind in InterviewKind::ALL {
                    let w = ui.fonts(|f| {
                        f.layout_no_wrap(kind.label().to_owned(), font.clone(), motif::TEXT)
                            .size()
                            .x
                    }) + 28.0;
                    if x + w > rect.width() && x > 0.0 {
                        lines += 1.0;
                        x = 0.0;
                    }
                    x += w;
                }
                lines
            };
            let legend_h = 4.0 + 18.0 * legend_rows;
            // The scale the per-day load bars are drawn against.
            let busiest = session
                .agenda_week
                .iter()
                .map(|d| {
                    session.appointments.iter().filter(|r| r.date == *d).count()
                        + grid_events.iter().filter(|e| e.day == *d).count()
                })
                .max()
                .unwrap_or(0);
            let grid = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.right(), rect.bottom() - legend_h),
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
                // A hairline under the header, filled in proportion to
                // the busiest day of the week: the week's shape is read
                // off the top of the grid without counting blocks.
                {
                    let load = session
                        .appointments
                        .iter()
                        .filter(|r| r.date == *date)
                        .count()
                        + grid_events.iter().filter(|e| e.day == *date).count();
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(col.left() + 3.0, col.top() + 21.0),
                        egui::pos2(col.right() - 3.0, col.top() + 24.0),
                    );
                    ui.painter()
                        .rect_filled(bar, 0.0, motif::BG_DARK.gamma_multiply(0.35));
                    if load > 0 && busiest > 0 {
                        let mut fill = bar;
                        fill.set_width(bar.width() * (load as f32 / busiest as f32));
                        ui.painter().rect_filled(fill, 0.0, motif::ACCENT);
                    }
                }
                // Colored blocks, one per RDV of that day.
                let day_rdvs: Vec<&Appointment> = session
                    .appointments
                    .iter()
                    .filter(|r| r.date == *date)
                    .collect();
                let max_blocks = ((col.height() - 32.0) / 24.0) as usize;
                for (bi, rdv) in day_rdvs.iter().take(max_blocks).enumerate() {
                    let block = egui::Rect::from_min_size(
                        egui::pos2(col.left() + 3.0, col.top() + 30.0 + bi as f32 * 24.0),
                        egui::vec2(col.width() - 6.0, 21.0),
                    );
                    ui.painter().rect_filled(block, 0.0, kind_color(rdv.kind));
                    // The hour leads the block when it is known.
                    let label = if rdv.time.is_empty() {
                        rdv.patient_name.clone()
                    } else {
                        format!("{} {}", rdv.time, rdv.patient_name)
                    };
                    let label = elide(ui, &label, block.width() - 8.0, 11.0);
                    ui.painter().text(
                        egui::pos2(block.left() + 4.0, block.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
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
                        *open_id = Some(rdv.patient_id);
                    }
                }
                // Entries that are not acts, in their own muted colour.
                let day_events: Vec<&db::Event> =
                    grid_events.iter().filter(|e| e.day == *date).collect();
                let used = day_rdvs.len().min(max_blocks);
                for (ei, ev) in day_events
                    .iter()
                    .take(max_blocks.saturating_sub(used))
                    .enumerate()
                {
                    let block = egui::Rect::from_min_size(
                        egui::pos2(
                            col.left() + 3.0,
                            col.top() + 30.0 + (used + ei) as f32 * 24.0,
                        ),
                        egui::vec2(col.width() - 6.0, 21.0),
                    );
                    ui.painter().rect_filled(block, 0.0, motif::BG_DARK);
                    let label = if ev.time.is_empty() {
                        ev.title.clone()
                    } else {
                        format!("{} {}", ev.time, ev.title)
                    };
                    let label = elide(ui, &label, block.width() - 8.0, 11.0);
                    ui.painter().text(
                        egui::pos2(block.left() + 4.0, block.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                    ui.interact(block, ui.id().with(("wkev", i, ei)), egui::Sense::hover())
                        .on_hover_text(format!("{} — {}", ev.category.label(), ev.title));
                }
                // Clicking the column header details that day below.
                let head = egui::Rect::from_min_size(col.min, egui::vec2(col.width(), 24.0));
                if ui
                    .interact(head, ui.id().with(("wkday", i)), egui::Sense::click())
                    .on_hover_text(tr("agenda_pick_day"))
                    .clicked()
                {
                    *pick_day = Some(date.clone());
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
            // Legend: one colored chip per act kind, on the strip left
            // under the grid.
            let legend =
                egui::Rect::from_min_max(egui::pos2(rect.left(), grid.bottom() + 4.0), rect.max);
            motif::inside(ui, legend, |ui| {
                let items: Vec<(&str, egui::Color32)> = InterviewKind::ALL
                    .iter()
                    .map(|k| (k.label(), kind_color(*k)))
                    .collect();
                motif::chart::legend(ui, &items);
            });
        }
    }

    /// Conversion tables (IPP, HBPM, statines…): selector, Motif table,
    /// numbered sources, and a printable A4 with all of them.
    /// The counter's calculators: clairance de Cockcroft, dose par
    /// kilo, and the decay of a drug once the treatment stops.
    fn calc_panel(ui: &mut egui::Ui, session: &mut Session) {
        ui.add_space(12.0);
        motif::column(ui, 940.0, |ui| {
            motif::section(ui, tr("calc_title"));
            ui.add_space(6.0);
            ui.columns(2, |cols| {
                // --- Cockcroft & Gault ---
                let ui = &mut cols[0];
                ui.label(egui::RichText::new(tr("calc_dfg")).strong().size(13.0));
                egui::Grid::new("calc_dfg")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(tr("calc_age"));
                        ui.add(egui::DragValue::new(&mut session.calc_age).range(1.0..=110.0));
                        ui.end_row();
                        ui.label(tr("calc_weight"));
                        ui.add(
                            egui::DragValue::new(&mut session.calc_weight)
                                .range(2.0..=250.0)
                                .suffix(" kg"),
                        );
                        ui.end_row();
                        ui.label(tr("calc_creat"));
                        ui.add(
                            egui::DragValue::new(&mut session.calc_creat)
                                .range(10.0..=1500.0)
                                .suffix(" µmol/L"),
                        );
                        ui.end_row();
                        ui.label(tr("calc_sex"));
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut session.calc_female, false, tr("calc_male"));
                            ui.radio_value(&mut session.calc_female, true, tr("calc_female"));
                        });
                        ui.end_row();
                    });
                let k = if session.calc_female { 1.04 } else { 1.23 };
                let clearance = if session.calc_creat > 0.0 {
                    (140.0 - session.calc_age) * session.calc_weight * k / session.calc_creat
                } else {
                    0.0
                };
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(trf("calc_dfg_result", format!("{clearance:.0}")))
                        .strong()
                        .color(motif::ACCENT),
                );
                let stage = match clearance {
                    c if c >= 90.0 => tr("calc_stage_g1"),
                    c if c >= 60.0 => tr("calc_stage_g2"),
                    c if c >= 45.0 => tr("calc_stage_g3a"),
                    c if c >= 30.0 => tr("calc_stage_g3b"),
                    c if c >= 15.0 => tr("calc_stage_g4"),
                    _ => tr("calc_stage_g5"),
                };
                ui.label(egui::RichText::new(stage).size(11.0).color(motif::TEXT_DIM));

                // --- Dose par kilo ---
                let ui = &mut cols[1];
                ui.label(egui::RichText::new(tr("calc_perkg")).strong().size(13.0));
                egui::Grid::new("calc_perkg")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(tr("calc_weight"));
                        ui.add(
                            egui::DragValue::new(&mut session.calc_weight)
                                .range(2.0..=250.0)
                                .suffix(" kg"),
                        );
                        ui.end_row();
                        ui.label(tr("calc_dose_kg"));
                        ui.add(
                            egui::DragValue::new(&mut session.calc_per_kg)
                                .range(0.1..=200.0)
                                .suffix(" mg/kg"),
                        );
                        ui.end_row();
                        ui.label(tr("calc_takes"));
                        ui.add(egui::DragValue::new(&mut session.calc_takes).range(1..=6));
                        ui.end_row();
                    });
                let per_take = session.calc_weight * session.calc_per_kg;
                let daily = per_take * session.calc_takes as f64;
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(trn(
                        "calc_perkg_result",
                        &[&format!("{per_take:.0}"), &format!("{daily:.0}")],
                    ))
                    .strong()
                    .color(motif::ACCENT),
                );
                ui.label(
                    egui::RichText::new(tr("calc_perkg_note"))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
            });

            // --- Décroissance et accumulation ---
            ui.add_space(10.0);
            ui.label(egui::RichText::new(tr("calc_halflife")).strong().size(13.0));
            ui.horizontal(|ui| {
                ui.label(tr("calc_t12"));
                ui.add(
                    egui::DragValue::new(&mut session.calc_half_life)
                        .range(0.1..=200.0)
                        .suffix(" h"),
                );
                ui.label(tr("calc_interval"));
                ui.add(
                    egui::DragValue::new(&mut session.calc_interval)
                        .range(1.0..=72.0)
                        .suffix(" h"),
                );
                // Any drug of the base whose demi-vie parses feeds the
                // curve directly.
                egui::ComboBox::from_id_salt("calc_drug")
                    .selected_text(tr("calc_from_drug"))
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for d in session
                            .drugs
                            .iter()
                            .filter(|d| parse_hours(&d.half_life).is_some())
                            .take(60)
                        {
                            if ui.selectable_label(false, &d.name).clicked() {
                                if let Some(h) = parse_hours(&d.half_life) {
                                    session.calc_half_life = h;
                                }
                            }
                        }
                    });
            });
            let t12 = session.calc_half_life.max(0.1);
            let elimination = t12 * 5.0;
            let ratio = 1.0 / (1.0 - 0.5_f64.powf(session.calc_interval.max(0.1) / t12));
            ui.label(
                egui::RichText::new(trn(
                    "calc_halflife_result",
                    &[&format!("{elimination:.0}"), &format!("{ratio:.1}")],
                ))
                .strong()
                .color(motif::ACCENT),
            );
            ui.add_space(4.0);
            // The curve: fraction remaining over five half-lives.
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width().min(880.0), 130.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 0.0, motif::TROUGH);
            motif::bevel(ui.painter(), rect, false);
            let plot = rect.shrink(10.0);
            let painter = ui.painter();
            for i in 0..=5 {
                let x = plot.left() + plot.width() * i as f32 / 5.0;
                painter.line_segment(
                    [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                    egui::Stroke::new(0.5_f32, motif::BG_DARK),
                );
                painter.text(
                    egui::pos2(x, plot.bottom() + 1.0),
                    egui::Align2::CENTER_TOP,
                    format!("{:.0} h", t12 * i as f64),
                    egui::FontId::proportional(9.0),
                    motif::TEXT_FAINT,
                );
            }
            let mut points = Vec::with_capacity(61);
            for i in 0..=60 {
                let frac = i as f64 / 60.0;
                let remaining = 0.5_f64.powf(frac * 5.0);
                points.push(egui::pos2(
                    plot.left() + plot.width() * frac as f32,
                    plot.bottom() - plot.height() * remaining as f32,
                ));
            }
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(1.6_f32, motif::ACCENT),
            ));
            for (frac, label) in [(0.5, "50 %"), (0.25, "25 %"), (0.03125, "3 %")] {
                let y = plot.bottom() - plot.height() * frac as f32;
                painter.text(
                    egui::pos2(plot.left() + 3.0, y),
                    egui::Align2::LEFT_BOTTOM,
                    label,
                    egui::FontId::proportional(9.0),
                    motif::TEXT_FAINT,
                );
            }
            ui.add_space(14.0);
            ui.label(
                egui::RichText::new(tr("calc_note"))
                    .size(11.0)
                    .italics()
                    .color(motif::TEXT_DIM),
            );
        });
    }

    /// Substitution protocols: the list, the tree editor, and the
    /// walk-through that asks the questions one at a time.
    fn protocols_view(ui: &mut egui::Ui, session: &mut Session) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            motif::column(ui, 900.0, |ui| {
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    ui.heading(tr("proto_title"));
                    if motif::button(ui, tr("patient_back")).clicked() {
                        session.show_protocols = false;
                        session.protocol_open = None;
                    }
                });
                ui.label(tr("proto_subtitle"));
                ui.add_space(10.0);
            });
            if session.protocol_open.is_none() {
                Self::protocol_list(ui, session);
            } else {
                Self::protocol_editor(ui, session);
            }
        });
    }

    fn protocol_list(ui: &mut egui::Ui, session: &mut Session) {
        let mut create = false;
        let mut open: Option<db::Protocol> = None;
        let mut delete: Option<(i64, String)> = None;
        motif::page(ui, 900.0, |ui| {
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 110.0).clamp(180.0, 460.0);
                ui.add_sized(
                    [w, 24.0],
                    egui::TextEdit::singleline(&mut session.protocol_new_title)
                        .hint_text(tr("proto_new_hint")),
                );
                if motif::button(ui, tr("proto_new")).clicked()
                    && !session.protocol_new_title.trim().is_empty()
                {
                    create = true;
                }
            });
            ui.add_space(8.0);
            if session.protocols.is_empty() {
                ui.label(
                    egui::RichText::new(tr("proto_empty"))
                        .size(12.0)
                        .color(motif::TEXT_DIM),
                );
                return;
            }
            // The same sunken list box as the patients and the drugs:
            // rows on a field of grey read as leftovers, not as a list.
            let rect = ui.available_rect_before_wrap();
            let rect =
                egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - 8.0));
            if rect.height() < 30.0 {
                return;
            }
            let inner = motif::well(ui, rect);
            motif::inside(ui, inner, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("protocols")
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for p in session.protocols.clone() {
                            let row_h = (ui.spacing().interact_size.y + 2.0).max(18.0);
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_h),
                                egui::Sense::click(),
                            );
                            if resp.hovered() {
                                ui.painter().rect_filled(rect, 0.0, motif::BG_HOVER);
                            }
                            ui.painter().text(
                                egui::pos2(rect.left() + 8.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                elide(ui, &p.title, rect.width() * 0.5, 14.0),
                                egui::FontId::proportional(14.0),
                                motif::TEXT,
                            );
                            if !p.subject.trim().is_empty() {
                                ui.painter().text(
                                    egui::pos2(rect.left() + rect.width() * 0.55, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    elide(ui, &p.subject, rect.width() * 0.3, 11.0),
                                    egui::FontId::proportional(11.0),
                                    motif::TEXT_DIM,
                                );
                            }
                            // The delete target is the row's right edge.
                            let x = egui::Rect::from_center_size(
                                egui::pos2(rect.right() - 12.0, rect.center().y),
                                egui::vec2(18.0, 18.0),
                            );
                            let hit = ui.interact(
                                x,
                                ui.id().with(("proto_del", p.id)),
                                egui::Sense::click(),
                            );
                            ui.painter().text(
                                x.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                egui::FontId::proportional(14.0),
                                if hit.hovered() {
                                    motif::ALERT
                                } else {
                                    motif::TEXT_DIM
                                },
                            );
                            if hit.clicked() {
                                delete = Some((p.id, p.title.clone()));
                            } else if resp.clicked() {
                                open = Some(p.clone());
                            }
                        }
                    });
            });
        });
        if create {
            let title = session.protocol_new_title.trim().to_owned();
            match session.db.add_protocol(&title, "") {
                Ok(_) => {
                    session.protocol_new_title.clear();
                    session.protocols = session.db.protocols().unwrap_or_default();
                }
                Err(e) => session.error = Some(e),
            }
        }
        if let Some((id, title)) = delete {
            match session.db.delete_protocol(id, &title) {
                Ok(true) => {}
                Ok(false) => session.error = Some(tr("proto_stale").to_owned()),
                Err(e) => session.error = Some(e),
            }
            session.protocols = session.db.protocols().unwrap_or_default();
        }
        if let Some(p) = open {
            session.protocol_nodes = session.db.protocol_nodes(p.id).unwrap_or_default();
            session.protocol_open = Some(p);
            session.protocol_walk = None;
            session.protocol_header = None;
        }
    }

    fn protocol_editor(ui: &mut egui::Ui, session: &mut Session) {
        let Some(proto) = session.protocol_open.clone() else {
            return;
        };
        let mut close = false;
        let mut walk = false;
        let mut print = false;
        let mut add: Option<(Option<i64>, db::Branch, db::NodeKind)> = None;
        let mut delete: Option<(i64, String)> = None;
        let mut save_edit = false;
        let mut rename: Option<(String, String)> = None;
        motif::column(ui, 900.0, |ui| {
            ui.horizontal(|ui| {
                let mut title = proto.title.clone();
                let mut subject = proto.subject.clone();
                let t = ui.add_sized([260.0, 24.0], egui::TextEdit::singleline(&mut title));
                ui.label(
                    egui::RichText::new(tr("proto_subject"))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
                let sj = ui.add_sized([200.0, 24.0], egui::TextEdit::singleline(&mut subject));
                if t.lost_focus() || sj.lost_focus() {
                    rename = Some((title, subject));
                }
                if motif::button(ui, tr("proto_back")).clicked() {
                    close = true;
                }
                let w = motif::button(
                    ui,
                    if session.protocol_walk.is_some() {
                        tr("proto_walk_stop")
                    } else {
                        tr("proto_walk")
                    },
                );
                if w.clicked() {
                    walk = true;
                }
                if motif::button(ui, tr("proto_print")).clicked() {
                    print = true;
                }
            });
            ui.add_space(8.0);
            if session.protocol_walk.is_some() {
                Self::protocol_walkthrough(ui, session);
                return;
            }
            if session.protocol_nodes.is_empty() {
                ui.label(
                    egui::RichText::new(tr("proto_no_steps"))
                        .size(12.0)
                        .color(motif::TEXT_DIM),
                );
                ui.horizontal(|ui| {
                    if motif::button(ui, tr("proto_add_question")).clicked() {
                        add = Some((None, db::Branch::Root, db::NodeKind::Question));
                    }
                    if motif::button(ui, tr("proto_add_action")).clicked() {
                        add = Some((None, db::Branch::Root, db::NodeKind::Action));
                    }
                });
                return;
            }
            // The tree, drawn depth-first with an indent per level.
            let nodes = session.protocol_nodes.clone();
            let roots: Vec<&db::ProtocolNode> =
                nodes.iter().filter(|n| n.parent_id.is_none()).collect();
            let mut stack: Vec<(&db::ProtocolNode, usize)> =
                roots.into_iter().rev().map(|n| (n, 0)).collect();
            while let Some((node, depth)) = stack.pop() {
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 22.0);
                    let tag = match node.branch {
                        db::Branch::Yes => tr("proto_branch_yes"),
                        db::Branch::No => tr("proto_branch_no"),
                        db::Branch::Root => "",
                    };
                    if !tag.is_empty() {
                        ui.label(egui::RichText::new(tag).size(11.0).color(motif::TEXT_DIM));
                    }
                    let editing = session
                        .protocol_node_edit
                        .as_ref()
                        .is_some_and(|(id, ..)| *id == node.id);
                    if editing {
                        let (_, _, text) = session.protocol_node_edit.as_mut().unwrap();
                        ui.add_sized([420.0, 22.0], egui::TextEdit::singleline(text));
                        if motif::button(ui, tr("form_save")).clicked() {
                            save_edit = true;
                        }
                    } else {
                        let label = if node.kind == db::NodeKind::Question {
                            egui::RichText::new(format!(
                                "{} ?",
                                node.text.trim_end_matches('?').trim()
                            ))
                            .strong()
                        } else {
                            egui::RichText::new(&node.text)
                        };
                        if ui.selectable_label(false, label).clicked() {
                            session.protocol_node_edit =
                                Some((node.id, node.kind, node.text.clone()));
                        }
                        if node.kind == db::NodeKind::Question {
                            if motif::button(ui, tr("proto_add_yes")).clicked() {
                                add = Some((Some(node.id), db::Branch::Yes, db::NodeKind::Action));
                            }
                            if motif::button(ui, tr("proto_add_no")).clicked() {
                                add = Some((Some(node.id), db::Branch::No, db::NodeKind::Action));
                            }
                        } else if motif::button(ui, tr("proto_add_question")).clicked() {
                            add = Some((Some(node.id), db::Branch::Root, db::NodeKind::Question));
                        }
                        if motif::button(ui, tr("itv_delete"))
                            .on_hover_text(tr("proto_delete_node"))
                            .clicked()
                        {
                            delete = Some((node.id, node.text.clone()));
                        }
                    }
                });
                let mut children: Vec<&db::ProtocolNode> = nodes
                    .iter()
                    .filter(|n| n.parent_id == Some(node.id))
                    .collect();
                children.sort_by_key(|n| (n.branch != db::Branch::Yes, n.position));
                for child in children.into_iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
        });
        let reload = |session: &mut Session| {
            session.protocol_nodes = session.db.protocol_nodes(proto.id).unwrap_or_default();
        };
        if let Some((parent, branch, kind)) = add {
            let text = if kind == db::NodeKind::Question {
                tr("proto_add_question")
            } else {
                tr("proto_add_action")
            };
            match session
                .db
                .add_protocol_node(proto.id, parent, branch, kind, text)
            {
                Ok(id) => {
                    session.protocol_node_edit = Some((id, kind, String::new()));
                    reload(session);
                }
                Err(e) => session.error = Some(e),
            }
        }
        if save_edit {
            if let Some((id, kind, text)) = session.protocol_node_edit.clone() {
                let expected = session
                    .protocol_nodes
                    .iter()
                    .find(|n| n.id == id)
                    .map(|n| n.text.clone())
                    .unwrap_or_default();
                match session
                    .db
                    .update_protocol_node(id, kind, text.trim(), &expected)
                {
                    Ok(true) => session.protocol_node_edit = None,
                    Ok(false) => session.error = Some(tr("proto_stale").to_owned()),
                    Err(e) => session.error = Some(e),
                }
                reload(session);
            }
        }
        if let Some((id, text)) = delete {
            match session.db.delete_protocol_node(proto.id, id, &text) {
                Ok(true) => {}
                Ok(false) => session.error = Some(tr("proto_stale").to_owned()),
                Err(e) => session.error = Some(e),
            }
            session.protocol_node_edit = None;
            reload(session);
        }
        if walk {
            session.protocol_walk = if session.protocol_walk.is_some() {
                None
            } else {
                session
                    .protocol_nodes
                    .iter()
                    .find(|n| n.parent_id.is_none())
                    .map(|n| n.id)
            };
        }
        if print {
            if let Err(e) =
                crate::pdf::open_protocol(&proto.title, &proto.subject, &session.protocol_nodes)
            {
                session.error = Some(e);
            }
        }
        if let Some((title, subject)) = rename {
            if title.trim() != proto.title || subject.trim() != proto.subject {
                match session.db.rename_protocol(
                    proto.id,
                    title.trim(),
                    subject.trim(),
                    &proto.title,
                ) {
                    Ok(true) => {
                        session.protocols = session.db.protocols().unwrap_or_default();
                        session.protocol_open =
                            session.protocols.iter().find(|p| p.id == proto.id).cloned();
                        session.protocol_header = None;
                    }
                    Ok(false) => {
                        session.error = Some(tr("proto_stale").to_owned());
                        session.protocols = session.db.protocols().unwrap_or_default();
                        session.protocol_header = None;
                    }
                    Err(e) => session.error = Some(e),
                }
            }
        }
        if close {
            session.protocol_open = None;
            session.protocol_node_edit = None;
            session.protocol_walk = None;
            session.protocol_header = None;
        }
    }

    /// The walk-through: one step at a time, answering the questions.
    fn protocol_walkthrough(ui: &mut egui::Ui, session: &mut Session) {
        let Some(current) = session.protocol_walk else {
            return;
        };
        let nodes = session.protocol_nodes.clone();
        let Some(node) = nodes.iter().find(|n| n.id == current) else {
            session.protocol_walk = None;
            return;
        };
        let mut go: Option<Option<i64>> = None;
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(&node.text)
                .size(16.0)
                .strong()
                .color(motif::ACCENT),
        );
        ui.add_space(10.0);
        let child = |branch: db::Branch| {
            nodes
                .iter()
                .find(|n| n.parent_id == Some(node.id) && n.branch == branch)
                .map(|n| n.id)
        };
        ui.horizontal(|ui| {
            if node.kind == db::NodeKind::Question {
                if motif::button(ui, tr("proto_walk_yes")).clicked() {
                    go = Some(child(db::Branch::Yes));
                }
                if motif::button(ui, tr("proto_walk_no")).clicked() {
                    go = Some(child(db::Branch::No));
                }
            } else if let Some(next) = child(db::Branch::Root) {
                if motif::button(ui, tr("itv_advance").replace("{}", "").trim()).clicked() {
                    go = Some(Some(next));
                }
            } else {
                ui.label(
                    egui::RichText::new(tr("proto_walk_done"))
                        .size(12.0)
                        .color(motif::TEXT_DIM),
                );
            }
        });
        if let Some(next) = go {
            match next {
                Some(id) => session.protocol_walk = Some(id),
                None => {
                    ui.label(tr("proto_walk_done"));
                    session.protocol_walk = None;
                }
            }
        }
    }

    fn tables_view(ui: &mut egui::Ui, session: &mut Session) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            Self::tables_body(ui, session);
        });
    }

    fn tables_body(ui: &mut egui::Ui, session: &mut Session) {
        if session.table_cells.is_empty() && session.table_edit.is_none() {
            // First paint (or after a view switch): load the team's
            // edits for the selected table.
            let key = crate::tables::TABLES
                [session.table_selected.min(crate::tables::TABLES.len() - 1)]
            .short;
            session.table_cells = session.db.table_cells(key).unwrap_or_default();
        }
        motif::page(ui, 1500.0, |ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.heading(tr("tables_title"));
                if motif::button(ui, tr("dash_print"))
                    .on_hover_text(tr("tables_print_tooltip"))
                    .clicked()
                {
                    let edits = session.db.all_table_cells().unwrap_or_default();
                    if let Err(e) = crate::pdf::open_conversion_tables(&edits) {
                        session.error = Some(e);
                    }
                }
                if motif::button(ui, tr("patient_back")).clicked() {
                    session.show_tables = false;
                }
                let calc = motif::button(ui, tr("tables_calc"));
                if session.calc_open {
                    motif::bevel(ui.painter(), calc.rect, false);
                }
                if calc.on_hover_text(tr("tables_calc_tooltip")).clicked() {
                    session.calc_open = !session.calc_open;
                }
            });
            ui.add_space(10.0);
            // Selector: one button per table, the active one sunken.
            // There are more tables than fit on one line — let it wrap.
            ui.horizontal_wrapped(|ui| {
                for (i, t) in crate::tables::TABLES.iter().enumerate() {
                    let btn = motif::toggle(ui, t.short, i == session.table_selected);
                    if btn.clicked() {
                        session.table_selected = i;
                        session.table_edit = None;
                        session.table_undo = None;
                        session.table_cells = session.db.table_cells(t.short).unwrap_or_default();
                    }
                }
            });
            ui.add_space(12.0);
        });
        // The calculators sit above the table, not a page below it: the
        // button that opens them is up here, and a tool you asked for
        // should not have to be scrolled to.
        if session.calc_open {
            Self::calc_panel(ui, session);
        }
        motif::page(ui, 1500.0, |ui| {
            let t =
                &crate::tables::TABLES[session.table_selected.min(crate::tables::TABLES.len() - 1)];
            ui.label(egui::RichText::new(t.title).strong().size(15.0));
            ui.add_space(6.0);
        });
        // Sunken box around the table grid, centered. Reference cells
        // are long sentences, so each column gets a fixed share of the
        // width and wraps inside it; the box is then painted behind the
        // content, once its real height is known.
        // The cells are long sentences: every pixel of width is one
        // fewer line to wrap. The old 940 px cap left a quarter of a
        // wide screen empty while the table wrapped to five lines.
        let avail = motif::visible_rect(ui);
        let avail =
            egui::Rect::from_min_max(avail.min, egui::pos2(avail.right() - 14.0, avail.bottom()));
        let t = &crate::tables::TABLES[session.table_selected.min(crate::tables::TABLES.len() - 1)];
        let w = avail.width().min(1500.0);
        const PAD: f32 = 8.0;
        const GAP: f32 = 20.0;
        let cols = t.columns.len().max(1) as f32;
        let col_w = ((w - 2.0 * PAD - GAP * (cols - 1.0)) / cols).max(80.0);
        // The cell edit committed this frame, applied after the grid.
        let mut commit: Option<(usize, usize, String)> = None;
        let bg = ui.painter().add(egui::Shape::Noop);
        let content = egui::Rect::from_min_size(
            egui::pos2(avail.center().x - w / 2.0 + PAD, avail.top() + PAD),
            egui::vec2(w - 2.0 * PAD, avail.height().max(1.0)),
        );
        let used = ui
            .allocate_new_ui(egui::UiBuilder::new().max_rect(content), |ui| {
                // A scope (not allocate_ui_with_layout) so the grid row
                // grows with a cell that wraps to several lines.
                let cell = |ui: &mut egui::Ui, text: egui::RichText| {
                    ui.scope(|ui| {
                        ui.set_max_width(col_w);
                        ui.add(egui::Label::new(text).wrap());
                    });
                };
                // A body cell: the team's value if it was edited, the
                // shipped text otherwise; click to correct it in place.
                let mut body_cell = |ui: &mut egui::Ui,
                                     session: &mut Session,
                                     r: usize,
                                     c: usize,
                                     shipped: &str| {
                    let edited = session.table_cells.get(&(r, c)).cloned();
                    if let Some((er, ec, text)) = &mut session.table_edit {
                        if *er == r && *ec == c {
                            let resp = ui.add_sized(
                                [col_w, 22.0],
                                egui::TextEdit::singleline(text).font(egui::TextStyle::Small),
                            );
                            if resp.lost_focus() {
                                commit = Some((r, c, text.clone()));
                            } else {
                                resp.request_focus();
                            }
                            return;
                        }
                    }
                    let shown = edited.clone().unwrap_or_else(|| shipped.to_owned());
                    let color = if edited.is_some() {
                        motif::ACCENT
                    } else {
                        motif::INK
                    };
                    ui.scope(|ui| {
                        ui.set_max_width(col_w);
                        let resp = ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(shown.clone()).size(13.0).color(color),
                                )
                                .wrap()
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(if edited.is_some() {
                                trf("tables_cell_edited", shipped)
                            } else {
                                tr("tables_cell_edit").to_owned()
                            });
                        if resp.clicked() {
                            session.table_edit = Some((r, c, shown));
                        }
                    });
                };
                // Striped: six columns of full sentences, and a row that
                // wraps to four lines in one column and one in the next
                // is impossible to follow across without a band behind
                // it. The band is a shade of the trough the table sits
                // in, not egui's default hover blue.
                ui.visuals_mut().faint_bg_color = egui::Color32::from_rgb(0x8b, 0x8f, 0xa1);
                egui::Grid::new(("conv_table", session.table_selected))
                    .num_columns(t.columns.len())
                    .spacing([GAP, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for c in t.columns {
                            cell(ui, egui::RichText::new(*c).strong().size(13.0));
                        }
                        ui.end_row();
                        for (ri, row) in t.rows.iter().enumerate() {
                            for (ci, c) in row.iter().enumerate() {
                                body_cell(ui, session, ri, ci, c);
                            }
                            ui.end_row();
                        }
                    });
            })
            .response
            .rect;
        let box_rect = egui::Rect::from_min_size(
            egui::pos2(avail.center().x - w / 2.0, avail.top()),
            egui::vec2(w, used.height() + 2.0 * PAD),
        );
        ui.painter()
            .set(bg, egui::Shape::rect_filled(box_rect, 0.0, motif::TROUGH));
        motif::bevel(ui.painter(), box_rect, false);
        // Drop below the box (the child ui only advanced by the grid's
        // own height), then the numbered sources on the column grid.
        let below = (box_rect.bottom() - ui.cursor().top()).max(0.0) + 10.0;
        ui.add_space(below);
        let (mut undo, mut reset) = (false, false);
        motif::page(ui, 1500.0, |ui| {
            ui.label(
                egui::RichText::new(tr("tables_sources"))
                    .size(11.0)
                    .strong()
                    .color(motif::TEXT_DIM),
            );
            for (i, src) in t.sources.iter().enumerate() {
                ui.label(
                    egui::RichText::new(format!("{}. {}", i + 1, src))
                        .size(11.0)
                        .color(motif::TEXT_DIM),
                );
            }
            // Team edits: how many, undo the last one, restore the
            // shipped table.
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if !session.table_cells.is_empty() {
                    ui.label(
                        egui::RichText::new(trf("tables_edited", session.table_cells.len()))
                            .size(11.0)
                            .color(motif::ACCENT),
                    );
                }
                if session.table_undo.is_some() && motif::button(ui, tr("tables_undo")).clicked() {
                    undo = true;
                }
                if !session.table_cells.is_empty()
                    && motif::button(ui, tr("tables_reset"))
                        .on_hover_text(tr("tables_reset_tooltip"))
                        .clicked()
                {
                    reset = true;
                }
            });
        });
        if let Some((r, c, value)) = commit {
            let shipped = t
                .rows
                .get(r)
                .and_then(|row| row.get(c))
                .copied()
                .unwrap_or("");
            let previous = session
                .table_cells
                .get(&(r, c))
                .cloned()
                .unwrap_or_else(|| shipped.to_owned());
            match session
                .db
                .set_table_cell(t.short, r, c, value.trim(), shipped, &previous)
            {
                Ok(true) => session.table_undo = Some((r, c, previous)),
                Ok(false) => session.error = Some(tr("tables_cell_stale").to_owned()),
                Err(e) => session.error = Some(e),
            }
            session.table_cells = session.db.table_cells(t.short).unwrap_or_default();
            session.table_edit = None;
        }
        if undo {
            if let Some((r, c, previous)) = session.table_undo.take() {
                let shipped = t
                    .rows
                    .get(r)
                    .and_then(|row| row.get(c))
                    .copied()
                    .unwrap_or("");
                let current = session
                    .table_cells
                    .get(&(r, c))
                    .cloned()
                    .unwrap_or_else(|| shipped.to_owned());
                if let Err(e) = session
                    .db
                    .set_table_cell(t.short, r, c, &previous, shipped, &current)
                {
                    session.error = Some(e);
                }
                session.table_cells = session.db.table_cells(t.short).unwrap_or_default();
            }
        }
        if reset {
            if let Err(e) = session.db.reset_table(t.short) {
                session.error = Some(e);
            }
            session.table_cells.clear();
            session.table_undo = None;
        }
        if let Some(err) = &session.error {
            ui.vertical_centered(|ui| {
                ui.colored_label(motif::ALERT, err.as_str());
            });
        }
    }

    /// What is in the base, before anything is typed: the classes it
    /// covers, the cards that name an antidote, and the cards the team
    /// has flagged. Returns a card to open.
    ///
    /// The search screen used to draw the whole base down the middle —
    /// the same list the left dock carries — with every row cut off at
    /// the panel edge.
    fn drug_home_panels(ui: &mut egui::Ui, session: &Session) -> Option<Drug> {
        let body = motif::visible_rect(ui).shrink2(egui::vec2(10.0, 0.0));
        if body.height() < 140.0 {
            return None;
        }
        // Classes by weight, biggest first.
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for d in &session.drugs {
            let c = d.class.trim();
            if !c.is_empty() {
                *counts.entry(c).or_default() += 1;
            }
        }
        let mut classes: Vec<(&str, usize)> = counts.into_iter().collect();
        classes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        classes.truncate(12);

        let mut open: Option<Drug> = None;
        let split = body.width() >= 760.0;
        let rects: [egui::Rect; 3] = if split {
            let rows = motif::split_rows(body, &[body.height() * 0.55, 0.0], 8.0);
            let top = motif::split_columns(rows[0], 2, 8.0);
            [top[0], top[1], rows[1]]
        } else {
            let rows = motif::split_rows(body, &[0.0, 0.0, 0.0], 8.0);
            [rows[0], rows[1], rows[2]]
        };

        motif::panel(ui, rects[0], Some(tr("drug_home_classes")), |ui| {
            let rect = ui.max_rect();
            if classes.is_empty() {
                return;
            }
            let rows: Vec<motif::chart::Row> = classes
                .iter()
                .map(|(name, n)| motif::chart::Row {
                    label: name,
                    value: *n as f64,
                    color: motif::ACCENT,
                })
                .collect();
            if let Some(i) = motif::chart::hbars(ui, rect, &rows, 200.0, &|v| format!("{v:.0}")) {
                egui::show_tooltip_text(
                    ui.ctx(),
                    ui.layer_id(),
                    ui.id().with("drug_class_tip"),
                    trn("drug_home_class_tip", &[&classes[i].1, &classes[i].0]),
                );
            }
        });

        // Antidotes: the one lookup nobody wants to be searching for.
        let antidotes: Vec<&Drug> = session
            .drugs
            .iter()
            .filter(|d| !d.antidote.trim().is_empty())
            .collect();
        motif::panel(ui, rects[1], Some(tr("drug_home_antidotes")), |ui| {
            let rect = ui.max_rect();
            if antidotes.is_empty() {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tr("drug_home_none"),
                    egui::FontId::proportional(12.0),
                    motif::TEXT_DIM,
                );
                return;
            }
            let inner = motif::well(ui, rect);
            motif::inside(ui, inner, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("drug_antidotes")
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for d in &antidotes {
                            let text = format!("{}   —   {}", d.name.trim(), d.antidote.trim());
                            if motif::list_row(ui, egui::RichText::new(text), false).clicked() {
                                open = Some((*d).clone());
                            }
                        }
                    });
            });
        });

        // Anything the team has flagged: retiré, hors AMM, rupture.
        let flagged: Vec<&Drug> = session
            .drugs
            .iter()
            .filter(|d| !d.status.trim().is_empty())
            .collect();
        motif::panel(ui, rects[2], Some(tr("drug_home_status")), |ui| {
            let rect = ui.max_rect();
            if flagged.is_empty() {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tr("drug_home_none"),
                    egui::FontId::proportional(12.0),
                    motif::TEXT_DIM,
                );
                return;
            }
            let inner = motif::well(ui, rect);
            motif::inside(ui, inner, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("drug_flagged")
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for d in &flagged {
                            let row_h = (ui.spacing().interact_size.y + 2.0).max(18.0);
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_h),
                                egui::Sense::click(),
                            );
                            if resp.hovered() {
                                ui.painter().rect_filled(rect, 0.0, motif::BG_HOVER);
                            }
                            // The status as its own coloured chip, so a
                            // withdrawn card is spotted, not read.
                            let chip_w = 176.0_f32.min(rect.width() * 0.45);
                            let chip = egui::Rect::from_min_size(
                                egui::pos2(rect.left() + 6.0, rect.center().y - 8.0),
                                egui::vec2(chip_w, 16.0),
                            );
                            ui.painter().rect_filled(chip, 0.0, status_color(&d.status));
                            ui.painter().text(
                                chip.center(),
                                egui::Align2::CENTER_CENTER,
                                elide(ui, d.status.trim(), chip_w - 8.0, 10.5),
                                egui::FontId::proportional(10.5),
                                egui::Color32::WHITE,
                            );
                            ui.painter().text(
                                egui::pos2(chip.right() + 10.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                elide(
                                    ui,
                                    d.name.trim(),
                                    (rect.right() - chip.right() - 18.0).max(20.0),
                                    13.0,
                                ),
                                egui::FontId::proportional(13.0),
                                motif::TEXT,
                            );
                            if resp.clicked() {
                                open = Some((*d).clone());
                            }
                        }
                    });
            });
        });
        ui.allocate_space(egui::vec2(body.width(), body.height()));
        open
    }

    /// The open drug card's dated notes journal. Shared by the card's
    /// side column and, on a narrow window, the foot of the monograph.
    fn drug_notes_pane(
        ui: &mut egui::Ui,
        session: &mut Session,
        drug_id: i64,
        operator: &str,
        height: f32,
    ) {
        let (add, delete) = notes_box(
            ui,
            "drug_notes",
            &session.drug_notes,
            &mut session.note_text,
            &mut session.note_confirm,
            height,
            true,
        );
        if let Some(body) = add {
            if let Err(e) = session
                .db
                .add_note(NoteSubject::Drug, drug_id, operator, &body)
            {
                session.error = Some(e);
            }
            session.note_text.clear();
            session.drug_notes = session
                .db
                .notes_for(NoteSubject::Drug, drug_id)
                .unwrap_or_default();
        }
        if let Some(id) = delete {
            if let Err(e) = session.db.delete_note(id) {
                session.error = Some(e);
            }
            session.drug_notes = session
                .db
                .notes_for(NoteSubject::Drug, drug_id)
                .unwrap_or_default();
        }
    }

    /// Reverse lookup: who is on this drug (recalls, alerts). A real
    /// list rather than a clipped row of chips — on a shortage or a
    /// withdrawal this is the call list, and all of it matters.
    fn drug_patients_pane(ui: &mut egui::Ui, session: &Session) -> Option<i64> {
        let mut open = None;
        let rect = ui.available_rect_before_wrap();
        if rect.height() < 20.0 {
            return None;
        }
        let inner = motif::well(ui, rect);
        motif::inside(ui, inner, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("drug_patients")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for p in &session.drug_patients {
                        if motif::list_row(ui, egui::RichText::new(p.full_name()), false)
                            .on_hover_text(tr("dash_open_patient"))
                            .clicked()
                        {
                            open = Some(p.id);
                        }
                    }
                });
        });
        open
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
        operator: &str,
    ) {
        let (doc_text, doc_dirty, doc_last_edit) = doc;
        // Escape closes the card or the tables first, then the view.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !ctx.wants_keyboard_input() {
            if session.drug_form.is_some() {
                session.drug_form = None;
                session.drug_base = None;
                session.confirm_delete_drug = false;
            } else if session.show_protocols {
                if session.protocol_open.is_some() {
                    session.protocol_open = None;
                } else {
                    session.show_protocols = false;
                }
            } else if session.show_tables {
                session.show_tables = false;
            } else {
                session.view = MainView::Search;
                return;
            }
        }

        if session.show_tables {
            Self::tables_view(ui, session);
            return;
        }
        if session.show_protocols {
            Self::protocols_view(ui, session);
            return;
        }

        // The title block belongs to the base's index, not to an open
        // card: the tab already says which drug is on screen, and the
        // card needs every pixel of height it can get.
        if session.drug_form.is_none() {
            motif::page(ui, 720.0, |ui| {
                ui.add_space(24.0);
                // The two side doors go hard right on a wide page and
                // drop to their own line on a narrow one, where they
                // used to be drawn over the title.
                let roomy = ui.available_width() >= 620.0;
                let doors = |ui: &mut egui::Ui, session: &mut Session| {
                    if motif::button(ui, tr("tables_button")).clicked() {
                        session.show_tables = true;
                    }
                    if motif::button(ui, tr("proto_button"))
                        .on_hover_text(tr("proto_button_tooltip"))
                        .clicked()
                    {
                        session.show_protocols = true;
                        session.protocols = session.db.protocols().unwrap_or_default();
                    }
                };
                ui.horizontal(|ui| {
                    ui.heading(tr("drug_title"));
                    if roomy {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            doors(ui, session);
                        });
                    }
                });
                if !roomy {
                    ui.horizontal_wrapped(|ui| doors(ui, session));
                }
                ui.label(tr("drug_subtitle"));
                // A base that predates the starter list (or was started by
                // hand) shows almost nothing — point at the one-click fix.
                if session.drugs.len() < db::STARTER_DRUG_COUNT / 4 {
                    ui.label(
                        egui::RichText::new(trf("drug_base_sparse", session.drugs.len()))
                            .size(11.0)
                            .color(motif::ALERT),
                    );
                }
                ui.add_space(12.0);
            });
        }

        if let Some(form) = &mut session.drug_form {
            // ---- Card: monograph to read, or the editable form ----
            let reading = session.drug_reading;
            let card = ui.available_rect_before_wrap().shrink(6.0);
            motif::bevel(ui.painter(), card, true);
            let mut save = false;
            let mut close = false;
            let mut delete = false;
            let mut insert_note = false;
            let mut edit = false;
            let mut edit_class = false;
            let mut lookup = false;
            let mut poso_add: Option<i64> = None;
            let mut poso_save = false;
            let mut poso_start_edit: Option<db::Posologie> = None;
            let mut poso_delete: Option<(i64, String)> = None;
            let mut print_mono = false;
            let mut open_patient_id: Option<i64> = None;
            // The actions stay above the scroll: a full monograph is
            // several screens tall, and « Modifier » or « Enregistrer »
            // must never be something to go looking for.
            motif::page(ui, 1400.0, |ui| {
                ui.add_space(8.0);
                // Wrapped: a narrow window, or an open side pane, must
                // push the last actions onto a second line rather than
                // cut them off.
                ui.horizontal_wrapped(|ui| {
                    if reading {
                        if motif::button(ui, tr("drug_edit")).clicked() {
                            edit = true;
                        }
                    } else if motif::button(ui, tr("form_save")).clicked() {
                        save = true;
                    }
                    if motif::button(ui, tr("drug_close")).clicked() {
                        close = true;
                    }
                    if reading {
                        if !form.class.trim().is_empty()
                            && motif::button(ui, tr("drug_class_edit"))
                                .on_hover_text(tr("drug_class_edit_tooltip"))
                                .clicked()
                        {
                            edit_class = true;
                        }
                        if motif::button(ui, tr("drug_lookup"))
                            .on_hover_text(tr("drug_lookup_tooltip"))
                            .clicked()
                        {
                            lookup = true;
                        }
                        if motif::button(ui, tr("drug_print"))
                            .on_hover_text(tr("drug_print_tooltip"))
                            .clicked()
                        {
                            print_mono = true;
                        }
                    }
                    if motif::button(ui, tr("drug_to_notes"))
                        .on_hover_text(tr("drug_to_notes_tooltip"))
                        .clicked()
                    {
                        insert_note = true;
                    }
                    // Destructive action last, after a visible gap.
                    ui.add_space(12.0);
                    let del_label = if session.confirm_delete_drug {
                        tr("patient_delete_confirm")
                    } else {
                        tr("patient_delete")
                    };
                    if motif::button(ui, del_label).clicked() {
                        delete = true;
                    }
                });
                ui.add_space(6.0);
                let sep = ui.available_rect_before_wrap();
                let line = egui::Rect::from_min_size(sep.min, egui::vec2(sep.width(), 2.0));
                motif::bevel(ui.painter(), line, false);
                ui.add_space(4.0);
            });
            // The monograph is a document, so it keeps a reading
            // measure; the width left over goes to the card's side
            // matter — who is on this drug, and what the team wrote
            // about it — instead of to grey gutters.
            let body = motif::visible_rect(ui);
            // Wide enough for a column beside the monograph? Otherwise
            // the side matter becomes a band under it. Either way it is
            // outside the monograph's scroll: the journal and the recall
            // list were at the foot of a document several screens long,
            // which is the same as not being there.
            // The card's body scrolls, so its right edge belongs to the
            // scrollbar: content laid out to the panel edge ends up
            // underneath it.
            let body =
                egui::Rect::from_min_max(body.min, egui::pos2(body.right() - 14.0, body.bottom()));
            let wide = body.width() >= 1180.0;
            let (main, side_rect) = if wide {
                let side_w = (body.width() * 0.28).clamp(260.0, 380.0);
                (
                    egui::Rect::from_min_max(
                        body.min,
                        egui::pos2(body.right() - side_w - 10.0, body.bottom() - 6.0),
                    ),
                    egui::Rect::from_min_max(
                        egui::pos2(body.right() - side_w, body.top()),
                        egui::pos2(body.right(), body.bottom() - 6.0),
                    ),
                )
            } else {
                let band = (body.height() * 0.26).clamp(140.0, 220.0);
                let rows = motif::split_rows(body.shrink2(egui::vec2(0.0, 3.0)), &[0.0, band], 8.0);
                (rows[0], rows[1])
            };
            let card_id = form.id;
            motif::inside(ui, main, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("drug_card")
                    // The monograph keeps a reading measure, so its
                    // content is narrower than the card. Left to shrink,
                    // the scroll area takes the content's width and puts
                    // its scrollbar down the middle of the sheet instead
                    // of against the card's edge.
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        motif::page(ui, 900.0, |ui| {
                            ui.add_space(18.0);
                            if reading {
                                drug_monograph(ui, form, &session.class_note, &session.posologies);
                            }
                            if !reading {
                                ui.vertical_centered(|ui| {
                                    // Identity header: brand name big, DCI underneath.
                                    ui.heading(if form.name.trim().is_empty() {
                                        tr("drug_unnamed")
                                    } else {
                                        form.name.trim()
                                    });
                                    {
                                        let mut sub = form.dci.trim().to_owned();
                                        if !form.class.trim().is_empty() {
                                            if !sub.is_empty() {
                                                sub.push_str(" — ");
                                            }
                                            sub.push_str(form.class.trim());
                                        }
                                        if !sub.is_empty() {
                                            ui.label(
                                                egui::RichText::new(sub)
                                                    .italics()
                                                    .color(motif::TEXT_DIM),
                                            );
                                        }
                                    }
                                    if !form.antidote.trim().is_empty() {
                                        ui.label(
                                            egui::RichText::new(trf(
                                                "drug_antidote_banner",
                                                form.antidote.trim(),
                                            ))
                                            .strong()
                                            .color(motif::ALERT),
                                        );
                                    }
                                });
                                ui.add_space(12.0);
                                // Two columns when there is room for them; one
                                // underneath the other when the window is narrow
                                // or the side pane is open, where two columns
                                // left five words to a line.
                                if ui.available_width() >= 720.0 {
                                    ui.columns(2, |cols| {
                                        drug_form_clinical(&mut cols[0], form);
                                        drug_form_pk(&mut cols[1], form);
                                    });
                                } else {
                                    drug_form_clinical(ui, form);
                                    ui.add_space(12.0);
                                    drug_form_pk(ui, form);
                                }
                            }
                            if !reading {
                                // Posologies by indication, editable line by line.
                                let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
                                ui.add_space(8.0);
                                motif::section(ui, tr("drug_sec_poso"));
                                ui.add_space(4.0);
                                // The three text columns share whatever is left
                                // after the row's buttons: hard-coded widths
                                // pushed « Remarque » off the card as soon as a
                                // dock was open.
                                let poso_w = (ui.available_width() - 190.0).max(240.0);
                                let poso_drug = form.id;
                                egui::Grid::new("poso_edit")
                                    .num_columns(4)
                                    .spacing([8.0, 5.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("poso_indication")));
                                        ui.label(dim(tr("poso_dose")));
                                        ui.label(dim(tr("poso_remark")));
                                        ui.label("");
                                        ui.end_row();
                                        for p in session.posologies.clone() {
                                            let editing = session
                                                .poso_edit
                                                .as_ref()
                                                .is_some_and(|e| e.id == p.id);
                                            if editing {
                                                let e = session.poso_edit.as_mut().unwrap();
                                                ui.add_sized(
                                                    [poso_w * 0.3, 22.0],
                                                    egui::TextEdit::singleline(&mut e.indication),
                                                );
                                                ui.add_sized(
                                                    [poso_w * 0.38, 22.0],
                                                    egui::TextEdit::singleline(&mut e.posologie),
                                                );
                                                ui.add_sized(
                                                    [poso_w * 0.32, 22.0],
                                                    egui::TextEdit::singleline(&mut e.remarque),
                                                );
                                                if motif::button(ui, tr("form_save")).clicked() {
                                                    poso_save = true;
                                                }
                                            } else {
                                                ui.label(&p.indication);
                                                ui.label(&p.posologie);
                                                ui.label(
                                                    egui::RichText::new(&p.remarque)
                                                        .size(11.0)
                                                        .color(motif::TEXT_DIM),
                                                );
                                                ui.horizontal(|ui| {
                                                    if motif::button(ui, tr("drug_edit")).clicked()
                                                    {
                                                        poso_start_edit = Some(p.clone());
                                                    }
                                                    if motif::button(ui, tr("itv_delete")).clicked()
                                                    {
                                                        poso_delete =
                                                            Some((p.id, p.indication.clone()));
                                                    }
                                                });
                                            }
                                            ui.end_row();
                                        }
                                        ui.add_sized(
                                            [poso_w * 0.3, 22.0],
                                            egui::TextEdit::singleline(&mut session.poso_new.0)
                                                .hint_text(tr("poso_indication")),
                                        );
                                        ui.add_sized(
                                            [poso_w * 0.38, 22.0],
                                            egui::TextEdit::singleline(&mut session.poso_new.1)
                                                .hint_text(tr("poso_dose")),
                                        );
                                        ui.add_sized(
                                            [poso_w * 0.32, 22.0],
                                            egui::TextEdit::singleline(&mut session.poso_new.2)
                                                .hint_text(tr("poso_remark")),
                                        );
                                        if motif::button(ui, tr("notes_add")).clicked()
                                            && !session.poso_new.0.trim().is_empty()
                                        {
                                            poso_add = Some(poso_drug);
                                        }
                                        ui.end_row();
                                    });
                            }
                            if let Some(err) = &session.error {
                                ui.add_space(6.0);
                                ui.colored_label(motif::ALERT, err.as_str());
                            }
                        });
                    });
            });
            // Stacked beside the monograph, or side by side under it.
            let (recalls, journal) = if wide {
                let rows = motif::split_rows(side_rect, &[0.0, 0.0], 8.0);
                (rows[0], rows[1])
            } else {
                let cols = motif::split_columns(side_rect, 2, 8.0);
                (cols[0], cols[1])
            };
            motif::panel(ui, recalls, Some(tr("drug_patients_label")), |ui| {
                if session.drug_patients.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("drug_patients_none"))
                            .size(11.5)
                            .color(motif::TEXT_DIM),
                    );
                } else if let Some(id) = Self::drug_patients_pane(ui, session) {
                    open_patient_id = Some(id);
                }
            });
            motif::panel(ui, journal, Some(tr("drug_notes_section")), |ui| {
                let h = 420.0;
                Self::drug_notes_pane(ui, session, card_id, operator, h);
            });

            if let Some(text) = session.class_note_edit.clone() {
                let class = session
                    .drug_form
                    .as_ref()
                    .map(|d| d.class.trim().to_owned())
                    .unwrap_or_default();
                let mut buffer = text;
                let (mut save_note, mut close_note) = (false, false);
                egui::Window::new(trf("drug_class_note", &class))
                    .collapsible(false)
                    .resizable(true)
                    .default_size([520.0, 240.0])
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(tr("drug_class_note_hint"))
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        ui.add_space(6.0);
                        ui.add_sized(
                            [500.0, 150.0],
                            egui::TextEdit::multiline(&mut buffer).desired_rows(8),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if motif::button(ui, tr("form_save")).clicked() {
                                save_note = true;
                            }
                            if motif::button(ui, tr("tpl_close")).clicked() {
                                close_note = true;
                            }
                        });
                    });
                if save_note {
                    let expected = session.class_note.clone();
                    match session.db.set_class_note(&class, &buffer, &expected) {
                        Ok(true) => {
                            session.class_note = buffer.trim().to_owned();
                            session.class_note_edit = None;
                        }
                        Ok(false) => {
                            session.class_note = session.db.class_note(&class).unwrap_or_default();
                            session.error = Some(tr("drug_class_stale").to_owned());
                        }
                        Err(e) => session.error = Some(e),
                    }
                } else if close_note {
                    session.class_note_edit = None;
                } else {
                    session.class_note_edit = Some(buffer);
                }
            }
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
            if edit {
                // Leaving the monograph for the form: the loaded card is
                // the compare-and-set baseline, as everywhere else.
                session.drug_reading = false;
                session.error = None;
            }
            if let Some(drug_id) = poso_add {
                let (i, d, r) = session.poso_new.clone();
                match session
                    .db
                    .add_posologie(drug_id, i.trim(), d.trim(), r.trim())
                {
                    Ok(_) => {
                        session.poso_new = (String::new(), String::new(), String::new());
                        session.posologies = session.db.posologies(drug_id).unwrap_or_default();
                    }
                    Err(e) => session.error = Some(e),
                }
            }
            if let Some(p) = poso_start_edit {
                session.poso_edit = Some(p);
            }
            if poso_save {
                if let Some(edited) = session.poso_edit.clone() {
                    let expected = session
                        .posologies
                        .iter()
                        .find(|p| p.id == edited.id)
                        .map(|p| p.indication.clone())
                        .unwrap_or_default();
                    let drug_id = session.drug_form.as_ref().map(|d| d.id).unwrap_or(0);
                    match session.db.update_posologie(edited.id, &edited, &expected) {
                        Ok(true) => session.poso_edit = None,
                        Ok(false) => session.error = Some(tr("drug_stale").to_owned()),
                        Err(e) => session.error = Some(e),
                    }
                    session.posologies = session.db.posologies(drug_id).unwrap_or_default();
                }
            }
            if let Some((id, indication)) = poso_delete {
                let drug_id = session.drug_form.as_ref().map(|d| d.id).unwrap_or(0);
                match session.db.delete_posologie(id, &indication) {
                    Ok(true) => {}
                    Ok(false) => session.error = Some(tr("drug_stale").to_owned()),
                    Err(e) => session.error = Some(e),
                }
                session.posologies = session.db.posologies(drug_id).unwrap_or_default();
            }
            if edit_class {
                session.class_note_edit = Some(session.class_note.clone());
            }
            if lookup {
                // The app stays offline: the query is handed to the
                // browser, on the public ANSM database.
                let query = session
                    .drug_form
                    .as_ref()
                    .map(|d| {
                        if d.dci.trim().is_empty() {
                            d.name.trim().to_owned()
                        } else {
                            format!("{} {}", d.name.trim(), d.dci.trim())
                        }
                    })
                    .unwrap_or_default();
                let url = format!(
                    "https://base-donnees-publique.medicaments.gouv.fr/index.php#result:{}",
                    urlencode(&query)
                );
                if let Err(e) = open::that(&url) {
                    session.error = Some(trf("drug_lookup_error", e));
                }
            }
            if print_mono {
                if let Some(card) = session.drug_form.clone() {
                    if let Err(e) = crate::pdf::open_drug_monograph(&card, &session.posologies) {
                        session.error = Some(e);
                    }
                }
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
                            session.drug_reading = true;
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
            if let Some(id) = open_patient_id {
                if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                    session.view = MainView::Search;
                    session.open_patient(p);
                }
            }
            return;
        }

        // ---- Search / list ----
        let mut open_drug: Option<Drug> = None;
        let idle = session.drug_query.trim().is_empty();
        motif::page(ui, 720.0, |ui| {
            let search = ui.add_sized(
                [ui.available_width(), 32.0],
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

            let results: Vec<Drug> = session.drug_results(40);

            // An empty query matches the whole base, and the left dock
            // is already its index: the middle of the screen is better
            // spent saying what is in the base.
            if !results.is_empty() && !idle {
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
                // Sunken Motif list box with full-width rows.
                let avail = ui.available_rect_before_wrap();
                let w = avail.width().min(620.0);
                let h = (avail.height() - 14.0).max(140.0);
                let box_rect = egui::Rect::from_min_size(
                    egui::pos2(avail.center().x - w / 2.0, avail.top()),
                    egui::vec2(w, h),
                );
                ui.painter().rect_filled(box_rect, 0.0, motif::TROUGH);
                motif::bevel(ui.painter(), box_rect, false);
                let builder = egui::UiBuilder::new().max_rect(box_rect.shrink(4.0));
                ui.allocate_new_ui(builder, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, d) in results.iter().enumerate() {
                            let mut text = d.name.clone();
                            if !d.dci.is_empty() {
                                text.push_str(&format!(" ({})", d.dci));
                            }
                            if !d.class.is_empty() {
                                text.push_str(&format!("   ·  {}", d.class));
                            }
                            if !d.dosage.is_empty() {
                                text.push_str(&format!("   ·  {}", d.dosage));
                            }
                            if !d.antidote.is_empty() {
                                text.push_str(&trf("drug_row_antidote", &d.antidote));
                            }
                            let row = motif::list_row(
                                ui,
                                egui::RichText::new(text),
                                i == session.drug_selected,
                            );
                            if row.clicked() {
                                open_drug = Some(d.clone());
                            }
                        }
                    });
                });
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
                    ui.colored_label(motif::ALERT, err.as_str());
                }
            }
        });
        if idle {
            if let Some(d) = Self::drug_home_panels(ui, session) {
                open_drug = Some(d);
            }
        }
        if let Some(d) = open_drug {
            session.open_drug_card(d);
            session.error = None;
        }
    }

    /// A raised KPI tile: caption, big figure, and an optional trend
    /// sparkline under it. `rect` is carved by the caller, so a row of
    /// tiles shares the width evenly instead of each guessing at it.
    fn kpi_tile(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        title: &str,
        value: &str,
        trend: &[f64],
        note: Option<&str>,
    ) {
        ui.painter().rect_filled(rect, 0.0, motif::BG);
        motif::bevel(ui.painter(), rect, true);
        // The figure is what the eye should land on: the caption above
        // it is small, spaced and quiet, the number large and accented.
        let caption: String = title
            .to_uppercase()
            .chars()
            .flat_map(|c| [c, '\u{2009}'])
            .collect();
        ui.painter().text(
            egui::pos2(rect.left() + 12.0, rect.top() + 13.0),
            egui::Align2::LEFT_CENTER,
            caption.trim_end(),
            egui::FontId::proportional(10.0),
            motif::TEXT_DIM,
        );
        motif::rule(
            ui.painter(),
            rect.left() + 12.0,
            rect.right() - 12.0,
            rect.top() + 22.0,
        );
        // The figure shrinks rather than overflowing its tile: a five
        // figure revenue must stay inside the bevel.
        let mut size = 24.0_f32;
        let fits = |size: f32| {
            ui.fonts(|f| {
                f.layout_no_wrap(
                    value.to_owned(),
                    egui::FontId::proportional(size),
                    motif::ACCENT,
                )
                .size()
                .x
            }) <= rect.width() - 24.0
        };
        while size > 12.0 && !fits(size) {
            size -= 1.0;
        }
        ui.painter().text(
            egui::pos2(rect.left() + 12.0, rect.top() + 44.0),
            egui::Align2::LEFT_CENTER,
            value,
            egui::FontId::proportional(size),
            motif::ACCENT,
        );
        if let Some(note) = note {
            ui.painter().text(
                egui::pos2(rect.right() - 12.0, rect.top() + 46.0),
                egui::Align2::RIGHT_CENTER,
                elide(ui, note, rect.width() * 0.45, 10.5),
                egui::FontId::proportional(10.5),
                motif::TEXT_FAINT,
            );
        }
        // The trend says whether the figure is going anywhere — the one
        // thing a bare number can never say.
        if trend.len() >= 2 && rect.height() > 70.0 {
            let strip = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 12.0, rect.bottom() - 26.0),
                egui::pos2(rect.right() - 12.0, rect.bottom() - 8.0),
            );
            motif::chart::sparkline(ui, strip, trend, motif::ACCENT);
        }
    }

    /// The "Exporter CSV" button with its status line: writes every
    /// interview (with fees) to `exports/` next to the database and
    /// opens the file, for billing reconciliation with the LGO.
    fn export_controls(ui: &mut egui::Ui, session: &mut Session, config: &Config) {
        // Right-to-left inside the header bar: the buttons hug the
        // corner instead of stacking down the middle of the page.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if motif::button(ui, tr("dash_export")).clicked() {
                match session.db.export_rows(config.rules.cycle_months.max(1)) {
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
            // The paper companion of the export: the acts to invoice,
            // with the code, the step and the amount the memo sets.
            if motif::button(ui, tr("dash_billing_recap"))
                .on_hover_text(tr("dash_billing_recap_tooltip"))
                .clicked()
            {
                match session.db.export_rows(config.rules.cycle_months.max(1)) {
                    Ok(rows) => {
                        let today = if session.today.is_empty() {
                            session.db.today_iso().unwrap_or_default()
                        } else {
                            session.today.clone()
                        };
                        let lines = billing_lines(&rows, config);
                        if lines.is_empty() {
                            session.export_notice = Some(tr("dash_billing_none").to_owned());
                        } else if let Err(e) = crate::pdf::open_billing_recap(
                            &lines,
                            tr("dash_billing_period"),
                            &db::format_french_date(&today),
                        ) {
                            session.error = Some(e);
                        }
                    }
                    Err(e) => session.error = Some(e),
                }
            }
            if let Some(notice) = &session.export_notice {
                ui.label(
                    egui::RichText::new(elide(ui, notice, 260.0, 10.5))
                        .size(10.5)
                        .color(motif::TEXT_FAINT),
                )
                .on_hover_text(notice.as_str());
            }
        });
    }

    /// Financial & statistical dashboard (spec 3.3).
    ///
    /// Laid out as a grid of panels that reflows with the window rather
    /// than a single centred column: the counter's screen is wide, and
    /// the figures it has to hold — revenue, pipeline, act mix, the
    /// week's load, what the team wrote — belong side by side, not
    /// stacked three screens deep.
    fn dashboard_view(ui: &mut egui::Ui, session: &mut Session, config: &Config) {
        // ---- Header: title, the discreet-mode switch, the exports ----
        let masked = config.ui.discreet_finances && !session.show_amounts;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.heading(tr("dash_title"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if config.ui.discreet_finances {
                    // Deliberately unobtrusive: a small unlabeled square,
                    // raised while masked, sunken while shown.
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(30.0, 20.0), egui::Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        if masked { motif::BG } else { motif::TROUGH },
                    );
                    motif::bevel(ui.painter(), rect, masked);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "•••",
                        egui::FontId::proportional(10.0),
                        motif::TEXT_DIM,
                    );
                    if resp.on_hover_text(tr("dash_discreet_tooltip")).clicked() {
                        session.show_amounts = !session.show_amounts;
                    }
                }
                Self::export_controls(ui, session, config);
            });
        });
        ui.add_space(6.0);

        // ---- The figures every panel below reads from ----
        let money = |v: f64| {
            if masked {
                "•••".to_owned()
            } else {
                format!("{v:.0} €")
            }
        };
        let total = |rows: &dyn Fn(&InterviewSummary) -> bool| -> f64 {
            session
                .summaries
                .iter()
                .filter(|s| rows(s))
                .map(|s| config.act_total(s.kind, s.fee_year, s.fee_rank, s.remote))
                .sum()
        };
        let billed = total(&|s| s.state == InterviewState::Billed);
        let pending = total(&|s| s.state != InterviewState::Billed);
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
            let v = billed / (billed_minutes as f64 / 60.0);
            if masked {
                "•••".to_owned()
            } else {
                format!("{v:.0} €/h")
            }
        } else {
            "— €/h".to_owned()
        };

        // Revenue per month, billed against pending, oldest first.
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
        let per_month: Vec<(String, f64, f64)> = months
            .iter()
            .map(|m| {
                let b = total(&|s| s.state == InterviewState::Billed && &s.updated_month == m);
                let p = total(&|s| s.state != InterviewState::Billed && &s.created_month == m);
                (m.clone(), b, p)
            })
            .collect();
        let trend: Vec<f64> = if masked {
            Vec::new()
        } else {
            per_month.iter().map(|(_, b, _)| *b).collect()
        };

        egui::ScrollArea::vertical()
            .id_salt("dashboard")
            .show(ui, |ui| {
                let full = motif::visible_rect(ui).shrink2(egui::vec2(4.0, 0.0));
                let w = full.width();
                let gutter = 8.0;

                // ---- KPI row: four across, two-up on a narrow window ----
                let per_row = if w >= 720.0 { 4 } else { 2 };
                let tile_h = if trend.len() >= 2 { 96.0 } else { 72.0 };
                let rows = 4_usize.div_ceil(per_row);
                let kpi_rect = egui::Rect::from_min_size(
                    full.min,
                    egui::vec2(w, rows as f32 * (tile_h + gutter) - gutter),
                );
                let tiles: [(&str, String, &[f64], Option<String>); 4] = [
                    (
                        tr("dash_billed"),
                        money(billed),
                        &trend[..],
                        Some(trf("dash_acts_n", billed_count)),
                    ),
                    (tr("dash_pending"), money(pending), &[], None),
                    (
                        tr("dash_billed_count"),
                        billed_count.to_string(),
                        &[],
                        Some(trf("dash_of_n", session.summaries.len())),
                    ),
                    (
                        tr("dash_hourly"),
                        roi.clone(),
                        &[],
                        Some(trf("dash_minutes", billed_minutes)),
                    ),
                ];
                motif::inside(ui, kpi_rect, |ui| {
                    for r in 0..rows {
                        let band = egui::Rect::from_min_size(
                            egui::pos2(
                                kpi_rect.left(),
                                kpi_rect.top() + r as f32 * (tile_h + gutter),
                            ),
                            egui::vec2(w, tile_h),
                        );
                        for (i, cell) in motif::split_columns(band, per_row, gutter)
                            .into_iter()
                            .enumerate()
                        {
                            let Some((title, value, trend, note)) = tiles.get(r * per_row + i)
                            else {
                                break;
                            };
                            Self::kpi_tile(ui, cell, title, value, trend, note.as_deref());
                        }
                    }
                });
                ui.add_space(gutter);

                // ---- The panel grid ----
                // 340 px is the narrowest a panel of bars stays readable at; at
                // 400 a 1280 px screen with both docks open fell to one lane
                // and stretched a one-bar chart to 350 px tall.
                let cols = motif::column_count(w, 340.0, 2);
                let mut open_patient: Option<i64> = None;
                let mut open_recent: Option<Patient> = None;

                // Each entry is (title, height, painter). They are dealt
                // into the columns in order, so a one-column window
                // simply stacks them in the same reading order.
                let mut panels: Vec<(&str, f32)> = vec![
                    (tr("dash_pipeline"), 172.0),
                    (tr("dash_monthly"), 232.0),
                    (tr("dash_per_kind"), 232.0),
                    (tr("dash_rdv"), 232.0),
                    (tr("dash_recent"), 190.0),
                    (tr("dash_today_notes"), 190.0),
                ];
                // On a tall screen the natural grid stopped short and
                // left a band of grey under it; stretch the panels to
                // fill what is there, within reason — a funnel of five
                // bars does not want to be 600 px tall.
                {
                    let natural: f32 = panels.iter().map(|(_, h)| *h + gutter).sum();
                    let per_lane = natural / cols as f32;
                    let room = full.height() - kpi_rect.height() - gutter;
                    if room > per_lane && per_lane > 1.0 {
                        let stretch = (room / per_lane).min(1.5);
                        for (_, h) in panels.iter_mut() {
                            *h *= stretch;
                        }
                    }
                }
                let mut y = vec![full.top() + kpi_rect.height() + gutter; cols];
                let lanes = motif::split_columns(
                    egui::Rect::from_min_size(egui::pos2(full.left(), 0.0), egui::vec2(w, 1.0)),
                    cols,
                    gutter,
                );
                let mut bottom = y[0];
                for (i, (title, height)) in panels.iter().enumerate() {
                    // Shortest-column-first keeps the two lanes level
                    // even though the panels are different heights.
                    let lane = if cols == 1 {
                        0
                    } else {
                        y.iter()
                            .enumerate()
                            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                            .map(|(k, _)| k)
                            .unwrap_or(0)
                    };
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(lanes[lane].left(), y[lane]),
                        egui::vec2(lanes[lane].width(), *height),
                    );
                    y[lane] += height + gutter;
                    bottom = bottom.max(y[lane]);
                    motif::panel(ui, rect, Some(title), |ui| {
                        let body = ui.max_rect();
                        match i {
                            0 => Self::dash_pipeline(ui, session, body),
                            1 => Self::dash_monthly(ui, body, &per_month, masked),
                            2 => Self::dash_per_kind(ui, session, config, body),
                            3 => open_patient = Self::dash_appointments(ui, session, body),
                            4 => open_recent = Self::dash_recent(ui, session, body),
                            _ => Self::dash_today_notes(ui, session, body),
                        }
                    });
                }
                // Claim the grid's full height so the scroll area knows
                // how far it goes: the panels were painted into rects,
                // not laid out by the cursor.
                ui.allocate_space(egui::vec2(w, bottom - full.top() - kpi_rect.height()));

                if let Some(p) = open_recent {
                    session.view = MainView::Search;
                    session.show_amounts = false;
                    session.open_patient(p);
                } else if let Some(id) = open_patient {
                    if let Some(p) = session.patients.iter().find(|p| p.id == id).cloned() {
                        session.view = MainView::Search;
                        session.show_amounts = false;
                        session.open_patient(p);
                    }
                }
                if let Some(err) = &session.error {
                    ui.colored_label(motif::ALERT, err.as_str());
                }
            });
    }

    /// The pipeline funnel: how many acts sit at each state.
    fn dash_pipeline(ui: &mut egui::Ui, session: &Session, rect: egui::Rect) {
        let rows: Vec<motif::chart::Row> = InterviewState::ALL
            .iter()
            .enumerate()
            .map(|(i, st)| motif::chart::Row {
                label: st.label(),
                value: session.summaries.iter().filter(|s| s.state == *st).count() as f64,
                // The ramp runs from grey to the accent blue: the
                // further right in the pipeline, the more "done" it is.
                color: motif::ACCENT.lerp_to_gamma(
                    motif::chart::SERIES[1],
                    1.0 - i as f32 / (InterviewState::ALL.len() - 1) as f32,
                ),
            })
            .collect();
        motif::chart::hbars(ui, rect, &rows, 96.0, &|v| format!("{v:.0}"));
    }

    /// Billed against pending revenue, one column per month.
    fn dash_monthly(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        per_month: &[(String, f64, f64)],
        masked: bool,
    ) {
        if masked {
            let inner = motif::chart::frame(ui, rect);
            ui.painter().text(
                inner.center(),
                egui::Align2::CENTER_CENTER,
                "• • •",
                egui::FontId::proportional(16.0),
                motif::TEXT_DIM,
            );
            return;
        }
        if per_month.is_empty() {
            let inner = motif::chart::frame(ui, rect);
            ui.painter().text(
                inner.center(),
                egui::Align2::CENTER_CENTER,
                tr("dash_empty"),
                egui::FontId::proportional(12.0),
                motif::TEXT_DIM,
            );
            return;
        }
        let rows = motif::split_rows(rect, &[0.0, 16.0], 4.0);
        // "2026-08" → "08/26"
        let labels: Vec<String> = per_month
            .iter()
            .map(|(m, _, _)| match (m.get(5..7), m.get(2..4)) {
                (Some(mm), Some(yy)) => format!("{mm}/{yy}"),
                _ => m.clone(),
            })
            .collect();
        let values: Vec<[f64; 2]> = per_month.iter().map(|(_, b, p)| [*b, *p]).collect();
        let groups: Vec<motif::chart::Group> = labels
            .iter()
            .zip(&values)
            .map(|(label, v)| motif::chart::Group {
                label,
                values: &v[..],
            })
            .collect();
        let colors = [motif::ACCENT, motif::chart::SERIES[1]];
        let hovered = motif::chart::bars(ui, rows[0], &groups, &colors, &|v| format!("{v:.0}"));
        if let Some(i) = hovered {
            let (m, b, p) = &per_month[i];
            egui::show_tooltip_text(
                ui.ctx(),
                ui.layer_id(),
                ui.id().with("dash_monthly_tip"),
                trn(
                    "dash_monthly_tooltip",
                    &[m, &format!("{b:.0}"), &format!("{p:.0}")],
                ),
            );
        }
        motif::inside(ui, rows[1], |ui| {
            motif::chart::legend(
                ui,
                &[
                    (tr("dash_legend_billed"), motif::ACCENT),
                    (tr("dash_legend_pending"), motif::chart::SERIES[1]),
                ],
            );
        });
    }

    /// The act mix: how many of each theme, against its yearly quota.
    fn dash_per_kind(ui: &mut egui::Ui, session: &Session, config: &Config, rect: egui::Rect) {
        let counts: Vec<(InterviewKind, usize)> = InterviewKind::ALL
            .iter()
            .map(|k| {
                (
                    *k,
                    session.summaries.iter().filter(|s| s.kind == *k).count(),
                )
            })
            .collect();
        let rows = motif::split_rows(rect, &[16.0, 0.0], 6.0);
        // A single stacked bar first: the mix as one shape, before the
        // per-theme numbers underneath it.
        let parts: Vec<(f64, egui::Color32)> = counts
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(k, n)| (*n as f64, kind_color(*k)))
            .collect();
        motif::chart::stacked(ui, rows[0], &parts);
        let bars: Vec<motif::chart::Row> = counts
            .iter()
            .map(|(k, n)| motif::chart::Row {
                label: k.label(),
                value: *n as f64,
                color: kind_color(*k),
            })
            .collect();
        let hovered = motif::chart::hbars(ui, rows[1], &bars, 160.0, &|v| format!("{v:.0}"));
        if let Some(i) = hovered {
            let (kind, n) = counts[i];
            let quota = config.per_year(kind);
            let mut text = match kind.act_code(0) {
                Some(code) if quota > 0 => trn("dash_kind_quota", &[&code, &n, &quota]),
                Some(code) => trn("dash_kind_tooltip", &[&code, &n]),
                None => trf("dash_acts_n", n),
            };
            // What the theme is worth, not only how often it is done —
            // the count alone never says which acts carry the month.
            if n > 0 {
                let earned: f64 = session
                    .summaries
                    .iter()
                    .filter(|s| s.kind == kind)
                    .map(|s| config.act_total(s.kind, s.fee_year, s.fee_rank, s.remote))
                    .sum();
                text.push_str(&trf("dash_kind_revenue", format!("{earned:.0}")));
            }
            egui::show_tooltip_text(ui.ctx(), ui.layer_id(), ui.id().with("dash_kind_tip"), text);
        }
    }

    /// Planned interviews not yet performed, soonest first, with the
    /// load of the coming weeks above them. Returns a patient to open.
    fn dash_appointments(
        ui: &mut egui::Ui,
        session: &mut Session,
        rect: egui::Rect,
    ) -> Option<i64> {
        if session.appointments.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("dash_rdv_empty"),
                egui::FontId::proportional(12.0),
                motif::TEXT_DIM,
            );
            return None;
        }
        let rows = motif::split_rows(rect, &[18.0, 12.0, 0.0], 3.0);
        // The 28 days from today: how loaded each one is. A month of
        // work in one strip of pixels.
        let days: Vec<String> = (0..28)
            .filter_map(|i| db::add_days(&session.today, i))
            .collect();
        if days.len() == 28 {
            let load: Vec<f64> = days
                .iter()
                .map(|d| session.appointments.iter().filter(|a| &a.date == d).count() as f64)
                .collect();
            if let Some(i) = motif::chart::heat_strip(ui, rows[0], &load, motif::ACCENT) {
                egui::show_tooltip_text(
                    ui.ctx(),
                    ui.layer_id(),
                    ui.id().with("dash_load_tip"),
                    trn(
                        "dash_load_tooltip",
                        &[&db::format_french_date(&days[i]), &(load[i] as i64)],
                    ),
                );
            }
            ui.painter().text(
                egui::pos2(rows[1].left(), rows[1].center().y),
                egui::Align2::LEFT_CENTER,
                tr("dash_load_caption"),
                egui::FontId::proportional(10.0),
                motif::TEXT_FAINT,
            );
        }
        let mut open = None;
        let today = session.today.clone();
        let appointments = session.appointments.clone();
        motif::inside(ui, rows[2], |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dash_rdv")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for rdv in &appointments {
                        let overdue = !today.is_empty() && rdv.date < today;
                        let is_today = !today.is_empty() && rdv.date == today;
                        let mut text = format!(
                            "{}   {}   ({})",
                            db::format_french_date(&rdv.date),
                            rdv.patient_name,
                            rdv.kind.label()
                        );
                        if !rdv.phone.is_empty() {
                            text.push_str(&format!("   —  {}", rdv.phone));
                        }
                        if overdue {
                            text.push_str(tr("dash_overdue"));
                        } else if is_today {
                            text.push_str(tr("dash_today"));
                        }
                        let label = egui::RichText::new(text);
                        let label = if overdue {
                            label.color(motif::ALERT)
                        } else if is_today {
                            label.color(motif::ACCENT).strong()
                        } else {
                            label
                        };
                        if motif::list_row(ui, label, false)
                            .on_hover_text(tr("dash_open_patient"))
                            .clicked()
                        {
                            open = Some(rdv.patient_id);
                        }
                    }
                });
        });
        open
    }

    /// The files that moved most recently. Returns one to open.
    fn dash_recent(ui: &mut egui::Ui, session: &Session, rect: egui::Rect) -> Option<Patient> {
        if session.recent.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("dash_recent_empty"),
                egui::FontId::proportional(12.0),
                motif::TEXT_DIM,
            );
            return None;
        }
        let mut open = None;
        let recent = session.recent.clone();
        motif::inside(ui, rect, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dash_recent")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for (p, moved) in &recent {
                        let text = format!(
                            "{}      {}",
                            p.full_name(),
                            db::format_french_date(&moved[..10.min(moved.len())])
                        );
                        if motif::list_row(ui, egui::RichText::new(text), false)
                            .on_hover_text(tr("dash_open_patient"))
                            .clicked()
                        {
                            open = Some(p.clone());
                        }
                    }
                });
        });
        open
    }

    /// What the team wrote today: day notes and transmissions.
    fn dash_today_notes(ui: &mut egui::Ui, session: &Session, rect: egui::Rect) {
        if session.today_notes.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("dash_today_notes_empty"),
                egui::FontId::proportional(12.0),
                motif::TEXT_DIM,
            );
            return;
        }
        let notes = session.today_notes.clone();
        motif::inside(ui, rect, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dash_notes")
                .show(ui, |ui| {
                    for note in &notes {
                        ui.label(
                            egui::RichText::new(note.stamp())
                                .size(10.0)
                                .color(operator_color(&note.operator)),
                        );
                        ui.add(egui::Label::new(rich_text(&note.body, 12.0, motif::TEXT)).wrap());
                        ui.add_space(3.0);
                    }
                });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The look follows the options: applied once, then only when
        // the scale or the density actually changes.
        let look = (
            (self.config.ui.text_scale * 100.0) as i32,
            self.config.density(),
        );
        if self.applied_look != Some(look) {
            motif::apply(ctx);
            motif::apply_scale(ctx, self.config.ui.text_scale, self.config.density());
            self.applied_look = Some(look);
        }
        // Track the window so the next session opens at this size.
        {
            let size = ctx.screen_rect().size();
            self.layout.window_width = size.x;
            self.layout.window_height = size.y;
            if self.layout != self.layout_saved {
                // A drag reports a new width every frame: wait until it
                // settles rather than writing the file 60 times a second.
                if self.layout_changed.elapsed() > Duration::from_secs(2) {
                    self.layout.save();
                    self.layout_saved = self.layout;
                } else {
                    // Idle sessions only repaint every 30 s; without
                    // this the debounce would not come round until the
                    // next thing the operator did.
                    ctx.request_repaint_after(Duration::from_millis(400));
                }
            } else {
                self.layout_changed = Instant::now();
            }
        }
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
                // The search view's home panels read the same rows.
                if matches!(session.view, MainView::Dashboard | MainView::Search) {
                    if let Ok(s) = session.db.interview_summaries(session.cycle_months) {
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
                if session.view == MainView::Transmissions {
                    session.load_transmissions();
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
        if ctx.input(|i| i.key_pressed(egui::Key::F6)) {
            self.show_nav = !self.show_nav;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.show_keys = !self.show_keys;
        }
        if self.show_keys && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_keys = false;
        }
        let toggle_dashboard = ctx.input(|i| i.key_pressed(egui::Key::F2));
        let toggle_drugs = ctx.input(|i| i.key_pressed(egui::Key::F3));
        let toggle_agenda = ctx.input(|i| i.key_pressed(egui::Key::F4));
        let toggle_trans = ctx.input(|i| i.key_pressed(egui::Key::F5));
        let toggle_map = ctx.input(|i| i.key_pressed(egui::Key::F7));

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("BPM-Caddy").strong())
                    .on_hover_text(format!(
                        concat!(
                            "BPM-Caddy v",
                            env!("CARGO_PKG_VERSION"),
                            "\nBase : {}\nConfiguration : {}"
                        ),
                        self.config.db_path().display(),
                        Config::path().display()
                    ));
                // The dock toggles sit next to the name, on the left:
                // they act on the frame around the work, not on the work.
                if matches!(self.state, State::Unlocked(_)) {
                    ui.add_space(10.0);
                    if motif::toggle(ui, tr("toolbar_nav"), self.show_nav)
                        .on_hover_text(tr("toolbar_nav_tooltip"))
                        .clicked()
                    {
                        self.show_nav = !self.show_nav;
                    }
                    if motif::toggle(ui, tr("toolbar_docs"), self.show_docs)
                        .on_hover_text(tr("toolbar_docs_tooltip"))
                        .clicked()
                    {
                        self.show_docs = !self.show_docs;
                    }
                    if motif::toggle(ui, tr("toolbar_keys"), self.show_keys)
                        .on_hover_text(tr("toolbar_keys_tooltip"))
                        .clicked()
                    {
                        self.show_keys = !self.show_keys;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Optional pictograms: painted, not typed (the
                    // bundled font has almost no symbols). They cost
                    // width, so they are off by default.
                    let icons = self.config.ui.icons;
                    let pict = |p: motif::Pict| if icons { Some(p) } else { None };
                    if !matches!(self.state, State::Unlocked(_))
                        && motif::icon_button(ui, pict(motif::Pict::Doc), tr("toolbar_docs"))
                            .clicked()
                    {
                        self.show_docs = !self.show_docs;
                    }
                    if let State::Unlocked(session) = &mut self.state {
                        if motif::icon_button(ui, pict(motif::Pict::Lock), tr("toolbar_lock"))
                            .clicked()
                        {
                            session.flush_date_edits();
                            self.state = State::Locked {
                                password: String::new(),
                                error: None,
                            };
                        }
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::icon_button(ui, pict(motif::Pict::Cog), tr("toolbar_options"))
                            .clicked()
                    {
                        self.options = if self.options.is_some() {
                            None
                        } else {
                            Some(OptionsEditor {
                                page: OptionsPage::Pharmacy,
                                cfg: self.config.clone(),
                                db_path_text: self
                                    .config
                                    .database
                                    .path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default(),
                                message: None,
                                confirm_reset: false,
                            })
                        };
                    }
                    if matches!(self.state, State::Unlocked(_))
                        && motif::icon_button(
                            ui,
                            pict(motif::Pict::Template),
                            tr("toolbar_template"),
                        )
                        .clicked()
                    {
                        self.tpl_editor = if self.tpl_editor.is_some() {
                            None
                        } else {
                            let path = self.config.template_path();
                            let text = std::fs::read_to_string(&path)
                                .unwrap_or_else(|_| crate::pdf::default_template().to_owned());
                            Some(TplEditor {
                                target: TplTarget::Fiche,
                                text,
                                message: None,
                            })
                        };
                    }
                });
            });
            ui.add_space(4.0);
        });

        // The workspace notebook: where the operator is, and everything
        // they have open. It replaces the row of view buttons the
        // toolbar used to carry — those said what you could reach, never
        // where you were.
        if matches!(self.state, State::Unlocked(_)) {
            // Ctrl+Tab walks the strip; Ctrl+W closes the open file.
            let (cycle, back, close) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab),
                    i.consume_key(
                        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                        egui::Key::Tab,
                    ),
                    i.consume_key(egui::Modifiers::CTRL, egui::Key::W),
                )
            });
            egui::TopBottomPanel::top("notebook")
                .frame(
                    egui::Frame::none()
                        .fill(motif::BG)
                        .inner_margin(egui::Margin::symmetric(6.0, 0.0)),
                )
                .show(ctx, |ui| {
                    let State::Unlocked(session) = &mut self.state else {
                        return;
                    };
                    session.note_tab();
                    let cur = session.current_tab();
                    let active = session.tabs.iter().position(|t| *t == cur).unwrap_or(0);
                    let labels: Vec<(String, bool, Option<egui::Color32>)> = session
                        .tabs
                        .iter()
                        .map(|t| {
                            let tint = match t {
                                WorkTab::Patient(_) => Some(motif::ACCENT),
                                WorkTab::Drug(_) => Some(motif::chart::SERIES[2]),
                                _ => None,
                            };
                            (session.tab_label(t), t.closable(), tint)
                        })
                        .collect();
                    let tabs: Vec<motif::Tab> = labels
                        .iter()
                        .map(|(label, closable, tint)| {
                            let mut t = motif::Tab::new(label);
                            if *closable {
                                t = t.closable();
                            }
                            if let Some(c) = tint {
                                t = t.tint(*c);
                            }
                            t
                        })
                        .collect();
                    let action = motif::tab_strip(ui, &tabs, active);
                    match action {
                        Some(motif::TabAction::Select(i)) => {
                            if let Some(t) = session.tabs.get(i).cloned() {
                                if t != cur {
                                    session.activate_tab(&t);
                                }
                            }
                        }
                        Some(motif::TabAction::Close(i)) => {
                            if let Some(t) = session.tabs.get(i).cloned() {
                                session.close_tab(&t);
                            }
                        }
                        None => {}
                    }
                    if back {
                        session.cycle_tab(-1);
                    } else if cycle {
                        session.cycle_tab(1);
                    }
                    if close {
                        let cur = session.current_tab();
                        session.close_tab(&cur);
                    }
                });
        }

        // Motif status bar: the at-a-glance numbers and which base this
        // post is on (multi-post support aid).
        let mut status_goto: Option<WorkTab> = None;
        if let State::Unlocked(session) = &self.state {
            let in_progress: i64 = session.pending.values().sum();
            let summary = trn(
                "status_summary",
                &[&session.patients.len(), &in_progress, &session.drugs.len()],
            );
            // What is actually late. A status bar that only counts rows
            // says nothing needs doing; this one says what does.
            let overdue = session
                .appointments
                .iter()
                .filter(|a| !session.today.is_empty() && a.date < session.today)
                .count();
            let today = session
                .appointments
                .iter()
                .filter(|a| !session.today.is_empty() && a.date == session.today)
                .count();
            let unbilled = session
                .summaries
                .iter()
                .filter(|s| s.state == InterviewState::ReportSent)
                .count();
            let operator = self.operator.trim().to_owned();
            let db_file = self
                .config
                .db_path()
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(summary)
                            .size(11.0)
                            .color(motif::TEXT_DIM),
                    );
                    // Each count is a way in: clicking goes where the
                    // work is, rather than reporting it and stopping.
                    let mut flag = |ui: &mut egui::Ui,
                                    text: String,
                                    color: egui::Color32,
                                    hint: &str,
                                    to: WorkTab| {
                        ui.add_space(6.0);
                        let resp = ui.add(
                            egui::Label::new(egui::RichText::new(text).size(11.0).color(color))
                                .sense(egui::Sense::click()),
                        );
                        if resp.on_hover_text(hint).clicked() {
                            status_goto = Some(to);
                        }
                    };
                    if overdue > 0 {
                        flag(
                            ui,
                            trf("status_overdue", overdue),
                            motif::ALERT,
                            tr("status_overdue_tooltip"),
                            WorkTab::Agenda,
                        );
                    }
                    if today > 0 {
                        flag(
                            ui,
                            trf("status_today", today),
                            motif::ACCENT,
                            tr("status_today_tooltip"),
                            WorkTab::Agenda,
                        );
                    }
                    if unbilled > 0 {
                        flag(
                            ui,
                            trf("status_to_bill", unbilled),
                            motif::ACCENT,
                            tr("status_to_bill_tooltip"),
                            WorkTab::Dashboard,
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(trf("lock_db_path", db_file))
                                .size(11.0)
                                .color(motif::TEXT_DIM),
                        );
                        // Who is stamping the notes, so a shared post
                        // never signs an entry with the last shift's
                        // initials without saying so.
                        if !operator.is_empty() {
                            ui.label(
                                egui::RichText::new(trf("status_operator", &operator))
                                    .size(11.0)
                                    .color(operator_color(&operator)),
                            );
                        }
                    });
                });
            });
        }
        if let (Some(to), State::Unlocked(session)) = (status_goto, &mut self.state) {
            session.activate_tab(&to);
        }

        if toggle_dashboard {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Search
                    | MainView::Drugs
                    | MainView::Agenda
                    | MainView::Transmissions
                    | MainView::VaccineMap => {
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
        if toggle_trans {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::Transmissions => MainView::Search,
                    _ => {
                        session.flush_date_edits();
                        session.show_amounts = false;
                        session.trans_day = String::new();
                        session.load_transmissions();
                        MainView::Transmissions
                    }
                };
            }
        }
        if toggle_map {
            if let State::Unlocked(session) = &mut self.state {
                session.view = match session.view {
                    MainView::VaccineMap => MainView::Search,
                    _ => {
                        session.flush_date_edits();
                        session.show_amounts = false;
                        MainView::VaccineMap
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
                        ui.colored_label(motif::ALERT, err.as_str());
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
        let mut switch_tpl: Option<TplTarget> = None;
        if let Some(TplEditor {
            target,
            text,
            message,
        }) = &mut self.tpl_editor
        {
            let path = match target {
                TplTarget::Fiche => self.config.template_path(),
                TplTarget::Courrier => self.config.cr_template_path(),
                TplTarget::Carnet => self.config.carnet_template_path(),
                TplTarget::Ordonnance => self.config.ordonnance_template_path(),
            };
            // A Typst source is code: give it the screen. A fixed
            // 680x540 box meant scrolling a page-long template through
            // a porthole on a monitor with room for all of it.
            let screen = ctx.screen_rect().size();
            let editor_h = (screen.y - 260.0).clamp(300.0, 900.0);
            egui::Window::new(tr("tpl_title"))
                .collapsible(false)
                .resizable(true)
                .default_size([
                    (screen.x * 0.62).clamp(620.0, 1100.0),
                    (screen.y * 0.86).clamp(480.0, 1100.0),
                ])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    // Which template: the interview sheet or the CR letter.
                    ui.horizontal(|ui| {
                        for (t, label) in [
                            (TplTarget::Fiche, tr("tpl_target_fiche")),
                            (TplTarget::Courrier, tr("tpl_target_cr")),
                            (TplTarget::Carnet, tr("tpl_target_carnet")),
                            (TplTarget::Ordonnance, tr("tpl_target_ordonnance")),
                        ] {
                            // Sunken marks the active template.
                            let btn = motif::toggle(ui, label, *target == t);
                            if btn.clicked() && *target != t {
                                switch_tpl = Some(t);
                            }
                        }
                    });
                    ui.label(
                        egui::RichText::new(trf("tpl_path", path.display()))
                            .size(11.0)
                            .color(motif::TEXT_DIM),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(editor_h)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), editor_h - 8.0],
                                egui::TextEdit::multiline(text)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor(),
                            );
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if motif::button(ui, tr("form_save")).clicked() {
                            let check = match target {
                                TplTarget::Fiche => crate::pdf::check_template(text),
                                TplTarget::Courrier => crate::pdf::check_cr_template(text),
                                TplTarget::Carnet => crate::pdf::check_trans_template(text),
                                TplTarget::Ordonnance => {
                                    crate::pdf::check_ordonnance_template(text)
                                }
                            };
                            match check {
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
                            let preview = match target {
                                TplTarget::Fiche => crate::pdf::preview_template(text),
                                TplTarget::Courrier => crate::pdf::preview_cr_template(text),
                                TplTarget::Carnet => crate::pdf::preview_trans_template(text),
                                TplTarget::Ordonnance => {
                                    crate::pdf::preview_ordonnance_template(text)
                                }
                            };
                            if let Err(e) = preview {
                                *message = Some((true, e));
                            }
                        }
                        if motif::button(ui, tr("tpl_reset"))
                            .on_hover_text(tr("tpl_reset_tooltip"))
                            .clicked()
                        {
                            *text = match target {
                                TplTarget::Fiche => crate::pdf::default_template().to_owned(),
                                TplTarget::Courrier => crate::pdf::default_cr_template().to_owned(),
                                TplTarget::Carnet => {
                                    crate::pdf::default_trans_template().to_owned()
                                }
                                TplTarget::Ordonnance => {
                                    crate::pdf::default_ordonnance_template().to_owned()
                                }
                            };
                            *message = None;
                        }
                        if motif::button(ui, tr("tpl_close")).clicked() {
                            close_tpl = true;
                        }
                    });
                    if let Some((is_error, msg)) = message {
                        ui.add_space(4.0);
                        let color = if *is_error {
                            motif::ALERT
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
        if let Some(t) = switch_tpl {
            // Load the other template (unsaved edits are discarded).
            let path = match t {
                TplTarget::Fiche => self.config.template_path(),
                TplTarget::Courrier => self.config.cr_template_path(),
                TplTarget::Carnet => self.config.carnet_template_path(),
                TplTarget::Ordonnance => self.config.ordonnance_template_path(),
            };
            let text = std::fs::read_to_string(&path).unwrap_or_else(|_| match t {
                TplTarget::Fiche => crate::pdf::default_template().to_owned(),
                TplTarget::Courrier => crate::pdf::default_cr_template().to_owned(),
                TplTarget::Carnet => crate::pdf::default_trans_template().to_owned(),
                TplTarget::Ordonnance => crate::pdf::default_ordonnance_template().to_owned(),
            });
            self.tpl_editor = Some(TplEditor {
                target: t,
                text,
                message: None,
            });
        }

        // Global options editor: edits config.toml from within the app.
        if !matches!(self.state, State::Unlocked(_)) {
            self.options = None;
        }
        let mut close_opts = false;
        let mut open_pw = false;
        let mut saved_cfg: Option<Config> = None;
        // (target, also_point_config_at_it) requested from the DB tools.
        let mut db_export: Option<(std::path::PathBuf, bool)> = None;
        let mut db_seed = false;
        let mut db_details = false;
        let mut db_reset = false;
        if let Some(editor) = &mut self.options {
            // Fit the dialog to the window: the options list is long,
            // and a fixed height clipped the last rows on small screens.
            let avail = ctx.screen_rect().height();
            egui::Window::new(tr("opts_title"))
                .collapsible(false)
                .resizable(true)
                .default_size([600.0, (avail - 80.0).clamp(420.0, 900.0)])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    // One page at a time: the fee matrix and the
                    // auto-lock timeout used to be the same five-screen
                    // scroll apart.
                    ui.horizontal_wrapped(|ui| {
                        for page in OptionsPage::ALL {
                            if motif::toggle(ui, page.label(), editor.page == page).clicked() {
                                editor.page = page;
                            }
                        }
                    });
                    ui.add_space(6.0);
                    let page = editor.page;
                    egui::ScrollArea::vertical()
                        .max_height((avail - 200.0).max(280.0))
                        .show(ui, |ui| {
                            let dim = |t: &str| egui::RichText::new(t).color(motif::TEXT_DIM);
                            if page == OptionsPage::Pharmacy {
                                motif::section(ui, tr("opts_pharmacy"));
                                egui::Grid::new("opts_pharmacy")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("form_last_name")));
                                        ui.add_sized(
                                            [300.0, 24.0],
                                            egui::TextEdit::singleline(
                                                &mut editor.cfg.pharmacy.name,
                                            ),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("form_address")));
                                        ui.add_sized(
                                            [300.0, 24.0],
                                            egui::TextEdit::singleline(
                                                &mut editor.cfg.pharmacy.address,
                                            ),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("form_phone")));
                                        ui.add_sized(
                                            [300.0, 24.0],
                                            egui::TextEdit::singleline(
                                                &mut editor.cfg.pharmacy.phone,
                                            ),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_pharmacist")));
                                        ui.add_sized(
                                            [300.0, 24.0],
                                            egui::TextEdit::singleline(
                                                &mut editor.cfg.pharmacy.pharmacist,
                                            ),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_am_number")));
                                        ui.add_sized(
                                            [300.0, 24.0],
                                            egui::TextEdit::singleline(
                                                &mut editor.cfg.pharmacy.am_number,
                                            )
                                            .hint_text(tr("opts_am_number_hint")),
                                        );
                                        ui.end_row();
                                    });
                            }
                            if page == OptionsPage::Ui {
                                motif::section(ui, tr("opts_ui"));
                                egui::Grid::new("opts_ui")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("docs_operator")));
                                        ui.add_sized(
                                            [80.0, 24.0],
                                            egui::TextEdit::singleline(&mut editor.cfg.ui.operator),
                                        );
                                        ui.end_row();
                                    });
                                ui.checkbox(
                                    &mut editor.cfg.ui.show_docs_on_start,
                                    tr("opts_show_docs"),
                                );
                                ui.checkbox(
                                    &mut editor.cfg.ui.show_nav_on_start,
                                    tr("opts_show_nav"),
                                );
                                ui.checkbox(&mut editor.cfg.ui.icons, tr("opts_icons"));
                                egui::Grid::new("opts_look")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("opts_text_scale")));
                                        ui.add(
                                            egui::Slider::new(
                                                &mut editor.cfg.ui.text_scale,
                                                0.8..=1.6,
                                            )
                                            .fixed_decimals(2)
                                            .suffix(" x"),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_font")));
                                        ui.horizontal(|ui| {
                                            let mut shown = editor
                                                .cfg
                                                .ui
                                                .font_path
                                                .as_ref()
                                                .map(|p| p.display().to_string())
                                                .unwrap_or_default();
                                            if ui
                                                .add_sized(
                                                    [220.0, 24.0],
                                                    egui::TextEdit::singleline(&mut shown)
                                                        .hint_text(tr("opts_font_default")),
                                                )
                                                .changed()
                                            {
                                                editor.cfg.ui.font_path = if shown.trim().is_empty()
                                                {
                                                    None
                                                } else {
                                                    Some(std::path::PathBuf::from(shown.trim()))
                                                };
                                            }
                                            if motif::button(ui, tr("opts_db_browse")).clicked() {
                                                if let Some(p) = rfd::FileDialog::new()
                                                    .add_filter(
                                                        "Police",
                                                        &["ttf", "otf", "TTF", "OTF"],
                                                    )
                                                    .pick_file()
                                                {
                                                    editor.cfg.ui.font_path = Some(p);
                                                }
                                            }
                                            if motif::button(ui, tr("tpl_reset")).clicked() {
                                                editor.cfg.ui.font_path = None;
                                            }
                                        });
                                        ui.end_row();
                                        ui.label(dim(tr("opts_side_pane")));
                                        ui.horizontal(|ui| {
                                            for (value, label) in [
                                                ("docs", tr("docs_title")),
                                                ("carnet", tr("trans_title")),
                                                ("notes", tr("side_pane_notes")),
                                            ] {
                                                if ui
                                                    .selectable_label(
                                                        editor.cfg.ui.side_pane == value,
                                                        label,
                                                    )
                                                    .clicked()
                                                {
                                                    editor.cfg.ui.side_pane = value.to_owned();
                                                }
                                            }
                                        });
                                        ui.end_row();
                                        ui.label(dim(tr("opts_font_note")));
                                        ui.label(
                                            egui::RichText::new(tr("opts_restart"))
                                                .size(11.0)
                                                .color(motif::TEXT_DIM),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_density")));
                                        ui.horizontal(|ui| {
                                            for (value, label) in [
                                                ("confortable", tr("opts_density_comfort")),
                                                ("compact", tr("opts_density_compact")),
                                            ] {
                                                if ui
                                                    .selectable_label(
                                                        editor
                                                            .cfg
                                                            .ui
                                                            .density
                                                            .eq_ignore_ascii_case(value),
                                                        label,
                                                    )
                                                    .clicked()
                                                {
                                                    editor.cfg.ui.density = value.to_owned();
                                                }
                                            }
                                        });
                                        ui.end_row();
                                    });
                                ui.checkbox(
                                    &mut editor.cfg.ui.discreet_finances,
                                    tr("opts_discreet"),
                                );
                            }
                            if page == OptionsPage::Database {
                                motif::section(ui, tr("opts_db"));
                                egui::Grid::new("opts_db")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("opts_autolock")));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut editor.cfg.database.auto_lock_timeout_minutes,
                                            )
                                            .range(0..=240),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_backups")));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut editor.cfg.database.backups_keep,
                                            )
                                            .range(0..=60),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_db_path")));
                                        ui.horizontal(|ui| {
                                            ui.add_sized(
                                                [258.0, 24.0],
                                                egui::TextEdit::singleline(
                                                    &mut editor.db_path_text,
                                                ),
                                            );
                                            if motif::button(ui, tr("opts_db_browse")).clicked() {
                                                if let Some(p) = rfd::FileDialog::new()
                                                    .add_filter("SQLite", &["db", "sqlite"])
                                                    .add_filter("*", &["*"])
                                                    .pick_file()
                                                {
                                                    editor.db_path_text = p.display().to_string();
                                                }
                                            }
                                        });
                                        ui.end_row();
                                    });
                                // File-level tools: consistent encrypted copy
                                // (VACUUM INTO) to any destination; "move"
                                // additionally points the config at the copy
                                // (applied on save + restart, old file kept).
                                ui.horizontal(|ui| {
                                    if motif::button(ui, tr("opts_db_copy")).clicked() {
                                        if let Some(p) = rfd::FileDialog::new()
                                            .set_file_name("bpm_caddy.db")
                                            .save_file()
                                        {
                                            db_export = Some((p, false));
                                        }
                                    }
                                    if motif::button(ui, tr("opts_db_move")).clicked() {
                                        if let Some(p) = rfd::FileDialog::new()
                                            .set_file_name("bpm_caddy.db")
                                            .save_file()
                                        {
                                            db_export = Some((p, true));
                                        }
                                    }
                                });
                                // Maintenance: complete a base created before
                                // the starter list grew, or wipe everything
                                // (debug/demo — two clicks, never one).
                                ui.horizontal(|ui| {
                                    if motif::button(ui, tr("opts_db_seed")).clicked() {
                                        db_seed = true;
                                    }
                                    if motif::button(ui, tr("opts_db_details"))
                                        .on_hover_text(tr("opts_db_details_tooltip"))
                                        .clicked()
                                    {
                                        db_details = true;
                                    }
                                    let danger = if editor.confirm_reset {
                                        tr("opts_db_reset_confirm")
                                    } else {
                                        tr("opts_db_reset")
                                    };
                                    let btn = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(danger)
                                                .color(egui::Color32::WHITE)
                                                .size(12.0),
                                        )
                                        .fill(motif::ALERT),
                                    );
                                    if btn.clicked() {
                                        if editor.confirm_reset {
                                            db_reset = true;
                                            editor.confirm_reset = false;
                                        } else {
                                            editor.confirm_reset = true;
                                        }
                                    }
                                });
                                ui.label(
                                    egui::RichText::new(tr("opts_db_note"))
                                        .size(11.0)
                                        .color(motif::TEXT_DIM),
                                );
                            }
                            if page == OptionsPage::Fees {
                                motif::section(ui, tr("opts_fees"));
                                ui.label(
                                    egui::RichText::new(tr("opts_fees_note"))
                                        .size(11.0)
                                        .color(motif::TEXT_DIM),
                                );
                                ui.label(
                                    egui::RichText::new(tr("opts_fees_rules"))
                                        .size(11.0)
                                        .color(motif::TEXT_DIM),
                                );
                                ui.add_space(4.0);
                                egui::Grid::new("opts_fees")
                                    .num_columns(7)
                                    .spacing([10.0, 5.0])
                                    .show(ui, |ui| {
                                        let themes: [(&str, &mut ActFees); 10] = [
                                            ("BPM", &mut editor.cfg.billing.bpm),
                                            ("AOD", &mut editor.cfg.billing.aod),
                                            ("AVK", &mut editor.cfg.billing.avk),
                                            ("Asthme", &mut editor.cfg.billing.asthme),
                                            (
                                                "Anticancéreux long cours",
                                                &mut editor.cfg.billing.anticancereux_lc,
                                            ),
                                            (
                                                "Anticancéreux (autres)",
                                                &mut editor.cfg.billing.anticancereux_autres,
                                            ),
                                            ("TROD angine", &mut editor.cfg.billing.trod_angine),
                                            ("TROD cystite", &mut editor.cfg.billing.trod_cystite),
                                            ("Vaccination", &mut editor.cfg.billing.vaccination),
                                            ("Prévention", &mut editor.cfg.billing.prevention),
                                        ];
                                        // Theme, code, then one column per
                                        // entretien of the sequence, then
                                        // the year's total.
                                        ui.label("");
                                        ui.label(dim(tr("opts_fee_code")));
                                        for i in 1..=ActFees::STEPS {
                                            ui.label(dim(&format!("{i}{}", tr("opts_fee_nth"))));
                                        }
                                        ui.label(dim(tr("opts_fee_total")));
                                        ui.end_row();
                                        for (label, fees) in themes {
                                            let kind = InterviewKind::ALL
                                                .into_iter()
                                                .find(|k| k.label() == label);
                                            for year in 0..2 {
                                                let steps = kind
                                                    .filter(|k| k.is_accompaniment())
                                                    .map(|k| k.sequence(year).len())
                                                    .unwrap_or(1);
                                                let row_label = match (kind, year) {
                                                    (Some(k), _) if !k.is_accompaniment() => {
                                                        label.to_owned()
                                                    }
                                                    (_, 0) => {
                                                        format!("{label} — {}", tr("opts_year_1"))
                                                    }
                                                    _ => format!(
                                                        "{label} — {}",
                                                        tr("opts_year_next")
                                                    ),
                                                };
                                                ui.label(dim(&row_label));
                                                ui.label(
                                                    egui::RichText::new(
                                                        kind.and_then(|k| k.act_code(year))
                                                            .unwrap_or("—"),
                                                    )
                                                    .size(11.0)
                                                    .strong()
                                                    .color(motif::ACCENT),
                                                );
                                                for rank in 0..ActFees::STEPS {
                                                    if rank < steps {
                                                        ui.add(
                                                            egui::DragValue::new(
                                                                fees.slot_mut(year, rank),
                                                            )
                                                            .range(0.0..=500.0)
                                                            .suffix(" €"),
                                                        );
                                                    } else {
                                                        ui.label(dim("—"))
                                                            .on_hover_text(tr("opts_fee_unused"));
                                                    }
                                                }
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{:.2} €",
                                                        fees.year_total(year)
                                                    ))
                                                    .strong(),
                                                );
                                                ui.end_row();
                                                // A theme outside the
                                                // convention has one rate,
                                                // not one per year.
                                                if kind.is_none_or(|k| !k.is_accompaniment()) {
                                                    break;
                                                }
                                            }
                                        }
                                        ui.label(dim(tr("opts_fee_adhesion")));
                                        ui.label(
                                            egui::RichText::new(db::ADHESION_CODE)
                                                .size(11.0)
                                                .strong()
                                                .color(motif::ACCENT),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut editor.cfg.billing.adhesion)
                                                .range(0.0..=100.0)
                                                .speed(0.01)
                                                .suffix(" €"),
                                        );
                                        ui.label("");
                                        ui.label("");
                                        ui.label("");
                                        ui.label("");
                                        ui.end_row();
                                        ui.label(dim(tr("opts_fee_remote")));
                                        ui.label(
                                            egui::RichText::new(db::REMOTE_CODE)
                                                .size(11.0)
                                                .strong()
                                                .color(motif::ACCENT),
                                        );
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut editor.cfg.billing.teleconsultation,
                                            )
                                            .range(0.0..=100.0)
                                            .suffix(" €"),
                                        );
                                        ui.label("");
                                        ui.label("");
                                        ui.label("");
                                        ui.label("");
                                        ui.end_row();
                                    });
                            }
                            if page == OptionsPage::Rules {
                                motif::section(ui, tr("opts_rules"));
                                egui::Grid::new("opts_rules_cycle")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(dim(tr("opts_cycle_months")));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut editor.cfg.rules.cycle_months,
                                            )
                                            .range(1..=36)
                                            .suffix(tr("opts_cycle_suffix")),
                                        );
                                        ui.end_row();
                                        ui.label(dim(tr("opts_enforcement")));
                                        ui.horizontal(|ui| {
                                            for (level, label) in [
                                                (RuleEnforcement::Warn, tr("opts_enforce_warn")),
                                                (
                                                    RuleEnforcement::Inform,
                                                    tr("opts_enforce_inform"),
                                                ),
                                                (RuleEnforcement::Block, tr("opts_enforce_block")),
                                            ] {
                                                ui.radio_value(
                                                    &mut editor.cfg.rules.enforcement,
                                                    level,
                                                    label,
                                                );
                                            }
                                        });
                                        ui.end_row();
                                    });
                                egui::Grid::new("opts_rules")
                                    .num_columns(4)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        let rules: [(&str, &mut u32); 9] = [
                                            ("BPM", &mut editor.cfg.rules.bpm_per_year),
                                            ("AOD", &mut editor.cfg.rules.aod_per_year),
                                            ("AVK", &mut editor.cfg.rules.avk_per_year),
                                            ("Asthme", &mut editor.cfg.rules.asthme_per_year),
                                            (
                                                "Anticancéreux",
                                                &mut editor.cfg.rules.anticancereux_per_year,
                                            ),
                                            (
                                                "TROD angine",
                                                &mut editor.cfg.rules.trod_angine_per_year,
                                            ),
                                            (
                                                "TROD cystite",
                                                &mut editor.cfg.rules.trod_cystite_per_year,
                                            ),
                                            (
                                                "Vaccination",
                                                &mut editor.cfg.rules.vaccination_per_year,
                                            ),
                                            (
                                                "Prévention",
                                                &mut editor.cfg.rules.prevention_per_year,
                                            ),
                                        ];
                                        for (i, (label, n)) in rules.into_iter().enumerate() {
                                            ui.label(dim(label));
                                            ui.add(egui::DragValue::new(n).range(0..=12));
                                            if i % 2 == 1 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                            }
                            if page == OptionsPage::Database {
                                ui.add_space(8.0);
                                motif::section(ui, tr("opts_security"));
                                if motif::button(ui, tr("opts_change_pw")).clicked() {
                                    open_pw = true;
                                }
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if motif::button(ui, tr("form_save")).clicked() {
                            editor.cfg.database.path = if editor.db_path_text.trim().is_empty() {
                                None
                            } else {
                                Some(std::path::PathBuf::from(editor.db_path_text.trim()))
                            };
                            match editor.cfg.save() {
                                Ok(()) => {
                                    saved_cfg = Some(editor.cfg.clone());
                                    editor.message = Some((false, tr("opts_saved").to_owned()));
                                }
                                Err(e) => {
                                    editor.message = Some((true, trf("opts_save_error", e)));
                                }
                            }
                        }
                        if motif::button(ui, tr("tpl_close")).clicked() {
                            close_opts = true;
                        }
                    });
                    if let Some((is_error, msg)) = &editor.message {
                        let color = if *is_error {
                            motif::ALERT
                        } else {
                            motif::ACCENT
                        };
                        ui.colored_label(color, msg.as_str());
                    }
                });
        }
        if db_seed || db_details || db_reset {
            let result = if let State::Unlocked(session) = &mut self.state {
                if db_reset {
                    session
                        .db
                        .reset_all_data()
                        .map(|()| (true, db::STARTER_DRUG_COUNT))
                } else if db_details {
                    session.db.fill_starter_details().map(|n| (false, n))
                } else {
                    session.db.seed_missing_drugs().map(|n| (false, n))
                }
            } else {
                Err(tr("opts_db_locked").to_owned())
            };
            match result {
                Ok((was_reset, n)) => {
                    if let State::Unlocked(session) = &mut self.state {
                        // Everything the views cache may have just been
                        // deleted — drop it all, then reload.
                        if let Ok(list) = session.db.patients() {
                            session.set_patients(list);
                        }
                        if let Ok(list) = session.db.drugs() {
                            session.drugs = list;
                        }
                        if let Ok(counts) = session.db.pending_counts() {
                            session.pending = counts;
                        }
                        session.viewing = None;
                        session.viewing_interviews.clear();
                        session.patient_treats.clear();
                        session.patient_notes.clear();
                        session.drug_notes.clear();
                        session.drug_patients.clear();
                        session.date_edits.clear();
                        session.drug_form = None;
                        session.drug_base = None;
                        session.drug_selected = 0;
                        session.selected = 0;
                        session.load_transmissions();
                        session.refresh_dashboard();
                    }
                    if let Some(editor) = &mut self.options {
                        editor.message = Some(if was_reset {
                            (false, trf("opts_db_reset_done", n))
                        } else if n == 0 {
                            (false, tr("opts_db_seed_none").to_owned())
                        } else if db_details {
                            (false, trf("opts_db_details_done", n))
                        } else {
                            (false, trf("opts_db_seed_done", n))
                        });
                    }
                }
                Err(e) => {
                    if let Some(editor) = &mut self.options {
                        editor.message = Some((true, e));
                    }
                }
            }
        }
        if let Some((target, point)) = db_export {
            let current = self.config.db_path();
            let same = target
                .canonicalize()
                .map(|t| current.canonicalize().map(|c| t == c).unwrap_or(false))
                .unwrap_or(false);
            let result = if same {
                Err(tr("opts_db_same").to_owned())
            } else if let State::Unlocked(session) = &self.state {
                // The native dialog already confirmed overwriting, but
                // VACUUM INTO refuses existing files — clear it first.
                if target.exists() {
                    let _ = std::fs::remove_file(&target);
                }
                session.db.backup_to(&target)
            } else {
                Err(tr("opts_db_locked").to_owned())
            };
            if let Some(editor) = &mut self.options {
                match result {
                    Ok(()) => {
                        if point {
                            editor.db_path_text = target.display().to_string();
                            editor.message = Some((false, tr("opts_db_moved").to_owned()));
                        } else {
                            editor.message = Some((false, trf("opts_db_copied", target.display())));
                        }
                    }
                    Err(e) => editor.message = Some((true, trf("opts_db_copy_error", e))),
                }
            }
        }
        if let Some(cfg) = saved_cfg {
            // Live-apply everything except the database path (restart).
            self.config = cfg;
            if let State::Unlocked(session) = &mut self.state {
                session.cycle_months = self.config.rules.cycle_months.max(1);
                session.refresh_dashboard();
            }
        }
        if open_pw {
            self.pw_change = Some(PwChangeForm::default());
        }
        if close_opts {
            self.options = None;
        }

        if self.show_keys && matches!(self.state, State::Unlocked(_)) {
            self.keys_window(ctx);
        }

        // The docs pane may hold patient-adjacent notes: never show it on
        // the lock screen. Same for the navigator, which lists patients.
        if self.show_docs && matches!(self.state, State::Unlocked(_)) {
            self.docs_pane(ctx);
        }
        if self.show_nav && matches!(self.state, State::Unlocked(_)) {
            self.nav_dock(ctx);
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
        // Reopen where the workspace was left: the window's size and the
        // widths the docks were dragged to.
        self.layout.save();
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
    fn half_life_units_are_whole_words() {
        use super::parse_hours;
        // "min" inside "administration" used to divide by sixty: a
        // five-hour half-life became five minutes on the curve.
        assert_eq!(
            parse_hours("environ 4 à 7 heures en administration répétée"),
            Some(5.5)
        );
        assert_eq!(
            parse_hours("environ 1 semaine, administration hebdomadaire"),
            Some(168.0)
        );
        // The real units still work.
        assert_eq!(parse_hours("30 min"), Some(0.5));
        assert_eq!(parse_hours("45 minutes"), Some(0.75));
        assert_eq!(parse_hours("≈ 7 jours"), Some(168.0));
        assert_eq!(parse_hours("2 semaines"), Some(336.0));
        assert_eq!(parse_hours("≈ 12 heures"), Some(12.0));
    }

    #[test]
    fn free_text_markup_is_rendered() {
        use super::rich_text;
        // *gras*, _italique_ and =surligné= become formatted sections;
        // the markers themselves are consumed.
        let job = rich_text(
            "Prise *le matin* et _à jeun_, =INR= à J3.",
            13.0,
            motif::TEXT,
        );
        assert!(!job.text.contains('*'));
        assert!(!job.text.contains('_'));
        assert!(!job.text.contains('='));
        assert!(job.text.contains("le matin"));
        assert!(job.sections.iter().any(|s| s.format.italics));
        assert!(job
            .sections
            .iter()
            .any(|s| s.format.background == motif::BG_LIGHT));
        // A lone marker inside a sentence stays literal.
        let job = rich_text("dose = 5 mg", 13.0, motif::TEXT);
        assert_eq!(job.text, "dose = 5 mg");
        // Plain text goes through untouched, in one section.
        let job = rich_text("Rien de particulier", 13.0, motif::TEXT);
        assert_eq!(job.text, "Rien de particulier");
        assert_eq!(job.sections.len(), 1);
    }

    #[test]
    fn half_lives_are_read_from_free_text() {
        use super::parse_hours;
        assert_eq!(parse_hours("≈ 12 heures"), Some(12.0));
        // A range is read as its middle.
        assert_eq!(parse_hours("5 à 13 h"), Some(9.0));
        assert_eq!(
            parse_hours("12 à 17 heures, allongée si DFG bas"),
            Some(14.5)
        );
        // Days and minutes are converted to hours.
        assert_eq!(parse_hours("≈ 7 jours"), Some(168.0));
        assert_eq!(parse_hours("30 min"), Some(0.5));
        assert_eq!(parse_hours("1,5 h"), Some(1.5));
        // Nothing numeric: no curve to draw.
        assert_eq!(parse_hours("Très longue"), None);
        assert_eq!(parse_hours(""), None);
    }

    /// The memo's derogation conditions, as the fiche checks them.
    #[test]
    fn the_derogation_reports_what_is_missing() {
        use crate::db::{Interview, InterviewState};
        let itv = |id: i64, day: u32, change: bool| Interview {
            id,
            kind: InterviewKind::AnticancereuxAutres,
            state: InterviewState::Performed,
            duration_minutes: 30,
            scheduled_date: None,
            scheduled_time: String::new(),
            remote: false,
            treatment_change: change,
            theme: String::new(),
            trod_result: String::new(),
            created_at: format!("2026-{day:02}-05 10:00:00"),
        };
        // Année 1 : il faut deux entretiens avant le changement et deux
        // après. Ici un seul avant, un seul après.
        let short = vec![itv(1, 1, false), itv(2, 3, true)];
        assert_eq!(
            super::treatment_change_shortfall(&short, &short[1], 12),
            Some((1, 1))
        );
        // Séquence complète de part et d'autre : plus rien à signaler.
        let full = vec![
            itv(1, 1, false),
            itv(2, 2, false),
            itv(3, 4, true),
            itv(4, 5, false),
        ];
        assert_eq!(super::treatment_change_shortfall(&full, &full[2], 12), None);
        // Changement survenu dans une année suivante : le mémo
        // n'exige qu'un entretien avant et deux après pour « autres »,
        // pas les minimums d'année 1.
        let later = vec![
            itv(1, 1, false),
            itv(2, 2, false),
            itv(3, 3, false),
            itv(4, 4, true),
            itv(5, 6, false),
            itv(6, 8, true),
            itv(7, 9, false),
        ];
        let ranks = super::interview_ranks(&later, 12);
        assert!(ranks.get(&6).map(|(y, _)| *y) >= Some(1));
        // Un seul entretien avant le second changement, deux après :
        // conforme, donc aucun manque signalé.
        assert_eq!(
            super::treatment_change_shortfall(&later, &later[5], 12),
            None
        );
        // Un entretien non marqué ne déclenche rien.
        assert_eq!(super::treatment_change_shortfall(&full, &full[0], 12), None);
        // Le thème doit porter la dérogation.
        let mut bpm = full.clone();
        for i in &mut bpm {
            i.kind = InterviewKind::Bpm;
        }
        assert_eq!(super::treatment_change_shortfall(&bpm, &bpm[2], 12), None);
    }

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
            theme: "Observance".to_owned(),
            fee_rank: 0,
            fee_year: 0,
            remote: false,
            situation: "ALD".to_owned(),
            treatment_change: false,
        }];
        let csv = interviews_csv(&rows, &Config::default());
        // BOM so Excel decodes UTF-8 accents, semicolons, CRLF.
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("Patient;Téléphone;Naissance;Type;Code acte;Année;Étape"));
        // The tricky name is quoted with doubled inner quotes; the act
        // code and the step come from the convention's own sequence.
        assert!(
            csv.contains("\"Jean; \"\"Le Grand\"\" Dupont\";06 12 34 56 78;03/07/1958;BPM;BMI;1;")
        );
        // Billed row: tariff and billed columns both carry the fee, the
        // situation travels with it, and the coverage rate is the 70 %
        // of the non-anticancéreux themes.
        assert!(csv.contains("23/08/2026;01/09/2026;45;;;ALD;15,00;70;15,00\r\n"));
        // Unbilled row: the "Facturé" column stays at zero.
        let mut pending = rows[0].clone();
        pending.state = InterviewState::Performed;
        let csv = interviews_csv(&[pending], &Config::default());
        assert!(csv.contains(";15,00;70;0,00\r\n"));
        // The last step of the first BMI sequence is the 20 € one.
        let mut last = rows[0].clone();
        last.fee_rank = 3;
        let csv = interviews_csv(&[last], &Config::default());
        assert!(csv.contains(";20,00;70;20,00\r\n"));
        // Held remotely: TPH appears and its fee is added on top.
        let mut remote = rows[0].clone();
        remote.remote = true;
        let mut cfg = Config::default();
        cfg.billing.teleconsultation = 5.0;
        let csv = interviews_csv(&[remote], &cfg);
        assert!(csv.contains(";TPH;;ALD;20,00;70;20,00\r\n"));
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

    /// A session on a scratch database, for the notebook tests.
    fn scratch_session(tag: &str) -> super::Session {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-tab-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::open(&dir.join("live.db"), "secret").unwrap();
        db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        db.add_patient("Martin", "Claire", "1949-02-11").unwrap();
        super::Session::new(db, 12).unwrap()
    }

    #[test]
    fn the_open_tab_follows_the_live_view() {
        let mut s = scratch_session("follow");
        assert_eq!(s.current_tab(), super::WorkTab::Search);
        // Opening a patient by any route — not only by clicking a tab —
        // must be what the strip then points at.
        let p = s.patients[0].clone();
        s.open_patient(p.clone());
        assert_eq!(s.current_tab(), super::WorkTab::Patient(p.id));
        s.note_tab();
        assert!(s.tabs.contains(&super::WorkTab::Patient(p.id)));
        // And noting it twice does not open it twice.
        let before = s.tabs.len();
        s.note_tab();
        assert_eq!(s.tabs.len(), before);
    }

    #[test]
    fn closing_the_open_file_falls_back_to_its_left_neighbour() {
        let mut s = scratch_session("close");
        let (a, b) = (s.patients[0].clone(), s.patients[1].clone());
        s.open_patient(a.clone());
        s.note_tab();
        s.open_patient(b.clone());
        s.note_tab();
        assert_eq!(s.current_tab(), super::WorkTab::Patient(b.id));
        s.close_tab(&super::WorkTab::Patient(b.id));
        assert_eq!(s.current_tab(), super::WorkTab::Patient(a.id));
        assert!(!s.tabs.contains(&super::WorkTab::Patient(b.id)));
    }

    #[test]
    fn the_standing_views_cannot_be_closed() {
        let mut s = scratch_session("standing");
        let before = s.tabs.clone();
        for tab in before.clone() {
            s.close_tab(&tab);
        }
        assert_eq!(s.tabs, before);
    }

    #[test]
    fn cycling_wraps_in_both_directions() {
        let mut s = scratch_session("cycle");
        s.activate_tab(&super::WorkTab::Dashboard);
        assert_eq!(s.current_tab(), super::WorkTab::Dashboard);
        // Dashboard is first: back one wraps to the last standing view.
        s.cycle_tab(-1);
        assert_eq!(s.current_tab(), *s.tabs.last().unwrap());
        s.cycle_tab(1);
        assert_eq!(s.current_tab(), super::WorkTab::Dashboard);
    }

    #[test]
    fn a_tab_whose_file_is_gone_drops_itself() {
        let mut s = scratch_session("gone");
        let p = s.patients[0].clone();
        s.open_patient(p.clone());
        s.note_tab();
        // Another post deletes the patient; the tab survives in the
        // strip until it is next activated, and then removes itself.
        s.db.delete_patient(p.id).unwrap();
        s.set_patients(s.db.patients().unwrap());
        s.activate_tab(&super::WorkTab::Patient(p.id));
        assert!(!s.tabs.contains(&super::WorkTab::Patient(p.id)));
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
