//! Old-school X/Motif look-and-feel for egui.
//!
//! Reproduces the classic `mwm` appearance: a blue-grey palette, square
//! corners, and two-pixel light/dark bevels that make widgets look raised
//! (buttons, panels) or sunken (text fields, troughs).

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

pub mod chart;
pub mod layout;

pub use layout::{
    column_count, inside, page, panel, rule, split_columns, split_rows, tab_strip, visible_rect,
    vrule, well, Tab, TabAction,
};

/// Classic mwm blue-grey widget background.
pub const BG: Color32 = Color32::from_rgb(0xae, 0xb2, 0xc3);
/// Top/left bevel highlight.
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xde, 0xe1, 0xec);
/// Bottom/right bevel shadow.
pub const BG_DARK: Color32 = Color32::from_rgb(0x5c, 0x5f, 0x6e);
/// Sunken areas: text fields, progress troughs.
pub const TROUGH: Color32 = Color32::from_rgb(0x94, 0x98, 0xaa);
/// Selection / active fill (Motif active-frame blue).
pub const ACCENT: Color32 = Color32::from_rgb(0x3a, 0x54, 0x7e);
/// Hover tint, slightly lighter than `BG`.
pub const BG_HOVER: Color32 = Color32::from_rgb(0xbb, 0xbf, 0xcf);
pub const TEXT: Color32 = Color32::BLACK;
/// Secondary text on `BG`. `BG_DARK` is the bevel shadow and is far too
/// light to read a label in: labels that used it were barely legible on
/// the widget grey, and unreadable once the screen was dimmed.
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x39, 0x3c, 0x48);
/// Third-level text: captions, units, timestamps. Still legible.
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x4c, 0x50, 0x5e);
/// Errors and destructive warnings (Motif dark red).
pub const ALERT: Color32 = Color32::from_rgb(0x8b, 0x1a, 0x1a);

/// Paper: the sheet a printed monograph would be read on, for the
/// document-style views inside the grey Motif shell.
pub const PAPER: Color32 = Color32::from_rgb(0xf6, 0xf4, 0xec);
/// Ink on that sheet, and the lighter shade for secondary lines.
pub const INK: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x20);
pub const INK_LIGHT: Color32 = Color32::from_rgb(0x55, 0x55, 0x60);

/// Draw a sheet of paper: flat fill, a thin ink border and a hard
/// shadow to the lower right, in keeping with the square Motif look.
pub fn sheet(painter: &egui::Painter, rect: egui::Rect) {
    let shadow = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0),
        egui::pos2(rect.max.x + 4.0, rect.max.y + 4.0),
    );
    painter.rect_filled(shadow, 0.0, BG_DARK);
    painter.rect_filled(rect, 0.0, PAPER);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, INK_LIGHT));
}

/// Lay content out in a fixed-width column centered in the available
/// space — the page grid every view aligns to. Content inside is
/// normal top-down, left-aligned layout.
pub fn column<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let avail = ui.available_rect_before_wrap();
    // Centre on what is actually on screen, not on what the panel
    // claimed: a side panel that grew after reserving its width leaves
    // `avail` wider than the visible area, and a column centred on it
    // would have its right edge cut away — buttons included.
    let visible = avail.intersect(ui.clip_rect());
    let visible = if visible.width() > 100.0 {
        visible
    } else {
        avail
    };
    // Always keep side margins, even when the panel is narrower than
    // the requested column.
    let w = width.min(visible.width() - 48.0).max(200.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(visible.center().x - w / 2.0, avail.top()),
        Vec2::new(w, avail.height()),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), add)
        .inner
}

/// Programmatic 32×32 window icon: a raised Motif bevel square with a
/// sunken accent centre — no embedded asset needed.
// Indices are used symmetrically (px[b][i] and px[i][b]): an iterator
// rewrite would obscure the mirroring.
#[allow(clippy::needless_range_loop)]
pub fn icon() -> egui::IconData {
    const N: usize = 32;
    let to4 = |c: Color32| [c.r(), c.g(), c.b(), 0xff];
    let (bg, light, dark, accent) = (to4(BG), to4(BG_LIGHT), to4(BG_DARK), to4(ACCENT));
    let mut px = [[bg; N]; N];
    for i in 0..N {
        for b in 0..2 {
            px[b][i] = light;
            px[i][b] = light;
            px[N - 1 - b][i] = dark;
            px[i][N - 1 - b] = dark;
        }
    }
    // Sunken inner square (bevel inverted), filled with the accent blue.
    for i in 8..N - 8 {
        for j in 8..N - 8 {
            px[i][j] = accent;
        }
    }
    for i in 8..N - 8 {
        px[8][i] = dark;
        px[i][8] = dark;
        px[N - 9][i] = light;
        px[i][N - 9] = light;
    }
    egui::IconData {
        rgba: px.iter().flatten().flatten().copied().collect(),
        width: N as u32,
        height: N as u32,
    }
}

