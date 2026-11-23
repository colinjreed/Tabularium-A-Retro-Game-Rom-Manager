//! Reusable UI pieces: system picker, filter editor, headings.

use egui::{Color32, RichText, Ui};
use std::collections::BTreeMap;
use tabularium_core::filters::{FilterSpec, COMMON_REGIONS};
use tabularium_core::library::SystemStats;
use tabularium_core::naming::{flag_description, ALL_FLAGS, DEFAULT_JUNK};
use tabularium_core::systems;

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.add_space(4.0);
    ui.heading(RichText::new(title).size(22.0).strong());
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).weak());
    }
    ui.add_space(8.0);
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).strong().size(15.0));
    ui.add_space(2.0);
}

pub fn ok_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(110, 200, 120)
    } else {
        Color32::from_rgb(20, 130, 40)
    }
}

pub fn warn_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(240, 180, 80)
    } else {
        Color32::from_rgb(170, 100, 0)
    }
}

pub fn err_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(240, 110, 100)
    } else {
        Color32::from_rgb(190, 30, 30)
    }
}

/// A card-like frame for grouping controls.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Checkbox grid of systems. An empty selection means "all systems".
/// Returns true if the selection changed.
pub fn systems_picker(
    ui: &mut Ui,
    _id: &str,
    selected: &mut Vec<String>,
    stats: &BTreeMap<String, SystemStats>,
) -> bool {
    let mut changed = false;
    let present: Vec<&systems::System> = systems::SYSTEMS
        .iter()
        .filter(|s| stats.get(s.id).is_some_and(|st| st.count > 0))
        .collect();
    if present.is_empty() {
        ui.label(RichText::new("No ROMs in the library yet.").weak());
        return false;
    }
    let all = selected.is_empty();
    ui.horizontal(|ui| {
        if ui.radio(all, "All systems").clicked() && !all {
            selected.clear();
            changed = true;
        }
        if ui.radio(!all, "Only these:").clicked() && all {
            *selected = present.iter().map(|s| s.id.to_string()).collect();
            changed = true;
        }
    });
    if !all {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for s in &present {
                let mut on = selected.iter().any(|x| x == s.id);
                let n = stats.get(s.id).map(|st| st.count).unwrap_or(0);
                if ui
                    .checkbox(&mut on, format!("{}  ({n})", s.display_name()))
                    .changed()
                {
                    if on {
                        selected.push(s.id.to_string());
                    } else {
                        selected.retain(|x| x != s.id);
                    }
                    changed = true;
                }
            }
        });
        if selected.is_empty() {
            ui.colored_label(
                warn_color(ui),
                "No systems ticked: switch back to All systems or pick some.",
            );
        }
    }
    changed
}

fn edit_list(ui: &mut Ui, list: &mut Vec<String>, sep: &str, hint: &str, multiline: bool) -> bool {
    let mut text = list.join(sep);
    let resp = if multiline {
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_rows(2)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        )
    } else {
        ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text(hint)
                .desired_width(260.0),
        )
    };
    if resp.changed() {
        *list = text.split(sep.trim()).map(String::from).collect();
        if text.is_empty() {
            list.clear();
        }
        return true;
    }
    false
}

