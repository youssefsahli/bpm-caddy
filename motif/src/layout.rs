//! Layout scaffolding: the page grid, docked panes, notebook tabs.
//!
//! egui gives panels and a top-down cursor; Motif interfaces are built
//! out of *managed* rectangles — a form is carved into regions and each
//! region gets a bevel. These helpers carve the rectangles so the views
//! can stop centring fixed-width columns in whatever space they happen
//! to be given.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::bevel;

/// The part of `ui`'s available space that is actually on screen.
///
/// A side panel that grows past the width it reserved leaves the central
/// view laid out wider than it is visible, and anything centred on the
/// claimed width has its right edge cut away. Every helper here measures
/// against this instead of `available_rect_before_wrap` alone.
pub fn visible_rect(ui: &egui::Ui) -> egui::Rect {
    let avail = ui.available_rect_before_wrap();
    let visible = avail.intersect(ui.clip_rect());
    if visible.width() > 100.0 {
        egui::Rect::from_min_max(
            egui::pos2(visible.left(), avail.top()),
            egui::pos2(visible.right(), avail.bottom()),
        )
    } else {
        avail
    }
}

/// A page of content: at most `max` px wide, centred, with margins that
/// survive a narrow window. The workhorse for document-shaped views.
///
/// Prefer [`spread`] for views that have something to *do* with extra
/// width; a page that simply grows to 2000 px is unreadable.
pub fn page<R>(ui: &mut egui::Ui, max: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let visible = visible_rect(ui);
    let w = max.min(visible.width() - 48.0).max(200.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(visible.center().x - w / 2.0, visible.top()),
        Vec2::new(w, visible.height()),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), add)
        .inner
}

/// Carve `rect` into `n` columns separated by `gutter` px.
pub fn split_columns(rect: egui::Rect, n: usize, gutter: f32) -> Vec<egui::Rect> {
    let n = n.max(1);
    let w = (rect.width() - gutter * (n - 1) as f32) / n as f32;
    (0..n)
        .map(|i| {
            egui::Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * (w + gutter), rect.top()),
                Vec2::new(w.max(1.0), rect.height()),
            )
        })
        .collect()
}

/// Carve `rect` into rows of the given heights, separated by `gutter`.
/// A height of `0.0` means "take whatever is left" (at most one such).
pub fn split_rows(rect: egui::Rect, heights: &[f32], gutter: f32) -> Vec<egui::Rect> {
    let fixed: f32 = heights.iter().sum();
    let gutters = gutter * heights.len().saturating_sub(1) as f32;
    let flex = (rect.height() - fixed - gutters).max(0.0);
    let mut y = rect.top();
    heights
        .iter()
        .map(|h| {
            let h = if *h == 0.0 { flex } else { *h };
            let r = egui::Rect::from_min_size(
                egui::pos2(rect.left(), y),
                Vec2::new(rect.width(), h.max(1.0)),
            );
            y += h + gutter;
            r
        })
        .collect()
}

/// How many columns of at least `min_col` px fit in `width`, capped at
/// `max_cols`. The one decision every responsive view has to make.
pub fn column_count(width: f32, min_col: f32, max_cols: usize) -> usize {
    ((width / min_col.max(1.0)).floor() as usize).clamp(1, max_cols.max(1))
}

/// Run `add` inside `rect`, clipped to it. Returns the height the
/// content actually used, so callers can grow a pane to fit.
pub fn inside<R>(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let builder = egui::UiBuilder::new().max_rect(rect);
    let mut child = ui.new_child(builder);
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    let out = add(&mut child);
    ui.advance_cursor_after_rect(rect);
    out
}

