"""The master library: an organised folder of ROMs plus a SQLite index.

Layout::

    LIBRARY/
      library.db                 index of every ROM, hashes and parsed tags
      roms/<system>/<file>       organised ROMs
      media/<system>/<kind>/     cached artwork (boxart, snap, title, mix, ...)
      trash/<system>/<file>      ROMs removed by `clean --apply` (restorable)
      unsorted/                  files whose system couldn't be identified
      collections/<name>.json    collection definitions
      bios/                      verified BIOS files (see `rom-manager bios`)
"""

from __future__ import annotations

import json
import shutil
import sqlite3
import time
from dataclasses import dataclass
from pathlib import Path

from . import naming, systems
from .detect import RomSet
from .hashing import content_crc

SCHEMA = """
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
"""


@dataclass
class Rom:
    id: int
    system: str
    path: str
    files: list[str]
    stem: str
    title: str
    key: str
    regions: list[str]
    languages: list[str]
    revision: tuple
    disc: int | None
    flags: set[str]
    verified: bool
    crc: str | None
    content_size: int | None
    total_size: int
    status: str
    status_reason: str | None

    @property
    def ext(self) -> str:
        return Path(self.path).suffix.lower()

    @classmethod
    def from_row(cls, r: sqlite3.Row) -> "Rom":
        return cls(
            id=r["id"], system=r["system"], path=r["path"], files=json.loads(r["files"]),
            stem=r["stem"], title=r["title"], key=r["key"],
            regions=json.loads(r["regions"]), languages=json.loads(r["languages"]),
            revision=tuple(json.loads(r["revision"])), disc=r["disc"],
            flags=set(json.loads(r["flags"])), verified=bool(r["verified"]), crc=r["crc"],
            content_size=r["content_size"], total_size=r["total_size"], status=r["status"],
            status_reason=r["status_reason"],
        )


class LibraryError(Exception):
    pass


