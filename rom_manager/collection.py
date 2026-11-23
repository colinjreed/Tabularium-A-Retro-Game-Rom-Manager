"""Collections: saved selections of the library exported to an SD card.

A collection is a JSON file in LIBRARY/collections/. It names a target
profile (firmware), which systems to include, filter rules, an optional size
budget and the artwork type. Exporting never modifies the library.
"""

from __future__ import annotations

import json
import posixpath
import re
import shutil
import time
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

from . import bios as bios_mod
from . import filters, naming, profiles, systems
from .library import Library, LibraryError, Rom

MANIFEST_DIR = ".rom-manager"


@dataclass
class Collection:
    name: str
    profile: str
    systems: list[str] = field(default_factory=list)
    filters: dict = field(default_factory=dict)
    max_size: int | None = None
    priority_titles: list[str] = field(default_factory=list)
    art_kind: str | None = None
    multidisc_m3u: bool = True
    folder_overrides: dict = field(default_factory=dict)
    include_ids: list[int] = field(default_factory=list)
    exclude_ids: list[int] = field(default_factory=list)

    @classmethod
    def load(cls, lib: Library, name: str) -> "Collection":
        path = collection_path(lib, name)
        if not path.exists():
            raise LibraryError(f"No collection named '{name}'. See: rom-manager collection list")
        data = json.loads(path.read_text())
        return cls(**{k: v for k, v in data.items() if k in cls.__dataclass_fields__})

    def save(self, lib: Library):
        path = collection_path(lib, self.name)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.__dict__, indent=2) + "\n")

    @property
    def filter_spec(self) -> filters.FilterSpec:
        return filters.FilterSpec.from_dict(self.filters)


def collection_path(lib: Library, name: str) -> Path:
    if not re.fullmatch(r"[A-Za-z0-9._ -]+", name):
        raise LibraryError("Collection names may contain letters, numbers, spaces, . _ -")
    return lib.root / "collections" / f"{name}.json"


def list_collections(lib: Library) -> list[str]:
    return sorted(p.stem for p in (lib.root / "collections").glob("*.json"))


# ---------------------------------------------------------------- resolving
@dataclass
class Selection:
    roms: list[Rom]
    rejected: list[tuple[Rom, str]]
    over_budget: list[Rom]

    @property
    def size(self) -> int:
        return sum(r.total_size for r in self.roms)


def _units(roms: list[Rom]) -> list[list[Rom]]:
    """Group discs of one release so a size budget never splits a game."""
    groups: dict[tuple, list[Rom]] = defaultdict(list)
    for r in roms:
        groups[(r.system, filters.variant_key(r))].append(r)
    return [sorted(g, key=lambda r: r.disc or 0) for g in groups.values()]


def resolve(lib: Library, col: Collection) -> Selection:
    pool = lib.roms(col.systems or None)
    spec = col.filter_spec
    kept, rejected = filters.apply(pool, spec)

    kept_ids = {r.id for r in kept}
    if col.include_ids:
        extra = [lib.get(i) for i in col.include_ids if i not in kept_ids]
        kept += [r for r in extra if r.status == "active"]
    if col.exclude_ids:
        ex = set(col.exclude_ids)
        rejected += [(r, "excluded by id") for r in kept if r.id in ex]
        kept = [r for r in kept if r.id not in ex]

    over: list[Rom] = []
    if col.max_size:
        units = _units(kept)

        def is_priority(u):
            return any(filters._match_any(n, col.priority_titles)
                       for r in u for n in (r.title, r.stem))
        prio = [u for u in units if col.priority_titles and is_priority(u)]
        rest = [u for u in units if not (col.priority_titles and is_priority(u))]
        # Round-robin across systems so one big system can't starve the others.
        by_sys: dict[str, list[list[Rom]]] = defaultdict(list)
        for u in sorted(rest, key=lambda u: (u[0].system, u[0].title)):
            by_sys[u[0].system].append(u)
        order = list(prio)
        queues = [by_sys[s] for s in sorted(by_sys)]
        while any(queues):
            for q in queues:
                if q:
                    order.append(q.pop(0))
        chosen, used = [], 0
        for u in order:
            size = sum(r.total_size for r in u)
            if used + size <= col.max_size:
                chosen.extend(u)
                used += size
            else:
                over.extend(u)
        kept = chosen
    kept.sort(key=lambda r: (r.system, r.title, r.disc or 0))
    return Selection(kept, rejected, over)


