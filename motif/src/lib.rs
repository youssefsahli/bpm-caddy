//! Old-school X/Motif look-and-feel for egui.
//!
//! Reproduces the classic `mwm` appearance: a blue-grey palette, square
//! corners, and two-pixel light/dark bevels that make widgets look raised
//! (buttons, panels) or sunken (text fields, troughs).

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

pub mod chart;
pub mod layout;

pub use layout::{
    column_count, inside, page, panel, panel_chrome, panel_forms, rule, split_columns, split_rows,
    tab_strip, tab_strip_height, visible_rect, vrule, well, Tab, TabAction,
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
/// three that are not history but eyesight — a counter in full sun, an
/// operator who wants the contrast turned up, a screen whose colours
/// have gone — and two dark ones, for the garde de nuit: an officine at
/// three in the morning is lit by whatever is on the screen. The first
/// is the default and must stay first — a `config.toml` naming a theme
/// this version does not know falls back to it.
///
/// **Les trois claires ne se ressemblent pas par accident.**
/// « Contraste » garde le gris bleuté de la maison et monte l'écart ;
/// « Papier » abandonne la teinte — un gris neutre et des cases
/// blanches, parce qu'un écran dont les couleurs ont dérivé rend un
/// bleu-gris indistinct d'un gris ; « Sépia » garde l'écart et enlève
/// le bleu, pour qui le blanc d'écran éblouit. Trois réponses à trois
/// gênes différentes, et non trois nuances du même réglage.
///
/// A dark skin is a palette and nothing else: no branch anywhere draws
/// differently for it. What it *does* change is that a colour picked
/// once for a light grey — a categorical hue, an amber warning — can no
/// longer be written down and drawn as it is, which is what
/// [`data_ramp`], [`data_tones`] and [`on_fill`] are for.
pub const THEMES: [Theme; 10] = [
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
        key: "papier",
        label: "Papier",
        note: "Le gris d'un formulaire et le blanc d'une case : l'écart le plus grand.",
        palette: Palette {
            bg: Color32::from_rgb(0xdc, 0xdc, 0xde),
            bg_light: Color32::WHITE,
            bg_dark: Color32::from_rgb(0x2a, 0x2a, 0x31),
            // Les cases sont **blanches**, et pas d'un gris un peu
            // creusé : sur un écran fatigué, ce que l'on remplit se
            // distingue de ce qui ne se remplit pas par la couleur bien
            // avant de se distinguer par un biseau de deux pixels.
            trough: Color32::WHITE,
            accent: Color32::from_rgb(0x12, 0x30, 0x6e),
            bg_hover: Color32::from_rgb(0xeb, 0xeb, 0xed),
            text: Color32::BLACK,
            text_dim: Color32::from_rgb(0x16, 0x16, 0x1c),
            text_faint: Color32::from_rgb(0x2b, 0x2b, 0x34),
            alert: Color32::from_rgb(0x8e, 0x00, 0x00),
            warn: Color32::from_rgb(0x5e, 0x3c, 0x00),
            paper: Color32::WHITE,
            ink: Color32::BLACK,
            ink_light: Color32::from_rgb(0x3a, 0x3a, 0x42),
        },
    },
    Theme {
        key: "sepia",
        label: "Sépia",
        note: "Le même écart, en chaud : pour qui le blanc d'écran éblouit.",
        palette: Palette {
            bg: Color32::from_rgb(0xe0, 0xd7, 0xbf),
            bg_light: Color32::from_rgb(0xfd, 0xf8, 0xea),
            bg_dark: Color32::from_rgb(0x44, 0x3c, 0x2a),
            trough: Color32::from_rgb(0xfd, 0xf8, 0xec),
            accent: Color32::from_rgb(0x2c, 0x3a, 0x6e),
            bg_hover: Color32::from_rgb(0xea, 0xe2, 0xcd),
            text: Color32::from_rgb(0x12, 0x0e, 0x06),
            text_dim: Color32::from_rgb(0x24, 0x1d, 0x10),
            text_faint: Color32::from_rgb(0x38, 0x30, 0x1c),
            alert: Color32::from_rgb(0x8a, 0x14, 0x08),
            warn: Color32::from_rgb(0x5c, 0x3c, 0x00),
            paper: Color32::from_rgb(0xfd, 0xf8, 0xec),
            ink: Color32::from_rgb(0x12, 0x0e, 0x06),
            ink_light: Color32::from_rgb(0x45, 0x3d, 0x2a),
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
/// L'invite d'un champ : **ce qu'on peut y écrire, et non ce qui y est
/// écrit**.
///
/// egui peint l'invite dans sa couleur affaiblie, et
/// [`apply`] pose un `override_text_color` sur tout le contexte — or
/// celui-ci l'emporte sur la couleur que le widget propose. Toutes les
/// invites de l'application sortaient donc dans **l'encre pleine**, à
/// l'octet près celle d'une valeur tapée : mesuré sur une capture de la
/// trame, « 14h » d'un après-midi vide et « 14:00 » d'un après-midi
/// posé étaient tous deux en (1, 1, 1) sur le même fond. La colonne des
/// totaux disait 3 h 30 et les champs semblaient dire huit heures et
/// demie ; c'est le total qui avait raison. C'est le piège que
/// `docs/ARCHITECTURE.md` nomme déjà pour la grille du planning — deux choses
/// différentes sous une même apparence —, un cran plus haut.
///
/// Une couleur explicite, elle, passe devant l'override (egui la lit en
/// premier). [`text_faint`] est le bon cran — les légendes, les unités,
/// les horodatages — et c'est le seul dont `every_palette_can_be_read`
/// garantisse déjà la lisibilité **dans un creux**, c'est-à-dire sur la
/// surface où l'on tape, sur les dix palettes.
///
/// Pas `RichText::weak()`, qui l'emporterait aussi : il teinte vers
/// `window_fill`, c'est-à-dire vers le fond du panneau et non vers celui
/// du champ — sur les palettes claires le creux est plus sombre que le
/// panneau, donc affaiblir vers le panneau *rapproche* l'invite de son
/// propre fond. Et aucun test ne tient cette couleur-là.
pub fn hint(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).color(crate::text_faint())
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
    data_ramp_on(bg(), ramp, apart)
}

/// La même, sur un fond **qu'on nomme** plutôt que sur le panneau.
///
/// Presque toutes les teintes de données se posent sur `bg()`, et c'est
/// ce que [`data_ramp`] suppose. Une exception existe et elle est
/// entière : ce qu'on écrit *dans* une case se pose sur le creux, qui
/// est plus sombre que le panneau sur les peaux claires. Une teinte
/// calculée à trente centièmes du panneau n'en est plus qu'à vingt du
/// creux — et la différence se voit exactement là où l'on relit son
/// propre texte. C'est la règle que ce dépôt écrit déjà pour les
/// invites : le pas juste dans un creux n'est pas le pas juste sur un
/// panneau.
pub fn data_ramp_on<const N: usize>(
    ground: Color32,
    ramp: [Color32; N],
    apart: f32,
) -> [Color32; N] {
    let on = luminance(ground);
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

/// Les cinq teintes d'un éditeur de code — dans cet ordre : le
/// commentaire, la chaîne, le nombre, le mot-clé, l'appel connu.
///
/// Une rampe nommée et une seule, comme toutes les teintes catégorielles
/// de ce chrome : ajouter une catégorie est ajouter une ligne ici, et
/// jamais une couleur écrite au point où l'on peint. Elles sont sombres
/// telles qu'elles sont écrites, parce que les six peaux claires sont
/// six sur huit — [`code_ink`] les relève sur les deux autres.
pub const CODE_RAMP: [Color32; 5] = [
    Color32::from_rgb(0x3f, 0x62, 0x38), // commentaire, vert
    Color32::from_rgb(0x8a, 0x36, 0x12), // chaîne, terre
    Color32::from_rgb(0x1c, 0x5a, 0x66), // nombre, bleu-vert
    Color32::from_rgb(0x2e, 0x2c, 0x84), // mot-clé, indigo
    Color32::from_rgb(0x75, 0x1f, 0x5e), // appel de l'API, prune
];

/// [`CODE_RAMP`], ajustée pour être lue **dans un creux**.
///
/// Et non sur le panneau : un éditeur de code est une case de saisie,
/// son fond est celui d'une case de saisie, et sur les peaux claires le
/// creux est plus sombre que le panneau. Passer par [`data_ramp`] aurait
/// donné cinq teintes calculées pour un fond qui n'est pas celui sur
/// lequel elles se posent — exactement la faute que ce fichier nomme
/// pour les invites.
pub fn code_ink() -> [Color32; 5] {
    data_ramp_on(trough(), CODE_RAMP, AS_TEXT)
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
    // **Et la colonne ne dépasse jamais ce qui est visible.** Dans une
    // région qui défile latéralement — la vue des tables en est une —
    // `avail` est la largeur du *contenu* et non celle du hublot, et
    // `allocate_new_ui` rend un enfant dont le rectangle s'étend
    // jusque-là : la phrase des calculs enveloppait donc à cinq cent
    // quatre-vingts pixels dans un volet qui en montre trois cent
    // quatre-vingt-dix-sept, et « … · accumulat » se lisait coupé net
    // au bord du panneau. Le calcul, lui, n'a rien à faire défiler de
    // côté ; c'est le tableau en dessous qui défile, et il est dessiné
    // hors de cette colonne.
    let rect = egui::Rect::from_min_max(
        egui::pos2(rect.left().max(visible.left()), rect.top()),
        egui::pos2(rect.right().min(visible.right()), rect.bottom()),
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
    // **Ce qui est pressé s'éclaire, il ne s'enfonce pas dans sa
    // propre gorge.** `bg_fill` à l'état actif ne sert ici qu'à deux
    // choses : le curseur d'une barre de défilement qu'on traîne, et
    // la case d'une case à cocher sous le doigt. Mis au creux, le
    // premier *disparaissait dans son rail* — qui est peint de cette
    // même couleur — c'est-à-dire exactement au moment où l'on a
    // besoin de le voir. Le relief enfoncé des boutons de cette maison
    // est dessiné par `button` et `toggle`, et `weak_bg_fill` — ce que
    // les boutons d'egui lisent — garde le creux.
    v.widgets.active.bg_fill = crate::bg_hover();
    v.widgets.active.weak_bg_fill = crate::trough();

    // **La barre de défilement est un objet de ce chrome aussi.**
    // egui la peint par défaut dans la *couleur du texte* — c'est ce
    // que dit `foreground_color` —, ce qui donnait une barre **noire**
    // posée sur un panneau gris : le seul aplat d'encre pure de
    // l'écran, sur l'objet qui n'a rien à dire. Elle prend le rail
    // creusé et le curseur couleur de panneau, comme tout le reste
    // d'ici.
    //
    // Le reste du réglage ne change pas : la barre **flotte** toujours
    // par défaut — une trentaine de régions l'éteignent chacune pour
    // leur raison, et leur largeur est comptée par
    // `App::scrolled_width`, qui lit ces deux nombres-là.
    style.spacing.scroll = egui::style::ScrollStyle {
        foreground_color: false,
        ..egui::style::ScrollStyle::floating()
    };

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
    // **Les bornes sont celles que la glissière offre.** Elles étaient
    // plus larges — 0,7 à 1,8 — que ce que l'application propose et que
    // ce que ses mises en page ont jamais vu : au-delà de 1,6, sur un
    // écran de 1024, la barre du haut ne tient plus et ses deux groupes
    // se peignent l'un sur l'autre, la barre d'état aussi. Un réglage
    // écrit à la main dans `config.toml` entrait dans cette zone-là sans
    // que rien ne le dise ; il y entre encore, mais ramené à ce que la
    // glissière montre — et la glissière, elle, dit où l'on est.
    let scale = scale.clamp(0.8, 1.6);
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
    // --- Les marques d'état -------------------------------------
    //
    // **Une couleur seule ne dit rien à qui ne la voit pas**, et sur
    // un écran fatigué elle ne dit plus grand-chose à personne. Les
    // quatre tons d'un signal se distinguent donc aussi par une
    // *forme* : le cercle barré, le triangle, la coche et l'attente.
    // Quatre silhouettes qu'on reconnaît avant d'avoir lu le mot — et
    // le mot reste, parce qu'une marque seule ne dit pas *quelle*
    // table parle.
    /// Un cercle barré : ce qu'on ne fait pas.
    Stop,
    /// Un triangle : ce sur quoi on s'arrête.
    Warn,
    /// Une coche : ce que la table autorise.
    Check,
    /// Trois points : la table nomme la ligne et attend un chiffre.
    Pending,
    // --- Les gestes de la barre ---------------------------------
    /// Deux traits croisés — le croisement d'une liste.
    Cross,
    /// Un registre : un cahier avec son dos et ses lignes.
    Register,
    /// Un cadre avec un coin plein — où poser une fenêtre.
    Corner,
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
        Pict::Stop => {
            // Le cercle barré : la seule silhouette qu'on lit comme
            // une interdiction sans avoir appris à la lire.
            let c = r.center();
            let rad = w.min(h) * 0.45;
            painter.circle_stroke(c, rad, Stroke::new(1.4_f32, color));
            let d = rad * 0.72;
            painter.line_segment(
                [egui::pos2(c.x - d, c.y + d), egui::pos2(c.x + d, c.y - d)],
                Stroke::new(1.4_f32, color),
            );
        }
        Pict::Warn => {
            // Un triangle plein : la pointe en haut, comme partout.
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(r.center().x, r.top()),
                    egui::pos2(r.right(), r.bottom()),
                    egui::pos2(r.left(), r.bottom()),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Pict::Check => {
            // La même coche que celle d'une case cochée : deux tiers en
            // descendant, un tiers en remontant. Écrite deux fois
            // serait deux coches qui ne se ressemblent plus.
            let s = Stroke::new((w * 0.16).max(1.4), color);
            let inner = r.shrink(w * 0.1);
            painter.line_segment(
                [
                    egui::pos2(inner.left(), inner.center().y),
                    egui::pos2(inner.left() + inner.width() * 0.38, inner.bottom()),
                ],
                s,
            );
            painter.line_segment(
                [
                    egui::pos2(inner.left() + inner.width() * 0.38, inner.bottom()),
                    egui::pos2(inner.right(), inner.top()),
                ],
                s,
            );
        }
        Pict::Pending => {
            // Trois points : ce qui attend. Une forme qui ne prétend
            // pas juger, ce qui est exactement ce que le ton dit.
            let rad = (w * 0.11).max(1.2);
            for i in 0..3 {
                let x = r.left() + w * (0.2 + 0.3 * i as f32);
                painter.circle_filled(egui::pos2(x, r.center().y), rad, color);
            }
        }
        Pict::Cross => {
            painter.line_segment(
                [r.left_top(), r.right_bottom()],
                Stroke::new(1.4_f32, color),
            );
            painter.line_segment(
                [r.right_top(), r.left_bottom()],
                Stroke::new(1.4_f32, color),
            );
        }
        Pict::Register => {
            painter.rect_stroke(r, 0.0, s);
            // Le dos du cahier, puis ses lignes : un registre se
            // distingue d'une feuille par sa reliure.
            let spine = r.left() + w * 0.28;
            painter.line_segment(
                [egui::pos2(spine, r.top()), egui::pos2(spine, r.bottom())],
                s,
            );
            for i in 1..3 {
                let y = r.top() + h * i as f32 / 3.0;
                painter.line_segment(
                    [egui::pos2(spine + 2.0, y), egui::pos2(r.right() - 2.0, y)],
                    s,
                );
            }
        }
        Pict::Corner => {
            // Un écran, et le coin qu'on occupe : la marque du menu qui
            // pose la barre quelque part.
            painter.rect_stroke(r, 0.0, s);
            let part = egui::Rect::from_min_max(
                egui::pos2(r.left() + 2.0, r.top() + 2.0),
                egui::pos2(r.left() + w * 0.5, r.top() + h * 0.5),
            );
            painter.rect_filled(part, 0.0, color);
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
/// Le libellé tel qu'un bouton à pictogramme le **dessine**.
///
/// La place de la marque est réservée dans le texte, et une bande qui
/// mesure ses rangées doit lire la même chaîne que le dessin : mesurée
/// sur le libellé nu, elle est courte de la marque à chaque bouton, ce
/// qui fait passer une rangée à deux sans que rien ne l'annonce.
pub fn icon_label(label: &str) -> String {
    format!("     {label}")
}

pub fn icon_button(ui: &mut egui::Ui, pict: Option<Pict>, label: &str) -> egui::Response {
    icon_button_enabled(ui, pict, label, true)
}

/// Le même, grisé quand `enabled` est faux — et **sa marque avec lui**.
///
/// Une marque restée noire sur un bouton éteint est la seule chose de
/// l'écran qui contredise le gris : c'est elle qu'on voit, donc c'est
/// elle qu'on clique.
pub fn icon_button_enabled(
    ui: &mut egui::Ui,
    pict: Option<Pict>,
    label: &str,
    enabled: bool,
) -> egui::Response {
    let Some(pict) = pict else {
        return button_enabled(ui, label, enabled);
    };
    let ink = if enabled {
        crate::text()
    } else {
        crate::text_dim()
    };
    let resp = button_enabled(ui, &icon_label(label), enabled);
    let size = (resp.rect.height() * 0.42).clamp(8.0, 14.0);
    let square = egui::Rect::from_min_size(
        egui::pos2(resp.rect.left() + 8.0, resp.rect.center().y - size / 2.0),
        egui::vec2(size, size),
    );
    pictogram(ui.painter(), square, pict, ink);
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

/// **La hauteur d'une rangée de cette maison** : celle d'un bouton.
///
/// Elle est calculée ici, où le bouton est dessiné, parce que c'est la
/// seule façon que les deux ne divergent pas. `interact_size.y` — ce
/// qu'egui demande pour un widget nu — est *plus petit* : vingt pixels
/// à l'échelle 1 contre trente et un, et l'écart grandit avec le texte.
/// Une bande qui réserve ses rangées au premier nombre et dessine des
/// boutons est courte de onze pixels par rangée.
///
/// C'est aussi la hauteur d'un [`field`], et ce n'est pas une
/// coquetterie : un champ et un bouton se posent sur la même rangée
/// vingt fois dans cette application, et deux hauteurs sur une rangée
/// se lisent comme un défaut d'alignement. Surtout, à
/// `interact_size.y` la case était **plus courte que son propre
/// texte** : les jambages du « p » de « Dupont » tombaient sur le
/// biseau du bas.
pub fn button_height(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let h = ui.fonts(|f| f.row_height(&font)) + (ui.spacing().button_padding.y + 1.0) * 2.0;
    // **Arrondi à la grille de pixels, comme egui arrondit ce qu'il
    // alloue.** Sans cela cette fonction annonce 37,68 là où le bouton
    // en occupe 38, et une bande qui empile dix rangées est courte de
    // trois pixels — assez pour trancher la dernière. Le sens de
    // l'arrondi est celui qui protège : on réserve le pixel, on ne le
    // gratte pas.
    let ppp = ui.ctx().pixels_per_point();
    (h * ppp).ceil() / ppp
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
pub fn label_rows(ui: &egui::Ui, text: &str, font: &egui::FontId, max_width: f32) -> usize {
    // **Un trait d'union est une occasion de couper**, et egui la prend :
    // mesuré, « lidocaine-bicarbonate-nystatine » se coupe en
    // « lidocaine-bicarbonate- » puis « nystatine », ce qui se lit. Compté
    // comme un seul mot de trente et un caractères, il faisait retomber
    // toute la ligne sur l'ellipse, et « Bain de bouche
    // lidocaïne-bicarbonate-nystatine » s'affichait « Bain de bouche … »
    // — c'est-à-dire le nom de quatre préparations à la fois, dans une
    // liste faite pour les distinguer. Les segments se mesurent donc
    // entre les traits d'union, comme entre les espaces.
    //
    // **Et un segment se mesure, il ne se compte pas en caractères.**
    // Le gabarit était « autant de « 0 » que de lettres », choisi plus
    // large que la moyenne pour pencher vers l'ellipse plutôt que vers
    // une coupure au milieu d'un mot. Mais un chiffre est large et une
    // minuscule ne l'est pas : à `[ui] text_scale = 1,6`, dans le volet
    // des familles thérapeutiques, « Cardiologie et vaisseaux » se
    // lisait « Cardiolo… » — onze lettres comptées pour cent
    // trente-deux pixels là où le mot en occupe cent. La colonne avait
    // la place des deux lignes, et le nom a été élidé pour un gabarit.
    //
    // Mesuré dans la fonte qui dessine, le gabarit n'a plus à pencher :
    // la condition devient exactement celle qu'egui appliquera — un
    // segment qui tient entre deux coupures possibles ne sera pas coupé.
    let longest = ui.fonts(|f| {
        text.split_whitespace()
            .flat_map(|w| w.split('-'))
            .fold(0.0_f32, |m, w| {
                m.max(
                    f.layout_no_wrap(w.to_owned(), font.clone(), crate::text())
                        .size()
                        .x,
                )
            })
    });
    if longest <= max_width {
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
/// La galée d'une rangée à deux moitiés, **la plus riche qui tienne**.
///
/// Posées dans une seule galée, les deux moitiés s'élidaient ensemble :
/// sur un volet étroit, « Ebixa » — un nom complet — se lisait
/// « Ebixa … », et ce point de suspension fait croire qu'il manque
/// quelque chose au nom. Pire, la moitié tranquille prenait la place
/// que le nom n'avait plus : « Effentora » et « Efferalgan » sortaient
/// coupés alors qu'ils tiennent entiers sans elle.
///
/// C'est la règle de la maison, celle de `richest_form` : de la plus
/// riche à la plus pauvre, et la première qui tient. Le nom seul tient
/// presque toujours ; s'il ne tient pas lui-même, il s'élide — un nom
/// coupé reste plus utile qu'une rangée vide.
fn pair_galley(
    ui: &egui::Ui,
    primary: &str,
    secondary: &str,
    selected: bool,
    indent: f32,
) -> std::sync::Arc<egui::Galley> {
    let width = ui.available_width();
    // Both faces come from the style, so the pair grows with the text
    // scale like the plain row beside it.
    let font = egui::TextStyle::Body.resolve(ui.style());
    let quiet = egui::FontId::new(font.size * 0.86, font.family.clone());
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
    let lay = |secondary: &str| -> std::sync::Arc<egui::Galley> {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            primary,
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
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
                    font_id: quiet.clone(),
                    color: dim,
                    italics: true,
                    ..Default::default()
                },
            );
        }
        let max_width = (width - 12.0 - indent).max(1.0);
        job.wrap = egui::text::TextWrapping {
            max_width,
            max_rows: label_rows(ui, &job.text, &font, max_width),
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        ui.fonts(|f| f.layout_job(job))
    };
    let full = lay(secondary);
    if full.elided && !secondary.is_empty() {
        lay("")
    } else {
        full
    }
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
    let color = if selected {
        Color32::WHITE
    } else {
        crate::text()
    };
    // **Mise en page d'abord, hauteur ensuite.** La rangée était
    // allouée sur une ligne puis peinte dedans, donc « Paul Bernard »
    // sur un volet étroit se lisait « Paul … » — et deux patients de
    // même prénom devenaient la même rangée. Elle prend deux lignes
    // quand il en faut deux, comme `list_row_count`, et la hauteur
    // suit la galée au lieu de la précéder.
    let galley = pair_galley(ui, primary, secondary, selected, indent);
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
///
/// `ink` est **la couleur de la ligne, ou celle de tout le monde** —
/// et non un booléen « en gris ». Le booléen ne savait dire qu'une
/// nuance, alors que la couleur d'une ligne de liste dit ici trois
/// choses différentes selon l'écran : une classe que la base ne peuple
/// pas, un produit archivé, un produit à aller compter. La liste des
/// stupéfiants portait la troisième et ne pouvait donc pas employer ce
/// widget : elle composait « libellé · solde » en une seule chaîne, que
/// l'élision mangeait par la fin — « Méthadone AP-HP gélule 40 mg ·
/// 1… », c'est-à-dire le solde perdu, qui est la seule raison de
/// regarder cette liste. Sur la sélection, l'encre reste blanche quelle
/// que soit la demande : un rouge sur le bleu de sélection ne se lit
/// pas.
///
/// `indent` décale le libellé, pour une liste rangée sous des
/// intertitres — même rôle que dans [`list_row_pair`].
///
/// `widest` est **le plus large des chiffres de la liste**, et non
/// celui de cette ligne : une rangée ne voit pas ses voisines, si bien
/// que la bascule se décidait ligne par ligne et qu'un caractère de
/// plus suffisait à la faire tomber de l'autre côté. Dans la liste des
/// tables, « IPP · 9 lignes » restait côte à côte et « Statines /
/// 12 lignes » passait dessous : une liste à deux dispositions se lit
/// comme un défaut de rendu. Une réserve commune, c'est une colonne —
/// ce que cette rangée est en réalité. Une liste dont les chiffres ont
/// tous la même largeur passe `count`, et rien ne change.
pub fn list_row_count(
    ui: &mut egui::Ui,
    label: &str,
    count: &str,
    widest: &str,
    selected: bool,
    ink: Option<Color32>,
    indent: f32,
) -> egui::Response {
    let width = ui.available_width();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let row = ui.fonts(|f| f.row_height(&font));
    let ink = if selected {
        Color32::WHITE
    } else {
        ink.unwrap_or_else(crate::text)
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
    let reserved = ui
        .painter()
        .layout_no_wrap(widest.to_owned(), font.clone(), quiet)
        .size()
        .x
        .max(num.size().x)
        + 16.0;
    // **Et le chiffre cède la ligne quand il ne laisse plus de quoi
    // lire le libellé.** Réservé à droite, « 14 gélules » prend les
    // deux tiers d'une colonne étroite, et ce qui restait au nom était
    // assez pour une lettre par ligne : « Mé / tha / don / e ». Un
    // chiffre sauvé au prix d'un libellé en confettis n'a rien sauvé.
    // Sous un seuil — la moitié de la ligne — les deux se superposent
    // donc au lieu de se partager la largeur : le libellé sur toute la
    // colonne, le chiffre dessous, et rien n'est perdu.
    let stacked = width - 8.0 - indent - reserved < width * 0.5;
    let room = if stacked {
        (width - 16.0 - indent).max(8.0)
    } else {
        (width - 8.0 - indent - reserved).max(8.0)
    };
    // Deux lignes si le libellé peut se couper proprement, une sinon —
    // la règle de [`label_rows`], qui manquait ici : `Painter::layout`
    // enroule sans borne et coupe au milieu des mots, ce que le reste
    // de la maison refuse depuis longtemps.
    let mut job = egui::text::LayoutJob::simple(label.to_owned(), font.clone(), ink, room);
    job.wrap.max_rows = label_rows(ui, label, &font, room);
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    let text = ui.fonts(|f| f.layout_job(job));
    // **Superposé, le chiffre enveloppe plutôt que de déborder.** Posé
    // sans borne il sortait du puits : « 21 comprimés sublinguaux » se
    // lisait « 21 comprimés sublir », coupé par le cadre et sans rien
    // pour le dire. Réservé à droite il tient par construction ; sous le
    // libellé, il faut le lui dire. Deux lignes, et l'unité cède avant
    // le nombre — c'est le nombre qu'on vient lire.
    let num = if stacked {
        let mut job = egui::text::LayoutJob::simple(count.to_owned(), font.clone(), quiet, room);
        job.wrap.max_rows = 2;
        job.wrap.break_anywhere = false;
        job.wrap.overflow_character = Some('…');
        ui.fonts(|f| f.layout_job(job))
    } else {
        num
    };
    let content = if stacked {
        text.size().y + num.size().y + 2.0
    } else {
        text.size().y
    };
    let height = (ui.spacing().interact_size.y + 2.0)
        .max(row + 4.0)
        .max(content + 6.0)
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
    let top = rect.center().y - content / 2.0;
    let text_h = text.size().y;
    ui.painter().galley(
        egui::pos2(
            rect.left() + 8.0 + indent,
            if stacked {
                top
            } else {
                rect.center().y - text_h / 2.0
            },
        ),
        text,
        ink,
    );
    ui.painter().galley(
        if stacked {
            egui::pos2(rect.left() + 8.0 + indent, top + text_h + 2.0)
        } else {
            egui::pos2(
                rect.right() - 8.0 - num.size().x,
                rect.center().y - num.size().y / 2.0,
            )
        },
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

/// Une case à cocher Motif : un carré **levé** quand elle est vide,
/// **creusé et marqué** quand elle est cochée.
///
/// C'est la règle de la maison appliquée au seul endroit qui y avait
/// échappé — ce qu'on remplit descend, ce qu'on presse monte. Celle
/// d'egui est un carré plat cerné d'un trait d'un pixel : à côté d'un
/// bouton biseauté et d'un champ creusé, elle se lit comme une bordure
/// oubliée, et surtout **vide et cochée se ressemblent** de loin, la
/// seule différence étant une coche fine dans la même couleur que le
/// cadre. Le relief, lui, se voit avant qu'on ait lu.
///
/// Rend la réponse, marquée modifiée quand l'état a changé : l'appelant
/// y accroche son infobulle comme sur n'importe quel widget.
pub fn checkbox(
    ui: &mut egui::Ui,
    on: &mut bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let row = ui.spacing().interact_size.y;
    let gap = ui.spacing().item_spacing.x * 0.6;
    // **Le libellé est pris tel qu'on le donne** — sa taille, son
    // encre : c'est la leçon de `list_row`, qui prenait un `RichText`,
    // en gardait la chaîne et jetait le reste. Le carré suit *cette*
    // fonte-là et non une constante : une case de treize pixels à côté
    // d'un texte de vingt-deux est une case qu'on rate.
    let galley = label.into().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let box_side = (galley.size().y + 2.0).min(row);
    let size = Vec2::new(
        box_side + gap + galley.size().x,
        row.max(galley.size().y).max(box_side),
    );
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let square = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.center().y - box_side / 2.0),
            Vec2::splat(box_side),
        );
        let fill = if *on {
            crate::trough()
        } else if response.hovered() {
            crate::bg_hover()
        } else {
            crate::bg()
        };
        ui.painter().rect_filled(square, 0.0, fill);
        bevel(ui.painter(), square, !*on);
        if *on {
            // La coche, en deux segments : deux tiers en descendant,
            // un tiers en remontant. Dessinée et non écrite — la fonte
            // livrée n'a pas toutes les marques, et c'est le genre de
            // caractère qui sort en carré vide.
            let s = square.shrink(box_side * 0.28);
            let ink = Stroke::new((box_side * 0.14).max(1.5), crate::text());
            ui.painter().line_segment(
                [
                    egui::pos2(s.left(), s.center().y),
                    egui::pos2(s.left() + s.width() * 0.38, s.bottom()),
                ],
                ink,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(s.left() + s.width() * 0.38, s.bottom()),
                    egui::pos2(s.right(), s.top()),
                ],
                ink,
            );
        }
        ui.painter().galley(
            egui::pos2(
                square.right() + gap,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            crate::text(),
        );
    }
    response
}

/// Un bouton radio Motif : un **losange**, levé quand il est libre,
/// creusé et plein quand il est choisi.
///
/// Le losange n'est pas une coquetterie : c'est ce qui distingue à
/// l'œil « un seul parmi ceux-ci » de « chacun indépendamment », et
/// c'est la forme que Motif lui donne. Deux carrés voisins dont l'un
/// est exclusif et l'autre non ne se distinguent que par l'essai.
pub fn radio(ui: &mut egui::Ui, on: bool, label: impl Into<egui::WidgetText>) -> egui::Response {
    let row = ui.spacing().interact_size.y;
    let gap = ui.spacing().item_spacing.x * 0.6;
    let galley = label.into().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let side = (galley.size().y + 2.0).min(row);
    let size = Vec2::new(
        side + gap + galley.size().x,
        row.max(galley.size().y).max(side),
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let c = egui::pos2(rect.left() + side / 2.0, rect.center().y);
        let r = side / 2.0;
        let (top, right, bottom, left) = (
            egui::pos2(c.x, c.y - r),
            egui::pos2(c.x + r, c.y),
            egui::pos2(c.x, c.y + r),
            egui::pos2(c.x - r, c.y),
        );
        let fill = if on {
            crate::accent()
        } else if response.hovered() {
            crate::bg_hover()
        } else {
            crate::bg()
        };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![top, right, bottom, left],
            fill,
            Stroke::NONE,
        ));
        // Les deux arêtes du haut portent la lumière quand il est
        // libre, l'ombre quand il est pris : le même mouvement que le
        // biseau d'un carré, sur une forme qui n'en a pas.
        let (lit, dark) = if on {
            (crate::bg_dark(), crate::bg_light())
        } else {
            (crate::bg_light(), crate::bg_dark())
        };
        for (a, b, ink) in [
            (left, top, lit),
            (top, right, lit),
            (right, bottom, dark),
            (bottom, left, dark),
        ] {
            ui.painter().line_segment([a, b], Stroke::new(1.5_f32, ink));
        }
        ui.painter().galley(
            egui::pos2(
                rect.left() + side + gap,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            crate::text(),
        );
    }
    response
}

/// Une pastille : un aplat, **son relief**, sa marque et son mot.
///
/// Elle était partout une étiquette à fond coloré — `RichText` avec un
/// `background_color` —, c'est-à-dire le seul objet de cette interface
/// sans biseau : dans un décor où tout est gravé, un rectangle plat ne
/// se lit pas comme un objet mais comme une surbrillance.
///
/// **Et la marque dit ce que la couleur dit.** Une couleur seule ne
/// dit rien à qui ne la voit pas, et sur un écran fatigué elle ne dit
/// plus grand-chose à personne : le cercle barré, le triangle, la
/// coche et les trois points se reconnaissent avant d'être lus. Le mot
/// reste, parce qu'une marque ne dit pas *de quoi* on parle.
///
/// La marque est donnée par l'appelant et jamais déduite de la
/// couleur : la couleur vient du thème, et deux des dix peaux sont des
/// peaux de nuit.
pub fn badge(
    ui: &mut egui::Ui,
    text: &str,
    mark: Option<Pict>,
    fill: Color32,
    raised: bool,
) -> egui::Response {
    let ink = on_fill(fill);
    let font = badge_font(ui);
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_owned(), font, ink));
    let pad = ui.spacing().button_padding * 0.6 + Vec2::splat(2.0);
    // **Elle écoute le clic**, parce que certaines se cliquent — les
    // puces du compagnon ouvrent l'écran qui répond en entier — et
    // qu'une seconde écriture de la même pastille finirait par ne plus
    // lui ressembler. Celles qu'on ne clique pas ignorent la réponse.
    let (rect, resp) =
        ui.allocate_exact_size(badge_size(ui, text, mark.is_some()), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, fill);
    bevel(painter, rect, raised);
    let room = match mark {
        Some(mark) => {
            let (side, room) = badge_mark_room(ui);
            let square = egui::Rect::from_min_size(
                egui::pos2(rect.left() + pad.x, rect.center().y - side / 2.0),
                Vec2::splat(side),
            );
            pictogram(painter, square, mark, ink);
            room
        }
        None => 0.0,
    };
    painter.galley(
        egui::pos2(
            rect.left() + pad.x + room,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        ink,
    );
    resp
}

/// La fonte d'une pastille — petite, mais **prise au barreau** : un
/// nombre de pixels ne suivrait pas `[ui] text_scale`.
fn badge_font(ui: &egui::Ui) -> egui::FontId {
    egui::FontId::proportional(pt(ui, 10.5))
}

/// Le côté de la marque d'une pastille, et l'air après elle.
///
/// Écrit une fois : la mesure et le dessin le lisent, sinon la pastille
/// est mesurée sans sa marque et son dernier mot sort du rectangle.
fn badge_mark_room(ui: &egui::Ui) -> (f32, f32) {
    let side = pt(ui, 10.5);
    (side, side + pt(ui, 4.0))
}

/// Ce qu'une pastille occupe. **C'est la fonction qui l'annonce**, et
/// le dessin la lit : deux écritures d'une même taille finissent par se
/// contredire, et c'est alors la promesse qui ment.
pub fn badge_size(ui: &egui::Ui, text: &str, marked: bool) -> Vec2 {
    let font = badge_font(ui);
    let w = ui.fonts(|f| {
        f.layout_no_wrap(text.to_owned(), font, crate::text())
            .size()
    });
    let room = if marked { badge_mark_room(ui).1 } else { 0.0 };
    w + ui.spacing().button_padding * 1.2 + Vec2::new(4.0 + room, 4.0)
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

/// Le séparateur gravé d'un `XmSeparator`, **posé dans la rangée** :
/// il prend la place, là où [`rule`] peint à des coordonnées.
///
/// Les deux existaient déjà sous forme de peintres — `rule` et
/// [`vrule`] —, et c'est justement ce qui manquait : une mise en page
/// découpée sait où poser son trait, une mise en page qui coule ne le
/// sait pas, et `ui.separator()` d'egui est un trait d'un pixel tiré de
/// `widgets.noninteractive.bg_stroke`. À côté d'un panneau biseauté et
/// d'un champ creusé, il se lit comme une bordure oubliée. Le creux dit
/// ce qu'il faut : cette ligne est en retrait, donc elle sépare.
///
/// La hauteur prise est celle d'un espacement, filet compris : une
/// bande qui mesure ses rangées compte `item_spacing.y` pour lui et
/// rien de plus.
pub fn separator(ui: &mut egui::Ui) {
    let h = ui.spacing().item_spacing.y.max(6.0);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        rule(
            ui.painter(),
            rect.left(),
            rect.right(),
            rect.center().y.round(),
        );
    }
}

/// Le même, debout : ce qui sépare deux groupes d'une barre d'outils.
///
/// La hauteur vient de la rangée qui l'accueille — un filet plus haut
/// que les boutons qu'il sépare déborde sur le panneau, et un filet
/// plus court flotte au milieu de rien. Sa largeur, espacement compris,
/// est [`separator_v_width`] : une barre qui mesure ses groupes avant
/// de les dessiner la lit là plutôt que de la recopier.
pub fn separator_v(ui: &mut egui::Ui, height: f32) {
    let w = separator_v_width(ui) - ui.spacing().item_spacing.x;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        vrule(
            ui.painter(),
            rect.top(),
            rect.bottom(),
            rect.center().x.round(),
        );
    }
}

/// Ce qu'un [`separator_v`] coûte à une rangée : son filet **et** la
/// gouttière que la mise en page insère après lui.
///
/// Deux écritures d'une largeur finissent par diverger, et c'est la
/// mesure qui ment — la règle que ce dépôt applique déjà aux colonnes
/// d'un tableau et aux groupes qu'on garde ensemble.
pub fn separator_v_width(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing().item_spacing.x;
    spacing.max(6.0) + spacing
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
        //
        // Et les douze pixels que le filet gardait en réserve, il les
        // rend aussi : un intitulé qui va s'élider vaut mieux entier
        // qu'accompagné d'un trait. « Médicaments » — le titre du dock
        // du référentiel — en tenait à huit pixels près à
        // `text_scale = 1,6`, et sortait « Médicame… » avec un filet
        // impeccable à côté. Un intitulé court, lui, ne voit pas la
        // différence : sa galée est plus étroite que les deux bornes, et
        // le filet reprend tout ce qui reste.
        let room = (ui.available_width() - 4.0).max(pt(ui, 24.0));
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
    scale_range(ui, width, value, 0.0, max, step)
}

/// Le relief d'une boîte de dialogue : un biseau levé sur son bord.
///
/// Une fenêtre flotte au-dessus du plan de travail, et dans ce chrome
/// cela se dit par un relief — c'est la même phrase que pour une liste
/// déroulée, et c'est le même objet : quelque chose posé par-dessus.
/// egui l'encadre d'un trait d'un pixel, qui se lit comme une bordure
/// oubliée à côté d'un panneau biseauté.
///
/// Peint sur **la couche de la fenêtre** et non sur celle qui l'appelle
/// : peint sur l'appelante, le biseau passerait sous le fond de la
/// fenêtre, c'est-à-dire nulle part. Et après son contenu, donc par
/// dessus — ce qui est juste, puisqu'il est sur le bord.
pub fn dialog_relief<R>(ctx: &egui::Context, shown: &Option<egui::InnerResponse<R>>) {
    let Some(out) = shown else { return };
    let painter = ctx.layer_painter(out.response.layer_id);
    bevel(&painter, out.response.rect, true);
}

/// Le relief d'une liste déroulée : un biseau levé autour d'elle.
///
/// Le panneau d'un menu ouvert est une chose qui **flotte au-dessus**
/// du reste, et dans ce chrome cela se dit d'une façon et d'une seule :
/// par un relief. egui l'encadre d'un trait d'un pixel, ce qui est la
/// même faute que partout ailleurs ici — le trait se lit comme une
/// bordure oubliée, et la liste comme une tache de la couleur du
/// panneau posée sur le panneau.
///
/// Posé **après** les lignes, et c'est voulu : le biseau tombe dans la
/// marge du cadre, où rien n'est peint, donc l'ordre ne coûte rien et
/// on n'a pas à connaître la hauteur avant de l'avoir remplie.
fn popup_relief(ui: &egui::Ui) {
    bevel(ui.painter(), ui.min_rect().expand(4.0), true);
}

/// Le contrôle fermé d'un menu : relief levé, libellé, marque.
///
/// Écrit une fois et partagé par [`select`] et [`menu`] : deux dessins
/// du même objet finiraient par ne plus se ressembler, et c'est
/// justement l'objet dont tout l'intérêt est de se reconnaître.
fn menu_head(ui: &mut egui::Ui, id: egui::Id, width: f32, shown: &str) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let open = ui.memory(|m| m.is_popup_open(id));
        let fill = if open {
            crate::trough()
        } else if response.hovered() {
            crate::bg_hover()
        } else {
            crate::bg()
        };
        ui.painter().rect_filled(rect, 0.0, fill);
        bevel(ui.painter(), rect, !open);
        // La marque, à droite : un triangle plein, le même que celui
        // d'un menu déroulant Motif.
        let m = height * 0.22;
        let cx = rect.right() - m - 6.0;
        let cy = rect.center().y;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(cx - m, cy - m * 0.5),
                egui::pos2(cx + m, cy - m * 0.5),
                egui::pos2(cx, cy + m * 0.7),
            ],
            crate::text(),
            Stroke::NONE,
        ));
        // Le libellé, à gauche, et **coupé à ce qui reste** : laissé
        // sans largeur il peindrait par-dessus la marque et jusque sur
        // le widget d'à côté — un `Painter` peint où on lui dit, rien
        // ne le coupe.
        if !shown.is_empty() {
            let room = (cx - m - rect.left() - 12.0).max(0.0);
            let mut job =
                egui::text::LayoutJob::simple_singleline(shown.to_owned(), font, crate::text());
            job.wrap.max_width = room;
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = Some('…');
            let galley = ui.fonts(|f| f.layout_job(job));
            ui.painter().galley(
                egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
                galley,
                crate::text(),
            );
        }
    }
    response
}

/// Un **menu** : il ne montre pas une valeur, il en propose une.
///
/// La différence avec [`select`] n'est pas cosmétique. Un menu d'actions
/// — « quel opérateur signe », « depuis quelle fiche » — n'a pas de
/// valeur courante à afficher : ce qu'on y choisit part ailleurs, et
/// écrire le dernier choix dans la case ferait croire à un réglage.
/// `label` peut donc être vide : il ne reste que la marque, ce qui est
/// exactement ce qu'on veut à côté d'un champ.
///
/// Rend ce qui a été choisi — ou `None` — **et la réponse du contrôle
/// fermé**, pour que l'appelant y accroche son infobulle. Une marque
/// seule, sans texte, n'a rien qui dise ce qu'elle ouvre : l'infobulle
/// est alors la seule explication qu'il y ait.
pub fn menu<T: Clone>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    width: f32,
    label: &str,
    options: &[(T, String)],
) -> egui::InnerResponse<Option<T>> {
    let id = ui.make_persistent_id(id_salt);
    let response = menu_head(ui, id, width, label);
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(id));
    }
    let mut picked = None;
    egui::popup::popup_below_widget(
        ui,
        id,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(width);
            for (value, text) in options {
                if list_row(ui, egui::RichText::new(text), false).clicked() {
                    picked = Some(value.clone());
                }
            }
            popup_relief(ui);
        },
    );
    egui::InnerResponse::new(picked, response)
}