/// Install the Motif style on the whole context.
pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    let v = &mut style.visuals;
    v.dark_mode = false;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = TROUGH;
    v.faint_bg_color = BG_HOVER;
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_rounding = Rounding::ZERO;
    v.menu_rounding = Rounding::ZERO;

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = Rounding::ZERO;
        w.bg_fill = BG;
        w.weak_bg_fill = BG;
        w.fg_stroke = Stroke::new(1.0_f32, TEXT);
        w.bg_stroke = Stroke::new(1.0_f32, BG_DARK);
        w.expansion = 0.0;
    }
    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.active.bg_fill = TROUGH;
    v.widgets.active.weak_bg_fill = TROUGH;

    style.spacing.button_padding = Vec2::new(14.0, 6.0);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    type_scale(&mut style, 1.0);

    ctx.set_style(style);
}

/// The type scale, set deliberately rather than left to the egui
/// defaults: a heading that reads as one, a body size sized for a
/// counter at arm's length, and a small size that is still a size and
/// not a whisper. `scale` multiplies the whole ladder at once.
fn type_scale(style: &mut egui::Style, scale: f32) {
    use egui::{FontFamily, FontId, TextStyle};
    let px = |v: f32| (v * scale).round().max(8.0);
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(px(21.0), FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(px(14.0), FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(px(14.0), FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(px(11.5), FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(px(13.0), FontFamily::Monospace),
        ),
    ]
    .into();
}

/// How generously the interface spends the screen. "Confortable" is the
/// historical spacing; "compact" fits noticeably more on a small
/// screen without changing any layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Density {
    Comfortable,
    Compact,
}

/// Apply a text scale and a spacing density on top of [`apply`].
/// `scale` multiplies every font size (0.8 to 1.4 is sensible).
pub fn apply_scale(ctx: &egui::Context, scale: f32, density: Density) {
    let scale = scale.clamp(0.7, 1.8);
    let mut style = (*ctx.style()).clone();
    // Rebuild the ladder from the base sizes. Multiplying whatever is
    // already there compounds on every call, so two visits to the
    // options used to leave the text bigger each time.
    type_scale(&mut style, scale);
    let (pad, spacing, row) = match density {
        Density::Comfortable => (Vec2::new(14.0, 6.0), Vec2::new(10.0, 10.0), 22.0),
        Density::Compact => (Vec2::new(9.0, 3.0), Vec2::new(6.0, 5.0), 18.0),
    };
    style.spacing.button_padding = pad * scale;
    style.spacing.item_spacing = spacing * scale;
    style.spacing.interact_size.y = row * scale;
    ctx.set_style(style);
}

/// The small pictograms the toolbar can draw beside its labels. They
/// are painted, not typed: the bundled font carries almost no symbol
/// glyphs, and hand-drawn shapes match the rest of the theme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pict {
    /// A sheet with lines — documentation.
    Doc,
    /// Bars — the dashboard.
    Chart,
    /// A capsule — the drug base.
    Pill,
    /// A month grid — the agenda.
    Calendar,
    /// A pen over a line — the carnet.
    Pen,
    /// A padlock — locking the session.
    Lock,
    /// A cog — the options.
    Cog,
    /// A sheet with a folded corner — the templates.
    Template,
}

