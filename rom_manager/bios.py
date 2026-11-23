"""BIOS / firmware files: what each system needs, checking and installing.

Hashes come from libretro's System.dat (libretro-database). "Required"
means games won't boot without the file in the usual RetroArch core used on
handhelds. "Optional" files improve compatibility or enable boot logos;
those cores have a built-in (HLE) replacement.
"""

from __future__ import annotations

import hashlib
import os
import shutil
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

from . import systems


@dataclass(frozen=True)
class BiosFile:
    name: str                    # path relative to the BIOS folder, e.g. "dc/dc_boot.bin"
    md5: tuple[str, ...] = ()    # accepted hashes; empty = any (name check only)
    size: int | None = None


@dataclass(frozen=True)
class Requirement:
    system: str
    files: tuple[BiosFile, ...]
    required: bool
    any_of: bool = False         # True: one of the files is enough (e.g. PS1 regions)
    note: str = ""


B = BiosFile
REQUIREMENTS: list[Requirement] = [
    Requirement("psx", (B("scph5501.bin", ("490f666e1afb15b7362b406ed1cea246",), 524288),
                        B("scph5500.bin", ("8dd7d5296a650fac7319bce665a6a53c",), 524288),
                        B("scph5502.bin", ("32736f17079d0b2b7024407c39bd3050",), 524288),
                        B("scph1001.bin", ("924e392ed05558ffdb115408c263dccf",), 524288)),
                True, any_of=True,
                note="PCSX ReARMed can run without one, but many games need a real BIOS. "
                     "scph5501 = USA, scph5500 = Japan, scph5502 = Europe."),
    Requirement("segacd", (B("bios_CD_U.bin", ("2efd74e3232ff260e371b99f84024f7f",), 131072),
                           B("bios_CD_E.bin", ("e66fa1dc5820d254611fdcdba0662372",), 131072),
                           B("bios_CD_J.bin", ("278a9397d192149e84e820ac621a8edd",), 131072)),
                True, any_of=True,
                note="Genesis Plus GX / PicoDrive use the BIOS matching each game's region "
                     "(U = USA, E = Europe, J = Japan); have at least the one for your games."),
    Requirement("saturn", (B("mpr-17933.bin", ("3240872c70984b6cbfda1586cab68dbe",), 524288),
                           B("sega_101.bin", ("85ec9ca47d8f6807718151cbcca8b964",), 524288),
                           B("saturn_bios.bin", ("af5828fdff51384f99b3c4926be27762",), 524288)),
                True, any_of=True,
                note="Beetle Saturn: mpr-17933 (US/EU) or sega_101 (JP). "
                     "YabaSanshiro: saturn_bios.bin."),
    Requirement("dreamcast", (B("dc/dc_boot.bin", ("e10c53c2f8b90bab96ead2d368858623",), 2097152),),
                True, note="Flycast. Some builds have an HLE BIOS, but compatibility is lower."),
    Requirement("pcenginecd", (B("syscard3.pce", ("38179df8f4ac870017db21ebcbf53114",
                                                  "0754f903b52e3b3342202bdafb13efa5"), 262144),),
                True, note="System Card 3.0 for Beetle PCE / PCE Fast."),
    Requirement("fds", (B("disksys.rom", ("ca30b50f880eb660a320674ed365ef7a",), 8192),), True),
    Requirement("atarilynx", (B("lynxboot.img", ("fcd403db69f54290b51035d82f835e7b",), 512),), True,
                note="Handy core."),
    Requirement("colecovision", (B("colecovision.rom", ("2c66f5911e5b42b8ebe113403548eee7",),
                                   8192),), True),
    Requirement("arcade", (B("neogeo.zip"),), False,
                note="Needed for Neo Geo games only. Must match your MAME/FBNeo romset "
                     "version, so only the name is checked."),
    Requirement("nds", (B("bios7.bin", ("df692a80a5b1bc90728bc3dfc76cd948",), 16384),
                        B("bios9.bin", ("a392174eb3e572fed6447e956bde4b25",), 4096),
                        B("firmware.bin", ("e45033d9b0fa6b0de071292bba7c9d13",), 262144)),
                False, note="DeSmuME runs without them; melonDS / DraStic benefit."),
    Requirement("gba", (B("gba_bios.bin", ("a860e8c0b6d573d191e4ec7db1b1e4f6",), 16384),), False,
                note="mGBA / gpSP have HLE BIOS; a few games need the real one."),
    Requirement("gb", (B("gb_bios.bin", ("32fbbd84168d3482956eb3c5051637f5",), 256),), False,
                note="Boot logo only."),
    Requirement("gbc", (B("gbc_bios.bin", ("dbfce9db9deaa2567f6a84fde55f9680",), 2304),), False,
                note="Boot logo only."),
    Requirement("pokemini", (B("bios.min", ("1e4fb124a3a886865acb574f388c803d",), 4096),), False),
    Requirement("atari7800", (B("7800 BIOS (U).rom", ("0763f1ffb006ddbe32e52d497ee848ae",), 4096),),
                False),
]

BY_MD5: dict[str, BiosFile] = {}
for _r in REQUIREMENTS:
    for _f in _r.files:
        for _m in _f.md5:
            BY_MD5.setdefault(_m, _f)
BY_NAME: dict[str, BiosFile] = {PurePosixPath(f.name).name.lower(): f
                                for r in REQUIREMENTS for f in r.files}


def requirements_for(system_ids) -> list[Requirement]:
    wanted = None if system_ids is None else set(system_ids)
    return [r for r in REQUIREMENTS if wanted is None or r.system in wanted]


