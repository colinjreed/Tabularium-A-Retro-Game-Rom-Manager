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

#[derive(Debug, Clone, PartialEq)]
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
        })
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

fn move_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if fs::rename(src, dst).is_err() {
        // Different drive: copy then delete.
        fs::copy(src, dst)?;
        fs::remove_file(src)?;
    }
    Ok(())
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
        sql += " ORDER BY system, stem COLLATE NOCASE";
        let mut st = self.db.prepare(&sql)?;
        let rows = st.query_map([status], Rom::from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn active(&self) -> Result<Vec<Rom>> {
        self.roms(&[], "active")
    }

    pub fn get(&self, id: i64) -> Result<Rom> {
        self.db
            .query_row("SELECT * FROM roms WHERE id = ?1", [id], Rom::from_row)
            .optional()?
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
        let name = rs.main.file_name().unwrap_or_default();
        let mut main_dest = dest_dir.join(name);
        if main_dest.exists() {
            if !rs.companions.is_empty() {
                return Ok(AddOutcome::Conflict(format!(
                    "{} already exists with different data",
                    name.to_string_lossy()
                )));
            }
            let stem = rs.main.file_stem().unwrap_or_default().to_string_lossy();
            let ext = rs
                .main
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
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
                fs::copy(src, dst)?;
            }
        }
        let stem = main_dest
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let p = naming::parse(&stem);
        let files: Vec<String> = dests.iter().map(|d| rel(&self.root, d)).collect();
        let total: u64 = dests
            .iter()
            .filter_map(|d| fs::metadata(d).ok())
            .map(|m| m.len())
            .sum();
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
                    fs::copy(&src, &d).map(|_| ())?
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
                let _ = fs::remove_file(self.root.join(f));
            }
            self.db.execute("DELETE FROM roms WHERE id = ?1", [r.id])?;
        }
        Ok(trashed.len())
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
