"""ScreenScraper.fr: matched by file hash, needs an account.

ScreenScraper identifies games by CRC/size/filename, so it finds art for
badly-named ROMs and arcade sets that libretro's name matching misses. It
also offers "mix" images (box + screenshot + logo composites) and wheels,
which many handheld themes use.

Its API needs two sets of credentials:
  * developer credentials (SCREENSCRAPER_DEVID / SCREENSCRAPER_DEVPASSWORD),
    requested from ScreenScraper for the software making the calls, and
  * your user account (SCREENSCRAPER_USER / SCREENSCRAPER_PASSWORD), which
    sets your daily quota and number of threads.
"""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.parse

from . import ScrapeError, Scraper, http_get

API = "https://api.screenscraper.fr/api2/jeuInfos.php"

MEDIA_TYPES = {
    "boxart": ["box-2D"],
    "snap": ["ss"],
    "title": ["sstitle"],
    "mix": ["mixrbv2", "mixrbv1"],
    "wheel": ["wheel", "wheel-hd"],
}

REGION_ORDER = ["us", "wor", "eu", "uk", "ss", "jp", "cus"]


def credentials_from_env() -> dict:
    return {
        "devid": os.environ.get("SCREENSCRAPER_DEVID", ""),
        "devpassword": os.environ.get("SCREENSCRAPER_DEVPASSWORD", ""),
        "ssid": os.environ.get("SCREENSCRAPER_USER", ""),
        "sspassword": os.environ.get("SCREENSCRAPER_PASSWORD", ""),
    }


def pick_media(medias: list[dict], kind: str) -> str | None:
    for mtype in MEDIA_TYPES.get(kind, []):
        options = [m for m in medias if m.get("type") == mtype and m.get("url")]
        if not options:
            continue

        def rank(m):
            region = m.get("region", "")
            return REGION_ORDER.index(region) if region in REGION_ORDER else len(REGION_ORDER)
        return min(options, key=rank)["url"]
    return None


class ScreenScraper(Scraper):
    name = "screenscraper"
    kinds = tuple(MEDIA_TYPES)

    def __init__(self, creds: dict | None = None, min_interval: float = 1.0,
                 max_width: int | None = None, log=lambda *_: None):
        self.creds = creds or credentials_from_env()
        if not (self.creds.get("devid") and self.creds.get("devpassword")):
            raise ScrapeError(
                "ScreenScraper needs developer credentials: set SCREENSCRAPER_DEVID and "
                "SCREENSCRAPER_DEVPASSWORD (and SCREENSCRAPER_USER / SCREENSCRAPER_PASSWORD "
                "for your account's quota).")
        self.min_interval = min_interval
        self.max_width = max_width
        self.log = log
        self._last = 0.0
        self._games: dict[int, dict | None] = {}

    def _query(self, rom, system) -> dict | None:
        if rom.id in self._games:
            return self._games[rom.id]
        params = {
            "devid": self.creds["devid"], "devpassword": self.creds["devpassword"],
            "softname": "rom-manager", "output": "json",
            "ssid": self.creds.get("ssid", ""), "sspassword": self.creds.get("sspassword", ""),
            "romtype": "rom", "romnom": os.path.basename(rom.path),
        }
        if system.screenscraper_id:
            params["systemeid"] = system.screenscraper_id
        if rom.crc:
            params["crc"] = rom.crc.upper()
        if rom.content_size:
            params["romtaille"] = rom.content_size
        wait = self.min_interval - (time.time() - self._last)
        if wait > 0:
            time.sleep(wait)
        self._last = time.time()
        url = API + "?" + urllib.parse.urlencode(params)
        try:
            data = http_get(url, retries=2)
        except urllib.error.HTTPError as e:
            if e.code in (401, 403):
                raise ScrapeError(f"ScreenScraper refused the credentials (HTTP {e.code}).")
            if e.code in (423, 426, 429, 430, 431):
                raise ScrapeError(f"ScreenScraper quota/availability problem (HTTP {e.code}); "
                                  "try again later.")
            self.log(f"  ScreenScraper HTTP {e.code} for {rom.stem}")
            data = None
        game = None
        if data:
            try:
                game = json.loads(data).get("response", {}).get("jeu")
            except ValueError:
                game = None  # ScreenScraper answers "not found" with plain text
        self._games[rom.id] = game
        return game

    def fetch(self, rom, system, kind: str) -> bytes | None:
        game = self._query(rom, system)
        if not game:
            return None
        url = pick_media(game.get("medias", []), kind)
        if not url:
            return None
        if self.max_width:
            url += ("&" if "?" in url else "?") + f"maxwidth={self.max_width}"
        try:
            return http_get(url)
        except (urllib.error.URLError, OSError):
            return None
