//! Output layouts for handheld firmwares: where ROMs, artwork and BIOS go on
//! the SD card, and whether a gamelist.xml is written.

use crate::systems::System;
use std::collections::BTreeMap;
use std::path::Path;
use xmltree::{Element, EmitterConfig, XMLNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub id: &'static str,
    pub label: &'static str,
    pub devices: &'static str,
    pub roms_root: &'static str,
    pub gamelist: bool,
    /// Resize art to this width.
    pub art_width: Option<u32>,
    pub notes: &'static str,
}

pub static PROFILES: &[Profile] = &[
    Profile {
        id: "onion", label: "Onion OS", devices: "Miyoo Mini / Mini+ / Mini Flip",
        roms_root: "Roms", gamelist: false, art_width: Some(250),
        notes: "Refresh ROM lists from the GameSwitcher menu after copying.",
    },
    Profile {
        id: "muos", label: "muOS", devices: "Anbernic RG35XX / RG40XX / RG CubeXX, TrimUI",
        roms_root: "ROMS", gamelist: false, art_width: None,
        notes: "Assign cores to new folders in muOS if they aren't detected automatically.",
    },
    Profile {
        id: "emulationstation", label: "EmulationStation (ArkOS, ROCKNIX, AmberELEC)",
        devices: "Anbernic RG353, R36S, Powkiddy, Odroid Go", roms_root: "roms", gamelist: true,
        art_width: None,
        notes: "Two-card devices keep ROMs on the second card: copy the roms folder there.",
    },
    Profile {
        id: "batocera", label: "Knulli / Batocera", devices: "Knulli on Anbernic RG35XX family, Batocera",
        roms_root: "roms", gamelist: true, art_width: None,
        notes: "Uses Batocera folder names (megadrive, lynx, wswan, mame…).",
    },
    Profile {
        id: "nextui", label: "NextUI / MinUI", devices: "TrimUI Brick / Smart Pro, Miyoo (MinUI)",
        roms_root: "Roms", gamelist: false, art_width: Some(300),
        notes: "Folders are named 'Console (TAG)'. Plain MinUI ignores the .media artwork.",
    },
    Profile {
        id: "esde", label: "ES-DE (Android / Linux)", devices: "Retroid Pocket, AYN Odin, Ayaneo, Steam Deck",
        roms_root: "ROMs", gamelist: false, art_width: None,
        notes: "Point ES-DE's ROM directory at ROMs/ and its media directory at ES-DE/downloaded_media.",
    },
];

pub fn get(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}

/// Join path segments with '/' (card paths are always relative, '/'-separated).
pub fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("/")
}

fn parent(p: &str) -> &str {
    p.rsplit_once('/').map(|(a, _)| a).unwrap_or("")
}

fn stem(p: &str) -> &str {
    let name = p.rsplit('/').next().unwrap_or(p);
    name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name)
}

impl Profile {
    pub fn system_folder(&self, s: &System, overrides: &BTreeMap<String, String>) -> String {
        if let Some(o) = overrides.get(s.id).filter(|o| !o.is_empty()) {
            return o.clone();
        }
        match self.id {
            "batocera" => s.batocera,
            "onion" => s.onion,
            "nextui" => s.nextui,
            _ => s.es,
        }
        .to_string()
    }

    pub fn rom_dir(&self, s: &System, overrides: &BTreeMap<String, String>) -> String {
        join(&[self.roms_root, &self.system_folder(s, overrides)])
    }

    /// Where artwork for a ROM (given its path on the card) goes.
    pub fn art_path(
        &self,
        s: &System,
        rom_rel: &str,
        overrides: &BTreeMap<String, String>,
        kind: &str,
    ) -> String {
        let st = stem(rom_rel);
        let dir = parent(rom_rel);
        let file = format!("{st}.png");
        match self.id {
            "emulationstation" | "batocera" => join(&[dir, "images", &file]),
            "onion" => join(&[dir, "Imgs", &file]),
            "nextui" => join(&[dir, ".media", &file]),
            "esde" => {
                let sub = match kind {
                    "snap" => "screenshots",
                    "title" => "titlescreens",
                    "mix" => "miximages",
                    "wheel" => "marquees",
                    _ => "covers",
                };
                join(&[
                    "ES-DE/downloaded_media",
                    &self.system_folder(s, overrides),
                    sub,
                    &file,
                ])
            }
            _ => {
                let sub = match kind {
                    "snap" | "title" => "preview",
                    _ => "box",
                };
                join(&["MUOS/info/catalogue", s.muos_catalogue, sub, &file])
            }
        }
    }

