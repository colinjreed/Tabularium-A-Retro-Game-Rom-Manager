//! Application state and the main window layout.

use crate::jobs::{Job, JobOutput};
use crate::pages;
use crate::settings::Settings;
use crate::themes::{self, Icon};
use crate::widgets;
use egui::{Color32, RichText, Ui};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use tabularium_core::human_size;
use tabularium_core::library::{Library, Rom, SystemStats};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Import,
    Clean,
    Artwork,
    Bios,
    Collections,
    Themes,
    Settings,
}

impl Page {
    const ALL: [Page; 8] = [
        Page::Library,
        Page::Import,
        Page::Clean,
        Page::Artwork,
        Page::Bios,
        Page::Collections,
        Page::Themes,
        Page::Settings,
    ];

    /// (pixel icon, emoji, name)
    fn label(self) -> (Icon, &'static str, &'static str) {
        match self {
            Page::Library => (Icon::Library, "📚", "Library"),
            Page::Import => (Icon::Import, "📥", "Import"),
            Page::Clean => (Icon::Clean, "✂", "Clean up"),
            Page::Artwork => (Icon::Artwork, "🖼", "Artwork"),
            Page::Bios => (Icon::Bios, "🔑", "BIOS"),
            Page::Collections => (Icon::Sd, "💾", "SD cards"),
            Page::Themes => (Icon::Themes, "🎨", "Themes"),
            Page::Settings => (Icon::Settings, "⚙", "Settings"),
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
    pub tags: Vec<(String, u64)>,
    pub genres: Vec<(String, u64)>,
    /// SD card (collection) names, for "Add to card".
    pub cards: Vec<String>,
    /// Bumped on every reload so cached views refresh.
    pub version: u64,
}

pub use crate::pages::library::LibraryView;

pub struct RomApp {
    /// The app's normal dark and light styles (the Standard theme).
    pub base_styles: (egui::Style, egui::Style),
    pub theme_tex: crate::themes::Textures,
    /// The theme's start-up screen, while it's showing.
    pub boot: Option<themes::Boot>,
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
        crate::themes::install_fonts(&cc.egui_ctx);
        let tweak = |mut st: egui::Style| {
            st.spacing.item_spacing = egui::vec2(8.0, 6.0);
            st.spacing.button_padding = egui::vec2(10.0, 4.0);
            st.interaction.selectable_labels = false;
            st
        };
        let base_styles = (
            tweak((*cc.egui_ctx.style_of(egui::Theme::Dark)).clone()),
            tweak((*cc.egui_ctx.style_of(egui::Theme::Light)).clone()),
        );

        let mut settings = Settings::load(cc.storage);
        tabularium_core::systems::set_name_region(settings.system_names);
        settings.screenscraper = settings.screenscraper.clone().with_env_fallback();
        crate::themes::apply(&cc.egui_ctx, settings.theme, &base_styles);
        let mut app = RomApp {
            base_styles,
            theme_tex: Default::default(),
            boot: None,
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
        if app.settings.boot_screen {
            let roms = app.data.stats.values().map(|s| s.count).sum();
            app.boot = themes::Boot::new(app.settings.theme, roms, app.data.stats.len());
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
        let res = (|| -> tabularium_core::Result<LibData> {
            Ok(LibData {
                roms: lib.active()?,
                trashed: lib.roms(&[], "trashed")?,
                stats: lib.stats()?,
                with_art: lib.roms_with_media("boxart")?,
                tags: lib.all_tags()?,
                genres: lib.all_genres()?,
                cards: tabularium_core::collection::list(lib),
                version: 0,
            })
        })();
        match res {
            Ok(mut d) => {
                d.version = self.data.version + 1;
                self.data = d;
            }
            Err(e) => self.notify_err(e.to_string()),
        }
        self.view.invalidate();
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
                if report.history.as_ref().is_some_and(|h| h.updated > 0) {
                    self.reload();
                }
                self.cards.export_result = Some((report, dest, dry_run));
            }
            Some(JobOutput::History(h)) => {
                self.reload();
                let mut msg = if h.found == 0 {
                    "No play history found on that card (EmulationStation-based firmware and Onion OS keep it).".to_string()
                } else {
                    format!(
                        "{} game(s) played on the card; {} newly marked as played.",
                        h.found, h.updated
                    )
                };
                if let Some(n) = h.notes.first() {
                    msg += &format!(" {n}");
                }
                self.notify(msg);
            }
            Some(JobOutput::Genres(n)) => {
                self.reload();
                self.notify(format!("Found genres for {n} game(s)."));
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
            if themes::has_pixel_icons() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                let col = ui.visuals().strong_text_color();
                themes::paint_icon(ui.painter(), r, Icon::Logo, col);
            } else {
                ui.label(RichText::new("🎮").size(22.0));
            }
            ui.label(themes::head("Tabularium", 19.0).strong());
        });
        ui.add_space(14.0);
        for p in Page::ALL {
            let (icon, emoji, name) = p.label();
            if themes::nav_item(ui, self.page == p, icon, emoji, name).clicked() {
                self.page = p;
            }
        }
        // Line art in the empty space above the library footer.
        let rest = ui.available_rect_before_wrap();
        let footer = if self.lib.is_some() { 56.0 } else { 8.0 };
        if rest.height() > footer + 80.0 {
            themes::paint_side_art(ui.painter(), rest.with_max_y(rest.bottom() - footer));
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
                // The count goes beside the bar, not on it: some themes'
                // bar colours make text on the bar unreadable.
                ui.add(egui::ProgressBar::new(frac).desired_width(220.0));
                if total > 0 {
                    ui.label(format!("{done} / {total}"));
                }
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
            ui.horizontal(|ui| {
                ui.label(RichText::new(themes::ready_text()).weak());
                if self.settings.animations {
                    if let Some(lit) = themes::cursor_lit(ui.ctx()) {
                        let h = ui.text_style_height(&egui::TextStyle::Body);
                        let (r, _) = ui.allocate_exact_size(
                            egui::vec2(h * 0.55, h * 0.9),
                            egui::Sense::hover(),
                        );
                        if lit {
                            ui.painter()
                                .rect_filled(r, 0, ui.visuals().weak_text_color());
                        }
                    }
                }
            });
        }
        ui.add_space(4.0);
    }

