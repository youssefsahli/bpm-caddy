#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
            .with_title("BPM-Caddy"),
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    search_query: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.heading("BPM-Caddy");
                ui.label("Clinical pharmacy workflow & analytics");
                ui.add_space(24.0);

                let search = ui.add_sized(
                    [420.0, 32.0],
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search patients (fuzzy)…"),
                );
                // Search is the default view: keep the bar focused on launch.
                if !ctx.wants_keyboard_input() {
                    search.request_focus();
                }
            });
        });
    }
}
