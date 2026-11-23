"""Work out which system a ROM file belongs to.

Order of evidence:
  1. Unambiguous extension (.gba, .sfc, ...)
  2. Contents of .zip archives (the extension of the ROM inside)
  3. Folder names anywhere in the path ("PS1", "Game Boy (GB)", "megadrive", ...)
  4. Header sniffing for .bin / .iso / .cue (Genesis, 32X, PS1, PSP, Sega CD, Saturn)
"""

from __future__ import annotations

import os
import re
import zipfile
from dataclasses import dataclass, field
from pathlib import Path

from . import systems
from .systems import AMBIGUOUS_EXTS, EXT_MAP, System

IGNORED_EXTS = {
    ".txt", ".nfo", ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".xml", ".dat", ".db",
    ".ini", ".cfg", ".srm", ".sav", ".state", ".sfv", ".md5", ".sha1", ".url",
    ".pdf", ".html", ".htm", ".json", ".log", ".ds_store", ".lnk", ".exe", ".dll",
    ".auto", ".ips", ".bps", ".ups", ".xdelta", ".rtc", ".mcr", ".mcd",
}


@dataclass
class RomSet:
    """A playable unit: one file, or a .cue/.gdi/.m3u plus the files it references."""
    main: Path
    companions: list[Path] = field(default_factory=list)
    system: System | None = None
    reason: str = ""

    @property
    def files(self) -> list[Path]:
        return [self.main, *self.companions]

    @property
    def size(self) -> int:
        return sum(p.stat().st_size for p in self.files if p.exists())


def _cue_refs(path: Path) -> list[Path]:
    refs = []
    try:
        text = path.read_text(errors="replace")
    except OSError:
        return refs
    for m in re.finditer(r'^\s*FILE\s+"?(.+?)"?\s+\S+\s*$', text, re.M | re.I):
        refs.append(path.parent / m.group(1))
    return refs


def _gdi_refs(path: Path) -> list[Path]:
    refs = []
    try:
        lines = path.read_text(errors="replace").splitlines()[1:]
    except OSError:
        return refs
    for line in lines:
        m = re.match(r'\s*\d+\s+\d+\s+\d+\s+\d+\s+(?:"([^"]+)"|(\S+))', line)
        if m:
            refs.append(path.parent / (m.group(1) or m.group(2)))
    return refs


def _m3u_refs(path: Path) -> list[Path]:
    refs = []
    try:
        for line in path.read_text(errors="replace").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                refs.append(path.parent / line)
    except OSError:
        pass
    return refs


def _references(path: Path) -> list[Path]:
    ext = path.suffix.lower()
    if ext == ".cue":
        return _cue_refs(path)
    if ext == ".gdi":
        return _gdi_refs(path)
    if ext == ".ccd":
        return [p for p in (path.with_suffix(".img"), path.with_suffix(".sub")) if p.exists()]
    return []


def _read(path: Path, offset: int, length: int) -> bytes:
    try:
        with open(path, "rb") as fh:
            fh.seek(offset)
            return fh.read(length)
    except OSError:
        return b""


def sniff(path: Path) -> System | None:
    """Look inside a cartridge dump or disc image for a known signature."""
    ext = path.suffix.lower()
    if ext == ".cue":
        refs = _cue_refs(path)
        return sniff(refs[0]) if refs and refs[0].exists() else None
    if ext not in (".bin", ".iso", ".img"):
        return None
    head = _read(path, 0, 0x200)
    if head[0x100:0x108] == b"SEGA 32X":
        return systems.get("sega32x")
    if head[0x100:0x104] == b"SEGA" or head[0x101:0x105] == b"SEGA":
        return systems.get("genesis")
    for off in (0, 0x10):
        if head[off:off + 14] == b"SEGADISCSYSTEM":
            return systems.get("segacd")
        if head[off:off + 15] == b"SEGA SEGASATURN":
            return systems.get("saturn")
    # ISO9660 primary volume descriptor: cooked (2048) or raw (2352 + 24 header)
    for pvd in (0x8000, 16 * 2352 + 24, 16 * 2352 + 16):
        block = _read(path, pvd, 64)
        if block[1:6] == b"CD001":
            ident = block[8:40]
            if ident.startswith(b"PSP GAME"):
                return systems.get("psp")
            if ident.startswith(b"PLAYSTATION"):
                return systems.get("psx")
    if ext == ".bin":
        size = path.stat().st_size
        if size <= 64 * 1024 and size in (2048, 4096, 8192, 12288, 16384, 32768, 65536):
            return systems.get("atari2600")
    return None


