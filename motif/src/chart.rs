//! Hand-painted charts in the Motif idiom.
//!
//! No plotting library (see `CLAUDE.md`): a chart here is a sunken
//! trough with flat rectangles in it, gridded like a paper form. Every
//! chart takes the rectangle it must fill — the caller carves the
//! layout — and returns which element the pointer is over, so the view
//! can attach a tooltip without the chart knowing about strings.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::bevel;

/// Gridlines inside a trough: light enough to read numbers through.
///
/// Mixed from the theme rather than fixed — a blue-grey grid drawn on
/// the HP VUE green reads as a stain, not as a rule.
fn grid_color() -> Color32 {
    crate::trough().lerp_to_gamma(crate::bg_dark(), 0.25)
}

/// How many colours the categorical ramp has.
pub const SERIES_LEN: usize = 8;

/// The categorical ramp: distinct at a glance on the Motif grey, and
/// ordered so the first two are the ones every chart uses — the theme's
/// own accent for what is done, a shadow grey for what is not.
///
/// A function and no longer a constant: the first colour follows the
/// theme, so a chart on the HP VUE green does not go on drawing its
/// first series in the Motif blue. The other seven are data colours,
/// chosen to stay apart from each other and readable on every one of the
/// palettes; they do not move.
pub fn series() -> [Color32; SERIES_LEN] {
    [
        crate::accent(),
        Color32::from_rgb(0x6b, 0x70, 0x82),
        Color32::from_rgb(0x2f, 0x6b, 0x5c),
        Color32::from_rgb(0x8b, 0x1a, 0x1a),
        Color32::from_rgb(0x7a, 0x5c, 0x1f),
        Color32::from_rgb(0x54, 0x3d, 0x73),
        Color32::from_rgb(0x1f, 0x5c, 0x7a),
        Color32::from_rgb(0x6e, 0x3d, 0x2a),
    ]
}

/// One colour of the ramp, wrapping round.
pub fn series_color(i: usize) -> Color32 {
    series()[i % SERIES_LEN]
}

/// Sink `rect` into a trough and return the plotting interior.
pub fn frame(ui: &egui::Ui, rect: egui::Rect) -> egui::Rect {
    ui.painter().rect_filled(rect, 0.0, crate::trough());
    bevel(ui.painter(), rect, false);
    rect.shrink(4.0)
}

/// Round `max` up to a readable axis top (1-2-5 ladder).
pub fn nice_max(max: f64) -> f64 {
    if max <= 0.0 {
        return 1.0;
    }
    let mag = 10_f64.powf(max.log10().floor());
    let n = max / mag;
    mag * if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    }
}

/// Horizontal gridlines with their values down the left gutter.
/// Returns the plot area left of the labels.
fn grid(
    ui: &egui::Ui,
    plot: egui::Rect,
    max: f64,
    steps: usize,
    fmt: &dyn Fn(f64) -> String,
) -> egui::Rect {
    let font = egui::FontId::proportional(9.5);
    let gutter = (0..=steps)
        .map(|i| {
            let v = max * i as f64 / steps as f64;
            ui.painter()
                .layout_no_wrap(fmt(v), font.clone(), crate::text_faint())
                .size()
                .x
        })
        .fold(0.0_f32, f32::max)
        + 6.0;
    let area = egui::Rect::from_min_max(egui::pos2(plot.left() + gutter, plot.top()), plot.max);
    for i in 0..=steps {
        let v = max * i as f64 / steps as f64;
        let y = area.bottom() - (area.height() - 16.0) * (i as f32 / steps as f32);
        ui.painter().line_segment(
            [egui::pos2(area.left(), y), egui::pos2(area.right(), y)],
            Stroke::new(
                1.0_f32,
                if i == 0 {
                    crate::bg_dark()
                } else {
                    grid_color()
                },
            ),
        );
        ui.painter().text(
            egui::pos2(area.left() - 4.0, y),
            egui::Align2::RIGHT_CENTER,
            fmt(v),
            font.clone(),
            crate::text_faint(),
        );
    }
    area
}

/// One column of a [`bars`] chart: a caption and one value per series.
pub struct Group<'a> {
    pub label: &'a str,
    pub values: &'a [f64],
}

