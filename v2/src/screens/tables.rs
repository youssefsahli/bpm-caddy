//! Tables: the shipped reference tables (equivalences, posologies…),
//! with the team's edits to their cells, read-only. Pick a table on the
//! left, filter its rows, read its sources under it.

use super::Ctx;
use crate::t::{t, tf, tn};
use crate::ui;
use bpm_caddy::tables::{ConvTable, FAMILIES, TABLES};
use eframe::egui;
use std::collections::HashMap;

/// The team's edits to one table's cells, keyed `(row, column)`.
type Cells = HashMap<(usize, usize), String>;

#[derive(Default)]
pub struct Tables {
    open: usize,
    filter: String,
    cells: Option<(usize, u64, Cells)>,
}

/// A row as shown: the team's text where it rewrote a cell.
pub fn row_text(
    table: &ConvTable,
    r: usize,
    cells: &HashMap<(usize, usize), String>,
) -> Vec<String> {
    table.rows[r]
        .iter()
        .enumerate()
        .map(|(c, v)| {
            cells
                .get(&(r, c))
                .cloned()
                .unwrap_or_else(|| (*v).to_owned())
        })
        .collect()
}

/// The rows of `table` holding `filter` in any cell (accents and case
/// folded); every row when the filter is empty.
pub fn rows_matching(
    table: &ConvTable,
    cells: &HashMap<(usize, usize), String>,
    filter: &str,
) -> Vec<usize> {
    let q = filter.trim();
    (0..table.rows.len())
        .filter(|&r| {
            q.is_empty()
                || row_text(table, r, cells)
                    .iter()
                    .any(|c| bpm_caddy::fuzzy::contains_folded(c, q))
        })
        .collect()
}

impl Tables {
    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        if TABLES.is_empty() {
            return;
        }
        self.open = self.open.min(TABLES.len() - 1);
        let full = ui.available_rect_before_wrap();
        let list_w = ui::chars_wide(ui, 26.0).min(full.width() * 0.3);
        let gap = ui.spacing().item_spacing.x * 2.0;
        let left = egui::Rect::from_min_size(full.min, egui::vec2(list_w, full.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.right() + gap, full.top()), full.max);
        motif::inside(ui, left, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("tables_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for family in FAMILIES {
                        ui::heading(ui, family);
                        for (i, table) in TABLES.iter().enumerate() {
                            if table.family != family {
                                continue;
                            }
                            if motif::list_row(ui, egui::RichText::new(table.short), i == self.open)
                                .clicked()
                            {
                                self.open = i;
                                self.filter.clear();
                            }
                        }
                    }
                });
        });
        motif::inside(ui, right, |ui| self.table_ui(ui, cx));
    }

    fn table_ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        let i = self.open;
        let table = &TABLES[i];
        if !self
            .cells
            .as_ref()
            .is_some_and(|(k, g, _)| *k == i && *g == cx.s.generation)
        {
            let cells = cx.s.db.table_cells(table.short).unwrap_or_default();
            self.cells = Some((i, cx.s.generation, cells));
        }
        let cells = self
            .cells
            .as_ref()
            .map(|(_, _, c)| c.clone())
            .unwrap_or_default();
        egui::ScrollArea::vertical()
            .id_salt("table_body")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui::title(
                    ui,
                    table.title,
                    Some(&tf("tables.reviewed", &[&table.reviewed])),
                );
                ui::field(ui, &mut self.filter, 30.0, t("tables.filter"));
                let rows = rows_matching(table, &cells, &self.filter);
                if !cells.is_empty() {
                    ui.label(motif::hint(tn("tables.edited", cells.len())));
                }
                if rows.is_empty() {
                    ui::empty(ui, t("tables.no_row"));
                }
                // One block per row: the first cell as its name, every
                // other cell under it with its column. Reads the same at
                // any width and any text scale, which a grid of wrapped
                // cells does not.
                for &r in &rows {
                    let row = row_text(table, r, &cells);
                    let ink = |c: usize| {
                        if cells.contains_key(&(r, c)) {
                            motif::accent()
                        } else {
                            motif::text()
                        }
                    };
                    ui.add_space(ui::line(ui) * 0.3);
                    if let Some(first) = row.first() {
                        ui.label(egui::RichText::new(first).strong().color(ink(0)));
                    }
                    ui.indent(("row", i, r), |ui| {
                        for (c, text) in row.iter().enumerate().skip(1) {
                            if text.trim().is_empty() {
                                continue;
                            }
                            let column = table.columns.get(c).copied().unwrap_or_default();
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(column).color(motif::text_dim()));
                                ui.label(egui::RichText::new(text).color(ink(c)));
                            });
                        }
                    });
                }
                if !table.sources.is_empty() {
                    ui::heading(ui, t("tables.sources"));
                    for (n, s) in table.sources.iter().enumerate() {
                        ui.label(motif::hint(format!("{}. {s}", n + 1)));
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_files_under_a_known_family() {
        for table in TABLES {
            assert!(FAMILIES.contains(&table.family), "{}", table.short);
            for row in table.rows {
                assert!(row.len() <= table.columns.len(), "{}", table.short);
            }
        }
    }

    #[test]
    fn a_team_edit_shows_and_is_found_by_the_filter() {
        let table = &TABLES[0];
        let mut cells = HashMap::new();
        cells.insert((0, 0), "zzqx réécrit".to_owned());
        assert_eq!(row_text(table, 0, &cells)[0], "zzqx réécrit");
        assert_eq!(rows_matching(table, &cells, "ZZQX"), vec![0]);
        assert_eq!(rows_matching(table, &cells, "").len(), table.rows.len());
    }
}
