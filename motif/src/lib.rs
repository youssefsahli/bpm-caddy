//! Old-school X/Motif look-and-feel for egui.
//!
//! Reproduces the classic `mwm` appearance: a blue-grey palette, square
//! corners, and two-pixel light/dark bevels that make widgets look raised
//! (buttons, panels) or sunken (text fields, troughs).

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

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
/// Errors and destructive warnings (Motif dark red).
pub const ALERT: Color32 = Color32::from_rgb(0x8b, 0x1a, 0x1a);

/// Lay content out in a fixed-width column centered in the available
/// space — the page grid every view aligns to. Content inside is
/// normal top-down, left-aligned layout.
pub fn column<R>(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let avail = ui.available_rect_before_wrap();
    // Always keep side margins, even when the panel is narrower than
    // the requested column.
    let w = width.min(avail.width() - 48.0).max(200.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(avail.center().x - w / 2.0, avail.top()),
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

    ctx.set_style(style);
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
    let padding = Vec2::new(18.0, 7.0);
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), egui::FontId::proportional(14.0), TEXT);
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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 24.0), egui::Sense::click());
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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 24.0), egui::Sense::click());
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
