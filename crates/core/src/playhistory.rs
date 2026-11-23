//! Read play history back from an SD card into the library.
//!
//! * EmulationStation family (ArkOS, ROCKNIX, Knulli/Batocera, ES-DE) keeps
//!   `<playcount>` and `<lastplayed>` per game in `gamelist.xml`.
//! * Onion OS keeps a play-time database under
//!   `Saves/CurrentProfile/play_activity/`.
//!
//! Games are matched by system folder and file name. Play history only ever
//! adds to what the library knows; it never marks games as unplayed.

use crate::library::{split_name, Library, Rom};
use crate::profiles::Profile;
use crate::progress::Reporter;
use crate::{naming, systems, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct PlayReport {
    /// Games on the card with any play history.
    pub found: usize,
    /// Library games whose played state / count / date changed.
    pub updated: usize,
    pub sources: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct Entry {
    folder: String,
    file: String,
    count: u32,
    last: Option<f64>,
}

/// "20240131T235959" -> Unix seconds (UTC; close enough for "last played").
pub fn parse_es_time(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.len() < 15 || s.as_bytes()[8] != b'T' {
        return None;
    }
    let n = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d) = (n(0, 4)?, n(4, 6)?, n(6, 8)?);
    let (h, mi, se) = (n(9, 11)?, n(11, 13)?, n(13, 15)?);
    if y < 1980 {
        return None; // ES writes "not-a-date-time" or zeros for never
    }
    // Days from civil date (Howard Hinnant's algorithm).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some((days * 86400 + h * 3600 + mi * 60 + se) as f64)
}

fn gamelist_entries(path: &Path, folder: &str) -> Vec<Entry> {
    crate::profiles::gamelist_play_info(path)
        .into_iter()
        .filter(|(_, count, last)| *count > 0 || last.is_some())
        .map(|(p, count, last)| Entry {
            folder: folder.to_string(),
            file: p.trim_start_matches("./").to_string(),
            count,
            last: last.as_deref().and_then(parse_es_time),
        })
        .collect()
}

fn onion_entries(card: &Path) -> std::result::Result<Vec<Entry>, String> {
    let db = card.join("Saves/CurrentProfile/play_activity/play_activity_db.sqlite");
    if !db.exists() {
        return Ok(vec![]);
    }
    let conn =
        rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;
    let mut st = conn
        .prepare(
            "SELECT r.file_path, COUNT(p.rom_id), MAX(p.created_at) FROM rom r \
             JOIN play_activity p ON p.rom_id = r.id GROUP BY r.id",
        )
        .map_err(|e| format!("unexpected Onion database layout ({e})"))?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = vec![];
    for row in rows.flatten() {
        let (path, count, last) = row;
        // e.g. "../../Roms/GB/Tetris (World).gb" or "/mnt/SDCARD/Roms/GB/…"
        let parts: Vec<&str> = path.split(['/', '\\']).collect();
        let Some(i) = parts.iter().position(|p| p.eq_ignore_ascii_case("Roms")) else {
            continue;
        };
        if parts.len() < i + 3 {
            continue;
        }
        out.push(Entry {
            folder: format!("Roms/{}", parts[i + 1]),
            file: parts[i + 2..].join("/"),
            count: count.max(1) as u32,
            last: last.map(|t| t as f64),
        });
    }
    Ok(out)
}

/// Read play history from a card laid out for `profile`.
pub fn read_card(
    lib: &Library,
    card: &Path,
    profile: &Profile,
    overrides: &BTreeMap<String, String>,
    rep: &dyn Reporter,
) -> Result<PlayReport> {
    let mut report = PlayReport::default();
    // Which card folder belongs to which system (with this card's overrides).
    let mut folder_system: HashMap<String, &'static str> = HashMap::new();
    for s in systems::SYSTEMS {
        folder_system.insert(profile.rom_dir(s, overrides).to_lowercase(), s.id);
    }
    let mut entries: Vec<Entry> = vec![];
    for folder in folder_system.keys() {
        let gl = card.join(folder).join("gamelist.xml");
        if gl.exists() {
            entries.extend(gamelist_entries(&gl, folder));
        }
    }
    // ES-DE keeps gamelists in its own folder.
    for s in systems::SYSTEMS {
        let gl = card
            .join("ES-DE/gamelists")
            .join(profile.system_folder(s, overrides))
            .join("gamelist.xml");
        if gl.exists() {
            entries.extend(gamelist_entries(
                &gl,
                &profile.rom_dir(s, overrides).to_lowercase(),
            ));
        }
    }
    if !entries.is_empty() {
        report.sources.push("gamelist.xml".into());
    }
    match onion_entries(card) {
        Ok(e) if !e.is_empty() => {
            report.sources.push("Onion play activity".into());
            entries.extend(e);
        }
        Ok(_) => {}
        Err(e) => report
            .notes
            .push(format!("Couldn't read Onion's play history: {e}")),
    }
    report.found = entries.len();
    if entries.is_empty() {
        return Ok(report);
    }

    // Index library games by (system, file stem); multi-disc playlists and
    // ScummVM / DOS folders by their base name too.
    let roms = lib.active()?;
    let mut index: HashMap<(String, String), Vec<&Rom>> = HashMap::new();
    for r in &roms {
        let stem = r.stem.to_lowercase();
        index
            .entry((r.system.clone(), stem.clone()))
            .or_default()
            .push(r);
        let base = naming::strip_disc(&r.stem).to_lowercase();
        if base != stem {
            index.entry((r.system.clone(), base)).or_default().push(r);
        }
    }
    lib.begin()?;
    for e in &entries {
        let Some(sys) = folder_system.get(&e.folder.to_lowercase()).copied() else {
            continue;
        };
        let first = e.file.split('/').next().unwrap_or(&e.file);
        let stem = split_name(Path::new(first)).0;
        let stem = stem.trim_start_matches('.').to_lowercase();
        if let Some(list) = index.get(&(sys.to_string(), stem)) {
            for r in list {
                if lib.record_play(r.id, e.count, e.last)? {
                    report.updated += 1;
                }
            }
        }
    }
    lib.commit()?;
    rep.log(format!(
        "Play history: {} game(s) on the card, {} updated in the library.",
        report.found, report.updated
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn es_dates() {
        assert_eq!(parse_es_time("19700101T000000"), None);
        assert_eq!(parse_es_time("20000101T000000"), Some(946_684_800.0));
        assert_eq!(parse_es_time("20240229T120000"), Some(1_709_208_000.0));
        assert_eq!(parse_es_time("not-a-date-time"), None);
    }
}
