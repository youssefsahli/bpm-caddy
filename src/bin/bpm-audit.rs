#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! `bpm-audit` — la fenêtre d'audit de l'officine. Voir
//! `src/audit_window.rs` : ce binaire n'en est que la porte.

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([800.0, 560.0])
            .with_icon(motif::icon())
            .with_title("BPM-Caddy — audit"),
        ..Default::default()
    };
    eframe::run_native(
        "bpm-audit",
        options,
        Box::new(|_cc| Ok(Box::new(bpm_caddy::audit_window::AuditApp::new()))),
    )
}