/// Le même menu, **avec une marque au lieu d'un libellé**.
///
/// Un menu d'actions n'a pas de valeur courante à afficher — c'est ce
/// qui le distingue d'un [`select`] — et il ne lui restait donc que son
/// triangle : une case vide surmontée d'une marque, dont rien ne dit ce
/// qu'elle ouvre. L'infobulle le disait, mais une infobulle se lit
/// après avoir cherché ; un pictogramme se lit avant.
pub fn menu_marked<T: Clone>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    width: f32,
    mark: Pict,
    options: &[(T, String)],
) -> egui::InnerResponse<Option<T>> {
    let out = menu(ui, id_salt, width, "", options);
    let rect = out.response.rect;
    let side = (rect.height() * 0.46).clamp(9.0, 16.0);
    let square = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 6.0, rect.center().y - side / 2.0),
        Vec2::splat(side),
    );
    pictogram(ui.painter(), square, mark, crate::text());
    out
}

/// Un champ de saisie **creusé**, comme tout ce qui se remplit ici.
///
/// Les boutons de cette interface montent, les champs descendent : c'est
/// la seule chose qui distingue à l'œil ce qu'on presse de ce qu'on
/// remplit, et c'est le mouvement que le reste du chrome suit partout.
/// Les champs, eux, étaient dessinés par egui : un rectangle de la
/// couleur du creux, cerné d'un trait d'un pixel. La couleur disait
/// « ici on écrit » ; le relief ne disait rien, et à côté d'un bouton
/// biseauté cela se voit.
///
/// Le `TextEdit` est passé tel quel, avec ses réglages — invite, mot de
/// passe, alignement : ce widget ne remplace pas le vôtre, il l'encadre.
/// Son propre cadre est éteint, sans quoi il y aurait deux bords.
///
/// **Le foyer se voit.** Un liseré d'accent à l'intérieur du biseau,
/// quand le champ a la main : sur un formulaire de dix champs, savoir
/// où l'on tape est ce qu'on demande d'abord à un écran.
pub fn field(ui: &mut egui::Ui, width: f32, edit: egui::TextEdit<'_>) -> egui::Response {
    // **La hauteur est celle d'un bouton**, et pas `interact_size.y`.
    // Ce dernier est ce qu'egui demande pour un widget nu : vingt
    // pixels à l'échelle 1, pour un texte qui en occupe dix-sept et un
    // `TextEdit` qui ajoute quatre de marge. La case était donc plus
    // courte que ce qu'elle contient — mesuré sur une capture du
    // dossier : les jambages du « p » de « Dupont » tombaient *sur* le
    // biseau du bas, et la lettre haute touchait celui du haut.
    //
    // Et c'est la même hauteur qu'un bouton parce qu'ils partagent une
    // rangée vingt fois ici : deux hauteurs sur une rangée se lisent
    // comme un défaut d'alignement, pas comme une intention.
    field_sized(ui, Vec2::new(width, button_height(ui)), edit)
}