/// Grouped vertical bars with a value grid. `colors` is indexed by
/// series; `fmt` formats the axis labels. Returns the hovered group.
pub fn bars(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    groups: &[Group],
    colors: &[Color32],
    fmt: &dyn Fn(f64) -> String,
) -> Option<usize> {
    let plot = frame(ui, rect);
    if groups.is_empty() {
        return None;
    }
    let max = nice_max(
        groups
            .iter()
            .flat_map(|g| g.values.iter().copied())
            .fold(0.0_f64, f64::max),
    );
    let area = grid(ui, plot, max, 4, fmt);
    let resp = ui.interact(rect, ui.id().with("motif_bars"), egui::Sense::hover());
    let pointer = resp.hover_pos();
    let slot = area.width() / groups.len() as f32;
    let floor = area.bottom();
    let usable = area.height() - 16.0;
    let mut hovered = None;
    for (i, g) in groups.iter().enumerate() {
        let x0 = area.left() + i as f32 * slot;
        let cell = egui::Rect::from_min_max(
            egui::pos2(x0, area.top()),
            egui::pos2(x0 + slot, area.bottom()),
        );
        let over = pointer.is_some_and(|p| cell.contains(p));
        if over {
            hovered = Some(i);
            ui.painter()
                .rect_filled(cell, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 14));
        }
        let n = g.values.len().max(1);
        let bar_w = ((slot - 10.0) / n as f32).clamp(3.0, 26.0);
        let span = bar_w * n as f32;
        for (k, v) in g.values.iter().enumerate() {
            let h = ((v / max) as f32 * usable).max(if *v > 0.0 { 1.0 } else { 0.0 });
            let left = x0 + slot / 2.0 - span / 2.0 + k as f32 * bar_w;
            let bar = egui::Rect::from_min_max(
                egui::pos2(left, floor - h),
                egui::pos2(left + bar_w - 1.0, floor),
            );
            let color = colors.get(k).copied().unwrap_or(crate::accent());
            ui.painter().rect_filled(bar, 0.0, color);
            if h > 3.0 {
                // A one-pixel highlight on the top edge: the bars read
                // as solid blocks rather than flat fills.
                ui.painter().line_segment(
                    [bar.left_top(), bar.right_top()],
                    Stroke::new(1.0_f32, crate::bg_light().gamma_multiply(0.5)),
                );
            }
        }
        // Captions rotate out of the way by thinning: when the slots get
        // narrow, only every other one is drawn.
        let every = (60.0 / slot).ceil() as usize;
        if every <= 1 || i % every == 0 {
            ui.painter().text(
                egui::pos2(x0 + slot / 2.0, area.bottom() + 8.0),
                egui::Align2::CENTER_CENTER,
                g.label,
                egui::FontId::proportional(9.5),
                if over {
                    crate::text()
                } else {
                    crate::text_dim()
                },
            );
        }
    }
    hovered
}

/// A row of a [`hbars`] list: caption, value, and the bar's colour.
pub struct Row<'a> {
    pub label: &'a str,
    pub value: f64,
    pub color: Color32,
}

/// A list of horizontal bars — the shape a funnel or a ranking wants.
/// Each row is `row_h` tall; the caption column is `label_w` wide.
/// Returns the hovered row.
pub fn hbars(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    rows: &[Row],
    label_w: f32,
    fmt: &dyn Fn(f64) -> String,
) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let max = rows
        .iter()
        .map(|r| r.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let row_h = (rect.height() / rows.len() as f32).clamp(14.0, 30.0);
    let font = egui::FontId::proportional((row_h * 0.46).clamp(10.0, 13.0));
    let value_w = 46.0;
    let mut hovered = None;
    let pointer = ui
        .interact(rect, ui.id().with("motif_hbars"), egui::Sense::hover())
        .hover_pos();
    for (i, r) in rows.iter().enumerate() {
        let top = rect.top() + i as f32 * row_h;
        let line =
            egui::Rect::from_min_size(egui::pos2(rect.left(), top), Vec2::new(rect.width(), row_h));
        let over = pointer.is_some_and(|p| line.contains(p));
        if over {
            hovered = Some(i);
            ui.painter().rect_filled(line, 0.0, crate::bg_hover());
        }
        ui.painter().text(
            egui::pos2(rect.left() + 2.0, line.center().y),
            egui::Align2::LEFT_CENTER,
            r.label,
            font.clone(),
            crate::text(),
        );
        let trough = egui::Rect::from_min_max(
            egui::pos2(rect.left() + label_w, top + 3.0),
            egui::pos2(rect.right() - value_w, top + row_h - 3.0),
        );
        if trough.width() < 4.0 {
            continue;
        }
        ui.painter().rect_filled(trough, 0.0, crate::trough());
        bevel(ui.painter(), trough, false);
        if r.value > 0.0 {
            let mut fill = trough.shrink(3.0);
            fill.set_width(fill.width() * (r.value / max) as f32);
            ui.painter().rect_filled(fill, 0.0, r.color);
        }
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, line.center().y),
            egui::Align2::RIGHT_CENTER,
            fmt(r.value),
            font.clone(),
            crate::text(),
        );
    }
    hovered
}

