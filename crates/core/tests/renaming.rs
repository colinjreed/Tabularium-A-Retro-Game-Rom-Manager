//! Official names by checksum, numbering removal and Game Gear `.sms` files.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tabularium_core::dats::{parse_dat, Dats};
use tabularium_core::http::{Fetcher, HttpError};
use tabularium_core::import::{import, ImportOptions};
use tabularium_core::library::Library;
use tabularium_core::progress::Silent;
use tabularium_core::rename::fix_library;

fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

fn crc(data: &[u8]) -> String {
    format!("{:08X}", crc32fast::hash(data))
}

fn dat(entries: &[(&str, &[u8], &str)]) -> String {
    entries
        .iter()
        .map(|(name, data, ext)| {
            format!(
                "game (\n\tname \"{name}\"\n\trom ( name \"{name}.{ext}\" size {} crc {} md5 X )\n)\n",
                data.len(),
                crc(data)
            )
        })
        .collect()
}

const SONIC: &[u8] = b"sonic genesis rom data";
const STREETS: &[u8] = b"streets of rage rom";
const COLUMNS_GG: &[u8] = b"columns game gear";
const SHINOBI_GG_SMS: &[u8] = b"shinobi gg in sms format";

fn fake_net() -> Fetcher {
    let genesis = dat(&[
        ("Sonic The Hedgehog (USA, Europe)", SONIC, "md"),
        ("Streets of Rage (World)", STREETS, "md"),
    ]);
    let gg = dat(&[
        ("Columns (USA, Europe)", COLUMNS_GG, "gg"),
        ("Shinobi (USA, Europe) (SMS Mode)", SHINOBI_GG_SMS, "sms"),
    ]);
    Arc::new(move |url: &str| -> Result<Option<Vec<u8>>, HttpError> {
        if url.contains("Mega%20Drive") {
            Ok(Some(genesis.clone().into_bytes()))
        } else if url.contains("Game%20Gear") {
            Ok(Some(gg.clone().into_bytes()))
        } else {
            Ok(None)
        }
    })
}

#[test]
fn import_renames_and_fixes_systems() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    // A numbered Genesis set: two known dumps, two unknown ones.
    write(&src.join("Genesis/001 Sonic.md"), SONIC);
    write(&src.join("Genesis/002 - Streets.md"), STREETS);
    write(
        &src.join("Genesis/003 - Gunstar Heroes (USA).md"),
        b"gunstar, not in list",
    );
    write(
        &src.join("Genesis/004_Golden Axe (World).md"),
        b"golden axe, not in list",
    );
    // Game Gear: a .gg, and an .sms-format GG game inside a Game Gear folder.
    write(&src.join("Game Gear/columns.gg"), COLUMNS_GG);
    write(
        &src.join("Game Gear/Some Homebrew.sms"),
        b"gg homebrew in sms format",
    );
    // An .sms file elsewhere whose checksum is in the Game Gear list.
    write(&src.join("misc/shinobi.sms"), SHINOBI_GG_SMS);
    // A real Master System game stays Master System.
    write(&src.join("misc/Alex Kidd (USA).sms"), b"alex kidd sms");

    let lib = Library::create(&tmp.path().join("lib")).unwrap();
    let opts = ImportOptions {
        fetch: Some(fake_net()),
        official_names: true,
        strip_numbers: true,
        ..Default::default()
    };
    import(&lib, &[src], &opts, &Silent).unwrap();
    let roms = lib.active().unwrap();
    let find = |stem: &str| {
        roms.iter()
            .find(|r| r.stem == stem)
            .map(|r| r.system.as_str())
    };
    assert_eq!(find("Sonic The Hedgehog (USA, Europe)"), Some("genesis"));
    assert_eq!(find("Streets of Rage (World)"), Some("genesis"));
    assert_eq!(find("Gunstar Heroes (USA)"), Some("genesis"));
    assert_eq!(find("Golden Axe (World)"), Some("genesis"));
    assert_eq!(find("Columns (USA, Europe)"), Some("gamegear"));
    assert_eq!(find("Some Homebrew"), Some("gamegear"));
    assert_eq!(find("Shinobi (USA, Europe) (SMS Mode)"), Some("gamegear"));
    assert_eq!(find("Alex Kidd (USA)"), Some("mastersystem"));
    assert!(lib
        .root
        .join("roms/genesis/Sonic The Hedgehog (USA, Europe).md")
        .exists());
    assert!(lib
        .root
        .join("roms/gamegear/Shinobi (USA, Europe) (SMS Mode).sms")
        .exists());
}