/// Le même, quand la rangée qui l'accueille a déjà décidé sa hauteur.
///
/// Une grille de saisie mesure ses rangées une fois et pose tout
/// dedans ; un champ qui reprendrait la hauteur du style s'y
/// désalignerait d'un ou deux pixels par ligne, ce qui se voit sur
/// cinq lignes et se lit comme un défaut de rendu.
pub fn field_sized(ui: &mut egui::Ui, size: Vec2, edit: egui::TextEdit<'_>) -> egui::Response {
    // Toute la largeur, et le texte au milieu de la hauteur : une
    // ligne se centre dans sa case.
    let layout = egui::Layout::left_to_right(egui::Align::Center).with_main_justify(true);
    // **Et jamais plus courte que ce qu'elle contient.** Trois bandes
    // trop serrées pour une rangée entière demandent encore
    // `interact_size.y`, et chacune y perdait les jambages de son
    // texte. Une bande peut refuser une rangée à un champ ; elle ne
    // peut pas lui refuser sa propre hauteur de texte, parce que ce
    // qu'on tape dedans est ce qu'on relit.
    sunken(
        ui,
        Vec2::new(size.x, size.y.max(field_floor(ui))),
        edit,
        layout,
    )
}

/// Ce qu'une case d'une ligne demande **au minimum** : son texte, la
/// marge propre du `TextEdit` et le creux autour.
///
/// La hauteur ordinaire d'un champ est celle d'une rangée
/// ([`button_height`]) ; celle-ci est le plancher sous lequel la case
/// devient plus courte que ce qu'elle contient — ce qui était le cas
/// partout où `interact_size.y` servait de hauteur.
pub fn field_floor(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    // 4 : la marge propre du `TextEdit` ; 6 : le creux de [`sunken`].
    ui.fonts(|f| f.row_height(&font)) + 4.0 + 6.0
}