/// Paint `pict` inside `rect` (a square of roughly 11 px) in `color`.
pub fn pictogram(painter: &egui::Painter, rect: egui::Rect, pict: Pict, color: Color32) {
    let s = Stroke::new(1.0_f32, color);
    let r = rect;
    let (w, h) = (r.width(), r.height());
    match pict {
        Pict::Doc | Pict::Template => {
            let body = egui::Rect::from_min_max(
                egui::pos2(r.left() + w * 0.15, r.top()),
                egui::pos2(r.right() - w * 0.15, r.bottom()),
            );
            painter.rect_stroke(body, 0.0, s);
            for i in 1..4 {
                let y = body.top() + body.height() * i as f32 / 4.0;
                painter.line_segment(
                    [
                        egui::pos2(body.left() + 2.0, y),
                        egui::pos2(body.right() - 2.0, y),
                    ],
                    s,
                );
            }
            if pict == Pict::Template {
                painter.line_segment(
                    [
                        egui::pos2(body.right() - w * 0.3, body.top()),
                        egui::pos2(body.right(), body.top() + h * 0.3),
                    ],
                    s,
                );
            }
        }
        Pict::Chart => {
            for (i, frac) in [0.45_f32, 0.75, 1.0].into_iter().enumerate() {
                let x = r.left() + w * (0.1 + 0.3 * i as f32);
                let bar = egui::Rect::from_min_max(
                    egui::pos2(x, r.bottom() - h * frac),
                    egui::pos2(x + w * 0.2, r.bottom()),
                );
                painter.rect_filled(bar, 0.0, color);
            }
        }
        Pict::Pill => {
            let body = egui::Rect::from_min_max(
                egui::pos2(r.left(), r.center().y - h * 0.22),
                egui::pos2(r.right(), r.center().y + h * 0.22),
            );
            painter.rect_stroke(body, 0.0, s);
            painter.line_segment(
                [
                    egui::pos2(body.center().x, body.top()),
                    egui::pos2(body.center().x, body.bottom()),
                ],
                s,
            );
        }
        Pict::Calendar => {
            painter.rect_stroke(r, 0.0, s);
            painter.line_segment(
                [
                    egui::pos2(r.left(), r.top() + h * 0.3),
                    egui::pos2(r.right(), r.top() + h * 0.3),
                ],
                s,
            );
            for i in 1..3 {
                let x = r.left() + w * i as f32 / 3.0;
                painter.line_segment(
                    [egui::pos2(x, r.top() + h * 0.3), egui::pos2(x, r.bottom())],
                    s,
                );
            }
        }
        Pict::Pen => {
            painter.line_segment([r.left_bottom(), r.right_top()], s);
            painter.line_segment(
                [
                    egui::pos2(r.left(), r.bottom()),
                    egui::pos2(r.left() + w * 0.3, r.bottom()),
                ],
                s,
            );
        }
        Pict::Lock => {
            let body = egui::Rect::from_min_max(
                egui::pos2(r.left() + w * 0.1, r.center().y - h * 0.05),
                egui::pos2(r.right() - w * 0.1, r.bottom()),
            );
            painter.rect_stroke(body, 0.0, s);
            let shackle = egui::Rect::from_min_max(
                egui::pos2(r.left() + w * 0.3, r.top()),
                egui::pos2(r.right() - w * 0.3, body.top()),
            );
            painter.rect_stroke(shackle, 0.0, s);
        }
        Pict::Cog => {
            // A hub with four teeth: distinct from the calendar grid.
            painter.rect_stroke(r.shrink(w * 0.3), 0.0, s);
            let t = w * 0.18;
            for (a, b) in [
                (
                    egui::pos2(r.center().x, r.top()),
                    egui::pos2(r.center().x, r.top() + t),
                ),
                (
                    egui::pos2(r.center().x, r.bottom() - t),
                    egui::pos2(r.center().x, r.bottom()),
                ),
                (
                    egui::pos2(r.left(), r.center().y),
                    egui::pos2(r.left() + t, r.center().y),
                ),
                (
                    egui::pos2(r.right() - t, r.center().y),
                    egui::pos2(r.right(), r.center().y),
                ),
            ] {
                painter.line_segment([a, b], s);
            }
        }
    }
}

/// A Motif button carrying a painted pictogram before its label.
pub fn icon_button(ui: &mut egui::Ui, pict: Option<Pict>, label: &str) -> egui::Response {
    let Some(pict) = pict else {
        return button(ui, label);
    };
    let resp = button(ui, &format!("     {label}"));
    let size = (resp.rect.height() * 0.42).clamp(8.0, 14.0);
    let square = egui::Rect::from_min_size(
        egui::pos2(resp.rect.left() + 8.0, resp.rect.center().y - size / 2.0),
        egui::vec2(size, size),
    );
    pictogram(ui.painter(), square, pict, TEXT);
    resp
}

/// Draw a two-pixel Motif bevel around `rect`. `raised` selects between the
/// raised (light top-left) and sunken (dark top-left) variants.
pub fn bevel(painter: &egui::Painter, rect: egui::Rect, raised: bool) {
    let (tl, br) = if raised {
        (BG_LIGHT, BG_DARK)
    } else {
        (BG_DARK, BG_LIGHT)
    };
    for i in 0..2 {
        let r = rect.shrink(i as f32 + 0.5);
        painter.line_segment([r.left_bottom(), r.left_top()], Stroke::new(1.0_f32, tl));
        painter.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0_f32, tl));
        painter.line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0_f32, br));
        painter.line_segment(
            [r.right_bottom(), r.left_bottom()],
            Stroke::new(1.0_f32, br),
        );
    }
}

