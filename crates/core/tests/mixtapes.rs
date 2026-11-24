//! Hand-picked games, tags, played state, random picks and play history.

use std::fs;
use std::path::Path;
use tabularium_core::collection::{self, Collection, ExportOptions};
use tabularium_core::import::{import, ImportOptions};
use tabularium_core::library::{Library, FINISHED, PLAYED};
use tabularium_core::progress::Silent;
use tabularium_core::{playhistory, profiles};

fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

fn setup() -> (tempfile::TempDir, Library) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    for (i, t) in [
        "Tetris (World)",
        "Kirby's Dream Land (USA)",
        "Metroid II (World)",
        "Dr. Mario (World)",
        "Wario Land (World)",
        "Super Mario Land (World)",
        "Pokemon Red (USA)",
        "Link's Awakening (USA)",
    ]
    .iter()
    .enumerate()
    {
        write(&src.join(format!("gb/{t}.gb")), format!("gb{i}").as_bytes());
    }
    for t in ["Sonic (USA)", "Streets of Rage (USA)", "Golden Axe (USA)"] {
        write(&src.join(format!("md/{t}.md")), t.as_bytes());
    }
    let lib = Library::create(&tmp.path().join("lib")).unwrap();
    import(&lib, &[src], &ImportOptions::default(), &Silent).unwrap();
    (tmp, lib)
}

fn id(lib: &Library, stem: &str) -> i64 {
    lib.active()
        .unwrap()
        .into_iter()
        .find(|r| r.stem == stem)
        .unwrap()
        .id
}