/// Une **zone** de saisie : le même creux, pour plusieurs lignes.
///
/// Elle existe parce que le creux s'était arrêté à mi-chemin. Les cent
/// soixante-dix cases d'une ligne sont passées par [`field`] ; les
/// treize zones de texte — les notes d'équipe, la console, le collage
/// de conciliation, l'éditeur de modèles, la remarque de caisse — sont
/// restées peintes par egui, c'est-à-dire plates. Et une case plate
/// entourée de cases creusées ne se lit pas comme « il en reste une » :
/// elle se lit comme un défaut de rendu, parce que c'est devenu la
/// seule de l'écran.
///
/// La différence avec [`field_sized`] tient en un mot : le texte s'y
/// range **en haut**, et non au milieu. Une ligne se centre dans sa
/// case ; dix lignes commencent au bord haut, sans quoi un paragraphe
/// qui grandit ferait remonter sa première ligne à chaque frappe.
pub fn area(ui: &mut egui::Ui, size: Vec2, edit: egui::TextEdit<'_>) -> egui::Response {
    // Toute la case, dans les deux sens : ce qu'on tape doit pouvoir
    // descendre jusqu'au bas du creux, et cliquer sous la dernière
    // ligne doit poser le curseur au bout du texte.
    let layout = egui::Layout::top_down(egui::Align::Min)
        .with_main_justify(true)
        .with_cross_justify(true);
    sunken(ui, size, edit, layout)
}