#[test]
fn fix_existing_library() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    write(&src.join("stuff/001 Sonic.md"), SONIC);
    write(&src.join("stuff/002 Something Else.md"), b"unknown 1");
    write(&src.join("stuff/003 Another Game.md"), b"unknown 2");
    write(&src.join("stuff/shinobi.sms"), SHINOBI_GG_SMS);
    let lib = Library::create(&tmp.path().join("lib")).unwrap();
    // Imported without renaming (like a library from an earlier version).
    import(&lib, &[src], &ImportOptions::default(), &Silent).unwrap();
    assert!(lib.root.join("roms/mastersystem/shinobi.sms").exists());
    // Give one ROM some artwork to check it gets renamed too.
    let sonic = lib
        .active()
        .unwrap()
        .into_iter()
        .find(|r| r.stem == "001 Sonic")
        .unwrap();
    let art = lib.media_dir("genesis", "boxart").join("001 Sonic.png");
    write(&art, b"png");
    lib.set_media(sonic.id, "boxart", Some(&art), Some("test"))
        .unwrap();

    let mut dats = Dats::new(tmp.path().join("cache"), fake_net());
    let preview = fix_library(&lib, Some(&mut dats), true, false, &Silent).unwrap();
    assert_eq!(
        preview.changes.len(),
        4,
        "{:?}",
        preview
            .changes
            .iter()
            .map(|c| &c.new_stem)
            .collect::<Vec<_>>()
    );
    assert!(lib.root.join("roms/genesis/001 Sonic.md").exists()); // preview changes nothing

    let rep = fix_library(&lib, Some(&mut dats), true, true, &Silent).unwrap();
    assert_eq!(rep.applied, 4);
    assert!(lib
        .root
        .join("roms/genesis/Sonic The Hedgehog (USA, Europe).md")
        .exists());
    assert!(lib.root.join("roms/genesis/Something Else.md").exists());
    assert!(lib
        .root
        .join("roms/gamegear/Shinobi (USA, Europe) (SMS Mode).sms")
        .exists());
    let roms = lib.active().unwrap();
    let sonic = roms.iter().find(|r| r.stem.starts_with("Sonic")).unwrap();
    assert_eq!(sonic.regions, vec!["USA", "Europe"]);
    let media = lib.media(sonic.id).unwrap();
    assert!(media["boxart"].ends_with("Sonic The Hedgehog (USA, Europe).png"));
    // Running again finds nothing more to do.
    assert!(fix_library(&lib, Some(&mut dats), true, false, &Silent)
        .unwrap()
        .changes
        .is_empty());
}

/// Live check of the real No-Intro lists (run with `cargo test -- --ignored`).
#[test]
#[ignore]
fn live_no_intro() {
    let fetch = tabularium_core::http::default_fetcher();
    for sys in ["genesis", "gamegear", "nes", "snes", "gba"] {
        let url = tabularium_core::dats::dat_url(sys).unwrap();
        let body = fetch(&url).unwrap().unwrap();
        let map = parse_dat(&String::from_utf8_lossy(&body));
        let sample = map
            .values()
            .find(|v| v.starts_with("Sonic") || v.starts_with("Super Mario"))
            .cloned();
        println!("{sys}: {} entries, e.g. {sample:?}", map.len());
        assert!(map.len() > 500);
    }
}

/// Builds a numbered demo library at $DEMO_DIR (run with `cargo test -- --ignored`).
#[test]
#[ignore]
fn demo_numbered_library() {
    let Ok(dir) = std::env::var("DEMO_DIR") else {
        return;
    };
    let dir = Path::new(&dir);
    let src = dir.join("src");
    for (i, t) in [
        "Sonic The Hedgehog (USA, Europe)",
        "Streets of Rage 2 (USA)",
        "Gunstar Heroes (USA)",
        "Altered Beast (USA, Europe)",
        "Ecco the Dolphin (USA, Europe)",
        "Golden Axe (World)",
    ]
    .iter()
    .enumerate()
    {
        write(
            &src.join(format!("Genesis/{:03} {t}.md", 120 - i * 17)),
            t.as_bytes(),
        );
    }
    let lib = Library::create(&dir.join("lib")).unwrap();
    import(&lib, &[src], &ImportOptions::default(), &Silent).unwrap();
}
