//! System definitions: file extensions, aliases and per-firmware folder names.
//!
//! Folder names are each firmware's defaults. Firmware releases change these,
//! so collections can override any folder name.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Extensions used by more than one system. Files with these need folder
/// hints, archive inspection or header sniffing to place.
pub const AMBIGUOUS_EXTS: &[&str] = &[
    "gz", "zso", "dsk", "bin", "iso", "img", "cue", "chd", "zip", "7z", "rom", "m3u", "ccd", "mdf",
    "toc", "cdi", "gdi", "pbp", "cso", "rvz",
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
        &["mx1"],
        &["rom", "dsk", "mx2", "zip", "7z"],
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
        &["arcade", "fbneo", "fba", "finalburn", "finalburnneo"],
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
    sys!(
        "3do",
        "3DO Interactive Multiplayer",
        Some("The 3DO Company - 3DO"),
        &[],
        &["7z", "bin", "chd", "cue", "iso", "zip"],
        &["3do"],
        "3do",
        "3do",
        "3DO",
        "3DO Interactive Multiplayer (3DO)",
        "3DO Interactive Multiplayer",
        Some(29),
        true
    ),
    sys!(
        "adam",
        "Coleco Adam",
        None,
        &["ddp"],
        &[
            "1dd", "7z", "bin", "col", "cqi", "cqm", "d77", "d88", "dfi", "dsk", "hfe", "imd",
            "mfi", "mfm", "rom", "td0", "wav", "zip"
        ],
        &["adam"],
        "adam",
        "adam",
        "ADAM",
        "Coleco Adam (ADAM)",
        "Coleco Adam",
        Some(89),
        false
    ),
    sys!(
        "amiga",
        "Commodore Amiga",
        Some("Commodore - Amiga"),
        &["adf", "adz", "dms"],
        &[
            "7z", "ccd", "chd", "cue", "fdi", "hdf", "hdz", "ipf", "iso", "lha", "m3u", "mds",
            "nrg", "rp9", "uae", "zip"
        ],
        &["amiga", "amiga1200", "amiga500", "amiga600"],
        "amiga",
        "amiga500",
        "AMIGA",
        "Commodore Amiga (AMIGA)",
        "Commodore Amiga",
        Some(64),
        false
    ),
    sys!(
        "amigacd32",
        "Commodore Amiga CD32",
        Some("Commodore - CD32"),
        &[],
        &[
            "7z", "adf", "adz", "ccd", "chd", "cue", "dms", "fdi", "hdf", "hdz", "ipf", "iso",
            "lha", "m3u", "mds", "nrg", "rp9", "uae", "zip"
        ],
        &["amigacd32"],
        "amigacd32",
        "amigacd32",
        "AMIGACD32",
        "Commodore Amiga CD32 (AMIGACD32)",
        "Commodore Amiga CD32",
        Some(130),
        true
    ),
    sys!(
        "amstradcpc",
        "Amstrad CPC",
        Some("Amstrad - CPC"),
        &["cdt", "cpr"],
        &["7z", "dsk", "kcr", "m3u", "sna", "tap", "tar", "voc", "zip"],
        &["amstradcpc"],
        "amstradcpc",
        "amstradcpc",
        "AMSTRADCPC",
        "Amstrad CPC (AMSTRADCPC)",
        "Amstrad CPC",
        Some(65),
        false
    ),
    sys!(
        "apple2",
        "Apple II",
        Some("Apple - II"),
        &["do", "po"],
        &["7z", "dsk", "nib", "zip"],
        &["apple2"],
        "apple2",
        "apple2",
        "APPLE2",
        "Apple II (APPLE2)",
        "Apple II",
        Some(86),
        false
    ),
    sys!(
        "apple2gs",
        "Apple IIGS",
        None,
        &["2mg"],
        &["7z", "zip"],
        &["apple2gs"],
        "apple2gs",
        "apple2gs",
        "APPLE2GS",
        "Apple IIGS (APPLE2GS)",
        "Apple IIGS",
        Some(217),
        false
    ),
    sys!(
        "arcadia",
        "Emerson Arcadia 2001",
        Some("Emerson - Arcadia 2001"),
        &["tvc"],
        &["7z", "bin", "zip"],
        &["arcadia"],
        "arcadia",
        "arcadia",
        "ARCADIA",
        "Emerson Arcadia 2001 (ARCADIA)",
        "Emerson Arcadia 2001",
        Some(94),
        false
    ),
    sys!(
        "archimedes",
        "Acorn Archimedes",
        None,
        &["apd", "ima", "jfd", "ufi"],
        &[
            "1dd", "360", "7z", "adf", "adl", "adm", "ads", "bbc", "chd", "cqi", "cqm", "d77",
            "d88", "dfi", "dsd", "dsk", "hfe", "imd", "img", "ipf", "mfi", "mfm", "msa", "ssd",
            "st", "td0", "zip"
        ],
        &["archimedes"],
        "archimedes",
        "archimedes",
        "ARCHIMEDES",
        "Acorn Archimedes (ARCHIMEDES)",
        "Acorn Archimedes",
        Some(84),
        false
    ),
    sys!(
        "arduboy",
        "Arduboy Miniature Game System",
        Some("Arduboy Inc - Arduboy"),
        &["arduboy", "hex"],
        &["7z", "zip"],
        &["arduboy"],
        "arduboy",
        "arduboy",
        "ARDUBOY",
        "Arduboy Miniature Game System (ARDUBOY)",
        "Arduboy Miniature Game System",
        Some(263),
        false
    ),
    sys!(
        "astrocde",
        "Bally Astrocade",
        None,
        &[],
        &["7z", "zip"],
        &["astrocade", "astrocde"],
        "astrocde",
        "astrocade",
        "ASTROCDE",
        "Bally Astrocade (ASTROCDE)",
        "Bally Astrocade",
        Some(44),
        false
    ),
    sys!(
        "atari5200",
        "Atari 5200",
        Some("Atari - 5200"),
        &["a52"],
        &["7z", "atr", "atx", "bin", "car", "cas", "cdm", "com", "rom", "xex", "xfd", "zip"],
        &["atari5200"],
        "atari5200",
        "atari5200",
        "ATARI5200",
        "Atari 5200 (ATARI5200)",
        "Atari 5200",
        Some(40),
        false
    ),
    sys!(
        "atari800",
        "Atari 800",
        None,
        &["atr", "xex", "xfd", "atx"],
        &["7z", "a52", "bin", "car", "cas", "cdm", "com", "rom", "zip"],
        &["atari800", "atarixe"],
        "atari800",
        "atari800",
        "ATARI800",
        "Atari 800 (ATARI800)",
        "Atari 800",
        Some(43),
        false
    ),
    sys!(
        "atarijaguar",
        "Atari Jaguar",
        Some("Atari - Jaguar"),
        &["j64", "jag"],
        &["7z", "abs", "bigpimg", "bin", "cdi", "cof", "cue", "prg", "rom", "zip"],
        &["atarijaguar", "jaguar"],
        "atarijaguar",
        "jaguar",
        "ATARIJAGUAR",
        "Atari Jaguar (ATARIJAGUAR)",
        "Atari Jaguar",
        Some(27),
        false
    ),
    sys!(
        "atarijaguarcd",
        "Atari Jaguar CD",
        Some("Atari - Jaguar CD"),
        &[],
        &["abs", "bigpimg", "bin", "cdi", "cof", "cue", "j64", "jag", "prg", "rom", "zip", "7z"],
        &["atarijaguarcd", "jaguarcd"],
        "atarijaguarcd",
        "jaguarcd",
        "ATARIJAGUARCD",
        "Atari Jaguar CD (ATARIJAGUARCD)",
        "Atari Jaguar CD",
        Some(171),
        true
    ),
    sys!(
        "atarist",
        "Atari ST",
        Some("Atari - ST"),
        &["stx", "msa"],
        &["7z", "dim", "ipf", "m3u", "st", "zip"],
        &["atarist"],
        "atarist",
        "atarist",
        "ATARIST",
        "Atari ST (ATARIST)",
        "Atari ST",
        Some(42),
        false
    ),
    sys!(
        "atomiswave",
        "Sammy Corporation Atomiswave",
        Some("Atomiswave"),
        &[],
        &["7z", "bin", "dat", "elf", "lst", "zip"],
        &["atomiswave"],
        "atomiswave",
        "atomiswave",
        "ATOMISWAVE",
        "Sammy Corporation Atomiswave (ATOMISWAVE)",
        "Sammy Corporation Atomiswave",
        Some(75),
        false
    ),
    sys!(
        "bbcmicro",
        "Acorn Computers BBC Micro",
        None,
        &[],
        &["7z", "dsd", "img", "ssd", "zip"],
        &["bbcmicro"],
        "bbcmicro",
        "bbcmicro",
        "BBCMICRO",
        "Acorn Computers BBC Micro (BBCMICRO)",
        "Acorn Computers BBC Micro",
        Some(37),
        false
    ),
    sys!(
        "c64",
        "Commodore 64",
        Some("Commodore - 64"),
        &["d64", "t64", "crt", "g64", "d71", "d81"],
        &[
            "7z", "bin", "cmd", "d2m", "d4m", "d6z", "d7z", "d80", "d82", "d8z", "g41", "g4z",
            "g6z", "gz", "lnx", "m3u", "nbz", "nib", "p00", "prg", "tap", "vfl", "vsf", "x64",
            "x6z", "zip"
        ],
        &["c64"],
        "c64",
        "c64",
        "C64",
        "Commodore 64 (C64)",
        "Commodore 64",
        Some(66),
        false
    ),
    sys!(
        "cdimono1",
        "Philips CD-i",
        Some("Philips - CD-i"),
        &[],
        &["chd", "cue", "iso", "zip", "7z"],
        &["cdi", "cdimono1"],
        "cdimono1",
        "cdi",
        "CDIMONO1",
        "Philips CD-i (CDIMONO1)",
        "Philips CD-i",
        Some(133),
        true
    ),
    sys!(
        "cdtv",
        "Commodore CDTV",
        Some("Commodore - CDTV"),
        &[],
        &[
            "7z", "adf", "adz", "ccd", "chd", "cue", "dms", "fdi", "hdf", "hdz", "ipf", "iso",
            "lha", "m3u", "mds", "nrg", "rp9", "uae", "zip"
        ],
        &["amigacdtv", "cdtv"],
        "cdtv",
        "amigacdtv",
        "CDTV",
        "Commodore CDTV (CDTV)",
        "Commodore CDTV",
        Some(129),
        true
    ),
    sys!(
        "chailove",
        "ChaiLove Game Engine",
        Some("ChaiLove"),
        &["chai", "chailove"],
        &["7z", "zip"],
        &["chailove"],
        "chailove",
        "chailove",
        "CHAILOVE",
        "ChaiLove Game Engine (CHAILOVE)",
        "ChaiLove Game Engine",
        None,
        false
    ),
    sys!(
        "channelf",
        "Fairchild Channel F",
        Some("Fairchild - Channel F"),
        &["chf"],
        &["7z", "bin", "zip"],
        &["channelf"],
        "channelf",
        "channelf",
        "CHANNELF",
        "Fairchild Channel F (CHANNELF)",
        "Fairchild Channel F",
        Some(80),
        false
    ),
    sys!(
        "coco",
        "Tandy Color Computer",
        None,
        &["ccc"],
        &["cas", "dsk", "rom", "zip", "7z"],
        &["coco"],
        "coco",
        "coco",
        "COCO",
        "Tandy Color Computer (COCO)",
        "Tandy Color Computer",
        Some(144),
        false
    ),
    sys!(
        "crvision",
        "VTech CreatiVision",
        Some("VTech - CreatiVision"),
        &[],
        &["7z", "bin", "rom", "zip"],
        &["crvision"],
        "crvision",
        "crvision",
        "CRVISION",
        "VTech CreatiVision (CRVISION)",
        "VTech CreatiVision",
        Some(241),
        false
    ),
    sys!(
        "doom",
        "Doom",
        Some("DOOM"),
        &["ipk3", "pk4", "pk3"],
        &["desktop", "iwad", "pwad", "sh", "wad", "zip", "7z"],
        &["doom", "prboom", "uzdoom"],
        "doom",
        "prboom",
        "DOOM",
        "Doom (DOOM)",
        "Doom",
        None,
        false
    ),
    sys!(
        "dragon32",
        "Dragon Data Dragon 32",
        None,
        &[],
        &["7z", "cas", "ccc", "dsk", "rom", "zip"],
        &["dragon32", "dragon64", "tanodragon"],
        "dragon32",
        "dragon64",
        "DRAGON32",
        "Dragon Data Dragon 32 (DRAGON32)",
        "Dragon Data Dragon 32",
        Some(91),
        false
    ),
    sys!(
        "easyrpg",
        "EasyRPG Game Engine",
        Some("RPG Maker"),
        &["easyrpg"],
        &["zip", "7z"],
        &["easyrpg"],
        "easyrpg",
        "easyrpg",
        "EASYRPG",
        "EasyRPG Game Engine (EASYRPG)",
        "EasyRPG Game Engine",
        Some(231),
        false
    ),
    sys!(
        "electron",
        "Acorn Electron",
        None,
        &["csw", "uef"],
        &[
            "1dd", "7z", "adf", "adl", "adm", "ads", "bbc", "bin", "cqi", "cqm", "d77", "d88",
            "dfi", "dsd", "dsk", "hfe", "imd", "img", "mfi", "mfm", "rom", "ssd", "td0", "wav",
            "zip"
        ],
        &["electron"],
        "electron",
        "electron",
        "ELECTRON",
        "Acorn Electron (ELECTRON)",
        "Acorn Electron",
        Some(85),
        false
    ),
    sys!(
        "flash",
        "Adobe Flash",
        None,
        &["ruf", "swf"],
        &["zip", "7z"],
        &["flash"],
        "flash",
        "flash",
        "FLASH",
        "Adobe Flash (FLASH)",
        "Adobe Flash",
        None,
        false
    ),
    sys!(
        "fm7",
        "Fujitsu FM-7",
        None,
        &["t77"],
        &[
            "1dd", "7z", "cqi", "cqm", "d77", "d88", "dfi", "dsk", "hfe", "imd", "mfi", "mfm",
            "td0", "wav", "zip"
        ],
        &["fm7"],
        "fm7",
        "fm7",
        "FM7",
        "Fujitsu FM-7 (FM7)",
        "Fujitsu FM-7",
        Some(97),
        false
    ),
    sys!(
        "fmtowns",
        "Fujitsu FM Towns",
        Some("Fujitsu - FM Towns"),
        &["cdr"],
        &["chd", "cue", "gdi", "iso", "zip", "7z"],
        &["fmtowns"],
        "fmtowns",
        "fmtowns",
        "FMTOWNS",
        "Fujitsu FM Towns (FMTOWNS)",
        "Fujitsu FM Towns",
        Some(253),
        true
    ),
    sys!(
        "fpinball",
        "Future Pinball",
        None,
        &["fpt"],
        &["zip", "7z"],
        &["fpinball"],
        "fpinball",
        "fpinball",
        "FPINBALL",
        "Future Pinball (FPINBALL)",
        "Future Pinball",
        Some(199),
        false
    ),
    sys!(
        "gamate",
        "Bit Corporation Gamate",
        None,
        &[],
        &["7z", "bin", "zip"],
        &["gamate"],
        "gamate",
        "gamate",
        "GAMATE",
        "Bit Corporation Gamate (GAMATE)",
        "Bit Corporation Gamate",
        Some(266),
        false
    ),
    sys!(
        "gameandwatch",
        "Game & Watch / LCD games",
        Some("Handheld Electronic Game"),
        &["mgw"],
        &["7z", "zip"],
        &["gameandwatch", "lcdgames"],
        "gameandwatch",
        "gameandwatch",
        "GAMEANDWATCH",
        "Game & Watch / LCD games (GAMEANDWATCH)",
        "Game & Watch / LCD games",
        Some(52),
        false
    ),
    sys!(
        "gamecom",
        "Tiger Electronics Game.com",
        Some("Tiger - Game.com"),
        &["tgc"],
        &["7z", "zip"],
        &["gamecom"],
        "gamecom",
        "gamecom",
        "GAMECOM",
        "Tiger Electronics Game.com (GAMECOM)",
        "Tiger Electronics Game.com",
        Some(121),
        false
    ),
    sys!(
        "gmaster",
        "Hartung Game Master",
        Some("Hartung - Game Master"),
        &[],
        &["7z", "bin", "zip"],
        &["gmaster"],
        "gmaster",
        "gmaster",
        "GMASTER",
        "Hartung Game Master (GMASTER)",
        "Hartung Game Master",
        Some(103),
        false
    ),
    sys!(
        "gx4000",
        "Amstrad GX4000",
        Some("Amstrad - GX4000"),
        &[],
        &["7z", "bin", "cdt", "cpr", "dsk", "kcr", "m3u", "sna", "tap", "tar", "voc", "zip"],
        &["gx4000"],
        "gx4000",
        "gx4000",
        "GX4000",
        "Amstrad GX4000 (GX4000)",
        "Amstrad GX4000",
        Some(87),
        false
    ),
    sys!(
        "intellivision",
        "Mattel Electronics Intellivision",
        Some("Mattel - Intellivision"),
        &["int"],
        &["7z", "bin", "rom", "zip"],
        &["intellivision"],
        "intellivision",
        "intellivision",
        "INTELLIVISION",
        "Mattel Electronics Intellivision (INTELLIVISION)",
        "Mattel Electronics Intellivision",
        Some(115),
        false
    ),
    sys!(
        "j2me",
        "Java 2 Micro Edition (J2ME)",
        None,
        &[],
        &["7z", "jar", "zip"],
        &["j2me"],
        "j2me",
        "j2me",
        "J2ME",
        "Java 2 Micro Edition (J2ME) (J2ME)",
        "Java 2 Micro Edition (J2ME)",
        Some(302),
        false
    ),
    sys!(
        "laserdisc",
        "LaserDisc (Daphne / Singe)",
        None,
        &["daphne", "dirksimple", "mmi", "singe"],
        &["7z", "ogv", "zip"],
        &["daphne", "laserdisc", "singe"],
        "laserdisc",
        "daphne",
        "LASERDISC",
        "LaserDisc (Daphne / Singe) (LASERDISC)",
        "LaserDisc (Daphne / Singe)",
        None,
        false
    ),
    sys!(
        "lowresnx",
        "LowRes NX Fantasy Console",
        Some("LowRes NX"),
        &["nx"],
        &["zip", "7z"],
        &["lowresnx"],
        "lowresnx",
        "lowresnx",
        "LOWRESNX",
        "LowRes NX Fantasy Console (LOWRESNX)",
        "LowRes NX Fantasy Console",
        Some(244),
        false
    ),
    sys!(
        "lutro",
        "Lutro Game Engine",
        Some("Lutro"),
        &["lutro"],
        &["7z", "lua", "zip"],
        &["lutro"],
        "lutro",
        "lutro",
        "LUTRO",
        "Lutro Game Engine (LUTRO)",
        "Lutro Game Engine",
        Some(206),
        false
    ),
    sys!(
        "macintosh",
        "Apple Macintosh",
        None,
        &[],
        &["dsk", "game", "img", "zip", "7z"],
        &["macintosh"],
        "macintosh",
        "macintosh",
        "MACINTOSH",
        "Apple Macintosh (MACINTOSH)",
        "Apple Macintosh",
        Some(146),
        false
    ),
    sys!(
        "megaduck",
        "Creatronic Mega Duck",
        None,
        &[],
        &["7z", "bin", "zip"],
        &["megaduck"],
        "megaduck",
        "megaduck",
        "MEGADUCK",
        "Creatronic Mega Duck (MEGADUCK)",
        "Creatronic Mega Duck",
        Some(90),
        false
    ),
    sys!(
        "mess",
        "Multi Emulator Super System",
        None,
        &[],
        &["7z", "chd", "zip"],
        &["mess"],
        "mess",
        "mess",
        "MESS",
        "Multi Emulator Super System (MESS)",
        "Multi Emulator Super System",
        None,
        false
    ),
    sys!(
        "model2",
        "Sega Model 2",
        None,
        &[],
        &["7z", "zip"],
        &["model2"],
        "model2",
        "model2",
        "MODEL2",
        "Sega Model 2 (MODEL2)",
        "Sega Model 2",
        Some(75),
        false
    ),
    sys!(
        "model3",
        "Sega Model 3",
        None,
        &[],
        &["7z", "zip"],
        &["model3"],
        "model3",
        "model3",
        "MODEL3",
        "Sega Model 3 (MODEL3)",
        "Sega Model 3",
        Some(75),
        false
    ),
    sys!(
        "moto",
        "Thomson MO/TO Series",
        Some("Thomson - MOTO"),
        &["fd", "k7", "m5", "m7", "sap"],
        &["7z", "rom", "zip"],
        &["moto", "thomson", "to8"],
        "moto",
        "thomson",
        "MOTO",
        "Thomson MO/TO Series (MOTO)",
        "Thomson MO/TO Series",
        Some(141),
        false
    ),
    sys!(
        "msx2",
        "MSX2",
        Some("Microsoft - MSX2"),
        &["mx2"],
        &[
            "7z", "cas", "col", "di1", "di2", "dmk", "dsk", "fd1", "fd2", "m3u", "mx1", "ogv",
            "ri", "rom", "sc", "sg", "wav", "xsa", "zip"
        ],
        &["msx2"],
        "msx2",
        "msx2",
        "MSX2",
        "MSX2 (MSX2)",
        "MSX2",
        Some(116),
        false
    ),
    sys!(
        "msxturbor",
        "MSX Turbo R",
        None,
        &[],
        &[
            "7z", "cas", "col", "di1", "di2", "dmk", "dsk", "fd1", "fd2", "m3u", "mx1", "mx2",
            "ogv", "ri", "rom", "sc", "sg", "wav", "xsa", "zip"
        ],
        &["msxturbor"],
        "msxturbor",
        "msxturbor",
        "MSXTURBOR",
        "MSX Turbo R (MSXTURBOR)",
        "MSX Turbo R",
        Some(118),
        false
    ),
    sys!(
        "mugen",
        "M.U.G.E.N Game Engine",
        None,
        &["mugen"],
        &["zip", "7z"],
        &["mugen"],
        "mugen",
        "mugen",
        "MUGEN",
        "M.U.G.E.N Game Engine (MUGEN)",
        "M.U.G.E.N Game Engine",
        None,
        false
    ),
    sys!(
        "multivision",
        "Othello Multivision",
        None,
        &[],
        &["7z", "bin", "gg", "rom", "sg", "sms", "zip"],
        &["multivision"],
        "multivision",
        "multivision",
        "MULTIVISION",
        "Othello Multivision (MULTIVISION)",
        "Othello Multivision",
        Some(109),
        false
    ),
    sys!(
        "n64dd",
        "Nintendo 64DD",
        Some("Nintendo - Nintendo 64DD"),
        &["ndd"],
        &["7z", "bin", "d64", "n64", "u1", "v64", "z64", "zip"],
        &["n64dd"],
        "n64dd",
        "n64dd",
        "N64DD",
        "Nintendo 64DD (N64DD)",
        "Nintendo 64DD",
        Some(14),
        false
    ),
    sys!(
        "naomi",
        "Sega NAOMI",
        Some("Sega - Naomi"),
        &[],
        &["7z", "bin", "dat", "elf", "lst", "zip"],
        &["naomi", "naomigd"],
        "naomi",
        "naomi",
        "NAOMI",
        "Sega NAOMI (NAOMI)",
        "Sega NAOMI",
        Some(75),
        false
    ),
    sys!(
        "naomi2",
        "Sega NAOMI 2",
        Some("Sega - Naomi 2"),
        &[],
        &["7z", "bin", "dat", "elf", "lst", "zip"],
        &["naomi2"],
        "naomi2",
        "naomi2",
        "NAOMI2",
        "Sega NAOMI 2 (NAOMI2)",
        "Sega NAOMI 2",
        Some(75),
        false
    ),
    sys!(
        "neogeo",
        "SNK Neo Geo",
        Some("SNK - Neo Geo"),
        &["neo"],
        &["7z", "zip"],
        &["neogeo"],
        "neogeo",
        "neogeo",
        "NEOGEO",
        "SNK Neo Geo (NEOGEO)",
        "SNK Neo Geo",
        Some(142),
        false
    ),
    sys!(
        "neogeocd",
        "SNK Neo Geo CD",
        Some("SNK - Neo Geo CD"),
        &[],
        &["chd", "cue", "zip", "7z"],
        &["neogeocd", "neogeocdjp"],
        "neogeocd",
        "neogeocd",
        "NEOGEOCD",
        "SNK Neo Geo CD (NEOGEOCD)",
        "SNK Neo Geo CD",
        Some(70),
        true
    ),
    sys!(
        "ngage",
        "Nokia N-Gage",
        None,
        &["ngage"],
        &["zip", "7z"],
        &["ngage"],
        "ngage",
        "ngage",
        "NGAGE",
        "Nokia N-Gage (NGAGE)",
        "Nokia N-Gage",
        Some(30),
        false
    ),
    sys!(
        "odyssey2",
        "Magnavox Odyssey 2",
        Some("Magnavox - Odyssey2"),
        &[],
        &["7z", "bin", "zip"],
        &["odyssey2"],
        "odyssey2",
        "odyssey2",
        "ODYSSEY2",
        "Magnavox Odyssey 2 (ODYSSEY2)",
        "Magnavox Odyssey 2",
        Some(104),
        false
    ),
    sys!(
        "openbor",
        "OpenBOR Game Engine",
        None,
        &[],
        &["appimage", "zip", "7z"],
        &["openbor"],
        "openbor",
        "openbor",
        "OPENBOR",
        "OpenBOR Game Engine (OPENBOR)",
        "OpenBOR Game Engine",
        Some(214),
        false
    ),
    sys!(
        "oric",
        "Tangerine Computer Systems Oric",
        None,
        &["ort"],
        &["dsk", "tap", "wav", "zip", "7z"],
        &["oric", "oricatmos"],
        "oric",
        "oricatmos",
        "ORIC",
        "Tangerine Computer Systems Oric (ORIC)",
        "Tangerine Computer Systems Oric",
        Some(131),
        false
    ),
    sys!(
        "palm",
        "Palm OS",
        None,
        &["pqa", "prc"],
        &["7z", "img", "zip"],
        &["palm"],
        "palm",
        "palm",
        "PALM",
        "Palm OS (PALM)",
        "Palm OS",
        Some(219),
        false
    ),
    sys!(
        "pc88",
        "NEC PC-8800 Series",
        Some("NEC - PC-8001 - PC-8801"),
        &["cmt", "t88", "u88"],
        &["88d", "d88", "m3u", "zip", "7z"],
        &["pc88"],
        "pc88",
        "pc88",
        "PC88",
        "NEC PC-8800 Series (PC88)",
        "NEC PC-8800 Series",
        Some(221),
        false
    ),
    sys!(
        "pc98",
        "NEC PC-9800 Series",
        Some("NEC - PC-98"),
        &["98d", "d98", "fdd", "hdd", "hdi", "hdn", "nhd", "thd"],
        &["2hd", "7z", "88d", "cmd", "d88", "dup", "fdi", "hdm", "m3u", "tfd", "xdf", "zip"],
        &["pc98"],
        "pc98",
        "pc98",
        "PC98",
        "NEC PC-9800 Series (PC98)",
        "NEC PC-9800 Series",
        Some(208),
        false
    ),
    sys!(
        "pcfx",
        "NEC PC-FX",
        Some("NEC - PC-FX"),
        &[],
        &["7z", "ccd", "chd", "cue", "m3u", "toc", "zip"],
        &["pcfx"],
        "pcfx",
        "pcfx",
        "PCFX",
        "NEC PC-FX (PCFX)",
        "NEC PC-FX",
        Some(72),
        true
    ),
    sys!(
        "plus4",
        "Commodore Plus/4",
        Some("Commodore - Plus-4"),
        &[],
        &[
            "7z", "bin", "cmd", "crt", "d2m", "d4m", "d64", "d6z", "d71", "d7z", "d80", "d81",
            "d82", "d8z", "g41", "g4z", "g64", "g6z", "gz", "lnx", "m3u", "nbz", "nib", "p00",
            "prg", "t64", "tap", "vfl", "vsf", "x64", "x6z", "zip"
        ],
        &["cplus4", "plus4"],
        "plus4",
        "cplus4",
        "PLUS4",
        "Commodore Plus/4 (PLUS4)",
        "Commodore Plus/4",
        Some(99),
        false
    ),
    sys!(
        "ps3",
        "Sony PlayStation 3",
        Some("Sony - PlayStation 3"),
        &["ps3", "ps3dir"],
        &["desktop", "iso", "zip", "7z"],
        &["ps3"],
        "ps3",
        "ps3",
        "PS3",
        "Sony PlayStation 3 (PS3)",
        "Sony PlayStation 3",
        Some(59),
        true
    ),
    sys!(
        "psvita",
        "Sony PlayStation Vita",
        Some("Sony - PlayStation Vita"),
        &["psvita"],
        &["zip", "7z"],
        &["psvita"],
        "psvita",
        "psvita",
        "PSVITA",
        "Sony PlayStation Vita (PSVITA)",
        "Sony PlayStation Vita",
        Some(62),
        false
    ),
    sys!(
        "pv1000",
        "Casio PV-1000",
        Some("Casio - PV-1000"),
        &[],
        &["7z", "bin", "zip"],
        &["pv1000"],
        "pv1000",
        "pv1000",
        "PV1000",
        "Casio PV-1000 (PV1000)",
        "Casio PV-1000",
        Some(74),
        false
    ),
    sys!(
        "quake",
        "Quake",
        Some("Quake"),
        &["pak"],
        &["desktop", "pk3", "sh", "zip", "7z"],
        &["quake"],
        "quake",
        "quake",
        "QUAKE",
        "Quake (QUAKE)",
        "Quake",
        None,
        false
    ),
    sys!(
        "samcoupe",
        "MGT SAM Coupé",
        None,
        &["sad", "sbt"],
        &["7z", "dsk", "mgt", "zip"],
        &["samcoupe"],
        "samcoupe",
        "samcoupe",
        "SAMCOUPE",
        "MGT SAM Coupé (SAMCOUPE)",
        "MGT SAM Coupé",
        Some(213),
        false
    ),
    sys!(
        "satellaview",
        "Nintendo Satellaview",
        Some("Nintendo - Satellaview"),
        &[],
        &["7z", "bml", "bs", "fig", "sfc", "smc", "st", "swc", "zip"],
        &["satellaview"],
        "satellaview",
        "satellaview",
        "SATELLAVIEW",
        "Nintendo Satellaview (SATELLAVIEW)",
        "Nintendo Satellaview",
        Some(107),
        false
    ),
    sys!(
        "scv",
        "Epoch Super Cassette Vision",
        Some("Epoch - Super Cassette Vision"),
        &[],
        &["0", "7z", "bin", "zip"],
        &["scv"],
        "scv",
        "scv",
        "SCV",
        "Epoch Super Cassette Vision (SCV)",
        "Epoch Super Cassette Vision",
        Some(67),
        false
    ),
    sys!(
        "solarus",
        "Solarus Game Engine",
        None,
        &["solarus"],
        &["zip", "7z"],
        &["solarus"],
        "solarus",
        "solarus",
        "SOLARUS",
        "Solarus Game Engine (SOLARUS)",
        "Solarus Game Engine",
        Some(223),
        false
    ),
    sys!(
        "spectravideo",
        "Spectravideo",
        Some("Spectravideo - SVI-318 - SVI-328"),
        &[],
        &["7z", "cas", "col", "dsk", "m3u", "mx1", "mx2", "ri", "rom", "sc", "sg", "zip"],
        &["spectravideo"],
        "spectravideo",
        "spectravideo",
        "SPECTRAVIDEO",
        "Spectravideo (SPECTRAVIDEO)",
        "Spectravideo",
        Some(218),
        false
    ),
    sys!(
        "stv",
        "Sega Titan Video Game System",
        None,
        &[],
        &["7z", "zip"],
        &["stv"],
        "stv",
        "stv",
        "STV",
        "Sega Titan Video Game System (STV)",
        "Sega Titan Video Game System",
        Some(75),
        false
    ),
    sys!(
        "sufami",
        "Bandai SuFami Turbo",
        Some("Nintendo - Sufami Turbo"),
        &[],
        &["7z", "bml", "bs", "fig", "sfc", "smc", "st", "zip"],
        &["sufami"],
        "sufami",
        "sufami",
        "SUFAMI",
        "Bandai SuFami Turbo (SUFAMI)",
        "Bandai SuFami Turbo",
        Some(108),
        false
    ),
    sys!(
        "supergrafx",
        "NEC SuperGrafx",
        Some("NEC - PC Engine SuperGrafx"),
        &["sgx"],
        &["7z", "ccd", "chd", "cue", "pce", "rom", "zip"],
        &["supergrafx"],
        "supergrafx",
        "supergrafx",
        "SUPERGRAFX",
        "NEC SuperGrafx (SUPERGRAFX)",
        "NEC SuperGrafx",
        Some(105),
        false
    ),
    sys!(
        "supervision",
        "Watara Supervision",
        Some("Watara - Supervision"),
        &["sv"],
        &["7z", "bin", "zip"],
        &["supervision"],
        "supervision",
        "supervision",
        "SUPERVISION",
        "Watara Supervision (SUPERVISION)",
        "Watara Supervision",
        Some(207),
        false
    ),
    sys!(
        "supracan",
        "Funtech Super A'Can",
        Some("Funtech - Super Acan"),
        &[],
        &["7z", "bin", "zip"],
        &["supracan"],
        "supracan",
        "supracan",
        "SUPRACAN",
        "Funtech Super A'Can (SUPRACAN)",
        "Funtech Super A'Can",
        Some(100),
        false
    ),
    sys!(
        "switch",
        "Nintendo Switch",
        None,
        &["nca", "nro", "nso", "nsp", "xci"],
        &["zip", "7z"],
        &["switch"],
        "switch",
        "switch",
        "SWITCH",
        "Nintendo Switch (SWITCH)",
        "Nintendo Switch",
        Some(225),
        false
    ),
    sys!(
        "symbian",
        "Symbian",
        None,
        &["sis", "sisx", "symbian"],
        &["zip", "7z"],
        &["symbian"],
        "symbian",
        "symbian",
        "SYMBIAN",
        "Symbian (SYMBIAN)",
        "Symbian",
        Some(30),
        false
    ),
    sys!(
        "ti99",
        "Texas Instruments TI-99",
        None,
        &["rpk"],
        &["7z", "zip"],
        &["ti99"],
        "ti99",
        "ti99",
        "TI99",
        "Texas Instruments TI-99 (TI99)",
        "Texas Instruments TI-99",
        Some(205),
        false
    ),
    sys!(
        "tic80",
        "TIC-80",
        Some("TIC-80"),
        &["tic"],
        &["png", "zip", "7z"],
        &["tic80"],
        "tic80",
        "tic80",
        "TIC80",
        "TIC-80 (TIC80)",
        "TIC-80",
        Some(222),
        false
    ),
    sys!(
        "triforce",
        "Namco-Sega-Nintendo Triforce",
        None,
        &[],
        &[
            "7z", "ciso", "dff", "dol", "elf", "gcm", "gcz", "iso", "json", "m3u", "rvz", "tgc",
            "wad", "wbfs", "wia", "zip"
        ],
        &["triforce"],
        "triforce",
        "triforce",
        "TRIFORCE",
        "Namco-Sega-Nintendo Triforce (TRIFORCE)",
        "Namco-Sega-Nintendo Triforce",
        Some(75),
        true
    ),
    sys!(
        "trs80",
        "Tandy TRS-80",
        None,
        &[],
        &["cmd", "dsk", "zip", "7z"],
        &["trs-80", "trs80"],
        "trs-80",
        "trs80",
        "TRS80",
        "Tandy TRS-80 (TRS80)",
        "Tandy TRS-80",
        Some(144),
        false
    ),
    sys!(
        "uzebox",
        "Uzebox Open Source Console",
        Some("Uzebox"),
        &["uze"],
        &["7z", "zip"],
        &["uzebox"],
        "uzebox",
        "uzebox",
        "UZEBOX",
        "Uzebox Open Source Console (UZEBOX)",
        "Uzebox Open Source Console",
        Some(216),
        false
    ),
    sys!(
        "vic20",
        "Commodore VIC-20",
        Some("Commodore - VIC-20"),
        &["a0", "b0"],
        &[
            "7z", "bin", "cmd", "crt", "d2m", "d4m", "d64", "d6z", "d71", "d7z", "d80", "d81",
            "d82", "d8z", "g41", "g4z", "g64", "g6z", "gz", "lnx", "m3u", "nbz", "nib", "p00",
            "prg", "rom", "t64", "tap", "vfl", "vsf", "x64", "x6z", "zip"
        ],
        &["c20", "vic20"],
        "vic20",
        "c20",
        "VIC20",
        "Commodore VIC-20 (VIC20)",
        "Commodore VIC-20",
        Some(73),
        false
    ),
    sys!(
        "videopac",
        "Philips Videopac G7000",
        Some("Philips - Videopac+"),
        &[],
        &["7z", "bin", "zip"],
        &["videopac", "videopacplus"],
        "videopac",
        "videopacplus",
        "VIDEOPAC",
        "Philips Videopac G7000 (VIDEOPAC)",
        "Philips Videopac G7000",
        Some(104),
        false
    ),
    sys!(
        "vircon32",
        "Vircon32 Virtual Console",
        Some("Vircon32"),
        &["v32"],
        &["7z", "zip"],
        &["vircon32"],
        "vircon32",
        "vircon32",
        "VIRCON32",
        "Vircon32 Virtual Console (VIRCON32)",
        "Vircon32 Virtual Console",
        Some(272),
        false
    ),
    sys!(
        "vpinball",
        "Visual Pinball",
        None,
        &["vpt", "vpx"],
        &["zip", "7z"],
        &["vpinball"],
        "vpinball",
        "vpinball",
        "VPINBALL",
        "Visual Pinball (VPINBALL)",
        "Visual Pinball",
        Some(198),
        false
    ),
    sys!(
        "vsmile",
        "VTech V.Smile",
        Some("VTech - V.Smile"),
        &[],
        &["7z", "bin", "zip"],
        &["vsmile"],
        "vsmile",
        "vsmile",
        "VSMILE",
        "VTech V.Smile (VSMILE)",
        "VTech V.Smile",
        Some(120),
        false
    ),
    sys!(
        "wasm4",
        "WASM-4 Fantasy Console",
        Some("WASM-4"),
        &["wasm"],
        &["7z", "zip"],
        &["wasm4"],
        "wasm4",
        "wasm4",
        "WASM4",
        "WASM-4 Fantasy Console (WASM4)",
        "WASM-4 Fantasy Console",
        Some(262),
        false
    ),
    sys!(
        "wii",
        "Nintendo Wii",
        Some("Nintendo - Wii"),
        &["wbfs"],
        &[
            "7z", "ciso", "dff", "dol", "elf", "gcm", "gcz", "iso", "json", "m3u", "rvz", "tgc",
            "wad", "wia", "zip"
        ],
        &["wii"],
        "wii",
        "wii",
        "WII",
        "Nintendo Wii (WII)",
        "Nintendo Wii",
        Some(16),
        true
    ),
    sys!(
        "wiiu",
        "Nintendo Wii U",
        Some("Nintendo - Wii U"),
        &["rpx", "tmd", "wua", "wud", "wuhb", "wux"],
        &["elf", "zip", "7z"],
        &["wiiu"],
        "wiiu",
        "wiiu",
        "WIIU",
        "Nintendo Wii U (WIIU)",
        "Nintendo Wii U",
        Some(18),
        true
    ),
    sys!(
        "windows3x",
        "Windows 3.x",
        None,
        &[],
        &["7z", "appimage", "bat", "desktop", "dosz", "sh", "zip"],
        &["windows3x"],
        "windows3x",
        "windows3x",
        "WINDOWS3X",
        "Windows 3.x (WINDOWS3X)",
        "Windows 3.x",
        Some(136),
        false
    ),
    sys!(
        "windows9x",
        "Windows 9x",
        None,
        &[],
        &["7z", "appimage", "bat", "desktop", "dosz", "sh", "zip"],
        &["windows9x"],
        "windows9x",
        "windows9x",
        "WINDOWS9X",
        "Windows 9x (WINDOWS9X)",
        "Windows 9x",
        Some(138),
        false
    ),
    sys!(
        "x1",
        "Sharp X1",
        Some("Sharp - X1"),
        &["2d", "dx1"],
        &["2hd", "7z", "88d", "cmd", "d88", "dup", "hdm", "tap", "tfd", "xdf", "zip"],
        &["x1"],
        "x1",
        "x1",
        "X1",
        "Sharp X1 (X1)",
        "Sharp X1",
        Some(220),
        false
    ),
    sys!(
        "x68000",
        "Sharp X68000",
        Some("Sharp - X68000"),
        &[],
        &[
            "2hd", "7z", "88d", "cmd", "d88", "dim", "dup", "hdf", "hdm", "img", "m3u", "xdf",
            "zip"
        ],
        &["x68000"],
        "x68000",
        "x68000",
        "X68000",
        "Sharp X68000 (X68000)",
        "Sharp X68000",
        Some(79),
        false
    ),
    sys!(
        "xbox360",
        "Microsoft Xbox 360",
        Some("Microsoft - Xbox 360"),
        &["zar"],
        &["desktop", "iso", "sh", "xex", "zip", "7z"],
        &["xbox360"],
        "xbox360",
        "xbox360",
        "XBOX360",
        "Microsoft Xbox 360 (XBOX360)",
        "Microsoft Xbox 360",
        Some(33),
        true
    ),
    sys!(
        "zmachine",
        "Infocom Z-machine",
        None,
        &["z1", "z2", "z3", "z4", "z5", "z6", "z7", "z8", "zblorb", "zlb"],
        &["dat", "zip", "7z"],
        &["zmachine"],
        "zmachine",
        "zmachine",
        "ZMACHINE",
        "Infocom Z-machine (ZMACHINE)",
        "Infocom Z-machine",
        Some(215),
        false
    ),
    sys!(
        "zx81",
        "Sinclair ZX81",
        Some("Sinclair - ZX 81"),
        &["p"],
        &["7z", "tzx", "zip"],
        &["zx81"],
        "zx81",
        "zx81",
        "ZX81",
        "Sinclair ZX81 (ZX81)",
        "Sinclair ZX81",
        Some(77),
        false
    ),
    sys!(
        "zxnext",
        "Sinclair ZX Spectrum Next",
        None,
        &["nex"],
        &["sna", "zip", "7z"],
        &["zxnext"],
        "zxnext",
        "zxnext",
        "ZXNEXT",
        "Sinclair ZX Spectrum Next (ZXNEXT)",
        "Sinclair ZX Spectrum Next",
        None,
        false
    ),
    sys!(
        "zxspectrum",
        "Sinclair ZX Spectrum",
        Some("Sinclair - ZX Spectrum"),
        &["rzx", "scl", "szx", "trd", "udi", "z80", "tzx"],
        &["7z", "dsk", "gz", "img", "mgt", "sh", "sna", "tap", "zip"],
        &["zxspectrum"],
        "zxspectrum",
        "zxspectrum",
        "ZXSPECTRUM",
        "Sinclair ZX Spectrum (ZXSPECTRUM)",
        "Sinclair ZX Spectrum",
        Some(76),
        false
    ),
];