/// A single 100 %-stacked bar: composition at a glance, in one row of
/// pixels the eye reads as a whole. Segments under 2 % are still drawn
/// (one pixel) so nothing silently disappears.
pub fn stacked(ui: &egui::Ui, rect: egui::Rect, parts: &[(f64, Color32)]) {
    let total: f64 = parts.iter().map(|(v, _)| *v).sum();
    ui.painter().rect_filled(rect, 0.0, crate::trough());
    bevel(ui.painter(), rect, false);
    if total <= 0.0 {
        return;
    }
    let inner = rect.shrink(3.0);
    let mut x = inner.left();
    for (v, color) in parts {
        if *v <= 0.0 {
            continue;
        }
        let w = (inner.width() * (*v / total) as f32).max(1.0);
        let seg = egui::Rect::from_min_max(
            egui::pos2(x, inner.top()),
            egui::pos2((x + w).min(inner.right()), inner.bottom()),
        );
        ui.painter().rect_filled(seg, 0.0, *color);
        x += w;
    }
}

/// A sparkline: the shape of a series, no axes, no labels. `rect` is
/// usually a strip 24-40 px tall beside a number.
pub fn sparkline(ui: &egui::Ui, rect: egui::Rect, values: &[f64], color: Color32) {
    if values.len() < 2 {
        return;
    }
    let max = values.iter().copied().fold(0.0_f64, f64::max).max(1e-9);
    let min = values.iter().copied().fold(max, f64::min).min(0.0);
    let span = (max - min).max(1e-9);
    let step = rect.width() / (values.len() - 1) as f32;
    let pt = |i: usize, v: f64| {
        egui::pos2(
            rect.left() + i as f32 * step,
            rect.bottom() - ((v - min) / span) as f32 * rect.height(),
        )
    };
    // Fill under the line first, in a washed-out tint of the stroke.
    let mut poly: Vec<egui::Pos2> = values.iter().enumerate().map(|(i, v)| pt(i, *v)).collect();
    let area = {
        let mut p = poly.clone();
        p.push(egui::pos2(rect.right(), rect.bottom()));
        p.push(egui::pos2(rect.left(), rect.bottom()));
        p
    };
    ui.painter().add(egui::Shape::convex_polygon(
        area,
        color.gamma_multiply(0.28),
        Stroke::NONE,
    ));
    ui.painter().add(egui::Shape::line(
        std::mem::take(&mut poly),
        Stroke::new(1.6_f32, color),
    ));
    // The last point is the one that matters: mark it.
    let last = pt(values.len() - 1, values[values.len() - 1]);
    ui.painter().rect_filled(
        egui::Rect::from_center_size(last, Vec2::splat(4.0)),
        0.0,
        color,
    );
}

/// A segmented meter — the Motif way to show a fraction of a quota.
/// `warn` above 1.0 turns the overflow segments red.
pub fn meter(ui: &egui::Ui, rect: egui::Rect, fraction: f32, color: Color32) {
    ui.painter().rect_filled(rect, 0.0, crate::trough());
    bevel(ui.painter(), rect, false);
    let inner = rect.shrink(3.0);
    let cells = ((inner.width() / 7.0).floor() as usize).clamp(4, 40);
    let lit = (fraction.clamp(0.0, 1.0) * cells as f32).round() as usize;
    let w = inner.width() / cells as f32;
    for i in 0..cells {
        if i >= lit {
            break;
        }
        let cell = egui::Rect::from_min_size(
            egui::pos2(inner.left() + i as f32 * w, inner.top()),
            Vec2::new(w - 1.5, inner.height()),
        );
        ui.painter().rect_filled(cell, 0.0, color);
    }
    if fraction > 1.0 {
        // Over quota: a hard red cap on the right edge.
        let cap = egui::Rect::from_min_max(egui::pos2(inner.right() - 4.0, inner.top()), inner.max);
        ui.painter().rect_filled(cap, 0.0, crate::alert());
    }
}

