#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod biology;
mod bulletin;
mod classes;
mod codex;
mod conciliation;
mod config;
mod db;
mod entretien;
mod facets;
mod fuzzy;
mod graph;
mod insulin;
mod location;
mod maintenance;
mod ordonnance;
mod ordonnancier;
mod pdf;
mod release;
mod revue;
mod scans;
mod strings;
mod surveillance;
mod tables;
mod vaccines;
mod vitale;
mod winscard;

use eframe::egui;

/// The size to open at: the screenshot/e2e hook
/// (`BPM_CADDY_WINDOW=1280x1200`) first, then the size the workspace
/// was left at, then a default that fits a counter screen.
fn window_size() -> [f32; 2] {
    let default = config::Layout::load().window().unwrap_or([1024.0, 700.0]);
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

/// Load a TrueType/OpenType file and make it the default family.
fn install_font(ctx: &egui::Context, path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // egui panics on a file it cannot parse, and the path lives in
    // config.toml — a mistyped one would make the app uncloseable.
    // Parse it here first and simply keep the embedded family instead.
    ab_glyph::FontRef::try_from_slice(&bytes)
        .map_err(|e| format!("fichier de police illisible : {e}"))?;
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("custom".to_owned(), egui::FontData::from_owned(bytes));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "custom".to_owned());
    }
    ctx.set_fonts(fonts);
    Ok(())
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
            // A font chosen in the options replaces the embedded family
            // for the whole interface; anything unreadable is ignored.
            if let Some(path) = config::Config::load().ui.font_path {
                if let Err(e) = install_font(&cc.egui_ctx, &path) {
                    eprintln!("police {} ignorée : {e}", path.display());
                }
            }
            Ok(Box::new(app::App::new()))
        }),
    )
}