pub fn get(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id == id)
}

/// Which region's names to show for systems known by different names
/// (Super Nintendo / Super Famicom, Genesis / Mega Drive, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum NameRegion {
    #[default]
    Us,
    Europe,
    Japan,
}

static NAME_REGION: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_name_region(r: NameRegion) {
    NAME_REGION.store(r as u8, std::sync::atomic::Ordering::Relaxed);
}

pub fn name_region() -> NameRegion {
    match NAME_REGION.load(std::sync::atomic::Ordering::Relaxed) {
        1 => NameRegion::Europe,
        2 => NameRegion::Japan,
        _ => NameRegion::Us,
    }
}

/// (system, US name, European name, Japanese name)
const REGIONAL_NAMES: &[(&str, &str, &str, &str)] = &[
    (
        "nes",
        "Nintendo Entertainment System",
        "Nintendo Entertainment System",
        "Famicom",
    ),
    ("snes", "Super Nintendo", "Super Nintendo", "Super Famicom"),
    (
        "genesis",
        "Sega Genesis",
        "Sega Mega Drive",
        "Sega Mega Drive",
    ),
    (
        "sega32x",
        "Sega 32X",
        "Sega Mega Drive 32X",
        "Sega Super 32X",
    ),
    ("segacd", "Sega CD", "Sega Mega-CD", "Sega Mega-CD"),
    (
        "mastersystem",
        "Sega Master System",
        "Sega Master System",
        "Sega Mark III",
    ),
    ("pcengine", "TurboGrafx-16", "PC Engine", "PC Engine"),
    (
        "pcenginecd",
        "TurboGrafx-CD",
        "PC Engine CD",
        "PC Engine CD-ROM²",
    ),
    (
        "supergrafx",
        "SuperGrafx",
        "PC Engine SuperGrafx",
        "PC Engine SuperGrafx",
    ),
    (
        "odyssey2",
        "Magnavox Odyssey²",
        "Philips Videopac G7000",
        "Odyssey²",
    ),
    ("n64dd", "Nintendo 64DD", "Nintendo 64DD", "64DD"),
];

