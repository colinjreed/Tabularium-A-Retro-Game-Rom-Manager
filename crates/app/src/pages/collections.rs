use crate::app::RomApp;
use crate::jobs::{Job, JobOutput};
use crate::widgets::{self, card, filter_editor, page_title, section, systems_picker};
use egui::{RichText, Ui};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tabularium_core::collection::{self, Collection, ExportOptions, ExportReport, Selection};
use tabularium_core::library::Library;
use tabularium_core::naming::DEFAULT_JUNK;
use tabularium_core::scrapers::{art_kind_label, ART_KINDS};
use tabularium_core::{human_size, profiles, systems};

/// Typical card sizes (marketing GB) and the space we plan to use on them.
const CARD_SIZES: &[u64] = &[8, 16, 32, 64, 128, 256, 512, 1000];

fn usable(card_gb: u64) -> u64 {
    // Marketing GB are 10^9 bytes; leave ~6% for firmware, saves and the filesystem.
    (card_gb as f64 * 1e9 * 0.94) as u64
}

#[derive(Default)]
pub struct CollectionsPage {
    names: Option<Vec<String>>,
    pub current: Option<Collection>,
    saved_name: Option<String>,
    pub preview: Option<Selection>,
    recompute_at: Option<f64>,
    custom_size: String,
    pub export_opts: ExportOptions,
    pub export_result: Option<(ExportReport, PathBuf, bool)>,
    pub dest: Option<PathBuf>,
    confirm_delete: bool,
    error: Option<String>,
}

impl CollectionsPage {
    /// Keep the open editor in step when games are added from the Library.
    pub fn note_picked(&mut self, card: &str, ids: &[i64]) {
        if let Some(c) = self.current.as_mut().filter(|c| c.name == card) {
            for id in ids {
                if !c.picked.contains(id) {
                    c.picked.push(*id);
                }
            }
        }
        self.mark_dirty();
    }

    pub fn mark_dirty(&mut self) {
        self.recompute_at = Some(0.0);
    }

    fn touch(&mut self, now: f64) {
        self.recompute_at = Some(now + 0.4);
    }
}

impl RomApp {
    pub fn collections_page(&mut self, ui: &mut Ui) {
        let Some(root) = self.library_root() else {
            return;
        };
        if self.cards.names.is_none() {
            self.cards.names = Some(self.lib.as_ref().map(collection::list).unwrap_or_default());
        }
        let now = ui.input(|i| i.time);

        egui::Panel::left("card-list")
            .resizable(false)
            .exact_size(190.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Your SD cards").strong());
                ui.add_space(4.0);
                if ui.button("➕ New SD card").clicked() {
                    self.new_collection();
                }
                ui.separator();
                let names = self.cards.names.clone().unwrap_or_default();
                if names.is_empty() {
                    ui.label(RichText::new("None yet.").weak());
                }
                for n in names {
                    let sel = self.cards.saved_name.as_deref() == Some(n.as_str());
                    if ui.selectable_label(sel, &n).clicked() && !sel {
                        self.load_collection(&n);
                    }
                }
            });

        if self.cards.current.is_none() {
            egui::CentralPanel::default().show(ui, |ui| {
                page_title(
                    ui,
                    "SD cards",
                    "Make one entry per handheld: which firmware it runs, which systems and games go on it, \
                     and how big the card is. Then copy it to the card in one click.",
                );
                card(ui, |ui| {
                    ui.label("Your library stays untouched; each card gets its own selection, folder layout and artwork.");
                    if ui.button(RichText::new("➕ Set up an SD card").strong()).clicked() {
                        self.new_collection();
                    }
                });
            });
            return;
        }

