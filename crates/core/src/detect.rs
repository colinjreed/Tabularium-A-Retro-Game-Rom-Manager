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
    /// New file name stem to use in the library (official name), if any.
    pub rename: Option<String>,
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

/// A minimal ISO9660 reader over cooked (2048-byte) or raw (2352-byte) images.
struct Iso<'a> {
    path: &'a Path,
    sector: u64,
    offset: u64,
}

impl<'a> Iso<'a> {
    fn open(path: &'a Path) -> Option<Iso<'a>> {
        for (sector, offset) in [(2048u64, 0u64), (2352, 24), (2352, 16)] {
            if at(&read_at(path, 16 * sector + offset, 8), 1, b"CD001") {
                return Some(Iso {
                    path,
                    sector,
                    offset,
                });
            }
        }
        None
    }

    fn read(&self, lba: u64, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        let mut lba = lba;
        while out.len() < len {
            let want = (len - out.len()).min(2048);
            let chunk = read_at(self.path, lba * self.sector + self.offset, want);
            if chunk.is_empty() {
                break;
            }
            out.extend_from_slice(&chunk);
            lba += 1;
        }
        out
    }

    fn pvd(&self) -> Vec<u8> {
        self.read(16, 2048)
    }

    /// Contents of a file in the root directory (first 4 KB).
    fn root_file(&self, wanted: &str) -> Option<Vec<u8>> {
        let pvd = self.pvd();
        let rec = pvd.get(156..190)?;
        let lba = u32::from_le_bytes(rec[2..6].try_into().ok()?) as u64;
        let size = (u32::from_le_bytes(rec[10..14].try_into().ok()?) as usize).min(64 * 1024);
        let dir = self.read(lba, size);
        let mut i = 0;
        while i < dir.len() {
            let len = dir[i] as usize;
            if len == 0 {
                i = (i / 2048 + 1) * 2048; // records don't cross sectors
                continue;
            }
            let r = dir.get(i..i + len)?;
            let name_len = *r.get(32)? as usize;
            let name = String::from_utf8_lossy(r.get(33..33 + name_len)?);
            let name = name.split(';').next().unwrap_or("").to_ascii_uppercase();
            if name == wanted {
                let flba = u32::from_le_bytes(r[2..6].try_into().ok()?) as u64;
                let fsize = (u32::from_le_bytes(r[10..14].try_into().ok()?) as usize).min(4096);
                return Some(self.read(flba, fsize));
            }
            i += len;
        }
        None
    }
}

/// How sure a header match is. Strong matches beat folder names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Strong,
    Weak,
}

