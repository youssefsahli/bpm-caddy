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

/// One skin: every colour the Motif chrome is drawn from.
///
/// The *shape* of the interface never changes — square corners, two-pixel
/// bevels, raised widgets and sunken troughs are what makes it Motif, and
/// a theme has no say in any of it. What a theme carries is the palette,
/// the way an X resource file did: `*background`, `*topShadowColor`,
/// `*bottomShadowColor`, `*selectColor`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    /// Widget background — the grey everything sits on.
    pub bg: Color32,
    /// Top/left bevel highlight (`topShadowColor`).
    pub bg_light: Color32,
    /// Bottom/right bevel shadow (`bottomShadowColor`).
    pub bg_dark: Color32,
    /// Sunken areas: text fields, progress troughs.
    pub trough: Color32,
    /// Selection and active fill (`selectColor`). White text is drawn on
    /// it, so it stays dark enough to read against.
    pub accent: Color32,
    /// Hover tint, a shade lighter than `bg`.
    pub bg_hover: Color32,
    pub text: Color32,
    /// Secondary text on `bg`. Never the bevel shadow: labels that used
    /// it were barely legible on the widget grey, and unreadable once the
    /// screen was dimmed behind a dialog.
    pub text_dim: Color32,
    /// Third-level text: captions, units, timestamps. Still legible.
    pub text_faint: Color32,
    /// Errors and destructive warnings.
    pub alert: Color32,
    /// The sheet a printed monograph would be read on, for the
    /// document-style views inside the grey shell.
    pub paper: Color32,
    /// Ink on that sheet, and the lighter shade for secondary lines.
    pub ink: Color32,
    pub ink_light: Color32,
}

/// A named skin, as it is written in `config.toml` and shown in Options.
pub struct Theme {
    /// The key `[ui] theme` takes. Stable: it is written to disk.
    pub key: &'static str,
    /// What Options shows.
    pub label: &'static str,
    /// One line on where it comes from.
    pub note: &'static str,
    pub palette: Palette,
}

