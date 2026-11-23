//! Every ES-DE ROM folder is recognised, and new systems detect correctly.

use std::fs;
use std::path::Path;
use tabularium_core::detect::scan;
use tabularium_core::systems::{self, NameRegion};

/// ES-DE's default ROM folders (launcher categories like steam/kodi excluded).
const ESDE_FOLDERS: &[&str] = &[
    "3do",
    "adam",
    "amiga",
    "amiga1200",
    "amiga600",
    "amigacd32",
    "amstradcpc",
    "apple2",
    "apple2gs",
    "arcade",
    "arcadia",
    "archimedes",
    "arduboy",
    "astrocde",
    "atari2600",
    "atari5200",
    "atari7800",
    "atari800",
    "atarijaguar",
    "atarijaguarcd",
    "atarilynx",
    "atarist",
    "atarixe",
    "atomiswave",
    "bbcmicro",
    "c64",
    "cdimono1",
    "cdtv",
    "chailove",
    "channelf",
    "coco",
    "colecovision",
    "cps",
    "cps1",
    "cps2",
    "cps3",
    "crvision",
    "daphne",
    "doom",
    "dos",
    "dragon32",
    "dreamcast",
    "easyrpg",
    "electron",
    "famicom",
    "fba",
    "fbneo",
    "fds",
    "flash",
    "fm7",
    "fmtowns",
    "fpinball",
    "gamate",
    "gameandwatch",
    "gamecom",
    "gamegear",
    "gb",
    "gba",
    "gbc",
    "gc",
    "genesis",
    "gmaster",
    "gx4000",
    "intellivision",
    "j2me",
    "laserdisc",
    "lcdgames",
    "lowresnx",
    "lutro",
    "macintosh",
    "mame",
    "mame-advmame",
    "mark3",
    "mastersystem",
    "megacd",
    "megacdjp",
    "megadrive",
    "megadrivejp",
    "megaduck",
    "mess",
    "model2",
    "model3",
    "moto",
    "msx",
    "msx1",
    "msx2",
    "msxturbor",
    "mugen",
    "multivision",
    "n3ds",
    "n64",
    "n64dd",
    "naomi",
    "naomi2",
    "naomigd",
    "nds",
    "neogeo",
    "neogeocd",
    "neogeocdjp",
    "nes",
    "ngage",
    "ngp",
    "ngpc",
    "odyssey2",
    "openbor",
    "oric",
    "palm",
    "pc",
    "pc88",
    "pc98",
    "pcengine",
    "pcenginecd",
    "pcfx",
    "pico8",
    "plus4",
    "pokemini",
    "ps2",
    "ps3",
    "psp",
    "psvita",
    "psx",
    "pv1000",
    "quake",
    "samcoupe",
    "satellaview",
    "saturn",
    "saturnjp",
    "scummvm",
    "scv",
    "sega32x",
    "sega32xjp",
    "sega32xna",
    "segacd",
    "sfc",
    "sg-1000",
    "sgb",
    "snes",
    "snesna",
    "solarus",
    "spectravideo",
    "stv",
    "sufami",
    "supergrafx",
    "supervision",
    "supracan",
    "switch",
    "symbian",
    "tanodragon",
    "tg16",
    "tg-cd",
    "ti99",
    "tic80",
    "to8",
    "triforce",
    "trs-80",
    "uzebox",
    "vectrex",
    "vic20",
    "videopac",
    "vircon32",
    "virtualboy",
    "vpinball",
    "vsmile",
    "wasm4",
    "wii",
    "wiiu",
    "windows3x",
    "windows9x",
    "wonderswan",
    "wonderswancolor",
    "x1",
    "x68000",
    "xbox",
    "xbox360",
    "zmachine",
    "zx81",
    "zxnext",
    "zxspectrum",
];

#[test]
fn every_esde_folder_maps_to_a_system() {
    let missing: Vec<&str> = ESDE_FOLDERS
        .iter()
        .copied()
        .filter(|f| systems::from_folder_name(f).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "unrecognised ES-DE folders: {missing:?}"
    );
    // Regional folders land on the same system.
    for (folder, id) in [
        ("megadrivejp", "genesis"),
        ("snesna", "snes"),
        ("sfc", "snes"),
        ("famicom", "nes"),
        ("tg16", "pcengine"),
        ("mark3", "mastersystem"),
        ("saturnjp", "saturn"),
        ("pc", "dos"),
        ("amiga1200", "amiga"),
        ("c20", "vic20"),
        ("Commodore 64", "c64"),
        ("Amstrad CPC", "amstradcpc"),
    ] {
        assert_eq!(
            systems::from_folder_name(folder).map(|s| s.id),
            Some(id),
            "{folder}"
        );
    }
}

fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

#[test]
fn new_systems_detect() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let files = [
        ("misc/Turrican (Europe).d64", "c64"),
        ("misc/Lemmings.adf", "amiga"),
        ("misc/Zelda BOTW.nsp", "switch"),
        ("misc/Manic Miner.tzx", "zxspectrum"),
        ("misc/Star Raiders.atr", "atari800"),
        ("misc/Metal Gear 2.mx2", "msx2"),
        ("misc/Metal Slug.neo", "neogeo"),
        ("misc/Zork I.z5", "zmachine"),
        ("misc/Tetris.uze", "uzebox"),
        ("misc/Burger Time.int", "intellivision"),
        ("misc/Daggerfall Demo.wbfs", "wii"),
        ("VIC-20/Gorf.crt", "vic20"),
        ("Commodore Plus-4/Ghost Town.d64", "plus4"),
        ("Amstrad CPC/Gryzor.dsk", "amstradcpc"),
        ("megadrivejp/Bare Knuckle (Japan).md", "genesis"),
        ("snesna/Super Metroid (USA).sfc", "snes"),
        ("Atari Jaguar/Tempest 2000.j64", "atarijaguar"),
    ];
    for (f, _) in files {
        write(&src.join(f), f.as_bytes());
    }
    write(&src.join("misc/mystery.dsk"), b"which computer?");
    write(&src.join("laserdisc/Dragon's Lair.daphne/lair.txt"), b"x");
    write(&src.join("laserdisc/Dragon's Lair.daphne/lair.m2v"), b"x");
    write(&src.join("PS3/Demon's Souls.ps3/PS3_GAME/PARAM.SFO"), b"x");
    let found = scan(&[src], None, &|| false);
    let sys = |name: &str| {
        found
            .sets
            .iter()
            .find(|r| r.main.file_name().unwrap().to_string_lossy() == name)
            .and_then(|r| r.system)
            .map(|s| s.id)
    };
    for (f, want) in files {
        let name = Path::new(f)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(sys(&name), Some(want), "{f}");
    }
    assert_eq!(
        sys("mystery.dsk"),
        None,
        "a lone .dsk could be many computers"
    );
    assert_eq!(sys("Dragon's Lair.daphne"), Some("laserdisc"));
    assert_eq!(sys("Demon's Souls.ps3"), Some("ps3"));
}

#[test]
fn regional_names() {
    let snes = systems::get("snes").unwrap();
    systems::set_name_region(NameRegion::Japan);
    assert_eq!(snes.display_name(), "Super Famicom");
    assert_eq!(systems::name_of("genesis"), "Sega Mega Drive");
    systems::set_name_region(NameRegion::Us);
    assert_eq!(snes.display_name(), "Super Nintendo");
    assert_eq!(systems::name_of("pcengine"), "TurboGrafx-16");
    assert!(systems::SYSTEMS.len() > 140);
}
