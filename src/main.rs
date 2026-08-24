#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod db;
mod fuzzy;
mod pdf;
mod strings;
mod tables;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
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