/// Look inside a cartridge dump or disc image for a known signature.
pub fn sniff(path: &Path) -> Option<(&'static System, Confidence)> {
    use Confidence::*;
    let ext = ext_of(path);
    if ext == "cue" {
        let refs = cue_refs(path);
        return refs.first().filter(|p| p.exists()).and_then(|p| sniff(p));
    }
    if !matches!(
        ext.as_str(),
        "bin" | "iso" | "img" | "rvz" | "wia" | "gcz" | "ciso" | "xiso" | "gcm"
    ) {
        return None;
    }
    let strong = |id| systems::get(id).map(|s| (s, Strong));
    let head = read_at(path, 0, 0x200);
    // Dolphin's compressed formats.
    if at(&head, 0, b"RVZ\x01") || at(&head, 0, b"WIA\x01") {
        let kind = head
            .get(0x48..0x4C)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        return if kind == Some(1) {
            strong("gamecube")
        } else {
            None
        }; // 2 = Wii
    }
    if at(&head, 0, &[0x01, 0xC0, 0x0B, 0xB1]) {
        let sub = head
            .get(4..8)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        return if sub == Some(0) {
            strong("gamecube")
        } else {
            None
        };
    }
    if ext == "ciso" && at(&head, 0, b"CISO") {
        return strong("gamecube");
    }
    // GameCube disc header magic (0x5D1C9EA3 at 0x18 would be Wii).
    if at(&head, 0x1C, &[0xC2, 0x33, 0x9F, 0x3D]) {
        return strong("gamecube");
    }
    if at(&head, 0x100, b"SEGA 32X") {
        return strong("sega32x");
    }
    if at(&head, 0x100, b"SEGA") || at(&head, 0x101, b"SEGA") {
        return strong("genesis");
    }
    for off in [0, 0x10] {
        if at(&head, off, b"SEGADISCSYSTEM") {
            return strong("segacd");
        }
        if at(&head, off, b"SEGA SEGASATURN") {
            return strong("saturn");
        }
    }
    // Xbox XISO (or the game partition inside a full Redump image).
    for off in [0x10000u64, 0x1830_0000 + 0x10000] {
        if at(&read_at(path, off, 20), 0, b"MICROSOFT*XBOX*MEDIA") {
            return strong("xbox");
        }
    }
    if let Some(iso) = Iso::open(path) {
        let pvd = iso.pvd();
        if at(&pvd, 8, b"PSP GAME") {
            return strong("psp");
        }
        if at(&pvd, 8, b"PLAYSTATION") {
            if let Some(cnf) = iso.root_file("SYSTEM.CNF") {
                let text = String::from_utf8_lossy(&cnf).to_ascii_uppercase();
                return strong(if text.contains("BOOT2") { "ps2" } else { "psx" });
            }
            // No SYSTEM.CNF found: DVD-sized images are PS2.
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            return strong(if size > 900 * 1024 * 1024 {
                "ps2"
            } else {
                "psx"
            });
        }
    }
    if ext == "bin" {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if [2048, 4096, 8192, 12288, 16384, 32768, 65536].contains(&size) {
            return systems::get("atari2600").map(|s| (s, Weak));
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
        rename: None,
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
    let hint = folder_hint(path, root);
    if !AMBIGUOUS_EXTS.contains(&ext.as_str()) {
        if let Some(s) = systems::by_unique_ext(&ext) {
            // A folder for a system that also uses this extension wins:
            // Game Gear games dumped as .sms belong in Game Gear.
            match hint.filter(|h| h.id != s.id && h.extra_exts.contains(&ext.as_str())) {
                Some(h) => set(Some(h), "folder name"),
                None => set(Some(s), "extension"),
            }
            return rs;
        }
    }
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
    let sniffed = sniff(path);
    if let Some((s, Confidence::Strong)) = sniffed {
        set(Some(s), "file header");
        return rs;
    }
    if let Some(h) = hint.filter(|h| h.accepts(&ext)) {
        set(Some(h), "folder name");
        return rs;
    }
    if let Some((s, _)) = sniffed {
        set(Some(s), "file header");
        return rs;
    }
    let by_ext = match ext.as_str() {
        "gdi" | "cdi" => systems::get("dreamcast"),
        "rvz" | "wia" => None, // Wii discs aren't supported
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

/// Is this directory a whole game (DOS or ScummVM)?
///
/// Recognised by a `.scummvm`/`.svm`/`.pc`/`.dos` folder suffix, a ScummVM
/// pointer file inside or beside it, or by sitting directly inside a folder
/// named after DOS or ScummVM.
pub fn folder_game_system(dir: &Path) -> Option<&'static System> {
    let name = dir.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".scummvm") || name.ends_with(".svm") {
        return systems::get("scummvm");
    }
    if name.ends_with(".pc") || name.ends_with(".dos") {
        return systems::get("dos");
    }
    // A folder that is itself a system folder ("DOS", "scummvm") isn't a game.
    if systems::from_folder_name(&name).is_some() {
        return None;
    }
    let has_pointer = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && matches!(ext_of(&e.path()).as_str(), "scummvm" | "svm")
        });
    let sibling = ["scummvm", "svm"].iter().any(|e| {
        dir.with_file_name(format!(
            "{}.{e}",
            dir.file_name().unwrap().to_string_lossy()
        ))
        .is_file()
    });
    if has_pointer || sibling {
        return systems::get("scummvm");
    }
    let parent = dir.parent()?.file_name()?.to_string_lossy().to_string();
    systems::from_folder_name(&parent).filter(|s| systems::folder_games(s.id))
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
    let mut folder_games: Vec<(PathBuf, &'static System)> = vec![];
    for root in paths {
        let root = dunce::canonicalize(root).unwrap_or_else(|_| root.clone());
        if root.is_file() {
            let parent = root.parent().unwrap_or(Path::new("/")).to_path_buf();
            candidates.push((root, parent));
            continue;
        }
        let mut it = WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'));
        while let Some(entry) = it.next() {
            let Ok(entry) = entry else { continue };
            if entry.file_type().is_dir() && entry.depth() > 0 {
                if let Some(sys) = folder_game_system(entry.path()) {
                    folder_games.push((entry.path().to_path_buf(), sys));
                    it.skip_current_dir();
                }
            } else if entry.file_type().is_file() {
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
    // ScummVM pointer files sitting next to their game folder belong to it.
    for (dir, sys) in &folder_games {
        if sys.id == "scummvm" {
            for e in ["scummvm", "svm"] {
                claimed.insert(dir.with_extension(e));
                if let Some(name) = dir.file_name() {
                    claimed.insert(dir.with_file_name(format!("{}.{e}", name.to_string_lossy())));
                }
            }
        }
    }
    let mut out = ScanResult::default();
    for (dir, sys) in folder_games {
        let system = forced.or(Some(sys));
        out.sets.push(RomSet {
            rename: None,
            main: dir,
            companions: vec![],
            system,
            reason: "game folder".into(),
        });
    }
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
