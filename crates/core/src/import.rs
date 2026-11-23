//! Sort ROMs from arbitrary folders into the library.

use crate::detect::{scan, RomSet};
use crate::library::{AddOutcome, Library};
use crate::progress::Reporter;
use crate::systems::System;
use crate::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
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
    let found = scan(sources, opts.forced_system, &|| rep.cancelled());
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
    rep.progress(total, total, "Done");
    Ok(out)
}
