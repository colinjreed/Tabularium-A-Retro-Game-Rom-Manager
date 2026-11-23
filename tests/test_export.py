import xml.etree.ElementTree as ET

from rom_manager import bios, filters
from rom_manager.collection import Collection, export, parse_size, resolve
from rom_manager.scrapers import libretro
from rom_manager.scrape import scrape_library

from test_library import import_all

def make_png(w, h):
    import struct
    import zlib

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    raw = b"".join(b"\x00" + b"\x80\x40\x20" * w for _ in range(h))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


PNG = make_png(600, 400)


class FakeScraper:
    name = "fake"
    kinds = ("boxart",)

    def fetch(self, rom, system, kind):
        return None if "Pitfall" in rom.stem else PNG


def setup(lib, messy, profile, **kw):
    import_all(lib, messy)
    scrape_library(lib, [FakeScraper()], ["boxart"], log=lambda *_: None)
    col = Collection(name="test", profile=profile,
                     filters=filters.FilterSpec(exclude_flags=filters.DEFAULT_EXCLUDE + ["hack"],
                                                one_game_one_rom=True).to_dict(), **kw)
    col.save(lib)
    return Collection.load(lib, "test")


def test_export_onion(lib, messy, tmp_path):
    col = setup(lib, messy, "onion")
    card = tmp_path / "card"
    card.mkdir()
    rep = export(lib, col, card)
    assert (card / "Roms/GB/Tetris (World) (Rev 1).gb").exists()
    img = card / "Roms/GB/Imgs/Tetris (World) (Rev 1).png"
    from PIL import Image
    with Image.open(img) as im:
        assert im.size == (250, 167)  # Onion profile resizes to 250px wide
    assert not (card / "Roms/GB/Tetris (Japan).gb").exists()
    assert (card / "Roms/MD/Sonic The Hedgehog (USA, Europe).bin").exists()
    # multi-disc -> m3u + hidden disc folder
    m3u = card / "Roms/PS/Final Fantasy VII (USA).m3u"
    assert m3u.read_text().splitlines() == [
        ".Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 1).cue",
        ".Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 2).cue"]
    assert (card / "Roms/PS/.Final Fantasy VII (USA)/Final Fantasy VII (USA) (Disc 2).bin").exists()
    assert (card / "Roms/PS/Imgs/Final Fantasy VII (USA).png").exists()
    assert rep.art_missing == 1  # Pitfall
    # PS1 BIOS is required and the library has none
    assert any(m.startswith("psx") for m in rep.bios_missing_required)
    # second export copies nothing
    rep2 = export(lib, col, card)
    assert rep2.copied == 0 and rep2.unchanged > 0


def test_export_emulationstation_gamelist_and_sync(lib, messy, tmp_path):
    col = setup(lib, messy, "emulationstation")
    card = tmp_path / "card"
    card.mkdir()
    export(lib, col, card)
    gl = ET.parse(card / "roms/gb/gamelist.xml").getroot()
    games = {g.findtext("path"): g for g in gl.findall("game")}
    assert games["./Tetris (World) (Rev 1).gb"].findtext("name") == "Tetris"
    assert games["./Tetris (World) (Rev 1).gb"].findtext("image") == \
        "./images/Tetris (World) (Rev 1).png"
    # a user's own file on the card must survive --sync
    (card / "roms/gb/my-homebrew.gb").write_bytes(b"x")
    # narrow the collection to GBA and sync: GB files we wrote are removed
    col.systems = ["gba"]
    rep = export(lib, col, card, sync=True)
    assert rep.removed > 0
    assert not (card / "roms/gb/Tetris (World) (Rev 1).gb").exists()
    assert (card / "roms/gb/my-homebrew.gb").exists()
    assert (card / "roms/gba/mario.zip").exists()
    assert not ET.parse(card / "roms/gb/gamelist.xml").getroot().findall("game")


def test_size_budget_keeps_discs_together(lib, messy):
    col = setup(lib, messy, "nextui", max_size=parse_size("150K"))
    sel = resolve(lib, col)
    assert sel.size <= col.max_size
    ff7 = [r for r in sel.roms if r.system == "psx"]
    assert len(ff7) in (0, 2)
    assert sel.over_budget


def test_nextui_and_muos_paths(lib, messy, tmp_path):
    col = setup(lib, messy, "nextui")
    export(lib, col, tmp_path / "n", force=True)
    assert (tmp_path / "n/Roms/Game Boy (GB)/.media/Tetris (World) (Rev 1).png").exists()
    col = setup(lib, messy, "muos")
    export(lib, col, tmp_path / "m", force=True)
    assert (tmp_path / "m/ROMS/gb/Tetris (World) (Rev 1).gb").exists()
    assert (tmp_path / "m/MUOS/info/catalogue/Nintendo Game Boy/box/Tetris (World) (Rev 1).png").exists()


def test_libretro_matching():
    names = ["Tetris (World) (Rev 1)", "Tetris (Japan)", "Tetris 2 (USA)",
             "Legend of Zelda, The - Link's Awakening (USA, Europe) (Rev 2)",
             "Final Fantasy VII (USA) (Disc 1)", "Q_bert (USA)"]
    m = libretro.best_match
    assert m("Tetris (World) (Rev 1)", names) == "Tetris (World) (Rev 1)"
    assert m("Tetris (USA)", names) == "Tetris (World) (Rev 1)"
    assert m("Tetris (Japan) (Rev A)", names) == "Tetris (Japan)"
    assert m("The Legend of Zelda - Link's Awakening (Europe)", names).startswith("Legend of Zelda")
    assert m("Q*bert (USA)", names) == "Q_bert (USA)"
    assert m("Completely Different Game (USA)", names) is None


def test_bios_check_and_install(tmp_path, monkeypatch):
    lib_bios = tmp_path / "libbios"
    lib_bios.mkdir()
    good = b"fake psx bios"
    digest = bios.md5sum((tmp_path / "x").write_bytes(good) and tmp_path / "x")
    req = bios.Requirement("psx", (bios.BiosFile("scph5501.bin", (digest,)),
                                   bios.BiosFile("scph1001.bin", ("0" * 32,))), True, any_of=True)
    monkeypatch.setattr(bios, "REQUIREMENTS", [req])
    monkeypatch.setattr(bios, "BY_MD5", {digest: req.files[0]})
    monkeypatch.setattr(bios, "BY_NAME", {})

    # a renamed BIOS is recognised by hash on import
    (tmp_path / "dl").mkdir()
    (tmp_path / "dl" / "PSX USA BIOS.bin").write_bytes(good)
    res = bios.import_files([tmp_path / "dl"], lib_bios)
    assert res[0][1] == "imported" and (lib_bios / "scph5501.bin").exists()
    assert bios.check(lib_bios, ["psx"])[0].satisfied

    card = tmp_path / "card"
    copied, miss_req, _ = bios.install(lib_bios, card, "onion", ["psx"])
    assert (card / "BIOS/scph5501.bin").exists() and not miss_req
    # card check with layout flags a file in the wrong place
    (card / "BIOS/scph5501.bin").rename(card / "psx.bin")
    st = bios.check(card, ["psx"], layout="onion")[0]
    assert not st.satisfied
    assert bios.bios_dir_for("nextui", "psx").as_posix() == "Bios/PS"
