"""libretro-thumbnails: free, no account, matched by No-Intro / Redump filename.

Images are looked up by name, so this works best for ROMs named to the
No-Intro / Redump standard. A per-system index is downloaded once so that
slightly different names (other region, revision, missing tags) can still be
matched.
"""

from __future__ import annotations

import difflib
import json
import re
import time
import urllib.error
import urllib.parse
from pathlib import Path

from .. import naming
from . import Scraper, http_get

KIND_DIRS = {"boxart": "Named_Boxarts", "snap": "Named_Snaps", "title": "Named_Titles"}
BASE = "https://thumbnails.libretro.com"
GITHUB_RAW = "https://raw.githubusercontent.com/libretro-thumbnails/{repo}/master/{dir}/{name}.png"
GITHUB_TREE = "https://api.github.com/repos/libretro-thumbnails/{repo}/git/trees/master?recursive=1"
INDEX_MAX_AGE = 14 * 86400


def repo_name(playlist: str) -> str:
    return playlist.replace(" ", "_")


def _region_rank(regions: list[str], wanted: list[str], priority: list[str]) -> int:
    if set(regions) & set(wanted):
        return -1
    prio = {r: i for i, r in enumerate(priority)}
    return min((prio.get(r, len(prio)) for r in regions), default=len(prio))


def best_match(stem: str, names: list[str], cutoff: float = 0.92,
               priority: list[str] | None = None) -> str | None:
    """Choose the thumbnail name that best fits a ROM filename stem."""
    from ..filters import DEFAULT_REGION_PRIORITY
    priority = priority or DEFAULT_REGION_PRIORITY
    if not names:
        return None
    safe = naming.libretro_safe(stem)
    name_set = set(names)
    if safe in name_set:
        return safe
    no_disc = naming.libretro_safe(naming.strip_disc(stem))
    if no_disc in name_set:
        return no_disc

    target = naming.parse(stem)
    by_key: dict[str, list[str]] = {}
    for n in names:
        by_key.setdefault(naming.parse(n).key, []).append(n)

    candidates = by_key.get(target.key)
    if not candidates:
        close = difflib.get_close_matches(target.key, list(by_key), n=1, cutoff=cutoff)
        if not close:
            return None
        candidates = by_key[close[0]]

    def rank(n: str):
        p = naming.parse(n)
        return (
            len(p.flags - {"alternate"}),          # prefer retail art
            _region_rank(p.regions, target.regions, priority),
            p.disc != target.disc,
            -sum(t in target.tags for t in p.tags),  # most shared tags
            len(n),
        )
    return min(candidates, key=rank)


class LibretroScraper(Scraper):
    name = "libretro"
    kinds = tuple(KIND_DIRS)

    def __init__(self, cache_dir: Path, cutoff: float = 0.92, region_priority=None,
                 log=lambda *_: None):
        self.cache_dir = Path(cache_dir)
        self.cutoff = cutoff
        self.region_priority = region_priority
        self.log = log
        self._indexes: dict[tuple[str, str], list[str] | None] = {}

    # -------------------------------------------------------------- indexing
    def _fetch_index(self, playlist: str, kind_dir: str) -> list[str] | None:
        url = f"{BASE}/{urllib.parse.quote(playlist)}/{kind_dir}/"
        try:
            html = http_get(url, retries=1)
            if html:
                names = re.findall(r'href="([^"?/]+)\.png"', html.decode("utf-8", "replace"))
                if names:
                    return sorted({urllib.parse.unquote(n) for n in names})
        except (urllib.error.URLError, OSError):
            pass
        try:
            data = http_get(GITHUB_TREE.format(repo=repo_name(playlist)), retries=1)
            if data:
                tree = json.loads(data).get("tree", [])
                prefix = kind_dir + "/"
                return sorted(t["path"][len(prefix):-4] for t in tree
                              if t["path"].startswith(prefix) and t["path"].endswith(".png"))
        except (urllib.error.URLError, OSError, ValueError):
            pass
        return None

    def index(self, playlist: str, kind_dir: str) -> list[str] | None:
        key = (playlist, kind_dir)
        if key in self._indexes:
            return self._indexes[key]
        cache = self.cache_dir / "libretro" / f"{repo_name(playlist)}__{kind_dir}.json"
        names = None
        if cache.exists() and time.time() - cache.stat().st_mtime < INDEX_MAX_AGE:
            names = json.loads(cache.read_text())
        else:
            names = self._fetch_index(playlist, kind_dir)
            if names is not None:
                cache.parent.mkdir(parents=True, exist_ok=True)
                cache.write_text(json.dumps(names))
            else:
                self.log(f"  (no thumbnail index for {playlist}; using exact-name lookups only)")
        self._indexes[key] = names
        return names

    # -------------------------------------------------------------- fetching
    def _download(self, playlist: str, kind_dir: str, name: str) -> bytes | None:
        quoted = urllib.parse.quote(name)
        for url in (f"{BASE}/{urllib.parse.quote(playlist)}/{kind_dir}/{quoted}.png",
                    GITHUB_RAW.format(repo=repo_name(playlist), dir=kind_dir, name=quoted)):
            try:
                data = http_get(url, retries=1)
            except (urllib.error.URLError, OSError):
                continue
            if data and data[:8] == b"\x89PNG\r\n\x1a\n":
                return data
        return None

    def fetch(self, rom, system, kind: str) -> bytes | None:
        if kind not in KIND_DIRS or not system.libretro:
            return None
        kind_dir = KIND_DIRS[kind]
        names = self.index(system.libretro, kind_dir)
        if names is not None:
            match = best_match(rom.stem, names, self.cutoff, self.region_priority)
            return self._download(system.libretro, kind_dir, match) if match else None
        # No index available: try the exact name, then without disc number.
        for guess in dict.fromkeys([naming.libretro_safe(rom.stem),
                                    naming.libretro_safe(naming.strip_disc(rom.stem))]):
            data = self._download(system.libretro, kind_dir, guess)
            if data:
                return data
        return None