/// The skins that ship.
///
/// Five palettes off the same workstations the look itself comes from,
/// plus one that is not history but eyesight: a counter in full sun, or
/// an operator who wants the contrast turned up. The first is the
/// default and must stay first — a `config.toml` naming a theme this
/// version does not know falls back to it.
pub const THEMES: [Theme; 6] = [
    Theme {
        key: "motif",
        label: "Motif",
        note: "Le bleu-gris de mwm.",
        palette: Palette {
            bg: Color32::from_rgb(0xae, 0xb2, 0xc3),
            bg_light: Color32::from_rgb(0xde, 0xe1, 0xec),
            bg_dark: Color32::from_rgb(0x5c, 0x5f, 0x6e),
            trough: Color32::from_rgb(0x94, 0x98, 0xaa),
            accent: Color32::from_rgb(0x3a, 0x54, 0x7e),
            bg_hover: Color32::from_rgb(0xbb, 0xbf, 0xcf),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x39, 0x3c, 0x48),
            text_faint: Color32::from_rgb(0x4c, 0x50, 0x5e),
            alert: Color32::from_rgb(0x8b, 0x1a, 0x1a),
            paper: Color32::from_rgb(0xf6, 0xf4, 0xec),
            ink: Color32::from_rgb(0x1a, 0x1a, 0x20),
            ink_light: Color32::from_rgb(0x55, 0x55, 0x60),
        },
    },
    Theme {
        key: "cde",
        label: "CDE",
        note: "Le gris taupe du Common Desktop Environment.",
        palette: Palette {
            bg: Color32::from_rgb(0xae, 0xa9, 0x9e),
            bg_light: Color32::from_rgb(0xe2, 0xdd, 0xd2),
            bg_dark: Color32::from_rgb(0x5d, 0x59, 0x50),
            trough: Color32::from_rgb(0x96, 0x91, 0x86),
            accent: Color32::from_rgb(0x3f, 0x63, 0x6b),
            bg_hover: Color32::from_rgb(0xbe, 0xb9, 0xad),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x3a, 0x37, 0x30),
            text_faint: Color32::from_rgb(0x4e, 0x4a, 0x42),
            alert: Color32::from_rgb(0x8b, 0x24, 0x14),
            paper: Color32::from_rgb(0xf7, 0xf3, 0xe8),
            ink: Color32::from_rgb(0x1c, 0x1a, 0x16),
            ink_light: Color32::from_rgb(0x57, 0x53, 0x4a),
        },
    },
    Theme {
        key: "decwindows",
        label: "DECwindows",
        note: "Le gris froid et le bleu profond d'Ultrix.",
        palette: Palette {
            bg: Color32::from_rgb(0xa8, 0xac, 0xb0),
            bg_light: Color32::from_rgb(0xdc, 0xdf, 0xe2),
            bg_dark: Color32::from_rgb(0x55, 0x58, 0x5c),
            trough: Color32::from_rgb(0x8e, 0x92, 0x96),
            accent: Color32::from_rgb(0x23, 0x3f, 0x77),
            bg_hover: Color32::from_rgb(0xb7, 0xbb, 0xbf),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x33, 0x36, 0x3a),
            text_faint: Color32::from_rgb(0x47, 0x4a, 0x4f),
            alert: Color32::from_rgb(0x87, 0x18, 0x22),
            paper: Color32::from_rgb(0xf4, 0xf4, 0xf0),
            ink: Color32::from_rgb(0x18, 0x18, 0x1c),
            ink_light: Color32::from_rgb(0x52, 0x53, 0x58),
        },
    },
    Theme {
        key: "indigo",
        label: "Indigo Magic",
        note: "Le bleu clair des stations SGI.",
        palette: Palette {
            bg: Color32::from_rgb(0xa6, 0xb0, 0xc8),
            bg_light: Color32::from_rgb(0xdb, 0xe2, 0xf2),
            bg_dark: Color32::from_rgb(0x50, 0x58, 0x70),
            trough: Color32::from_rgb(0x8b, 0x95, 0xaf),
            accent: Color32::from_rgb(0x27, 0x40, 0x8c),
            bg_hover: Color32::from_rgb(0xb4, 0xbe, 0xd6),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x2e, 0x34, 0x48),
            text_faint: Color32::from_rgb(0x43, 0x4a, 0x60),
            alert: Color32::from_rgb(0x8e, 0x1c, 0x2c),
            paper: Color32::from_rgb(0xf4, 0xf6, 0xfb),
            ink: Color32::from_rgb(0x16, 0x18, 0x22),
            ink_light: Color32::from_rgb(0x4d, 0x52, 0x66),
        },
    },
    Theme {
        key: "olive",
        label: "HP VUE",
        note: "Le vert-de-gris des stations HP.",
        palette: Palette {
            bg: Color32::from_rgb(0xa9, 0xb0, 0x9c),
            bg_light: Color32::from_rgb(0xdd, 0xe3, 0xcf),
            bg_dark: Color32::from_rgb(0x55, 0x5b, 0x4b),
            trough: Color32::from_rgb(0x8e, 0x95, 0x82),
            accent: Color32::from_rgb(0x35, 0x5a, 0x3f),
            bg_hover: Color32::from_rgb(0xb7, 0xbe, 0xaa),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x32, 0x37, 0x2b),
            text_faint: Color32::from_rgb(0x46, 0x4c, 0x3d),
            alert: Color32::from_rgb(0x87, 0x22, 0x18),
            paper: Color32::from_rgb(0xf6, 0xf6, 0xea),
            ink: Color32::from_rgb(0x18, 0x1a, 0x14),
            ink_light: Color32::from_rgb(0x51, 0x56, 0x48),
        },
    },
    Theme {
        key: "contraste",
        label: "Contraste",
        note: "Pour un comptoir en plein soleil : fond clair, traits nets.",
        palette: Palette {
            bg: Color32::from_rgb(0xd8, 0xda, 0xe2),
            bg_light: Color32::WHITE,
            bg_dark: Color32::from_rgb(0x3a, 0x3d, 0x46),
            trough: Color32::from_rgb(0xf2, 0xf3, 0xf7),
            accent: Color32::from_rgb(0x1c, 0x35, 0x60),
            bg_hover: Color32::from_rgb(0xe6, 0xe8, 0xee),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x1e, 0x21, 0x2a),
            text_faint: Color32::from_rgb(0x33, 0x36, 0x40),
            alert: Color32::from_rgb(0x7a, 0x10, 0x10),
            paper: Color32::WHITE,
            ink: Color32::BLACK,
            ink_light: Color32::from_rgb(0x3c, 0x3c, 0x46),
        },
    },
];

