//! End-to-end tests on a generated "messy" collection. No network access.

use md5::{Digest, Md5};
use rom_manager_core::collection::{self, Collection, ExportOptions};
use rom_manager_core::filters::FilterSpec;
use rom_manager_core::http::{Fetcher, HttpError};
use rom_manager_core::import::{import, ImportOptions};
use rom_manager_core::library::Library;
use rom_manager_core::naming::DEFAULT_JUNK;
use rom_manager_core::progress::Silent;
use rom_manager_core::scrape::scrape_library;
use rom_manager_core::scrapers::screenscraper::{pick_media, Credentials, ScreenScraper};
use rom_manager_core::scrapers::{ScrapeError, Scraper};
use rom_manager_core::{bios, profiles, systems, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn rand(n: usize, seed: &str) -> Vec<u8> {
    let mut out = vec![];
    let mut i = 0;
    while out.len() < n {
        out.extend_from_slice(&Md5::digest(format!("{seed}{i}").as_bytes()));
        i += 1;
    }
    out.truncate(n);
    out
}

fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

fn genesis(seed: &str) -> Vec<u8> {
    let mut d = rand(0x20000, seed);
    d[0x100..0x110].copy_from_slice(b"SEGA GENESIS    ");
    d
}

fn psx_bin(seed: &str) -> Vec<u8> {
    let mut d = rand(20 * 2352, seed);
    let off = 16 * 2352 + 24;
    d[off..off + 6].copy_from_slice(b"\x01CD001");
    d[off + 8..off + 8 + 11].copy_from_slice(b"PLAYSTATION");
    d
}

fn zip_one(path: &Path, inner: &str, data: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
    z.start_file(inner, zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(data).unwrap();
    z.finish().unwrap();
}

fn messy(root: &Path) -> PathBuf {
    let src = root.join("downloads");
    let tetris = rand(32768, "tetris");
    write(&src.join("random/Tetris (World) (Rev 1).gb"), &tetris);
    write(
        &src.join("random/Tetris (Japan).gb"),
        &rand(32768, "tetris-jp"),
    );
    write(
        &src.join("random/Tetris (World) (Beta).gb"),
        &rand(32768, "tetris-beta"),
    );
    write(
        &src.join("random/Tetris DX (World) (Hack).gbc"),
        &rand(32768, "tdx"),
    );
    zip_one(
        &src.join("zips/Tetris (World) (Rev 1).zip"),
        "Tetris (World) (Rev 1).gb",
        &tetris,
    );
    zip_one(
        &src.join("zips/mario.zip"),
        "Super Mario Advance (USA).gba",
        &rand(65536, "sma"),
    );
    write(
        &src.join("gb/Super Mario Land 2 - 6 Golden Coins (USA, Europe).gb"),
        &rand(4096, "s"),
    );
    write(
        &src.join("gb/Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2).gb"),
        &rand(4096, "s2"),
    );
    write(
        &src.join("misc/Sonic The Hedgehog (USA, Europe).bin"),
        &genesis("sonic"),
    );
    write(
        &src.join("Atari 2600/Pitfall! (USA).bin"),
        &rand(4096, "pitfall"),
    );
    for d in 1..=2 {
        let stem = format!("Final Fantasy VII (USA) (Disc {d})");
        write(
            &src.join(format!("discs/{stem}.bin")),
            &psx_bin(&format!("ff7-{d}")),
        );
        write(
            &src.join(format!("discs/{stem}.cue")),
            format!("FILE \"{stem}.bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n")
                .as_bytes(),
        );
    }
    write(&src.join("random/readme.txt"), b"hello");
    write(&src.join("random/mystery.bin"), &rand(100000, "mystery"));
    src
}

fn setup() -> (tempfile::TempDir, Library, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let lib = Library::create(&tmp.path().join("library")).unwrap();
    let src = messy(tmp.path());
    (tmp, lib, src)
}

fn junk_1g1r() -> FilterSpec {
    FilterSpec {
        exclude_flags: DEFAULT_JUNK
            .iter()
            .map(|s| s.to_string())
            .chain(["hack".to_string()])
            .collect(),
        one_game_one_rom: true,
        ..Default::default()
    }
}

#[test]
fn detection_and_import() -> Result<()> {
    let (_t, lib, src) = setup();
    let rep = import(
        &lib,
        std::slice::from_ref(&src),
        &ImportOptions::default(),
        &Silent,
    )?;
    let sys = |stem: &str| {
        lib.active()
            .unwrap()
            .into_iter()
            .find(|r| r.stem == stem)
            .map(|r| r.system)
    };
    assert_eq!(sys("mario").as_deref(), Some("gba")); // zip contents
    assert_eq!(
        sys("Sonic The Hedgehog (USA, Europe)").as_deref(),
        Some("genesis")
    ); // header
    assert_eq!(sys("Pitfall! (USA)").as_deref(), Some("atari2600")); // folder name
    assert_eq!(
        sys("Final Fantasy VII (USA) (Disc 1)").as_deref(),
        Some("psx")
    ); // disc header
    assert_eq!(rep.duplicates.len(), 1); // zipped Tetris == loose Tetris
    assert_eq!(rep.unknown.len(), 1);
    assert_eq!(rep.ignored, 1);
    let ff7 = lib
        .active()?
        .into_iter()
        .find(|r| r.stem.ends_with("(Disc 1)"))
        .unwrap();
    assert_eq!(ff7.files.len(), 2);
    assert!(lib
        .root
        .join("roms/psx/Final Fantasy VII (USA) (Disc 1).bin")
        .exists());
    // re-import adds nothing
    let again = import(&lib, &[src], &ImportOptions::default(), &Silent)?;
    assert_eq!(again.added_total(), 0);
    Ok(())
}

#[test]
fn clean_1g1r_trash_restore() -> Result<()> {
    let (_t, lib, src) = setup();
    import(&lib, &[src], &ImportOptions::default(), &Silent)?;
    let (kept, rejected) = rom_manager_core::filters::apply(lib.active()?, &junk_1g1r());
    let stems: Vec<&str> = kept.iter().map(|r| r.stem.as_str()).collect();
    assert!(stems.contains(&"Tetris (World) (Rev 1)"));
    assert!(!stems.contains(&"Tetris (Japan)"));
    assert!(!stems.contains(&"Tetris (World) (Beta)"));
    assert!(!stems.contains(&"Tetris DX (World) (Hack)"));
    assert!(stems.contains(&"Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2)"));
    assert!(!stems.contains(&"Super Mario Land 2 - 6 Golden Coins (USA, Europe)"));
    assert!(
        stems.contains(&"Final Fantasy VII (USA) (Disc 1)")
            && stems.contains(&"Final Fantasy VII (USA) (Disc 2)")
    );
    for r in &rejected {
        lib.trash(&r.rom, &r.reason)?;
    }
    assert!(lib.root.join("trash/gb/Tetris (Japan).gb").exists());
    let jp = lib
        .roms(&[], "trashed")?
        .into_iter()
        .find(|r| r.stem == "Tetris (Japan)")
        .unwrap();
    lib.restore(&jp)?;
    assert!(lib.root.join("roms/gb/Tetris (Japan).gb").exists());
    // Japan-first priority picks the Japanese release
    let f = FilterSpec {
        one_game_one_rom: true,
        region_priority: vec!["Japan".into(), "USA".into()],
        ..Default::default()
    };
    let (kept, _) = rom_manager_core::filters::apply(lib.roms(&["gb".into()], "active")?, &f);
    assert!(kept.iter().any(|r| r.stem == "Tetris (Japan)"));
    Ok(())
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(w, h, image::Rgb([128, 64, 32]));
    let mut out = std::io::Cursor::new(vec![]);
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

struct FakeScraper;

impl Scraper for FakeScraper {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn kinds(&self) -> &'static [&'static str] {
        &["boxart"]
    }
    fn fetch(
        &mut self,
        rom: &rom_manager_core::library::Rom,
        _: &systems::System,
        _: &str,
    ) -> std::result::Result<Option<Vec<u8>>, ScrapeError> {
        Ok((!rom.stem.contains("Pitfall")).then(|| png(600, 400)))
    }
}

fn collection(lib: &Library, src: PathBuf, profile: &str) -> Result<Collection> {
    import(lib, &[src], &ImportOptions::default(), &Silent)?;
    let mut scrapers: Vec<Box<dyn Scraper>> = vec![Box::new(FakeScraper)];
    scrape_library(lib, &mut scrapers, &["boxart".into()], &[], false, &Silent)?;
    let col = Collection {
        name: "test".into(),
        profile: profile.into(),
        filters: junk_1g1r(),
        ..Default::default()
    };
    col.save(lib)?;
    Collection::load(lib, "test")
}

fn force() -> ExportOptions {
    ExportOptions {
        ignore_free_space: true,
        ..Default::default()
    }
}

#[test]
fn export_onion() -> Result<()> {
    let (t, lib, src) = setup();
    let col = collection(&lib, src, "onion")?;
    let card = t.path().join("card");
    let rep = collection::export(&lib, &col, &card, &force(), &Silent)?;
    assert!(card.join("Roms/GB/Tetris (World) (Rev 1).gb").exists());
    assert!(!card.join("Roms/GB/Tetris (Japan).gb").exists());
    assert!(card
        .join("Roms/MD/Sonic The Hedgehog (USA, Europe).bin")
        .exists());
    let img = image::open(card.join("Roms/GB/Imgs/Tetris (World) (Rev 1).png")).unwrap();
    assert_eq!((img.width(), img.height()), (250, 167)); // Onion art width
    let m3u = fs::read_to_string(card.join("Roms/PS/Final Fantasy VII (USA).m3u"))?;
    assert_eq!(
        m3u.lines().collect::<Vec<_>>(),
        [
            ".Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 1).cue",
            ".Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 2).cue"
        ]
    );
    assert!(card
        .join("Roms/PS/.Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 2).bin")
        .exists());
    assert!(card
        .join("Roms/PS/Imgs/Final Fantasy VII (USA).png")
        .exists());
    assert_eq!(rep.art_missing, 1);
    assert!(rep
        .bios
        .missing_required
        .iter()
        .any(|m| m.starts_with("Sony PlayStation")));
    let again = collection::export(&lib, &col, &card, &force(), &Silent)?;
    assert_eq!(again.copied, 0);
    assert!(again.unchanged > 0);
    Ok(())
}

#[test]
fn export_emulationstation_gamelist_and_sync() -> Result<()> {
    let (t, lib, src) = setup();
    let mut col = collection(&lib, src, "emulationstation")?;
    let card = t.path().join("card");
    collection::export(&lib, &col, &card, &force(), &Silent)?;
    let games = profiles::gamelist_games(&card.join("roms/gb/gamelist.xml"));
    let tetris = games
        .iter()
        .find(|g| g.0 == "./Tetris (World) (Rev 1).gb")
        .unwrap();
    assert_eq!(tetris.1.as_deref(), Some("Tetris"));
    assert_eq!(
        tetris.2.as_deref(),
        Some("./images/Tetris (World) (Rev 1).png")
    );
    write(&card.join("roms/gb/my-homebrew.gb"), b"x");
    col.systems = vec!["gba".into()];
    let rep = collection::export(
        &lib,
        &col,
        &card,
        &ExportOptions {
            sync: true,
            ..force()
        },
        &Silent,
    )?;
    assert!(rep.removed > 0);
    assert!(!card.join("roms/gb/Tetris (World) (Rev 1).gb").exists());
    assert!(card.join("roms/gb/my-homebrew.gb").exists());
    assert!(card.join("roms/gba/mario.zip").exists());
    assert!(profiles::gamelist_games(&card.join("roms/gb/gamelist.xml")).is_empty());
    Ok(())
}

#[test]
fn nextui_muos_paths_and_budget() -> Result<()> {
    let (t, lib, src) = setup();
    let mut col = collection(&lib, src, "nextui")?;
    collection::export(&lib, &col, &t.path().join("n"), &force(), &Silent)?;
    assert!(t
        .path()
        .join("n/Roms/Game Boy (GB)/.media/Tetris (World) (Rev 1).png")
        .exists());
    col.profile = "muos".into();
    collection::export(&lib, &col, &t.path().join("m"), &force(), &Silent)?;
    assert!(t
        .path()
        .join("m/ROMS/gb/Tetris (World) (Rev 1).gb")
        .exists());
    assert!(t
        .path()
        .join("m/MUOS/info/catalogue/Nintendo Game Boy/box/Tetris (World) (Rev 1).png")
        .exists());
    col.max_size = Some(150 * 1024);
    let sel = collection::resolve(&lib, &col)?;
    assert!(sel.size() <= 150 * 1024);
    let discs = sel.roms.iter().filter(|r| r.system == "psx").count();
    assert!(discs == 0 || discs == 2);
    assert!(!sel.over_budget.is_empty());
    Ok(())
}

#[test]
fn bios_import_check_install() {
    let tmp = tempfile::tempdir().unwrap();
    // We can't ship real BIOS data, so check the name-only entry (neogeo.zip)
    // and that renamed / corrupt files are handled.
    let dl = tmp.path().join("dl");
    write(&dl.join("neogeo.zip"), b"any neo geo bios");
    write(&dl.join("scph5501.bin"), b"not really a bios");
    write(&dl.join("random.txt"), b"ignored");
    let lib_bios = tmp.path().join("bios");
    let res = bios::import_files(&[dl], &lib_bios, false);
    assert!(res
        .iter()
        .any(|(_, o)| matches!(o, bios::ImportOutcome::Imported(n) if n == "neogeo.zip")));
    assert!(res
        .iter()
        .any(|(_, o)| matches!(o, bios::ImportOutcome::BadHash(_))));
    assert_eq!(res.len(), 2);
    let st = bios::check(&lib_bios, Some(&["arcade".into(), "psx".into()]), None);
    assert!(st
        .iter()
        .find(|s| s.req.system == "arcade")
        .unwrap()
        .satisfied());
    assert!(!st
        .iter()
        .find(|s| s.req.system == "psx")
        .unwrap()
        .satisfied());
    let onion = profiles::get("onion").unwrap();
    let card = tmp.path().join("card");
    let rep = bios::install(
        &lib_bios,
        &card,
        onion,
        &["arcade".into(), "psx".into()],
        false,
    );
    assert!(card.join("BIOS/neogeo.zip").exists());
    assert_eq!(rep.missing_required.len(), 1);
    assert_eq!(
        profiles::get("nextui")
            .unwrap()
            .bios_dir(systems::get("psx").unwrap()),
        "Bios/PS"
    );
}

#[test]
fn screenscraper_queries_once_per_rom() {
    let calls = Arc::new(Mutex::new(vec![]));
    let c2 = calls.clone();
    let fetch: Fetcher = Arc::new(
        move |url: &str| -> std::result::Result<Option<Vec<u8>>, HttpError> {
            c2.lock().unwrap().push(url.to_string());
            if url.contains("jeuInfos") {
                Ok(Some(
                    br#"{"response":{"jeu":{"medias":[
                {"type":"box-2D","region":"jp","url":"https://m/jbox"},
                {"type":"box-2D","region":"us","url":"https://m/box"},
                {"type":"ss","region":"wor","url":"https://m/ss"}]}}}"#
                        .to_vec(),
                ))
            } else {
                Ok(Some(format!("IMG {url}").into_bytes()))
            }
        },
    );
    let creds = Credentials {
        dev_id: "d".into(),
        dev_password: "p".into(),
        ..Default::default()
    };
    let mut ss = ScreenScraper::new(creds, fetch)
        .unwrap()
        .with_interval(std::time::Duration::ZERO);
    let rom = rom_manager_core::library::Rom {
        id: 1,
        system: "gb".into(),
        path: "roms/gb/Tetris (World).gb".into(),
        files: vec![],
        stem: "Tetris (World)".into(),
        title: "Tetris".into(),
        key: "tetris".into(),
        regions: vec![],
        languages: vec![],
        revision: vec![0],
        disc: None,
        flags: Default::default(),
        verified: false,
        crc: Some("46df91ad".into()),
        content_size: Some(32768),
        total_size: 32768,
        status: "active".into(),
        status_reason: None,
        alt_name: None,
        source: None,
    };
    let gb = systems::get("gb").unwrap();
    assert_eq!(
        ss.fetch(&rom, gb, "boxart").unwrap().unwrap(),
        b"IMG https://m/box"
    );
    assert_eq!(
        ss.fetch(&rom, gb, "snap").unwrap().unwrap(),
        b"IMG https://m/ss"
    );
    assert!(ss.fetch(&rom, gb, "wheel").unwrap().is_none());
    let calls = calls.lock().unwrap();
    let api: Vec<&String> = calls.iter().filter(|c| c.contains("jeuInfos")).collect();
    assert_eq!(api.len(), 1);
    assert!(
        api[0].contains("crc=46DF91AD")
            && api[0].contains("systemeid=9")
            && api[0].contains("romtaille=32768")
    );
    assert!(ScreenScraper::new(Credentials::default(), Arc::new(|_: &str| Ok(None))).is_err());
    assert_eq!(pick_media(&[], "boxart"), None);
}