/// Editor for a FilterSpec. Returns true when anything changed.
pub fn filter_editor(
    ui: &mut Ui,
    id: &str,
    spec: &mut FilterSpec,
    genres: &[(String, u64)],
    tags: &[(String, u64)],
) -> bool {
    let mut changed = false;

    section(ui, "Genres, tags and played");
    changed |= ui
        .checkbox(&mut spec.only_unplayed, "Only games I haven't played")
        .changed();
    let toggle_list =
        |ui: &mut Ui, list: &mut Vec<String>, items: &[(String, u64)], prefix: &str| {
            let mut ch = false;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for (name, n) in items {
                    let mut on = list.contains(name);
                    if ui
                        .checkbox(&mut on, format!("{prefix}{name} ({n})"))
                        .changed()
                    {
                        if on {
                            list.push(name.clone());
                        } else {
                            list.retain(|x| x != name);
                        }
                        ch = true;
                    }
                }
            });
            ch
        };
    if genres.is_empty() {
        ui.label(
            RichText::new("No genres yet: they're filled in when you import or download artwork.")
                .weak(),
        );
    } else {
        ui.label("Only these genres (none ticked = any):");
        changed |= toggle_list(ui, &mut spec.genres, genres, "");
    }
    if !tags.is_empty() {
        ui.label("Only games with one of these tags (none ticked = any):");
        changed |= toggle_list(ui, &mut spec.tags, tags, "#");
        ui.label("Leave out games with these tags:");
        let mut ex = spec.exclude_tags.clone();
        let c = ui
            .push_id(format!("{id}-ex"), |ui| toggle_list(ui, &mut ex, tags, "#"))
            .inner;
        if c {
            spec.exclude_tags = ex;
            changed = true;
        }
    }

    section(ui, "Unwanted releases");
    ui.horizontal(|ui| {
        if ui
            .button("Tick the usual junk")
            .on_hover_text("Betas, prototypes, demos, BIOS, bad dumps…")
            .clicked()
        {
            for f in DEFAULT_JUNK {
                if !spec.exclude_flags.iter().any(|x| x == f) {
                    spec.exclude_flags.push(f.to_string());
                }
            }
            changed = true;
        }
        if ui.button("Clear").clicked() {
            spec.exclude_flags.clear();
            changed = true;
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        for f in ALL_FLAGS {
            let mut on = spec.exclude_flags.iter().any(|x| x == f);
            if ui.checkbox(&mut on, flag_description(f)).changed() {
                if on {
                    spec.exclude_flags.push(f.to_string());
                } else {
                    spec.exclude_flags.retain(|x| x != f);
                }
                changed = true;
            }
        }
    });

    section(ui, "One game, one ROM (1G1R)");
    changed |= ui
        .checkbox(&mut spec.one_game_one_rom, "Keep only the best version of each game")
        .on_hover_text("Picks by region preference, then verified dumps, then the latest revision. Multi-disc games stay together.")
        .changed();
    if spec.one_game_one_rom {
        ui.label(RichText::new("Region preference (top = most preferred):").weak());
        let mut swap: Option<(usize, usize)> = None;
        let mut remove: Option<usize> = None;
        let n = spec.region_priority.len();
        egui::Grid::new(format!("{id}-prio"))
            .num_columns(4)
            .spacing([6.0, 2.0])
            .show(ui, |ui| {
                for (i, r) in spec.region_priority.iter().enumerate() {
                    ui.label(format!("{}. {r}", i + 1));
                    if ui
                        .add_enabled(i > 0, egui::Button::new("⬆").small())
                        .on_hover_text("Prefer more")
                        .clicked()
                    {
                        swap = Some((i, i - 1));
                    }
                    if ui
                        .add_enabled(i + 1 < n, egui::Button::new("⬇").small())
                        .on_hover_text("Prefer less")
                        .clicked()
                    {
                        swap = Some((i, i + 1));
                    }
                    if ui.small_button("×").on_hover_text("Remove").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some((a, b)) = swap {
            spec.region_priority.swap(a, b);
            changed = true;
        }
        if let Some(i) = remove {
            spec.region_priority.remove(i);
            changed = true;
        }
        let missing: Vec<&&str> = COMMON_REGIONS
            .iter()
            .filter(|r| !spec.region_priority.iter().any(|x| x == **r))
            .collect();
        if !missing.is_empty() {
            egui::ComboBox::from_id_salt(format!("{id}-addregion"))
                .selected_text("Add a region…")
                .show_ui(ui, |ui| {
                    for r in missing {
                        if ui.selectable_label(false, r.to_string()).clicked() {
                            spec.region_priority.push(r.to_string());
                            changed = true;
                        }
                    }
                });
        }
    }

    section(ui, "Regions and languages");
    ui.label(RichText::new("Leave everything unticked to keep all regions.").weak());
    ui.horizontal_wrapped(|ui| {
        for r in COMMON_REGIONS {
            let mut on = spec.regions.iter().any(|x| x == r);
            if ui.checkbox(&mut on, *r).changed() {
                if on {
                    spec.regions.push(r.to_string());
                } else {
                    spec.regions.retain(|x| x != r);
                }
                changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Only languages:");
        changed |= edit_list(
            ui,
            &mut spec.languages,
            ",",
            "e.g. en, fr  (blank = any)",
            false,
        );
    });

    section(ui, "Titles");
    ui.label("Leave out titles matching (one per line, * wildcards allowed):");
    changed |= edit_list(ui, &mut spec.exclude_titles, "\n", "e.g. *Pinball*", true);
    ui.label("Only include titles matching (blank = everything):");
    changed |= edit_list(ui, &mut spec.include_titles, "\n", "e.g. Pokemon*", true);
    changed
}

/// A text box for typing a folder path, with a button. Returns the path when
/// the button is pressed (or Enter), as a fallback for systems where the
/// native folder dialog isn't available.
pub fn path_entry(ui: &mut Ui, id: &str, button: &str) -> Option<std::path::PathBuf> {
    let key = egui::Id::unique(("path-entry", id));
    let mut text: String = ui.data_mut(|d| d.get_temp(key).unwrap_or_default());
    let mut out = None;
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text("…or type a folder path")
                .desired_width(320.0),
        );
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui
            .add_enabled(!text.trim().is_empty(), egui::Button::new(button))
            .clicked()
            || enter)
            && !text.trim().is_empty()
        {
            out = Some(std::path::PathBuf::from(text.trim()));
        }
    });
    if out.is_some() {
        text.clear();
    }
    ui.data_mut(|d| d.insert_temp(key, text));
    out
}