/// La même zone, quand ce qu'on y écrit peut dépasser la case.
///
/// Un `TextEdit` grandit avec son contenu : une zone qui tient une
/// section de monographie débordait sur les rangées d'en dessous et
/// peignait par-dessus leurs intitulés. Ici le texte défile **dans** le
/// creux, qui garde la hauteur qu'on lui a donnée.
///
/// L'`id` est celui de la zone de défilement, et il doit être unique
/// dans la vue : deux `ScrollArea` anonymes dans un même écran partagent
/// leur identité et egui peint sa plainte en rouge en travers.
pub fn area_scrolled(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    size: Vec2,
    edit: egui::TextEdit<'_>,
) -> egui::Response {
    let (rect, frame) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, crate::trough());
        bevel(ui.painter(), rect, false);
    }
    let inner = rect.shrink2(Vec2::new(6.0, 3.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    // La barre de défilement flotte : ce qui passe dessous ne se voit
    // pas, et ici ce qui passe dessous est ce qu'on vient d'écrire. Le
    // réglage se pose sur le `Ui` **avant** d'ouvrir la zone : écrit
    // dans son contenu, il n'atteint que les zones imbriquées.
    child.spacing_mut().scroll.floating = false;
    let response = egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(inner.height())
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let w = ui.available_width();
            ui.add(edit.desired_width(w).frame(false))
        })
        .inner;
    if response.has_focus() && ui.is_rect_visible(rect) {
        ui.painter()
            .rect_stroke(rect.shrink(2.0), 0.0, Stroke::new(1.0_f32, crate::accent()));
    }
    if frame.clicked() {
        response.request_focus();
    }
    response | frame
}

/// Ce que les deux partagent : le creux, le liseré, et la géométrie.
///
/// **Trois choses qu'un `ui.put` ne donne pas.**
///
/// La première est le rectangle rendu. `put` rend la réponse du
/// `TextEdit`, dont le rectangle est celui du *texte* — six pixels plus
/// étroit et trois plus court que la case dessinée. Or c'est à ce
/// rectangle-là que l'appelant accroche son infobulle, sa liste de
/// propositions et son propre biseau : quatre vues en dessinaient un
/// second, deux pixels *à l'intérieur* du premier, et la case sortait à
/// double bord. On rend donc l'identité et l'état du champ avec la
/// géométrie du cadre — c'est exactement ce que fait l'union de deux
/// réponses, qui garde l'`Id` de la première.
///
/// La deuxième est le curseur de la rangée. `put` alloue une seconde
/// fois, et comme le rectangle intérieur est en retrait, l'allocation
/// *recule* le curseur de six pixels : le widget suivant mordait sur la
/// marge de celui-ci. Un `new_child` ne prend pas de place, et la place
/// a déjà été prise.
///
/// La troisième est le clic. La marge du biseau appartenait à personne :
/// cliquer sur le bord d'une case ne faisait rien, ce qui est le genre
/// de détail qu'on ne signale jamais et qui fait cliquer deux fois. Le
/// cadre est donc alloué **avant** le texte et écoute le clic — egui
/// donne la main au dernier inscrit là où deux se recouvrent, donc le
/// `TextEdit` garde le sien et la marge renvoie vers lui.
fn sunken(
    ui: &mut egui::Ui,
    size: Vec2,
    edit: egui::TextEdit<'_>,
    layout: egui::Layout,
) -> egui::Response {
    sunken_with(ui, size, Some(layout), |ui| {
        let r = ui.add(edit.frame(false));
        (r.clone(), r)
    })
    .0
}

/// Le creux, le liseré et la géométrie — autour de **ce qu'on voudra**.
///
/// Un éditeur qui sait compléter ce qu'on tape a besoin de savoir où est
/// le curseur, donc de la sortie entière d'un `TextEdit` et pas
/// seulement de sa réponse. Le cadre, lui, ne change pas : il est écrit
/// ici une fois, et trois widgets le portent.
fn sunken_with<R>(
    ui: &mut egui::Ui,
    size: Vec2,
    layout: Option<egui::Layout>,
    add: impl FnOnce(&mut egui::Ui) -> (egui::Response, R),
) -> (egui::Response, R) {
    let (rect, frame) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, crate::trough());
        bevel(ui.painter(), rect, false);
    }
    // Ce qui reste une fois le biseau posé : deux pixels de chaque
    // côté, plus l'air qu'un texte demande pour ne pas toucher le bord.
    let inner = rect.shrink2(Vec2::new(6.0, 3.0));
    let mut builder = egui::UiBuilder::new().max_rect(inner);
    if let Some(layout) = layout {
        builder = builder.layout(layout);
    }
    let mut child = ui.new_child(builder);
    let (response, extra) = add(&mut child);
    if response.has_focus() && ui.is_rect_visible(rect) {
        ui.painter()
            .rect_stroke(rect.shrink(2.0), 0.0, Stroke::new(1.0_f32, crate::accent()));
    }
    if frame.clicked() {
        response.request_focus();
    }
    (response | frame, extra)
}

/// Une zone de saisie de **code** : le même creux, et la sortie entière
/// du `TextEdit` — sa galée et son curseur avec sa réponse.
///
/// C'est ce qu'il faut pour proposer une complétion : sans la galée on
/// ne sait pas *où* poser la liste, et une liste posée sous la case
/// plutôt que sous le mot en cours se lit comme un panneau de plus.
///
/// Elle défile dans les deux sens : une ligne de code ne se replie pas
/// — un repli change les numéros de ligne que la console rapporte dans
/// ses erreurs, et c'est par eux qu'on retrouve la faute.
pub fn code_area(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    size: Vec2,
    edit: egui::TextEdit<'_>,
) -> (egui::Response, egui::text_edit::TextEditOutput) {
    sunken_with(ui, size, None, |ui| {
        // **Le réglage se pose sur le `Ui` qui ouvre la zone**, pas
        // dans son contenu : `ScrollArea` lit l'espacement au moment
        // où on l'ouvre, et une ligne écrite à l'intérieur n'atteint
        // que les zones imbriquées. Posée là, elle ne faisait rien —
        // et une barre flottante sur un éditeur de code est une barre
        // invisible sur ce qui dépasse à droite, c'est-à-dire sur la
        // moitié d'une ligne longue.
        ui.spacing_mut().scroll.floating = false;
        egui::ScrollArea::both()
            .id_salt(id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let out = edit.frame(false).show(ui);
                (out.response.clone(), out)
            })
            .inner
    })
}

