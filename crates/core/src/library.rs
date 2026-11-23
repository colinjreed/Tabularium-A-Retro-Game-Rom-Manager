//! The master library: an organised folder of ROMs plus a SQLite index.
//!
//! ```text
//! LIBRARY/
//!   library.db                 index of every ROM, hashes and parsed tags
//!   roms/<system>/<file>       organised ROMs
//!   media/<system>/<kind>/     cached artwork (boxart, snap, title, mix, wheel)
//!   bios/                      verified BIOS files
//!   trash/<system>/<file>      ROMs removed by cleaning (restorable)
//!   unsorted/                  files whose system couldn't be identified
//!   collections/<name>.json    collection definitions
//! ```

use crate::detect::RomSet;
use crate::hashing::content_crc;
use crate::{naming, Error, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS roms (
    id INTEGER PRIMARY KEY,
    system TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    files TEXT NOT NULL,
    stem TEXT NOT NULL,
    title TEXT NOT NULL,
    key TEXT NOT NULL,
    regions TEXT NOT NULL,
    languages TEXT NOT NULL,
    revision TEXT NOT NULL,
    disc INTEGER,
    flags TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    crc TEXT,
    content_size INTEGER,
    total_size INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    status_reason TEXT,
    source TEXT,
    added REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS roms_system ON roms(system, status);
CREATE INDEX IF NOT EXISTS roms_crc ON roms(crc);
CREATE TABLE IF NOT EXISTS media (
    rom_id INTEGER NOT NULL REFERENCES roms(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    path TEXT,
    source TEXT,
    fetched REAL NOT NULL,
    PRIMARY KEY (rom_id, kind)
);
";

const SUBDIRS: &[&str] = &["roms", "media", "trash", "unsorted", "collections", "bios"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rom {
    pub id: i64,
    pub system: String,
    /// Main file, relative to the library root (always '/'-separated).
    pub path: String,
    pub files: Vec<String>,
    pub stem: String,
    pub title: String,
    pub key: String,
    pub regions: Vec<String>,
    pub languages: Vec<String>,
    pub revision: Vec<u32>,
    pub disc: Option<u32>,
    pub flags: BTreeSet<String>,
    pub verified: bool,
    pub crc: Option<String>,
    pub content_size: Option<u64>,
    pub total_size: u64,
    pub status: String,
    pub status_reason: Option<String>,
    /// Full name for arcade sets ("Street Fighter II …" for sf2.zip).
    pub alt_name: Option<String>,
    /// Where the file was imported from.
    pub source: Option<String>,
    /// Genre from libretro's genre lists (or set by hand).
    pub genre: Option<String>,
    /// 0 = not played, 1 = played, 2 = finished.
    pub played: u8,
    pub play_count: u32,
    /// Unix time of the last play we know of.
    pub last_played: Option<f64>,
    pub tags: BTreeSet<String>,
}

pub const NOT_PLAYED: u8 = 0;
pub const PLAYED: u8 = 1;
pub const FINISHED: u8 = 2;

pub fn played_label(state: u8) -> &'static str {
    match state {
        PLAYED => "Played",
        FINISHED => "Finished",
        _ => "Not played",
    }
}

impl Rom {
    fn from_row(r: &Row) -> rusqlite::Result<Rom> {
        let js = |i: &str| -> rusqlite::Result<String> { r.get(i) };
        let list = |s: String| serde_json::from_str::<Vec<String>>(&s).unwrap_or_default();
        Ok(Rom {
            id: r.get("id")?,
            system: r.get("system")?,
            path: r.get("path")?,
            files: list(js("files")?),
            stem: r.get("stem")?,
            title: r.get("title")?,
            key: r.get("key")?,
            regions: list(js("regions")?),
            languages: list(js("languages")?),
            revision: serde_json::from_str(&js("revision")?).unwrap_or_default(),
            disc: r.get::<_, Option<i64>>("disc")?.map(|d| d as u32),
            flags: list(js("flags")?).into_iter().collect(),
            verified: r.get::<_, i64>("verified")? != 0,
            crc: r.get("crc")?,
            content_size: r.get::<_, Option<i64>>("content_size")?.map(|n| n as u64),
            total_size: r.get::<_, i64>("total_size")? as u64,
            status: r.get("status")?,
            status_reason: r.get("status_reason")?,
            alt_name: r.get("alt_name").unwrap_or(None),
            source: r.get("source").unwrap_or(None),
            genre: r.get("genre").unwrap_or(None),
            played: r.get::<_, i64>("played").unwrap_or(0) as u8,
            play_count: r.get::<_, i64>("play_count").unwrap_or(0) as u32,
            last_played: r.get("last_played").unwrap_or(None),
            tags: BTreeSet::new(),
        })
    }

    pub fn source_path(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// What to show people: the arcade full name, else the file name.
    pub fn display_name(&self) -> &str {
        self.alt_name.as_deref().unwrap_or(&self.stem)
    }

    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    Added(String),
    Duplicate(String),
    Conflict(String),
}

#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    pub count: u64,
    pub bytes: u64,
    pub trashed: u64,
}

pub struct Library {
    pub root: PathBuf,
    db: Connection,
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Split a ROM path into (stem, ".ext"). Game folders only lose a known
/// suffix, so "Mr. Bones" stays whole.
pub fn split_name(p: &Path) -> (String, String) {
    let name = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if p.is_dir() {
        for suf in [".scummvm", ".svm", ".pc", ".dos"] {
            if name.to_ascii_lowercase().ends_with(suf) && name.len() > suf.len() {
                let cut = name.len() - suf.len();
                return (name[..cut].to_string(), name[cut..].to_string());
            }
        }
        return (name, String::new());
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name, String::new()),
    }
}

/// Copy a file, or a whole folder (DOS / ScummVM games).
pub fn copy_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for e in fs::read_dir(src)? {
            let e = e?;
            copy_path(&e.path(), &dst.join(e.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

fn move_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if fs::rename(src, dst).is_err() {
        // Different drive: copy then delete.
        copy_path(src, dst)?;
        remove_path(src)?;
    }
    Ok(())
}

pub fn remove_path(p: &Path) -> std::io::Result<()> {
    if p.is_dir() {
        fs::remove_dir_all(p)
    } else {
        fs::remove_file(p)
    }
}

/// Size of a file, or of everything in a folder.
pub fn path_size(p: &Path) -> u64 {
    if p.is_dir() {
        walkdir::WalkDir::new(p)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        fs::metadata(p).map(|m| m.len()).unwrap_or(0)
    }
}

impl Library {
    pub fn is_library(path: &Path) -> bool {
        path.join("library.db").is_file()
    }

    pub fn open(root: &Path) -> Result<Library> {
        let root = dunce::canonicalize(root).map_err(|_| Error::NoLibrary(root.to_path_buf()))?;
        if !Self::is_library(&root) {
            return Err(Error::NoLibrary(root));
        }
        let db = Connection::open(root.join("library.db"))?;
        db.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
        db.execute_batch(SCHEMA)?;
        // Added in 0.3: full names for arcade sets (from the MAME/FBNeo lists).
        // Columns added in later versions; older libraries are upgraded in place.
        for (col, decl) in [
            ("alt_name", "TEXT"),
            ("genre", "TEXT"),
            ("played", "INTEGER NOT NULL DEFAULT 0"),
            ("play_count", "INTEGER NOT NULL DEFAULT 0"),
            ("last_played", "REAL"),
        ] {
            let exists: bool = db
                .prepare("SELECT 1 FROM pragma_table_info('roms') WHERE name = ?1")?
                .exists([col])?;
            if !exists {
                db.execute_batch(&format!("ALTER TABLE roms ADD COLUMN {col} {decl}"))?;
            }
        }
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS rom_tags (
                rom_id INTEGER NOT NULL REFERENCES roms(id) ON DELETE CASCADE,
                tag TEXT NOT NULL,
                PRIMARY KEY (rom_id, tag)
            );",
        )?;
        for d in SUBDIRS {
            let _ = fs::create_dir_all(root.join(d));
        }
        Ok(Library { root, db })
    }

    pub fn create(root: &Path) -> Result<Library> {
        fs::create_dir_all(root)?;
        for d in SUBDIRS {
            fs::create_dir_all(root.join(d))?;
        }
        Connection::open(root.join("library.db"))?.execute_batch(SCHEMA)?;
        Self::open(root)
    }

    /// Open a second handle (for a background job).
    pub fn reopen(&self) -> Result<Library> {
        Self::open(&self.root)
    }

    // ------------------------------------------------------------ queries
    pub fn roms(&self, systems: &[String], status: &str) -> Result<Vec<Rom>> {
        let mut sql = "SELECT * FROM roms WHERE status = ?1".to_string();
        if !systems.is_empty() {
            let list: Vec<String> = systems
                .iter()
                .map(|s| format!("'{}'", s.replace('\'', "")))
                .collect();
            sql += &format!(" AND system IN ({})", list.join(","));
        }
        sql += " ORDER BY system, COALESCE(alt_name, stem) COLLATE NOCASE";
        let mut st = self.db.prepare(&sql)?;
        let rows = st.query_map([status], Rom::from_row)?;
        let mut roms = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        let tags = self.tag_map()?;
        for r in &mut roms {
            if let Some(t) = tags.get(&r.id) {
                r.tags = t.clone();
            }
        }
        Ok(roms)
    }

    fn tag_map(&self) -> Result<std::collections::HashMap<i64, BTreeSet<String>>> {
        let mut st = self.db.prepare("SELECT rom_id, tag FROM rom_tags")?;
        let mut out: std::collections::HashMap<i64, BTreeSet<String>> = Default::default();
        for row in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (id, tag) = row?;
            out.entry(id).or_default().insert(tag);
        }
        Ok(out)
    }

    // ------------------------------------------------------------ tags & play state
    pub fn add_tag(&self, ids: &[i64], tag: &str) -> Result<()> {
        let tag = tag.trim();
        if tag.is_empty() {
            return Ok(());
        }
        for id in ids {
            self.db.execute(
                "INSERT OR IGNORE INTO rom_tags VALUES (?1, ?2)",
                params![id, tag],
            )?;
        }
        Ok(())
    }

    pub fn remove_tag(&self, ids: &[i64], tag: &str) -> Result<()> {
        for id in ids {
            self.db.execute(
                "DELETE FROM rom_tags WHERE rom_id = ?1 AND tag = ?2",
                params![id, tag],
            )?;
        }
        Ok(())
    }

    /// Every tag in use, with how many active ROMs carry it.
    pub fn all_tags(&self) -> Result<Vec<(String, u64)>> {
        let mut st = self.db.prepare(
            "SELECT t.tag, COUNT(*) FROM rom_tags t JOIN roms r ON r.id = t.rom_id \
             WHERE r.status = 'active' GROUP BY t.tag ORDER BY t.tag COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn all_genres(&self) -> Result<Vec<(String, u64)>> {
        let mut st = self.db.prepare(
            "SELECT genre, COUNT(*) FROM roms WHERE status = 'active' AND genre IS NOT NULL \
             GROUP BY genre ORDER BY genre COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_genre(&self, id: i64, genre: Option<&str>) -> Result<()> {
        let g = genre.map(str::trim).filter(|g| !g.is_empty());
        self.db
            .execute("UPDATE roms SET genre = ?1 WHERE id = ?2", params![g, id])?;
        Ok(())
    }

    pub fn set_played(&self, ids: &[i64], state: u8) -> Result<()> {
        for id in ids {
            self.db.execute(
                "UPDATE roms SET played = ?1 WHERE id = ?2",
                params![state as i64, id],
            )?;
        }
        Ok(())
    }

    /// Merge play history read from a device: never lowers what we know.
    pub fn record_play(&self, id: i64, count: u32, last: Option<f64>) -> Result<bool> {
        let changed = self.db.execute(
            "UPDATE roms SET played = MAX(played, 1), play_count = MAX(play_count, ?1), \
             last_played = CASE WHEN ?2 IS NULL THEN last_played ELSE MAX(COALESCE(last_played, 0), ?2) END \
             WHERE id = ?3 AND (played = 0 OR play_count < ?1 OR COALESCE(last_played, 0) < COALESCE(?2, 0))",
            params![count as i64, last, id],
        )?;
        Ok(changed > 0)
    }

    pub fn active(&self) -> Result<Vec<Rom>> {
        self.roms(&[], "active")
    }

    pub fn get(&self, id: i64) -> Result<Rom> {
        self.db
            .query_row("SELECT * FROM roms WHERE id = ?1", [id], Rom::from_row)
            .optional()?
            .map(|mut r| {
                r.tags = self
                    .tag_map()
                    .ok()
                    .and_then(|m| m.get(&id).cloned())
                    .unwrap_or_default();
                r
            })
            .ok_or_else(|| Error::Other(format!("No ROM with id {id}")))
    }

    pub fn abspath(&self, relpath: &str) -> PathBuf {
        self.root.join(relpath)
    }

    pub fn abspaths(&self, rom: &Rom) -> Vec<PathBuf> {
        rom.files.iter().map(|f| self.root.join(f)).collect()
    }

    pub fn stats(&self) -> Result<BTreeMap<String, SystemStats>> {
        let mut st = self.db.prepare(
            "SELECT system, status, COUNT(*), COALESCE(SUM(total_size),0) FROM roms GROUP BY system, status",
        )?;
        let mut out: BTreeMap<String, SystemStats> = BTreeMap::new();
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        for row in rows {
            let (sys, status, n, b) = row?;
            let e = out.entry(sys).or_default();
            if status == "active" {
                e.count = n as u64;
                e.bytes = b as u64;
            } else {
                e.trashed = n as u64;
            }
        }
        Ok(out)
    }

    // ------------------------------------------------------------ media
    pub fn media(&self, rom_id: i64) -> Result<BTreeMap<String, PathBuf>> {
        let mut st = self
            .db
            .prepare("SELECT kind, path FROM media WHERE rom_id = ?1 AND path IS NOT NULL")?;
        let rows = st.query_map([rom_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (k, p) = row?;
            let abs = self.root.join(p);
            if abs.exists() {
                out.insert(k, abs);
            }
        }
        Ok(out)
    }

    pub fn media_attempted(&self, rom_id: i64, kind: &str) -> Result<bool> {
        Ok(self
            .db
            .query_row(
                "SELECT 1 FROM media WHERE rom_id = ?1 AND kind = ?2",
                params![rom_id, kind],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub fn set_media(
        &self,
        rom_id: i64,
        kind: &str,
        path: Option<&Path>,
        source: Option<&str>,
    ) -> Result<()> {
        let relp = path.map(|p| rel(&self.root, p));
        self.db.execute(
            "INSERT OR REPLACE INTO media VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rom_id, kind, relp, source, now()],
        )?;
        Ok(())
    }

    pub fn media_dir(&self, system: &str, kind: &str) -> PathBuf {
        self.root.join("media").join(system).join(kind)
    }

    /// Count of active ROMs that have art of this kind.
    pub fn media_count(&self, kind: &str) -> Result<u64> {
        Ok(self.db.query_row(
            "SELECT COUNT(*) FROM media m JOIN roms r ON r.id = m.rom_id \
             WHERE m.kind = ?1 AND m.path IS NOT NULL AND r.status = 'active'",
            [kind],
            |r| r.get::<_, i64>(0),
        )? as u64)
    }

    /// Ids of active ROMs that have art of this kind.
    pub fn roms_with_media(&self, kind: &str) -> Result<std::collections::HashSet<i64>> {
        let mut st = self
            .db
            .prepare("SELECT rom_id FROM media WHERE kind = ?1 AND path IS NOT NULL")?;
        let rows = st.query_map([kind], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ------------------------------------------------------------ import
    fn find_by_crc(&self, system: &str, crc: &str, size: u64) -> Result<Option<Rom>> {
        Ok(self
            .db
            .query_row(
                "SELECT * FROM roms WHERE system = ?1 AND crc = ?2 AND content_size = ?3 AND status = 'active'",
                params![system, crc, size as i64],
                Rom::from_row,
            )
            .optional()?)
    }

    /// Copy (or move) a detected RomSet into the library.
    pub fn add(&self, rs: &RomSet, move_files: bool, dry_run: bool) -> Result<AddOutcome> {
        let sys = rs
            .system
            .ok_or_else(|| Error::Other("ROM has no system".into()))?;
        let (crc, csize) = content_crc(&rs.main)?;
        if let Some(existing) = self.find_by_crc(sys.id, &crc, csize)? {
            return Ok(AddOutcome::Duplicate(existing.path));
        }
        let dest_dir = self.root.join("roms").join(sys.id);
        let (stem, ext) = match (&rs.rename, rs.companions.is_empty() && rs.main.is_file()) {
            (Some(new), true) => (naming::safe_filename(new), split_name(&rs.main).1),
            _ => split_name(&rs.main),
        };
        let name = format!("{stem}{ext}");
        let mut main_dest = dest_dir.join(&name);
        if main_dest.exists() {
            if !rs.companions.is_empty() {
                return Ok(AddOutcome::Conflict(format!(
                    "{name} already exists with different data"
                )));
            }
            let mut n = 2;
            while dest_dir.join(format!("{stem} ({n}){ext}")).exists() {
                n += 1;
            }
            main_dest = dest_dir.join(format!("{stem} ({n}){ext}"));
        }
        let comp_dests: Vec<PathBuf> = rs
            .companions
            .iter()
            .map(|c| dest_dir.join(c.file_name().unwrap_or_default()))
            .collect();
        if let Some(c) = comp_dests.iter().find(|c| c.exists()) {
            return Ok(AddOutcome::Conflict(format!(
                "companion file {} already exists",
                c.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        if dry_run {
            return Ok(AddOutcome::Added(rel(&self.root, &main_dest)));
        }
        fs::create_dir_all(&dest_dir)?;
        let dests: Vec<PathBuf> = std::iter::once(main_dest.clone())
            .chain(comp_dests)
            .collect();
        for (src, dst) in rs.files().iter().zip(&dests) {
            if move_files {
                move_file(src, dst)?;
            } else {
                copy_path(src, dst)?;
            }
        }
        let stem = split_name(&main_dest).0;
        if sys.id == "scummvm" && main_dest.is_dir() {
            crate::folder_games::ensure_scummvm_id(&main_dest, &stem, Some(&rs.main));
        }
        let p = naming::parse(&stem);
        let files: Vec<String> = dests.iter().map(|d| rel(&self.root, d)).collect();
        let total: u64 = dests.iter().map(|d| path_size(d)).sum();
        let flags: Vec<&String> = p.flags.iter().collect();
        self.db.execute(
            "INSERT INTO roms (system, path, files, stem, title, key, regions, languages, revision, disc, \
             flags, verified, crc, content_size, total_size, source, added) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                sys.id,
                files[0],
                serde_json::to_string(&files).unwrap(),
                stem,
                p.title,
                p.key,
                serde_json::to_string(&p.regions).unwrap(),
                serde_json::to_string(&p.languages).unwrap(),
                serde_json::to_string(&p.revision).unwrap(),
                p.disc.map(|d| d as i64),
                serde_json::to_string(&flags).unwrap(),
                p.verified as i64,
                crc,
                csize as i64,
                total as i64,
                rs.main.to_string_lossy(),
                now()
            ],
        )?;
        Ok(AddOutcome::Added(files[0].clone()))
    }

    pub fn add_unsorted(&self, rs: &RomSet, move_files: bool) -> Result<()> {
        let dir = self.root.join("unsorted");
        fs::create_dir_all(&dir)?;
        for src in rs.files() {
            let d = dir.join(src.file_name().unwrap_or_default());
            if !d.exists() {
                if move_files {
                    move_file(&src, &d)?
                } else {
                    copy_path(&src, &d)?
                }
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ trash
    fn relocate(&self, rom: &Rom, dir: &Path, status: &str, reason: Option<&str>) -> Result<()> {
        fs::create_dir_all(dir)?;
        let mut new_files = vec![];
        for f in &rom.files {
            let src = self.root.join(f);
            let dst = dir.join(src.file_name().unwrap_or_default());
            if status == "active" && dst.exists() {
                return Err(Error::Other(format!(
                    "Can't restore: {} already exists",
                    dst.display()
                )));
            }
            if src.exists() {
                move_file(&src, &dst)?;
            }
            new_files.push(rel(&self.root, &dst));
        }
        self.db.execute(
            "UPDATE roms SET status=?1, status_reason=?2, path=?3, files=?4 WHERE id=?5",
            params![
                status,
                reason,
                new_files[0],
                serde_json::to_string(&new_files).unwrap(),
                rom.id
            ],
        )?;
        Ok(())
    }

    pub fn trash(&self, rom: &Rom, reason: &str) -> Result<()> {
        self.relocate(
            rom,
            &self.root.join("trash").join(&rom.system),
            "trashed",
            Some(reason),
        )
    }

    pub fn restore(&self, rom: &Rom) -> Result<()> {
        self.relocate(
            rom,
            &self.root.join("roms").join(&rom.system),
            "active",
            None,
        )
    }

    pub fn empty_trash(&self) -> Result<usize> {
        let trashed = self.roms(&[], "trashed")?;
        for r in &trashed {
            for f in &r.files {
                let _ = remove_path(&self.root.join(f));
            }
            self.db.execute("DELETE FROM roms WHERE id = ?1", [r.id])?;
        }
        Ok(trashed.len())
    }

    /// Move a ROM to another system and/or give it a new file name stem,
    /// renaming its artwork too. Returns false if the target name is taken.
    pub fn relocate_rom(&self, rom: &Rom, system: &str, new_stem: &str) -> Result<bool> {
        let new_stem = naming::safe_filename(new_stem);
        let src = self.root.join(&rom.path);
        let (_, ext) = split_name(&src);
        let dir = self.root.join("roms").join(system);
        let dest = dir.join(format!("{new_stem}{ext}"));
        if dest == src {
            return Ok(true);
        }
        // Case-only renames are fine on case-insensitive disks.
        let same_file =
            dest.to_string_lossy().to_lowercase() == src.to_string_lossy().to_lowercase();
        if dest.exists() && !same_file {
            return Ok(false);
        }
        fs::create_dir_all(&dir)?;
        if same_file {
            let tmp = dir.join(format!(".renaming-{}", rom.id));
            fs::rename(&src, &tmp)?;
            fs::rename(&tmp, &dest)?;
        } else {
            move_file(&src, &dest)?;
        }
        let p = naming::parse(&new_stem);
        let rel_main = rel(&self.root, &dest);
        let flags: Vec<&String> = p.flags.iter().collect();
        self.db.execute(
            "UPDATE roms SET system=?1, path=?2, files=?3, stem=?4, title=?5, key=?6, regions=?7, languages=?8, \
             revision=?9, disc=?10, flags=?11, verified=?12 WHERE id=?13",
            params![
                system,
                rel_main,
                serde_json::to_string(&vec![rel_main.clone()]).unwrap(),
                new_stem,
                p.title,
                p.key,
                serde_json::to_string(&p.regions).unwrap(),
                serde_json::to_string(&p.languages).unwrap(),
                serde_json::to_string(&p.revision).unwrap(),
                p.disc.map(|d| d as i64),
                serde_json::to_string(&flags).unwrap(),
                p.verified as i64,
                rom.id
            ],
        )?;
        // Rename cached artwork to match.
        for (kind, path) in self.media(rom.id)? {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let target = self
                .media_dir(system, &kind)
                .join(format!("{new_stem}.{ext}"));
            if target != path {
                fs::create_dir_all(target.parent().unwrap())?;
                if move_file(&path, &target).is_ok() {
                    self.set_media(rom.id, &kind, Some(&target), None)?;
                }
            }
        }
        Ok(true)
    }

    /// Record an arcade set's full name and re-derive its title, region and flags.
    pub fn set_alt_name(&self, rom_id: i64, full_name: &str) -> Result<()> {
        let mut p = naming::parse_arcade(full_name);
        p.flags.remove("bios"); // arcade BIOS sets (neogeo.zip) must stay with the games
        let flags: Vec<&String> = p.flags.iter().collect();
        self.db.execute(
            "UPDATE roms SET alt_name=?1, title=?2, key=?3, regions=?4, revision=?5, flags=?6 WHERE id=?7",
            params![
                full_name,
                p.title,
                p.key,
                serde_json::to_string(&p.regions).unwrap(),
                serde_json::to_string(&p.revision).unwrap(),
                serde_json::to_string(&flags).unwrap(),
                rom_id
            ],
        )?;
        Ok(())
    }

    pub fn begin(&self) -> Result<()> {
        self.db.execute_batch("BEGIN")?;
        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        self.db.execute_batch("COMMIT")?;
        Ok(())
    }
}