        // Debounced: save and recompute the preview shortly after edits.
        if let Some(t) = self.cards.recompute_at {
            if now >= t {
                self.cards.recompute_at = None;
                self.save_and_preview();
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(t - now));
            }
        }

        egui::Panel::right("card-preview")
            .resizable(true)
            .default_size(380.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("card-preview-scroll")
                    .show(ui, |ui| self.card_preview(ui, &root));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let changed = self.card_editor(ui);
                if changed {
                    self.cards.touch(now);
                }
            });
        });
        self.delete_dialog(ui);
    }

    fn new_collection(&mut self) {
        let names = self.cards.names.clone().unwrap_or_default();
        let mut n = 1;
        let mut name = "My handheld".to_string();
        while names.contains(&name) {
            n += 1;
            name = format!("My handheld {n}");
        }
        let mut col = Collection {
            name: name.clone(),
            ..Default::default()
        };
        col.filters.exclude_flags = DEFAULT_JUNK.iter().map(|s| s.to_string()).collect();
        col.filters.one_game_one_rom = true;
        self.cards.current = Some(col);
        self.cards.saved_name = None;
        self.cards.export_result = None;
        self.cards.mark_dirty();
    }

    fn load_collection(&mut self, name: &str) {
        let Some(lib) = &self.lib else { return };
        match Collection::load(lib, name) {
            Ok(c) => {
                self.cards.current = Some(c);
                self.cards.saved_name = Some(name.to_string());
                self.cards.export_result = None;
                self.cards.preview = None;
                self.cards.mark_dirty();
            }
            Err(e) => self.notify_err(e.to_string()),
        }
    }

    fn save_and_preview(&mut self) {
        let (Some(lib), Some(col)) = (&self.lib, &self.cards.current) else {
            return;
        };
        self.cards.error = None;
        if !collection::valid_name(&col.name) {
            self.cards.error =
                Some("Give this card a name (letters, numbers, spaces, . _ -).".into());
        } else {
            let renamed = self
                .cards
                .saved_name
                .as_deref()
                .is_some_and(|s| s != col.name);
            let taken = self
                .cards
                .names
                .as_ref()
                .is_some_and(|n| n.contains(&col.name));
            if (renamed || self.cards.saved_name.is_none()) && taken {
                self.cards.error = Some(format!("There's already a card called '{}'.", col.name));
            } else {
                if let Err(e) = col.save(lib) {
                    self.cards.error = Some(e.to_string());
                } else {
                    if renamed {
                        let _ = Collection::delete(lib, self.cards.saved_name.as_ref().unwrap());
                    }
                    self.cards.saved_name = Some(col.name.clone());
                    self.cards.names = Some(collection::list(lib));
                }
            }
        }
        match collection::resolve(lib, col) {
            Ok(sel) => self.cards.preview = Some(sel),
            Err(e) => self.cards.error = Some(e.to_string()),
        }
    }

    fn card_editor(&mut self, ui: &mut Ui) -> bool {
        let stats = self.data.stats.clone();
        let col = self.cards.current.as_mut().unwrap();
        let mut changed = false;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Name").strong());
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut col.name)
                        .desired_width(240.0)
                        .font(egui::TextStyle::Heading),
                )
                .changed();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Delete…").clicked() {
                    self.cards.confirm_delete = true;
                }
            });
        });
        if let Some(e) = &self.cards.error {
            ui.colored_label(widgets::err_color(ui), e);
        }

        card(ui, |ui| {
            section(ui, "Handheld firmware");
            for p in profiles::PROFILES {
                let resp = ui.radio(col.profile == p.id, RichText::new(p.label).strong());
                ui.label(RichText::new(format!("      {}", p.devices)).weak());
                if resp.clicked() && col.profile != p.id {
                    col.profile = p.id.to_string();
                    changed = true;
                }
            }
        });

        ui.add_space(8.0);
        card(ui, |ui| {
            section(ui, "Card size");
            let mut limited = col.max_size.is_some();
            if ui
                .checkbox(&mut limited, "Fit everything on a card of this size")
                .changed()
            {
                col.max_size = limited.then(|| usable(64));
                changed = true;
            }
            if let Some(max) = col.max_size {
                ui.horizontal_wrapped(|ui| {
                    for gb in CARD_SIZES {
                        let label = if *gb >= 1000 {
                            "1 TB".to_string()
                        } else {
                            format!("{gb} GB")
                        };
                        if ui.selectable_label(max == usable(*gb), label).clicked() {
                            col.max_size = Some(usable(*gb));
                            changed = true;
                        }
                    }
                    ui.label("or");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.cards.custom_size)
                            .hint_text("e.g. 100G")
                            .desired_width(70.0),
                    );
                    if resp.lost_focus() {
                        if let Some(b) = tabularium_core::parse_size(&self.cards.custom_size) {
                            col.max_size = Some(b);
                            changed = true;
                        }
                    }
                });
                ui.label(RichText::new(format!(
                    "Uses up to {}. If not everything fits, games are spread evenly across systems.",
                    human_size(max)
                )).weak());
                ui.label("Fit these first (one per line, * wildcards):");
                let mut text = col.priority_titles.join("\n");
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut text)
                            .desired_rows(2)
                            .hint_text("e.g. *Zelda*")
                            .desired_width(f32::INFINITY),
                    )
                    .changed()
                {
                    col.priority_titles = text.split('\n').map(String::from).collect();
                    if text.is_empty() {
                        col.priority_titles.clear();
                    }
                    changed = true;
                }
            }
        });

        ui.add_space(8.0);
        card(ui, |ui| {
            section(ui, "Hand-picked games");
            ui.label(
                RichText::new(
                    "Always on this card. Search below, or select games in the Library and use \
                     'Add to SD card'.",
                )
                .weak(),
            );
            let roms = &self.data.roms;
            let mut remove = None;
            for (i, id) in col.picked.iter().enumerate() {
                let Some(r) = roms.iter().find(|r| r.id == *id) else {
                    continue;
                };
                ui.horizontal(|ui| {
                    if ui
                        .small_button("×")
                        .on_hover_text("Take off this card")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                    ui.label(r.display_name());
                    ui.label(RichText::new(systems::name_of(&r.system)).weak());
                });
            }
            if let Some(i) = remove {
                col.picked.remove(i);
                changed = true;
            }
            let key = egui::Id::unique("pick-search");
            let mut q: String = ui.data_mut(|d| d.get_temp(key)).unwrap_or_default();
            ui.add(
                egui::TextEdit::singleline(&mut q)
                    .hint_text("🔍 find a game to add…")
                    .desired_width(280.0),
            );
            let ql = q.trim().to_lowercase();
            if ql.len() >= 2 {
                let hits: Vec<&tabularium_core::library::Rom> = roms
                    .iter()
                    .filter(|r| !col.picked.contains(&r.id))
                    .filter(|r| {
                        r.stem.to_lowercase().contains(&ql)
                            || r.alt_name
                                .as_ref()
                                .is_some_and(|a| a.to_lowercase().contains(&ql))
                    })
                    .take(12)
                    .collect();
                if hits.is_empty() {
                    ui.label(RichText::new("No matches").weak());
                }
                for r in hits {
                    ui.horizontal(|ui| {
                        if ui.small_button("+").clicked() {
                            col.picked.push(r.id);
                            changed = true;
                        }
                        ui.label(r.display_name());
                        ui.label(RichText::new(systems::name_of(&r.system)).weak());
                    });
                }
            }
            ui.data_mut(|d| d.insert_temp(key, q));
        });

        ui.add_space(8.0);
        card(ui, |ui| {
            changed |= ui
                .checkbox(
                    &mut col.use_rules,
                    RichText::new("Also add games by rules").strong().size(15.0),
                )
                .on_hover_text(
                    "Untick for a card that holds only your hand-picked games (a mixtape).",
                )
                .changed();
            if !col.use_rules {
                return;
            }
            section(ui, "Systems");
            changed |= systems_picker(ui, "card-systems", &mut col.systems, &stats);
            changed |= filter_editor(
                ui,
                "card",
                &mut col.filters,
                &self.data.genres,
                &self.data.tags,
            );
            section(ui, "Random pick");
            let mut random = col.random_count.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut random, "Pick").changed() {
                    col.random_count = random.then_some(20);
                    changed = true;
                }
                if let Some(n) = col.random_count.as_mut() {
                    changed |= ui.add(egui::DragValue::new(n).range(1..=10000)).changed();
                }
                ui.label("random games from those the rules match");
                if col.random_count.is_some() && ui.button("🎲 Reshuffle").clicked() {
                    col.random_seed = col
                        .random_seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    changed = true;
                }
            });
            if col.random_count.is_some() {
                ui.label(
                    RichText::new(
                        "Tip: with 'Only games I haven't played' ticked above, each copy can bring \
                         fresh games. Games you play drop off the next time 'Remove games I've taken \
                         out' runs; your saves stay on the card.",
                    )
                    .weak(),
                );
            }
        });

        ui.add_space(8.0);
        card(ui, |ui| {
            section(ui, "Artwork and extras");
            ui.horizontal(|ui| {
                ui.label("Image to show:");
                let current = col.art_kind.clone().unwrap_or_else(|| "boxart".into());
                egui::ComboBox::from_id_salt("art-kind")
                    .selected_text(art_kind_label(&current))
                    .show_ui(ui, |ui| {
                        for k in ART_KINDS {
                            if ui
                                .selectable_label(current == *k, art_kind_label(k))
                                .clicked()
                            {
                                col.art_kind = Some(k.to_string());
                                changed = true;
                            }
                        }
                    });
            });
            changed |= ui
                .checkbox(
                    &mut col.multidisc_m3u,
                    "Combine multi-disc games into one entry (.m3u)",
                )
                .changed();
            let profile = profiles::get(&col.profile).unwrap_or(&profiles::PROFILES[0]);
            egui::CollapsingHeader::new("Folder names on the card").show(ui, |ui| {
                ui.label(RichText::new(
                    "Firmware updates sometimes rename folders. Type a name to override the default.",
                ).weak());
                egui::Grid::new("folders").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    for s in systems::SYSTEMS.iter().filter(|s| stats.get(s.id).is_some_and(|st| st.count > 0)) {
                        ui.label(s.display_name());
                        let default = profile.system_folder(s, &BTreeMap::new());
                        let mut v = col.folder_overrides.get(s.id).cloned().unwrap_or_default();
                        if ui.add(egui::TextEdit::singleline(&mut v).hint_text(default).desired_width(220.0)).changed() {
                            if v.trim().is_empty() {
                                col.folder_overrides.remove(s.id);
                            } else {
                                col.folder_overrides.insert(s.id.to_string(), v);
                            }
                            changed = true;
                        }
                        ui.end_row();
                    }
                });
            });
        });
        ui.add_space(20.0);
        changed
    }

    fn card_preview(&mut self, ui: &mut Ui, root: &std::path::Path) {
        let Some(col) = self.cards.current.clone() else {
            return;
        };
        let profile = col.profile();
        ui.add_space(6.0);
        ui.label(RichText::new("What goes on the card").strong().size(16.0));
        let Some(sel) = &self.cards.preview else {
            ui.spinner();
            return;
        };
        let mut per: BTreeMap<&str, (usize, u64)> = BTreeMap::new();
        for r in &sel.roms {
            let e = per.entry(r.system.as_str()).or_default();
            e.0 += 1;
            e.1 += r.total_size;
        }
        egui::Grid::new("preview")
            .num_columns(3)
            .striped(true)
            .spacing([10.0, 3.0])
            .show(ui, |ui| {
                for (sid, (n, b)) in &per {
                    let folder = systems::get(sid)
                        .map(|s| profile.rom_dir(s, &col.folder_overrides))
                        .unwrap_or_default();
                    ui.label(systems::name_of(sid))
                        .on_hover_text(format!("{folder}/"));
                    ui.label(format!("{n}"));
                    ui.label(human_size(*b));
                    ui.end_row();
                }
            });
        ui.separator();
        ui.label(
            RichText::new(format!(
                "{} games · {}",
                sel.roms.len(),
                human_size(sel.size())
            ))
            .strong(),
        );
        if let Some(max) = col.max_size {
            let frac = (sel.size() as f32 / max as f32).min(1.0);
            ui.add(egui::ProgressBar::new(frac).text(format!(
                "{} of {}",
                human_size(sel.size()),
                human_size(max)
            )));
        }
        if !sel.over_budget.is_empty() {
            ui.colored_label(
                widgets::warn_color(ui),
                format!(
                    "{} game(s) don't fit and were left out",
                    sel.over_budget.len()
                ),
            );
        }
        ui.label(
            RichText::new(format!(
                "{} version(s) filtered out by your rules",
                sel.rejected.len()
            ))
            .weak(),
        );

        ui.add_space(12.0);
        ui.label(RichText::new("Copy to SD card").strong().size(16.0));
        ui.label(RichText::new(profile.label).weak());
        let dest_text = self
            .cards
            .dest
            .as_ref()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "No card chosen".into());
        ui.add(egui::Label::new(RichText::new(&dest_text).weak()).truncate())
            .on_hover_text(&dest_text);
        if ui
            .button("Choose card…")
            .on_hover_text("Pick the top folder of the SD card (or any folder)")
            .clicked()
        {
            let mut dlg = rfd::FileDialog::new().set_title("SD card (its top folder)");
            if let Some(d) = self
                .cards
                .dest
                .as_ref()
                .or(self.settings.last_card_folder.as_ref())
            {
                dlg = dlg.set_directory(d);
            }
            if let Some(d) = dlg.pick_folder() {
                self.settings.last_card_folder = Some(d.clone());
                self.cards.dest = Some(d);
            }
        }
        if let Some(p) = widgets::path_entry(ui, "card-dest", "Use") {
            self.settings.last_card_folder = Some(p.clone());
            self.cards.dest = Some(p);
        }
        let o = &mut self.cards.export_opts;
        ui.checkbox(&mut o.art, "Copy artwork");
        ui.checkbox(&mut o.bios, "Copy BIOS files these systems need");
        ui.checkbox(&mut o.sync, "Remove games I've taken out of this card since last time")
            .on_hover_text("Only removes files this app put on the card. Your saves and other files are never touched.");
        ui.checkbox(&mut o.read_history, "Read play history from the card first")
            .on_hover_text("Marks games you've played on the handheld as played in the library (EmulationStation-based firmware and Onion OS).");
        if ui
            .add_enabled(
                self.cards.dest.is_some() && !self.busy(),
                egui::Button::new("Read play history now"),
            )
            .clicked()
        {
            self.start_read_history(ui.ctx(), root);
        }
        ui.horizontal(|ui| {
            let can = self.cards.dest.is_some() && !self.busy() && self.cards.error.is_none();
            if ui
                .add_enabled(can, egui::Button::new("Preview"))
                .on_hover_text("List what would be copied, without copying")
                .clicked()
            {
                self.start_export(ui.ctx(), root, true);
            }
            if ui
                .add_enabled(
                    can,
                    egui::Button::new(RichText::new("Copy to card").strong()),
                )
                .clicked()
            {
                self.start_export(ui.ctx(), root, false);
            }
        });

        if let Some((rep, dest, dry)) = &self.cards.export_result {
            ui.add_space(8.0);
            card(ui, |ui| {
                let verb = if *dry { "Would copy" } else { "Copied" };
                ui.label(
                    RichText::new(format!(
                        "{verb} {} file(s), {}",
                        rep.copied,
                        human_size(rep.bytes_copied)
                    ))
                    .strong(),
                );
                ui.label(format!(
                    "{} already up to date · {} image(s) · {} without art",
                    rep.unchanged, rep.art, rep.art_missing
                ));
                if rep.removed > 0 {
                    ui.label(format!(
                        "{} old file(s) {}",
                        rep.removed,
                        if *dry { "would be removed" } else { "removed" }
                    ));
                }
                if rep.cancelled {
                    ui.colored_label(
                        widgets::warn_color(ui),
                        "Cancelled part-way. Run it again to finish.",
                    );
                }
                if let Some(h) = &rep.history {
                    if h.found > 0 {
                        ui.label(format!(
                            "Play history: {} game(s) played on the card, {} newly marked in the library",
                            h.found, h.updated
                        ));
                    }
                    for n in &h.notes {
                        ui.colored_label(widgets::warn_color(ui), n);
                    }
                }
                if !rep.bios.copied.is_empty() {
                    ui.label(format!("BIOS: {}", rep.bios.copied.join(", ")));
                }
                for m in &rep.bios.missing_required {
                    ui.colored_label(
                        widgets::err_color(ui),
                        format!("Missing required BIOS — {m}"),
                    );
                }
                for w in &rep.warnings {
                    ui.colored_label(widgets::warn_color(ui), w);
                }
                if !*dry {
                    ui.label(RichText::new(profile.notes).weak());
                    if ui.button("Open card folder").clicked() {
                        let _ = open::that(dest);
                    }
                }
                if *dry && !rep.planned.is_empty() {
                    egui::CollapsingHeader::new(format!("Files ({})", rep.planned.len())).show(
                        ui,
                        |ui| {
                            for p in rep.planned.iter().take(1000) {
                                ui.label(RichText::new(p).small());
                            }
                        },
                    );
                }
            });
        }
    }

    fn start_read_history(&mut self, ctx: &egui::Context, root: &std::path::Path) {
        let (Some(col), Some(dest)) = (self.cards.current.clone(), self.cards.dest.clone()) else {
            return;
        };
        let root = root.to_path_buf();
        self.job = Some(Job::spawn("Reading play history", ctx, move |rep| {
            let res = Library::open(&root).and_then(|lib| {
                tabularium_core::playhistory::read_card(
                    &lib,
                    &dest,
                    col.profile(),
                    &col.folder_overrides,
                    rep,
                )
            });
            match res {
                Ok(h) => JobOutput::History(h),
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }

    fn start_export(&mut self, ctx: &egui::Context, root: &std::path::Path, dry_run: bool) {
        let (Some(col), Some(dest)) = (self.cards.current.clone(), self.cards.dest.clone()) else {
            return;
        };
        let root = root.to_path_buf();
        let opts = ExportOptions {
            dry_run,
            ..self.cards.export_opts.clone()
        };
        self.cards.export_result = None;
        let title = if dry_run {
            "Checking what to copy"
        } else {
            "Copying to SD card"
        };
        self.job = Some(Job::spawn(title, ctx, move |rep| {
            match Library::open(&root)
                .and_then(|lib| collection::export(&lib, &col, &dest, &opts, rep))
            {
                Ok(report) => JobOutput::Export {
                    report,
                    dest,
                    dry_run,
                },
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }

    fn delete_dialog(&mut self, ui: &mut Ui) {
        if !self.cards.confirm_delete {
            return;
        }
        let name = self
            .cards
            .current
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let m = egui::Modal::new(egui::Id::unique("del-card")).show(ui.ctx(), |ui| {
            ui.heading(format!("Delete '{name}'?"));
            ui.label("This removes the card's settings. Your library and anything already on the SD card stay as they are.");
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let (Some(lib), Some(saved)) = (&self.lib, &self.cards.saved_name) {
                        let _ = Collection::delete(lib, saved);
                    }
                    self.cards.current = None;
                    self.cards.saved_name = None;
                    self.cards.names = None;
                    self.cards.confirm_delete = false;
                }
                if ui.button("Cancel").clicked() {
                    self.cards.confirm_delete = false;
                }
            });
        });
        if m.should_close() {
            self.cards.confirm_delete = false;
        }
    }
}
