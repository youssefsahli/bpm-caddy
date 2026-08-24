#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod db;
mod fuzzy;
mod pdf;
mod strings;
mod tables;

use eframe::egui;

/// Screenshot/e2e hook: `BPM_CADDY_WINDOW=1280x1200` opens the window
/// at that size instead of the default.
fn window_size() -> [f32; 2] {
    let default = [1024.0, 700.0];
    let Ok(spec) = std::env::var("BPM_CADDY_WINDOW") else {
        return default;
    };
    let Some((w, h)) = spec.split_once(['x', 'X']) else {
        return default;
    };
    match (w.trim().parse::<f32>(), h.trim().parse::<f32>()) {
        (Ok(w), Ok(h)) if w >= 640.0 && h >= 480.0 => [w, h],
        _ => default,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size())
            .with_min_inner_size([960.0, 640.0])
            .with_icon(motif::icon())
            .with_title("BPM-Caddy"),
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy",
        options,
        Box::new(|cc| {
            motif::apply(&cc.egui_ctx);
            Ok(Box::new(app::App::new()))
        }),
    )
}
