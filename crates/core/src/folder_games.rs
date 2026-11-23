//! Games that are whole folders: DOS and ScummVM.
//!
//! ScummVM needs to know which engine/game a folder holds. The convention
//! used by Batocera, ES-DE, ROCKNIX and the libretro core is a small text
//! file `<Game>.scummvm` containing the ScummVM game ID (e.g. `monkey2`).

use std::fs;
use std::path::{Path, PathBuf};

fn pointer_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("scummvm") || e.eq_ignore_ascii_case("svm")
                })
        })
        .collect();
    v.sort();
    v
}

fn read_id(p: &Path) -> Option<String> {
    let text = fs::read_to_string(p).ok()?;
    let id = text.lines().next()?.trim().to_string();
    (!id.is_empty() && id.len() < 64).then_some(id)
}

/// The ScummVM game ID stored in a game folder, if any.
pub fn scummvm_id(dir: &Path) -> Option<String> {
    pointer_files(dir).iter().find_map(|p| read_id(p))
}

/// Make sure a ScummVM game folder in the library has `<stem>.scummvm`
/// inside it, using a pointer file found inside or next to `source`.
pub fn ensure_scummvm_id(dest_dir: &Path, stem: &str, source: Option<&Path>) {
    let target = dest_dir.join(format!("{stem}.scummvm"));
    if target.is_file() {
        return;
    }
    let id = scummvm_id(dest_dir).or_else(|| {
        let src = source?;
        let name = src.file_name()?.to_string_lossy().to_string();
        [
            format!("{name}.scummvm"),
            format!("{name}.svm"),
            format!("{stem}.scummvm"),
            format!("{stem}.svm"),
        ]
        .iter()
        .find_map(|n| read_id(&src.with_file_name(n)))
    });
    if let Some(id) = id {
        let _ = write_scummvm_id(dest_dir, stem, &id);
    }
}

/// Write `<stem>.scummvm` containing the ID, replacing any other pointer files.
pub fn write_scummvm_id(dir: &Path, stem: &str, id: &str) -> std::io::Result<()> {
    let target = dir.join(format!("{stem}.scummvm"));
    for p in pointer_files(dir) {
        if p != target {
            fs::remove_file(p)?;
        }
    }
    fs::write(target, format!("{id}\n"))
}

/// Distinctive data files of popular ScummVM games -> game ID.
/// (File names are compared case-insensitively.)
const SIGNATURES: &[(&str, &str)] = &[
    ("monkey2.000", "monkey2"),
    ("monkey1.000", "monkey"),
    ("monkey.000", "monkey"),
    ("monkey.la0", "monkey"),
    ("atlantis.000", "atlantis"),
    ("tentacle.000", "tentacle"),
    ("samnmax.000", "samnmax"),
    ("samnmax.sm0", "samnmax"),
    ("comi.la0", "comi"),
    ("ft.la0", "ft"),
    ("dig.la0", "dig"),
    ("loom.000", "loom"),
    ("queen.1", "queen"),
    ("queen.1c", "queen"),
    ("sky.dnr", "sky"),
    ("sky.dsk", "sky"),
    ("disk1.vga", "lure"),
    ("swordres.rif", "sword1"),
    ("docks.clu", "sword2"),
    ("dreamweb.r00", "dreamweb"),
    ("malcolm.pak", "kyra3"),
    ("simon.gme", "simon1"),
    ("simon2.gme", "simon2"),
    ("feeble.exe", "feeble"),
];

/// Work out a ScummVM game ID from the folder's files or its name.
/// `known_ids` is the set of valid IDs (from libretro's ScummVM list), used to
/// accept a folder that is simply named after its ID.
pub fn detect_scummvm_id(
    dir: &Path,
    known_ids: Option<&std::collections::HashSet<String>>,
) -> Option<String> {
    if let Some(id) = scummvm_id(dir) {
        return Some(id);
    }
    let files: Vec<String> = walkdir::WalkDir::new(dir)
        .max_depth(2)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.file_name().to_string_lossy().to_ascii_lowercase())
        .collect();
    for (sig, id) in SIGNATURES {
        if files.iter().any(|f| f == sig) {
            return Some(id.to_string());
        }
    }
    let name = crate::library::split_name(dir).0.to_ascii_lowercase();
    known_ids.filter(|ids| ids.contains(&name)).map(|_| name)
}
