import hashlib
import os
import zipfile
from pathlib import Path

import pytest

from rom_manager.library import Library


def write(path: Path, data: bytes) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return path


def rand(n: int, seed: str) -> bytes:
    out = b""
    i = 0
    while len(out) < n:
        out += hashlib.sha256(f"{seed}{i}".encode()).digest()
        i += 1
    return out[:n]


def genesis_rom(seed: str) -> bytes:
    data = bytearray(rand(0x20000, seed))
    data[0x100:0x110] = b"SEGA GENESIS    "
    return bytes(data)


def psx_bin(seed: str) -> bytes:
    """Raw 2352-byte-sector image with an ISO9660 PVD that says PLAYSTATION."""
    data = bytearray(rand(20 * 2352, seed))
    off = 16 * 2352 + 24
    data[off:off + 6] = b"\x01CD001"
    data[off + 8:off + 8 + 32] = b"PLAYSTATION".ljust(32)
    return bytes(data)


@pytest.fixture
def messy(tmp_path):
    """A messy source collection like people actually have."""
    src = tmp_path / "downloads"
    # Game Boy: two regions of one game, a beta, a hack, and a byte-identical zipped copy
    tetris = rand(32768, "tetris")
    write(src / "random" / "Tetris (World) (Rev 1).gb", tetris)
    write(src / "random" / "Tetris (Japan).gb", rand(32768, "tetris-jp"))
    write(src / "random" / "Tetris (World) (Beta).gb", rand(32768, "tetris-beta"))
    write(src / "random" / "Tetris DX (World) (Hack).gbc", rand(32768, "tdx-hack"))
    zp = src / "zips" / "Tetris (World) (Rev 1).zip"
    zp.parent.mkdir(parents=True)
    with zipfile.ZipFile(zp, "w") as zf:
        zf.writestr("Tetris (World) (Rev 1).gb", tetris)
    # GBA in a zip with a non-obvious name
    with zipfile.ZipFile(src / "zips" / "mario.zip", "w") as zf:
        zf.writestr("Super Mario Advance (USA).gba", rand(65536, "sma"))
    # Revisions: Rev 2 should win
    write(src / "gb" / "Super Mario Land 2 - 6 Golden Coins (USA, Europe).gb", rand(4096, "sml2"))
    write(src / "gb" / "Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2).gb",
          rand(4096, "sml2r2"))
    # Genesis .bin detected by header, Atari .bin by folder name
    write(src / "misc" / "Sonic The Hedgehog (USA, Europe).bin", genesis_rom("sonic"))
    write(src / "Atari 2600" / "Pitfall! (USA).bin", rand(4096, "pitfall"))
    # PS1 multi-disc with cue sheets, detected from the disc header
    for d in (1, 2):
        stem = f"Final Fantasy VII (USA) (Disc {d})"
        write(src / "discs" / f"{stem}.bin", psx_bin(f"ff7-{d}"))
        (src / "discs" / f"{stem}.cue").write_text(
            f'FILE "{stem}.bin" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n')
    # Junk that should be ignored or left unknown
    write(src / "random" / "readme.txt", b"hello")
    write(src / "random" / "mystery.bin", rand(100000, "mystery"))
    return src


@pytest.fixture
def lib(tmp_path):
    return Library.create(tmp_path / "library")


@pytest.fixture(autouse=True)
def no_network(monkeypatch):
    import urllib.request

    def blocked(*a, **k):
        raise AssertionError("tests must not touch the network")
    monkeypatch.setattr(urllib.request, "urlopen", blocked)
    os.environ.pop("ROM_MANAGER_LIBRARY", None)
