"""Output layouts for handheld firmwares.

Each profile decides where ROMs go on the SD card, where artwork goes and
how it is named, and whether a gamelist.xml is written.
"""

from __future__ import annotations

import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

from .systems import System


@dataclass(frozen=True)
class Profile:
    id: str
    label: str
    devices: str
    roms_root: str                 # relative to SD card root
    art_kind_default: str = "boxart"
    gamelist: bool = False
    art_width: int | None = None   # resize art to this width if Pillow is installed
    notes: str = ""

    def rom_dir(self, system: System, overrides: dict) -> PurePosixPath:
        folder = overrides.get(system.id) or system.folder_for(self.id)
        return PurePosixPath(self.roms_root) / folder

    def art_path(self, system: System, rom_rel: PurePosixPath, overrides: dict,
                 kind: str) -> PurePosixPath | None:
        """Where artwork for a ROM (given its path on the card) should be written."""
        stem = rom_rel.stem
        sys_dir = rom_rel.parent
        if self.id in ("emulationstation", "batocera"):
            return sys_dir / "images" / f"{stem}.png"
        if self.id == "esde":
            sub = {"boxart": "covers", "snap": "screenshots", "title": "titlescreens",
                   "mix": "miximages", "wheel": "marquees"}.get(kind, "covers")
            folder = overrides.get(system.id) or system.folder_for("esde")
            return PurePosixPath("ES-DE/downloaded_media") / folder / sub / f"{stem}.png"
        if self.id == "onion":
            return sys_dir / "Imgs" / f"{stem}.png"
        if self.id == "nextui":
            return sys_dir / ".media" / f"{stem}.png"
        if self.id == "muos":
            cat = overrides.get(f"catalogue:{system.id}") or system.muos_catalogue
            sub = {"boxart": "box", "snap": "preview", "title": "preview", "mix": "box",
                   "wheel": "box"}.get(kind, "box")
            return PurePosixPath("MUOS/info/catalogue") / cat / sub / f"{stem}.png"
        return None


PROFILES: dict[str, Profile] = {p.id: p for p in [
    Profile("emulationstation", "EmulationStation (ArkOS, ROCKNIX, AmberELEC, dArkOS)",
            "Anbernic RG35XX/RG353 (ArkOS/ROCKNIX), R36S, Powkiddy, Odroid Go",
            "roms", "boxart", gamelist=True,
            notes="Art goes in roms/<system>/images and is referenced from gamelist.xml. "
                  "Some devices use a two-card layout; copy the roms folder to card 2 if so."),
    Profile("batocera", "Batocera / Knulli", "Knulli on Anbernic RG35XX family, Batocera PCs",
            "roms", "boxart", gamelist=True,
            notes="Batocera folder names (megadrive, lynx, wswan, mame...). "
                  "Place the 'roms' folder at /userdata/roms (SHARE partition)."),
    Profile("esde", "ES-DE (Android / Linux)", "Retroid Pocket, AYN Odin, Ayaneo, Steam Deck",
            "ROMs", "boxart", gamelist=False,
            notes="Point ES-DE's ROM directory at ROMs/ and its media directory at "
                  "ES-DE/downloaded_media on the card."),
    Profile("muos", "muOS", "Anbernic RG35XX/RG40XX/RG CubeXX, TrimUI (muOS)", "ROMS",
            "boxart", art_width=None,
            notes="Artwork goes in MUOS/info/catalogue/<System>/box. Assign cores to new folders "
                  "in muOS if they aren't auto-detected."),
    Profile("onion", "Onion OS", "Miyoo Mini / Mini+ / Mini Flip", "Roms", "boxart",
            art_width=250,
            notes="Art is resized to 250px wide (Onion's recommendation) when Pillow is "
                  "installed. Refresh ROM lists from the GameSwitcher menu after copying."),
    Profile("nextui", "NextUI / MinUI-style", "TrimUI Brick / Smart Pro, Miyoo (MinUI)",
            "Roms", "boxart", art_width=300,
            notes="Folders are named 'Console Name (TAG)'. Plain MinUI ignores the .media art."),
]}


def get(profile_id: str) -> Profile:
    try:
        return PROFILES[profile_id]
    except KeyError:
        raise KeyError(f"Unknown profile '{profile_id}'. Choose from: {', '.join(PROFILES)}")


# ------------------------------------------------------------------ gamelist
def write_gamelist(path: Path, entries: list[dict]):
    """Write/merge an EmulationStation gamelist.xml.

    ``entries`` are dicts with keys path, name, image (optional), hidden (optional).
    Existing entries for other games (and fields we don't manage, like play
    counts or favourites) are preserved.
    """
    if path.exists():
        try:
            tree = ET.parse(path)
            root = tree.getroot()
        except ET.ParseError:
            root = ET.Element("gameList")
    else:
        root = ET.Element("gameList")
    by_path = {g.findtext("path"): g for g in root.findall("game")}
    for e in entries:
        game = by_path.get(e["path"])
        if game is None:
            game = ET.SubElement(root, "game")
            ET.SubElement(game, "path").text = e["path"]
            by_path[e["path"]] = game
        for tag in ("name", "image", "thumbnail"):
            value = e.get(tag if tag != "thumbnail" else "image")
            if value is None:
                continue
            el = game.find(tag)
            if el is None:
                el = ET.SubElement(game, tag)
            el.text = value
        if e.get("hidden"):
            el = game.find("hidden")
            if el is None:
                el = ET.SubElement(game, "hidden")
            el.text = "true"
    ET.indent(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(root).write(path, encoding="utf-8", xml_declaration=True)


def remove_from_gamelist(path: Path, rom_paths: set[str]):
    if not path.exists():
        return
    try:
        tree = ET.parse(path)
    except ET.ParseError:
        return
    root = tree.getroot()
    for g in root.findall("game"):
        if g.findtext("path") in rom_paths:
            root.remove(g)
    ET.indent(root)
    tree.write(path, encoding="utf-8", xml_declaration=True)
