use crate::app::RomApp;
use crate::jobs::{Job, JobOutput};
use crate::widgets::{card, filter_editor, page_title, section, systems_picker};
use egui::{RichText, Ui};
use std::collections::BTreeMap;
use tabularium_core::filters::{self, FilterSpec, Rejected};
use tabularium_core::library::Library;
use tabularium_core::{human_size, systems};

#[derive(Default)]
pub struct CleanPage {
    pub rename: Option<tabularium_core::rename::FixReport>,
    pub spec: FilterSpec,
    pub systems: Vec<String>,
    pub results: Option<Vec<Rejected>>,
    pub confirm_empty_trash: bool,
}

impl RomApp {
    pub fn clean_page(&mut self, ui: &mut Ui) {
        egui::Panel::right("clean-results")
            .resizable(true)
            .default_size(420.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("clean-res")
                    .show(ui, |ui| self.clean_results(ui));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                page_title(
                    ui,
                    "Clean up",
                    "Find duplicates and versions you don't want. Nothing is deleted: removed ROMs go to the \
                     trash, where you can restore them.",
                );
                let missing = self.data.scummvm_missing;
                if missing > 0 {
                    card(ui, |ui| {
                        section(ui, "ScummVM game IDs");
                        ui.label(format!(
                            "{missing} ScummVM game(s) have no game ID, so handheld firmware won't know \
                             how to start them. If ScummVM is installed on this computer, Tabularium asks \
                             it to recognise each game from its files, the same way ScummVM's 'Add Game' \
                             does; otherwise it matches by name."
                        ));
                        if ui
                            .add_enabled(!self.busy(), egui::Button::new(RichText::new("Find ScummVM IDs").strong()))
                            .clicked()
                        {
                            self.start_scummvm_ids(ui.ctx());
                        }
                    });
                    ui.add_space(10.0);
                }
                let n = self.data.discs_to_recheck;
                if n > 0 {
                    card(ui, |ui| {
                        section(ui, "Disc games");
                        ui.label(format!(
                            "{n} disc game(s) were added by an older version, which compared disc games by \
                             their .cue/.gdi/.ccd file rather than the disc itself, so copies with different \
                             file names weren't spotted as duplicates. Recheck them once to fix this. It \
                             reads every track, so it can take a while for large collections."
                        ));
                        if ui
                            .add_enabled(!self.busy(), egui::Button::new(RichText::new("Recheck disc games").strong()))
                            .clicked()
                        {
                            self.clean.results = None;
                            self.start_disc_recheck(ui.ctx());
                        }
                    });
                    ui.add_space(10.0);
                }
                card(ui, |ui| {
                    section(ui, "File names");
                    ui.label(
                        "Rename ROMs to their official names, looked up by checksum in the No-Intro \
                         lists, and remove numbering like '003 - '. Also moves games filed under the \
                         wrong system (e.g. Game Gear games saved as .sms). Disc-based and arcade \
                         systems aren't renamed.",
                    );
                    if ui
                        .add_enabled(!self.busy(), egui::Button::new(RichText::new("Find names to fix").strong()))
                        .clicked()
                    {
                        self.clean.results = None;
                        self.start_rename(ui.ctx(), false);
                    }
                });
                ui.add_space(10.0);
                card(ui, |ui| {
                    section(ui, "Systems");
                    if systems_picker(ui, "clean-systems", &mut self.clean.systems, &self.data.stats) {
                        self.clean.results = None;
                    }
                    if filter_editor(ui, "clean", &mut self.clean.spec, &self.data.genres, &self.data.tags) {
                        self.clean.results = None;
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Byte-identical duplicates are always included.").weak());
                    if ui.button(RichText::new("Find ROMs to remove").strong()).clicked() {
                        let (_, rejected) = filters::apply(
                            self.data.roms.iter().filter(|r| self.clean.systems.is_empty() || self.clean.systems.contains(&r.system)).cloned().collect(),
                            &self.clean.spec,
                        );
                        self.clean.results = Some(rejected);
                    }
                });
            });
        });
    }

    fn clean_results(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        if let Some(rep) = &self.clean.rename {
            if rep.changes.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "All {} cartridge ROMs already have good names.",
                        rep.checked
                    ))
                    .strong(),
                );
                if ui.button("Close").clicked() {
                    self.clean.rename = None;
                }
                return;
            }
            let moved = rep
                .changes
                .iter()
                .filter(|c| c.new_system != c.rom.system)
                .count();
            ui.label(
                RichText::new(format!("{} name(s) to fix", rep.changes.len()))
                    .strong()
                    .size(crate::themes::fs(15.0)),
            );
            if moved > 0 {
                ui.label(format!("{moved} of them move to a different system"));
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.busy(),
                        egui::Button::new(RichText::new("Rename them").strong()),
                    )
                    .clicked()
                {
                    self.start_rename(ui.ctx(), true);
                }
                if ui.button("Cancel").clicked() {
                    self.clean.rename = None;
                }
            });
            ui.separator();
            if let Some(rep) = &self.clean.rename {
                egui::Grid::new("rename-list")
                    .striped(true)
                    .num_columns(1)
                    .show(ui, |ui| {
                        for c in rep.changes.iter().take(3000) {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&c.rom.stem).weak());
                                let arrow = if c.new_system != c.rom.system {
                                    format!(
                                        "> {}  (to {})",
                                        c.new_stem,
                                        systems::name_of(&c.new_system)
                                    )
                                } else {
                                    format!("> {}", c.new_stem)
                                };
                                ui.label(arrow);
                            });
                            ui.end_row();
                        }
                    });
            }
            return;
        }
        let Some(results) = &self.clean.results else {
            ui.label(RichText::new("Choose what to remove, then press Find.").weak());
            return;
        };
        if results.is_empty() {
            ui.label(
                RichText::new("Nothing to remove — your library already matches these rules.")
                    .strong(),
            );
            return;
        }
        let bytes: u64 = results.iter().map(|r| r.rom.total_size).sum();
        ui.label(
            RichText::new(format!(
                "{} ROM(s) to remove · {}",
                results.len(),
                human_size(bytes)
            ))
            .strong()
            .size(crate::themes::fs(15.0)),
        );
        let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
        for r in results {
            *by_reason
                .entry(r.reason.split(':').next().unwrap_or("").to_string())
                .or_default() += 1;
        }
        ui.label(
            by_reason
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(" · "),
        );
        ui.add_space(6.0);
        if ui
            .add_enabled(
                !self.busy(),
                egui::Button::new(
                    RichText::new(format!("Move {} to trash", results.len())).strong(),
                ),
            )
            .clicked()
        {
            self.start_trash(ui.ctx());
            return;
        }
        ui.separator();
        let results = self.clean.results.as_ref().unwrap();
        egui::Grid::new("clean-list")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                for r in results.iter().take(3000) {
                    ui.label(&r.rom.stem)
                        .on_hover_text(systems::name_of(&r.rom.system));
                    ui.label(RichText::new(&r.reason).weak());
                    ui.end_row();
                }
            });
        if results.len() > 3000 {
            ui.label(format!("…and {} more", results.len() - 3000));
        }
    }

    fn start_scummvm_ids(&mut self, ctx: &egui::Context) {
        let Some(root) = self.library_root() else {
            return;
        };
        self.job = Some(Job::spawn("Finding ScummVM game IDs", ctx, move |rep| {
            let fetch = tabularium_core::http::default_fetcher();
            match Library::open(&root).and_then(|lib| {
                tabularium_core::metadata::identify_scummvm(&lib, Some(&fetch), rep)
            }) {
                Ok((found, missing)) => JobOutput::ScummvmIds(found, missing),
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }

    fn start_disc_recheck(&mut self, ctx: &egui::Context) {
        let Some(root) = self.library_root() else {
            return;
        };
        self.job = Some(Job::spawn(
            "Rechecking disc games",
            ctx,
            move |rep| match Library::open(&root).and_then(|lib| lib.recheck_disc_games(rep)) {
                Ok(n) => JobOutput::DiscsRechecked(n),
                Err(e) => JobOutput::Error(e.to_string()),
            },
        ));
    }

    fn start_rename(&mut self, ctx: &egui::Context, apply: bool) {
        let Some(root) = self.library_root() else {
            return;
        };
        let title = if apply {
            "Renaming"
        } else {
            "Looking up official names"
        };
        self.job = Some(Job::spawn(title, ctx, move |rep| {
            let lib = match Library::open(&root) {
                Ok(l) => l,
                Err(e) => return JobOutput::Error(e.to_string()),
            };
            let mut dats = tabularium_core::dats::Dats::new(
                tabularium_core::metadata::cache_dir(&lib),
                tabularium_core::http::default_fetcher(),
            );
            match tabularium_core::rename::fix_library(&lib, Some(&mut dats), true, apply, rep) {
                Ok(report) => JobOutput::Rename {
                    report,
                    applied: apply,
                },
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }

    fn start_trash(&mut self, ctx: &egui::Context) {
        let Some(root) = self.library_root() else {
            return;
        };
        let Some(results) = self.clean.results.clone() else {
            return;
        };
        self.job = Some(Job::spawn("Moving to trash", ctx, move |rep| {
            let lib = match Library::open(&root) {
                Ok(l) => l,
                Err(e) => return JobOutput::Error(e.to_string()),
            };
            let total = results.len();
            let mut n = 0;
            for (i, r) in results.iter().enumerate() {
                if rep.cancelled() {
                    break;
                }
                rep.progress(i, total, &r.rom.stem);
                match lib.trash(&r.rom, &r.reason) {
                    Ok(()) => n += 1,
                    Err(e) => rep.log(format!("{}: {e}", r.rom.stem)),
                }
            }
            JobOutput::Trashed(n)
        }));
    }
}
