"""Content hashing used for duplicate detection and ScreenScraper lookups."""

from __future__ import annotations

import hashlib
import zipfile
import zlib
from pathlib import Path

CHUNK = 1024 * 1024


def file_hashes(path: Path) -> tuple[str, str, str]:
    """Return (crc32, md5, sha1) hex digests of a file."""
    crc = 0
    md5 = hashlib.md5()
    sha1 = hashlib.sha1()
    with open(path, "rb") as fh:
        while chunk := fh.read(CHUNK):
            crc = zlib.crc32(chunk, crc)
            md5.update(chunk)
            sha1.update(chunk)
    return f"{crc & 0xFFFFFFFF:08x}", md5.hexdigest(), sha1.hexdigest()


def content_crc(path: Path) -> tuple[str, int]:
    """CRC32 + size of the *game data*.

    For a zip holding a single file this is the inner file's CRC (read from
    the zip directory, no decompression), so "Tetris.zip" and "Tetris.gb"
    are recognised as the same game.
    """
    if path.suffix.lower() == ".zip":
        try:
            with zipfile.ZipFile(path) as zf:
                infos = [i for i in zf.infolist() if not i.is_dir()]
                if len(infos) == 1:
                    return f"{infos[0].CRC:08x}", infos[0].file_size
        except (zipfile.BadZipFile, OSError):
            pass
    crc = 0
    size = 0
    with open(path, "rb") as fh:
        while chunk := fh.read(CHUNK):
            crc = zlib.crc32(chunk, crc)
            size += len(chunk)
    return f"{crc & 0xFFFFFFFF:08x}", size