# ---------------------------------------------------------------- exporting
@dataclass
class ExportReport:
    copied: int = 0
    unchanged: int = 0
    art: int = 0
    art_missing: int = 0
    removed: int = 0
    bytes_copied: int = 0
    bios_copied: list[str] = field(default_factory=list)
    bios_missing_required: list[str] = field(default_factory=list)
    bios_missing_optional: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    planned: list[str] = field(default_factory=list)


def _prepare_art(src: Path, width: int | None) -> bytes:
    """Return PNG bytes, resized to `width` when Pillow is available."""
    data = src.read_bytes()
    try:
        from PIL import Image  # optional dependency
    except ImportError:
        return data
    import io
    try:
        im = Image.open(io.BytesIO(data))
        im.load()
    except Exception:  # corrupt/unknown image: copy as-is rather than abort the export
        return data
    with im:
        changed = False
        if width and im.width > width:
            h = round(im.height * width / im.width)
            im = im.resize((width, h), Image.LANCZOS)
            changed = True
        if src.suffix.lower() != ".png" or changed:
            if im.mode not in ("RGB", "RGBA"):
                im = im.convert("RGBA")
            out = io.BytesIO()
            im.save(out, "PNG", optimize=True)
            return out.getvalue()
    return data


def _copy(src: Path, dst: Path, report: ExportReport, dry_run: bool):
    if dst.exists() and dst.stat().st_size == src.stat().st_size:
        report.unchanged += 1
        return
    report.planned.append(str(dst))
    if dry_run:
        report.copied += 1
        report.bytes_copied += src.stat().st_size
        return
    dst.parent.mkdir(parents=True, exist_ok=True)
    tmp = dst.with_name(dst.name + ".part")
    shutil.copyfile(src, tmp)
    tmp.replace(dst)
    report.copied += 1
    report.bytes_copied += src.stat().st_size


FAT_BAD = re.compile(r'[<>:"|?*]')
FAT32_MAX = 4 * 1024 ** 3 - 1


