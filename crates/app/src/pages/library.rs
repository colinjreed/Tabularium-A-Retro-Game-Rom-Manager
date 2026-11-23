//! The Library page: browse, filter, select, tag and mark games played.

use crate::app::{Page, RomApp};
use crate::widgets::{self, page_title};
use egui::{RichText, Ui};
use std::collections::BTreeSet;
use tabularium_core::collection::Collection;
use tabularium_core::library::{Rom, FINISHED, NOT_PLAYED, PLAYED};
use tabularium_core::{human_size, systems};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PlayedFilter {
    #[default]
    Any,
    Unplayed,
    Played,
    Finished,
}

impl PlayedFilter {
    fn label(self) -> &'static str {
        match self {
            PlayedFilter::Any => "Played or not",
            PlayedFilter::Unplayed => "Not played",
            PlayedFilter::Played => "Played",
            PlayedFilter::Finished => "Finished",
        }
    }

    fn accepts(self, state: u8) -> bool {
        match self {
            PlayedFilter::Any => true,
            PlayedFilter::Unplayed => state == NOT_PLAYED,
            PlayedFilter::Played => state == PLAYED,
            PlayedFilter::Finished => state == FINISHED,
        }
    }
}

type FilterKey = (
    Option<String>,
    String,
    bool,
    Option<String>,
    Option<String>,
    PlayedFilter,
    u64,
);

#[derive(Default)]
pub struct LibraryView {
    pub system: Option<String>,
    pub search: String,
    pub show_trash: bool,
    pub selected: BTreeSet<i64>,
    pub genre: Option<String>,
    pub tag: Option<String>,
    pub played: PlayedFilter,
    anchor: Option<usize>,
    filtered: Vec<usize>,
    key: Option<FilterKey>,
    scroll_to: Option<usize>,
    new_tag: String,
    genre_text: Option<(i64, String)>,
}

impl LibraryView {
    pub fn invalidate(&mut self) {
        self.key = None;
    }
}

