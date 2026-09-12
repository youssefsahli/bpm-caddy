//! Old-school X/Motif look-and-feel for egui.
//!
//! Reproduces the classic `mwm` appearance: a blue-grey palette, square
//! corners, and two-pixel light/dark bevels that make widgets look raised
//! (buttons, panels) or sunken (text fields, troughs).

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

pub mod chart;
pub mod layout;

pub use layout::{
    column_count, inside, page, panel, rule, split_columns, split_rows, tab_strip,
    tab_strip_height, visible_rect, vrule, well, Tab, TabAction,
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
    /// Ce qui demande un regard sans être une erreur : une surveillance
    /// en retard, une association à connaître, un forfait qui approche.
    ///
    /// Elle existe parce que ce ton-là était **écrit en dur** à neuf
    /// endroits — le même `0x7a5c1f` recopié — alors que la maison veut
    /// que toute couleur de chrome vienne du thème. Un ambre choisi pour
    /// le bleu-gris de mwm n'a rien à faire sur l'olive de HP VUE, et
    /// sur « contraste » il était simplement trop pâle.
    pub warn: Color32,
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
/// one that is not history but eyesight — a counter in full sun, or an
/// operator who wants the contrast turned up — and two dark ones, for
/// the garde de nuit: an officine at three in the morning is lit by
/// whatever is on the screen. The first is the default and must stay
/// first — a `config.toml` naming a theme this version does not know
/// falls back to it.
///
/// A dark skin is a palette and nothing else: no branch anywhere draws
/// differently for it. What it *does* change is that a colour picked
/// once for a light grey — a categorical hue, an amber warning — can no
/// longer be written down and drawn as it is, which is what
/// [`data_ramp`], [`data_tones`] and [`on_fill`] are for.
pub const THEMES: [Theme; 8] = [
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
            warn: Color32::from_rgb(0x7a, 0x56, 0x12),
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
            warn: Color32::from_rgb(0x7d, 0x5a, 0x10),
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
            warn: Color32::from_rgb(0x74, 0x54, 0x14),
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
            warn: Color32::from_rgb(0x7b, 0x57, 0x14),
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
            warn: Color32::from_rgb(0x6f, 0x55, 0x0f),
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
            warn: Color32::from_rgb(0x6a, 0x46, 0x00),
            paper: Color32::WHITE,
            ink: Color32::BLACK,
            ink_light: Color32::from_rgb(0x3c, 0x3c, 0x46),
        },
    },
    Theme {
        key: "nuit",
        label: "Nuit",
        note: "L'ardoise d'une garde : l'écran éclaire le comptoir, pas l'inverse.",
        palette: Palette {
            bg: Color32::from_rgb(0x2c, 0x2f, 0x3a),
            bg_light: Color32::from_rgb(0x45, 0x4a, 0x5a),
            bg_dark: Color32::from_rgb(0x17, 0x19, 0x22),
            trough: Color32::from_rgb(0x21, 0x23, 0x2c),
            accent: Color32::from_rgb(0x4a, 0x6e, 0xa8),
            bg_hover: Color32::from_rgb(0x38, 0x3c, 0x49),
            text: Color32::from_rgb(0xe8, 0xea, 0xf2),
            text_dim: Color32::from_rgb(0xbc, 0xc1, 0xd0),
            text_faint: Color32::from_rgb(0x96, 0x9c, 0xae),
            alert: Color32::from_rgb(0xe0, 0x6a, 0x6a),
            warn: Color32::from_rgb(0xd0, 0xa2, 0x4a),
            // The sheet is a sheet at night too — dark, and read with a
            // pale ink. A white page in the middle of this would be the
            // one thing on the screen nobody can look at.
            paper: Color32::from_rgb(0x1e, 0x20, 0x28),
            ink: Color32::from_rgb(0xe6, 0xe6, 0xee),
            ink_light: Color32::from_rgb(0xa8, 0xab, 0xba),
        },
    },
    Theme {
        key: "ambre",
        label: "Ambre",
        note: "Le phosphore ambré d'un terminal, en fond sombre.",
        palette: Palette {
            bg: Color32::from_rgb(0x2b, 0x27, 0x21),
            bg_light: Color32::from_rgb(0x4a, 0x44, 0x3a),
            bg_dark: Color32::from_rgb(0x15, 0x13, 0x10),
            trough: Color32::from_rgb(0x20, 0x1d, 0x18),
            accent: Color32::from_rgb(0x8a, 0x5a, 0x1c),
            bg_hover: Color32::from_rgb(0x39, 0x34, 0x29),
            text: Color32::from_rgb(0xf2, 0xdc, 0xb4),
            text_dim: Color32::from_rgb(0xd2, 0xb9, 0x8c),
            text_faint: Color32::from_rgb(0xab, 0x96, 0x70),
            alert: Color32::from_rgb(0xe0, 0x7a, 0x5a),
            warn: Color32::from_rgb(0xd9, 0xb0, 0x4a),
            paper: Color32::from_rgb(0x1d, 0x1a, 0x15),
            ink: Color32::from_rgb(0xf0, 0xdc, 0xb8),
            ink_light: Color32::from_rgb(0xb4, 0xa1, 0x80),
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

/// Ce qui demande un regard sans être une erreur.
#[inline]
pub fn warn() -> Color32 {
    palette().warn
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

/// Rough perceptual luminance, 0 (black) to 1 (white).
///
/// The same weighted sum the readability test uses, and public for the
/// same reason the palette is: whether a colour can be *read* on another
/// is a question the chrome asks at run time — a badge does not know
/// which of the skins is in force.
#[inline]
pub fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// Whether the shell in force is a dark one.
///
/// Not a field of the palette: it *is* the background's luminance, and a
/// second place saying so is a second place to get it wrong.
#[inline]
pub fn is_dark() -> bool {
    luminance(bg()) < 0.5
}

/// The ink that reads on a chosen fill.
///
/// Black or white and nothing from the palette, deliberately: this is
/// not chrome, it is the only two inks that read on an arbitrary badge
/// colour. Every filled chip in the application used to write
/// `Color32::WHITE` by hand — correct as long as every fill was dark,
/// which stopped being true the day a skin got a light one.
#[inline]
pub fn on_fill(fill: Color32) -> Color32 {
    if luminance(fill) > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

/// How far from the shell's own lightness a colour has to stand to be
/// **read as text** — letters are thin, and a hue that shows perfectly
/// well as a block of colour is a smudge at nine points.
pub const AS_TEXT: f32 = 0.30;
/// The same for a **filled shape** — a badge, a bar, a chip — which is
/// large and needs less.
pub const AS_FILL: f32 = 0.20;
/// The far side of the band a data colour may occupy: on a night skin,
/// the lightness past which a block of colour is a lamp. Somebody who
/// chooses a dark palette at three in the morning is choosing a screen
/// that has stopped shouting, and a ramp lifted until it clears the
/// background can arrive louder than the daylight one it came from —
/// the agenda's ten act kinds came out at three-quarters white before
/// this figure existed.
const NO_GLARE: f32 = 0.66;
/// Its opposite on a daylight palette: below this everything is black
/// and one hue is another.
const NO_MURK: f32 = 0.08;

/// The same colour at another lightness, **by scaling and not by
/// mixing**.
///
/// Mixing toward white is the obvious way to lift a colour onto a dark
/// shell and it is the wrong one: it walks every hue toward the same
/// point, so a ramp of eight distinct colours arrives as eight pale
/// neighbours — the chart test caught the grey-blue of series 1 and the
/// violet of series 5 landing within twenty-one of each other, where the
/// ramp's own rule is thirty-five apart. Multiplying the three channels
/// keeps their ratios, which is what the eye reads as the hue, and
/// *spreads* the ramp as it lifts it.
///
/// Luminance being a weighted sum of the channels, the factor is exact —
/// until a channel saturates, and the shortfall after that is the one
/// case where there is nowhere left to go but white.
fn to_luminance(c: Color32, target: f32) -> Color32 {
    let l = luminance(c);
    if l < 0.02 {
        // A near-black has no ratio to keep: scaling it by fifty would
        // still be a near-black, and its hue is not information anybody
        // reads. It becomes a grey of the right weight.
        let v = (target * 255.0).clamp(0.0, 255.0) as u8;
        return Color32::from_gray(v);
    }
    let k = target / l;
    let scale = |x: u8| (x as f32 * k).clamp(0.0, 255.0) as u8;
    let out = Color32::from_rgb(scale(c.r()), scale(c.g()), scale(c.b()));
    if luminance(out) + 0.01 >= target {
        return out;
    }
    // A channel saturated, so the factor could not carry the whole
    // distance: the rest is made up toward white, the only direction
    // left. **Bisected and not stepped** — a walk in twentieths stops
    // at the first step past the target, which overshot the band by
    // three hundredths on the CDE taupe and put a tone outside the
    // room it was being fitted into.
    let (mut under, mut over) = (0.0_f32, 1.0_f32);
    for _ in 0..12 {
        let t = (under + over) / 2.0;
        if luminance(out.lerp_to_gamma(Color32::WHITE, t)) < target {
            under = t;
        } else {
            over = t;
        }
    }
    out.lerp_to_gamma(Color32::WHITE, over)
}

/// `ink` if it can be read on `surface`, and the plain black or white
/// that can if it cannot.
///
/// For the one case where a colour chosen for one ground is painted on
/// another: a highlighted run of prose keeps the ink of the sentence it
/// belongs to, unless the highlight has swallowed it.
pub fn readable_on(ink: Color32, surface: Color32) -> Color32 {
    if (luminance(ink) - luminance(surface)).abs() >= AS_TEXT {
        ink
    } else {
        on_fill(surface)
    }
}

/// Emphasis where there is no bold face.
///
/// The application draws with egui's own faces and the proportional one
/// has no bold, so `*gras*` is carried by a stronger ink — which was
/// written as « darker », and darker is emphasis on a daylight palette
/// and a *whisper* on a night one. What was meant is « further from the
/// ground », which is the same thing on the six greys and the opposite
/// on the two dark ones.
pub fn emphasize(c: Color32) -> Color32 {
    c.lerp_to_gamma(on_fill(bg()), 0.45)
}

/// A whole categorical ramp **fitted into the band the shell leaves it**.
///
/// Moving each colour as far as that colour needs is the obvious way and
/// the wrong one for a set read side by side: the members that already
/// stand off the background stay put while their neighbours come up to
/// meet them, and a ramp whose closest pair was thirty-five apart
/// arrives with two at thirty-three. The agenda's ten act kinds and the
/// chart's eight series are read *against each other*, in one legend —
/// and a lone badge is a ramp of one, which is why there is no second
/// function for it.
///
/// So the ramp moves as one, by a single `a·c + b` on the three channels
/// — luminance being a weighted sum, that is an affine move in lightness
/// too, and the arithmetic is exact. Three cases, tried in order, and
/// each one gives up something the one before it kept:
///
/// 1. **Scale** (`b = 0`): keeps the ratios between the channels, so the
///    hues arrive saturated as they were written, and *spreads* the ramp
///    as it lifts it. Taken whenever it fits under [`NO_GLARE`].
/// 2. **Translate** (`a = 1`): keeps the distances between the members
///    exactly, at the cost of some saturation. It is what the ten act
///    kinds take on a night skin — scaling them would have put the
///    brightest at three-quarters white, a lamp on a screen chosen for
///    not being one.
/// 3. **Compress**: only when the band is narrower than the ramp's own
///    spread. This is the one that loses something real — the members
///    come closer together — so it is last.
///
/// On the daylight palettes no member needs moving and the ramp is
/// returned as written.
///
/// `apart` says what the ramp is *for*: [`AS_TEXT`] or [`AS_FILL`].
pub fn data_ramp<const N: usize>(ramp: [Color32; N], apart: f32) -> [Color32; N] {
    let on = luminance(bg());
    let dark = on < 0.5;
    let mut lo = 1.0_f32;
    let mut hi = 0.0_f32;
    for c in ramp {
        lo = lo.min(luminance(c));
        hi = hi.max(luminance(c));
    }
    // `near` is the closest to the ground any member may come, `far` the
    // end of the room it has on the other side.
    let (near, far) = if dark {
        ((on + apart).min(1.0), NO_GLARE)
    } else {
        ((on - apart).max(0.0), NO_MURK)
    };
    // The member nearest the ground is the one that decides, and if it
    // already clears the ground the ramp is left exactly as it is.
    let (worst, other) = if dark { (lo, hi) } else { (hi, lo) };
    if (worst - on).abs() >= apart {
        return ramp;
    }
    // Does the far member still sit inside the room, once moved?
    let fits = |x: f32| if dark { x <= far } else { x >= far };
    // A near-black cannot be scaled — multiplying nothing gives nothing,
    // and the factor runs away — so it goes straight to the translation,
    // which is the honest answer for it: black lifts to a grey.
    let scalable = worst > 0.02;
    let k = if scalable { near / worst } else { 1.0 };
    let (a, b) = if scalable && fits(other * k) {
        (k, 0.0)
    } else if fits(other + (near - worst)) {
        (1.0, near - worst)
    } else if (other - worst).abs() > f32::EPSILON {
        let a = (far - near) / (other - worst);
        (a, near - a * worst)
    } else {
        // One lightness for the whole ramp: nothing to spread, so the
        // whole of it moves to the near edge together.
        (1.0, near - worst)
    };
    let mut out = ramp;
    for c in out.iter_mut() {
        // Rounded and not truncated: a translation is supposed to keep
        // the distances between the members *exactly*, and a unit of
        // green lost to a cast is enough to bring the closest pair of
        // the chart ramp under the line it is held to.
        let s = |x: u8| (x as f32 * a + b * 255.0).round().clamp(0.0, 255.0) as u8;
        *c = Color32::from_rgb(s(c.r()), s(c.g()), s(c.b()));
    }
    out
}

/// The three tones of one hue, for a set with more members than the
/// ramp has colours.
///
/// The vaccine map has seventeen regions and the ramp has eight, so it
/// walks the ramp three times: the colour itself, then a lighter and a
/// darker tone of it. That was a bare `gamma_multiply(1.6)` and a
/// `gamma_multiply(0.55)`, which are unbounded — applied to a colour
/// already lifted onto a night shell the first clipped to **white**, and
/// a dozen countries came out as a hole in the map.
///
/// Bounded, they have to be bounded by **the band [`data_ramp`] fits a
/// ramp into**, and not by a pair of absolute limits: held only under
/// [`NO_GLARE`], the lighter tone was free to walk up to within six
/// hundredths of a *daylight* background, and « Afrique de l'Est »
/// became a tile you had to look for. A ceiling written for a dark
/// shell is not a ceiling on a light one.
///
/// And the three are chosen **together**. Computed one at a time they
/// collided: on the night skin the violet sits high enough that its
/// darker tone hit the floor, turned round, and landed four hundredths
/// from its own lighter tone — two regions, one swatch, which is the
/// one thing the group lens cannot afford. So the band is cut in three,
/// the slot the colour already occupies is dropped, and the two tones
/// take the two that are left: half a band apart by construction, and
/// never a matter of which order they were asked for.
pub fn data_tones(c: Color32) -> [Color32; 3] {
    let on = luminance(bg());
    let (near, far) = if on < 0.5 {
        (on + AS_FILL, NO_GLARE)
    } else {
        (on - AS_FILL, NO_MURK)
    };
    let (lo, hi) = if near < far { (near, far) } else { (far, near) };
    let mid = (lo + hi) / 2.0;
    let l = luminance(c);
    let slots = [lo, mid, hi];
    let mut nearest = 0;
    for (i, s) in slots.iter().enumerate() {
        if (s - l).abs() < (slots[nearest] - l).abs() {
            nearest = i;
        }
    }
    // The lighter of the two kept slots first, so tone 1 is always the
    // paler one — the map's second round, as it has always been.
    let (lighter, darker) = match nearest {
        0 => (hi, mid),
        1 => (hi, lo),
        _ => (mid, lo),
    };
    [c, to_luminance(c, lighter), to_luminance(c, darker)]
}

/// The band behind every other row of a striped table.
///
/// A shade of the trough the table sits in, mixed from the theme — the
/// literal grey that used to be written here was picked against the
/// Motif blue and read as a stain on the HP VUE green.
///
/// And **toward the bevel that is away from the trough**, not always
/// toward the shadow: a trough is already the darkest thing on a night
/// skin, so a band darker again is a band nobody sees. The same lesson
/// as everywhere else here — what a stripe owes is a distance from the
/// ground, and the direction is the shell's to choose.
pub fn stripe() -> Color32 {
    let toward = if is_dark() { bg_light() } else { bg_dark() };
    trough().lerp_to_gamma(toward, 0.18)
}

/// The theme in force is process-wide — an atomic, read thousands of
/// times a frame — so any test that *moves* it races every other one
/// that reads it. They take this in turn.
///
/// Poisoning is stepped over on purpose: a failed assertion in one test
/// must report *its* failure, not turn its neighbour into an unwrap
/// panic that says nothing about either.
#[cfg(test)]
pub(crate) fn theme_lock() -> std::sync::MutexGuard<'static, ()> {
    static THEME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    THEME.lock().unwrap_or_else(|e| e.into_inner())
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
    // egui reads this back for the handful of colours it still picks
    // itself — a code block's background, a hyperlink, the text cursor.
    // Left at `false` under a dark skin they come out as light-theme
    // defaults on a dark shell.
    v.dark_mode = crate::is_dark();
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
            FontId::new(px(BODY_PT), FontFamily::Proportional),
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
/// The type ladder's base body size, in points. `[ui] text_scale`
/// multiplies it, and [`pt`] reads the multiplier back off the style.
pub const BODY_PT: f32 = 14.0;

/// A size in points, scaled the way the style is.
///
/// **A `RichText::size(11.0)` does not follow `[ui] text_scale`.** The
/// ladder in `type_scale` does, and so does everything that resolves a
/// `TextStyle` — but a literal handed to `size()` or to
/// `FontId::proportional` is a number of pixels, full stop. Two hundred
/// and fourteen of them meant that on a screen set to 1,6 the captions,
/// the counts, the dates in the register and the labels beside every
/// figure stayed at eleven pixels while the buttons around them grew by
/// half: the very text that a person who enlarges the type most needs
/// enlarged. `motif::pt(ui, 11.0)` is that eleven, scaled.
pub fn pt(ui: &egui::Ui, points: f32) -> f32 {
    let body = egui::TextStyle::Body.resolve(ui.style()).size;
    (points * body / BODY_PT).max(6.0)
}

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
///
/// **Everything the `RichText` says is honoured**, and it did not use to
/// be: this kept the string and threw the rest away, so three call sites
/// that painted an overdue rendez-vous in `crate::alert()` had been
/// drawing it in the ordinary ink since the day they were written.
/// Nothing errored, nothing looked broken, and the red simply was not
/// there. A widget that accepts a type and ignores most of it will go on
/// fooling whoever calls it next — so the layout is egui's own now, and
/// colour, weight and italics arrive with it.
///
/// A selected row overrides the colour: white on the accent bar is a
/// legibility rule and not a preference, and a red on that blue reads as
/// neither.
/// How many rows a list label may take: **two, but only if it can break
/// cleanly.**
///
/// egui breaks a word that does not fit on a line of its own wherever it
/// must — `break_anywhere: false` cannot save a word wider than the
/// column. So « Benzodiazépines » on a narrow dock came out
/// « Benzodiazép / ines », which reads worse than the ellipsis it
/// replaced. Two lines are worth having when there is a space to break
/// at and every word fits: « Efferalgan / *paracétamol* » and « Paul /
/// Bernard » are what a one-line row was losing.
///
/// Le mot le plus long est mesuré au gabarit du « 0 », qui est plus
/// large que la moyenne des lettres : l'erreur penche donc vers une
/// seule ligne, c'est-à-dire vers l'ellipse plutôt que vers la coupe.
fn label_rows(ui: &egui::Ui, text: &str, font: &egui::FontId, max_width: f32) -> usize {
    let longest = text
        .split_whitespace()
        .map(|w| w.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let ch = ui.fonts(|f| f.glyph_width(font, '0'));
    if longest * ch <= max_width {
        2
    } else {
        1
    }
}

pub fn list_row(ui: &mut egui::Ui, text: egui::RichText, selected: bool) -> egui::Response {
    let width = ui.available_width();
    // The face comes from the style, so `[ui] text_scale` and the
    // density reach the lists too: a hardcoded 14 px left every list row
    // at 14 px while the rest of the interface grew, which is the bug
    // the buttons had and had fixed.
    let mut style = egui::Style::clone(ui.style());
    // The fallback when the caller sets no colour of its own; a colour
    // it *does* set wins over this, which is the whole point.
    style.visuals.override_text_color = Some(crate::text());
    let mut job = egui::text::LayoutJob::default();
    text.append_to(
        &mut job,
        &style,
        egui::FontSelection::Style(egui::TextStyle::Body),
        egui::Align::Center,
    );
    if selected {
        for section in &mut job.sections {
            section.format.color = Color32::WHITE;
        }
    }
    // **Deux lignes au plus**, et une ellipse plutôt qu'une coupe au
    // milieu d'une lettre : une rangée tranchée par le bord du panneau
    // se lit comme un défaut de rendu et cache qu'il y avait plus à
    // lire. Une seule ligne ne suffisait pas : sur un volet étroit,
    // « Bain de bouche à la chlorhexidine » et « Bain de bouche au
    // bicarbonate » se lisaient tous les deux « Bain de bouche … », et
    // une liste où quatre entrées se ressemblent ne se navigue plus.
    // Les rangées qui tiennent sur une ligne y restent — la hauteur est
    // mesurée sur la galée, pas réservée d'avance.
    let max_width = (width - 12.0).max(1.0);
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: label_rows(
            ui,
            &job.text,
            &egui::TextStyle::Body.resolve(ui.style()),
            max_width,
        ),
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    let galley = ui.fonts(|f| f.layout_job(job));
    // Measured after laying out, not guessed before: a caller that asks
    // for a bigger face gets a taller row instead of a clipped one.
    let height = (ui.spacing().interact_size.y + 2.0)
        .max(galley.size().y + 4.0)
        .max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, crate::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
        }
        let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, crate::text());
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
    // Both faces come from the style, so the pair grows with the text
    // scale like the plain row beside it.
    let font = egui::TextStyle::Body.resolve(ui.style());
    let quiet = egui::FontId::new(font.size * 0.86, font.family.clone());
    let font_for_rows = font.clone();
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
    // **Mise en page d'abord, hauteur ensuite.** La rangée était
    // allouée sur une ligne puis peinte dedans, donc « Paul Bernard »
    // sur un volet étroit se lisait « Paul … » — et deux patients de
    // même prénom devenaient la même rangée. Elle prend deux lignes
    // quand il en faut deux, comme `list_row_count`, et la hauteur
    // suit la galée au lieu de la précéder.
    let galley = {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            primary,
            0.0,
            egui::TextFormat {
                font_id: font,
                color,
                ..Default::default()
            },
        );
        if !secondary.is_empty() {
            // Un vrai espace, et non le seul `leading_space` : celui-ci
            // est un écart en pixels, pas une frontière de mot, et egui
            // coupait alors « Efferalgan paracétamol » en
            // « paracéta / mol » faute d'avoir où passer à la ligne.
            job.append(
                &format!(" {secondary}"),
                0.0,
                egui::TextFormat {
                    font_id: quiet,
                    color: dim,
                    italics: true,
                    ..Default::default()
                },
            );
        }
        let max_width = (width - 12.0 - indent).max(1.0);
        job.wrap = egui::text::TextWrapping {
            max_width,
            max_rows: label_rows(ui, &job.text, &font_for_rows, max_width),
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        ui.fonts(|f| f.layout_job(job))
    };
    let height = (ui.spacing().interact_size.y + 2.0)
        .max(galley.size().y + 4.0)
        .max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 0.0, crate::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
        }
        let pos = egui::pos2(
            rect.left() + 8.0 + indent,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }
    response
}

/// [`list_row`] with a figure held against the right edge.
///
/// The count is the half a narrow column must not lose. Appended after
/// the label — as [`list_row_pair`] does — it is the *end* of the line,
/// so it is the first thing elision eats: « Cardiologie et vaisseaux ·
/// … » is a row whose only quantitative content has gone, and a list of
/// them says nothing at all. Here the figure is measured first, reserved,
/// and the label is elided into what is left.
///
/// Which is the right thing to lose: a truncated label is still
/// recognisable and the selection says which row it is, whereas a
/// missing number is simply absent.
pub fn list_row_count(
    ui: &mut egui::Ui,
    label: &str,
    count: &str,
    selected: bool,
    dim: bool,
) -> egui::Response {
    let width = ui.available_width();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let row = ui.fonts(|f| f.row_height(&font));
    let ink = if selected {
        Color32::WHITE
    } else if dim {
        crate::text_faint()
    } else {
        crate::text()
    };
    // The figure keeps its own quieter ink, except on the selection blue
    // where a grey would be unreadable.
    let quiet = if selected {
        Color32::WHITE
    } else {
        crate::text_dim()
    };
    // **Laid out before the row is allocated**, because the row's height
    // depends on it: a therapeutic class can be « antagoniste non
    // stéroïdien des récepteurs minéralocorticoïdes », which wraps to two
    // lines in any column narrow enough to need this widget. Allocating
    // one line and painting two centres the text *outside* its own row
    // and into the neighbours — the label wraps, so the row grows.
    let num = ui
        .painter()
        .layout_no_wrap(count.to_owned(), font.clone(), quiet);
    let reserved = num.size().x + 16.0;
    let text = ui.painter().layout(
        label.to_owned(),
        font,
        ink,
        (width - 8.0 - reserved).max(8.0),
    );
    let height = (ui.spacing().interact_size.y + 2.0)
        .max(row + 4.0)
        .max(text.size().y + 6.0)
        .max(18.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    if selected {
        ui.painter().rect_filled(rect, 0.0, crate::accent());
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, crate::bg_hover());
    }
    ui.painter().galley(
        egui::pos2(rect.left() + 8.0, rect.center().y - text.size().y / 2.0),
        text,
        ink,
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - 8.0 - num.size().x,
            rect.center().y - num.size().y / 2.0,
        ),
        num,
        quiet,
    );
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
    section_ink(ui, label, crate::text());
}

/// [`section`] with the heading in an ink of its own.
///
/// A separator that *groups* carries what the group has in common, and
/// that is sometimes the thing to raise the eyes at: a day already past,
/// in the list of the rendez-vous still waiting. The rule stays the
/// quiet sunken one — it is the decoration, not the statement.
pub fn section_ink(ui: &mut egui::Ui, label: &str, ink: Color32) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let sz = pt(ui, 13.0);
        // **L'intitulé tient dans ce qu'on lui donne.** Posé sans
        // limite, il s'étalait et le volet le tranchait sans rien dire :
        // dans la liste des pays, « Amérique du Nord » se lisait
        // « Amérique du » et « Amérique centrale » « Amérique cen », au
        // ras du bord, sans même l'ellipse qui aurait dit qu'il manquait
        // quelque chose. Même famille que le titre de [`panel`], qui
        // peignait par-dessus la gouttière : un `Painter` peint où on
        // lui dit, et rien ne l'arrête.
        //
        // Le filet est la décoration et c'est lui qui cède déjà ; ici
        // c'est l'intitulé qui s'élide, une fois qu'il n'y a plus de
        // filet à sacrifier.
        //
        // Deux lignes quand chaque mot y tient, une sinon — la règle de
        // [`list_row`], et pour la même raison : élidés, « Amérique du
        // Nord » et « Amérique centrale » se lisent tous deux
        // « Amérique… », et l'intitulé ne distingue plus rien.
        //
        // **La galée est posée ici, pas confiée à `Label`.** Un
        // `Label::new(LayoutJob)` écrase le `max_width` du job par la
        // largeur d'enveloppement de l'`ui` et ne garde que le nombre de
        // lignes : la borne calculée ici ne servait donc à rien, et
        // l'intitulé repartait se faire trancher par le volet. C'est
        // pour la même raison que [`panel`] pose lui-même la galée de
        // son titre. Trois tentatives ont buté là-dessus en cherchant la
        // bonne largeur, alors que la largeur était bonne et que c'est
        // le chemin qui la jetait.
        let room = (ui.available_width() - 12.0).max(pt(ui, 24.0));
        let font = egui::FontId::proportional(sz);
        let rows = label_rows(ui, label, &font, room);
        let mut job = egui::text::LayoutJob::single_section(
            label.to_owned(),
            egui::TextFormat {
                font_id: font,
                color: ink,
                ..Default::default()
            },
        );
        job.wrap = egui::text::TextWrapping {
            max_width: room,
            max_rows: rows,
            overflow_character: Some('…'),
            break_anywhere: false,
        };
        let galley = ui.fonts(|f| f.layout_job(job));
        ui.add(egui::Label::new(galley));
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

/// The Motif *scale*: a sunken trough with a raised sliding thumb.
///
/// The one widget of the toolkit this project had not needed until a
/// quantity had to be set at the counter, and it is the right one for
/// that: dragging is faster than typing when the number is small and
/// the range is known, and the raised thumb in a sunken groove is what
/// a Motif scale has always looked like — no rounded pill, no colour.
///
/// The value stays the caller's: this only asks for a change. It is
/// **not** the only way to set the number — the text field beside it
/// stays authoritative, because a slider cannot express « 2,5 » and a
/// counter sometimes dispenses half a patch.
///
/// Clicking anywhere in the trough jumps there; dragging follows the
/// pointer. Values snap to `step`, so a scale over boxes lands on whole
/// boxes rather than on 6,97.
pub fn scale(
    ui: &mut egui::Ui,
    width: f32,
    value: f64,
    max: f64,
    step: f64,
) -> (egui::Response, f64) {
    let h = ui.spacing().interact_size.y.max(18.0);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(width.max(24.0), h), egui::Sense::click_and_drag());
    let max = if max > 0.0 { max } else { 1.0 };
    // The groove: a shallow sunken channel across the middle, the full
    // height being the thumb's travel.
    let groove = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 6.0));
    ui.painter().rect_filled(groove, 0.0, crate::trough());
    bevel(ui.painter(), groove, false);
    let mut out = value.clamp(0.0, max);
    if let Some(pos) = resp.interact_pointer_pos() {
        // The thumb has width, so the travel is shorter than the trough:
        // mapping the pointer to the full width would make the last
        // value unreachable and the first one sticky.
        let thumb_w = (h * 0.5).max(10.0);
        let travel = (rect.width() - thumb_w).max(1.0);
        let t = ((pos.x - rect.left() - thumb_w / 2.0) / travel).clamp(0.0, 1.0) as f64;
        let raw = t * max;
        out = if step > 0.0 {
            (raw / step).round() * step
        } else {
            raw
        };
        out = out.clamp(0.0, max);
    }
    let thumb_w = (h * 0.5).max(10.0);
    let travel = (rect.width() - thumb_w).max(1.0);
    let t = (out / max).clamp(0.0, 1.0) as f32;
    let thumb = egui::Rect::from_min_size(
        egui::pos2(rect.left() + travel * t, rect.top()),
        Vec2::new(thumb_w, rect.height()),
    );
    ui.painter().rect_filled(
        thumb,
        0.0,
        if resp.hovered() {
            crate::bg_hover()
        } else {
            crate::bg()
        },
    );
    bevel(ui.painter(), thumb, true);
    (resp, out)
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

    use super::theme_lock;

    /// A theme is chosen by key, and a key this version does not know is
    /// not an error: it is the classic palette. Anything else would mean
    /// a `config.toml` carried from a newer release leaves the officine
    /// staring at a blank window.
    #[test]
    fn the_theme_in_force_is_chosen_by_key() {
        let _guard = theme_lock();
        for t in super::THEMES.iter() {
            super::set_theme(t.key);
            assert_eq!(super::theme().key, t.key);
            // Case does not matter: it is typed by hand into a file.
            super::set_theme(&t.key.to_uppercase());
            assert_eq!(super::theme().key, t.key);
        }
        for wrong in ["", "   ", "amiga", "solarized"] {
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
    ///
    /// **Every rule here is a distance, never a direction**, and that is
    /// the whole reason a dark skin could be added at all: « l'encre est
    /// sombre » is true of six palettes and false of the two written for
    /// the night, whereas « l'encre est loin du papier » is what was
    /// meant every time. The two that stayed directional — the bevel and
    /// the hover tint — are directional in the *look*: a Motif widget is
    /// lit from the top left whatever the hour.
    #[test]
    fn every_palette_can_be_read() {
        let lum = super::luminance;
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
            // The selection fill carries white — in `list_row`, in the
            // scale's thumb, in every progress trough — so it stays dark
            // enough for it, on a night skin as much as on a day one.
            // And it has to be *seen* against the shell: a dark blue on
            // a dark slate is a selection nobody notices moving.
            assert!(lum(p.accent) < 0.45, "{k} : sélection trop claire");
            assert!(
                (lum(p.accent) - lum(p.bg)).abs() > 0.15,
                "{k} : sélection noyée dans le fond"
            );
            // Alerte and mise en garde are read as *text* far more often
            // than as a badge — an overdue rendez-vous, a surveillance
            // late, a forfait that is running out — so what they owe is
            // distance from the background. The two badges that fill
            // with them take their ink from `on_fill`, which is why
            // there is no ceiling here any more: on a dark shell a red
            // dark enough to carry white is a red nobody can read.
            for (name, c) in [("alerte", p.alert), ("mise en garde", p.warn)] {
                assert!(
                    (lum(c) - lum(p.bg)).abs() > 0.25,
                    "{k} : {name} illisible sur le fond"
                );
            }
            // Elle doit aussi **se distinguer de l'alerte** : les deux
            // se lisent côte à côte sur la même ordonnance, et deux
            // rouges voisins ne disent plus lequel presse.
            assert!(
                (lum(p.warn) - lum(p.alert)).abs() > 0.04
                    || (i32::from(p.warn.g()) - i32::from(p.alert.g())).abs() > 24,
                "{k} : mise en garde et alerte trop proches"
            );
            // And the three text shades have to stand off the grey they
            // are written on, faintest included — and off the trough,
            // which is the other surface text is drawn on: it is what
            // one types into.
            for (name, c) in [
                ("text", p.text),
                ("text_dim", p.text_dim),
                ("text_faint", p.text_faint),
            ] {
                assert!(
                    (lum(p.bg) - lum(c)).abs() > 0.25,
                    "{k} : {name} illisible sur le fond"
                );
                assert!(
                    (lum(p.trough) - lum(c)).abs() > 0.25,
                    "{k} : {name} illisible dans un champ"
                );
            }
            // The printed-sheet colours are their own pair, and a pair
            // is a distance: a night skin reads its monographs off a
            // dark sheet in a pale ink, which is the same relation the
            // other way up.
            assert!(
                (lum(p.paper) - lum(p.ink)).abs() > 0.55,
                "{k} : encre trop proche du papier"
            );
            assert!(
                (lum(p.paper) - lum(p.ink_light)).abs() > 0.30,
                "{k} : encre pâle trop proche du papier"
            );
            // The secondary ink lies *between* the two, whichever way
            // round they are: paler than the ink, darker than the sheet.
            let (lo, hi) = if lum(p.ink) < lum(p.paper) {
                (lum(p.ink), lum(p.paper))
            } else {
                (lum(p.paper), lum(p.ink))
            };
            assert!(
                lo < lum(p.ink_light) && lum(p.ink_light) < hi,
                "{k} : encre pâle hors bornes"
            );
        }
    }

    /// The three ways a ramp is fitted, each held on the case that
    /// forced it into existence.
    ///
    /// Written as a test because the choice between them is arithmetic
    /// and invisible: nothing on screen says « this ramp was translated
    /// rather than scaled », and the difference is a legend whose
    /// colours have quietly moved together.
    #[test]
    fn a_ramp_is_fitted_by_the_gentlest_move_that_works() {
        let _guard = theme_lock();
        let lum = super::luminance;
        let d = |a: super::Color32, b: super::Color32| {
            let f = |x: u8, y: u8| (x as f32 - y as f32).powi(2);
            (f(a.r(), b.r()) + f(a.g(), b.g()) + f(a.b(), b.b())).sqrt()
        };

        // A daylight palette moves nothing: every one of these already
        // stands off the grey, and a skin that redraws what it did not
        // have to is a skin that changes every screenshot for nothing.
        super::set_theme("motif");
        let day = [
            super::Color32::from_rgb(0x3a, 0x54, 0x7e),
            super::Color32::from_rgb(0x2e, 0x6e, 0x4e),
        ];
        assert_eq!(super::data_ramp(day, super::AS_FILL), day);

        super::set_theme("nuit");
        // Two dark neighbours: lifting the lower one by scaling takes
        // the other nowhere near the ceiling, so the ratios — the hue —
        // survive, and the pair comes out *further* apart than it went
        // in. That is the whole reason scaling is tried first.
        let tight = [
            super::Color32::from_rgb(0x20, 0x30, 0x40),
            super::Color32::from_rgb(0x28, 0x38, 0x48),
        ];
        let fitted = super::data_ramp(tight, super::AS_FILL);
        assert!(
            d(fitted[0], fitted[1]) > d(tight[0], tight[1]),
            "une mise à l'échelle écarte, elle ne rapproche pas"
        );
        for (a, b) in tight.iter().zip(fitted.iter()) {
            // Same hue: the channels kept their ratios, so the largest
            // and the smallest are still the same two.
            assert_eq!(
                (a.r() < a.g(), a.g() < a.b()),
                (b.r() < b.g(), b.g() < b.b()),
                "la teinte a bougé"
            );
        }

        // The act kinds: their brightest member is close enough to the
        // ceiling that scaling would put it through it, so the ramp is
        // translated instead — and a translation keeps every distance,
        // to the rounding of a byte.
        let kinds = [
            super::Color32::from_rgb(0x6e, 0x2e, 0x2e),
            super::Color32::from_rgb(0x5e, 0x7e, 0x3a),
            super::Color32::from_rgb(0x3a, 0x54, 0x7e),
        ];
        let fitted = super::data_ramp(kinds, super::AS_FILL);
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                assert!(
                    (d(fitted[i], fitted[j]) - d(kinds[i], kinds[j])).abs() <= 2.0,
                    "une translation garde les écarts : {} contre {}",
                    d(fitted[i], fitted[j]),
                    d(kinds[i], kinds[j])
                );
            }
        }
        // And whichever move was made, nothing leaves the band.
        for c in fitted {
            assert!(lum(c) >= lum(super::bg()) + super::AS_FILL - 0.01);
            assert!(lum(c) <= super::NO_GLARE + 0.01);
        }

        // The two extra tones of one hue, on every skin: three tones
        // that must be three, and all three inside the band. The map
        // walks the ramp three times, and a round that lands on another
        // round is two regions with one swatch.
        for t in super::THEMES.iter() {
            super::set_theme(t.key);
            let on = lum(super::bg());
            let (near, far) = if on < 0.5 {
                (on + super::AS_FILL, super::NO_GLARE)
            } else {
                (on - super::AS_FILL, super::NO_MURK)
            };
            let (lo, hi) = if near < far { (near, far) } else { (far, near) };
            for base in super::chart::series() {
                let tones = super::data_tones(base);
                for (i, a) in tones.iter().enumerate() {
                    assert!(
                        lum(*a) >= lo - 0.01 && lum(*a) <= hi + 0.01,
                        "{} : le ton {i} de {base:?} sort de la bande ({:.2})",
                        t.key,
                        lum(*a)
                    );
                    for (j, b) in tones.iter().enumerate().skip(i + 1) {
                        assert!(
                            (lum(*a) - lum(*b)).abs() > 0.06,
                            "{} : les tons {i} et {j} de {base:?} sont le même ton",
                            t.key
                        );
                    }
                }
            }
        }
        super::set_theme(super::THEMES[0].key);
    }

    /// The categorical colours are chosen for their hue and drawn on
    /// whichever skin is in force. On the light greys they are fine as
    /// written; on a night one a mid-dark blue is a blue on a blue, and
    /// the journal loses the column that says who wrote what.
    ///
    /// So the two adapters owe an invariant apiece, on **every** palette:
    /// what is drawn as text stands off the background, and what is
    /// drawn as a fill stands off it too *and* still carries its ink.
    #[test]
    fn a_data_colour_is_legible_under_every_skin() {
        let _guard = theme_lock();
        let lum = super::luminance;
        // The ramp as it is actually written down: the chart's seven
        // fixed series, plus the hues app.rs assigns to an operator, an
        // act kind and a drug's administrative status.
        const RAMP: [super::Color32; 17] = [
            super::Color32::from_rgb(0x6b, 0x70, 0x82),
            super::Color32::from_rgb(0x2f, 0x6b, 0x5c),
            super::Color32::from_rgb(0x8b, 0x1a, 0x1a),
            super::Color32::from_rgb(0x7a, 0x5c, 0x1f),
            super::Color32::from_rgb(0x54, 0x3d, 0x73),
            super::Color32::from_rgb(0x1f, 0x5c, 0x7a),
            super::Color32::from_rgb(0x6e, 0x3d, 0x2a),
            super::Color32::from_rgb(0x3a, 0x54, 0x7e),
            super::Color32::from_rgb(0x2e, 0x6e, 0x4e),
            super::Color32::from_rgb(0x7e, 0x3a, 0x5e),
            super::Color32::from_rgb(0x8b, 0x5a, 0x1a),
            super::Color32::from_rgb(0x1a, 0x6e, 0x8b),
            super::Color32::from_rgb(0x5e, 0x3a, 0x7e),
            super::Color32::from_rgb(0x5e, 0x7e, 0x3a),
            super::Color32::from_rgb(0x6e, 0x2e, 0x2e),
            super::Color32::from_rgb(0x7e, 0x4a, 0x2e),
            super::Color32::from_rgb(0x2e, 0x6e, 0x6e),
        ];
        for t in super::THEMES.iter() {
            super::set_theme(t.key);
            let k = t.key;
            let on = lum(super::bg());
            let inks = super::data_ramp(RAMP, super::AS_TEXT);
            let fills = super::data_ramp(RAMP, super::AS_FILL);
            for (i, c) in RAMP.iter().copied().enumerate() {
                let ink = inks[i];
                assert!(
                    (lum(ink) - on).abs() > 0.25,
                    "{k} : {c:?} illisible en texte sur le fond"
                );
                let fill = fills[i];
                assert!(
                    (lum(fill) - on).abs() > 0.14,
                    "{k} : {c:?} en pastille se confond avec le fond"
                );
                // Ni au-delà de la bande, dans un sens ou dans l'autre :
                // une pastille qui éclaire est ce qu'une peau de nuit
                // est choisie pour ne plus avoir.
                assert!(
                    lum(fill) <= super::NO_GLARE + 0.01 && lum(fill) >= super::NO_MURK - 0.01,
                    "{k} : {c:?} sort de la bande ({:.2})",
                    lum(fill)
                );
                // `on_fill` picks the ink; what is asserted is that the
                // ink it picks is far enough from the fill to be read,
                // never which of the two it picked.
                assert!(
                    (lum(fill) - lum(super::on_fill(fill))).abs() > 0.45,
                    "{k} : pastille {c:?} ne porte plus son texte"
                );
            }
            // The stripe behind every other row of a table is a shade of
            // the trough, so the text typed into that table still reads
            // on it.
            assert!(
                (lum(super::stripe()) - lum(super::text())).abs() > 0.25,
                "{k} : bande zébrée illisible"
            );
            assert!(
                super::stripe() != super::trough(),
                "{k} : bande zébrée invisible"
            );
        }
        super::set_theme(super::THEMES[0].key);
    }
}