/// Which of [`THEMES`] is in force, by index.
///
/// An atomic and not a lock: the palette is read thousands of times per
/// frame — every label, every bevel, every row — and a lock on that path
/// would cost more than the drawing. A relaxed load is a register read.
static CURRENT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Put a theme in force by its key. An unknown key — a `config.toml`
/// naming a theme this version has dropped, or a typo — falls back to
/// the first, which is why the first is the classic one.
pub fn set_theme(key: &str) {
    let i = THEMES
        .iter()
        .position(|t| t.key.eq_ignore_ascii_case(key.trim()))
        .unwrap_or(0);
    CURRENT.store(i, std::sync::atomic::Ordering::Relaxed);
}

/// The theme in force.
pub fn theme() -> &'static Theme {
    let i = CURRENT.load(std::sync::atomic::Ordering::Relaxed);
    // `min` rather than trust: the index only ever comes from
    // `set_theme`, but an out-of-bounds palette would panic on the paint
    // path, which is every frame of every view.
    &THEMES[i.min(THEMES.len() - 1)]
}

/// The palette in force.
#[inline]
pub fn palette() -> &'static Palette {
    &theme().palette
}

/// Widget background — the grey everything sits on.
#[inline]
pub fn bg() -> Color32 {
    palette().bg
}
/// Top/left bevel highlight.
#[inline]
pub fn bg_light() -> Color32 {
    palette().bg_light
}
/// Bottom/right bevel shadow.
#[inline]
pub fn bg_dark() -> Color32 {
    palette().bg_dark
}
/// Sunken areas: text fields, progress troughs.
#[inline]
pub fn trough() -> Color32 {
    palette().trough
}
/// Selection / active fill.
#[inline]
pub fn accent() -> Color32 {
    palette().accent
}
/// Hover tint, slightly lighter than [`bg`].
#[inline]
pub fn bg_hover() -> Color32 {
    palette().bg_hover
}
#[inline]
pub fn text() -> Color32 {
    palette().text
}
/// Secondary text on [`bg`].
#[inline]
pub fn text_dim() -> Color32 {
    palette().text_dim
}
/// Third-level text: captions, units, timestamps.
#[inline]
pub fn text_faint() -> Color32 {
    palette().text_faint
}
/// Errors and destructive warnings.
#[inline]
pub fn alert() -> Color32 {
    palette().alert
}
/// The sheet a printed monograph would be read on.
#[inline]
pub fn paper() -> Color32 {
    palette().paper
}
/// Ink on that sheet.
#[inline]
pub fn ink() -> Color32 {
    palette().ink
}
#[inline]
pub fn ink_light() -> Color32 {
    palette().ink_light
}