/// A raised Motif panel filling `rect`, with an optional inset title on
/// the top edge. `add` draws inside the padded interior.
///
/// This is the workspace's basic building block: every dock, card and
/// chart well sits in one of these instead of floating on the grey.
pub fn panel<R>(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: Option<&str>,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.painter().rect_filled(rect, 0.0, crate::bg());
    bevel(ui.painter(), rect, true);
    let mut inner = rect.shrink(8.0);
    if let Some(title) = title {
        let font = egui::FontId::proportional(11.0);
        let caption: String = title
            .to_uppercase()
            .chars()
            .flat_map(|c| [c, '\u{2009}'])
            .collect();
        // Élidé sur la largeur du panneau, et **jamais** `layout_no_wrap`.
        //
        // Le titre était posé sans limite de largeur : « PATIENTS SOUS CE
        // TRAITEMENT » sur un volet de cent quatre-vingts pixels se
        // peignait par-dessus la gouttière et sur le panneau d'à côté.
        // Rien ne l'arrêtait — un `Painter` peint où on lui dit —, et
        // c'est la même famille de défaut que `ui.columns` qui ne découpe
        // pas et que `list_row` qui jetait son `RichText` : le
        // débordement ne casse rien, il se lit juste faux.
        //
        // La capitale espacée coûte cher en largeur, ce qui rend le cas
        // fréquent plutôt que rare.
        // Sur **une** ligne : un titre replié en deux repousserait le
        // filet et volerait au contenu une ligne que personne n'a
        // demandée. Un titre coupé se lit encore ; un panneau plus court
        // se remarque moins et coûte plus.
        let mut job = egui::text::LayoutJob::single_section(
            caption.trim_end().to_owned(),
            egui::TextFormat {
                font_id: font,
                color: crate::text_dim(),
                ..Default::default()
            },
        );
        job.wrap = egui::text::TextWrapping {
            max_width: (inner.width() - 4.0).max(8.0),
            max_rows: 1,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        let galley = ui.fonts(|f| f.layout_job(job));
        ui.painter().galley(
            egui::pos2(inner.left() + 2.0, inner.top()),
            galley.clone(),
            crate::text_dim(),
        );
        let y = inner.top() + galley.size().y + 3.0;
        rule(ui.painter(), inner.left(), inner.right(), y);
        inner.set_top(y + 6.0);
    }
    inside(ui, inner, add)
}

/// A sunken well — the trough a list, a chart or a scrolling text sits
/// in. Returns the padded interior.
pub fn well(ui: &egui::Ui, rect: egui::Rect) -> egui::Rect {
    ui.painter().rect_filled(rect, 0.0, crate::trough());
    bevel(ui.painter(), rect, false);
    rect.shrink(4.0)
}

/// The Motif two-tone hairline: a dark line with a light one under it.
pub fn rule(painter: &egui::Painter, x0: f32, x1: f32, y: f32) {
    painter.line_segment(
        [egui::pos2(x0, y), egui::pos2(x1, y)],
        Stroke::new(1.0_f32, crate::bg_dark()),
    );
    painter.line_segment(
        [egui::pos2(x0, y + 1.0), egui::pos2(x1, y + 1.0)],
        Stroke::new(1.0_f32, crate::bg_light()),
    );
}

/// The vertical twin of [`rule`], for splitting a panel into columns.
pub fn vrule(painter: &egui::Painter, y0: f32, y1: f32, x: f32) {
    painter.line_segment(
        [egui::pos2(x, y0), egui::pos2(x, y1)],
        Stroke::new(1.0_f32, crate::bg_dark()),
    );
    painter.line_segment(
        [egui::pos2(x + 1.0, y0), egui::pos2(x + 1.0, y1)],
        Stroke::new(1.0_f32, crate::bg_light()),
    );
}

/// One tab of a notebook.
pub struct Tab<'a> {
    pub label: &'a str,
    /// Draws a × on the right of the tab when true.
    pub closable: bool,
    /// A colour bar along the tab's top edge — the view's identity.
    pub tint: Option<Color32>,
}

impl<'a> Tab<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            closable: false,
            tint: None,
        }
    }
    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
}

/// What the user did to a notebook tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabAction {
    Select(usize),
    Close(usize),
}

