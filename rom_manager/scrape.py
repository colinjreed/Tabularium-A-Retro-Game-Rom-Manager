"""Download artwork for library ROMs into the library's media cache."""

from __future__ import annotations

from dataclasses import dataclass, field

from . import systems
from .library import Library
from .scrapers import ScrapeError, Scraper


@dataclass
class ScrapeReport:
    found: int = 0
    missing: int = 0
    skipped: int = 0
    missing_list: list[str] = field(default_factory=list)
    stopped: str | None = None


def _ext_for(data: bytes) -> str:
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return ".png"
    if data[:3] == b"\xff\xd8\xff":
        return ".jpg"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return ".webp"
    return ".png"


def scrape_library(lib: Library, scrapers: list[Scraper], kinds: list[str],
                   system_ids: list[str] | None = None, refresh: bool = False,
                   log=print) -> ScrapeReport:
    report = ScrapeReport()
    roms = lib.roms(system_ids)
    total = len(roms)
    for i, rom in enumerate(roms, 1):
        system = systems.BY_ID.get(rom.system)
        if system is None:
            continue
        for kind in kinds:
            if not refresh and lib.media_attempted(rom.id, kind):
                report.skipped += 1
                continue
            data, source = None, None
            for sc in scrapers:
                if kind not in sc.kinds:
                    continue
                try:
                    data = sc.fetch(rom, system, kind)
                except ScrapeError as e:
                    report.stopped = str(e)
                    log(f"Stopping: {e}")
                    return report
                except Exception as e:  # network hiccup on one ROM shouldn't stop the run
                    log(f"  {sc.name} error on {rom.stem}: {e}")
                    data = None
                if data:
                    source = sc.name
                    break
            if data:
                dest = lib.media_dir(rom.system, kind) / (rom.stem + _ext_for(data))
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(data)
                lib.set_media(rom.id, kind, dest, source)
                report.found += 1
                log(f"[{i}/{total}] {kind:6} ✓ {rom.system}/{rom.stem}  ({source})")
            else:
                lib.set_media(rom.id, kind, None, None)
                report.missing += 1
                report.missing_list.append(f"{rom.system}/{rom.stem} [{kind}]")
                log(f"[{i}/{total}] {kind:6} ✗ {rom.system}/{rom.stem}")
    return report