#[test]
fn tags_played_picked_and_random() {
    let (_t, lib) = setup();
    let tetris = id(&lib, "Tetris (World)");
    let sonic = id(&lib, "Sonic (USA)");
    let kirby = id(&lib, "Kirby's Dream Land (USA)");
    lib.add_tag(&[tetris, kirby], "puzzle-ish").unwrap();
    lib.add_tag(&[tetris], "favourites").unwrap();
    lib.remove_tag(&[kirby], "puzzle-ish").unwrap();
    assert_eq!(
        lib.all_tags().unwrap(),
        vec![("favourites".into(), 1), ("puzzle-ish".into(), 1)]
    );
    assert!(lib.get(tetris).unwrap().tags.contains("favourites"));
    lib.set_played(&[kirby], PLAYED).unwrap();
    lib.set_played(&[id(&lib, "Wario Land (World)")], FINISHED)
        .unwrap();
    lib.set_genre(sonic, Some("Platform")).unwrap();

    // Hand-picked only: exactly the picks, regardless of system.
    let mut col = Collection {
        name: "mix".into(),
        use_rules: false,
        picked: vec![sonic, tetris],
        ..Default::default()
    };
    let sel = collection::resolve(&lib, &col).unwrap();
    assert_eq!(sel.roms.len(), 2);

    // Picks + rules: Game Boy, unplayed only, 3 at random.
    col.use_rules = true;
    col.systems = vec!["gb".into()];
    col.filters.only_unplayed = true;
    col.random_count = Some(3);
    col.random_seed = 42;
    let sel = collection::resolve(&lib, &col).unwrap();
    let ids: Vec<i64> = sel.roms.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), 5, "2 picks + 3 random");
    assert!(ids.contains(&sonic) && ids.contains(&tetris));
    assert!(!ids.contains(&kirby), "played games are left out");
    // Same seed, same pick; another seed usually differs.
    assert_eq!(
        collection::resolve(&lib, &col)
            .unwrap()
            .roms
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        ids
    );
    let others: Vec<Vec<i64>> = (1..6)
        .map(|s| {
            col.random_seed = s;
            collection::resolve(&lib, &col)
                .unwrap()
                .roms
                .iter()
                .map(|r| r.id)
                .collect()
        })
        .collect();
    assert!(others.iter().any(|o| o != &ids));

    // Tag and genre rules.
    let col = Collection {
        name: "tags".into(),
        filters: tabularium_core::filters::FilterSpec {
            tags: vec!["favourites".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(collection::resolve(&lib, &col).unwrap().roms.len(), 1);
    let col = Collection {
        name: "genre".into(),
        filters: tabularium_core::filters::FilterSpec {
            genres: vec!["Platform".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(collection::resolve(&lib, &col).unwrap().roms[0].id, sonic);

    // Old collection files (include_ids) still load as picks.
    let json = r#"{"name":"old","profile":"onion","include_ids":[5,6]}"#;
    let old: Collection = serde_json::from_str(json).unwrap();
    assert_eq!(old.picked, vec![5, 6]);
    assert!(old.use_rules);
}

#[test]
fn play_history_from_gamelist_and_onion() {
    let (tmp, lib) = setup();
    // An EmulationStation card that has been played on.
    let card = tmp.path().join("es-card");
    let col = Collection {
        name: "es".into(),
        profile: "emulationstation".into(),
        ..Default::default()
    };
    let opts = ExportOptions {
        ignore_free_space: true,
        ..Default::default()
    };
    collection::export(&lib, &col, &card, &opts, &Silent).unwrap();
    let gl = card.join("roms/gb/gamelist.xml");
    let xml = fs::read_to_string(&gl).unwrap().replace(
        "<name>Tetris</name>",
        "<name>Tetris</name><playcount>7</playcount><lastplayed>20250102T030405</lastplayed>",
    );
    fs::write(&gl, xml).unwrap();
    // Copying again reads the history first, and keeps the play count in the gamelist.
    let rep = collection::export(&lib, &col, &card, &opts, &Silent).unwrap();
    assert_eq!(rep.history.as_ref().unwrap().updated, 1);
    let t = lib.get(id(&lib, "Tetris (World)")).unwrap();
    assert_eq!((t.played, t.play_count), (PLAYED, 7));
    assert_eq!(t.last_played, playhistory::parse_es_time("20250102T030405"));
    assert!(fs::read_to_string(&gl)
        .unwrap()
        .contains("<playcount>7</playcount>"));
    // Reading again changes nothing; finished stays finished.
    lib.set_played(&[t.id], FINISHED).unwrap();
    let again = playhistory::read_card(
        &lib,
        &card,
        profiles::get("emulationstation").unwrap(),
        &Default::default(),
        &Silent,
    )
    .unwrap();
    assert_eq!(again.updated, 0);
    assert_eq!(lib.get(t.id).unwrap().played, FINISHED);

    // An Onion card with a play-activity database.
    let onion = tmp.path().join("onion-card");
    let dbp = onion.join("Saves/CurrentProfile/play_activity/play_activity_db.sqlite");
    fs::create_dir_all(dbp.parent().unwrap()).unwrap();
    let db = rusqlite::Connection::open(&dbp).unwrap();
    db.execute_batch(
        "CREATE TABLE rom (id INTEGER PRIMARY KEY, type TEXT, name TEXT, file_path TEXT, image_path TEXT);
         CREATE TABLE play_activity (rom_id INTEGER, play_time INTEGER, created_at INTEGER, updated_at INTEGER);
         INSERT INTO rom VALUES (1, 'ROM', 'Sonic', '../../Roms/MD/Sonic (USA).md', NULL);
         INSERT INTO play_activity VALUES (1, 600, 1700000000, 1700000600);
         INSERT INTO play_activity VALUES (1, 300, 1700100000, 1700100300);",
    )
    .unwrap();
    drop(db);
    let rep = playhistory::read_card(
        &lib,
        &onion,
        profiles::get("onion").unwrap(),
        &Default::default(),
        &Silent,
    )
    .unwrap();
    assert_eq!(rep.updated, 1, "{:?}", rep.notes);
    let s = lib.get(id(&lib, "Sonic (USA)")).unwrap();
    assert_eq!(
        (s.played, s.play_count, s.last_played),
        (PLAYED, 2, Some(1_700_100_000.0))
    );
}
