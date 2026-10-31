//! The few shapes every v2 screen is made of, on top of `motif`.
//!
//! The rule of v2: a screen is a title, one row of actions, and a body.
//! No tooltips that explain the screen — the label says it. Sizes come
//! from the text (characters, rows), never from pixels.

use crate::analyse::Gravity;
use crate::t::t;
use eframe::egui;
use motif::Pict;

/// Width of `n` body digits: what fields and columns are sized in.
pub fn chars_wide(ui: &egui::Ui, n: f32) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let ch = ui.fonts(|f| f.glyph_width(&font, '0'));
    (ch * n).max(motif::field_floor(ui))
}

/// The height of one body line.
pub fn line(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Body)
}

/// A screen's title, and an optional short line under it.
pub fn title(ui: &mut egui::Ui, text: &str, sub: Option<&str>) {
    ui.label(
        egui::RichText::new(text)
            .size(motif::pt(ui, 20.0))
            .strong()
            .color(motif::text()),
    );
    if let Some(sub) = sub.filter(|s| !s.is_empty()) {
        ui.label(motif::hint(sub));
    }
    ui.add_space(line(ui) * 0.4);
}

/// A heading inside a screen.
pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(line(ui) * 0.3);
    motif::section(ui, text);
}

/// The quiet sentence that stands where a list would be.
pub fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(line(ui) * 0.3);
    ui.label(motif::hint(text));
}

/// A one-line text field `n` characters wide, with a hint inside.
pub fn field(ui: &mut egui::Ui, value: &mut String, n: f32, hint: &str) -> egui::Response {
    let w = chars_wide(ui, n).min(ui.available_width());
    motif::field(
        ui,
        w,
        egui::TextEdit::singleline(value).hint_text(motif::hint(hint)),
    )
}

/// The width a Motif button with `text` takes, spacing after it included.
pub fn button_width(ui: &egui::Ui, text: &str) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_owned(), font, motif::text())
            .size()
            .x
    }) + (ui.spacing().button_padding.x + 4.0) * 2.0
        + ui.spacing().item_spacing.x
}

/// Text in a cell of fixed width, left-aligned, cut short with an
/// ellipsis when too long; clickable.
pub fn cell_text(ui: &mut egui::Ui, text: egui::RichText, width: f32) -> egui::Response {
    let h = motif::button_height(ui);
    ui.allocate_ui_with_layout(
        egui::vec2(width, h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(width, h));
            ui.add(
                egui::Label::new(text)
                    .truncate()
                    .sense(egui::Sense::click()),
            )
        },
    )
    .inner
}

/// A field that takes the row, and a button after it: the note boxes.
/// Returns whether to submit (button or Enter with text in the field).
pub fn field_and_button(ui: &mut egui::Ui, value: &mut String, hint: &str, button: &str) -> bool {
    ui.horizontal(|ui| {
        let font = egui::TextStyle::Button.resolve(ui.style());
        let bw = ui.fonts(|f| {
            f.layout_no_wrap(button.to_owned(), font, motif::text())
                .size()
                .x
        }) + (ui.spacing().button_padding.x + 4.0) * 2.0
            + ui.spacing().item_spacing.x * 2.0;
        let w = (ui.available_width() - bw).max(chars_wide(ui, 12.0));
        let r = motif::field(
            ui,
            w,
            egui::TextEdit::singleline(value).hint_text(motif::hint(hint)),
        );
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let clicked = motif::button(ui, button).clicked();
        if enter {
            r.request_focus();
        }
        (clicked || enter) && !value.trim().is_empty()
    })
    .inner
}

/// A label and its field on one row, the label in a column of `label_n`
/// characters so a form's fields line up.
pub fn form_row(
    ui: &mut egui::Ui,
    label: &str,
    label_n: f32,
    value: &mut String,
    n: f32,
    hint: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        let w = chars_wide(ui, label_n);
        cell_text(ui, egui::RichText::new(label).color(motif::text_dim()), w);
        field(ui, value, n, hint)
    })
    .inner
}

/// The ink a gravity is painted in. Colour never stands alone: every
/// chip also carries its pictogram.
pub fn gravity_ink(g: Gravity) -> egui::Color32 {
    match g {
        Gravity::Stop => motif::alert(),
        Gravity::Adapt => motif::warn(),
        Gravity::Watch => motif::accent(),
        Gravity::Check | Gravity::Fine => motif::text_dim(),
    }
}