/// A Motif push button: raised bevel, sinks (and nudges its label) while
/// pressed.
pub fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    // Both come from the style: a hardcoded font size ignored the text
    // scale, so every Motif button stayed 14 px while the rest of the
    // interface grew, and a hardcoded padding ignored the density.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let padding = ui.spacing().button_padding + Vec2::new(4.0, 1.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, TEXT);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let pressed = response.is_pointer_button_down_on();
        let fill = if pressed {
            TROUGH
        } else if response.hovered() {
            BG_HOVER
        } else {
            BG
        };
        ui.painter().rect_filled(rect, 0.0, fill);
        bevel(ui.painter(), rect, !pressed);
        let nudge = if pressed {
            Vec2::splat(1.0)
        } else {
            Vec2::ZERO
        };
        let pos = rect.center() - galley.size() / 2.0 + nudge;
        ui.painter().galley(pos, galley, TEXT);
    }
    response
}

/// A full-width Motif list row: hover tint, `ACCENT` selection bar,
/// left-aligned text. For the sunken list boxes (patients, drugs…).
pub fn list_row(ui: &mut egui::Ui, text: egui::RichText, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let height = (ui.spacing().interact_size.y + 2.0).max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, ACCENT);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, BG_HOVER);
        }
        let color = if selected { Color32::WHITE } else { TEXT };
        let galley = ui.painter().layout_no_wrap(
            text.text().to_owned(),
            egui::FontId::proportional(14.0),
            color,
        );
        let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .with_clip_rect(rect.shrink2(Vec2::new(4.0, 0.0)))
            .galley(pos, galley, color);
    }
    response
}

/// [`list_row`] variant taking a prebuilt layout job, for rows with
/// per-character styling (fuzzy-match highlighting).
pub fn list_row_job(
    ui: &mut egui::Ui,
    job: egui::text::LayoutJob,
    selected: bool,
) -> egui::Response {
    let width = ui.available_width();
    let height = (ui.spacing().interact_size.y + 2.0).max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, ACCENT);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, BG_HOVER);
        }
        let galley = ui.fonts(|f| f.layout_job(job));
        let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .with_clip_rect(rect.shrink2(Vec2::new(4.0, 0.0)))
            .galley(pos, galley, TEXT);
    }
    response
}

/// A push button that stays in: raised when off, sunken when on. The
/// Motif idiom for a mode, a filter or a flag.
pub fn toggle(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let resp = button(ui, text);
    if on {
        ui.painter().rect_filled(resp.rect, 0.0, TROUGH);
        bevel(ui.painter(), resp.rect, false);
        let font = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(text.to_owned(), font, TEXT);
        let pos = resp.rect.center() - galley.size() / 2.0 + Vec2::splat(1.0);
        ui.painter().galley(pos, galley, TEXT);
    }
    resp
}

/// A section heading: small bold label with a sunken rule to the right,
/// the Motif take on group separators.
pub fn section(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(label).strong().size(13.0));
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width() - 8.0, 2.0),
            egui::Sense::hover(),
        );
        let y = rect.center().y;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            Stroke::new(1.0_f32, BG_DARK),
        );
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), y + 1.0),
                egui::pos2(rect.right(), y + 1.0),
            ],
            Stroke::new(1.0_f32, BG_LIGHT),
        );
    });
}

/// A sunken determinate progress trough with an `ACCENT` fill.
pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, TROUGH);
    bevel(ui.painter(), rect, false);
    let inner = rect.shrink(3.0);
    let mut fill = inner;
    fill.set_width(inner.width() * fraction.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, 0.0, ACCENT);
}

/// An indeterminate variant: a sliding `ACCENT` block, driven by `t` seconds.
pub fn progress_marquee(ui: &mut egui::Ui, width: f32, t: f64) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, TROUGH);
    bevel(ui.painter(), rect, false);
    let inner = rect.shrink(3.0);
    let block_w = inner.width() * 0.3;
    let span = inner.width() + block_w;
    let x = ((t * 0.7).fract() as f32) * span - block_w;
    let fill = egui::Rect::from_min_size(
        egui::pos2(inner.left() + x.max(0.0), inner.top()),
        Vec2::new(
            (block_w + x.min(0.0)).min(inner.right() - (inner.left() + x.max(0.0))),
            inner.height(),
        ),
    );
    if fill.width() > 0.0 {
        ui.painter().rect_filled(fill, 0.0, ACCENT);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_is_32x32_rgba() {
        let icon = super::icon();
        assert_eq!(icon.width, 32);
        assert_eq!(icon.height, 32);
        assert_eq!(icon.rgba.len(), 32 * 32 * 4);
    }
}