/// A Motif notebook strip: square tabs, the active one raised and
/// merged into the page below it, the rest sunk behind a shared rule.
///
/// Scrolls horizontally when there are more tabs than room, so an
/// operator who has opened a dozen patients never loses one off-screen.
///
/// `salt` names this strip. Two strips on one screen used to share a
/// hard-coded one, and egui painted « First use of ScrollArea ID … »
/// across both of them the day a second one appeared: a widget meant to
/// be used twice cannot name itself.
pub fn tab_strip(ui: &mut egui::Ui, salt: &str, tabs: &[Tab], active: usize) -> Option<TabAction> {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let height = (font.size + 14.0).max(26.0);
    let mut action = None;
    // Where the selected tab sits, so the rule under the strip can be
    // broken there: an unbroken line makes every tab look inactive.
    let mut active_span: Option<(f32, f32)> = None;
    let strip = egui::Rect::from_min_size(
        ui.cursor().min,
        Vec2::new(ui.available_width(), height + 2.0),
    );
    egui::ScrollArea::horizontal()
        .id_salt(salt)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, tab) in tabs.iter().enumerate() {
                    let is_active = i == active;
                    let galley = ui.painter().layout_no_wrap(
                        tab.label.to_owned(),
                        font.clone(),
                        crate::text(),
                    );
                    let close_w = if tab.closable { 18.0 } else { 0.0 };
                    let w = galley.size().x + 24.0 + close_w;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(w, height), egui::Sense::click());
                    // Bring the selected tab into view. Without this the
                    // strip scrolled but never *to* anything: six tabs
                    // on a 1024 px screen at `text_scale = 1.25` left the
                    // sixth cut mid-word with no scrollbar to explain it,
                    // and a file opened straight onto that tab showed a
                    // page whose own tab was off-screen. A tab clipped to
                    // one letter does not read as « il y en a d'autres »,
                    // it reads as broken.
                    if is_active {
                        ui.scroll_to_rect(rect, None);
                    }
                    if !ui.is_rect_visible(rect) {
                        continue;
                    }
                    // The active tab keeps its full height and loses its
                    // bottom edge; the others are shortened from the top
                    // so the strip reads as depth, not as a button row.
                    let body = if is_active {
                        active_span = Some((rect.left(), rect.right()));
                        rect
                    } else {
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.top() + 3.0),
                            rect.right_bottom(),
                        )
                    };
                    let fill = if is_active {
                        crate::bg()
                    } else if resp.hovered() {
                        crate::bg_hover()
                    } else {
                        crate::trough()
                    };
                    ui.painter().rect_filled(body, 0.0, fill);
                    // Bevel the three visible sides only: the fourth
                    // opens onto the page.
                    let p = ui.painter();
                    for k in 0..2 {
                        let r = body.shrink(k as f32 + 0.5);
                        p.line_segment(
                            [r.left_bottom(), r.left_top()],
                            Stroke::new(1.0_f32, crate::bg_light()),
                        );
                        p.line_segment(
                            [r.left_top(), r.right_top()],
                            Stroke::new(1.0_f32, crate::bg_light()),
                        );
                        p.line_segment(
                            [r.right_top(), r.right_bottom()],
                            Stroke::new(1.0_f32, crate::bg_dark()),
                        );
                    }
                    if let Some(tint) = tab.tint {
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(body.left() + 2.0, body.top() + 2.0),
                            egui::pos2(body.right() - 2.0, body.top() + 5.0),
                        );
                        ui.painter().rect_filled(bar, 0.0, tint);
                    }
                    let color = if is_active {
                        crate::text()
                    } else {
                        crate::text_dim()
                    };
                    let galley =
                        ui.painter()
                            .layout_no_wrap(tab.label.to_owned(), font.clone(), color);
                    ui.painter().galley(
                        egui::pos2(body.left() + 12.0, body.center().y - galley.size().y / 2.0),
                        galley,
                        color,
                    );
                    if tab.closable {
                        let x = egui::Rect::from_center_size(
                            egui::pos2(body.right() - 13.0, body.center().y),
                            Vec2::splat(14.0),
                        );
                        let hit = ui.interact(
                            x,
                            ui.id().with(("motif_tab_close", i)),
                            egui::Sense::click(),
                        );
                        if hit.hovered() {
                            ui.painter().rect_filled(x, 0.0, bg_hover_strong());
                        }
                        ui.painter().text(
                            x.center(),
                            egui::Align2::CENTER_CENTER,
                            "×",
                            egui::FontId::proportional(14.0),
                            if hit.hovered() {
                                crate::text()
                            } else {
                                crate::text_dim()
                            },
                        );
                        if hit.clicked() {
                            action = Some(TabAction::Close(i));
                        }
                        // A middle click closes too, the way every
                        // notebook since has behaved.
                        if resp.middle_clicked() {
                            action = Some(TabAction::Close(i));
                        }
                        if hit.hovered() {
                            continue;
                        }
                    }
                    if resp.clicked() && action.is_none() {
                        action = Some(TabAction::Select(i));
                    }
                }
            });
        });
    // The rule the tabs sit on, broken under the active tab so that tab
    // reads as the front of the page rather than one more button.
    let y = strip.bottom() - 2.0;
    match active_span {
        Some((l, r)) => {
            if l > strip.left() {
                rule(ui.painter(), strip.left(), l, y);
            }
            if r < strip.right() {
                rule(ui.painter(), r, strip.right(), y);
            }
        }
        None => rule(ui.painter(), strip.left(), strip.right(), y),
    }
    action
}

/// Slightly stronger than the hover tint, for the × target inside a tab.
/// Mixed from the theme rather than fixed: a blue-grey highlight on the
/// HP VUE green reads as a stain, not as a hover.
fn bg_hover_strong() -> Color32 {
    crate::bg_hover().lerp_to_gamma(crate::bg_light(), 0.45)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_share_the_width_and_leave_gutters() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(320.0, 100.0));
        let cols = split_columns(rect, 3, 10.0);
        assert_eq!(cols.len(), 3);
        assert!((cols[0].width() - 100.0).abs() < 0.01);
        assert!((cols[1].left() - 110.0).abs() < 0.01);
        assert!((cols[2].right() - 320.0).abs() < 0.01);
    }

    #[test]
    fn a_zero_height_row_takes_what_is_left() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(100.0, 300.0));
        let rows = split_rows(rect, &[40.0, 0.0, 60.0], 10.0);
        assert!((rows[1].height() - 180.0).abs() < 0.01);
        assert!((rows[2].bottom() - 300.0).abs() < 0.01);
    }

    #[test]
    fn column_count_is_clamped_both_ways() {
        assert_eq!(column_count(300.0, 320.0, 4), 1);
        assert_eq!(column_count(1000.0, 320.0, 4), 3);
        assert_eq!(column_count(4000.0, 320.0, 4), 4);
    }
}
