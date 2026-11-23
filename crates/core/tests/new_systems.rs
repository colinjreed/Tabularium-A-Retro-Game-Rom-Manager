//! PS2, GameCube, 3DS, Xbox, DOS, ScummVM and arcade (MAME / CPS / FBNeo).

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tabularium_core::collection::{self, Collection, ExportOptions};
use tabularium_core::folder_games::scummvm_id;
use tabularium_core::http::{Fetcher, HttpError};
use tabularium_core::import::{import, ImportOptions};
use tabularium_core::library::Library;
use tabularium_core::progress::Silent;
use tabularium_core::{bios, profiles, systems};

fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// A tiny cooked ISO9660 image whose root directory holds SYSTEM.CNF.
fn ps_iso(system_cnf: &str, filler: u8) -> Vec<u8> {
    let mut img = vec![filler; 2048 * 24];
    let pvd = 16 * 2048;
    img[pvd] = 1;
    img[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
    img[pvd + 8..pvd + 8 + 11].copy_from_slice(b"PLAYSTATION");
    // root directory record at PVD+156: extent 20, size 2048
    let root = pvd + 156;
    img[root] = 34;
    img[root + 2..root + 6].copy_from_slice(&le32(20));
    img[root + 10..root + 14].copy_from_slice(&le32(2048));
    // directory at sector 20: one record for SYSTEM.CNF;1 -> sector 21
    let dir = 20 * 2048;
    img[dir..dir + 2048].fill(0);
    let name = b"SYSTEM.CNF;1";
    let len = 33 + name.len() + 1;
    img[dir] = len as u8;
    img[dir + 2..dir + 6].copy_from_slice(&le32(21));
    img[dir + 10..dir + 14].copy_from_slice(&le32(system_cnf.len() as u32));
    img[dir + 32] = name.len() as u8;
    img[dir + 33..dir + 33 + name.len()].copy_from_slice(name);
    let f = 21 * 2048;
    img[f..f + system_cnf.len()].copy_from_slice(system_cnf.as_bytes());
    img
}

fn zip_with(p: &Path, files: &[(&str, &[u8])]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut z = zip::ZipWriter::new(fs::File::create(p).unwrap());
    for (n, d) in files {
        z.start_file(*n, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(d).unwrap();
    }
    z.finish().unwrap();
}

const ARCADE_DAT: &str = r#"
game (
	name "Street Fighter II - The World Warrior (World 910522)"
	rom ( name sf2.zip size 1 crc 0 )
)
game (
	name "Super Street Fighter II Turbo (World 940223)"
	rom ( name ssf2t.zip size 1 crc 0 )
)
game (
	name "Street Fighter II - The World Warrior (Japan 911210)"
	rom ( name sf2j.zip size 1 crc 0 )
)
"#;

const SCUMMVM_DAT: &str = r#"
game (
	name "Flight of the Amazon Queen"
	code "queen"
	rom ( name "queen.scummvm" size 6 crc 0 )
)
game (
	name "Beneath a Steel Sky"
	code "sky"
	rom ( name "sky.scummvm" size 4 crc 0 )
)
"#;

fn fake_net() -> Fetcher {
    Arc::new(|url: &str| -> Result<Option<Vec<u8>>, HttpError> {
        if url.contains("Arcade%20Games.dat") {
            Ok(Some(ARCADE_DAT.as_bytes().to_vec()))
        } else if url.contains("ScummVM.dat") {
            Ok(Some(SCUMMVM_DAT.as_bytes().to_vec()))
        } else {
            Ok(None)
        }
    })
}

fn build(src: &Path) {
    // PS2 disc in a folder called "ps" (a PS1 folder name): the disc header wins.
    write(
        &src.join("ps/God of War (USA).iso"),
        &ps_iso("BOOT2 = cdrom0:\\SCUS_973.99;1\r\n", 1),
    );
    write(
        &src.join("isos/Crash Bandicoot (USA).iso"),
        &ps_iso("BOOT = cdrom:\\SCUS_949.00;1\r\n", 2),
    );
    // GameCube: disc magic, and an RVZ whose disc type is GameCube
    let mut gc = vec![0u8; 0x400];
    gc[0x1C..0x20].copy_from_slice(&[0xC2, 0x33, 0x9F, 0x3D]);
    write(&src.join("isos/Metroid Prime (USA).iso"), &gc);
    let mut rvz = vec![0u8; 0x100];
    rvz[0..4].copy_from_slice(b"RVZ\x01");
    rvz[0x48..0x4C].copy_from_slice(&1u32.to_be_bytes());
    write(&src.join("isos/Pikmin (USA).rvz"), &rvz);
    let mut wii = rvz.clone();
    wii[0x48..0x4C].copy_from_slice(&2u32.to_be_bytes());
    write(&src.join("isos/Wii Sports (USA).rvz"), &wii);
    // Xbox XISO
    let mut x = vec![0u8; 0x10100];
    x[0x10000..0x10014].copy_from_slice(b"MICROSOFT*XBOX*MEDIA");
    write(&src.join("isos/Halo - Combat Evolved (USA).iso"), &x);
    write(&src.join("misc/Super Mario 3D Land (USA).3ds"), b"3ds rom");
    // DOS: a folder game and a zipped game, by folder name
    write(&src.join("DOS/Doom/DOOM.EXE"), b"doom");
    write(&src.join("DOS/Doom/DOOM.WAD"), b"wad");
    zip_with(
        &src.join("DOS/Commander Keen.zip"),
        &[("KEEN1.EXE", b"keen"), ("KEEN1.DAT", b"x")],
    );
    // ScummVM: identified by data file, by title, and by an existing pointer file
    write(&src.join("ScummVM/Monkey Island 2/MONKEY2.000"), b"mi2");
    write(&src.join("ScummVM/Monkey Island 2/MONKEY2.001"), b"mi2b");
    write(&src.join("ScummVM/Beneath a Steel Sky/sky.dsk"), b"sky");
    write(&src.join("adventures/Loom.scummvm/LOOM.LFL"), b"loom");
    write(&src.join("adventures/Loom.scummvm/Loom.scummvm"), b"loom\n");
    write(
        &src.join("ScummVM/Flight of the Amazon Queen/data.bin"),
        b"q",
    );
    // Arcade: unknown zips named by the arcade list; CPS2 by folder
    zip_with(
        &src.join("downloads/sf2.zip"),
        &[("sf2_30a.bin", b"a"), ("sf2_37a.bin", b"b")],
    );
    zip_with(
        &src.join("downloads/sf2j.zip"),
        &[("sf2j30.bin", b"c"), ("sf2j37.bin", b"d")],
    );
    zip_with(
        &src.join("cps2/ssf2t.zip"),
        &[("sfxe.03c", b"e"), ("sfxe.04a", b"f")],
    );
    zip_with(
        &src.join("downloads/notarcade.zip"),
        &[("a.txt", b"1"), ("b.txt", b"2")],
    );
}

#[test]
fn new_systems_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    build(&src);
    let lib = Library::create(&tmp.path().join("lib")).unwrap();
    let opts = ImportOptions {
        fetch: Some(fake_net()),
        ..Default::default()
    };
    let rep = import(&lib, &[src], &opts, &Silent).unwrap();

    let roms = lib.active().unwrap();
    let sys = |stem: &str| {
        roms.iter()
            .find(|r| r.stem == stem)
            .map(|r| r.system.as_str())
    };
    assert_eq!(sys("God of War (USA)"), Some("ps2"));
    assert_eq!(sys("Crash Bandicoot (USA)"), Some("psx"));
    assert_eq!(sys("Metroid Prime (USA)"), Some("gamecube"));
    assert_eq!(sys("Pikmin (USA)"), Some("gamecube"));
    assert_eq!(sys("Wii Sports (USA)"), None); // Wii isn't supported
    assert_eq!(sys("Halo - Combat Evolved (USA)"), Some("xbox"));
    assert_eq!(sys("Super Mario 3D Land (USA)"), Some("n3ds"));
    assert_eq!(sys("Doom"), Some("dos"));
    assert_eq!(sys("Commander Keen"), Some("dos"));
    assert_eq!(sys("Monkey Island 2"), Some("scummvm"));
    assert_eq!(sys("Loom"), Some("scummvm"));
    assert_eq!(sys("sf2"), Some("arcade"));
    assert_eq!(sys("ssf2t"), Some("cps2"));
    assert_eq!(sys("notarcade"), None);
    assert!(rep
        .unknown
        .iter()
        .any(|u| u.main.ends_with("notarcade.zip")));

    // Folder games keep all their files.
    let doom = roms.iter().find(|r| r.stem == "Doom").unwrap();
    assert!(lib.abspath(&doom.path).join("DOOM.WAD").exists());
    assert_eq!(doom.total_size, 7);

    // ScummVM IDs: data-file signature, existing pointer, title match, title match.
    let id = |stem: &str| {
        let r = roms.iter().find(|r| r.stem == stem).unwrap();
        scummvm_id(&lib.abspath(&r.path))
    };
    assert_eq!(id("Monkey Island 2").as_deref(), Some("monkey2"));
    assert_eq!(id("Loom").as_deref(), Some("loom"));
    assert_eq!(id("Beneath a Steel Sky").as_deref(), Some("sky"));
    assert_eq!(id("Flight of the Amazon Queen").as_deref(), Some("queen"));

    // Arcade full names, regions and 1G1R between sf2 (World) and sf2j (Japan).
    let sf2 = roms.iter().find(|r| r.stem == "sf2").unwrap();
    assert_eq!(
        sf2.alt_name.as_deref(),
        Some("Street Fighter II - The World Warrior (World 910522)")
    );
    assert_eq!(sf2.regions, vec!["World"]);
    let mut col = Collection {
        name: "bato".into(),
        profile: "batocera".into(),
        ..Default::default()
    };
    col.filters.one_game_one_rom = true;
    let sel = collection::resolve(&lib, &col).unwrap();
    assert!(sel.roms.iter().any(|r| r.stem == "sf2"));
    assert!(!sel.roms.iter().any(|r| r.stem == "sf2j"));

    // Export to Batocera layout.
    let card = tmp.path().join("card");
    let rep = collection::export(
        &lib,
        &col,
        &card,
        &ExportOptions {
            ignore_free_space: true,
            ..Default::default()
        },
        &Silent,
    )
    .unwrap();
    assert!(card.join("roms/ps2/God of War (USA).iso").exists());
    assert!(card.join("roms/gamecube/Metroid Prime (USA).iso").exists());
    assert!(card.join("roms/3ds/Super Mario 3D Land (USA).3ds").exists());
    assert!(card
        .join("roms/xbox/Halo - Combat Evolved (USA).iso")
        .exists());
    assert!(card.join("roms/dos/Doom.pc/DOOM.EXE").exists());
    assert!(card.join("roms/dos/Commander Keen.zip").exists());
    let mi2 = card.join("roms/scummvm/Monkey Island 2.scummvm");
    assert_eq!(
        fs::read_to_string(mi2.join("Monkey Island 2.scummvm"))
            .unwrap()
            .trim(),
        "monkey2"
    );
    assert!(mi2.join("MONKEY2.001").exists());
    assert!(card.join("roms/fbneo/sf2.zip").exists());
    assert!(card.join("roms/cps2/ssf2t.zip").exists());
    let games = profiles::gamelist_games(&card.join("roms/scummvm/gamelist.xml"));
    assert!(games.iter().any(|g| g.0 == "./Monkey Island 2.scummvm"));
    // PS2 and Xbox need BIOS files the library doesn't have.
    let missing = rep.bios.missing_required.join(" | ");
    assert!(
        missing.contains("PlayStation 2") && missing.contains("Xbox"),
        "{missing}"
    );

    // Sync after dropping DOS removes the whole Doom folder we wrote.
    col.systems = vec!["ps2".into()];
    collection::export(
        &lib,
        &col,
        &card,
        &ExportOptions {
            ignore_free_space: true,
            sync: true,
            ..Default::default()
        },
        &Silent,
    )
    .unwrap();
    assert!(!card.join("roms/dos/Doom.pc/DOOM.EXE").exists());

    // Trash / restore a folder game.
    lib.trash(doom, "test").unwrap();
    assert!(lib.root.join("trash/dos/Doom/DOOM.WAD").exists());
    let t = lib.roms(&[], "trashed").unwrap();
    lib.restore(&t[0]).unwrap();
    assert!(lib.root.join("roms/dos/Doom/DOOM.WAD").exists());
}

#[test]
fn new_bios_and_folders() {
    assert_eq!(bios::requirements_for(Some(&["ps2".into()])).len(), 1);
    assert!(bios::requirements_for(Some(&["ps2".into()]))[0].files.len() > 50);
    assert_eq!(
        systems::from_folder_name("Nintendo GameCube (GC)")
            .unwrap()
            .id,
        "gamecube"
    );
    assert_eq!(systems::from_folder_name("PS2").unwrap().id, "ps2");
    assert_eq!(
        systems::from_folder_name("mame2003plus").unwrap().id,
        "mame"
    );
    assert_eq!(systems::from_folder_name("3ds").unwrap().id, "n3ds");
    let onion = profiles::get("onion").unwrap();
    assert_eq!(
        onion.rom_dir(systems::get("cps3").unwrap(), &Default::default()),
        "Roms/CPS3"
    );
}

/// Live check against libretro-database (run with `cargo test -- --ignored`).
#[test]
#[ignore]
fn live_name_lists() {
    let tmp = tempfile::tempdir().unwrap();
    let fetch = tabularium_core::http::default_fetcher();
    let names = tabularium_core::metadata::arcade_names(tmp.path(), &fetch).expect("arcade list");
    println!("arcade names: {}", names.len());
    for s in ["sf2", "sfiii3", "ssf2t", "mslug", "pacman", "dkong"] {
        println!("  {s} -> {:?}", names.get(s));
    }
    let ids = tabularium_core::metadata::scummvm_ids(tmp.path(), &fetch).expect("scummvm list");
    println!("scummvm titles: {}", ids.len());
    for t in [
        "Monkey Island 2 - LeChuck's Revenge",
        "Day of the Tentacle",
        "Beneath a Steel Sky",
        "Sam & Max Hit the Road",
        "Grim Fandango",
    ] {
        println!(
            "  {t} -> {:?}",
            tabularium_core::metadata::guess_scummvm_id(&ids, t)
        );
    }
}

/// Live: arcade box art via full name (run with `cargo test -- --ignored`).
#[test]
#[ignore]
fn live_arcade_art() {
    use tabularium_core::scrapers::{libretro::LibretroScraper, Scraper};
    let tmp = tempfile::tempdir().unwrap();
    let mut sc = LibretroScraper::new(
        tmp.path().to_path_buf(),
        tabularium_core::http::default_fetcher(),
    );
    let rom = tabularium_core::library::Rom {
        id: 1,
        system: "arcade".into(),
        path: "roms/arcade/sf2.zip".into(),
        files: vec![],
        stem: "sf2".into(),
        title: "".into(),
        key: "".into(),
        regions: vec![],
        languages: vec![],
        revision: vec![0],
        disc: None,
        flags: Default::default(),
        verified: false,
        crc: None,
        content_size: None,
        total_size: 0,
        status: "active".into(),
        status_reason: None,
        alt_name: Some("Street Fighter II: The World Warrior (World 910522)".into()),
        source: None,
        ..Default::default()
    };
    let img = sc
        .fetch(&rom, systems::get("arcade").unwrap(), "boxart")
        .unwrap();
    println!("sf2 boxart bytes: {:?}", img.as_ref().map(|d| d.len()));
    assert!(img.is_some());
}

/// Builds a demo library at $DEMO_DIR (run with `cargo test -- --ignored`).
#[test]
#[ignore]
fn demo_library() {
    let Ok(dir) = std::env::var("DEMO_DIR") else {
        return;
    };
    let dir = Path::new(&dir);
    let src = dir.join("src");
    build(&src);
    let lib = Library::create(&dir.join("lib")).unwrap();
    let opts = ImportOptions {
        fetch: Some(tabularium_core::http::default_fetcher()),
        ..Default::default()
    };
    import(&lib, &[src], &opts, &Silent).unwrap();
}