/// Draw a sheet of paper: flat fill, a thin ink border and a hard
/// shadow to the lower right, in keeping with the square Motif look.
pub fn sheet(painter: &egui::Painter, rect: egui::Rect) {
    let shadow = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0),
        egui::pos2(rect.max.x + 4.0, rect.max.y + 4.0),
    );
    painter.rect_filled(shadow, 0.0, crate::bg_dark());
    painter.rect_filled(rect, 0.0, crate::paper());
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, crate::ink_light()));
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
    let (bg, light, dark, accent) = (
        to4(crate::bg()),
        to4(crate::bg_light()),
        to4(crate::bg_dark()),
        to4(crate::accent()),
    );
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
    v.override_text_color = Some(crate::text());
    v.panel_fill = crate::bg();
    v.window_fill = crate::bg();
    v.extreme_bg_color = crate::trough();
    v.faint_bg_color = crate::bg_hover();
    v.selection.bg_fill = crate::accent();
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
        w.bg_fill = crate::bg();
        w.weak_bg_fill = crate::bg();
        w.fg_stroke = Stroke::new(1.0_f32, crate::text());
        w.bg_stroke = Stroke::new(1.0_f32, crate::bg_dark());
        w.expansion = 0.0;
    }
    v.widgets.hovered.bg_fill = crate::bg_hover();
    v.widgets.hovered.weak_bg_fill = crate::bg_hover();
    v.widgets.active.bg_fill = crate::trough();
    v.widgets.active.weak_bg_fill = crate::trough();

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
    pictogram(ui.painter(), square, pict, crate::text());
    resp
}

/// Draw a two-pixel Motif bevel around `rect`. `raised` selects between the
/// raised (light top-left) and sunken (dark top-left) variants.
pub fn bevel(painter: &egui::Painter, rect: egui::Rect, raised: bool) {
    let (tl, br) = if raised {
        (crate::bg_light(), crate::bg_dark())
    } else {
        (crate::bg_dark(), crate::bg_light())
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
    button_enabled(ui, text, true)
}

/// The same button, greyed out when `enabled` is false.
///
/// A disabled Motif button keeps its raised bevel and takes the same
/// room — the toolbar must not reflow because a pass is running — but
/// its label goes to [`text_dim`], it stops answering the pointer, and
/// its `clicked()` is always false. That last part is the one that
/// matters: a caller reading `.clicked()` without checking a flag of its
/// own would otherwise start a second copy of the work.
pub fn button_enabled(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    // Both come from the style: a hardcoded font size ignored the text
    // scale, so every Motif button stayed 14 px while the rest of the
    // interface grew, and a hardcoded padding ignored the density.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let padding = ui.spacing().button_padding + Vec2::new(4.0, 1.0);
    let ink = if enabled {
        crate::text()
    } else {
        crate::text_dim()
    };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, ink);
    let size = galley.size() + padding * 2.0;
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let pressed = enabled && response.is_pointer_button_down_on();
        let fill = if pressed {
            crate::trough()
        } else if enabled && response.hovered() {
            crate::bg_hover()
        } else {
            crate::bg()
        };
        ui.painter().rect_filled(rect, 0.0, fill);
        bevel(ui.painter(), rect, !pressed);
        let nudge = if pressed {
            Vec2::splat(1.0)
        } else {
            Vec2::ZERO
        };
        let pos = rect.center() - galley.size() / 2.0 + nudge;
        ui.painter().galley(pos, galley, ink);
    }
    response
}

/// A full-width Motif list row: hover tint, `crate::accent()` selection bar,
/// left-aligned text. For the sunken list boxes (patients, drugs…).
pub fn list_row(ui: &mut egui::Ui, text: egui::RichText, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let height = (ui.spacing().interact_size.y + 2.0).max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, crate::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
        }
        let color = if selected {
            Color32::WHITE
        } else {
            crate::text()
        };
        // One line, ending in an ellipsis rather than mid-letter: a row
        // clipped by the panel edge reads as a rendering fault, and
        // hides the fact that there was more to read.
        let mut job = egui::text::LayoutJob::single_section(
            text.text().to_owned(),
            egui::TextFormat {
                font_id: egui::FontId::proportional(14.0),
                color,
                ..Default::default()
            },
        );
        job.wrap = egui::text::TextWrapping {
            max_width: rect.width() - 12.0,
            max_rows: 1,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        let galley = ui.fonts(|f| f.layout_job(job));
        let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
    }
    response
}

