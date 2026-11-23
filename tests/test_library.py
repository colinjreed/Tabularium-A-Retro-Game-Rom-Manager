import json

from rom_manager import filters
from rom_manager.detect import scan
from rom_manager.library import Library


def import_all(lib, src):
    sets, skipped = scan([src])
    results = {}
    for rs in sets:
        if rs.system:
            results[rs.main.name] = lib.add(rs)[0]
        else:
            results[rs.main.name] = "unknown"
    return results, skipped


def test_detection_and_import(lib, messy):
    results, skipped = import_all(lib, messy)
    systems = {r.stem: r.system for r in lib.roms()}
    assert systems["mario"] == "gba"                      # zip contents
    assert systems["Sonic The Hedgehog (USA, Europe)"] == "genesis"   # header
    assert systems["Pitfall! (USA)"] == "atari2600"       # folder name
    assert systems["Final Fantasy VII (USA) (Disc 1)"] == "psx"       # disc header
    assert systems["Tetris DX (World) (Hack)"] == "gbc"
    # byte-identical zip of Tetris is recognised as a duplicate
    assert sorted(results.values()).count("duplicate") == 1
    assert results["mystery.bin"] == "unknown"
    assert any(p.name == "readme.txt" for p in skipped)
    # cue sheets carry their .bin along
    ff7 = next(r for r in lib.roms() if r.stem.endswith("(Disc 1)"))
    assert [f.split("/")[-1] for f in ff7.files] == [
        "Final Fantasy VII (USA) (Disc 1).cue", "Final Fantasy VII (USA) (Disc 1).bin"]
    assert (lib.root / "roms" / "psx" / "Final Fantasy VII (USA) (Disc 1).bin").exists()


def test_reimport_is_idempotent(lib, messy):
    import_all(lib, messy)
    n = len(lib.roms())
    results, _ = import_all(lib, messy)
    assert len(lib.roms()) == n
    assert "added" not in results.values()


def test_clean_1g1r_and_junk(lib, messy):
    import_all(lib, messy)
    spec = filters.FilterSpec(exclude_flags=filters.DEFAULT_EXCLUDE + ["hack"],
                              one_game_one_rom=True)
    kept, rejected = filters.apply(lib.roms(), spec)
    stems = {r.stem for r in kept}
    assert "Tetris (World) (Rev 1)" in stems
    assert "Tetris (Japan)" not in stems
    assert "Tetris (World) (Beta)" not in stems
    assert "Tetris DX (World) (Hack)" not in stems
    assert "Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2)" in stems
    assert "Super Mario Land 2 - 6 Golden Coins (USA, Europe)" not in stems
    # both discs of the chosen release are kept
    assert {"Final Fantasy VII (USA) (Disc 1)", "Final Fantasy VII (USA) (Disc 2)"} <= stems

    for rom, why in rejected:
        lib.trash(rom, why)
    assert not (lib.root / "roms" / "gb" / "Tetris (Japan).gb").exists()
    assert (lib.root / "trash" / "gb" / "Tetris (Japan).gb").exists()
    trashed = lib.roms(status="trashed")
    lib.restore(next(r for r in trashed if r.stem == "Tetris (Japan)"))
    assert (lib.root / "roms" / "gb" / "Tetris (Japan).gb").exists()


def test_region_priority_japan_first(lib, messy):
    import_all(lib, messy)
    spec = filters.FilterSpec(one_game_one_rom=True, region_priority=["Japan", "USA", "World"])
    kept, _ = filters.apply(lib.roms(["gb"]), spec)
    assert "Tetris (Japan)" in {r.stem for r in kept}


def test_reopen_library(lib, messy, tmp_path):
    import_all(lib, messy)
    again = Library(lib.root)
    assert len(again.roms()) == len(lib.roms())
    assert json.loads(again.db.execute("SELECT files FROM roms LIMIT 1").fetchone()[0])