pub fn gravity_pict(g: Gravity) -> Pict {
    match g {
        Gravity::Stop => Pict::Stop,
        Gravity::Adapt => Pict::Warn,
        Gravity::Watch => Pict::Warn,
        Gravity::Check => Pict::Pending,
        Gravity::Fine => Pict::Check,
    }
}

pub fn gravity_label(g: Gravity) -> &'static str {
    match g {
        Gravity::Stop => t("gravite.stop"),
        Gravity::Adapt => t("gravite.adapt"),
        Gravity::Watch => t("gravite.watch"),
        Gravity::Check => t("gravite.check"),
        Gravity::Fine => t("gravite.fine"),
    }
}

/// A small filled chip: the gravity, its pictogram, and a count when
/// one is given.
pub fn gravity_chip(ui: &mut egui::Ui, g: Gravity, count: Option<usize>) -> egui::Response {
    let text = match count {
        Some(n) => format!("{} {n}", gravity_label(g)),
        None => gravity_label(g).to_owned(),
    };
    motif::badge(ui, &text, Some(gravity_pict(g)), gravity_ink(g), false)
}

/// A pictogram the size of a body line, in `ink`.
pub fn pict(ui: &mut egui::Ui, p: Pict, ink: egui::Color32) {
    let s = line(ui);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(s, s), egui::Sense::hover());
    motif::pictogram(ui.painter(), rect.shrink(s * 0.12), p, ink);
}

/// A date as the counter reads it: `JJ/MM/AAAA`, or a dash.
pub fn date(iso: &str) -> String {
    if iso.trim().is_empty() {
        t("app.dash").to_owned()
    } else {
        bpm_caddy::db::format_french_date(iso)
    }
}

/// The day of the week and the date, « jeudi 01/10/2026 ».
pub fn day_long(iso: &str) -> String {
    let names = [
        t("jours.lundi"),
        t("jours.mardi"),
        t("jours.mercredi"),
        t("jours.jeudi"),
        t("jours.vendredi"),
        t("jours.samedi"),
        t("jours.dimanche"),
    ];
    match bpm_caddy::date::weekday(iso) {
        Some(w) if (1..=7).contains(&w) => format!("{} {}", names[w as usize - 1], date(iso)),
        _ => date(iso),
    }
}

/// A key–value line of a card: the key dimmed, the value wrapped.
pub fn fact(ui: &mut egui::Ui, key: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(key).color(motif::text_dim()));
        ui.label(value);
    });
}

/// The one cell being typed in, on a screen that shows many editable
/// values read from the base. Only the focused cell has a buffer: every
/// other cell shows the base as it is now, so a write from another post
/// shows at once, and never overwrites what someone is typing.
#[derive(Clone, Debug, Default)]
pub struct Typing {
    key: Option<egui::Id>,
    text: String,
}

impl Typing {
    /// Draw an editable cell showing `shown`. Returns the new text when
    /// the person leaves the cell (Enter, Tab, click elsewhere) having
    /// changed it — the caller writes it compare-and-set against `shown`.
    pub fn cell(
        &mut self,
        ui: &mut egui::Ui,
        key: impl std::hash::Hash,
        shown: &str,
        width: f32,
        hint: &str,
    ) -> Option<String> {
        let id = egui::Id::new(("cell", key));
        if self.key == Some(id) {
            let r = motif::field(
                ui,
                width,
                egui::TextEdit::singleline(&mut self.text)
                    .id(id)
                    .hint_text(motif::hint(hint)),
            );
            if r.lost_focus() || !r.has_focus() {
                self.key = None;
                let text = std::mem::take(&mut self.text);
                if text.trim() != shown.trim() {
                    return Some(text.trim().to_owned());
                }
            }
            None
        } else {
            let mut tmp = shown.to_owned();
            let r = motif::field(
                ui,
                width,
                egui::TextEdit::singleline(&mut tmp)
                    .id(id)
                    .hint_text(motif::hint(hint)),
            );
            if r.has_focus() || r.changed() {
                self.key = Some(id);
                self.text = tmp;
            }
            None
        }
    }

    pub fn active(&self) -> bool {
        self.key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gravity_has_words_and_a_mark() {
        for g in Gravity::ALL {
            assert!(!gravity_label(g).contains('.'), "{g:?}");
            let _ = gravity_pict(g);
        }
    }

    #[test]
    fn day_long_names_the_weekday() {
        // 2026-10-01 is a Thursday.
        let s = day_long("2026-10-01");
        assert!(s.starts_with(t("jours.jeudi")), "{s}");
        assert!(s.ends_with("01/10/2026"));
        assert_eq!(date(""), t("app.dash"));
    }
}
