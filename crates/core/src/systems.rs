//! System definitions: file extensions, aliases and per-firmware folder names.
//!
//! Folder names are each firmware's defaults. Firmware releases change these,
//! so collections can override any folder name.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Extensions used by more than one system. Files with these need folder
/// hints, archive inspection or header sniffing to place.
pub const AMBIGUOUS_EXTS: &[&str] = &[
    "gz", "zso", "bin", "iso", "img", "cue", "chd", "zip", "7z", "rom", "m3u", "ccd", "mdf", "toc",
    "cdi", "gdi", "pbp", "cso", "rvz",
];

const CD: &[&str] = &[
    "cue", "chd", "iso", "img", "ccd", "mdf", "toc", "m3u", "bin",
];
const ZIP: &[&str] = &["zip", "7z"];

#[derive(Debug)]
pub struct System {
    pub id: &'static str,
    pub name: &'static str,
    /// libretro-thumbnails playlist name.
    pub libretro: Option<&'static str>,
    /// Extensions unique to this system.
    pub exts: &'static [&'static str],
    /// Shared extensions this system also accepts.
    pub extra_exts: &'static [&'static str],
    pub aliases: &'static [&'static str],
    /// EmulationStation / ArkOS / ROCKNIX / ES-DE / muOS folder.
    pub es: &'static str,
    /// Batocera / Knulli folder.
    pub batocera: &'static str,
    /// Onion OS folder.
    pub onion: &'static str,
    /// MinUI / NextUI "Name (TAG)" folder.
    pub nextui: &'static str,
    /// muOS artwork catalogue folder.
    pub muos_catalogue: &'static str,
    pub screenscraper_id: Option<u32>,
    pub disc_based: bool,
}

impl System {
    pub fn accepts(&self, ext: &str) -> bool {
        let e = ext.to_ascii_lowercase();
        self.exts.contains(&e.as_str()) || self.extra_exts.contains(&e.as_str())
    }

    /// The tag in a NextUI folder name: "Game Boy (GB)" -> "GB".
    pub fn nextui_tag(&self) -> &'static str {
        match (self.nextui.rfind('('), self.nextui.rfind(')')) {
            (Some(a), Some(b)) if b > a => &self.nextui[a + 1..b],
            _ => self.id,
        }
    }
}

macro_rules! sys {
    ($id:expr, $name:expr, $lr:expr, $exts:expr, $extra:expr, $aliases:expr,
     $es:expr, $bato:expr, $onion:expr, $nextui:expr, $muos:expr, $ss:expr, $disc:expr) => {
        System {
            id: $id,
            name: $name,
            libretro: $lr,
            exts: $exts,
            extra_exts: $extra,
            aliases: $aliases,
            es: $es,
            batocera: $bato,
            onion: $onion,
            nextui: $nextui,
            muos_catalogue: $muos,
            screenscraper_id: $ss,
            disc_based: $disc,
        }
    };
}

