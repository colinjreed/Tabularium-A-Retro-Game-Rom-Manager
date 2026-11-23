//! Application state and the main window layout.

use crate::jobs::{Job, JobOutput};
use crate::pages;
use crate::settings::Settings;
use crate::widgets::{self, page_title};
use egui::{Color32, RichText, Ui};
use rom_manager_core::library::{Library, Rom, SystemStats};
use rom_manager_core::{human_size, systems};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Import,
    Clean,
    Artwork,
    Bios,
    Collections,
    Settings,
}

impl Page {
    const ALL: [Page; 7] = [
        Page::Library,
        Page::Import,
        Page::Clean,
        Page::Artwork,
        Page::Bios,
        Page::Collections,
        Page::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Page::Library => "📚  Library",
            Page::Import => "📥  Import",
            Page::Clean => "✂  Clean up",
            Page::Artwork => "🖼  Artwork",
            Page::Bios => "🔑  BIOS",
            Page::Collections => "💾  SD cards",
            Page::Settings => "⚙  Settings",
        }
    }
}

/// Cached library contents for fast UI.
#[derive(Default)]
pub struct LibData {
    pub roms: Vec<Rom>,
    pub trashed: Vec<Rom>,
    pub stats: BTreeMap<String, SystemStats>,
    pub with_art: HashSet<i64>,
}

#[derive(Default)]
pub struct LibraryView {
    pub system: Option<String>,
    pub search: String,
    pub show_trash: bool,
    pub selected: Option<i64>,
    filtered: Vec<usize>,
    filter_key: Option<(Option<String>, String, bool, usize)>,
}

pub struct RomApp {
    pub settings: Settings,
    pub lib: Option<Library>,
    pub data: LibData,
    pub page: Page,
    pub job: Option<Job>,
    pub notice: Option<(String, bool)>,
    pub view: LibraryView,
    pub import: pages::import::ImportPage,
    pub clean: pages::clean::CleanPage,
    pub artwork: pages::artwork::ArtworkPage,
    pub bios: pages::bios::BiosPage,
    pub cards: pages::collections::CollectionsPage,
    new_library_name: String,
}