/// "2025-01-02" from Unix seconds.
fn date_string(t: f64) -> String {
    let days = (t / 86400.0).floor() as i64;
    // Civil from days (Howard Hinnant).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

fn random_index(len: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(7);
    let mut x = nanos ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    (x % len.max(1) as u64) as usize
}

impl RomApp {
    pub fn library_page(&mut self, ui: &mut Ui) {
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
                        self.view.selected.clear();
                    }
                });
            });

        let list = if self.view.show_trash {
            &self.data.trashed
        } else {
            &self.data.roms
        };
        let chosen: Vec<Rom> = list
            .iter()
            .filter(|r| self.view.selected.contains(&r.id))
            .cloned()
            .collect();
        if !chosen.is_empty() {
            egui::Panel::right("rom-detail")
                .resizable(true)
                .default_size(310.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if chosen.len() == 1 {
                            self.rom_detail(ui, &chosen[0]);
                        } else {
                            self.bulk_panel(ui, &chosen);
                        }
                    });
                });
        }
        egui::CentralPanel::default().show(ui, |ui| self.rom_table(ui));
    }

    fn filter_bar(&mut self, ui: &mut Ui) {
        // Two fixed rows, so the table doesn't jump when the side panel opens.
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.view.search).hint_text("Search titles…").desired_width(200.0));
            if !self.view.show_trash
                && ui
                    .button("🎲 Surprise me")
                    .on_hover_text("Pick a random game from the list below, preferring ones you haven't played")
                    .clicked()
            {
                self.surprise_me();
            }
            if self.view.show_trash && !self.data.trashed.is_empty() {
                if ui.button("Restore all").clicked() {
                    self.restore_roms(self.data.trashed.clone());
                }
                if ui.button("Empty trash…").clicked() {
                    self.clean.confirm_empty_trash = true;
                }
            }
        });
        if self.view.show_trash {
            return;
        }
        ui.horizontal(|ui| {
            let genres = self.data.genres.clone();
            egui::ComboBox::from_id_salt("genre-filter")
                .selected_text(
                    self.view
                        .genre
                        .clone()
                        .unwrap_or_else(|| "Any genre".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view.genre, None, "Any genre");
                    for (g, n) in genres {
                        ui.selectable_value(
                            &mut self.view.genre,
                            Some(g.clone()),
                            format!("{g}  ({n})"),
                        );
                    }
                });
            let tags = self.data.tags.clone();
            egui::ComboBox::from_id_salt("tag-filter")
                .selected_text(
                    self.view
                        .tag
                        .clone()
                        .map(|t| format!("#{t}"))
                        .unwrap_or_else(|| "Any tag".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view.tag, None, "Any tag");
                    for (t, n) in tags {
                        ui.selectable_value(
                            &mut self.view.tag,
                            Some(t.clone()),
                            format!("#{t}  ({n})"),
                        );
                    }
                });
            egui::ComboBox::from_id_salt("played-filter")
                .selected_text(self.view.played.label())
                .show_ui(ui, |ui| {
                    for f in [
                        PlayedFilter::Any,
                        PlayedFilter::Unplayed,
                        PlayedFilter::Played,
                        PlayedFilter::Finished,
                    ] {
                        ui.selectable_value(&mut self.view.played, f, f.label());
                    }
                });
        });
    }

    fn refilter(&mut self) {
        let v = &self.view;
        let key: FilterKey = (
            v.system.clone(),
            v.search.to_lowercase(),
            v.show_trash,
            v.genre.clone(),
            v.tag.clone(),
            v.played,
            self.data.version,
        );
        if self.view.key.as_ref() == Some(&key) {
            return;
        }
        let list = if key.2 {
            &self.data.trashed
        } else {
            &self.data.roms
        };
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
            .filter(|(_, r)| key.3.as_ref().is_none_or(|g| r.genre.as_ref() == Some(g)))
            .filter(|(_, r)| key.4.as_ref().is_none_or(|t| r.tags.contains(t)))
            .filter(|(_, r)| key.5.accepts(r.played))
            .map(|(i, _)| i)
            .collect();
        self.view.key = Some(key);
        self.view.anchor = None;
    }

    fn surprise_me(&mut self) {
        self.refilter();
        let rows = &self.view.filtered;
        let unplayed: Vec<usize> = (0..rows.len())
            .filter(|&i| self.data.roms[rows[i]].played == NOT_PLAYED)
            .collect();
        let pool: Vec<usize> = if unplayed.is_empty() {
            (0..rows.len()).collect()
        } else {
            unplayed
        };
        if pool.is_empty() {
            self.notify("Nothing in the list to pick from.");
            return;
        }
        let row = pool[random_index(pool.len())];
        let rom = &self.data.roms[rows[row]];
        self.view.selected = BTreeSet::from([rom.id]);
        self.view.anchor = Some(row);
        self.view.scroll_to = Some(row);
        self.notify(format!("How about {}?", rom.display_name()));
    }

    fn rom_table(&mut self, ui: &mut Ui) {
        self.filter_bar(ui);
        self.refilter();
        let n_sel = self.view.selected.len();
        let hint = if n_sel > 1 {
            format!("{} ROMs · {n_sel} selected", self.view.filtered.len())
        } else {
            format!(
                "{} ROMs · Ctrl/⌘-click or Shift-click to select several",
                self.view.filtered.len()
            )
        };
        ui.label(RichText::new(hint).weak());
        let show_trash = self.view.show_trash;
        let rows = self.view.filtered.clone();
        let mut clicked: Option<usize> = None;
        let text_h = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        let mut table = egui_extras::TableBuilder::new(ui)
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
            .column(
                egui_extras::Column::initial(90.0)
                    .at_least(50.0)
                    .clip(true)
                    .resizable(true),
            )
            .column(egui_extras::Column::exact(28.0))
            .column(egui_extras::Column::exact(28.0));
        if let Some(r) = self.view.scroll_to.take() {
            table = table.scroll_to_row(r, Some(egui::Align::Center));
        }
        table
            .header(22.0, |mut h| {
                h.col(|ui| {
                    ui.strong("Title");
                });
                h.col(|ui| {
                    ui.strong("System");
                });
                h.col(|ui| {
                    ui.strong(if show_trash { "Why removed" } else { "Genre" });
                });
                h.col(|ui| {
                    ui.strong("Region");
                });
                h.col(|ui| {
                    ui.strong("▶").on_hover_text("Played (▶) or finished (✔)");
                });
                h.col(|ui| {
                    ui.strong("Art");
                });
            })
            .body(|body| {
                body.rows(text_h, rows.len(), |mut row| {
                    let idx = row.index();
                    let list = if show_trash {
                        &self.data.trashed
                    } else {
                        &self.data.roms
                    };
                    let r = &list[rows[idx]];
                    row.set_selected(self.view.selected.contains(&r.id));
                    row.col(|ui| {
                        let mut label = r.display_name().to_string();
                        if !r.tags.is_empty() {
                            label += &format!(
                                "   #{}",
                                r.tags.iter().cloned().collect::<Vec<_>>().join(" #")
                            );
                        }
                        ui.label(label).on_hover_text(r.file_name());
                    });
                    row.col(|ui| {
                        ui.label(
                            systems::get(&r.system)
                                .map(|s| s.display_name())
                                .unwrap_or(&r.system),
                        );
                    });
                    row.col(|ui| {
                        if show_trash {
                            ui.label(r.status_reason.clone().unwrap_or_default());
                        } else {
                            ui.label(r.genre.clone().unwrap_or_default());
                        }
                    });
                    row.col(|ui| {
                        ui.label(r.regions.join(", "));
                    });
                    row.col(|ui| {
                        match r.played {
                            PLAYED => ui.label("▶").on_hover_text("Played"),
                            FINISHED => ui.label("✔").on_hover_text("Finished"),
                            _ => ui.label(""),
                        };
                    });
                    row.col(|ui| {
                        if self.data.with_art.contains(&r.id) {
                            ui.label("🖼");
                        }
                    });
                    if row.response().clicked() {
                        clicked = Some(idx);
                    }
                });
            });
        if let Some(idx) = clicked {
            self.click_row(ui, idx);
        }
    }

    fn click_row(&mut self, ui: &Ui, idx: usize) {
        let m = ui.input(|i| i.modifiers);
        let list = if self.view.show_trash {
            &self.data.trashed
        } else {
            &self.data.roms
        };
        let id_at = |i: usize| list[self.view.filtered[i]].id;
        let id = id_at(idx);
        if m.shift {
            let a = self.view.anchor.unwrap_or(idx);
            let (lo, hi) = (a.min(idx), a.max(idx));
            if !m.command {
                self.view.selected.clear();
            }
            for i in lo..=hi {
                self.view.selected.insert(id_at(i));
            }
            return;
        }
        if m.command {
            if !self.view.selected.remove(&id) {
                self.view.selected.insert(id);
            }
        } else if self.view.selected.len() == 1 && self.view.selected.contains(&id) {
            self.view.selected.clear();
        } else {
            self.view.selected = BTreeSet::from([id]);
        }
        self.view.anchor = Some(idx);
    }

    // ------------------------------------------------------------------ actions

    fn ids(roms: &[Rom]) -> Vec<i64> {
        roms.iter().map(|r| r.id).collect()
    }

    fn set_played(&mut self, ids: &[i64], state: u8) {
        if let Some(lib) = &self.lib {
            if let Err(e) = lib.set_played(ids, state) {
                self.notify_err(e.to_string());
            }
        }
        self.reload();
    }

    fn add_tag(&mut self, ids: &[i64], tag: &str) {
        let tag = tag.trim().trim_start_matches('#').trim().to_string();
        if tag.is_empty() {
            return;
        }
        if let Some(lib) = &self.lib {
            if let Err(e) = lib.add_tag(ids, &tag) {
                self.notify_err(e.to_string());
            }
        }
        self.view.new_tag.clear();
        self.reload();
    }

    fn remove_tag(&mut self, ids: &[i64], tag: &str) {
        if let Some(lib) = &self.lib {
            if let Err(e) = lib.remove_tag(ids, tag) {
                self.notify_err(e.to_string());
            }
        }
        self.reload();
    }

    /// Add games to an SD card's hand-picked list.
    pub fn add_to_card(&mut self, card: &str, ids: &[i64]) {
        let Some(lib) = &self.lib else { return };
        let res = Collection::load(lib, card).and_then(|mut c| {
            let before = c.picked.len();
            for id in ids {
                if !c.picked.contains(id) {
                    c.picked.push(*id);
                }
            }
            let added = c.picked.len() - before;
            c.save(lib).map(|_| added)
        });
        match res {
            Ok(added) => {
                self.cards.note_picked(card, ids);
                self.notify(format!("Added {added} game(s) to '{card}'."));
            }
            Err(e) => self.notify_err(e.to_string()),
        }
    }

    fn add_to_card_menu(&mut self, ui: &mut Ui, ids: &[i64]) {
        let cards = self.data.cards.clone();
        let mut chosen = None;
        egui::ComboBox::from_id_salt(("add-to-card", ids.len()))
            .selected_text("Add to SD card…")
            .show_ui(ui, |ui| {
                if cards.is_empty() {
                    ui.label("No SD cards yet. Make one on the SD cards page.");
                }
                for c in &cards {
                    if ui.selectable_label(false, c).clicked() {
                        chosen = Some(c.clone());
                    }
                }
            });
        if let Some(c) = chosen {
            self.add_to_card(&c, ids);
        }
    }

    fn tag_editor(&mut self, ui: &mut Ui, ids: &[i64], current: &BTreeSet<String>) {
        ui.horizontal_wrapped(|ui| {
            for t in current {
                egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{t}"));
                            if ui.small_button("×").on_hover_text("Remove tag").clicked() {
                                self.remove_tag(ids, t);
                            }
                        });
                    });
            }
        });
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.view.new_tag)
                    .hint_text("add a tag, e.g. co-op")
                    .desired_width(150.0),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Add").clicked() || enter {
                let t = self.view.new_tag.clone();
                self.add_tag(ids, &t);
            }
            let existing: Vec<String> = self
                .data
                .tags
                .iter()
                .map(|(t, _)| t.clone())
                .filter(|t| !current.contains(t))
                .collect();
            if !existing.is_empty() {
                let mut pick = None;
                egui::ComboBox::from_id_salt(("tag-pick", ids.len()))
                    .selected_text("Existing…")
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for t in existing {
                            if ui.selectable_label(false, format!("#{t}")).clicked() {
                                pick = Some(t);
                            }
                        }
                    });
                if let Some(t) = pick {
                    self.add_tag(ids, &t);
                }
            }
        });
    }

    fn bulk_panel(&mut self, ui: &mut Ui, roms: &[Rom]) {
        let ids = Self::ids(roms);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} games selected", roms.len()))
                    .strong()
                    .size(17.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui
                    .small_button("×")
                    .on_hover_text("Clear selection")
                    .clicked()
                {
                    self.view.selected.clear();
                }
            });
        });
        let bytes: u64 = roms.iter().map(|r| r.total_size).sum();
        ui.label(RichText::new(human_size(bytes)).weak());
        if self.view.show_trash {
            if ui.button("Restore these").clicked() {
                self.restore_roms(roms.to_vec());
                self.view.selected.clear();
            }
            return;
        }
        widgets::section(ui, "SD card");
        self.add_to_card_menu(ui, &ids);
        widgets::section(ui, "Played");
        ui.horizontal(|ui| {
            if ui.button("▶ Played").clicked() {
                self.set_played(&ids, PLAYED);
            }
            if ui.button("✔ Finished").clicked() {
                self.set_played(&ids, FINISHED);
            }
            if ui.button("Not played").clicked() {
                self.set_played(&ids, NOT_PLAYED);
            }
        });
        widgets::section(ui, "Tags");
        // Show tags every selected game shares.
        let mut common: BTreeSet<String> = roms[0].tags.clone();
        for r in &roms[1..] {
            common = common.intersection(&r.tags).cloned().collect();
        }
        ui.label(RichText::new("Tags all of them have:").weak());
        self.tag_editor(ui, &ids, &common);
        ui.add_space(12.0);
        if ui.button(format!("Move {} to trash", roms.len())).clicked() {
            if let Some(lib) = &self.lib {
                for r in roms {
                    let _ = lib.trash(r, "removed by hand");
                }
            }
            self.view.selected.clear();
            self.reload();
            self.notify(format!("Moved {} game(s) to the trash.", roms.len()));
        }
    }

    fn rom_detail(&mut self, ui: &mut Ui, rom: &Rom) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(rom.display_name()).strong().size(17.0)).wrap());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.small_button("×").clicked() {
                    self.view.selected.clear();
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
                                .max_height(300.0)
                                .corner_radius(4.0),
                        );
                        break;
                    }
                }
            }
        }
        let active = rom.status == "active";
        if active {
            widgets::section(ui, "Played");
            ui.horizontal(|ui| {
                for (state, label) in [
                    (NOT_PLAYED, "Not played"),
                    (PLAYED, "▶ Played"),
                    (FINISHED, "✔ Finished"),
                ] {
                    if ui.selectable_label(rom.played == state, label).clicked()
                        && rom.played != state
                    {
                        self.set_played(&[rom.id], state);
                    }
                }
            });
            let mut facts = vec![];
            if rom.play_count > 0 {
                facts.push(format!("played {} time(s)", rom.play_count));
            }
            if let Some(t) = rom.last_played.filter(|t| *t > 0.0) {
                facts.push(format!("last on {}", date_string(t)));
            }
            if !facts.is_empty() {
                ui.label(RichText::new(facts.join(", ")).weak());
            }
            widgets::section(ui, "Tags");
            self.tag_editor(ui, &[rom.id], &rom.tags);
            widgets::section(ui, "SD card");
            self.add_to_card_menu(ui, &[rom.id]);
        }
        widgets::section(ui, "Details");
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
        if active {
            // Genre: filled in automatically when known, editable by hand.
            let text = match &self.view.genre_text {
                Some((id, t)) if *id == rom.id => t.clone(),
                _ => rom.genre.clone().unwrap_or_default(),
            };
            let mut t = text;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Genre").weak());
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut t)
                        .hint_text("unknown")
                        .desired_width(150.0),
                );
                if resp.changed() {
                    self.view.genre_text = Some((rom.id, t.clone()));
                }
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let dirty = t.trim() != rom.genre.clone().unwrap_or_default();
                if dirty && (ui.small_button("Save").clicked() || enter) {
                    if let Some(lib) = &self.lib {
                        let _ = lib.set_genre(rom.id, Some(&t));
                    }
                    self.view.genre_text = None;
                    self.reload();
                }
            });
        }
        if rom.system == "scummvm" && active {
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
            if active {
                if ui.button("Move to trash").clicked() {
                    self.trash_now(rom.clone(), "removed by hand");
                }
            } else if ui.button("Restore").clicked() {
                self.restore_roms(vec![rom.clone()]);
            }
        });
    }
}