/// Un menu d'options : **relief levé**, comme un bouton, avec sa marque.
///
/// Le `ComboBox` d'egui se peint dans `widgets.*.weak_bg_fill`, que
/// `apply` met au fond du panneau pour tous les états — il sortait donc
/// plat, de la couleur du panneau, cerné d'un trait d'un pixel, à côté
/// de boutons biseautés. C'est le même piège que la glissière : tout ce
/// qu'egui dessine à partir de ce champ-là n'a pas de relief ici.
///
/// Rend la [`egui::Response`] du contrôle fermé, marquée modifiée quand
/// le choix a changé : l'appelant peut donc y accrocher son infobulle,
/// comme sur n'importe quel widget.
///
/// Le libellé de chaque option est donné par l'appelant, jamais deviné :
/// ces listes portent des intitulés de `strings.fr.toml`, et un `Debug`
/// qui s'échapperait à l'écran serait de l'anglais au comptoir.
pub fn select<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    width: f32,
    current: &mut T,
    options: &[(T, String)],
) -> egui::Response {
    let hinted: Vec<(T, String, String)> = options
        .iter()
        .map(|(v, l)| (v.clone(), l.clone(), String::new()))
        .collect();
    select_hinted(ui, id_salt, width, current, &hinted)
}

/// Le même, quand **chaque ligne a quelque chose à expliquer**.
///
/// « Les semaines paires » et « une semaine sur deux » portent le même
/// nombre de mots et ne sont pas la même règle : elles s'accordent des
/// années puis divergent pour toujours à la première année ISO de
/// cinquante-trois semaines. Ce genre d'écart se dit sur la ligne qui
/// le porte, au moment où l'on choisit — pas dans un manuel.
///
/// Une explication vide ne pose pas d'infobulle : ce n'est pas la même
/// chose qu'une infobulle vide, qui est une boîte qui s'ouvre pour ne
/// rien dire.
pub fn select_hinted<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    width: f32,
    current: &mut T,
    options: &[(T, String, String)],
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    let shown = options
        .iter()
        .find(|(v, _, _)| v == current)
        .map(|(_, label, _)| label.clone())
        // Une valeur qui n'est dans aucune option n'invente pas de
        // libellé : elle laisse la case vide plutôt que d'écrire un
        // nom qui n'est pas le sien.
        .unwrap_or_default();
    let response = menu_head(ui, id, width, &shown);
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(id));
    }
    let mut changed = false;
    egui::popup::popup_below_widget(
        ui,
        id,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(width);
            for (value, label, hint) in options {
                let row = list_row(ui, egui::RichText::new(label), value == current);
                // Une explication vide ne pose pas d'infobulle : une
                // boîte qui s'ouvre pour ne rien dire est pire que pas
                // de boîte du tout.
                let row = if hint.is_empty() {
                    row
                } else {
                    row.on_hover_text(hint)
                };
                if row.clicked() && value != current {
                    *current = value.clone();
                    changed = true;
                }
            }
            popup_relief(ui);
        },
    );
    let mut response = response;
    if changed {
        response.mark_changed();
    }
    response
}

