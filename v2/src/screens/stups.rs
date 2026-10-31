//! Stupéfiants: where each product stands, and the last lines of the
//! register. Read-only in v2 — the register is INSERT-only by law and
//! its entry form (with the LGO's ordonnancier number) stays in the
//! first window, where its guards and its tests live.

use super::{Ctx, Go};
use crate::t::{t, tn};
use crate::ui;
use bpm_caddy::db::{Standing, StupMove};
use bpm_caddy::ordonnancier::Kind;
use eframe::egui;

#[derive(Default)]
pub struct Stups {
    memo: Option<(u64, Vec<Standing>, Vec<StupMove>)>,
    filter: String,
    archived: bool,
}

/// A quantity as the register writes it: whole when it is.
fn qty(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        bpm_caddy::strings::decimal(v, 2)
    }
}

/// The products under their alert threshold, worst first.
pub fn low(standing: &[Standing]) -> Vec<&Standing> {
    let mut out: Vec<&Standing> = standing
        .iter()
        .filter(|s| {
            !s.product.archived && s.product.threshold > 0.0 && s.stock <= s.product.threshold
        })
        .collect();
    out.sort_by(|a, b| (a.stock - a.product.threshold).total_cmp(&(b.stock - b.product.threshold)));
    out
}

impl Stups {
    fn load(&mut self, cx: &mut Ctx) {
        if self
            .memo
            .as_ref()
            .is_some_and(|(g, _, _)| *g == cx.s.generation)
        {
            return;
        }
        let mut standing = cx.s.db.stup_summary().unwrap_or_default();
        standing.sort_by_cached_key(|s| bpm_caddy::fuzzy::sort_key(&s.product.label));
        let recent = cx.s.db.stup_recent(40).unwrap_or_default();
        self.memo = Some((cx.s.generation, standing, recent));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        self.load(cx);
        let Some((_, standing, recent)) = self.memo.clone() else {
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt("stups")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui::title(ui, t("stups.title"), Some(t("stups.sub")));
                let short = low(&standing);
                if !short.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        motif::badge(
                            ui,
                            &tn("stups.low", short.len()),
                            Some(motif::Pict::Warn),
                            motif::warn(),
                            false,
                        );
                        for s in short.iter().take(6) {
                            ui.label(egui::RichText::new(&s.product.label).color(motif::warn()));
                        }
                    });
                }
                ui::heading(ui, t("stups.stock"));
                ui.horizontal(|ui| {
                    ui::field(ui, &mut self.filter, 24.0, t("stups.filter"));
                    motif::checkbox(ui, &mut self.archived, t("stups.archived"));
                });
                let shown: Vec<&Standing> = standing
                    .iter()
                    .filter(|s| self.archived || !s.product.archived)
                    .filter(|s| {
                        self.filter.trim().is_empty()
                            || bpm_caddy::fuzzy::contains_folded(
                                &s.product.label,
                                self.filter.trim(),
                            )
                    })
                    .collect();
                if shown.is_empty() {
                    ui::empty(ui, t("stups.none"));
                }
                egui::Grid::new("stup_stock")
                    .striped(true)
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.label(motif::hint(t("stups.product")));
                        ui.label(motif::hint(t("stups.in_stock")));
                        ui.label(motif::hint(t("stups.to_destroy")));
                        ui.label(motif::hint(t("stups.counted")));
                        ui.end_row();
                        for s in shown {
                            let under = s.product.threshold > 0.0 && s.stock <= s.product.threshold;
                            ui.horizontal(|ui| {
                                if under {
                                    ui::pict(ui, motif::Pict::Warn, motif::warn());
                                }
                                ui.label(&s.product.label);
                            });
                            let stock =
                                egui::RichText::new(format!("{} {}", qty(s.stock), s.product.unit));
                            ui.label(if under {
                                stock.color(motif::warn()).strong()
                            } else {
                                stock
                            });
                            ui.label(if s.to_destroy > 0.0 {
                                qty(s.to_destroy)
                            } else {
                                String::new()
                            });
                            ui.label(ui::date(&s.last_count));
                            ui.end_row();
                        }
                    });

                ui::heading(ui, t("stups.recent"));
                if recent.is_empty() {
                    ui::empty(ui, t("stups.no_move"));
                }
                let label_of = |id: i64| {
                    standing
                        .iter()
                        .find(|s| s.product.id == id)
                        .map(|s| s.product.label.clone())
                        .unwrap_or_default()
                };
                let mut open = None;
                egui::Grid::new("stup_recent")
                    .striped(true)
                    .num_columns(5)
                    .show(ui, |ui| {
                        for m in &recent {
                            ui.label(ui::date(&m.happened_on));
                            ui.label(bpm_caddy::strings::tr(Kind::from_key(&m.kind).label_key()));
                            ui.label(label_of(m.stup_id));
                            ui.label(qty(m.quantity));
                            if m.ordo_no > 0 {
                                ui.label(motif::hint(format!("{}/{}", m.ordo_year, m.ordo_no)));
                            } else {
                                ui.label("");
                            }
                            if m.patient_id > 0 && motif::button(ui, t("stups.file")).clicked() {
                                open = Some(m.patient_id);
                            }
                            ui.end_row();
                        }
                    });
                if let Some(pid) = open {
                    cx.go(Go::Patient(pid));
                }
                ui.add_space(ui::line(ui));
                ui.label(motif::hint(t("stups.entry_elsewhere")));
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_read_whole_when_they_are() {
        assert_eq!(qty(14.0), "14");
        assert!(qty(2.5).starts_with('2'));
    }

    #[test]
    fn an_empty_base_has_nothing_low() {
        let (s, dir) = crate::session::tests::scratch("stups");
        let standing = s.db.stup_summary().unwrap();
        assert!(low(&standing).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
