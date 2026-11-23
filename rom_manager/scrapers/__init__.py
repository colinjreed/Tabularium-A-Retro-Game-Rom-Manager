"""Artwork scrapers."""

from __future__ import annotations

import time
import urllib.error
import urllib.request

USER_AGENT = "rom-manager/0.1"

ART_KINDS = ("boxart", "snap", "title", "mix", "wheel")


class ScrapeError(Exception):
    """A fatal scraper problem (bad credentials, quota exhausted): stop scraping."""


def http_get(url: str, timeout: float = 30, retries: int = 2) -> bytes | None:
    """GET a URL. Returns None on 404; raises urllib errors after retries otherwise."""
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(retries + 1):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return resp.read()
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code in (429, 500, 502, 503, 504) and attempt < retries:
                time.sleep(2 ** attempt)
                continue
            raise
        except (urllib.error.URLError, TimeoutError):
            if attempt < retries:
                time.sleep(2 ** attempt)
                continue
            raise
    return None


class Scraper:
    name = "base"
    kinds: tuple[str, ...] = ()

    def fetch(self, rom, system, kind: str) -> bytes | None:  # pragma: no cover - interface
        raise NotImplementedError
