//! Work out which system a ROM file belongs to.
//!
//! Evidence, in order:
//! 1. a unique extension (.gba, .sfc, ...)
//! 2. the contents of .zip archives
//! 3. folder names anywhere in the path ("PS1", "Game Boy (GB)", "megadrive", ...)
//! 4. file headers for .bin / .iso / .cue (Genesis, 32X, PS1, PSP, Sega CD, Saturn)

use crate::systems::{self, System, AMBIGUOUS_EXTS};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IGNORED_EXTS: &[&str] = &[
    "txt", "nfo", "jpg", "jpeg", "png", "gif", "bmp", "xml", "dat", "db", "ini", "cfg", "srm",
    "sav", "state", "sfv", "md5", "sha1", "url", "pdf", "html", "htm", "json", "log", "lnk", "exe",
    "dll", "auto", "ips", "bps", "ups", "xdelta", "rtc", "mcr", "mcd", "part",
];

/// A playable unit: one file, or a .cue/.gdi plus the files it references.
#[derive(Debug, Clone)]
pub struct RomSet {
    pub main: PathBuf,
    pub companions: Vec<PathBuf>,
    pub system: Option<&'static System>,
    pub reason: String,
}

impl RomSet {
    pub fn files(&self) -> Vec<PathBuf> {
        std::iter::once(self.main.clone())
            .chain(self.companions.iter().cloned())
            .collect()
    }
}

pub fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn read_text(path: &Path) -> String {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

fn cue_refs(path: &Path) -> Vec<PathBuf> {
    let dir = path.parent().unwrap_or(Path::new("."));
    read_text(path)
        .lines()
        .filter_map(|line| {
            let l = line.trim();
            if !l.to_ascii_uppercase().starts_with("FILE ") {
                return None;
            }
            let rest = l[5..].trim();
            let name = if let Some(stripped) = rest.strip_prefix('"') {
                stripped.split('"').next()?.to_string()
            } else {
                let mut parts: Vec<&str> = rest.split_whitespace().collect();
                parts.pop(); // file type (BINARY, WAVE, ...)
                parts.join(" ")
            };
            (!name.is_empty()).then(|| dir.join(name))
        })
        .collect()
}

fn gdi_refs(path: &Path) -> Vec<PathBuf> {
    let dir = path.parent().unwrap_or(Path::new("."));
    read_text(path)
        .lines()
        .skip(1)
        .filter_map(|line| {
            let l = line.trim();
            let mut fields = l.splitn(5, char::is_whitespace);
            for _ in 0..4 {
                fields.next()?;
            }
            let rest = fields.next()?.trim();
            let name = if let Some(s) = rest.strip_prefix('"') {
                s.split('"').next()?.to_string()
            } else {
                rest.split_whitespace().next()?.to_string()
            };
            Some(dir.join(name))
        })
        .collect()
}

/// Files a .cue/.gdi/.ccd sheet points at.
pub fn references(path: &Path) -> Vec<PathBuf> {
    match ext_of(path).as_str() {
        "cue" => cue_refs(path),
        "gdi" => gdi_refs(path),
        "ccd" => ["img", "sub"]
            .iter()
            .map(|e| path.with_extension(e))
            .filter(|p| p.exists())
            .collect(),
        _ => vec![],
    }
}

fn read_at(path: &Path, offset: u64, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    let Ok(mut f) = File::open(path) else {
        return vec![];
    };
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return vec![];
    }
    let mut n = 0;
    while n < len {
        match f.read(&mut buf[n..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    buf.truncate(n);
    buf
}

fn at(buf: &[u8], off: usize, sig: &[u8]) -> bool {
    buf.len() >= off + sig.len() && &buf[off..off + sig.len()] == sig
}

/// Look inside a cartridge dump or disc image for a known signature.
pub fn sniff(path: &Path) -> Option<&'static System> {
    let ext = ext_of(path);
    if ext == "cue" {
        let refs = cue_refs(path);
        return refs.first().filter(|p| p.exists()).and_then(|p| sniff(p));
    }
    if !matches!(ext.as_str(), "bin" | "iso" | "img") {
        return None;
    }
    let head = read_at(path, 0, 0x200);
    if at(&head, 0x100, b"SEGA 32X") {
        return systems::get("sega32x");
    }
    if at(&head, 0x100, b"SEGA") || at(&head, 0x101, b"SEGA") {
        return systems::get("genesis");
    }
    for off in [0, 0x10] {
        if at(&head, off, b"SEGADISCSYSTEM") {
            return systems::get("segacd");
        }
        if at(&head, off, b"SEGA SEGASATURN") {
            return systems::get("saturn");
        }
    }
    // ISO9660 primary volume descriptor: cooked (2048) or raw (2352 + header)
    for pvd in [0x8000u64, 16 * 2352 + 24, 16 * 2352 + 16] {
        let block = read_at(path, pvd, 64);
        if at(&block, 1, b"CD001") {
            if at(&block, 8, b"PSP GAME") {
                return systems::get("psp");
            }
            if at(&block, 8, b"PLAYSTATION") {
                return systems::get("psx");
            }
        }
    }
    if ext == "bin" {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if [2048, 4096, 8192, 12288, 16384, 32768, 65536].contains(&size) {
            return systems::get("atari2600");
        }
    }
    None
}

fn zip_system(path: &Path) -> (Option<&'static System>, &'static str) {
    let Ok(f) = File::open(path) else {
        return (None, "unreadable zip");
    };
    let Ok(mut z) = zip::ZipArchive::new(f) else {
        return (None, "unreadable zip");
    };
    let mut names = vec![];
    for i in 0..z.len() {
        if let Ok(f) = z.by_index_raw(i) {
            if !f.is_dir() {
                names.push(f.name().to_string());
            }
        }
    }
    let found: HashSet<&str> = names
        .iter()
        .filter_map(|n| systems::by_unique_ext(&ext_of(Path::new(n))))
        .map(|s| s.id)
        .collect();
    if found.len() == 1 {
        return (
            systems::get(found.into_iter().next().unwrap()),
            "zip contents",
        );
    }
    if found.is_empty() && names.len() > 1 {
        return (None, "zip with many non-ROM files (arcade?)");
    }
    (None, "zip contents ambiguous")
}

pub fn folder_hint(path: &Path, root: Option<&Path>) -> Option<&'static System> {
    let parent = path.parent()?;
    let rel = root
        .and_then(|r| parent.strip_prefix(r).ok())
        .unwrap_or(parent);
    rel.components()
        .rev()
        .filter_map(|c| c.as_os_str().to_str())
        .find_map(systems::from_folder_name)
}

pub fn detect(path: &Path, root: Option<&Path>, forced: Option<&'static System>) -> RomSet {
    let ext = ext_of(path);
    let companions = references(path)
        .into_iter()
        .filter(|p| p.exists() && p != path)
        .collect();
    let mut rs = RomSet {
        main: path.to_path_buf(),
        companions,
        system: None,
        reason: String::new(),
    };
    let mut set = |s: Option<&'static System>, why: &str| {
        rs.system = s;
        rs.reason = why.to_string();
    };
    if forced.is_some() {
        set(forced, "chosen");
        return rs;
    }
    if !AMBIGUOUS_EXTS.contains(&ext.as_str()) {
        if let Some(s) = systems::by_unique_ext(&ext) {
            set(Some(s), "extension");
            return rs;
        }
    }
    let hint = folder_hint(path, root);
    if ext == "zip" {
        let (s, why) = zip_system(path);
        if s.is_some() {
            set(s, why);
        } else if let Some(h) = hint.filter(|h| h.accepts(&ext)) {
            set(Some(h), "folder name");
        } else {
            set(None, why);
        }
        return rs;
    }
    if let Some(h) = hint.filter(|h| h.accepts(&ext)) {
        set(Some(h), "folder name");
        return rs;
    }
    if let Some(s) = sniff(path) {
        set(Some(s), "file header");
        return rs;
    }
    let by_ext = match ext.as_str() {
        "gdi" | "cdi" => systems::get("dreamcast"),
        "cso" => systems::get("psp"),
        "pbp" => systems::get("psx"),
        _ => None,
    };
    if by_ext.is_some() {
        set(by_ext, "extension");
        return rs;
    }
    let why = if AMBIGUOUS_EXTS.contains(&ext.as_str()) {
        "shared extension and no folder name to go on"
    } else {
        "unknown file type"
    };
    set(None, why);
    rs
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub sets: Vec<RomSet>,
    pub skipped: Vec<PathBuf>,
}