def export(lib: Library, col: Collection, dest: Path, dry_run: bool = False,
           sync: bool = False, art: bool = True, force: bool = False,
           bios: bool = True, log=print) -> ExportReport:
    profile = profiles.get(col.profile)
    art_kind = col.art_kind or profile.art_kind_default
    sel = resolve(lib, col)
    report = ExportReport()
    dest = Path(dest).expanduser().resolve()
    overrides = col.folder_overrides

    if not dry_run and not force and dest.exists():
        free = shutil.disk_usage(dest).free
        need = sum(r.total_size for r in sel.roms)
        if need > free:
            raise LibraryError(
                f"Collection needs {human(need)} but {dest} has {human(free)} free. "
                "Set a max size on the collection or pass --force.")

    written: set[str] = set()
    gamelists: dict[PurePosixPath, list[dict]] = defaultdict(list)

    def write_bytes(rel: PurePosixPath, data: bytes):
        target = dest / rel
        written.add(str(rel))
        if target.exists() and target.read_bytes() == data:
            return
        report.planned.append(str(target))
        if not dry_run:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)

    def place_art(rom: Rom, system, entry_rel: PurePosixPath) -> str | None:
        if not art:
            return None
        media = lib.media(rom.id)
        src = media.get(art_kind) or media.get("boxart") or next(iter(media.values()), None)
        if not src:
            report.art_missing += 1
            return None
        rel = profile.art_path(system, entry_rel, overrides, art_kind)
        if rel is None:
            return None
        write_bytes(rel, _prepare_art(src, profile.art_width))
        report.art += 1
        return posixpath.relpath(rel.as_posix(), entry_rel.parent.as_posix())

    for unit in _units(sel.roms):
        first = unit[0]
        system = systems.BY_ID[first.system]
        sys_dir = profile.rom_dir(system, overrides)
        multidisc = (col.multidisc_m3u and system.disc_based and len(unit) > 1
                     and all(r.disc for r in unit))
        if multidisc:
            base = naming.strip_disc(first.stem)
            hidden = sys_dir / f".{base}"
            lines = []
            for r in unit:
                for f in lib.abspaths(r):
                    rel = hidden / f.name
                    _copy(f, dest / rel, report, dry_run)
                    written.add(str(rel))
                lines.append(f".{base}/{Path(r.path).name}")
            m3u_rel = sys_dir / f"{base}.m3u"
            write_bytes(m3u_rel, ("\n".join(lines) + "\n").encode())
            image = place_art(first, system, m3u_rel)
            gamelists[sys_dir].append({"path": f"./{m3u_rel.name}",
                                       "name": naming.strip_disc(first.title),
                                       "image": f"./{image}" if image else None})
            continue
        for r in unit:
            main_rel = None
            for f in lib.abspaths(r):
                rel = sys_dir / f.name
                if FAT_BAD.search(f.name):
                    report.warnings.append(f"'{f.name}' has characters FAT/exFAT cards reject")
                if f.stat().st_size > FAT32_MAX:
                    report.warnings.append(f"'{f.name}' is over 4 GB: card must be exFAT, not FAT32")
                _copy(f, dest / rel, report, dry_run)
                written.add(str(rel))
                main_rel = main_rel or rel
            image = place_art(r, system, main_rel)
            gamelists[sys_dir].append({"path": f"./{main_rel.name}", "name": r.title,
                                       "image": f"./{image}" if image else None})

    if bios:
        exported_systems = sorted({r.system for r in sel.roms})
        (report.bios_copied, report.bios_missing_required,
         report.bios_missing_optional) = bios_mod.install(
            lib.root / "bios", dest, col.profile, exported_systems, dry_run)

    manifest_path = dest / MANIFEST_DIR / f"{col.name}.json"
    previous: set[str] = set()
    if manifest_path.exists():
        previous = set(json.loads(manifest_path.read_text()).get("files", []))

    if sync and previous:
        stale = sorted(previous - written)
        for rel in stale:
            target = dest / rel
            if target.exists():
                report.removed += 1
                report.planned.append(f"remove {target}")
                if not dry_run:
                    target.unlink()
        if profile.gamelist and not dry_run:
            stale_by_dir: dict[PurePosixPath, set[str]] = defaultdict(set)
            for rel in stale:
                p = PurePosixPath(rel)
                stale_by_dir[p.parent].add(f"./{p.name}")
            for d, paths in stale_by_dir.items():
                profiles.remove_from_gamelist(dest / d / "gamelist.xml", paths)

    if profile.gamelist and not dry_run:
        for sys_dir, entries in gamelists.items():
            profiles.write_gamelist(dest / sys_dir / "gamelist.xml", entries)

    if not dry_run:
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        files = sorted(written | (previous if not sync else set()))
        manifest_path.write_text(json.dumps({
            "collection": col.name, "profile": col.profile, "exported": time.time(),
            "files": files}, indent=1))
    report.warnings = sorted(set(report.warnings))
    if sel.over_budget:
        report.warnings.append(f"{len(sel.over_budget)} ROM(s) left out to fit the size budget")
    return report


def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if abs(n) < 1024 or unit == "TB":
            return f"{n:.1f} {unit}" if unit != "B" else f"{int(n)} B"
        n /= 1024
    return f"{n:.1f} TB"


def parse_size(text: str) -> int:
    m = re.fullmatch(r"\s*([\d.]+)\s*([kmgt]?)i?b?\s*", text, re.I)
    if not m:
        raise ValueError(f"Can't parse size '{text}' (try 32G, 500M)")
    mult = {"": 1, "k": 1024, "m": 1024 ** 2, "g": 1024 ** 3, "t": 1024 ** 4}[m.group(2).lower()]
    return int(float(m.group(1)) * mult)