/// Discrete pips: `filled` of `total` cells lit. Where [`meter`] shows a
/// proportion, this shows a count — four entretiens in a sequence are
/// four squares, not 50 % of a bar.
pub fn pips(ui: &egui::Ui, rect: egui::Rect, filled: usize, total: usize, color: Color32) {
    let total = total.max(1);
    let gap = 2.0_f32;
    let w = ((rect.width() - gap * (total - 1) as f32) / total as f32).max(3.0);
    for i in 0..total {
        let cell = egui::Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * (w + gap), rect.top()),
            Vec2::new(w, rect.height()),
        );
        if i < filled {
            ui.painter().rect_filled(cell, 0.0, color);
            bevel(ui.painter(), cell, true);
        } else {
            ui.painter().rect_filled(cell, 0.0, crate::trough());
            bevel(ui.painter(), cell, false);
        }
    }
    // Past the sequence's length there is nothing left to bill: say so
    // rather than silently drawing a full row.
    if filled > total {
        ui.painter().text(
            egui::pos2(rect.right() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("+{}", filled - total),
            egui::FontId::proportional(10.0),
            crate::alert(),
        );
    }
}

/// A calendar heat strip: one cell per day, shaded by intensity.
/// Returns the hovered cell index.
pub fn heat_strip(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    values: &[f64],
    color: Color32,
) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    let max = values.iter().copied().fold(0.0_f64, f64::max).max(1.0);
    let w = rect.width() / values.len() as f32;
    let pointer = ui
        .interact(rect, ui.id().with("motif_heat"), egui::Sense::hover())
        .hover_pos();
    let mut hovered = None;
    for (i, v) in values.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * w, rect.top()),
            Vec2::new((w - 1.0).max(1.0), rect.height()),
        );
        let t = (*v / max) as f32;
        let fill = if *v <= 0.0 {
            crate::trough()
        } else {
            color.gamma_multiply(0.25 + 0.75 * t)
        };
        ui.painter().rect_filled(cell, 0.0, fill);
        if pointer.is_some_and(|p| cell.contains(p)) {
            hovered = Some(i);
            ui.painter()
                .rect_stroke(cell, 0.0, Stroke::new(1.0_f32, crate::text()));
        }
    }
    hovered
}

/// A legend row: a swatch and a caption per series, wrapped.
pub fn legend(ui: &mut egui::Ui, items: &[(&str, Color32)]) {
    ui.horizontal_wrapped(|ui| {
        // A legend is a caption, not a form: it wraps tightly, so the
        // strip a caller reserves for it is the strip it needs.
        ui.spacing_mut().item_spacing = Vec2::new(10.0, 2.0);
        for (label, color) in items {
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(
                    ui.fonts(|f| {
                        f.layout_no_wrap(
                            (*label).to_owned(),
                            egui::FontId::proportional(11.0),
                            crate::text(),
                        )
                        .size()
                        .x
                    }) + 18.0,
                    14.0,
                ),
                egui::Sense::hover(),
            );
            let swatch = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - 5.0),
                Vec2::splat(10.0),
            );
            ui.painter().rect_filled(swatch, 0.0, *color);
            ui.painter()
                .rect_stroke(swatch, 0.0, Stroke::new(1.0_f32, crate::bg_dark()));
            ui.painter().text(
                egui::pos2(rect.left() + 14.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                *label,
                egui::FontId::proportional(11.0),
                crate::text_dim(),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::nice_max;

    #[test]
    fn axis_tops_land_on_the_one_two_five_ladder() {
        assert_eq!(nice_max(0.0), 1.0);
        assert_eq!(nice_max(7.0), 10.0);
        assert_eq!(nice_max(12.0), 20.0);
        assert_eq!(nice_max(430.0), 500.0);
        assert_eq!(nice_max(1000.0), 1000.0);
    }

    #[test]
    fn a_nice_max_is_never_below_its_input() {
        for v in [1.0, 3.3, 55.0, 99.0, 101.0, 4999.0] {
            assert!(nice_max(v) >= v, "{v}");
        }
    }
}
