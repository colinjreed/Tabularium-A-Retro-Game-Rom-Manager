use crate::app::RomApp;
use crate::jobs::{Job, JobOutput};
use crate::widgets::{self, card, page_title, section};
use egui::{RichText, Ui};
use std::path::PathBuf;
use tabularium_core::import::{import, ImportOptions, ImportReport};
use tabularium_core::library::Library;
use tabularium_core::systems;

pub struct ImportPage {
    pub sources: Vec<PathBuf>,
    pub move_files: bool,
    pub forced: Option<&'static str>,
    pub keep_unknown: bool,
    pub official_names: bool,
    pub strip_numbers: bool,
    pub result: Option<(ImportReport, bool)>,
}

impl Default for ImportPage {
    fn default() -> Self {
        ImportPage {
            sources: vec![],
            move_files: false,
            forced: None,
            keep_unknown: false,
            official_names: true,
            strip_numbers: true,
            result: None,
        }
    }
}

impl RomApp {
    pub fn import_page(&mut self, ui: &mut Ui) {
        page_title(
            ui,
            "Import ROMs",
            "Add folders of ROMs. Each file's system is worked out from its extension, zip contents, \
             folder names or file header, and it's filed into the library.",
        );
        if let Some(svc) = self
            .lib
            .as_ref()
            .and_then(|l| tabularium_core::detect::cloud_folder(&l.root))
        {
            card(ui, |ui| {
                ui.colored_label(
                    widgets::warn_color(ui),
                    RichText::new(format!("Your library is in {svc}")).strong(),
                );
                ui.label(
                    "Syncing services can damage the library's database while Tabularium has it \
                     open, and every ROM you import gets uploaded. For the best results, quit \
                     Tabularium, move the library folder somewhere ordinary (for example your \
                     home folder) in Finder, then use Settings → Open another library… to open \
                     it from there.",
                );
            });
            ui.add_space(10.0);
        }
        card(ui, |ui| {
            section(ui, "Folders and files to import");
            let mut remove = None;
            for (i, p) in self.import.sources.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.small_button("×").clicked() {
                        remove = Some(i);
                    }
                    ui.label(p.display().to_string());
                });
            }
            if let Some(i) = remove {
                self.import.sources.remove(i);
            }
            if self.import.sources.is_empty() {
                ui.label(RichText::new("Nothing added yet.").weak());
            }
            if let Some(svc) = self
                .import
                .sources
                .iter()
                .find_map(|p| tabularium_core::detect::cloud_folder(p))
            {
                ui.colored_label(
                    widgets::warn_color(ui),
                    format!(
                        "This is in {svc}. Files that are only stored online are downloaded one by \
                         one as they're imported, which can be very slow. Right-click the folder in \
                         Finder and choose Download Now (or Keep Downloaded) first."
                    ),
                );
            }
            ui.horizontal(|ui| {
                if ui.button("Add folder…").clicked() {
                    if let Some(ps) = rfd::FileDialog::new().pick_folders() {
                        self.import.sources.extend(ps);
                    }
                }
                if ui.button("Add files…").clicked() {
                    if let Some(ps) = rfd::FileDialog::new().pick_files() {
                        self.import.sources.extend(ps);
                    }
                }
            });
            if let Some(p) = widgets::path_entry(ui, "import-src", "Add") {
                self.import.sources.push(p);
            }
            ui.label(
                RichText::new("Tip: you can also drag folders onto this window.")
                    .weak()
                    .small(),
            );