impl System {
    /// The system's name in the chosen region.
    pub fn display_name(&self) -> &'static str {
        match REGIONAL_NAMES.iter().find(|r| r.0 == self.id) {
            Some(&(_, us, eu, jp)) => match name_region() {
                NameRegion::Us => us,
                NameRegion::Europe => eu,
                NameRegion::Japan => jp,
            },
            None => self.name,
        }
    }
}

pub fn name_of(id: &str) -> &str {
    get(id).map(|s| s.display_name()).unwrap_or(id)
}

fn norm(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Other folder names for existing systems (ES-DE regional folders, etc.).
const EXTRA_ALIASES: &[(&str, &str)] = &[
    ("megadrive", "genesis"),
    ("megadrivejp", "genesis"),
    ("mark3", "mastersystem"),
    ("megacd", "segacd"),
    ("megacdjp", "segacd"),
    ("saturnjp", "saturn"),
    ("sega32xjp", "sega32x"),
    ("sega32xna", "sega32x"),
    ("sfc", "snes"),
    ("snesna", "snes"),
    ("famicom", "nes"),
    ("tg16", "pcengine"),
    ("tgcd", "pcenginecd"),
    ("fba", "arcade"),
    ("fbneo", "arcade"),
    ("cps", "arcade"),
    ("mameadvmame", "mame"),
    ("advmame", "mame"),
    ("msx1", "msx"),
    ("pc", "dos"),
    ("sgb", "gb"),
    ("supergameboy", "gb"),
];

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
        for (alias, id) in EXTRA_ALIASES {
            if let Some(s) = get(id) {
                out.insert(norm(alias), s);
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
    matches!(
        id,
        "arcade"
            | "mame"
            | "cps1"
            | "cps2"
            | "cps3"
            | "neogeo"
            | "naomi"
            | "naomi2"
            | "atomiswave"
            | "model2"
            | "model3"
            | "stv"
    )
}

/// Systems whose games can be whole folders rather than single files.
pub fn folder_games(id: &str) -> bool {
    matches!(
        id,
        "dos" | "scummvm" | "laserdisc" | "ps3" | "mugen" | "easyrpg" | "windows3x" | "windows9x"
    )
}

/// Folder-name suffixes that mark a folder as one game of a system.
pub const FOLDER_SUFFIXES: &[(&str, &str)] = &[
    ("scummvm", "scummvm"),
    ("svm", "scummvm"),
    ("pc", "dos"),
    ("dos", "dos"),
    ("ps3", "ps3"),
    ("daphne", "laserdisc"),
    ("singe", "laserdisc"),
    ("dirksimple", "laserdisc"),
    ("mugen", "mugen"),
    ("easyrpg", "easyrpg"),
];

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
