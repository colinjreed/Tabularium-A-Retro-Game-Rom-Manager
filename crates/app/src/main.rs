//! Tabularium desktop app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod jobs;
mod pages;
mod settings;
mod sound;
mod themes;
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

/// Bring settings over from when the app was called "ROM Manager".
fn migrate_settings() {
    let (Some(new), Some(old)) = (
        eframe::storage_dir("tabularium"),
        eframe::storage_dir("rom-manager"),
    ) else {
        return;
    };
    let (new_file, old_file) = (new.join("app.ron"), old.join("app.ron"));
    if !new_file.exists() && old_file.exists() {
        let _ = std::fs::create_dir_all(&new);
        let _ = std::fs::copy(&old_file, &new_file);
    }
}

fn main() -> eframe::Result {
    migrate_settings();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Tabularium")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([900.0, 560.0])
        .with_drag_and_drop(true)
        .with_app_id("tabularium");
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
        "Tabularium",
        options,
        Box::new(|cc| Ok(Box::new(app::RomApp::new(cc, open_path)))),
    )
}
