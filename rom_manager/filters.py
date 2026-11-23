"""Filtering and 1G1R (one game, one ROM) selection.

The same rules are used by `clean` (to weed the library) and by collections
(to choose what goes on an SD card without touching the library).
"""

from __future__ import annotations

import fnmatch
from collections import defaultdict
from dataclasses import dataclass, field

from . import naming
from .library import Rom

DEFAULT_REGION_PRIORITY = ["USA", "World", "Europe", "UK", "Australia", "Canada", "Japan"]

# Sensible defaults for "remove the junk".
DEFAULT_EXCLUDE = ["beta", "proto", "demo", "sample", "kiosk", "program", "bios", "bad",
                   "overdump", "trainer", "pirate"]


@dataclass
class FilterSpec:
    exclude_flags: list[str] = field(default_factory=list)
    regions: list[str] = field(default_factory=list)        # keep only these (empty = any)
    languages: list[str] = field(default_factory=list)      # keep only these (empty = any)
    include_titles: list[str] = field(default_factory=list)  # glob patterns
    exclude_titles: list[str] = field(default_factory=list)
    one_game_one_rom: bool = False
    region_priority: list[str] = field(default_factory=lambda: list(DEFAULT_REGION_PRIORITY))

    @classmethod
    def from_dict(cls, d: dict) -> "FilterSpec":
        known = {k: v for k, v in d.items() if k in cls.__dataclass_fields__}
        return cls(**known)

    def to_dict(self) -> dict:
        return {k: getattr(self, k) for k in self.__dataclass_fields__}


def _match_any(text: str, patterns: list[str]) -> bool:
    t = text.lower()
    return any(fnmatch.fnmatch(t, p.lower()) for p in patterns)


def rejection_reason(rom: Rom, spec: FilterSpec) -> str | None:
    """Why a single ROM fails the filters (ignoring 1G1R), or None if it passes."""
    hit = rom.flags & set(spec.exclude_flags)
    if hit:
        return "flag: " + ",".join(sorted(hit))
    if spec.regions and rom.regions:
        wanted = {r.lower() for r in spec.regions}
        if not any(r.lower() in wanted for r in rom.regions):
            return "region: " + ",".join(rom.regions)
    if spec.languages and rom.languages:
        wanted = {lang.lower() for lang in spec.languages}
        if not any(lang in wanted for lang in rom.languages):
            return "language: " + ",".join(rom.languages)
    names = [rom.title, rom.stem]
    if spec.include_titles and not any(_match_any(n, spec.include_titles) for n in names):
        return "not in include list"
    if spec.exclude_titles and any(_match_any(n, spec.exclude_titles) for n in names):
        return "title excluded"
    return None


def score(rom: Rom, priority: list[str]) -> tuple:
    """Higher is better. Used to pick the single best ROM of a game."""
    prio = {r.lower(): i for i, r in enumerate(priority)}
    region_rank = min((prio.get(r.lower(), len(prio)) for r in rom.regions), default=len(prio))
    bad = len(rom.flags - {"alternate", "fixed"})
    return (
        -bad,
        -region_rank,
        "translation" not in rom.flags,
        rom.verified,
        rom.revision,
        "alternate" not in rom.flags,
        -len(rom.stem),
        -rom.id,
    )


def variant_key(rom: Rom) -> str:
    """Discs of the same release share a variant key."""
    return naming.strip_disc(rom.stem).lower()


def pick_1g1r(roms: list[Rom], priority: list[str]) -> tuple[list[Rom], dict[int, Rom]]:
    """Return (kept, dropped) where dropped maps rom id -> the ROM that replaced it."""
    groups: dict[tuple, list[Rom]] = defaultdict(list)
    for r in roms:
        groups[(r.system, r.key)].append(r)
    kept: list[Rom] = []
    dropped: dict[int, Rom] = {}
    for members in groups.values():
        variants: dict[str, list[Rom]] = defaultdict(list)
        for r in members:
            variants[variant_key(r)].append(r)
        # Score a variant by its first disc (all discs share the same tags).
        best_key = max(variants, key=lambda k: score(min(variants[k], key=lambda r: r.disc or 0),
                                                     priority))
        winner = min(variants[best_key], key=lambda r: r.disc or 0)
        for k, rs in variants.items():
            if k == best_key:
                kept.extend(rs)
            else:
                for r in rs:
                    dropped[r.id] = winner
    kept.sort(key=lambda r: (r.system, r.title, r.disc or 0))
    return kept, dropped


def exact_duplicates(roms: list[Rom]) -> dict[int, Rom]:
    """ROMs whose data is byte-identical to another; maps dropped id -> kept ROM."""
    groups: dict[tuple, list[Rom]] = defaultdict(list)
    for r in roms:
        if r.crc:
            groups[(r.system, r.crc, r.content_size)].append(r)
    out = {}
    for members in groups.values():
        if len(members) > 1:
            best = max(members, key=lambda r: score(r, DEFAULT_REGION_PRIORITY))
            for r in members:
                if r is not best:
                    out[r.id] = best
    return out


def apply(roms: list[Rom], spec: FilterSpec) -> tuple[list[Rom], list[tuple[Rom, str]]]:
    """Run the full filter pipeline. Returns (kept, [(rejected_rom, reason), ...])."""
    rejected: list[tuple[Rom, str]] = []
    survivors = []
    for r in roms:
        why = rejection_reason(r, spec)
        if why:
            rejected.append((r, why))
        else:
            survivors.append(r)
    dupes = exact_duplicates(survivors)
    if dupes:
        rejected += [(r, f"exact duplicate of {dupes[r.id].stem}") for r in survivors
                     if r.id in dupes]
        survivors = [r for r in survivors if r.id not in dupes]
    if spec.one_game_one_rom:
        survivors, dropped = pick_1g1r(survivors, spec.region_priority)
        rejected += [(r, f"1G1R: kept {w.stem}") for r in roms if r.id in dropped
                     for w in [dropped[r.id]]]
    return survivors, rejected
