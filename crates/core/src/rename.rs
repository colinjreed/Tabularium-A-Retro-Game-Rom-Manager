//! Give ROMs proper names: official No-Intro names found by checksum, or at
//! least without "003 - " style numbering.

use crate::dats::{self, Dats};
use crate::detect::{self, RomSet};
use crate::hashing::content_crc;
use crate::library::{Library, Rom};
use crate::progress::Reporter;
use crate::{systems, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

fn eligible(system: &str) -> bool {
    systems::get(system).is_some_and(|s| !s.disc_based)
        && !systems::is_arcade(system)
        && !systems::folder_games(system)
}

/// During import: set `rename` (and fix `system`) on detected sets.
pub fn plan_import(
    sets: &mut [RomSet],
    mut dats: Option<&mut Dats>,
    strip_numbers: bool,
    rep: &dyn Reporter,
) {
    let mut by_dir: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for rs in sets.iter() {
        if let Some(dir) = rs.main.parent() {
            let stem = rs
                .main
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            by_dir.entry(dir.to_path_buf()).or_default().push(stem);
        }
    }
    let numbered_dirs: Vec<&PathBuf> = by_dir
        .iter()
        .filter(|(_, v)| dats::mostly_numbered(v.iter().map(|s| s.as_str())))
        .map(|(k, _)| k)
        .collect();
    let total = sets.len();
    for (i, rs) in sets.iter_mut().enumerate() {
        if rep.cancelled() {
            return;
        }
        if i % 50 == 0 {
            rep.progress(i, total, "Looking up official names…");
        }
        let Some(sys) = rs.system else { continue };
        if !eligible(sys.id) || !rs.companions.is_empty() || !rs.main.is_file() {
            continue;
        }
        if let Some(d) = dats.as_deref_mut() {
            if !d.is_loaded(sys.id) {
                rep.progress(
                    i,
                    total,
                    &format!("Getting the official name list for {}…", sys.display_name()),
                );
            }
            let crc = match &rs.crc {
                Some((c, _)) => Some(c.clone()),
                None => content_crc(&rs.main).ok().map(|c| c.0),
            };
            if let Some(crc) = crc {
                let crcs = dats::candidate_crcs(&rs.main, &crc);
                let hint = detect::folder_hint(&rs.main, None).map(|h| h.id);
                if let Some((sys_id, name)) = d.identify(&crcs, sys.id, hint) {
                    rs.system = systems::get(&sys_id);
                    rs.rename = Some(name);
                    continue;
                }
            }
        }
        if strip_numbers
            && rs
                .main
                .parent()
                .is_some_and(|p| numbered_dirs.iter().any(|d| d.as_path() == p))
        {
            let stem = rs
                .main
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            rs.rename = dats::strip_numbering(&stem);
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameChange {
    pub rom: Rom,
    pub new_system: String,
    pub new_stem: String,
    /// "official name" or "numbering removed".
    pub why: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct FixReport {
    pub changes: Vec<NameChange>,
    pub applied: usize,
    pub conflicts: Vec<String>,
    pub checked: usize,
}

/// Work out (and optionally apply) better names for ROMs already in the library.
pub fn fix_library(
    lib: &Library,
    dats: Option<&mut Dats>,
    strip_numbers: bool,
    apply: bool,
    rep: &dyn Reporter,
) -> Result<FixReport> {
    let roms: Vec<Rom> = lib
        .active()?
        .into_iter()
        .filter(|r| eligible(&r.system) && r.files.len() == 1)
        .collect();
    let mut report = FixReport {
        checked: roms.len(),
        ..Default::default()
    };
    let mut unmatched: BTreeMap<String, Vec<&Rom>> = BTreeMap::new();
    let mut dats = dats;
    for (i, r) in roms.iter().enumerate() {
        if rep.cancelled() {
            break;
        }
        if i % 25 == 0 {
            rep.progress(i, roms.len(), &r.stem);
        }
        let path = lib.abspath(&r.path);
        let found = match (dats.as_deref_mut(), &r.crc) {
            (Some(d), Some(crc)) => {
                let crcs = dats::candidate_crcs(&path, crc);
                let hint = detect::folder_hint(Path::new(r.source_path().unwrap_or("")), None)
                    .map(|h| h.id);
                d.identify(&crcs, &r.system, hint)
            }
            _ => None,
        };
        match found {
            Some((sys, name)) => {
                let name = crate::naming::safe_filename(&name);
                if name != r.stem || sys != r.system {
                    report.changes.push(NameChange {
                        rom: r.clone(),
                        new_system: sys,
                        new_stem: name,
                        why: "official name",
                    });
                }
            }
            None => unmatched.entry(r.system.clone()).or_default().push(r),
        }
    }
    if strip_numbers {
        for (system, list) in &unmatched {
            // Judge by the whole system: matched ROMs count as evidence too.
            let all = roms
                .iter()
                .filter(|r| &r.system == system)
                .map(|r| r.stem.as_str());
            if dats::mostly_numbered(all) {
                for r in list {
                    if let Some(s) = dats::strip_numbering(&r.stem) {
                        report.changes.push(NameChange {
                            rom: (*r).clone(),
                            new_system: r.system.clone(),
                            new_stem: s,
                            why: "numbering removed",
                        });
                    }
                }
            }
        }
    }
    if apply {
        lib.begin()?;
        let n = report.changes.len();
        for (i, c) in report.changes.iter().enumerate() {
            rep.progress(i, n, &c.new_stem);
            match lib.relocate_rom(&c.rom, &c.new_system, &c.new_stem) {
                Ok(true) => report.applied += 1,
                Ok(false) => report.conflicts.push(format!(
                    "{} -> {} (name already taken)",
                    c.rom.stem, c.new_stem
                )),
                Err(e) => report.conflicts.push(format!("{}: {e}", c.rom.stem)),
            }
        }
        lib.commit()?;
    }
    rep.progress(1, 1, "Done");
    Ok(report)
}