    fn welcome(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(RichText::new("🎮").size(56.0));
            ui.heading(crate::themes::head("Welcome to Tabularium", 26.0).strong());
            ui.add_space(6.0);
            ui.label(
                "Organise your ROMs into one library, clean out duplicates, fetch artwork, \
                      check BIOS files and build SD cards for your handhelds.",
            );
            ui.add_space(24.0);
        });
        ui.columns(2, |cols| {
            widgets::card(&mut cols[0], |ui| {
                ui.label(
                    RichText::new("Start a new library")
                        .strong()
                        .size(crate::themes::fs(16.0)),
                );
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
                        .size(crate::themes::fs(16.0)),
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
                                "{} isn't a Tabularium library (no library.db).",
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
                            "{} isn't a Tabularium library (no library.db).",
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

    /// Show / set the ScummVM game ID (written as `<Game>.scummvm` in the folder).
    pub(crate) fn scummvm_id_editor(&mut self, ui: &mut Ui, rom: &Rom) {
        let Some(lib) = &self.lib else { return };
        let dir = lib.abspath(&rom.path);
        if !dir.is_dir() {
            return;
        }
        let current = tabularium_core::folder_games::scummvm_id(&dir);
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
            match tabularium_core::folder_games::write_scummvm_id(&dir, &rom.stem, &id) {
                Ok(()) => {
                    self.notice =
                        Some((format!("Saved ScummVM ID '{id}' for {}.", rom.stem), false))
                }
                Err(e) => self.notice = Some((e.to_string(), true)),
            }
        }
        ui.data_mut(|d| d.insert_temp(key, text));
    }

    pub(crate) fn trash_now(&mut self, rom: Rom, reason: &str) {
        if let Some(lib) = &self.lib {
            match lib.trash(&rom, reason) {
                Ok(()) => {
                    self.view.selected.clear();
                    self.reload();
                    self.notify(format!("Moved {} to the trash.", rom.stem));
                }
                Err(e) => self.notify_err(e.to_string()),
            }
        }
    }

    pub(crate) fn restore_roms(&mut self, roms: Vec<Rom>) {
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
        crate::themes::paint_background(ui.ctx(), &mut self.theme_tex);
        let style = ui.style().clone();
        egui::Panel::bottom("status")
            .frame(crate::themes::panel_frame(crate::themes::Area::Bar, &style))
            .show(ui, |ui| {
                crate::themes::paint_panel(ui, crate::themes::Area::Bar, &mut self.theme_tex);
                self.status_bar(ui)
            });
        egui::Panel::left("nav")
            .frame(crate::themes::panel_frame(
                crate::themes::Area::Side,
                &style,
            ))
            .resizable(false)
            .exact_size(180.0)
            .show(ui, |ui| {
                crate::themes::paint_panel(ui, crate::themes::Area::Side, &mut self.theme_tex);
                self.nav(ui)
            });
        egui::CentralPanel::default()
            .frame(crate::themes::panel_frame(
                crate::themes::Area::Main,
                &style,
            ))
            .show(ui, |ui| {
                crate::themes::paint_panel(ui, crate::themes::Area::Main, &mut self.theme_tex);
                if self.page == Page::Themes {
                    egui::ScrollArea::vertical().show(ui, |ui| self.themes_page(ui));
                    return;
                }
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
                    Page::Themes => {}
                    Page::Settings => {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| self.settings_page(ui))
                            .inner
                    }
                }
            });
        self.empty_trash_dialog(ui);
        if self.settings.animations {
            themes::paint_motion(ui.ctx(), self.settings.crt_effect);
        }
        if let Some(boot) = &mut self.boot {
            match boot.show(ui.ctx()) {
                themes::BootStep::Running => {}
                themes::BootStep::Finished => self.boot = None,
            }
        }
        crate::themes::paint_overlay(
            ui.ctx(),
            &mut self.theme_tex,
            self.settings.crt_effect,
            self.settings.lcd_effect,
        );
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.save(storage);
    }
}