/// [`list_row`] with a second, quieter half: a name and what it *is*,
/// on one line — « Aclasta  acide zolédronique », the second in italics
/// and dimmed.
///
/// The two are laid out as one galley so the ellipsis falls where the
/// panel edge is: the secondary half is what gets eaten as the dock
/// narrows, and the name it belongs to stays whole. `indent` is the room
/// left on the left for a tree's disclosure column.
pub fn list_row_pair(
    ui: &mut egui::Ui,
    primary: &str,
    secondary: &str,
    selected: bool,
    indent: f32,
) -> egui::Response {
    let width = ui.available_width();
    let height = (ui.spacing().interact_size.y + 2.0).max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, crate::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
        }
        let color = if selected {
            Color32::WHITE
        } else {
            crate::text()
        };
        // On the selection blue, a dimmed grey is unreadable: the quiet
        // half stays white and leans on the italics alone.
        let dim = if selected {
            Color32::WHITE
        } else {
            crate::text_faint()
        };
        let mut job = egui::text::LayoutJob::default();
        job.append(
            primary,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(14.0),
                color,
                ..Default::default()
            },
        );
        if !secondary.is_empty() {
            job.append(
                secondary,
                8.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: dim,
                    italics: true,
                    ..Default::default()
                },
            );
        }
        job.wrap = egui::text::TextWrapping {
            max_width: rect.width() - 12.0 - indent,
            max_rows: 1,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        let galley = ui.fonts(|f| f.layout_job(job));
        let pos = egui::pos2(
            rect.left() + 8.0 + indent,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
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
            ui.painter().rect_filled(rect, 0.0, crate::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
        }
        let galley = ui.fonts(|f| f.layout_job(job));
        let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .with_clip_rect(rect.shrink2(Vec2::new(4.0, 0.0)))
            .galley(pos, galley, crate::text());
    }
    response
}

/// A push button that stays in: raised when off, sunken when on. The
/// Motif idiom for a mode, a filter or a flag.
pub fn toggle(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let resp = button(ui, text);
    if on {
        ui.painter().rect_filled(resp.rect, 0.0, crate::trough());
        bevel(ui.painter(), resp.rect, false);
        let font = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font, crate::text());
        let pos = resp.rect.center() - galley.size() / 2.0 + Vec2::splat(1.0);
        ui.painter().galley(pos, galley, crate::text());
    }
    resp
}

/// A section heading: small bold label with a sunken rule to the right,
/// the Motif take on group separators.
pub fn section(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(label).strong().size(13.0));
        // A heading long enough to fill the row leaves nothing for the
        // rule — and egui panics on a negative allocation. The rule is
        // the decoration here, so it is what gives way.
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new((ui.available_width() - 8.0).max(0.0), 2.0),
            egui::Sense::hover(),
        );
        let y = rect.center().y;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            Stroke::new(1.0_f32, crate::bg_dark()),
        );
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), y + 1.0),
                egui::pos2(rect.right(), y + 1.0),
            ],
            Stroke::new(1.0_f32, crate::bg_light()),
        );
    });
}

/// A sunken determinate progress trough with an `crate::accent()` fill.
pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, crate::trough());
    bevel(ui.painter(), rect, false);
    let inner = rect.shrink(3.0);
    let mut fill = inner;
    fill.set_width(inner.width() * fraction.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, 0.0, crate::accent());
}