/// The same scale over a range that **does not start at zero**.
///
/// A quantity at the counter runs from nothing upwards, which is why
/// [`scale`] takes a ceiling alone. A setting often does not: the text
/// scale runs from 0,8 to 1,6, and a scale from zero would spend half
/// its travel on sizes the interface refuses.
///
/// Written the day the only `egui::Slider` left in the application was
/// replaced. That slider painted its rail with
/// `widgets.inactive.bg_fill`, and [`apply`] sets that to [`bg`] for
/// every widget state — so on the ten palettes alike the rail was the
/// panel's own ground and the thumb floated on nothing. Measured on a
/// capture of Options › Interface: along the middle of the control,
/// two hundred and thirty pixels of background and two pixels of thumb
/// edge. Nothing said where 0,8 was, where 1,6 was, nor where one
/// stood — on the very control someone who cannot read the screen goes
/// to first.
pub fn scale_range(
    ui: &mut egui::Ui,
    width: f32,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
) -> (egui::Response, f64) {
    let h = ui.spacing().interact_size.y.max(18.0);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(width.max(24.0), h), egui::Sense::click_and_drag());
    // Une étendue nulle ou à l'envers donnerait une division par zéro
    // plus bas, et un pouce collé à gauche : la borne haute cède.
    let (min, max) = if max > min {
        (min, max)
    } else {
        (min, min + 1.0)
    };
    // The groove: a shallow sunken channel across the middle, the full
    // height being the thumb's travel.
    let groove = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 6.0));
    ui.painter().rect_filled(groove, 0.0, crate::trough());
    bevel(ui.painter(), groove, false);
    let mut out = value.clamp(min, max);
    if let Some(pos) = resp.interact_pointer_pos() {
        // The thumb has width, so the travel is shorter than the trough:
        // mapping the pointer to the full width would make the last
        // value unreachable and the first one sticky.
        let thumb_w = (h * 0.5).max(10.0);
        let travel = (rect.width() - thumb_w).max(1.0);
        let t = ((pos.x - rect.left() - thumb_w / 2.0) / travel).clamp(0.0, 1.0) as f64;
        let raw = min + t * (max - min);
        out = if step > 0.0 {
            // Les crans se comptent **depuis la borne basse**, sinon un
            // pas de 0,05 sur 0,8–1,6 tomberait sur 0,80 par hasard et
            // sur 1,575 partout ailleurs.
            min + ((raw - min) / step).round() * step
        } else {
            raw
        };
        out = out.clamp(min, max);
    }
    let thumb_w = (h * 0.5).max(10.0);
    let travel = (rect.width() - thumb_w).max(1.0);
    let t = (((out - min) / (max - min)) as f32).clamp(0.0, 1.0);
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
    use super::*;

    /// **Une taille de texte passe par `pt`, y compris ici.**
    ///
    /// `app.rs` a son test qui refuse la prochaine taille écrite en
    /// pixels — et il ne lit **que** `app.rs`. Ce coin-là n'était donc
    /// tenu par rien, et il en portait une : la légende d'une barre
    /// horizontale était bornée entre dix et treize pixels, la rangée
    /// qui la porte entre quatorze et trente. À `[ui] text_scale = 1,6`
    /// tout l'écran grandissait de moitié et ces légendes restaient
    /// où elles étaient — le plus petit texte de l'écran, seul à ne pas
    /// bouger, c'est-à-dire justement celui que veut agrandir qui
    /// agrandit la police.
    ///
    /// Le test d'`app.rs` ne l'aurait pas vue : il refuse un chiffre
    /// **collé** à l'appel, et celui-ci s'écrivait
    /// `FontId::proportional((row_h * 0.46).clamp(10.0, 13.0))`, qui
    /// commence par une parenthèse. Celui-ci lit l'argument entier : un
    /// nombre écrit dedans, sans `pt(` sur la ligne, est refusé — où
    /// qu'il soit dans l'expression.
    ///
    /// Un filet, non une preuve : une variable peut toujours porter une
    /// constante. Vérifié en remettant les bornes de `hbar_metrics`.
    /// **Ce que ces fonctions annoncent est ce que le dessin occupe.**
    ///
    /// Elles sont le modèle sur lequel toute bande carrée de cette
    /// application réserve sa place, et trois d'entre elles mentaient
    /// — chacune de quelques pixels, toutes dans le même sens, celui
    /// qui coupe. Mises bout à bout sur le bandeau du dossier, la vue
    /// la plus regardée de l'application, elles faisaient une rangée de
    /// boutons entière : « Étiquettes… » et « Écraser ? » sortaient
    /// tranchées par le bas, sur tous les dossiers et à toutes les
    /// tailles de texte.
    ///
    /// * **`button_height`** annonçait 37,68 pour un bouton qui en
    ///   occupe 38 : egui arrondit ce qu'il alloue à la grille de
    ///   pixels, et une pile de dix rangées perdait trois pixels.
    /// * **`field_sized`** relève toute case sous `field_floor` — son
    ///   texte, la marge propre du `TextEdit`, le creux —, donc une
    ///   case demandée à `interact_size.y` en occupe trois de plus que
    ///   ce qu'on lui a demandé. C'est `field_floor` qu'il faut
    ///   interroger, et il est public pour cela.
    /// * **`panel_chrome`** est ce que `panel` prend sur le rectangle
    ///   qu'on lui donne. Seize pixels, que le bandeau du dossier ne
    ///   comptait pas : il rendait une hauteur de *contenu* là où
    ///   l'appelant en fait un rectangle de *panneau*.
    ///
    /// Mesuré dans le dessin, aux trois échelles qui comptent, et dans
    /// le sens qui protège : annoncer trop remet du gris, annoncer trop
    /// peu tranche.
    #[test]
    fn what_these_heights_announce_is_what_the_drawing_takes() {
        for scale in [1.0_f32, 1.25, 1.6] {
            let ctx = egui::Context::default();
            apply_scale(&ctx, scale, Density::Comfortable);
            let seen: std::cell::RefCell<Vec<(String, f32, f32)>> =
                std::cell::RefCell::new(Vec::new());
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let drawn = button(ui, "Étiquettes…").rect.height();
                    seen.borrow_mut()
                        .push(("button".to_owned(), button_height(ui), drawn));

                    // Une case demandée plus courte que son plancher :
                    // c'est ce que font les bandes trop serrées pour
                    // une rangée entière, et c'est là que l'écart se
                    // paie.
                    let mut buf = String::new();
                    let drawn = ui
                        .horizontal(|ui| {
                            field_sized(
                                ui,
                                Vec2::new(140.0, ui.spacing().interact_size.y),
                                egui::TextEdit::singleline(&mut buf),
                            );
                        })
                        .response
                        .rect
                        .height();
                    seen.borrow_mut()
                        .push(("field".to_owned(), field_floor(ui), drawn));

                    // Et ce qu'un panneau prend sur le rectangle qu'on
                    // lui donne.
                    let rect =
                        egui::Rect::from_min_size(ui.max_rect().min, Vec2::new(400.0, 300.0));
                    let inner = panel(ui, rect, None, |ui| ui.max_rect().height());
                    seen.borrow_mut().push((
                        "panel".to_owned(),
                        panel_chrome(ui, false),
                        rect.height() - inner,
                    ));
                });
            });
            for (what, announced, drawn) in seen.into_inner() {
                assert!(
                    announced >= drawn - 0.01,
                    "à l'échelle {scale}, {what} annonce {announced:.2} et occupe {drawn:.2} : \
                     annoncer moins que ce qu'on dessine, c'est trancher"
                );
                assert!(
                    announced <= drawn + 1.0,
                    "à l'échelle {scale}, {what} annonce {announced:.2} pour {drawn:.2} : \
                     réserver du vide se lit comme une intention"
                );
            }
        }
    }

    #[test]
    fn no_text_size_in_this_crate_is_written_in_pixels() {
        // Assemblés, sinon le test se trouve lui-même.
        let calls = [
            concat!("FontId::propor", "tional("),
            concat!("FontId::mono", "space("),
        ];
        let files = [
            ("lib.rs", include_str!("lib.rs")),
            ("chart.rs", include_str!("chart.rs")),
            ("layout.rs", include_str!("layout.rs")),
        ];
        let mut offenders: Vec<String> = Vec::new();
        for (name, source) in files {
            for (i, line) in source.lines().enumerate() {
                // Un commentaire n'est pas du code — celui de ce test
                // cite l'appel fautif, et se trouvait lui-même.
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for call in calls {
                    let Some((_, tail)) = line.split_once(call) else {
                        continue;
                    };
                    // L'argument, jusqu'à la parenthèse qui ferme
                    // l'appel : le compte des parenthèses, parce que
                    // l'expression en contient elle-même.
                    let mut depth = 1usize;
                    let arg: String = tail
                        .chars()
                        .take_while(|c| {
                            match c {
                                '(' => depth += 1,
                                ')' => depth -= 1,
                                _ => {}
                            }
                            depth > 0
                        })
                        .collect();
                    let numbered = arg.chars().any(|c| c.is_ascii_digit());
                    if numbered && !line.contains("pt(") {
                        offenders.push(format!("{name}:{} : {}", i + 1, line.trim()));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "une taille de texte passe par motif::pt, elle ne s'écrit pas \
             en pixels :\n{}",
            offenders.join("\n")
        );
    }

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
    /// **Une invite garde sa couleur sous l'override du contexte**, et
    /// se tient plus près du champ que l'encre d'une valeur.
    ///
    /// Les deux moitiés comptent. La première est le mécanisme :
    /// [`apply`] pose un `override_text_color`, egui le lit avant la
    /// couleur qu'il destine à l'invite, et c'est *pour cela* que les
    /// cent vingt-huit invites sortaient dans l'encre pleine. Une
    /// couleur explicite passe devant — mais c'est une affirmation sur
    /// egui, donc elle se vérifie plutôt qu'elle ne se suppose : le
    /// test monte un style dont l'override est rouge et regarde ce que
    /// la section porte.
    ///
    /// La seconde est la raison d'être : l'invite doit être **plus près
    /// du fond du champ que l'encre**, sur les dix palettes, sinon on
    /// a changé la teinte sans lever la confusion. C'est une distance
    /// et jamais une direction — les deux palettes de nuit écrivent
    /// clair sur sombre.
    /// **La moitié tranquille part entière ou ne part pas.**
    ///
    /// Les deux moitiés partageaient une galée et s'élidaient ensemble.
    /// Sur un volet étroit, « Ebixa » — un nom complet — se lisait
    /// « Ebixa … », et ce point de suspension fait croire qu'il manque
    /// quelque chose au nom. Pire : la moitié tranquille prenait la
    /// place que le nom n'avait plus, si bien que « Effentora » et
    /// « Efferalgan » sortaient coupés alors qu'ils tiennent entiers
    /// sans elle. Mesuré sur la liste des médicaments à 1024x700 en
    /// texte 1,6 : cinq rangées, cinq noms abîmés, zéro après.
    /// **Ce qu'on remplit descend, ce qu'on presse monte.**
    ///
    /// C'est le seul signe qui distingue à l'œil un champ d'un bouton
    /// dans ce chrome, et les champs ne l'avaient pas : egui les
    /// dessinait de la couleur du creux, cernés d'un trait d'un pixel.
    /// La couleur disait « ici on écrit », le relief ne disait rien.
    ///
    /// Dessiné pour de vrai et relu dans la liste des formes : le biseau
    /// creusé pose sa teinte sombre **en haut à gauche**, le levé en
    /// bas à droite. C'est cela qu'on vérifie, et non qu'un trait
    /// existe.
    #[test]
    fn what_is_filled_in_is_sunk_and_what_is_pressed_rises() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("gris");
        let ctx = egui::Context::default();
        super::apply(&ctx);
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);

        // La couleur d'un segment, par son point de départ.
        let corner_ink = |shapes: &[egui::epaint::ClippedShape], top_left: bool| {
            let mut found = None;
            for s in shapes {
                if let egui::Shape::LineSegment { points, stroke } = &s.shape {
                    let going_up = points[0].y > points[1].y;
                    let going_right = points[1].x > points[0].x;
                    if (top_left && (going_up || going_right))
                        || (!top_left && !going_up && !going_right)
                    {
                        found.get_or_insert(stroke.color.clone());
                    }
                }
            }
            found
        };

        let mut text = String::from("Dupont");
        let field = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                super::field(ui, 160.0, egui::TextEdit::singleline(&mut text));
            });
        });
        let sunk = corner_ink(&field.shapes, true).expect("un biseau");
        assert_eq!(
            sunk,
            egui::epaint::ColorMode::Solid(super::bg_dark()),
            "le champ est creusé"
        );

        let button = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                super::button(ui, "Enregistrer");
            });
        });
        let raised = corner_ink(&button.shapes, true).expect("un biseau");
        assert_eq!(
            raised,
            egui::epaint::ColorMode::Solid(super::bg_light()),
            "le bouton est levé"
        );
        assert_ne!(sunk, raised, "les deux reliefs ne se ressemblent pas");
    }

    /// **Une case n'est jamais plus courte que son texte.**
    ///
    /// Sa hauteur venait d'`interact_size.y` — ce qu'egui demande pour
    /// un widget nu : vingt pixels à l'échelle 1, pour un texte qui en
    /// occupe dix-sept et un `TextEdit` qui ajoute quatre de marge.
    /// Mesuré sur une capture du dossier, le jambage du « p » de
    /// « Dupont » tombait *sur* le biseau du bas. Trois bandes trop
    /// serrées pour une rangée entière demandent encore cette
    /// hauteur-là : le plancher les relève sans qu'elles aient à le
    /// savoir.
    #[test]
    fn a_field_is_never_shorter_than_the_text_it_holds() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("motif");
        for scale in [1.0_f32, 1.25, 1.6] {
            let ctx = egui::Context::default();
            super::apply(&ctx);
            super::apply_scale(&ctx, scale, super::Density::Comfortable);
            let mut text = String::from("Dupont");
            let (mut asked, mut drawn, mut row, mut floor) = (0.0, 0.0, 0.0, 0.0);
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    asked = ui.spacing().interact_size.y;
                    row = super::button_height(ui);
                    floor = super::field_floor(ui);
                    drawn = super::field_sized(
                        ui,
                        egui::Vec2::new(160.0, asked),
                        egui::TextEdit::singleline(&mut text),
                    )
                    .rect
                    .height();
                });
            });
            assert!(
                drawn >= floor - 0.5,
                "à {scale}, une case de {drawn} px pour un plancher de {floor}"
            );
            // Et le plancher est bien **sous** la rangée ordinaire : le
            // jour où il la dépasse, c'est la rangée qu'il faut revoir,
            // pas le plancher.
            assert!(
                floor <= row + 0.5,
                "à {scale}, le plancher dépasse la rangée"
            );
            // Le défaut qu'il corrige existait : ce qu'egui demande pour
            // un widget nu est plus court que ce qu'un texte occupe.
            assert!(asked < floor, "à {scale}, interact_size suffisait déjà");
        }
    }

    /// **Une zone de plusieurs lignes est creusée comme une case d'une
    /// ligne, et un filet sépare sans prendre plus qu'une gouttière.**
    ///
    /// Les treize zones de texte de l'application étaient restées
    /// peintes par egui, c'est-à-dire plates, au milieu de cent
    /// soixante-dix cases creusées : une case plate entourée de cases
    /// creusées ne se lit pas comme « il en reste une » mais comme un
    /// défaut de rendu, puisqu'elle est devenue la seule de l'écran.
    #[test]
    fn an_area_is_sunken_like_a_field_and_a_separator_costs_a_gutter() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("motif");
        let ctx = egui::Context::default();
        super::apply(&ctx);
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);

        let mut text = String::from("Consignes du jour\nRappels");
        let mut rect = egui::Rect::NOTHING;
        let out = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = super::area(
                    ui,
                    egui::vec2(240.0, 80.0),
                    egui::TextEdit::multiline(&mut text),
                )
                .rect;
            });
        });
        assert!(
            (rect.width() - 240.0).abs() < 0.5 && (rect.height() - 80.0).abs() < 0.5,
            "la zone rend le cadre qu'elle a dessiné : {rect:?}"
        );
        // Le fond du creux est peint, et de la couleur du creux.
        assert!(
            out.shapes.iter().any(|s| matches!(
                &s.shape,
                egui::Shape::Rect(r) if r.fill == super::trough()
            )),
            "la zone n'a pas de fond creusé"
        );

        // Le filet : il prend une gouttière de haut, filet compris —
        // une bande qui mesure ses rangées ne lui ajoute rien d'autre.
        let mut before = 0.0;
        let mut after = 0.0;
        let mut gap = 0.0;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                gap = ui.spacing().item_spacing.y;
                before = ui.cursor().top();
                super::separator(ui);
                after = ui.cursor().top();
            });
        });
        assert!(
            (after - before - gap.max(6.0) - gap).abs() < 0.5,
            "un filet coûte sa rangée et la gouttière d'après : {}",
            after - before
        );
    }

    /// **Une case vide monte, une case cochée descend.**
    ///
    /// C'est la règle de cette maison appliquée au dernier objet qui y
    /// échappait. Celle d'egui est un carré plat cerné d'un trait, et
    /// surtout : vide et cochée se ressemblent de loin, la seule
    /// différence étant une coche fine de la couleur du cadre. Le
    /// relief se voit avant qu'on ait lu — c'est ce qu'on vérifie ici,
    /// sur le biseau et non sur l'existence d'un trait.
    #[test]
    fn a_checkbox_rises_when_it_is_empty_and_sinks_when_it_is_ticked() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("motif");
        let ctx = egui::Context::default();
        super::apply(&ctx);
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);

        // La teinte du segment qui part du coin haut-gauche : claire
        // quand l'objet est levé, sombre quand il est creusé.
        let top_left_ink = |shapes: &[egui::epaint::ClippedShape]| {
            let mut found = None;
            for s in shapes {
                if let egui::Shape::LineSegment { points, stroke } = &s.shape {
                    let going_up = points[0].y > points[1].y;
                    let going_right = points[1].x > points[0].x;
                    if going_up || going_right {
                        found.get_or_insert(stroke.color.clone());
                    }
                }
            }
            found
        };

        let draw = |on: bool| {
            let mut value = on;
            ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    super::checkbox(ui, &mut value, "Documentation ouverte au démarrage");
                });
            })
            .shapes
        };
        let empty = top_left_ink(&draw(false)).expect("un biseau");
        let ticked = top_left_ink(&draw(true)).expect("un biseau");
        assert_eq!(
            empty,
            egui::epaint::ColorMode::Solid(super::bg_light()),
            "une case vide est levée"
        );
        assert_eq!(
            ticked,
            egui::epaint::ColorMode::Solid(super::bg_dark()),
            "une case cochée est creusée"
        );
        assert_ne!(empty, ticked, "les deux états ne se ressemblent pas");

        // Et le clic bascule : c'est la réponse qui le dit, marquée
        // modifiée, pour que l'appelant sache qu'il a quelque chose à
        // enregistrer.
        let mut value = false;
        let mut changed = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = Some(super::checkbox(ui, &mut value, "Compact").changed());
            });
        });
        assert_eq!(changed, Some(false), "rien n'a été cliqué");
        assert!(!value);

        // Le losange d'un bouton exclusif se dessine aussi, et il n'est
        // pas un carré : quatre segments et un polygone, pas un
        // rectangle.
        let out = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                super::radio(ui, true, "Confortable");
            });
        });
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Path(_))),
            "un losange, et non un carré"
        );
    }

    /// **Un champ rend le cadre qu'il a dessiné, et prend la place
    /// qu'il a demandée.**
    ///
    /// Les deux se sont perdues ensemble, et pour la même raison : le
    /// champ était posé par `ui.put`, qui rend la réponse du `TextEdit`
    /// — le rectangle du *texte*, six pixels plus étroit que la case —
    /// et qui **alloue une seconde fois**, en retrait, ce qui faisait
    /// reculer le curseur de la rangée. Quatre vues dessinaient leur
    /// propre biseau sur le rectangle rendu, deux pixels à l'intérieur
    /// du vrai : la case sortait à double bord. Et le widget suivant
    /// mordait de six pixels sur la marge de celui-ci.
    ///
    /// Les deux se vérifient d'un coup en posant deux champs dans une
    /// rangée : le premier doit rendre sa taille entière, le second doit
    /// commencer exactement une gouttière plus loin.
    #[test]
    fn a_field_hands_back_the_frame_it_drew_and_takes_the_room_it_asked_for() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("motif");
        let ctx = egui::Context::default();
        super::apply(&ctx);
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);

        let (mut a, mut b) = (String::from("Dupont"), String::from("Jean"));
        let mut rects = Vec::new();
        let mut gap = 0.0;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    gap = ui.spacing().item_spacing.x;
                    rects.push(super::field(ui, 160.0, egui::TextEdit::singleline(&mut a)).rect);
                    rects.push(super::field(ui, 120.0, egui::TextEdit::singleline(&mut b)).rect);
                });
            });
        });
        assert_eq!(rects.len(), 2);
        assert!(
            (rects[0].width() - 160.0).abs() < 0.5,
            "le rectangle rendu n'est pas celui du cadre : {:?}",
            rects[0].width()
        );
        assert!(
            (rects[1].width() - 120.0).abs() < 0.5,
            "le second non plus : {:?}",
            rects[1].width()
        );
        assert!(
            (rects[1].left() - rects[0].right() - gap).abs() < 0.5,
            "le champ n'a pas pris la place qu'il a demandée : {} puis {}",
            rects[0].right(),
            rects[1].left()
        );
        // Et la hauteur vient du style, jamais d'une constante : c'est
        // ce qui la fait suivre `[ui] text_scale`.
        let ctx2 = egui::Context::default();
        super::apply(&ctx2);
        super::apply_scale(&ctx2, 1.6, super::Density::Comfortable);
        let mut tall = egui::Rect::NOTHING;
        let _ = ctx2.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tall = super::field(ui, 160.0, egui::TextEdit::singleline(&mut a)).rect;
            });
        });
        assert!(
            tall.height() > rects[0].height() + 4.0,
            "le champ ne grandit pas avec le texte : {} puis {}",
            rects[0].height(),
            tall.height()
        );
    }

    /// Un menu d'options ne se peint pas dans `weak_bg_fill`.
    ///
    /// C'est le piège que ce fichier nomme déjà pour la glissière :
    /// `apply` met ce champ au fond du panneau pour tous les états, si
    /// bien que tout ce qu'egui en tire sort plat. Le `ComboBox` en
    /// sortait : de la couleur du panneau, cerné d'un trait, à côté de
    /// boutons biseautés.
    ///
    /// Celui-ci est dessiné ici, et il monte comme un bouton — plus une
    /// marque, qu'on vérifie aussi : un menu sans marque est un bouton
    /// qui ment.
    #[test]
    fn a_select_rises_like_a_button_and_shows_its_mark() {
        use eframe::egui;
        let _guard = theme_lock();
        super::set_theme("gris");
        let ctx = egui::Context::default();
        super::apply(&ctx);
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);

        let options = vec![
            (1_u8, "Docteur".to_owned()),
            (2, "Sage-femme".to_owned()),
            (3, "Chirurgien-dentiste".to_owned()),
        ];
        let mut picked = 2_u8;
        let out = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                super::select(ui, "qualite", 180.0, &mut picked, &options);
            });
        });
        // Le relief : le biseau levé pose sa teinte claire en haut à
        // gauche, comme un bouton.
        let lit = out.shapes.iter().any(|s| {
            matches!(&s.shape, egui::Shape::LineSegment { stroke, .. }
                if stroke.color == egui::epaint::ColorMode::Solid(super::bg_light()))
        });
        assert!(lit, "le menu ne monte pas");
        // La marque : un triangle, c'est-à-dire un polygone de trois
        // points. Sans elle, rien ne dit que cela s'ouvre.
        let marked = out
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Path(p) if p.points.len() == 3));
        assert!(marked, "le menu n'a pas de marque");
        // Et il montre le libellé choisi, jamais la valeur.
        let drawn: String = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        assert!(drawn.contains("Sage-femme"), "« {drawn} »");
        assert!(
            !drawn.contains('2'),
            "la valeur ne s'écrit pas : « {drawn} »"
        );
    }

    #[test]
    fn the_quiet_half_of_a_row_leaves_whole_or_does_not_leave() {
        use eframe::egui;
        let _guard = theme_lock();
        let ctx = egui::Context::default();
        super::apply_scale(&ctx, 1.0, super::Density::Comfortable);
        let seen = std::cell::RefCell::new((String::new(), String::new()));
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Étroit : le couple ne tient pas, le nom seul si.
                let narrow = ui
                    .scope(|ui| {
                        ui.set_max_width(90.0);
                        super::pair_galley(ui, "Ebixa", "chlorhydratedemémantine", false, 0.0)
                    })
                    .inner;
                // Large : les deux tiennent, et les deux sont là.
                let wide = ui
                    .scope(|ui| {
                        ui.set_max_width(600.0);
                        super::pair_galley(ui, "Ebixa", "chlorhydratedemémantine", false, 0.0)
                    })
                    .inner;
                *seen.borrow_mut() = (narrow.text().to_owned(), wide.text().to_owned());
            });
        });
        let (narrow, wide) = seen.into_inner();
        assert_eq!(
            narrow, "Ebixa",
            "le nom seul, entier, sans point de suspension"
        );
        assert!(
            wide.contains("Ebixa") && wide.contains("chlorhydratedemémantine"),
            "au large, les deux moitiés : {wide}"
        );
    }

    #[test]
    fn a_hint_keeps_its_colour_and_stays_nearer_the_field_than_an_ink() {
        use eframe::egui::{self, Color32};
        // La palette est globale, et les tests tournent en parallèle :
        // sans ce verrou, la couleur posée par `hint` et celle relue
        // pour la comparer peuvent venir de deux palettes différentes.
        // Écrit sans, le test passait — et a échoué au troisième
        // lancement, ce qui est la pire façon de l'apprendre.
        let _guard = theme_lock();
        let expected = super::text_faint();
        let mut style = egui::Style::default();
        style.visuals.override_text_color = Some(Color32::RED);
        let job = egui::WidgetText::from(super::hint("9h30")).into_layout_job(
            &style,
            egui::FontSelection::Default,
            egui::Align::Center,
        );
        assert_eq!(
            job.sections[0].format.color, expected,
            "l'override du contexte a mangé la couleur de l'invite"
        );

        let lum = super::luminance;
        for t in super::THEMES {
            let p = t.palette;
            let d = |a: Color32, b: Color32| (lum(a) - lum(b)).abs();
            assert!(
                d(p.text_faint, p.trough) < d(p.text, p.trough),
                "{} : l'invite est aussi loin du champ que l'encre",
                t.key
            );
        }
    }

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
    /// **Un trait d'union est une occasion de couper ; une longue suite
    /// de lettres n'en est pas une.**
    ///
    /// egui coupe à l'union — mesuré :
    /// « lidocaine-bicarbonate-nystatine » devient
    /// « lidocaine-bicarbonate- » puis « nystatine », ce qui se lit.
    /// Compté comme un mot de trente et un caractères, il faisait
    /// retomber la ligne entière sur l'ellipse, et quatre préparations
    /// du codex s'affichaient toutes « Bain de bouche … » dans une liste
    /// faite pour les distinguer.
    ///
    /// Et l'inverse ne bouge pas : « Benzodiazépines » n'a pas d'union,
    /// egui le couperait donc n'importe où — « Benzodiazép / ines » se
    /// lit plus mal que l'ellipse, et c'est l'ellipse qu'on garde.
    #[test]
    fn a_hyphen_is_a_place_to_break_and_a_long_word_is_not() {
        use eframe::egui;
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let font = egui::TextStyle::Body.resolve(ui.style());
                let ch = ui.fonts(|f| f.glyph_width(&font, '0'));
                // Une colonne qui tient « bicarbonate » — onze lettres,
                // le plus long segment entre deux unions — et pas
                // « Benzodiazepines », qui en fait quinze. C'est entre
                // ces deux-là que la question se pose.
                let room = ch * 13.0;
                assert_eq!(
                    super::label_rows(
                        ui,
                        "Bain de bouche lidocaine-bicarbonate-nystatine (formule type)",
                        &font,
                        room
                    ),
                    2,
                    "les segments se mesurent entre les traits d'union"
                );
                assert_eq!(
                    super::label_rows(ui, "Benzodiazepines et apparentes", &font, room),
                    1,
                    "sans union, une longue suite de lettres garde l'ellipse"
                );
            });
        });
    }

    /// **Un segment se mesure, il ne se compte pas en caractères.**
    ///
    /// Le gabarit était « autant de « 0 » que de lettres ». Un chiffre
    /// est large et une minuscule ne l'est pas : dans le volet des
    /// familles thérapeutiques, à `[ui] text_scale = 1,6`,
    /// « Cardiologie et vaisseaux » se lisait « Cardiolo… » alors que la
    /// colonne avait la place de ses deux lignes. Un nom élidé pour un
    /// gabarit.
    #[test]
    fn a_segment_is_measured_and_not_counted_in_characters() {
        use eframe::egui;
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let font = egui::TextStyle::Body.resolve(ui.style());
                let ch = ui.fonts(|f| f.glyph_width(&font, '0'));
                let word = "Cardiologie";
                let wide = ui.fonts(|f| {
                    f.layout_no_wrap(word.to_owned(), font.clone(), super::text())
                        .size()
                        .x
                });
                // La colonne tient le mot tel qu'il se dessine, et pas
                // onze « 0 » : c'est exactement l'écart entre les deux
                // mesures, et c'est là que le nom se perdait.
                let room = wide + 1.0;
                assert!(
                    room < ch * word.chars().count() as f32,
                    "le gabarit en « 0 » sur-compte bien « {word} » : \
                     {room} px mesurés contre {} comptés",
                    ch * word.chars().count() as f32
                );
                assert_eq!(
                    super::label_rows(ui, "Cardiologie et vaisseaux", &font, room),
                    2,
                    "une colonne qui tient le mot dessiné le garde entier"
                );
            });
        });
    }
}
