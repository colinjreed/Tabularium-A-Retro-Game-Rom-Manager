use crate::app::RomApp;
use crate::widgets::{card, page_title, section};
use egui::{RichText, Ui};
use tabularium_core::library::Library;

impl RomApp {
    pub fn settings_page(&mut self, ui: &mut Ui) {
        page_title(ui, "Settings", "");
        card(ui, |ui| {
            section(ui, "Library");
            match &self.lib {
                Some(l) => {
                    ui.label(l.root.display().to_string());
                }
                None => {
                    ui.label(RichText::new("No library open.").weak());
                }
            }
            ui.horizontal(|ui| {
                if ui.button("Open another library…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        if Library::is_library(&p) {
                            self.open_library(p);
                        } else {
                            self.notify_err(format!("{} isn't a Tabularium library.", p.display()));
                        }
                    }
                }
                if self.lib.is_some() && ui.button("Close library").clicked() {
                    self.lib = None;
                    self.settings.library = None;
                    self.data = Default::default();
                }
                if let Some(root) = self.library_root() {
                    if ui.button("Open library folder").clicked() {
                        let _ = open::that(root);
                    }
                }
            });
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            section(ui, "ScreenScraper account");
            ui.label(
                "ScreenScraper needs your user account plus developer credentials. Request developer \
                 credentials for this program on the ScreenScraper forum.",
            );
            let c = &mut self.settings.screenscraper;
            egui::Grid::new("ss")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Username");
                    ui.text_edit_singleline(&mut c.user);
                    ui.end_row();
                    ui.label("Password");
                    ui.add(egui::TextEdit::singleline(&mut c.password).password(true));
                    ui.end_row();
                    ui.label("Developer ID");
                    ui.text_edit_singleline(&mut c.dev_id);
                    ui.end_row();
                    ui.label("Developer password");
                    ui.add(egui::TextEdit::singleline(&mut c.dev_password).password(true));
                    ui.end_row();
                });
            ui.checkbox(
                &mut self.settings.remember_passwords,
                "Remember passwords on this computer",
            );
            ui.label(RichText::new(
                "Passwords are stored unencrypted in this app's settings file when remembered. Otherwise \
                 they're kept until you quit. The SCREENSCRAPER_USER / _PASSWORD / _DEVID / _DEVPASSWORD \
                 environment variables also work.",
            ).weak().small());
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            section(ui, "System names");
            use tabularium_core::systems::{set_name_region, NameRegion};
            for (r, label) in [
                (
                    NameRegion::Us,
                    "North America: Super Nintendo, Genesis, TurboGrafx-16",
                ),
                (
                    NameRegion::Europe,
                    "Europe: Super Nintendo, Mega Drive, PC Engine",
                ),
                (
                    NameRegion::Japan,
                    "Japan: Super Famicom, Mega Drive, PC Engine, Mark III",
                ),
            ] {
                if ui
                    .radio_value(&mut self.settings.system_names, r, label)
                    .changed()
                {
                    set_name_region(r);
                    self.view.invalidate();
                }
            }
            ui.label(RichText::new("Only changes the names shown here; folder names on SD cards follow each firmware.").weak());
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            section(ui, "libretro artwork matching");
            ui.add(
                egui::Slider::new(&mut self.settings.match_cutoff, 0.70..=1.0).text("strictness"),
            );
            ui.label(RichText::new(
                "Lower finds art for more oddly named ROMs, but risks picking the wrong game's art. 0.92 is a safe default.",
            ).weak());
        });
        ui.add_space(10.0);
        ui.label(RichText::new(format!("Tabularium {}", env!("CARGO_PKG_VERSION"))).weak());
    }
}
