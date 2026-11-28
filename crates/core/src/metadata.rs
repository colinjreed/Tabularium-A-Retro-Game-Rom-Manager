//! Name lists from libretro-database, downloaded once and cached:
//!
//! * arcade: MAME / FBNeo short name ("sf2") -> full name
//!   ("Street Fighter II - The World Warrior (World 910522)");
//! * ScummVM: game title -> ScummVM game ID ("monkey2").

use crate::folder_games;
use crate::http::Fetcher;
use crate::library::Library;
use crate::naming::title_key;
use crate::progress::Reporter;
use crate::{systems, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DB: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master";
const ARCADE_DATS: &[&str] = &[
    "metadat/fbneo-split/FBNeo%20-%20Arcade%20Games.dat",
    "metadat/mame/MAME.dat",
];
const SCUMMVM_DAT: &str = "dat/ScummVM.dat";
const MAX_AGE: Duration = Duration::from_secs(60 * 86400);

pub fn cache_dir(lib: &Library) -> PathBuf {
    lib.root.join("media").join(".cache")
}

/// (full name, short name) pairs from a clrmamepro DAT of zipped sets.
pub fn parse_arcade_dat(text: &str) -> Vec<(String, String)> {
    let re =
        Regex::new(r#"(?s)game \(\s*name "((?:[^"\\]|\\.)*)".*?rom \( name "?([^"\s]+)\.zip"?"#)
            .unwrap();
    re.captures_iter(text)
        .map(|c| (c[1].replace("\\\"", "\""), c[2].to_lowercase()))
        .collect()
}

/// (title, game id) pairs from libretro's ScummVM DAT.
pub fn parse_scummvm_dat(text: &str) -> Vec<(String, String)> {
    let re = Regex::new(r#"(?s)game \(\s*name "((?:[^"\\]|\\.)*)".*?code "([^"]+)""#).unwrap();
    re.captures_iter(text)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect()
}

fn fresh(p: &Path) -> bool {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < MAX_AGE)
}

fn load_or_fetch(
    cache: &Path,
    file: &str,
    urls: &[&str],
    fetch: &Fetcher,
    parse: fn(&str) -> Vec<(String, String)>,
    key: fn(&(String, String)) -> String,
    value: fn(&(String, String)) -> String,
) -> Option<HashMap<String, String>> {
    let path = cache.join(file);
    if fresh(&path) {
        if let Some(m) = fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
        {
            return Some(m);
        }
    }
    let mut map: HashMap<String, String> = HashMap::new();
    let mut any = false;
    for u in urls {
        if let Ok(Some(body)) = fetch(&format!("{DB}/{u}")) {
            any = true;
            for pair in parse(&String::from_utf8_lossy(&body)) {
                map.entry(key(&pair)).or_insert_with(|| value(&pair)); // first list wins
            }
        }
    }
    if !any {
        // Offline: fall back to a stale cache if there is one.
        return fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
    }
    let _ = fs::create_dir_all(cache);
    let _ = fs::write(&path, serde_json::to_vec(&map).unwrap());
    Some(map)
}

/// Arcade short name -> full name.
pub fn arcade_names(cache: &Path, fetch: &Fetcher) -> Option<HashMap<String, String>> {
    load_or_fetch(
        cache,
        "arcade-names.json",
        ARCADE_DATS,
        fetch,
        parse_arcade_dat,
        |p| p.1.clone(),
        |p| p.0.clone(),
    )
}

/// title_key(ScummVM title) -> game id.
pub fn scummvm_ids(cache: &Path, fetch: &Fetcher) -> Option<HashMap<String, String>> {
    load_or_fetch(
        cache,
        "scummvm-ids.json",
        &[SCUMMVM_DAT],
        fetch,
        parse_scummvm_dat,
        |p| title_key(p.0.trim_start_matches('_')),
        |p| p.1.clone(),
    )
}

/// Guess a ScummVM ID from a folder's title.
pub fn guess_scummvm_id(ids: &HashMap<String, String>, title: &str) -> Option<String> {
    let parsed = crate::naming::parse(title);
    ids.get(&parsed.key).cloned()
}

/// Fill in arcade full names and missing ScummVM IDs for library games,
/// downloading the name lists if needed. Network problems are logged, not fatal.
pub fn enrich(lib: &Library, fetch: &Fetcher, rep: &dyn Reporter) -> Result<()> {
    let roms = lib.active()?;
    let arcade: Vec<_> = roms
        .iter()
        .filter(|r| systems::is_arcade(&r.system) && r.alt_name.is_none())
        .collect();
    let scumm: Vec<_> = roms
        .iter()
        .filter(|r| r.system == "scummvm")
        .filter(|r| {
            let p = lib.abspath(&r.path);
            p.is_dir() && folder_games::scummvm_id(&p).is_none()
        })
        .collect();
    let cache = cache_dir(lib);
    if !arcade.is_empty() {
        rep.progress(0, 0, "Fetching arcade game names…");
        match arcade_names(&cache, fetch) {
            Some(names) => {
                let mut n = 0;
                for r in &arcade {
                    if let Some(full) = names.get(&r.stem.to_lowercase()) {
                        lib.set_alt_name(r.id, full)?;
                        n += 1;
                    }
                }
                rep.log(format!("Named {n} of {} arcade sets.", arcade.len()));
            }
            None => rep.log(
                "Couldn't download the arcade name list; arcade games keep their short names."
                    .into(),
            ),
        }
    }
    if !scumm.is_empty() {
        identify_scummvm(lib, Some(fetch), rep)?;
    }
    enrich_genres(lib, fetch, rep)?;
    Ok(())
}

// ------------------------------------------------------------------ genres

/// CRC -> genre from a libretro genre DAT.
pub fn parse_genre_dat(text: &str) -> HashMap<String, String> {
    let re =
        Regex::new(r#"(?s)game \(.*?genre "([^"]+)".*?rom \([^)]*?crc ([0-9A-Fa-f]{8})"#).unwrap();
    let mut out = HashMap::new();
    for c in re.captures_iter(text) {
        out.entry(c[2].to_lowercase())
            .or_insert_with(|| c[1].to_string());
    }
    out
}

fn genre_url(system: &str) -> Option<String> {
    let s = systems::get(system)?;
    if s.disc_based || systems::is_arcade(system) || systems::folder_games(system) {
        return None;
    }
    let enc =
        percent_encoding::utf8_percent_encode(s.libretro?, percent_encoding::NON_ALPHANUMERIC);
    Some(format!("{DB}/metadat/genre/{enc}.dat"))
}

fn genre_map(cache: &Path, system: &str, fetch: &Fetcher) -> Option<HashMap<String, String>> {
    let url = genre_url(system)?;
    let path = cache.join("genre").join(format!("{system}.json"));
    if fresh(&path) {
        if let Some(m) = fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
        {
            return Some(m);
        }
    }
    match fetch(&url) {
        Ok(Some(body)) => {
            let map = parse_genre_dat(&String::from_utf8_lossy(&body));
            let _ = fs::create_dir_all(path.parent().unwrap());
            let _ = fs::write(&path, serde_json::to_vec(&map).unwrap());
            Some(map)
        }
        _ => fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok()),
    }
}

/// Fill in genres for ROMs that don't have one yet. Returns how many were found.
/// ScummVM game folders in the library that have no game ID yet.
pub fn scummvm_missing_ids(lib: &Library) -> Result<Vec<crate::library::Rom>> {
    Ok(lib
        .active()?
        .into_iter()
        .filter(|r| r.system == "scummvm")
        .filter(|r| {
            let p = lib.abspath(&r.path);
            p.is_dir() && folder_games::scummvm_id(&p).is_none()
        })
        .collect())
}

/// Give every ScummVM game folder without one a game ID, writing the
/// `<Game>.scummvm` file firmware launches. Tries well-known files, then an
/// installed ScummVM (which knows every supported game by its files), then
/// libretro's ScummVM list by title. Returns (identified, still missing).
pub fn identify_scummvm(
    lib: &Library,
    fetch: Option<&Fetcher>,
    rep: &dyn Reporter,
) -> Result<(usize, usize)> {
    let todo = scummvm_missing_ids(lib)?;
    if todo.is_empty() {
        return Ok((0, 0));
    }
    let exe = folder_games::find_scummvm();
    match &exe {
        Some(p) => rep.log(format!(
            "Asking ScummVM ({}) to identify games.",
            p.display()
        )),
        None => rep.log(
            "ScummVM isn't installed (or wasn't found), so games are matched by name only. \
             Install ScummVM and run this again to identify more."
                .into(),
        ),
    }
    rep.progress(0, 0, "Fetching ScummVM game list…");
    let ids = fetch.and_then(|f| scummvm_ids(&cache_dir(lib), f));
    let valid: Option<std::collections::HashSet<String>> =
        ids.as_ref().map(|m| m.values().cloned().collect());
    let (mut found, mut missing) = (0, 0);
    for (i, r) in todo.iter().enumerate() {
        if rep.cancelled() {
            break;
        }
        rep.progress(i, todo.len(), &format!("Identifying {}", r.stem));
        let dir = lib.abspath(&r.path);
        let id = folder_games::detect_scummvm_id(&dir, valid.as_ref())
            .or_else(|| {
                exe.as_deref()
                    .and_then(|e| folder_games::scummvm_detect(e, &dir))
            })
            .or_else(|| ids.as_ref().and_then(|m| guess_scummvm_id(m, &r.stem)));
        match id {
            Some(id) => {
                folder_games::write_scummvm_id(&dir, &r.stem, &id)?;
                rep.log(format!("ScummVM: {} -> {id}", r.stem));
                found += 1;
            }
            None => {
                rep.log(format!(
                    "ScummVM: couldn't identify '{}'. Set its game ID in the Library.",
                    r.stem
                ));
                missing += 1;
            }
        }
    }
    Ok((found, missing))
}

pub fn enrich_genres(lib: &Library, fetch: &Fetcher, rep: &dyn Reporter) -> Result<usize> {
    let roms: Vec<_> = lib
        .active()?
        .into_iter()
        .filter(|r| r.genre.is_none() && r.crc.is_some())
        .collect();
    let cache = cache_dir(lib);
    let mut by_sys: HashMap<String, Vec<&crate::library::Rom>> = HashMap::new();
    for r in &roms {
        by_sys.entry(r.system.clone()).or_default().push(r);
    }
    let mut found = 0;
    for (sys, list) in by_sys {
        if genre_url(&sys).is_none() {
            continue;
        }
        rep.progress(
            0,
            0,
            &format!("Looking up genres: {}", systems::name_of(&sys)),
        );
        let Some(map) = genre_map(&cache, &sys, fetch) else {
            continue;
        };
        for r in list {
            let path = lib.abspath(&r.path);
            let crcs = crate::dats::candidate_crcs(&path, r.crc.as_deref().unwrap_or(""));
            if let Some(g) = crcs.iter().find_map(|c| map.get(c)) {
                lib.set_genre(r.id, Some(g))?;
                found += 1;
            }
        }
    }
    if found > 0 {
        rep.log(format!("Found genres for {found} game(s)."));
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dats() {
        let arcade = r#"
game (
	name "Street Fighter II - The World Warrior (World 910522)"
	year "1991"
	rom ( name sf2.zip size 1 crc 0 )
)
game (
	name "Final Fight (US, rev 1)"
	rom ( name ffightu1.zip size 1 crc 0 )
)
"#;
        let a = parse_arcade_dat(arcade);
        assert_eq!(
            a[0],
            (
                "Street Fighter II - The World Warrior (World 910522)".into(),
                "sf2".into()
            )
        );
        assert_eq!(a[1].1, "ffightu1");
        let scumm = r#"
game (
	name "Monkey Island 2: LeChuck's Revenge"
	code "monkey2"
	rom ( name "monkey2.scummvm" size 7 crc 0 )
)
"#;
        let g = parse_genre_dat(
            "game (\n\tcomment \"3 Ninjas Kick Back (USA)\"\n\tgenre \"Platform\"\n\trom ( crc F2EE11F9 )\n)\n",
        );
        assert_eq!(g["f2ee11f9"], "Platform");
        let s = parse_scummvm_dat(scumm);
        assert_eq!(s[0].1, "monkey2");
        let ids: HashMap<String, String> =
            s.iter().map(|p| (title_key(&p.0), p.1.clone())).collect();
        assert_eq!(
            guess_scummvm_id(&ids, "Monkey Island 2 - LeChuck's Revenge").as_deref(),
            Some("monkey2")
        );
    }
}