/// Find every ROM under the given files/folders.
///
/// Files referenced by a .cue/.gdi/.ccd are folded into that set. Original
/// .m3u playlists are dropped; they are regenerated on export.
pub fn scan(
    paths: &[PathBuf],
    forced: Option<&'static System>,
    cancel: &dyn Fn() -> bool,
) -> ScanResult {
    let mut candidates: Vec<(PathBuf, PathBuf)> = vec![];
    for root in paths {
        let root = dunce::canonicalize(root).unwrap_or_else(|_| root.clone());
        if root.is_file() {
            let parent = root.parent().unwrap_or(Path::new("/")).to_path_buf();
            candidates.push((root, parent));
            continue;
        }
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
            .flatten()
        {
            if entry.file_type().is_file() {
                candidates.push((entry.path().to_path_buf(), root.clone()));
            }
        }
        if cancel() {
            return ScanResult::default();
        }
    }
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    for (p, _) in &candidates {
        match ext_of(p).as_str() {
            "cue" | "gdi" | "ccd" => claimed.extend(references(p)),
            "m3u" => {
                claimed.insert(p.clone());
            }
            _ => {}
        }
    }
    let mut out = ScanResult::default();
    for (p, root) in candidates {
        if claimed.contains(&p) {
            continue;
        }
        if IGNORED_EXTS.contains(&ext_of(&p).as_str()) {
            out.skipped.push(p);
            continue;
        }
        out.sets.push(detect(&p, Some(&root), forced));
    }
    out
}