            section(ui, "Options");
            ui.radio_value(
                &mut self.import.move_files,
                false,
                "Copy into the library (originals stay where they are)",
            );
            ui.radio_value(
                &mut self.import.move_files,
                true,
                "Move into the library (saves space)",
            );
            ui.horizontal(|ui| {
                ui.label("System:");
                egui::ComboBox::from_id_salt("forced-system")
                    .selected_text(self.import.forced.and_then(systems::get).map(|s| s.display_name()).unwrap_or("Detect automatically"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.import.forced, None, "Detect automatically");
                        for s in systems::SYSTEMS {
                            ui.selectable_value(&mut self.import.forced, Some(s.id), s.display_name());
                        }
                    })
                    .response
                    .on_hover_text("Force every file to one system, for folders of .bin/.iso files that can't be detected.");
            });
            ui.checkbox(
                &mut self.import.keep_unknown,
                "Copy files I can't identify into the library's 'unsorted' folder",
            );
            ui.checkbox(
                &mut self.import.official_names,
                "Rename to official names (looked up by each file's checksum)",
            )
            .on_hover_text(
                "Uses the No-Intro lists for cartridge systems, e.g. '003 sonic.md' becomes \
                 'Sonic The Hedgehog (USA, Europe).md'. Also catches games filed under the wrong \
                 system, like Game Gear games saved as .sms.",
            );
            ui.checkbox(
                &mut self.import.strip_numbers,
                "Remove numbering like '003 - ' from the start of names",
            )
            .on_hover_text(
                "Only when most files in a folder are numbered, so titles like '1942' are kept.",
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let can = !self.import.sources.is_empty() && !self.busy();
                if ui
                    .add_enabled(can, egui::Button::new("Preview"))
                    .on_hover_text("See what would happen, without copying")
                    .clicked()
                {
                    self.start_import(ui.ctx(), true);
                }
                if ui
                    .add_enabled(can, egui::Button::new(RichText::new("Import").strong()))
                    .clicked()
                {
                    self.start_import(ui.ctx(), false);
                }
            });
        });

        if let Some((rep, dry)) = &self.import.result {
            ui.add_space(10.0);
            card(ui, |ui| import_results(ui, rep, *dry));
        }

        // Drag and drop.
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if !dropped.is_empty() {
            self.import.sources.extend(dropped);
        }
    }

    fn start_import(&mut self, ctx: &egui::Context, dry_run: bool) {
        let Some(root) = self.library_root() else {
            return;
        };
        let opts = ImportOptions {
            fetch: Some(tabularium_core::http::default_fetcher()),
            move_files: self.import.move_files,
            dry_run,
            forced_system: self.import.forced.and_then(systems::get),
            keep_unknown: self.import.keep_unknown,
            official_names: self.import.official_names,
            strip_numbers: self.import.strip_numbers,
        };
        let sources = self.import.sources.clone();
        self.import.result = None;
        let title = if dry_run {
            "Previewing import"
        } else {
            "Importing"
        };
        self.job = Some(Job::spawn(title, ctx, move |rep| {
            match Library::open(&root).and_then(|lib| import(&lib, &sources, &opts, rep)) {
                Ok(report) => JobOutput::Import { report, dry_run },
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }
}

fn import_results(ui: &mut Ui, rep: &ImportReport, dry: bool) {
    let verb = if dry { "Would add" } else { "Added" };
    let head = format!(
        "{verb} {} ROM(s) · {} duplicate(s) skipped · {} not identified",
        rep.added_total(),
        rep.duplicates.len(),
        rep.unknown.len()
    );
    ui.label(RichText::new(head).strong().size(crate::themes::fs(15.0)));
    if rep.from_cloud > 0 {
        ui.colored_label(
            widgets::warn_color(ui),
            format!(
                "{} file(s) were stored only in iCloud and had to be downloaded first, which is slow. \
                 Before importing, right-click the folder in Finder and choose Download Now \
                 (or Keep Downloaded).",
                rep.from_cloud
            ),
        );
    }
    if rep.cancelled {
        ui.colored_label(widgets::warn_color(ui), "Stopped before finishing.");
    }
    if dry {
        ui.label(RichText::new("Preview only: nothing was copied.").weak());
    }
    if !rep.added.is_empty() {
        egui::Grid::new("import-by-system")
            .num_columns(2)
            .show(ui, |ui| {
                for (s, n) in &rep.added {
                    ui.label(systems::name_of(s));
                    ui.label(n.to_string());
                    ui.end_row();
                }
            });
    }
    if !rep.unknown.is_empty() {
        egui::CollapsingHeader::new(format!("Couldn't identify {} file(s)", rep.unknown.len())).show(ui, |ui| {
            ui.label(RichText::new(
                "Put these in a folder named after their system (e.g. 'PS1', 'megadrive') and import again, \
                 or pick the system in Options above.",
            ).weak());
            for rs in rep.unknown.iter().take(500) {
                ui.label(format!("{}  — {}", rs.main.display(), rs.reason));
            }
        });
    }
    if !rep.duplicates.is_empty() {
        egui::CollapsingHeader::new(format!("{} duplicate(s) skipped", rep.duplicates.len())).show(
            ui,
            |ui| {
                for (p, of) in rep.duplicates.iter().take(500) {
                    ui.label(format!("{}  = {of}", p.display()));
                }
            },
        );
    }
    for (title, list) in [("Name conflicts", &rep.conflicts), ("Errors", &rep.errors)] {
        if !list.is_empty() {
            egui::CollapsingHeader::new(format!("{title} ({})", list.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for (p, why) in list {
                        ui.label(format!("{}: {why}", p.display()));
                    }
                });
        }
    }
}
