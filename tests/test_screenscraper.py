import json
import urllib.parse

import pytest

from rom_manager import systems
from rom_manager.scrapers import ScrapeError, screenscraper
from rom_manager.scrapers.screenscraper import ScreenScraper, pick_media


class R:
    id = 1
    path = "roms/gb/Tetris (World) (Rev 1).gb"
    stem = "Tetris (World) (Rev 1)"
    crc = "46df91ad"
    content_size = 32768


def test_requires_dev_credentials(monkeypatch):
    for k in ("SCREENSCRAPER_DEVID", "SCREENSCRAPER_DEVPASSWORD"):
        monkeypatch.delenv(k, raising=False)
    with pytest.raises(ScrapeError):
        ScreenScraper()


def test_pick_media_prefers_us_then_world():
    medias = [{"type": "box-2D", "region": "jp", "url": "J"},
              {"type": "box-2D", "region": "wor", "url": "W"},
              {"type": "ss", "region": "us", "url": "S"},
              {"type": "mixrbv1", "region": "us", "url": "M1"}]
    assert pick_media(medias, "boxart") == "W"
    assert pick_media(medias, "snap") == "S"
    assert pick_media(medias, "mix") == "M1"
    assert pick_media(medias, "wheel") is None


def test_fetch_queries_by_crc_and_downloads(monkeypatch):
    calls = []

    def fake_get(url, timeout=30, retries=2):
        calls.append(url)
        if "jeuInfos" in url:
            return json.dumps({"response": {"jeu": {"medias": [
                {"type": "box-2D", "region": "us", "url": "https://media/box"},
                {"type": "ss", "region": "us", "url": "https://media/ss"}]}}}).encode()
        return b"IMG:" + url.encode()
    monkeypatch.setattr(screenscraper, "http_get", fake_get)
    ss = ScreenScraper({"devid": "d", "devpassword": "p", "ssid": "u", "sspassword": "x"},
                       min_interval=0)
    gb = systems.get("gb")
    assert ss.fetch(R(), gb, "boxart") == b"IMG:https://media/box"
    assert ss.fetch(R(), gb, "snap") == b"IMG:https://media/ss"
    api_calls = [c for c in calls if "jeuInfos" in c]
    assert len(api_calls) == 1  # one API lookup per ROM, reused for every art type
    q = urllib.parse.parse_qs(urllib.parse.urlparse(api_calls[0]).query)
    assert q["crc"] == ["46DF91AD"] and q["systemeid"] == ["9"] and q["romtaille"] == ["32768"]


def test_not_found_returns_none(monkeypatch):
    monkeypatch.setattr(screenscraper, "http_get",
                        lambda *a, **k: "Erreur : Rom/Iso/Dossier non trouvée !".encode())
    ss = ScreenScraper({"devid": "d", "devpassword": "p"}, min_interval=0)
    assert ss.fetch(R(), systems.get("gb"), "boxart") is None
