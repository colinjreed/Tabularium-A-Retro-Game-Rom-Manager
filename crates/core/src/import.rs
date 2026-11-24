//! Sort ROMs from arbitrary folders into the library.

use crate::detect::{scan_with_progress, RomSet};
use crate::library::{AddOutcome, Library};
use crate::progress::Reporter;
use crate::systems::System;
use crate::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct ImportOptions {
    /// Network access for arcade / ScummVM name lists (None = offline).
    pub fetch: Option<crate::http::Fetcher>,
    /// Rename to official No-Intro names (needs `fetch`) and fix the system
    /// when the checksum says so.
    pub official_names: bool,
    /// Remove "003 - " style numbering from names that aren't in the lists.
    pub strip_numbers: bool,
    pub move_files: bool,
    pub dry_run: bool,
    /// Treat every file as this system.
    pub forced_system: Option<&'static System>,
    /// Copy files with an unknown system into LIBRARY/unsorted.
    pub keep_unknown: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub added: BTreeMap<String, usize>,
    pub duplicates: Vec<(PathBuf, String)>,
    pub conflicts: Vec<(PathBuf, String)>,
    pub errors: Vec<(PathBuf, String)>,
    pub unknown: Vec<RomSet>,
    pub ignored: usize,
    pub cancelled: bool,
}

impl ImportReport {
    pub fn added_total(&self) -> usize {
        self.added.values().sum()
    }
}

pub fn import(
    lib: &Library,
    sources: &[PathBuf],
    opts: &ImportOptions,
    rep: &dyn Reporter,
) -> Result<ImportReport> {
    let mut out = ImportReport::default();
    rep.progress(0, 0, "Scanning folders…");
    let mut found = scan_with_progress(
        sources,
        opts.forced_system,
        &|| rep.cancelled(),
        &|d, t, what| rep.progress(d, t, what),
    );
    recognise_arcade_zips(lib, &mut found.sets, opts, rep);
    if checksum_all(&mut found.sets, rep) {
        out.cancelled = true;
        return Ok(out);
    }
    if opts.official_names || opts.strip_numbers {
        let mut dats = match (&opts.fetch, opts.official_names) {
            (Some(f), true) => Some(crate::dats::Dats::new(
                crate::metadata::cache_dir(lib),
                f.clone(),
            )),
            _ => None,
        };
        crate::rename::plan_import(&mut found.sets, dats.as_mut(), opts.strip_numbers, rep);
    }
    out.ignored = found.skipped.len();
    let total = found.sets.len();
    lib.begin()?;
    for (i, rs) in found.sets.into_iter().enumerate() {
        if rep.cancelled() {
            out.cancelled = true;
            break;
        }
        let name = rs
            .main
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        rep.progress(i, total, &name);
        let Some(sys) = rs.system else {
            if opts.keep_unknown && !opts.dry_run {
                if let Err(e) = lib.add_unsorted(&rs, opts.move_files) {
                    out.errors.push((rs.main.clone(), e.to_string()));
                }
            }
            out.unknown.push(rs);
            continue;
        };
        match lib.add(&rs, opts.move_files, opts.dry_run) {
            Ok(AddOutcome::Added(_)) => *out.added.entry(sys.id.to_string()).or_default() += 1,
            Ok(AddOutcome::Duplicate(of)) => out.duplicates.push((rs.main, of)),
            Ok(AddOutcome::Conflict(why)) => {
                rep.log(format!("Conflict: {name}: {why}"));
                out.conflicts.push((rs.main, why))
            }
            Err(e) => {
                rep.log(format!("Error: {name}: {e}"));
                out.errors.push((rs.main, e.to_string()))
            }
        }
        if i % 200 == 199 {
            lib.commit()?;
            lib.begin()?;
        }
    }
    lib.commit()?;
    if let (Some(fetch), false) = (&opts.fetch, opts.dry_run) {
        crate::metadata::enrich(lib, fetch, rep)?;
    }
    rep.progress(total, total, "Done");
    Ok(out)
}

/// Work out every identified file's checksum (for duplicate checks and
/// official names) in one pass, so nothing is read twice. Small files are
/// read several at a time; big ones one after another, which keeps a hard
/// drive from seeking back and forth. Returns true if cancelled.
fn checksum_all(sets: &mut [RomSet], rep: &dyn Reporter) -> bool {
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    const BIG: u64 = 64 << 20;
    let is_big = |rs: &RomSet| {
        rs.main.is_dir()
            || (crate::detect::ext_of(&rs.main) != "zip"
                && std::fs::metadata(&rs.main).map(|m| m.len()).unwrap_or(0) >= BIG)
    };
    let total = sets.iter().filter(|rs| rs.system.is_some()).count();
    let done = AtomicUsize::new(0);
    let one = |rs: &mut RomSet| {
        if rep.cancelled() {
            return;
        }
        rs.crc = crate::hashing::content_crc(&rs.main).ok();
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        let name = rs.main.file_name().unwrap_or_default().to_string_lossy();
        rep.progress(n, total, &format!("Checking {name}"));
    };
    let (big, small): (Vec<&mut RomSet>, Vec<&mut RomSet>) = sets
        .iter_mut()
        .filter(|rs| rs.system.is_some())
        .partition(|rs| is_big(rs));
    small.into_par_iter().for_each(one);
    for rs in big {
        one(rs);
    }
    rep.cancelled()
}

/// Zips nothing else could place: if the file name is a known MAME/FBNeo
/// set, it's arcade. The folder name picks MAME / CPS1-3, else FBNeo.
fn recognise_arcade_zips(
    lib: &Library,
    sets: &mut [RomSet],
    opts: &ImportOptions,
    rep: &dyn Reporter,
) {
    let unknown_zip = |rs: &RomSet| rs.system.is_none() && crate::detect::ext_of(&rs.main) == "zip";
    if !sets.iter().any(unknown_zip) {
        return;
    }
    let Some(fetch) = &opts.fetch else { return };
    rep.progress(0, 0, "Checking the arcade game list…");
    let Some(names) = crate::metadata::arcade_names(&crate::metadata::cache_dir(lib), fetch) else {
        return;
    };
    for rs in sets.iter_mut().filter(|rs| unknown_zip(rs)) {
        let stem = rs
            .main
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if names.contains_key(&stem) {
            let hint = crate::detect::folder_hint(&rs.main, None)
                .filter(|s| crate::systems::is_arcade(s.id));
            rs.system = hint.or(crate::systems::get("arcade"));
            rs.reason = "arcade game list".into();
        }
    }
}