    /// Where the firmware looks for BIOS files for a system.
    pub fn bios_dir(&self, s: &System) -> String {
        match self.id {
            "nextui" => join(&["Bios", s.nextui_tag()]),
            "emulationstation" => "roms/bios".into(),
            "batocera" => "bios".into(),
            "esde" => "RetroArch/system".into(),
            "muos" => "MUOS/bios".into(),
            "onion" => "BIOS".into(),
            _ => "bios".into(),
        }
    }
}

// ------------------------------------------------------------------ gamelist

pub struct GamelistEntry {
    pub path: String,
    pub name: String,
    pub image: Option<String>,
}

fn child_text(e: &Element, name: &str) -> Option<String> {
    e.get_child(name)
        .and_then(|c| c.get_text())
        .map(|t| t.into_owned())
}

fn set_child(e: &mut Element, name: &str, value: &str) {
    if e.get_child(name).is_none() {
        e.children.push(XMLNode::Element(Element::new(name)));
    }
    let c = e.get_mut_child(name).unwrap();
    c.children = vec![XMLNode::Text(value.to_string())];
}

fn load(path: &Path) -> Element {
    std::fs::read(path)
        .ok()
        .and_then(|b| Element::parse(b.as_slice()).ok())
        .unwrap_or_else(|| Element::new("gameList"))
}

fn save(path: &Path, root: &Element) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut out = vec![];
    root.write_with_config(&mut out, EmitterConfig::new().perform_indent(true))
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(path, out)
}

/// Write or merge an EmulationStation gamelist.xml. Existing entries and
/// fields we don't manage (play counts, favourites…) are kept.
pub fn write_gamelist(path: &Path, entries: &[GamelistEntry]) -> std::io::Result<()> {
    let mut root = load(path);
    for e in entries {
        let idx = root.children.iter().position(|n| {
            n.as_element().is_some_and(|g| {
                g.name == "game" && child_text(g, "path").as_deref() == Some(&e.path)
            })
        });
        let idx = match idx {
            Some(i) => i,
            None => {
                let mut g = Element::new("game");
                set_child(&mut g, "path", &e.path);
                root.children.push(XMLNode::Element(g));
                root.children.len() - 1
            }
        };
        if let XMLNode::Element(g) = &mut root.children[idx] {
            set_child(g, "name", &e.name);
            if let Some(img) = &e.image {
                set_child(g, "image", img);
                set_child(g, "thumbnail", img);
            }
        }
    }
    save(path, &root)
}

pub fn remove_from_gamelist(path: &Path, rom_paths: &[String]) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = load(path);
    root.children.retain(|n| {
        !n.as_element().is_some_and(|g| {
            g.name == "game" && child_text(g, "path").is_some_and(|p| rom_paths.contains(&p))
        })
    });
    save(path, &root)
}

pub fn gamelist_games(path: &Path) -> Vec<(String, Option<String>, Option<String>)> {
    load(path)
        .children
        .iter()
        .filter_map(|n| n.as_element())
        .filter(|g| g.name == "game")
        .map(|g| {
            (
                child_text(g, "path").unwrap_or_default(),
                child_text(g, "name"),
                child_text(g, "image"),
            )
        })
        .collect()
}

/// (path, play count, last played) for every game in a gamelist.xml.
pub fn gamelist_play_info(path: &Path) -> Vec<(String, u32, Option<String>)> {
    load(path)
        .children
        .iter()
        .filter_map(|n| n.as_element())
        .filter(|g| g.name == "game")
        .map(|g| {
            let count = child_text(g, "playcount")
                .and_then(|c| c.trim().parse().ok())
                .unwrap_or(0);
            (
                child_text(g, "path").unwrap_or_default(),
                count,
                child_text(g, "lastplayed"),
            )
        })
        .collect()
}
