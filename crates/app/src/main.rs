//! ROM Manager desktop app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod jobs;
mod pages;
mod settings;
mod widgets;

fn icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../../../assets/icon-256.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("ROM Manager")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([900.0, 560.0])
        .with_drag_and_drop(true)
        .with_app_id("rom-manager");
    if let Some(i) = icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(i));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    // Optional: a library folder to open, e.g. from a desktop shortcut.
    let open_path = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "ROM Manager",
        options,
        Box::new(|cc| Ok(Box::new(app::RomApp::new(cc, open_path)))),
    )
}
