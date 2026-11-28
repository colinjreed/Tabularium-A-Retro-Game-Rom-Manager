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
    /// Content CRC and size, once worked out (see `import`), so files are
    /// only read once.
    pub crc: Option<(String, u64)>,
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
        return match kind {
            Some(1) => strong("gamecube"),
            Some(2) => strong("wii"),
            _ => None,
        };
    }
    if at(&head, 0, &[0x01, 0xC0, 0x0B, 0xB1]) {
        let sub = head
            .get(4..8)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        return match sub {
            Some(0) => strong("gamecube"),
            Some(1) => strong("wii"),
            _ => None,
        };
    }
    if ext == "ciso" && at(&head, 0, b"CISO") {
        return strong("gamecube");
    }
    // GameCube / Wii disc header magic.
    if at(&head, 0x1C, &[0xC2, 0x33, 0x9F, 0x3D]) {
        return strong("gamecube");
    }
    if at(&head, 0x18, &[0x5D, 0x1C, 0x9E, 0xA3]) {
        return strong("wii");
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
        crc: None,
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
    for (suffix, system) in systems::FOLDER_SUFFIXES {
        if name.len() > suffix.len() + 1 && name.ends_with(&format!(".{suffix}")) {
            return systems::get(system);
        }
    }
    // Inside a DOS / ScummVM folder: a game, whatever it's called ("DOS/Doom"
    // is the game Doom, not the Doom-engine system), unless it's a folder of
    // games ("DOS/Sierra/..."), which is searched instead. A grouping folder's
    // contents still belong to that system, so look a few levels up.
    let parent_system = dir
        .ancestors()
        .skip(1)
        .take(4)
        .filter_map(|p| p.file_name())
        // The nearest system-named folder decides ("PC/roms/nes/Hacks" is NES).
        .find_map(|n| systems::from_folder_name(&n.to_string_lossy()))
        .filter(|s| systems::folder_games(s.id));
    if parent_system.is_some() && is_grouping_folder(dir) {
        return None;
    }
    if let Some(s) = parent_system {
        if systems::from_folder_name(&name)
            .is_none_or(|own| !systems::folder_games(own.id) || own.id == s.id)
        {
            return Some(s);
        }
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
    None
}

/// A folder that only holds other folders (at least two) and no files of its
/// own, like "DOS/Sierra" or "ScummVM/LucasArts": a group of games, not one.
fn is_grouping_folder(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut subdirs = 0;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if name.starts_with('.') {
            continue;
        }
        let Ok(t) = e.file_type() else { continue };
        if t.is_dir() {
            // A PS3 game folder holds a PS3_GAME folder and maybe nothing else.
            if name == "ps3_game" || name == "usrdir" {
                return false;
            }
            subdirs += 1;
        } else if !matches!(
            ext_of(&e.path()).as_str(),
            "txt"
                | "nfo"
                | "md"
                | "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "ini"
                | "db"
                | "url"
                | "pdf"
                | "htm"
                | "html"
                | "xml"
        ) {
            return false;
        }
    }
    subdirs >= 2
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub sets: Vec<RomSet>,
    pub skipped: Vec<PathBuf>,
    /// Files macOS had to download from iCloud before they could be read.
    pub from_cloud: usize,
}

/// A file macOS keeps only in iCloud (or another cloud drive): reading it
/// makes macOS download the whole file first, which can take a while.
/// Checking doesn't download anything. Always false on other systems.
pub fn is_cloud_placeholder(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        const SF_DATALESS: u32 = 0x4000_0000;
        if let Ok(m) = std::fs::symlink_metadata(path) {
            return m.st_flags() & SF_DATALESS != 0;
        }
    }
    let _ = path;
    false
}

/// The cloud service whose synced folder `path` is in, if any.
pub fn cloud_folder(path: &Path) -> Option<&'static str> {
    let s = path.to_string_lossy().replace('\\', "/");
    let lower = s.to_lowercase();
    if lower.contains("/library/mobile documents/") || lower.ends_with("/library/mobile documents")
    {
        return Some("iCloud Drive");
    }
    for (needle, name) in [
        ("/library/cloudstorage/dropbox", "Dropbox"),
        ("/library/cloudstorage/onedrive", "OneDrive"),
        ("/library/cloudstorage/googledrive", "Google Drive"),
        ("/library/cloudstorage/", "a cloud drive"),
        ("/dropbox/", "Dropbox"),
        ("/onedrive", "OneDrive"),
        ("/google drive/", "Google Drive"),
        ("/my drive/", "Google Drive"),
    ] {
        if lower.contains(needle) {
            return Some(name);
        }
    }
    None
}

/// Find every ROM under the given files/folders.
///
/// Files referenced by a .cue/.gdi/.ccd are folded into that set. Original
/// .m3u playlists are dropped; they are regenerated on export.
pub fn scan(
    paths: &[PathBuf],
    forced: Option<&'static System>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> ScanResult {
    scan_with_progress(paths, forced, cancel, &|_, _, _| {})
}

/// [`scan`], reporting (done, total, what) as it goes. `total` is 0 while
/// still walking the folders.
pub fn scan_with_progress(
    paths: &[PathBuf],
    forced: Option<&'static System>,
    cancel: &(dyn Fn() -> bool + Sync),
    progress: &(dyn Fn(usize, usize, &str) + Sync),
) -> ScanResult {
    use rayon::prelude::*;
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
                if candidates.len().is_multiple_of(500) {
                    progress(
                        0,
                        0,
                        &format!("Scanning folders… {} files found", candidates.len()),
                    );
                    if cancel() {
                        return ScanResult::default();
                    }
                }
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
            crc: None,
            rename: None,
            main: dir,
            companions: vec![],
            system,
            reason: "game folder".into(),
        });
    }
    let mut todo = vec![];
    for (p, root) in candidates {
        if claimed.contains(&p) {
            continue;
        }
        if IGNORED_EXTS.contains(&ext_of(&p).as_str()) {
            out.skipped.push(p);
            continue;
        }
        todo.push((p, root));
    }
    // Identifying a file means opening it (zip directories, disc headers),
    // so do several at once; slow drives and network shares benefit most.
    let total = todo.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let cloud = std::sync::atomic::AtomicUsize::new(0);
    let detected: Vec<Option<RomSet>> = todo
        .par_iter()
        .map(|(p, root)| {
            if cancel() {
                return None;
            }
            // Name the file before opening it, so a slow one is visible.
            let n = done.load(std::sync::atomic::Ordering::Relaxed);
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if is_cloud_placeholder(p) {
                cloud.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                progress(n, total, &format!("Downloading from iCloud: {name}"));
            } else {
                progress(n, total, &format!("Identifying {name}"));
            }
            let rs = detect(p, Some(root), forced);
            done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(rs)
        })
        .collect();
    if cancel() {
        return ScanResult::default();
    }
    out.sets.extend(detected.into_iter().flatten());
    out.from_cloud = cloud.into_inner();
    out
}
