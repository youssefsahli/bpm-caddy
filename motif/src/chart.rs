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
/// the HP VUE green reads as a stain, not as a rule — and mixed toward
/// whichever bevel is *away* from the trough: on a night skin the
/// trough is already the darkest surface there is, and a rule drawn
/// darker again is a rule nobody reads a number through.
/// The five-per-cent film that marks the column the pointer is over.
///
/// A *veil* and not a colour: it dims a daylight palette and lifts a
/// night one, because « a little darker » says nothing on a ground that
/// is already the darkest thing on the screen.
fn veil() -> Color32 {
    if crate::is_dark() {
        Color32::from_rgba_unmultiplied(255, 255, 255, 14)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 14)
    }
}

/// The baseline of a chart: the same rule, drawn firmly. It is the
/// bevel *away* from the trough for the same reason the gridlines are
/// mixed toward it.
fn axis_color() -> Color32 {
    if crate::is_dark() {
        crate::bg_light()
    } else {
        crate::bg_dark()
    }
}

fn grid_color() -> Color32 {
    let toward = if crate::is_dark() {
        crate::bg_light()
    } else {
        crate::bg_dark()
    };
    crate::trough().lerp_to_gamma(toward, 0.25)
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
/// palettes.
///
/// **Their hue does not move; their lightness is the skin's business.**
/// Written down for a light grey, all seven are mid-dark, and on the two
/// night palettes that is a chart drawn in the background's own tone —
/// eight series and nothing to tell them from the trough they sit in.
/// [`crate::data_ramp`] keeps the hues and lifts the set as one, so two
/// screenshots of the same chart under two skins still show the same
/// series in the same colour family, and no two of them meet on the way.
pub fn series() -> [Color32; SERIES_LEN] {
    let lifted = crate::data_ramp(
        [
            Color32::from_rgb(0x6b, 0x70, 0x82),
            Color32::from_rgb(0x2f, 0x6b, 0x5c),
            Color32::from_rgb(0x8b, 0x1a, 0x1a),
            Color32::from_rgb(0x7a, 0x5c, 0x1f),
            Color32::from_rgb(0x54, 0x3d, 0x73),
            Color32::from_rgb(0x1f, 0x5c, 0x7a),
            Color32::from_rgb(0x6e, 0x3d, 0x2a),
        ],
        crate::AS_FILL,
    );
    [
        crate::accent(),
        lifted[0],
        lifted[1],
        lifted[2],
        lifted[3],
        lifted[4],
        lifted[5],
        lifted[6],
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
    let font = egui::FontId::proportional(crate::pt(ui, 9.5));
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
            Stroke::new(1.0_f32, if i == 0 { axis_color() } else { grid_color() }),
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
            ui.painter().rect_filled(cell, 0.0, veil());
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
                egui::FontId::proportional(crate::pt(ui, 9.5)),
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
/// Each row is `row_h` tall. Returns the hovered row.
///
/// **The caption column measures itself.** It used to be a number of
/// pixels every caller guessed — 96, 130, 150, 160, 200 — and the
/// guesses were wrong in two directions at once: « Méthadone AP-HP
/// gélule » painted straight over its own bar, and at
/// `[ui] text_scale = 1,6` almost every caption did. A count of pixels
/// does not follow the text scale, and the label is drawn here, in a
/// face this function chooses from the row height — so this is the only
/// place that can measure it in the face that will draw it.
///
/// Bounded both ways: never more than half the plot, or a long caption
/// would leave no bar to look at; never less than a few characters. And
/// what still does not fit is elided rather than painted over the bar.
pub fn hbars(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    rows: &[Row],
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
    let text_w = |t: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(t.to_owned(), font.clone(), crate::text())
                .size()
                .x
        })
    };
    // La colonne des valeurs aussi : « 124 h 30 » ne tient pas dans les
    // quarante-six pixels que « 14 » demandait.
    let value_w = rows
        .iter()
        .map(|r| text_w(&fmt(r.value)))
        .fold(0.0_f32, f32::max)
        + 8.0;
    let label_w = rows.iter().map(|r| text_w(r.label)).fold(0.0_f32, f32::max) + 8.0;
    let label_w = label_w.clamp(
        text_w("0000").min(rect.width() * 0.5),
        (rect.width() * 0.5).max(1.0),
    );
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
        // Découpé à sa colonne : sans cela, une longue légende se
        // peint par-dessus la barre qu'elle nomme, et on ne lit plus
        // ni l'une ni l'autre.
        ui.painter()
            .with_clip_rect(egui::Rect::from_min_size(
                egui::pos2(rect.left(), top),
                Vec2::new(label_w - 4.0, row_h),
            ))
            .text(
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

/// Several series in one plot, on **one shared scale**.
///
/// [`sparkline`] scales each call to its own data, which is right for a
/// figure standing beside a number and wrong the moment two curves are
/// meant to be compared: two series with maxima of 100 and 96,9 would be
/// drawn to the same height and read as equal. Here the ceiling is given
/// by the caller, once, and every series is measured against it — so
/// where two curves cross is where they actually cross.
///
/// The floor is zero: these are quantities, and a line chart of a
/// quantity that does not start at zero is a lie about its shape. Only
/// the first series is filled underneath — two washes overlapping read
/// as a third colour that means nothing.
pub fn lines(ui: &egui::Ui, rect: egui::Rect, series: &[(&[f64], Color32)], max: f64) {
    for (n, (values, color)) in series.iter().enumerate() {
        if values.len() < 2 {
            continue;
        }
        let poly = line_points(rect, values, max);
        if n == 0 {
            let mut area = poly.clone();
            area.push(egui::pos2(rect.right(), rect.bottom()));
            area.push(egui::pos2(rect.left(), rect.bottom()));
            ui.painter().add(egui::Shape::convex_polygon(
                area,
                color.gamma_multiply(0.22),
                Stroke::NONE,
            ));
        }
        ui.painter()
            .add(egui::Shape::line(poly, Stroke::new(1.6_f32, *color)));
    }
}

/// Where one series lands inside `rect`, against a ceiling of `max`.
///
/// Split out of [`lines`] because it is the whole of what that function
/// promises and the only part of it a test can reach: a `Painter` needs
/// a live context, arithmetic does not.
///
/// Every value is **clamped** into `0..=max`, and anything that is not
/// a number is read as zero. A series that overshoots the ceiling it was
/// given is a caller's mistake, and the answer to it is a flat line
/// along the top of the plot — not a stroke painted across the panel
/// above, which is what an unclamped `y` would do. `f64::clamp` leaves a
/// NaN a NaN, so that is caught first: a NaN coordinate handed to
/// `Shape::line` is a shape nobody can predict.
fn line_points(rect: egui::Rect, values: &[f64], max: f64) -> Vec<egui::Pos2> {
    let max = max.max(1e-9);
    let step = rect.width() / (values.len().max(2) - 1) as f32;
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let v = if v.is_finite() { *v } else { 0.0 };
            egui::pos2(
                rect.left() + i as f32 * step,
                rect.bottom() - (v.clamp(0.0, max) / max) as f32 * rect.height(),
            )
        })
        .collect()
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
            egui::FontId::proportional(crate::pt(ui, 10.0)),
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
/// The height of one row of [`legend`], so a caller can refuse to draw
/// a legend it cannot show whole.
///
/// **Elle grandit avec le texte.** Écrite à quatorze pixels en dur, la
/// rangée portait un texte de onze pixels — donc dix-sept virgule six à
/// l'échelle 1,6 —, et les deux lignes de la légende de l'agenda se
/// chevauchaient.
pub fn legend_row_height(ui: &egui::Ui) -> f32 {
    crate::pt(ui, 14.0)
}

pub fn legend(ui: &mut egui::Ui, items: &[(&str, Color32)]) {
    // **Peinte rangée par rangée contre le rectangle qu'on lui donne.**
    // `horizontal_wrapped` dessine autant de rangées qu'il en faut et
    // `motif::inside` coupe la dernière en deux : une demi-pastille de
    // couleur ne dit rien de plus qu'une pastille absente. Ici la bande
    // s'arrête d'elle-même sur une rangée entière et compte ce qu'elle
    // laisse — « +4 » dit ce que la demi-rangée cachait.
    let row = legend_row_height(ui);
    let gap_x = crate::pt(ui, 10.0);
    let gap_y = 2.0;
    let pad = crate::pt(ui, 18.0);
    let swatch_w = crate::pt(ui, 10.0);
    let font = egui::FontId::proportional(crate::pt(ui, 11.0));
    let (widths, marker_w) = ui.fonts(|f| {
        let w = |s: &str| {
            f.layout_no_wrap(s.to_owned(), font.clone(), crate::text())
                .size()
                .x
        };
        (
            items.iter().map(|(l, _)| w(l) + pad).collect::<Vec<f32>>(),
            w("+99") + pad,
        )
    });
    // Depuis le curseur, pas depuis le haut du panneau : la légende
    // s'écrit *sous* la courbe qu'elle explique, et `max_rect` commence
    // là où le panneau commence.
    let area = egui::Rect::from_min_max(
        ui.cursor().min,
        egui::pos2(ui.max_rect().right(), ui.max_rect().bottom()),
    );
    let (mut x, mut y) = (area.left(), area.top());
    let mut drawn = 0usize;
    let mut bottom = area.top();
    // Où le compte s'écrira : au bout de la dernière pastille dessinée,
    // et non là où la boucle s'est arrêtée — elle peut s'être arrêtée
    // sur une rangée qui n'existe pas.
    let mut mark = egui::pos2(area.left(), area.top());
    for (i, ((label, color), w)) in items.iter().zip(&widths).enumerate() {
        if x + w > area.right() && x > area.left() {
            x = area.left();
            y += row + gap_y;
        }
        // La rangée suivante ne tient pas : ce qui reste se compte.
        if y + row > area.bottom() + 0.5 {
            break;
        }
        // Sur la dernière rangée possible, on garde la place du compte.
        let last_row = y + row + gap_y + row > area.bottom() + 0.5;
        if last_row && i + 1 < items.len() && x + w + gap_x + marker_w > area.right() {
            break;
        }
        let rect = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(*w, row));
        let swatch = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.center().y - swatch_w / 2.0),
            Vec2::splat(swatch_w),
        );
        ui.painter().rect_filled(swatch, 0.0, *color);
        ui.painter()
            .rect_stroke(swatch, 0.0, Stroke::new(1.0_f32, crate::bg_dark()));
        ui.painter().text(
            egui::pos2(rect.left() + swatch_w + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            font.clone(),
            crate::text_dim(),
        );
        x += w + gap_x;
        bottom = rect.bottom();
        mark = egui::pos2(x, y);
        drawn += 1;
    }
    let hidden = items.len() - drawn;
    if hidden > 0 && mark.x + marker_w <= area.right() + 0.5 {
        ui.painter().text(
            egui::pos2(mark.x, mark.y + row / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("+{hidden}"),
            font,
            crate::text_dim(),
        );
        bottom = bottom.max(mark.y + row);
    }
    // La place réellement prise, pour que ce qui suit s'écrive dessous.
    ui.allocate_rect(
        egui::Rect::from_min_max(area.min, egui::pos2(area.right(), bottom)),
        egui::Sense::hover(),
    );
}