class Library:
    def __init__(self, root: Path):
        self.root = Path(root).expanduser().resolve()
        self.db_path = self.root / "library.db"
        if not self.db_path.exists():
            raise LibraryError(
                f"No library at {self.root}. Create one with: rom-manager init {self.root}")
        self.db = sqlite3.connect(self.db_path)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA foreign_keys = ON")
        self.db.executescript(SCHEMA)

    @classmethod
    def create(cls, root: Path) -> "Library":
        root = Path(root).expanduser().resolve()
        for sub in ("roms", "media", "trash", "unsorted", "collections", "bios"):
            (root / sub).mkdir(parents=True, exist_ok=True)
        sqlite3.connect(root / "library.db").executescript(SCHEMA)
        return cls(root)

    # ---------------------------------------------------------------- queries
    def roms(self, systems_: list[str] | None = None, status: str = "active") -> list[Rom]:
        sql = "SELECT * FROM roms WHERE status = ?"
        args: list = [status]
        if systems_:
            sql += f" AND system IN ({','.join('?' * len(systems_))})"
            args += systems_
        sql += " ORDER BY system, title, stem"
        return [Rom.from_row(r) for r in self.db.execute(sql, args)]

    def get(self, rom_id: int) -> Rom:
        r = self.db.execute("SELECT * FROM roms WHERE id = ?", (rom_id,)).fetchone()
        if not r:
            raise LibraryError(f"No ROM with id {rom_id}")
        return Rom.from_row(r)

    def media(self, rom_id: int) -> dict[str, Path]:
        out = {}
        for r in self.db.execute("SELECT kind, path FROM media WHERE rom_id = ? AND path IS NOT NULL",
                                 (rom_id,)):
            p = self.root / r["path"]
            if p.exists():
                out[r["kind"]] = p
        return out

    def media_attempted(self, rom_id: int, kind: str) -> bool:
        return self.db.execute("SELECT 1 FROM media WHERE rom_id = ? AND kind = ?",
                               (rom_id, kind)).fetchone() is not None

    def set_media(self, rom_id: int, kind: str, path: Path | None, source: str | None):
        rel = str(path.relative_to(self.root)) if path else None
        self.db.execute("INSERT OR REPLACE INTO media VALUES (?, ?, ?, ?, ?)",
                        (rom_id, kind, rel, source, time.time()))
        self.db.commit()

    def media_dir(self, system: str, kind: str) -> Path:
        return self.root / "media" / system / kind

    def stats(self) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT system, status, COUNT(*) AS n, SUM(total_size) AS bytes "
            "FROM roms GROUP BY system, status ORDER BY system, status").fetchall()

    def abspath(self, rom: Rom) -> Path:
        return self.root / rom.path

    def abspaths(self, rom: Rom) -> list[Path]:
        return [self.root / f for f in rom.files]

    # ---------------------------------------------------------------- import
    def find_by_crc(self, system: str, crc: str, size: int) -> Rom | None:
        r = self.db.execute(
            "SELECT * FROM roms WHERE system = ? AND crc = ? AND content_size = ? "
            "AND status = 'active'", (system, crc, size)).fetchone()
        return Rom.from_row(r) if r else None

    def add(self, rs: RomSet, move: bool = False, dry_run: bool = False) -> tuple[str, str]:
        """Copy (or move) a detected RomSet into the library.

        Returns (outcome, detail) where outcome is one of
        'added', 'duplicate', 'conflict', 'error'.
        """
        sys_ = rs.system
        assert sys_ is not None
        try:
            crc, csize = content_crc(rs.main)
        except OSError as e:
            return "error", str(e)
        existing = self.find_by_crc(sys_.id, crc, csize)
        if existing:
            return "duplicate", f"same data as {existing.path}"

        dest_dir = self.root / "roms" / sys_.id
        main_dest = dest_dir / rs.main.name
        if main_dest.exists():
            stem, ext = rs.main.stem, rs.main.suffix
            n = 2
            while (dest_dir / f"{stem} ({n}){ext}").exists():
                n += 1
            if rs.companions:
                return "conflict", f"{main_dest.name} already exists with different data"
            main_dest = dest_dir / f"{stem} ({n}){ext}"
        comp_dests = [dest_dir / c.name for c in rs.companions]
        for c in comp_dests:
            if c.exists():
                return "conflict", f"companion file {c.name} already exists"

        if dry_run:
            return "added", str(main_dest.relative_to(self.root))

        dest_dir.mkdir(parents=True, exist_ok=True)
        op = shutil.move if move else shutil.copy2
        for src, dst in zip(rs.files, [main_dest, *comp_dests]):
            op(str(src), str(dst))

        p = naming.parse(main_dest.stem)
        files = [str(x.relative_to(self.root)) for x in [main_dest, *comp_dests]]
        total = sum((self.root / f).stat().st_size for f in files)
        self.db.execute(
            "INSERT INTO roms (system, path, files, stem, title, key, regions, languages, "
            "revision, disc, flags, verified, crc, content_size, total_size, source, added) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (sys_.id, files[0], json.dumps(files), main_dest.stem, p.title, p.key,
             json.dumps(p.regions), json.dumps(p.languages), json.dumps(list(p.revision)),
             p.disc, json.dumps(sorted(p.flags)), int(p.verified), crc, csize, total,
             str(rs.main), time.time()))
        self.db.commit()
        return "added", files[0]

    def add_unsorted(self, rs: RomSet, move: bool = False) -> Path:
        dest = self.root / "unsorted" / rs.main.name
        dest.parent.mkdir(parents=True, exist_ok=True)
        op = shutil.move if move else shutil.copy2
        for src in rs.files:
            d = dest.parent / src.name
            if not d.exists():
                op(str(src), str(d))
        return dest

    # ---------------------------------------------------------------- trash
    def trash(self, rom: Rom, reason: str):
        tdir = self.root / "trash" / rom.system
        tdir.mkdir(parents=True, exist_ok=True)
        new_files = []
        for f in rom.files:
            src = self.root / f
            dst = tdir / src.name
            if src.exists():
                shutil.move(str(src), str(dst))
            new_files.append(str(dst.relative_to(self.root)))
        self.db.execute("UPDATE roms SET status='trashed', status_reason=?, path=?, files=? "
                        "WHERE id=?", (reason, new_files[0], json.dumps(new_files), rom.id))
        self.db.commit()

    def restore(self, rom: Rom):
        rdir = self.root / "roms" / rom.system
        rdir.mkdir(parents=True, exist_ok=True)
        new_files = []
        for f in rom.files:
            src = self.root / f
            dst = rdir / src.name
            if dst.exists():
                raise LibraryError(f"Cannot restore {src.name}: {dst} already exists")
            if src.exists():
                shutil.move(str(src), str(dst))
            new_files.append(str(dst.relative_to(self.root)))
        self.db.execute("UPDATE roms SET status='active', status_reason=NULL, path=?, files=? "
                        "WHERE id=?", (new_files[0], json.dumps(new_files), rom.id))
        self.db.commit()

    def empty_trash(self) -> int:
        n = 0
        for r in self.roms(status="trashed"):
            for f in r.files:
                (self.root / f).unlink(missing_ok=True)
            self.db.execute("DELETE FROM roms WHERE id = ?", (r.id,))
            n += 1
        self.db.commit()
        return n

    def system_ids(self) -> list[str]:
        return [r[0] for r in self.db.execute(
            "SELECT DISTINCT system FROM roms WHERE status='active' ORDER BY system")]


def system_name(system_id: str) -> str:
    return systems.BY_ID[system_id].name if system_id in systems.BY_ID else system_id
