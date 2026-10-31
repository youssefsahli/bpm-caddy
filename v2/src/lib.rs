//! BPM-Caddy v2: a new window on the same base.
//!
//! The library holds everything but `main`, so every screen can be drawn
//! in a plain `#[test]` with a headless `egui::Context`.

pub mod analyse;
pub mod app;
pub mod screens;
pub mod session;
pub mod t;
pub mod ui;