/// An indeterminate variant: a sliding `crate::accent()` block, driven by `t` seconds.
pub fn progress_marquee(ui: &mut egui::Ui, width: f32, t: f64) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, crate::trough());
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
        ui.painter().rect_filled(fill, 0.0, crate::accent());
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

    /// A theme is chosen by key, and a key this version does not know is
    /// not an error: it is the classic palette. Anything else would mean
    /// a `config.toml` carried from a newer release leaves the officine
    /// staring at a blank window.
    ///
    /// One test and not two, deliberately: the palette in force is
    /// process-wide — an atomic, because it is read thousands of times a
    /// frame — so two tests setting it would run in parallel and read
    /// each other's theme. Everything that *moves* it lives here.
    #[test]
    fn the_theme_in_force_is_chosen_by_key() {
        for t in super::THEMES.iter() {
            super::set_theme(t.key);
            assert_eq!(super::theme().key, t.key);
            // Case does not matter: it is typed by hand into a file.
            super::set_theme(&t.key.to_uppercase());
            assert_eq!(super::theme().key, t.key);
        }
        for wrong in ["", "   ", "nuit", "solarized"] {
            super::set_theme(wrong);
            assert_eq!(super::theme().key, super::THEMES[0].key, "{wrong:?}");
        }

        // The chart ramp follows the theme in its first colour and
        // nowhere else: the rest are data colours, and a series that
        // changed hue with the skin would make two screenshots
        // incomparable.
        super::set_theme("olive");
        assert_eq!(super::chart::series_color(0), super::accent());
        let others = super::chart::series();
        super::set_theme("indigo");
        assert_eq!(super::chart::series_color(0), super::accent());
        assert_eq!(super::chart::series()[1..], others[1..]);
        // And it wraps rather than panicking past the end.
        assert_eq!(
            super::chart::series_color(super::chart::SERIES_LEN + 2),
            super::chart::series_color(2)
        );
        super::set_theme(super::THEMES[0].key);
    }

    /// Every palette has to be *usable*, not merely different: white
    /// reads on the selection fill, the two bevel shades sit either side
    /// of the background, and the text is far enough from the grey it is
    /// written on. A theme that fails this is a screen nobody can work
    /// at, and it would fail silently.
    #[test]
    fn every_palette_can_be_read() {
        // Rough perceptual luminance, enough to tell « white reads on
        // this » from « it does not ».
        fn lum(c: super::Color32) -> f32 {
            (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
        }
        let mut keys: Vec<&str> = Vec::new();
        for t in super::THEMES.iter() {
            let p = &t.palette;
            let k = t.key;
            assert!(!keys.contains(&k), "clé en double : {k}");
            keys.push(k);
            assert!(!t.label.is_empty() && !t.note.is_empty(), "{k}");
            // The bevel: a highlight above the background, a shadow
            // below it. Reversed, every widget looks pressed.
            assert!(lum(p.bg_light) > lum(p.bg), "{k} : biseau clair");
            assert!(lum(p.bg_dark) < lum(p.bg), "{k} : biseau sombre");
            // The hover tint is a tint, not a second background.
            assert!(lum(p.bg_hover) > lum(p.bg), "{k} : survol");
            // White is drawn on the selection fill and on the alert
            // badges: both have to be dark enough to carry it.
            assert!(lum(p.accent) < 0.45, "{k} : sélection trop claire");
            assert!(lum(p.alert) < 0.45, "{k} : alerte trop claire");
            // And the three text shades have to stand off the grey they
            // are written on, faintest included.
            for (name, c) in [
                ("text", p.text),
                ("text_dim", p.text_dim),
                ("text_faint", p.text_faint),
            ] {
                assert!(
                    (lum(p.bg) - lum(c)).abs() > 0.25,
                    "{k} : {name} illisible sur le fond"
                );
            }
            // The printed-sheet colours are their own pair.
            assert!(lum(p.paper) > 0.7, "{k} : papier trop sombre");
            assert!(lum(p.ink) < 0.3, "{k} : encre trop claire");
            assert!(lum(p.ink_light) < lum(p.paper), "{k} : encre pâle");
        }
    }
}
