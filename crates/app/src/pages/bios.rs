use crate::app::RomApp;
use crate::jobs::{Job, JobOutput};
use crate::widgets::{self, card, page_title, section};
use egui::{RichText, Ui};
use std::path::PathBuf;
use tabularium_core::bios::{self, FileState, ImportOutcome, ReqStatus};
use tabularium_core::{profiles, systems};

pub struct BiosPage {
    pub lib_status: Option<Vec<ReqStatus>>,
    pub card: Option<(PathBuf, String, Vec<ReqStatus>)>,
    pub card_profile: String,
    pub import_result: Option<Vec<(PathBuf, ImportOutcome)>>,
    pub only_my_systems: bool,
}

impl Default for BiosPage {
    fn default() -> Self {
        BiosPage {
            lib_status: None,
            card: None,
            card_profile: "onion".into(),
            import_result: None,
            only_my_systems: true,
        }
    }
}

fn status_list(ui: &mut Ui, _id: &str, statuses: &[ReqStatus]) {
    for rs in statuses {
        let ok = rs.satisfied();
        let (icon, color) = if ok {
            ("✔", widgets::ok_color(ui))
        } else if rs.req.required {
            ("✖", widgets::err_color(ui))
        } else {
            ("○", ui.visuals().weak_text_color())
        };
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.colored_label(color, RichText::new(icon).size(15.0));
            ui.label(RichText::new(systems::name_of(rs.req.system)).strong());
            let kind = if rs.req.required {
                "required"
            } else {
                "optional"
            };
            let any = if rs.req.any_of && rs.req.files.len() > 1 {
                ", any one of these"
            } else {
                ""
            };
            ui.label(RichText::new(format!("({kind}{any})")).weak());
        });
        ui.indent(("bios", rs.req.system), |ui| {
            let long = rs.files.len() > 6;
            if long && !rs.files.iter().any(|f| f.state != FileState::Missing) {
                ui.colored_label(
                    ui.visuals().weak_text_color(),
                    format!("· none of the {} known BIOS dumps found", rs.files.len()),
                );
            }
            for f in rs
                .files
                .iter()
                .filter(|f| !long || f.state != FileState::Missing)
            {
                let (mark, text) = match &f.state {
                    FileState::Ok => ("✔", f.file.name.to_string()),
                    FileState::Missing => ("·", format!("{} — missing", f.file.name)),
                    FileState::BadHash(h) => (
                        "⚠",
                        format!("{} — wrong version or corrupt (MD5 {h})", f.file.name),
                    ),
                    FileState::Misplaced(p) => (
                        "⚠",
                        format!(
                            "{} — found at {}; move it into the BIOS folder",
                            f.file.name,
                            p.display()
                        ),
                    ),
                };
                let color = match &f.state {
                    FileState::Ok => widgets::ok_color(ui),
                    FileState::Missing => ui.visuals().weak_text_color(),
                    _ => widgets::warn_color(ui),
                };
                ui.colored_label(color, format!("{mark} {text}"));
            }
            if !ok && !rs.req.note.is_empty() {
                ui.label(RichText::new(rs.req.note).weak().small());
            }
        });
    }
}