#[cfg(test)]
mod tests {
    use super::nice_max;

    /// One scale for every series, which is the only reason [`lines`]
    /// exists beside [`sparkline`].
    ///
    /// The case it was written for is the drug card's two half-life
    /// curves: what is left after the last dose, falling from 100, and
    /// how far a regular dose has come, rising to 96,9. Drawn by two
    /// `sparkline` calls each scaled to its own data, the 96,9 plateau
    /// and the 100 start land on the same pixel and the card says the
    /// two are equal. Here they cannot.
    #[test]
    fn one_ceiling_holds_every_series_and_nothing_escapes_the_plot() {
        use super::{egui, line_points};
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 40.0));

        // The ceiling is the caller's, not the data's: a series that
        // stops short of it is drawn short of the top.
        let full = line_points(rect, &[100.0, 100.0], 100.0);
        let nearly = line_points(rect, &[96.875, 96.875], 100.0);
        assert!(
            (full[0].y - rect.top()).abs() < 0.01,
            "100 % touche le haut"
        );
        assert!(
            nearly[0].y > full[0].y,
            "96,9 % doit être dessiné sous 100 %"
        );

        // Zero is the floor, always: a quantity plotted off a shifted
        // baseline lies about its own shape.
        let none = line_points(rect, &[0.0, 0.0], 100.0);
        assert!((none[0].y - rect.bottom()).abs() < 0.01);

        // The two curves cross where the arithmetic says they cross —
        // at half the ceiling, one half-life in.
        let a = line_points(rect, &[50.0, 50.0], 100.0);
        assert!((a[0].y - rect.center().y).abs() < 0.01);

        // The ends sit on the edges, and the points march across.
        assert!((full[0].x - rect.left()).abs() < 0.01);
        assert!((full[1].x - rect.right()).abs() < 0.01);

        // A series that overshoots its ceiling, or dips below zero, is
        // flattened onto the plot's own edges — never painted across the
        // panel above or below it.
        for p in line_points(rect, &[500.0, -80.0, f64::NAN], 100.0) {
            assert!(
                p.y >= rect.top() - 0.01 && p.y <= rect.bottom() + 0.01,
                "{p:?} sort du cadre"
            );
        }
        // And a ceiling of zero divides by nothing.
        for p in line_points(rect, &[0.0, 1.0], 0.0) {
            assert!(p.y.is_finite(), "{p:?}");
        }
    }

    /// The data colours have to stand off the trough they are painted
    /// on, on every palette, and none of them may be another one.
    ///
    /// The readability half is the one that matters: a bar the colour of
    /// its own background is a bar nobody sees, and it would reach the
    /// counter unnoticed because a chart always *looks* like a chart.
    ///
    /// The separation half is deliberately a ratchet and not a rule.
    /// The closest pair today is the amber of series 4 and the brown of
    /// series 7, a hair over thirty-five apart in plain RGB; the legend
    /// is what tells them apart, and repainting them would change every
    /// screenshot and every printed sheet for a difference the officine
    /// has not asked for. The test holds the line where it is: no new
    /// colour may come closer than the closest pair already there.
    ///
    /// **The ratchet is on the ramp as it is written**, and what each
    /// skin owes is that adapting it does not close those gaps: a tenth
    /// is the whole allowance, and it is there for the rounding to a
    /// byte, not for a design decision. Held that way round because it
    /// is the truthful reading — the hues are chosen once and `data_ramp`
    /// promises to carry them, not to re-choose them.
    #[test]
    fn the_data_colours_read_on_every_palette_and_none_repeats_another() {
        use super::Color32;
        let _guard = crate::theme_lock();
        let lum = crate::luminance;
        // Distance in plain RGB: cheap, and enough to catch two colours
        // that would land on the same bar.
        fn apart(a: Color32, b: Color32) -> f32 {
            let d = |x: u8, y: u8| (x as f32 - y as f32).powi(2);
            (d(a.r(), b.r()) + d(a.g(), b.g()) + d(a.b(), b.b())).sqrt()
        }
        // Every assertion is made under the theme in force, because the
        // ramp *is* read off it: series 0 is the accent, and the seven
        // others are lifted toward the shell they are painted on. Read
        // once, outside the loop, they would be one palette's ramp
        // checked against another palette's trough — which is what this
        // test used to do.
        // The ramp as written: the seven fixed hues, read off the
        // palette the application starts on, where nothing is adapted.
        crate::set_theme(crate::THEMES[0].key);
        let written: Vec<Color32> = super::series()[1..].to_vec();
        assert_eq!(written.len(), super::SERIES_LEN - 1);
        for (i, a) in written.iter().enumerate() {
            for (j, b) in written.iter().enumerate().skip(i + 1) {
                assert!(
                    apart(*a, *b) > 35.0,
                    "les séries {} et {} sont plus proches que la paire la plus proche d'aujourd'hui ({:?} / {:?})",
                    i + 1,
                    j + 1,
                    a,
                    b
                );
            }
        }
        for t in crate::THEMES.iter() {
            crate::set_theme(t.key);
            let ramp = super::series();
            let fixed: Vec<Color32> = ramp[1..].to_vec();
            for (i, a) in fixed.iter().enumerate() {
                for (j, b) in fixed.iter().enumerate().skip(i + 1) {
                    assert!(
                        apart(*a, *b) >= apart(written[i], written[j]) * 0.9,
                        "{} : la peau a rapproché les séries {} et {} ({:?} / {:?})",
                        t.key,
                        i + 1,
                        j + 1,
                        a,
                        b
                    );
                }
            }
            // A chart is painted in the trough and not on the panel:
            // that is the ground the data has to stand off.
            for (i, c) in ramp.iter().enumerate() {
                assert!(
                    (lum(crate::trough()) - lum(*c)).abs() > 0.12,
                    "{} : la série {i} se perd dans le fond du graphique",
                    t.key
                );
            }
            // Series 0 *is* the accent on this theme. It may sit in the
            // same family as one of the seven — two of the palettes are
            // built on a teal and a green — but it must not be one of
            // them, or a chart would draw two series identically.
            for (i, c) in fixed.iter().enumerate() {
                assert!(
                    apart(crate::accent(), *c) > 12.0,
                    "{} : l'accent est la série {}",
                    t.key,
                    i + 1
                );
            }
        }
        crate::set_theme(crate::THEMES[0].key);
    }

    /// The ramp wraps rather than panicking: a chart with more series
    /// than colours is a chart that repeats, not a crash at the counter.
    #[test]
    fn the_colour_ramp_wraps_instead_of_running_out() {
        // `series_color(0)` **is** the accent of the theme in force,
        // and the theme is process-wide: without this the two halves of
        // an assertion could be read under two different skins, and the
        // test failed once in ten runs saying nothing about wrapping.
        let _guard = crate::theme_lock();
        for i in [0usize, 7, 8, 15, 1_000, usize::MAX] {
            let _ = super::series_color(i);
        }
        assert_eq!(
            super::series_color(0),
            super::series_color(super::SERIES_LEN)
        );
        assert_eq!(
            super::series_color(3),
            super::series_color(super::SERIES_LEN + 3)
        );
    }

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
