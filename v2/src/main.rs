#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bpm_caddy_v2::{app::App, screens::options, session};
use eframe::egui;

/// `BPM_CADDY_WINDOW=1280x800` (screenshots), else a counter screen.
fn window_size() -> [f32; 2] {
    let default = [1024.0, 700.0];
    let Ok(spec) = std::env::var("BPM_CADDY_WINDOW") else {
        return default;
    };
    match spec.split_once(['x', 'X']) {
        Some((w, h)) => match (w.trim().parse::<f32>(), h.trim().parse::<f32>()) {
            (Ok(w), Ok(h)) if w >= 640.0 && h >= 480.0 => [w, h],
            _ => default,
        },
        None => default,
    }
}

struct Window {
    app: App,
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.update(ctx);
    }
}

fn main() -> eframe::Result {
    // Read and remove the password hook before any thread starts.
    let _ = session::env_password();
    let config = bpm_caddy::config::Config::load();
    motif::set_theme(&config.ui.theme);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size())
            .with_min_inner_size([960.0, 640.0])
            .with_icon(motif::icon())
            .with_title("BPM-Caddy v2"),
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy v2",
        options,
        Box::new(move |cc| {
            motif::apply(&cc.egui_ctx);
            motif::apply_scale(
                &cc.egui_ctx,
                config.ui.text_scale,
                options::density(&config),
            );
            Ok(Box::new(Window { app: App::new() }))
        }),
    )
}
