//! Options: what this post looks like, who works at it, which base it
//! opened. Written to the same config.toml as the first window.

use super::Ctx;
use crate::t::{t, tf};
use crate::ui;
use bpm_caddy::config::Config;
use eframe::egui;

pub struct Options {
    config: Config,
    operator: String,
    password_old: String,
    password_new: String,
    password_again: String,
}

impl Default for Options {
    fn default() -> Self {
        let config = Config::load();
        Options {
            operator: config.ui.operator.clone(),
            config,
            password_old: String::new(),
            password_new: String::new(),
            password_again: String::new(),
        }
    }
}

/// The text scales offered: the range every layout is held to.
pub const SCALES: [f32; 4] = [1.0, 1.15, 1.25, 1.6];

impl Options {
    pub fn ui(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        egui::ScrollArea::vertical()
            .id_salt("options")
            .auto_shrink([false, false])
            .show(ui, |ui| self.body(ui, cx));
    }

    fn body(&mut self, ui: &mut egui::Ui, cx: &mut Ctx) {
        ui::title(ui, t("options.title"), None);

        ui::heading(ui, t("options.look"));
        let mut theme = motif::theme().key.to_owned();
        let themes: Vec<(String, String)> = motif::THEMES
            .iter()
            .map(|th| (th.key.to_owned(), th.label.to_owned()))
            .collect();
        ui.horizontal(|ui| {
            ui.label(t("options.theme"));
            let w = motif::select_width(ui, themes.iter().map(|(_, l)| l.as_str()));
            if motif::select(ui, "theme", w, &mut theme, &themes).changed() {
                motif::set_theme(&theme);
                motif::apply(ui.ctx());
                self.apply_scale(ui.ctx());
                self.config.ui.theme = theme.clone();
                self.save(cx);
            }
        });
        ui.horizontal(|ui| {
            ui.label(t("options.scale"));
            for s in SCALES {
                let on = (self.config.ui.text_scale - s).abs() < 0.01;
                if motif::radio(ui, on, format!("{:.0} %", s * 100.0)).clicked() && !on {
                    self.config.ui.text_scale = s;
                    self.apply_scale(ui.ctx());
                    self.save(cx);
                }
            }
        });

        ui::heading(ui, t("options.post"));
        ui.horizontal(|ui| {
            ui.label(t("options.operator"));
            let r = ui::field(ui, &mut self.operator, 6.0, t("options.operator_hint"));
            if r.lost_focus() && self.operator.trim() != self.config.ui.operator {
                self.config.ui.operator = self.operator.trim().to_uppercase();
                self.operator = self.config.ui.operator.clone();
                cx.s.operator = self.operator.clone();
                self.save(cx);
            }
        });

        ui::heading(ui, t("options.base"));
        ui.label(tf("options.base_at", &[&cx.s.path.display()]));
        ui.label(motif::hint(t("options.base_shared")));

        ui::heading(ui, t("options.password"));
        let lw = 22.0;
        for (label, value) in [
            (t("options.pw_old"), &mut self.password_old),
            (t("options.pw_new"), &mut self.password_new),
            (t("options.pw_again"), &mut self.password_again),
        ] {
            ui.horizontal(|ui| {
                ui::cell_text(
                    ui,
                    egui::RichText::new(label).color(motif::text_dim()),
                    ui::chars_wide(ui, lw),
                );
                motif::field(
                    ui,
                    ui::chars_wide(ui, 24.0),
                    egui::TextEdit::singleline(value).password(true),
                );
            });
        }
        if motif::button(ui, t("options.pw_change")).clicked() {
            if self.password_new.is_empty() || self.password_new != self.password_again {
                cx.s.fail(t("options.pw_mismatch"));
            } else {
                match cx
                    .s
                    .db
                    .change_password(&self.password_old, &self.password_new)
                {
                    Ok(()) => {
                        cx.s.say(t("options.pw_changed"));
                        self.password_old.clear();
                        self.password_new.clear();
                        self.password_again.clear();
                    }
                    Err(e) => cx.s.fail(e),
                }
            }
        }

        ui::heading(ui, t("options.about"));
        ui.label(tf("options.version", &[&env!("CARGO_PKG_VERSION")]));
        ui.label(motif::hint(t("options.about_v2")));
    }

    pub fn apply_scale(&self, ctx: &egui::Context) {
        motif::apply_scale(ctx, self.config.ui.text_scale, density(&self.config));
    }

    fn save(&mut self, cx: &mut Ctx) {
        if let Err(e) = self.config.save() {
            cx.s.fail(e);
        }
    }
}

pub fn density(config: &Config) -> motif::Density {
    if config.ui.density == "compact" {
        motif::Density::Compact
    } else {
        motif::Density::Comfortable
    }
}