pub static SYSTEMS: &[System] = &[
    sys!(
        "nes",
        "Nintendo Entertainment System",
        Some("Nintendo - Nintendo Entertainment System"),
        &["nes", "unf", "unif"],
        ZIP,
        &["famicom", "fc", "nes"],
        "nes",
        "nes",
        "FC",
        "Nintendo Entertainment System (FC)",
        "Nintendo NES - Famicom",
        Some(3),
        false
    ),
    sys!(
        "fds",
        "Famicom Disk System",
        Some("Nintendo - Family Computer Disk System"),
        &["fds"],
        ZIP,
        &["fds", "famicomdisk"],
        "fds",
        "fds",
        "FDS",
        "Famicom Disk System (FDS)",
        "Nintendo FDS",
        Some(106),
        false
    ),
    sys!(
        "snes",
        "Super Nintendo",
        Some("Nintendo - Super Nintendo Entertainment System"),
        &["sfc", "smc", "swc", "fig"],
        ZIP,
        &["superfamicom", "sfc", "supernintendo", "snes"],
        "snes",
        "snes",
        "SFC",
        "Super Nintendo Entertainment System (SFC)",
        "Nintendo SNES - SFC",
        Some(4),
        false
    ),
    sys!(
        "gb",
        "Game Boy",
        Some("Nintendo - Game Boy"),
        &["gb"],
        ZIP,
        &["gameboy", "gb"],
        "gb",
        "gb",
        "GB",
        "Game Boy (GB)",
        "Nintendo Game Boy",
        Some(9),
        false
    ),
    sys!(
        "gbc",
        "Game Boy Color",
        Some("Nintendo - Game Boy Color"),
        &["gbc"],
        ZIP,
        &["gameboycolor", "gbc"],
        "gbc",
        "gbc",
        "GBC",
        "Game Boy Color (GBC)",
        "Nintendo Game Boy Color",
        Some(10),
        false
    ),
    sys!(
        "gba",
        "Game Boy Advance",
        Some("Nintendo - Game Boy Advance"),
        &["gba"],
        ZIP,
        &["gameboyadvance", "gba"],
        "gba",
        "gba",
        "GBA",
        "Game Boy Advance (GBA)",
        "Nintendo Game Boy Advance",
        Some(12),
        false
    ),
    sys!(
        "n64",
        "Nintendo 64",
        Some("Nintendo - Nintendo 64"),
        &["n64", "z64", "v64"],
        ZIP,
        &["nintendo64", "n64"],
        "n64",
        "n64",
        "N64",
        "Nintendo 64 (N64)",
        "Nintendo N64",
        Some(14),
        false
    ),
    sys!(
        "nds",
        "Nintendo DS",
        Some("Nintendo - Nintendo DS"),
        &["nds"],
        ZIP,
        &["nintendods", "ds", "nds"],
        "nds",
        "nds",
        "NDS",
        "Nintendo DS (NDS)",
        "Nintendo DS",
        Some(15),
        false
    ),
    sys!(
        "virtualboy",
        "Virtual Boy",
        Some("Nintendo - Virtual Boy"),
        &["vb", "vboy"],
        ZIP,
        &["virtualboy", "vb"],
        "virtualboy",
        "virtualboy",
        "VB",
        "Virtual Boy (VB)",
        "Nintendo Virtual Boy",
        Some(11),
        false
    ),
    sys!(
        "pokemini",
        "Pokemon Mini",
        Some("Nintendo - Pokemon Mini"),
        &["min"],
        ZIP,
        &["pokemonmini", "pokemini", "pkm"],
        "pokemini",
        "pokemini",
        "POKE",
        "Pokemon mini (PKM)",
        "Nintendo Pokemon Mini",
        Some(211),
        false
    ),
    sys!(
        "genesis",
        "Sega Genesis / Mega Drive",
        Some("Sega - Mega Drive - Genesis"),
        &["md", "gen", "smd"],
        &["bin", "zip", "7z"],
        &["megadrive", "genesis", "md", "segagenesis", "segamegadrive"],
        "genesis",
        "megadrive",
        "MD",
        "Sega Genesis (MD)",
        "Sega Mega Drive - Genesis",
        Some(1),
        false
    ),
    sys!(
        "sega32x",
        "Sega 32X",
        Some("Sega - 32X"),
        &["32x"],
        &["bin", "zip", "7z"],
        &["32x", "sega32x"],
        "sega32x",
        "sega32x",
        "THIRTYTWOX",
        "Sega 32X (32X)",
        "Sega 32X",
        Some(19),
        false
    ),
    sys!(
        "mastersystem",
        "Sega Master System",
        Some("Sega - Master System - Mark III"),
        &["sms"],
        ZIP,
        &["mastersystem", "sms", "markiii"],
        "mastersystem",
        "mastersystem",
        "MS",
        "Sega Master System (SMS)",
        "Sega Master System",
        Some(2),
        false
    ),
    sys!(
        "gamegear",
        "Sega Game Gear",
        Some("Sega - Game Gear"),
        &["gg"],
        &["sms", "zip", "7z"],
        &["gamegear", "gg"],
        "gamegear",
        "gamegear",
        "GG",
        "Sega Game Gear (GG)",
        "Sega Game Gear",
        Some(21),
        false
    ),
    sys!(
        "sg1000",
        "Sega SG-1000",
        Some("Sega - SG-1000"),
        &["sg"],
        ZIP,
        &["sg1000"],
        "sg-1000",
        "sg1000",
        "SEGASGONE",
        "Sega SG-1000 (SG1000)",
        "Sega SG-1000",
        Some(109),
        false
    ),
    sys!(
        "segacd",
        "Sega CD / Mega-CD",
        Some("Sega - Mega-CD - Sega CD"),
        &[],
        CD,
        &["segacd", "megacd", "mcd"],
        "segacd",
        "segacd",
        "SEGACD",
        "Sega CD (SEGACD)",
        "Sega Mega CD - Sega CD",
        Some(20),
        true
    ),
    sys!(
        "saturn",
        "Sega Saturn",
        Some("Sega - Saturn"),
        &[],
        CD,
        &["saturn", "segasaturn"],
        "saturn",
        "saturn",
        "SATURN",
        "Sega Saturn (SATURN)",
        "Sega Saturn",
        Some(22),
        true
    ),
    sys!(
        "dreamcast",
        "Sega Dreamcast",
        Some("Sega - Dreamcast"),
        &[],
        &["gdi", "cdi", "chd", "m3u"],
        &["dreamcast", "dc"],
        "dreamcast",
        "dreamcast",
        "DC",
        "Sega Dreamcast (DC)",
        "Sega Dreamcast",
        Some(23),
        true
    ),
    sys!(
        "psx",
        "Sony PlayStation",
        Some("Sony - PlayStation"),
        &[],
        &["cue", "chd", "iso", "img", "ccd", "mdf", "toc", "m3u", "bin", "pbp"],
        &["playstation", "psx", "ps1", "ps", "sonyplaystation"],
        "psx",
        "psx",
        "PS",
        "Sony PlayStation (PS)",
        "Sony PlayStation",
        Some(57),
        true
    ),
    sys!(
        "psp",
        "Sony PlayStation Portable",
        Some("Sony - PlayStation Portable"),
        &[],
        &["iso", "cso", "pbp", "chd"],
        &["psp", "playstationportable"],
        "psp",
        "psp",
        "PSP",
        "Sony PlayStation Portable (PSP)",
        "Sony PlayStation Portable",
        Some(61),
        true
    ),
    sys!(
        "pcengine",
        "PC Engine / TurboGrafx-16",
        Some("NEC - PC Engine - TurboGrafx 16"),
        &["pce"],
        ZIP,
        &["pcengine", "pce", "turbografx16", "tg16", "turbografx"],
        "pcengine",
        "pcengine",
        "PCE",
        "TurboGrafx-16 (PCE)",
        "NEC PC Engine",
        Some(31),
        false
    ),
    sys!(
        "pcenginecd",
        "PC Engine CD / TurboGrafx-CD",
        Some("NEC - PC Engine CD - TurboGrafx-CD"),
        &[],
        CD,
        &["pcenginecd", "pcecd", "turbografxcd", "tgcd"],
        "pcenginecd",
        "pcenginecd",
        "PCECD",
        "TurboGrafx-CD (PCECD)",
        "NEC PC Engine CD",
        Some(114),
        true
    ),
    sys!(
        "atari2600",
        "Atari 2600",
        Some("Atari - 2600"),
        &["a26"],
        &["bin", "zip", "7z"],
        &["atari2600", "2600", "a2600", "atari"],
        "atari2600",
        "atari2600",
        "ATARI",
        "Atari 2600 (A2600)",
        "Atari 2600",
        Some(26),
        false
    ),
    sys!(
        "atari7800",
        "Atari 7800",
        Some("Atari - 7800"),
        &["a78"],
        ZIP,
        &["atari7800", "7800", "a7800"],
        "atari7800",
        "atari7800",
        "SEVENTYEIGHTHUNDRED",
        "Atari 7800 (A7800)",
        "Atari 7800",
        Some(41),
        false
    ),
    sys!(
        "atarilynx",
        "Atari Lynx",
        Some("Atari - Lynx"),
        &["lnx", "lyx"],
        ZIP,
        &["lynx", "atarilynx"],
        "atarilynx",
        "lynx",
        "LYNX",
        "Atari Lynx (LYNX)",
        "Atari Lynx",
        Some(28),
        false
    ),
    sys!(
        "ngp",
        "Neo Geo Pocket",
        Some("SNK - Neo Geo Pocket"),
        &["ngp"],
        ZIP,
        &["neogeopocket", "ngp"],
        "ngp",
        "ngp",
        "NGP",
        "Neo Geo Pocket (NGP)",
        "SNK Neo Geo Pocket - Color",
        Some(25),
        false
    ),
    sys!(
        "ngpc",
        "Neo Geo Pocket Color",
        Some("SNK - Neo Geo Pocket Color"),
        &["ngc", "npc"],
        ZIP,
        &["neogeopocketcolor", "ngpc"],
        "ngpc",
        "ngpc",
        "NGP",
        "Neo Geo Pocket Color (NGPC)",
        "SNK Neo Geo Pocket - Color",
        Some(82),
        false
    ),
    sys!(
        "wonderswan",
        "WonderSwan",
        Some("Bandai - WonderSwan"),
        &["ws"],
        ZIP,
        &["wonderswan", "ws", "wswan"],
        "wonderswan",
        "wswan",
        "WS",
        "WonderSwan (WS)",
        "Bandai WonderSwan - Color",
        Some(45),
        false
    ),
    sys!(
        "wonderswancolor",
        "WonderSwan Color",
        Some("Bandai - WonderSwan Color"),
        &["wsc"],
        ZIP,
        &["wonderswancolor", "wsc", "wswanc"],
        "wonderswancolor",
        "wswanc",
        "WS",
        "WonderSwan Color (WSC)",
        "Bandai WonderSwan - Color",
        Some(46),
        false
    ),
    sys!(
        "colecovision",
        "ColecoVision",
        Some("Coleco - ColecoVision"),
        &["col"],
        &["rom", "zip", "7z"],
        &["colecovision", "coleco"],
        "colecovision",
        "colecovision",
        "COLECO",
        "ColecoVision (COLECO)",
        "ColecoVision",
        Some(48),
        false
    ),
    sys!(
        "msx",
        "MSX",
        Some("Microsoft - MSX"),
        &["mx1", "mx2", "dsk"],
        &["rom", "zip", "7z"],
        &["msx", "msx1", "msx2"],
        "msx",
        "msx1",
        "MSX",
        "MSX (MSX)",
        "Microsoft - MSX",
        Some(113),
        false
    ),
    sys!(
        "vectrex",
        "Vectrex",
        Some("GCE - Vectrex"),
        &["vec"],
        &["bin", "zip", "7z"],
        &["vectrex"],
        "vectrex",
        "vectrex",
        "VECTREX",
        "Vectrex (VECTREX)",
        "GCE-Vectrex",
        Some(102),
        false
    ),
    sys!(
        "pico8",
        "PICO-8",
        None,
        &["p8"],
        &[],
        &["pico8", "pico"],
        "pico8",
        "pico8",
        "PICO",
        "Pico-8 (P8)",
        "PICO-8",
        Some(234),
        false
    ),
    sys!(
        "ps2",
        "Sony PlayStation 2",
        Some("Sony - PlayStation 2"),
        &[],
        &["iso", "chd", "cso", "zso", "gz", "bin", "cue", "m3u"],
        &["playstation2", "ps2", "sonyplaystation2"],
        "ps2",
        "ps2",
        "PS2",
        "Sony PlayStation 2 (PS2)",
        "Sony PlayStation 2",
        Some(58),
        true
    ),
    sys!(
        "gamecube",
        "Nintendo GameCube",
        Some("Nintendo - GameCube"),
        &["gcm", "gcz"],
        &["iso", "rvz", "ciso", "m3u"],
        &["gamecube", "gc", "ngc", "nintendogamecube"],
        "gc",
        "gamecube",
        "GC",
        "Nintendo GameCube (GC)",
        "Nintendo GameCube",
        Some(13),
        true
    ),
    sys!(
        "n3ds",
        "Nintendo 3DS",
        Some("Nintendo - Nintendo 3DS"),
        &["3ds", "cia", "cci", "cxi", "3dsx"],
        ZIP,
        &["3ds", "n3ds", "nintendo3ds"],
        "n3ds",
        "3ds",
        "3DS",
        "Nintendo 3DS (3DS)",
        "Nintendo 3DS",
        Some(17),
        false
    ),
    sys!(
        "xbox",
        "Microsoft Xbox",
        Some("Microsoft - Xbox"),
        &["xiso"],
        &["iso"],
        &["xbox", "microsoftxbox", "xemu"],
        "xbox",
        "xbox",
        "XBOX",
        "Microsoft Xbox (XBOX)",
        "Microsoft Xbox",
        Some(32),
        true
    ),
    sys!(
        "dos",
        "DOS",
        Some("DOS"),
        &["dosz"],
        &["zip", "7z"],
        &["dos", "msdos", "pc", "dosbox", "exodos", "ibmpc"],
        "dos",
        "dos",
        "DOS",
        "DOS (DOS)",
        "DOS",
        Some(135),
        false
    ),
    sys!(
        "scummvm",
        "ScummVM",
        Some("ScummVM"),
        &["scummvm", "svm"],
        &[],
        &["scummvm", "scumm", "svm"],
        "scummvm",
        "scummvm",
        "SCUMMVM",
        "ScummVM (SCUMMVM)",
        "ScummVM",
        Some(123),
        false
    ),
    sys!(
        "arcade",
        "Arcade (FBNeo)",
        Some("FBNeo - Arcade Games"),
        &[],
        ZIP,
        &[
            "arcade",
            "fbneo",
            "fba",
            "finalburn",
            "finalburnneo",
            "neogeo"
        ],
        "arcade",
        "fbneo",
        "ARCADE",
        "Arcade (FBN)",
        "Arcade",
        Some(75),
        false
    ),
    sys!(
        "mame",
        "Arcade (MAME)",
        Some("MAME"),
        &[],
        ZIP,
        &["mame", "mame2003", "mame2003plus", "mame2010", "mame2000"],
        "mame",
        "mame",
        "MAME",
        "MAME (MAME)",
        "Arcade",
        Some(75),
        false
    ),
    sys!(
        "cps1",
        "Capcom CPS-1",
        Some("FBNeo - Arcade Games"),
        &[],
        ZIP,
        &["cps1", "cps"],
        "cps1",
        "cps1",
        "CPS1",
        "Capcom Play System (CPS1)",
        "Capcom Play System",
        Some(6),
        false
    ),
    sys!(
        "cps2",
        "Capcom CPS-2",
        Some("FBNeo - Arcade Games"),
        &[],
        ZIP,
        &["cps2"],
        "cps2",
        "cps2",
        "CPS2",
        "Capcom Play System II (CPS2)",
        "Capcom Play System 2",
        Some(7),
        false
    ),
    sys!(
        "cps3",
        "Capcom CPS-3",
        Some("FBNeo - Arcade Games"),
        &[],
        ZIP,
        &["cps3"],
        "cps3",
        "cps3",
        "CPS3",
        "Capcom Play System III (CPS3)",
        "Capcom Play System 3",
        Some(8),
        false
    ),
];