def _zip_system(path: Path) -> tuple[System | None, str]:
    try:
        with zipfile.ZipFile(path) as zf:
            names = [n for n in zf.namelist() if not n.endswith("/")]
    except (zipfile.BadZipFile, OSError):
        return None, "unreadable zip"
    found = {EXT_MAP[e] for e in (os.path.splitext(n)[1].lower() for n in names) if e in EXT_MAP}
    if len(found) == 1:
        return found.pop(), "zip contents"
    if not found and len(names) > 1:
        return None, "zip with many non-rom files (arcade?)"
    return None, "zip contents ambiguous"


def folder_hint(path: Path, root: Path | None = None) -> System | None:
    parts = path.parent.parts
    if root is not None:
        try:
            parts = path.parent.relative_to(root).parts
        except ValueError:
            pass
    for part in reversed(parts):
        s = systems.system_from_folder_name(part)
        if s:
            return s
    return None


def detect(path: Path, root: Path | None = None, forced: System | None = None) -> RomSet:
    ext = path.suffix.lower()
    rs = RomSet(path, [p for p in _references(path) if p.exists() and p != path])
    if forced:
        rs.system, rs.reason = forced, "forced"
        return rs
    if ext in EXT_MAP and ext not in AMBIGUOUS_EXTS:
        rs.system, rs.reason = EXT_MAP[ext], "extension"
        return rs
    hint = folder_hint(path, root)
    if ext == ".zip":
        s, why = _zip_system(path)
        if s:
            rs.system, rs.reason = s, why
            return rs
        if hint and hint.accepts(ext):
            rs.system, rs.reason = hint, "folder name"
            return rs
        rs.reason = why
        return rs
    if hint and hint.accepts(ext):
        rs.system, rs.reason = hint, "folder name"
        return rs
    s = sniff(path)
    if s:
        rs.system, rs.reason = s, "file header"
        return rs
    if ext == ".gdi" or ext == ".cdi":
        rs.system, rs.reason = systems.get("dreamcast"), "extension"
        return rs
    if ext == ".cso":
        rs.system, rs.reason = systems.get("psp"), "extension"
        return rs
    if ext == ".pbp":
        rs.system, rs.reason = systems.get("psx"), "extension"
        return rs
    rs.reason = "unknown extension" if ext not in AMBIGUOUS_EXTS else "ambiguous; no folder hint"
    return rs


def scan(paths: list[Path], forced: System | None = None):
    """Yield RomSets for every ROM under the given files/folders.

    Files referenced by a .cue/.gdi/.m3u are folded into that set rather than
    yielded on their own. Returns (romsets, skipped_paths).
    """
    candidates: list[tuple[Path, Path]] = []
    for root in paths:
        root = root.resolve()
        if root.is_file():
            candidates.append((root, root.parent))
            continue
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if not d.startswith(".")]
            for f in filenames:
                if not f.startswith("."):
                    candidates.append((Path(dirpath) / f, root))

    claimed: set[Path] = set()
    for p, _ in candidates:
        ext = p.suffix.lower()
        if ext in (".cue", ".gdi", ".ccd"):
            claimed.update(r.resolve() for r in _references(p))
        elif ext == ".m3u":
            # discs listed in an m3u stay as their own entries; the m3u is
            # regenerated on export, so drop the original playlist.
            claimed.add(p.resolve())

    sets, skipped = [], []
    for p, root in candidates:
        if p.resolve() in claimed:
            continue
        if p.suffix.lower() in IGNORED_EXTS:
            skipped.append(p)
            continue
        sets.append(detect(p, root, forced))
    return sets, skipped