impl RomApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open_path: Option<PathBuf>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.interaction.selectable_labels = false;
        cc.egui_ctx.set_global_style(style);

        let mut settings = Settings::load(cc.storage);
        settings.screenscraper = settings.screenscraper.clone().with_env_fallback();
        let mut app = RomApp {
            settings,
            lib: None,
            data: LibData::default(),
            page: Page::Library,
            job: None,
            notice: None,
            view: LibraryView::default(),
            import: Default::default(),
            clean: Default::default(),
            artwork: Default::default(),
            bios: Default::default(),
            cards: Default::default(),
            new_library_name: "ROM Library".into(),
        };
        if let Some(p) = open_path
            .filter(|p| Library::is_library(p))
            .or_else(|| app.settings.library.clone())
        {
            if Library::is_library(&p) {
                app.open_library(p);
            }
        }
        app
    }

    pub fn open_library(&mut self, path: PathBuf) {
        match Library::open(&path) {
            Ok(lib) => {
                self.settings.remember_library(lib.root.clone());
                self.lib = Some(lib);
                self.view = LibraryView::default();
                self.cards = Default::default();
                self.bios = Default::default();
                self.reload();
            }
            Err(e) => self.notify_err(e.to_string()),
        }
    }

    pub fn create_library(&mut self, path: PathBuf) {
        match Library::create(&path) {
            Ok(_) => self.open_library(path),
            Err(e) => self.notify_err(e.to_string()),
        }
    }

    /// Re-read the library index after something changed.
    pub fn reload(&mut self) {
        let Some(lib) = &self.lib else { return };
        let res = (|| -> rom_manager_core::Result<LibData> {
            Ok(LibData {
                roms: lib.active()?,
                trashed: lib.roms(&[], "trashed")?,
                stats: lib.stats()?,
                with_art: lib.roms_with_media("boxart")?,
            })
        })();
        match res {
            Ok(d) => self.data = d,
            Err(e) => self.notify_err(e.to_string()),
        }
        self.view.filter_key = None;
        self.cards.mark_dirty();
        self.bios.lib_status = None;
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notice = Some((msg.into(), false));
    }

    pub fn notify_err(&mut self, msg: impl Into<String>) {
        self.notice = Some((msg.into(), true));
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn library_root(&self) -> Option<PathBuf> {
        self.lib.as_ref().map(|l| l.root.clone())
    }

    fn poll_job(&mut self) {
        let Some(job) = &self.job else { return };
        if !job.is_finished() {
            return;
        }
        let out = job.take_output();
        self.job = None;
        match out {
            Some(JobOutput::Import { report, dry_run }) => {
                if !dry_run {
                    self.reload();
                }
                self.import.result = Some((report, dry_run));
            }
            Some(JobOutput::Scrape(r)) => {
                self.reload();
                self.artwork.result = Some(r);
            }
            Some(JobOutput::Trashed(n)) => {
                self.reload();
                self.clean.results = None;
                self.notify(format!(
                    "Moved {n} ROM(s) to the trash. You can restore them from Library > Trash."
                ));
            }
            Some(JobOutput::Export {
                report,
                dest,
                dry_run,
            }) => {
                self.cards.export_result = Some((report, dest, dry_run));
            }
            Some(JobOutput::Rename { report, applied }) => {
                if applied {
                    self.reload();
                    let mut msg = format!("Renamed {} ROM(s).", report.applied);
                    if !report.conflicts.is_empty() {
                        msg += &format!(
                            " {} skipped because the name was taken.",
                            report.conflicts.len()
                        );
                    }
                    self.notify(msg);
                    self.clean.rename = None;
                } else {
                    self.clean.rename = Some(report);
                }
            }
            Some(JobOutput::BiosImport(r)) => {
                self.bios.import_result = Some(r);
                self.bios.lib_status = None;
            }
            Some(JobOutput::Error(e)) => self.notify_err(e),
            None => {}
        }
    }

    // ------------------------------------------------------------------ shell

    fn nav(&mut self, ui: &mut Ui) {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("🎮").size(22.0));
            ui.label(RichText::new("ROM Manager").size(19.0).strong());
        });
        ui.add_space(14.0);
        for p in Page::ALL {
            let selected = self.page == p;
            let resp = ui.add(
                egui::Button::selectable(selected, RichText::new(p.label()).size(15.0))
                    .min_size(egui::vec2(ui.available_width(), 32.0)),
            );
            if resp.clicked() {
                self.page = p;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(8.0);
            if let Some(lib) = &self.lib {
                let total: u64 = self.data.stats.values().map(|s| s.count).sum();
                let bytes: u64 = self.data.stats.values().map(|s| s.bytes).sum();
                ui.label(RichText::new(format!("{total} ROMs · {}", human_size(bytes))).weak());
                ui.add(
                    egui::Label::new(RichText::new(lib.root.display().to_string()).weak().small())
                        .truncate(),
                )
                .on_hover_text(lib.root.display().to_string());
            }
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        if let Some(job) = &self.job {
            let (done, total, current) = {
                let s = job.state.lock().unwrap();
                (s.done, s.total, s.current.clone())
            };
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(&job.title).strong());
                let frac = if total > 0 {
                    done as f32 / total as f32
                } else {
                    0.0
                };
                let text = if total > 0 {
                    format!("{done} / {total}")
                } else {
                    "Working…".into()
                };
                ui.add(egui::ProgressBar::new(frac).text(text).desired_width(260.0));
                ui.label(RichText::new(current).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if job.cancel_requested() {
                        ui.label("Stopping…");
                    } else if ui.button("Cancel").clicked() {
                        job.request_cancel();
                    }
                });
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
        } else if let Some((msg, is_err)) = self.notice.clone() {
            ui.horizontal(|ui| {
                let color = if is_err {
                    widgets::err_color(ui)
                } else {
                    widgets::ok_color(ui)
                };
                ui.colored_label(color, if is_err { "⚠" } else { "✔" });
                ui.label(msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("×").clicked() {
                        self.notice = None;
                    }
                });
            });
        } else {
            ui.label(RichText::new("Ready").weak());
        }
        ui.add_space(4.0);
    }

    fn welcome(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(RichText::new("🎮").size(56.0));
            ui.heading(RichText::new("Welcome to ROM Manager").size(26.0).strong());
            ui.add_space(6.0);
            ui.label(
                "Organise your ROMs into one library, clean out duplicates, fetch artwork, \
                      check BIOS files and build SD cards for your handhelds.",
            );
            ui.add_space(24.0);
        });
        ui.columns(2, |cols| {
            widgets::card(&mut cols[0], |ui| {
                ui.label(RichText::new("Start a new library").strong().size(16.0));
                ui.label(
                    "Pick a folder with plenty of space. ROMs you import are copied into it, \
                          sorted by system.",
                );
                ui.horizontal(|ui| {
                    ui.label("Folder name:");
                    ui.text_edit_singleline(&mut self.new_library_name);
                });
                if let Some(p) = widgets::path_entry(ui, "new-lib", "Create here") {
                    self.create_library(p);
                }
                if ui.button("Choose where to create it…").clicked() {
                    if let Some(parent) = rfd::FileDialog::new()
                        .set_title("Where should the library go?")
                        .pick_folder()
                    {
                        let name = self.new_library_name.trim();
                        let path = if name.is_empty() {
                            parent
                        } else {
                            parent.join(name)
                        };
                        self.create_library(path);
                    }
                }
            });
            widgets::card(&mut cols[1], |ui| {
                ui.label(
                    RichText::new("Open an existing library")
                        .strong()
                        .size(16.0),
                );
                ui.label(
                    "A folder containing library.db (libraries from the earlier command-line \
                          version work too).",
                );
                if ui.button("Open library folder…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        if Library::is_library(&p) {
                            self.open_library(p);
                        } else {
                            self.notify_err(format!(
                                "{} isn't a ROM Manager library (no library.db).",
                                p.display()
                            ));
                        }
                    }
                }
                if let Some(p) = widgets::path_entry(ui, "open-lib", "Open") {
                    if Library::is_library(&p) {
                        self.open_library(p);
                    } else {
                        self.notify_err(format!(
                            "{} isn't a ROM Manager library (no library.db).",
                            p.display()
                        ));
                    }
                }
                let recent: Vec<PathBuf> = self
                    .settings
                    .recent_libraries
                    .iter()
                    .filter(|p| Library::is_library(p))
                    .cloned()
                    .collect();
                if !recent.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Recent").weak());
                    for p in recent {
                        if ui.link(p.display().to_string()).clicked() {
                            self.open_library(p);
                        }
                    }
                }
            });
        });
    }

    // ------------------------------------------------------------------ library page

    fn library_page(&mut self, ui: &mut Ui) {
        let total: u64 = self.data.stats.values().map(|s| s.count).sum();
        if total == 0 && self.data.trashed.is_empty() {
            page_title(ui, "Library", "Your library is empty.");
            widgets::card(ui, |ui| {
                ui.label(
                    "Start by importing the folders where your ROMs are now. They'll be copied in \
                          and sorted by system; your originals are left alone.",
                );
                if ui.button("Go to Import").clicked() {
                    self.page = Page::Import;
                }
            });
            return;
        }

        egui::Panel::left("systems-list")
            .resizable(true)
            .default_size(210.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let all_sel = self.view.system.is_none() && !self.view.show_trash;
                    if ui
                        .selectable_label(all_sel, format!("All systems  ({total})"))
                        .clicked()
                    {
                        self.view.system = None;
                        self.view.show_trash = false;
                    }
                    ui.separator();
                    for (sid, st) in &self.data.stats {
                        if st.count == 0 {
                            continue;
                        }
                        let sel = self.view.system.as_deref() == Some(sid.as_str())
                            && !self.view.show_trash;
                        let label = format!("{}  ({})", systems::name_of(sid), st.count);
                        if ui
                            .selectable_label(sel, label)
                            .on_hover_text(human_size(st.bytes))
                            .clicked()
                        {
                            self.view.system = Some(sid.clone());
                            self.view.show_trash = false;
                        }
                    }
                    ui.separator();
                    let n = self.data.trashed.len();
                    if ui
                        .selectable_label(self.view.show_trash, format!("🗑 Trash  ({n})"))
                        .clicked()
                    {
                        self.view.show_trash = true;
                        self.view.system = None;
                    }
                });
            });

        let selected = self.view.selected.and_then(|id| {
            self.data
                .roms
                .iter()
                .chain(self.data.trashed.iter())
                .find(|r| r.id == id)
                .cloned()
        });
        if let Some(rom) = &selected {
            egui::Panel::right("rom-detail")
                .resizable(true)
                .default_size(300.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.rom_detail(ui, rom));
                });
        }

        egui::CentralPanel::default().show(ui, |ui| self.rom_table(ui));
    }

    fn rom_table(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.view.search)
                    .hint_text("Search titles…")
                    .desired_width(280.0),
            );
            if self.view.show_trash && !self.data.trashed.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Empty trash…").clicked() {
                        self.clean.confirm_empty_trash = true;
                    }
                    if ui.button("Restore all").clicked() {
                        self.restore_roms(self.data.trashed.clone());
                    }
                });
            }
        });
        let list = if self.view.show_trash {
            &self.data.trashed
        } else {
            &self.data.roms
        };
        let key = (
            self.view.system.clone(),
            self.view.search.to_lowercase(),
            self.view.show_trash,
            list.len(),
        );
        if self.view.filter_key.as_ref() != Some(&key) {
            self.view.filtered = list
                .iter()
                .enumerate()
                .filter(|(_, r)| key.0.as_ref().is_none_or(|s| &r.system == s))
                .filter(|(_, r)| {
                    key.1.is_empty()
                        || r.stem.to_lowercase().contains(&key.1)
                        || r.alt_name
                            .as_ref()
                            .is_some_and(|a| a.to_lowercase().contains(&key.1))
                })
                .map(|(i, _)| i)
                .collect();
            self.view.filter_key = Some(key);
        }
        ui.label(RichText::new(format!("{} ROMs", self.view.filtered.len())).weak());
        let show_trash = self.view.show_trash;
        let rows = self.view.filtered.clone();
        let mut clicked = None;
        let text_h = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .auto_shrink(false)
            .column(egui_extras::Column::remainder().at_least(160.0).clip(true))
            .column(
                egui_extras::Column::initial(120.0)
                    .at_least(60.0)
                    .clip(true)
                    .resizable(true),
            )
            .column(
                egui_extras::Column::initial(100.0)
                    .at_least(50.0)
                    .clip(true)
                    .resizable(true),
            )
            .column(egui_extras::Column::exact(70.0))
            .column(egui_extras::Column::exact(32.0))
            .header(22.0, |mut h| {
                h.col(|ui| {
                    ui.strong("Title");
                });
                h.col(|ui| {
                    ui.strong("System");
                });
                h.col(|ui| {
                    ui.strong(if show_trash { "Why removed" } else { "Region" });
                });
                h.col(|ui| {
                    ui.strong("Size");
                });
                h.col(|ui| {
                    ui.strong("Art");
                });
            })
            .body(|body| {
                body.rows(text_h, rows.len(), |mut row| {
                    let list = if show_trash {
                        &self.data.trashed
                    } else {
                        &self.data.roms
                    };
                    let r = &list[rows[row.index()]];
                    row.set_selected(self.view.selected == Some(r.id));
                    row.col(|ui| {
                        ui.label(r.display_name()).on_hover_text(r.file_name());
                    });
                    row.col(|ui| {
                        ui.label(systems::get(&r.system).map(|s| s.name).unwrap_or(&r.system));
                    });
                    row.col(|ui| {
                        if show_trash {
                            ui.label(r.status_reason.clone().unwrap_or_default());
                        } else {
                            ui.label(r.regions.join(", "));
                        }
                    });
                    row.col(|ui| {
                        ui.label(human_size(r.total_size));
                    });
                    row.col(|ui| {
                        if self.data.with_art.contains(&r.id) {
                            ui.label("✔");
                        }
                    });
                    if row.response().clicked() {
                        clicked = Some(r.id);
                    }
                });
            });
        if let Some(id) = clicked {
            self.view.selected = if self.view.selected == Some(id) {
                None
            } else {
                Some(id)
            };
        }
    }

    fn rom_detail(&mut self, ui: &mut Ui, rom: &Rom) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&rom.title).strong().size(17.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.small_button("×").clicked() {
                    self.view.selected = None;
                }
            });
        });
        if let Some(lib) = &self.lib {
            if let Ok(media) = lib.media(rom.id) {
                for kind in ["boxart", "mix", "snap", "title"] {
                    if let Some(p) = media.get(kind) {
                        ui.add(
                            egui::Image::new(format!("file://{}", p.display()))
                                .max_width(ui.available_width().min(280.0))
                                .max_height(320.0)
                                .corner_radius(4.0),
                        );
                        break;
                    }
                }
            }
        }
        egui::Grid::new("detail")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                let mut kv = |k: &str, v: String| {
                    ui.label(RichText::new(k).weak());
                    ui.label(v);
                    ui.end_row();
                };
                kv("System", systems::name_of(&rom.system).to_string());
                kv("File", rom.file_name().to_string());
                if rom.files.len() > 1 {
                    kv("Also", format!("{} more file(s)", rom.files.len() - 1));
                }
                kv(
                    "Regions",
                    if rom.regions.is_empty() {
                        "—".into()
                    } else {
                        rom.regions.join(", ")
                    },
                );
                if !rom.languages.is_empty() {
                    kv("Languages", rom.languages.join(", "));
                }
                if rom.revision.iter().any(|v| *v > 0) {
                    kv(
                        "Revision",
                        rom.revision
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join("."),
                    );
                }
                if let Some(d) = rom.disc {
                    kv("Disc", d.to_string());
                }
                if !rom.flags.is_empty() {
                    kv(
                        "Flags",
                        rom.flags.iter().cloned().collect::<Vec<_>>().join(", "),
                    );
                }
                kv("Size", human_size(rom.total_size));
                if let Some(c) = &rom.crc {
                    kv("CRC32", c.to_uppercase());
                }
                if let Some(r) = &rom.status_reason {
                    kv("Removed", r.clone());
                }
            });
        if rom.system == "scummvm" && rom.status == "active" {
            self.scummvm_id_editor(ui, rom);
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if let Some(lib) = &self.lib {
                let path = lib.abspath(&rom.path);
                if ui.button("Show in folder").clicked() {
                    let _ = open::that(path.parent().unwrap_or(&path));
                }
            }
            if rom.status == "active" {
                if ui.button("Move to trash").clicked() {
                    self.trash_now(rom.clone(), "removed by hand");
                }
            } else if ui.button("Restore").clicked() {
                self.restore_roms(vec![rom.clone()]);
            }
        });
    }

    /// Show / set the ScummVM game ID (written as `<Game>.scummvm` in the folder).
    fn scummvm_id_editor(&mut self, ui: &mut Ui, rom: &Rom) {
        let Some(lib) = &self.lib else { return };
        let dir = lib.abspath(&rom.path);
        if !dir.is_dir() {
            return;
        }
        let current = rom_manager_core::folder_games::scummvm_id(&dir);
        let key = egui::Id::unique(("scummvm-id", rom.id));
        let mut text: String = ui
            .data_mut(|d| d.get_temp(key))
            .unwrap_or_else(|| current.clone().unwrap_or_default());
        ui.add_space(8.0);
        ui.label(RichText::new("ScummVM game ID").strong());
        if current.is_none() {
            ui.colored_label(
                widgets::warn_color(ui),
                "Not identified yet: firmware won't know how to launch this game.",
            );
        }
        let mut save = false;
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .hint_text("e.g. monkey2")
                    .desired_width(140.0),
            );
            save = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            save |= ui
                .add_enabled(!text.trim().is_empty(), egui::Button::new("Save"))
                .clicked();
        });
        ui.label(
            RichText::new(
                "Find IDs at scummvm.org/compatibility (the short name, like 'monkey2').",
            )
            .weak()
            .small(),
        );
        if save && !text.trim().is_empty() {
            let id = text.trim().to_string();
            match rom_manager_core::folder_games::write_scummvm_id(&dir, &rom.stem, &id) {
                Ok(()) => {
                    self.notice =
                        Some((format!("Saved ScummVM ID '{id}' for {}.", rom.stem), false))
                }
                Err(e) => self.notice = Some((e.to_string(), true)),
            }
        }
        ui.data_mut(|d| d.insert_temp(key, text));
    }

    fn trash_now(&mut self, rom: Rom, reason: &str) {
        if let Some(lib) = &self.lib {
            match lib.trash(&rom, reason) {
                Ok(()) => {
                    self.view.selected = None;
                    self.reload();
                    self.notify(format!("Moved {} to the trash.", rom.stem));
                }
                Err(e) => self.notify_err(e.to_string()),
            }
        }
    }

    fn restore_roms(&mut self, roms: Vec<Rom>) {
        let Some(lib) = &self.lib else { return };
        let mut errors = vec![];
        let mut n = 0;
        for r in &roms {
            match lib.restore(r) {
                Ok(()) => n += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }
        self.reload();
        if errors.is_empty() {
            self.notify(format!("Restored {n} ROM(s)."));
        } else {
            self.notify_err(format!(
                "Restored {n}; {} failed: {}",
                errors.len(),
                errors[0]
            ));
        }
    }

    fn empty_trash_dialog(&mut self, ui: &mut Ui) {
        if !self.clean.confirm_empty_trash {
            return;
        }
        let n = self.data.trashed.len();
        let bytes: u64 = self.data.trashed.iter().map(|r| r.total_size).sum();
        let modal = egui::Modal::new(egui::Id::unique("empty-trash")).show(ui.ctx(), |ui| {
            ui.heading("Empty the trash?");
            ui.label(format!(
                "This permanently deletes {n} ROM(s) ({}). It can't be undone.",
                human_size(bytes)
            ));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Delete permanently").color(Color32::WHITE))
                    .on_hover_text("Delete the files from disk")
                    .clicked()
                {
                    if let Some(lib) = &self.lib {
                        match lib.empty_trash() {
                            Ok(k) => self.notice = Some((format!("Deleted {k} ROM(s)."), false)),
                            Err(e) => self.notice = Some((e.to_string(), true)),
                        }
                    }
                    self.clean.confirm_empty_trash = false;
                }
                if ui.button("Cancel").clicked() {
                    self.clean.confirm_empty_trash = false;
                }
            });
        });
        if modal.should_close() {
            self.clean.confirm_empty_trash = false;
        }
        if !self.clean.confirm_empty_trash {
            self.reload();
        }
    }
}

impl eframe::App for RomApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.poll_job();
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(180.0)
            .show(ui, |ui| self.nav(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            if self.lib.is_none() && self.page != Page::Settings {
                egui::ScrollArea::vertical().show(ui, |ui| self.welcome(ui));
                return;
            }
            match self.page {
                Page::Library => self.library_page(ui),
                Page::Import => {
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| self.import_page(ui))
                        .inner
                }
                Page::Clean => self.clean_page(ui),
                Page::Artwork => {
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| self.artwork_page(ui))
                        .inner
                }
                Page::Bios => {
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| self.bios_page(ui))
                        .inner
                }
                Page::Collections => self.collections_page(ui),
                Page::Settings => {
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| self.settings_page(ui))
                        .inner
                }
            }
        });
        self.empty_trash_dialog(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.save(storage);
    }
}