pub fn get(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id == id)
}

pub fn name_of(id: &str) -> &str {
    get(id).map(|s| s.name).unwrap_or(id)
}

fn norm(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn alias_map() -> &'static HashMap<String, &'static System> {
    static MAP: OnceLock<HashMap<String, &'static System>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut out = HashMap::new();
        for s in SYSTEMS {
            let mut names = vec![
                s.id,
                s.name,
                s.es,
                s.batocera,
                s.onion,
                s.nextui,
                s.nextui_tag(),
            ];
            names.extend_from_slice(s.aliases);
            if let Some(l) = s.libretro {
                names.push(l);
            }
            for n in names {
                let k = norm(n);
                if !k.is_empty() {
                    out.entry(k).or_insert(s);
                }
            }
        }
        out
    })
}

/// System whose unique extension this is (".gba" -> Game Boy Advance).
pub fn by_unique_ext(ext: &str) -> Option<&'static System> {
    static MAP: OnceLock<HashMap<&'static str, &'static System>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        let mut m = HashMap::new();
        for s in SYSTEMS {
            for e in s.exts {
                m.entry(*e).or_insert(s);
            }
        }
        m
    });
    map.get(ext.to_ascii_lowercase().as_str()).copied()
}

/// Map a folder name like "Game Boy (GB)", "megadrive" or "PS1" to a system.
pub fn from_folder_name(name: &str) -> Option<&'static System> {
    let map = alias_map();
    if let Some(s) = map.get(&norm(name)) {
        return Some(s);
    }
    let trimmed = name.trim_end();
    if trimmed.ends_with(')') {
        if let Some(a) = trimmed.rfind('(') {
            return map.get(&norm(&trimmed[a + 1..trimmed.len() - 1])).copied();
        }
    }
    None
}

/// Systems whose games are arcade romsets (zips named by MAME short name).
pub fn is_arcade(id: &str) -> bool {
    matches!(id, "arcade" | "mame" | "cps1" | "cps2" | "cps3")
}

/// Systems whose games can be whole folders rather than single files.
pub fn folder_games(id: &str) -> bool {
    matches!(id, "dos" | "scummvm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names() {
        assert_eq!(from_folder_name("Game Boy (GB)").unwrap().id, "gb");
        assert_eq!(from_folder_name("megadrive").unwrap().id, "genesis");
        assert_eq!(from_folder_name("PS1").unwrap().id, "psx");
        assert_eq!(from_folder_name("Atari 2600").unwrap().id, "atari2600");
        assert!(from_folder_name("Downloads").is_none());
        assert_eq!(by_unique_ext("GBA").unwrap().id, "gba");
        assert!(by_unique_ext("bin").is_none());
    }
}