impl RomApp {
    pub fn bios_page(&mut self, ui: &mut Ui) {
        page_title(
            ui,
            "BIOS files",
            "Some systems need the original console's BIOS to run games. Files are recognised by their MD5 \
             hash, so it doesn't matter what they're called.",
        );
        let Some(root) = self.library_root() else {
            return;
        };
        let lib_bios = root.join("bios");
        let my_systems: Vec<String> = self
            .data
            .stats
            .iter()
            .filter(|(_, s)| s.count > 0)
            .map(|(k, _)| k.clone())
            .collect();

        card(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.busy(),
                        egui::Button::new(RichText::new("Import BIOS files…").strong()),
                    )
                    .on_hover_text(
                        "Pick a folder; known BIOS files are copied in under their correct names.",
                    )
                    .clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title("Folder containing BIOS files")
                        .pick_folder()
                    {
                        let dest = lib_bios.clone();
                        self.job = Some(Job::spawn("Importing BIOS files", ui.ctx(), move |_| {
                            JobOutput::BiosImport(bios::import_files(&[dir], &dest, false))
                        }));
                    }
                }
                if ui
                    .button("Open BIOS folder")
                    .on_hover_text("You can also copy BIOS files into this folder yourself")
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&lib_bios);
                    let _ = open::that(&lib_bios);
                }
                if ui
                    .checkbox(&mut self.bios.only_my_systems, "Only systems in my library")
                    .changed()
                {
                    self.bios.lib_status = None;
                }
            });
            if let Some(dir) = widgets::path_entry(ui, "bios-src", "Import") {
                let dest = lib_bios.clone();
                self.job = Some(Job::spawn("Importing BIOS files", ui.ctx(), move |_| {
                    JobOutput::BiosImport(bios::import_files(&[dir], &dest, false))
                }));
            }
            if let Some(res) = &self.bios.import_result {
                let imported: Vec<&str> = res
                    .iter()
                    .filter_map(|(_, o)| match o {
                        ImportOutcome::Imported(n) => Some(n.as_str()),
                        _ => None,
                    })
                    .collect();
                let present = res
                    .iter()
                    .filter(|(_, o)| matches!(o, ImportOutcome::AlreadyPresent(_)))
                    .count();
                ui.label(format!(
                    "Imported {}{}; {present} already in the library.",
                    imported.len(),
                    if imported.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", imported.join(", "))
                    }
                ));
                for (p, o) in res {
                    if let ImportOutcome::BadHash(why) = o {
                        ui.colored_label(
                            widgets::warn_color(ui),
                            format!("{}: {why}", p.display()),
                        );
                    }
                }
            }
        });

        if self.bios.lib_status.is_none() {
            let filter = self.bios.only_my_systems.then_some(my_systems.as_slice());
            self.bios.lib_status = Some(bios::check(&lib_bios, filter, None));
        }
        section(ui, "In your library");
        let statuses = self.bios.lib_status.clone().unwrap_or_default();
        if statuses.is_empty() {
            ui.label(RichText::new("None of the systems in your library need a BIOS.").weak());
        } else {
            status_list(ui, "lib-bios", &statuses);
        }

        ui.add_space(12.0);
        card(ui, |ui| {
            section(ui, "Check an SD card");
            ui.label(
                "See whether a card has the right BIOS files in the right folder for its firmware.",
            );
            ui.horizontal(|ui| {
                ui.label("Firmware:");
                egui::ComboBox::from_id_salt("bios-profile")
                    .selected_text(
                        profiles::get(&self.bios.card_profile)
                            .map(|p| p.label)
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for p in profiles::PROFILES {
                            ui.selectable_value(
                                &mut self.bios.card_profile,
                                p.id.to_string(),
                                p.label,
                            );
                        }
                    });
                if ui.button("Choose card…").clicked() {
                    let mut dlg = rfd::FileDialog::new().set_title("SD card (its top folder)");
                    if let Some(d) = &self.settings.last_card_folder {
                        dlg = dlg.set_directory(d);
                    }
                    if let Some(card_dir) = dlg.pick_folder() {
                        let p = profiles::get(&self.bios.card_profile).unwrap();
                        let st = bios::check(&card_dir, Some(&my_systems), Some(p));
                        self.settings.last_card_folder = Some(card_dir.clone());
                        self.bios.card = Some((card_dir, p.id.to_string(), st));
                    }
                }
            });
            if let Some(card_dir) = widgets::path_entry(ui, "bios-card", "Check") {
                let p = profiles::get(&self.bios.card_profile).unwrap();
                let st = bios::check(&card_dir, Some(&my_systems), Some(p));
                self.bios.card = Some((card_dir, p.id.to_string(), st));
            }
            if let Some((dir, prof, st)) = &self.bios.card {
                let p = profiles::get(prof).unwrap();
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{} — {}", dir.display(), p.label)).weak(),
                    )
                    .truncate(),
                );
                status_list(ui, "card-bios", st);
            }
        });
    }
}