def bios_dir_for(profile_id: str, system_id: str) -> PurePosixPath:
    """Where a firmware looks for BIOS files, relative to the SD card root."""
    if profile_id == "nextui":
        tag = systems.BY_ID[system_id].nextui
        tag = tag[tag.rfind("(") + 1:-1] if "(" in tag else system_id.upper()
        return PurePosixPath("Bios") / tag
    return PurePosixPath({
        "emulationstation": "roms/bios",
        "batocera": "bios",
        "esde": "RetroArch/system",
        "muos": "MUOS/bios",
        "onion": "BIOS",
    }.get(profile_id, "bios"))


def md5sum(path: Path) -> str:
    h = hashlib.md5()
    with open(path, "rb") as fh:
        while chunk := fh.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


# ------------------------------------------------------------------ checking
@dataclass
class FileStatus:
    file: BiosFile
    state: str          # ok | missing | bad_hash | misnamed
    found: Path | None = None
    detail: str = ""


@dataclass
class ReqStatus:
    req: Requirement
    files: list[FileStatus] = field(default_factory=list)

    @property
    def satisfied(self) -> bool:
        ok = [f.state == "ok" for f in self.files]
        return any(ok) if self.req.any_of else all(ok)


def _index_folder(folder: Path, max_size: int = 64 * 1024 * 1024):
    """Map md5 -> path and lowercase filename -> [paths] for files in a folder tree."""
    by_md5: dict[str, Path] = {}
    by_name: dict[str, list[Path]] = {}
    if not folder.exists():
        return by_md5, by_name
    for dirpath, _, files in os.walk(folder):
        for f in files:
            p = Path(dirpath) / f
            try:
                size = p.stat().st_size
            except OSError:
                continue
            by_name.setdefault(f.lower(), []).append(p)
            if size <= max_size:
                by_md5.setdefault(md5sum(p), p)
    return by_md5, by_name


def check(folder: Path, system_ids=None, layout: str | None = None) -> list[ReqStatus]:
    """Check which BIOS files are in `folder`.

    With ``layout`` (a profile id) the folder is treated as an SD card root
    and each system's BIOS folder for that firmware is checked; otherwise
    files are looked for anywhere under ``folder``.
    """
    out = []
    cache: dict[Path, tuple] = {}
    for req in requirements_for(system_ids):
        base = folder / bios_dir_for(layout, req.system) if layout else folder
        if base not in cache:
            cache[base] = _index_folder(base)
        by_md5, by_name = cache[base]
        rs = ReqStatus(req)
        for bf in req.files:
            expected = base / bf.name
            if expected.exists():
                if not bf.md5:
                    rs.files.append(FileStatus(bf, "ok", expected))
                    continue
                digest = md5sum(expected)
                if digest in bf.md5:
                    rs.files.append(FileStatus(bf, "ok", expected))
                else:
                    rs.files.append(FileStatus(bf, "bad_hash", expected,
                                               f"md5 {digest} doesn't match a known good dump"))
                continue
            hit = next((by_md5[m] for m in bf.md5 if m in by_md5), None)
            if hit is None and not layout:
                named = by_name.get(PurePosixPath(bf.name).name.lower())
                if named and (not bf.md5 or md5sum(named[0]) in bf.md5):
                    hit = named[0]
            if hit is not None and not layout:
                rs.files.append(FileStatus(bf, "ok", hit))
            elif hit is not None:
                rs.files.append(FileStatus(bf, "misnamed", hit,
                                           f"found as {hit.relative_to(folder)}; "
                                           f"rename/move to {bios_dir_for(layout, req.system)}/"
                                           f"{bf.name}"))
            else:
                rs.files.append(FileStatus(bf, "missing"))
        out.append(rs)
    return out


# ------------------------------------------------------------------ library
def import_files(sources: list[Path], dest: Path, dry_run: bool = False):
    """Copy recognised BIOS files (by hash, or by name for name-only entries) into dest.

    Returns a list of (source, outcome, detail).
    """
    results = []
    for src in sources:
        paths = [src] if src.is_file() else [Path(d) / f for d, _, fs in os.walk(src) for f in fs]
        for p in paths:
            try:
                if p.stat().st_size > 64 * 1024 * 1024:
                    continue
            except OSError:
                continue
            digest = md5sum(p)
            bf = BY_MD5.get(digest)
            if bf is None:
                named = BY_NAME.get(p.name.lower())
                if named and not named.md5:
                    bf = named
                elif named:
                    results.append((p, "bad_hash", f"named like {named.name} but md5 {digest} "
                                                   "isn't a known good dump"))
                    continue
                else:
                    continue
            target = dest / bf.name
            if target.exists() and md5sum(target) == digest:
                results.append((p, "present", str(target)))
                continue
            if not dry_run:
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(p, target)
            results.append((p, "imported", bf.name))
    return results


def install(library_bios: Path, card_root: Path, profile_id: str, system_ids,
            dry_run: bool = False) -> tuple[list[str], list[str], list[str]]:
    """Copy the BIOS files these systems use from the library to a card.

    Returns (copied, missing_required, missing_optional) as display strings.
    """
    copied, miss_req, miss_opt = [], [], []
    for rs in check(library_bios, system_ids):
        dest_dir = card_root / bios_dir_for(profile_id, rs.req.system)
        for fs in rs.files:
            if fs.state == "ok" and fs.found:
                target = dest_dir / fs.file.name
                if not (target.exists() and target.stat().st_size == fs.found.stat().st_size):
                    if not dry_run:
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copy2(fs.found, target)
                    copied.append(f"{rs.req.system}: {fs.file.name}")
        if not rs.satisfied:
            names = [f.file.name for f in rs.files if f.state != "ok"]
            label = f"{rs.req.system}: {' or '.join(names) if rs.req.any_of else ', '.join(names)}"
            (miss_req if rs.req.required else miss_opt).append(label)
    return copied, miss_req, miss_opt
