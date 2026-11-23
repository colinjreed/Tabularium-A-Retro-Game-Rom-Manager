"""System definitions: file extensions, aliases and per-firmware folder names.

Folder names are the defaults used by each firmware. Firmware versions do
change these, so every value can be overridden with a JSON config file
(see ``config.py``).
"""

from __future__ import annotations

import re
from dataclasses import dataclass

# Extensions that more than one system uses. Files with these extensions
# need folder hints, archive inspection or header sniffing to place.
AMBIGUOUS_EXTS = {
    ".bin", ".iso", ".img", ".cue", ".chd", ".zip", ".7z", ".rom", ".m3u",
    ".ccd", ".mdf", ".toc", ".cdi", ".gdi", ".pbp", ".cso", ".rvz",
}

DISC_EXTS = {".cue", ".chd", ".iso", ".img", ".ccd", ".mdf", ".toc", ".m3u", ".bin"}


@dataclass(frozen=True)
class System:
    id: str
    name: str
    libretro: str | None          # libretro-thumbnails playlist name
    exts: tuple[str, ...]          # extensions unique to this system
    extra_exts: tuple[str, ...] = ()   # shared extensions this system accepts
    aliases: tuple[str, ...] = ()
    es: str = ""                   # EmulationStation / ArkOS / ROCKNIX / ES-DE
    batocera: str = ""             # Batocera / Knulli
    onion: str = ""                # Onion OS (Miyoo Mini)
    nextui: str = ""               # MinUI / NextUI style "Name (TAG)"
    muos_catalogue: str = ""       # muOS artwork catalogue folder
    screenscraper_id: int | None = None
    disc_based: bool = False

    def accepts(self, ext: str) -> bool:
        ext = ext.lower()
        return ext in self.exts or ext in self.extra_exts

    def folder_for(self, profile: str) -> str:
        return {
            "emulationstation": self.es,
            "esde": self.es,
            "batocera": self.batocera or self.es,
            "onion": self.onion or self.es.upper(),
            "nextui": self.nextui or self.name,
            "muos": self.es,
            "library": self.id,
        }.get(profile, self.es)


CD = (".cue", ".chd", ".iso", ".img", ".ccd", ".mdf", ".toc", ".m3u", ".bin")

SYSTEMS: list[System] = [
    System("nes", "Nintendo Entertainment System", "Nintendo - Nintendo Entertainment System",
           (".nes", ".unf", ".unif"), (".zip", ".7z"),
           ("famicom", "fc", "nes"), "nes", "nes", "FC",
           "Nintendo Entertainment System (FC)", "Nintendo NES - Famicom", 3),
    System("fds", "Famicom Disk System", "Nintendo - Family Computer Disk System",
           (".fds",), (".zip", ".7z"), ("fds", "famicomdisk"), "fds", "fds", "FDS",
           "Famicom Disk System (FDS)", "Nintendo FDS", 106),
    System("snes", "Super Nintendo", "Nintendo - Super Nintendo Entertainment System",
           (".sfc", ".smc", ".swc", ".fig"), (".zip", ".7z"),
           ("superfamicom", "sfc", "supernintendo", "snes"), "snes", "snes", "SFC",
           "Super Nintendo Entertainment System (SFC)", "Nintendo SNES - SFC", 4),
    System("gb", "Game Boy", "Nintendo - Game Boy", (".gb",), (".zip", ".7z"),
           ("gameboy", "gb"), "gb", "gb", "GB", "Game Boy (GB)", "Nintendo Game Boy", 9),
    System("gbc", "Game Boy Color", "Nintendo - Game Boy Color", (".gbc",), (".zip", ".7z"),
           ("gameboycolor", "gbc"), "gbc", "gbc", "GBC", "Game Boy Color (GBC)",
           "Nintendo Game Boy Color", 10),
    System("gba", "Game Boy Advance", "Nintendo - Game Boy Advance", (".gba",), (".zip", ".7z"),
           ("gameboyadvance", "gba"), "gba", "gba", "GBA", "Game Boy Advance (GBA)",
           "Nintendo Game Boy Advance", 12),
    System("n64", "Nintendo 64", "Nintendo - Nintendo 64", (".n64", ".z64", ".v64"), (".zip", ".7z"),
           ("nintendo64", "n64"), "n64", "n64", "N64", "Nintendo 64 (N64)", "Nintendo N64", 14),
    System("nds", "Nintendo DS", "Nintendo - Nintendo DS", (".nds",), (".zip", ".7z"),
           ("nintendods", "ds", "nds"), "nds", "nds", "NDS", "Nintendo DS (NDS)",
           "Nintendo DS", 15),
    System("virtualboy", "Virtual Boy", "Nintendo - Virtual Boy", (".vb", ".vboy"), (".zip", ".7z"),
           ("virtualboy", "vb"), "virtualboy", "virtualboy", "VB", "Virtual Boy (VB)",
           "Nintendo Virtual Boy", 11),
    System("pokemini", "Pokemon Mini", "Nintendo - Pokemon Mini", (".min",), (".zip", ".7z"),
           ("pokemonmini", "pokemini", "pkm"), "pokemini", "pokemini", "POKE",
           "Pokemon mini (PKM)", "Nintendo Pokemon Mini", 211),
    System("genesis", "Sega Genesis / Mega Drive", "Sega - Mega Drive - Genesis",
           (".md", ".gen", ".smd"), (".bin", ".zip", ".7z"),
           ("megadrive", "genesis", "md", "segagenesis", "segamegadrive"),
           "genesis", "megadrive", "MD", "Sega Genesis (MD)", "Sega Mega Drive - Genesis", 1),
    System("sega32x", "Sega 32X", "Sega - 32X", (".32x",), (".bin", ".zip", ".7z"),
           ("32x", "sega32x"), "sega32x", "sega32x", "THIRTYTWOX", "Sega 32X (32X)",
           "Sega 32X", 19),
    System("mastersystem", "Sega Master System", "Sega - Master System - Mark III",
           (".sms",), (".zip", ".7z"), ("mastersystem", "sms", "markiii"),
           "mastersystem", "mastersystem", "MS", "Sega Master System (SMS)",
           "Sega Master System", 2),
    System("gamegear", "Sega Game Gear", "Sega - Game Gear", (".gg",), (".zip", ".7z"),
           ("gamegear", "gg"), "gamegear", "gamegear", "GG", "Sega Game Gear (GG)",
           "Sega Game Gear", 21),
    System("sg1000", "Sega SG-1000", "Sega - SG-1000", (".sg",), (".zip", ".7z"),
           ("sg1000",), "sg-1000", "sg1000", "SEGASGONE", "Sega SG-1000 (SG1000)",
           "Sega SG-1000", 109),
    System("segacd", "Sega CD / Mega-CD", "Sega - Mega-CD - Sega CD", (), CD,
           ("segacd", "megacd", "mcd"), "segacd", "segacd", "SEGACD", "Sega CD (SEGACD)",
           "Sega Mega CD - Sega CD", 20, True),
    System("saturn", "Sega Saturn", "Sega - Saturn", (), CD, ("saturn", "segasaturn"),
           "saturn", "saturn", "SATURN", "Sega Saturn (SATURN)", "Sega Saturn", 22, True),
    System("dreamcast", "Sega Dreamcast", "Sega - Dreamcast", (), (".gdi", ".cdi", ".chd", ".m3u"),
           ("dreamcast", "dc"), "dreamcast", "dreamcast", "DC", "Sega Dreamcast (DC)",
           "Sega Dreamcast", 23, True),
    System("psx", "Sony PlayStation", "Sony - PlayStation", (), CD + (".pbp",),
           ("playstation", "psx", "ps1", "ps", "sonyplaystation"), "psx", "psx", "PS",
           "Sony PlayStation (PS)", "Sony PlayStation", 57, True),
    System("psp", "Sony PlayStation Portable", "Sony - PlayStation Portable", (),
           (".iso", ".cso", ".pbp", ".chd"), ("psp", "playstationportable"), "psp", "psp", "PSP",
           "Sony PlayStation Portable (PSP)", "Sony PlayStation Portable", 61, True),
    System("pcengine", "PC Engine / TurboGrafx-16", "NEC - PC Engine - TurboGrafx 16",
           (".pce",), (".zip", ".7z"), ("pcengine", "pce", "turbografx16", "tg16", "turbografx"),
           "pcengine", "pcengine", "PCE", "TurboGrafx-16 (PCE)", "NEC PC Engine", 31),
    System("pcenginecd", "PC Engine CD / TurboGrafx-CD", "NEC - PC Engine CD - TurboGrafx-CD",
           (), CD, ("pcenginecd", "pcecd", "turbografxcd", "tgcd"), "pcenginecd", "pcenginecd",
           "PCECD", "TurboGrafx-CD (PCECD)", "NEC PC Engine CD", 114, True),
    System("atari2600", "Atari 2600", "Atari - 2600", (".a26",), (".bin", ".zip", ".7z"),
           ("atari2600", "2600", "a2600", "atari"), "atari2600", "atari2600", "ATARI",
           "Atari 2600 (A2600)", "Atari 2600", 26),
    System("atari7800", "Atari 7800", "Atari - 7800", (".a78",), (".zip", ".7z"),
           ("atari7800", "7800", "a7800"), "atari7800", "atari7800", "SEVENTYEIGHTHUNDRED",
           "Atari 7800 (A7800)", "Atari 7800", 41),
    System("atarilynx", "Atari Lynx", "Atari - Lynx", (".lnx", ".lyx"), (".zip", ".7z"),
           ("lynx", "atarilynx"), "atarilynx", "lynx", "LYNX", "Atari Lynx (LYNX)",
           "Atari Lynx", 28),
    System("ngp", "Neo Geo Pocket", "SNK - Neo Geo Pocket", (".ngp",), (".zip", ".7z"),
           ("neogeopocket", "ngp"), "ngp", "ngp", "NGP", "Neo Geo Pocket (NGP)",
           "SNK Neo Geo Pocket - Color", 25),
    System("ngpc", "Neo Geo Pocket Color", "SNK - Neo Geo Pocket Color", (".ngc", ".npc"),
           (".zip", ".7z"), ("neogeopocketcolor", "ngpc"), "ngpc", "ngpc", "NGP",
           "Neo Geo Pocket Color (NGPC)", "SNK Neo Geo Pocket - Color", 82),
    System("wonderswan", "WonderSwan", "Bandai - WonderSwan", (".ws",), (".zip", ".7z"),
           ("wonderswan", "ws", "wswan"), "wonderswan", "wswan", "WS", "WonderSwan (WS)",
           "Bandai WonderSwan - Color", 45),
    System("wonderswancolor", "WonderSwan Color", "Bandai - WonderSwan Color", (".wsc",),
           (".zip", ".7z"), ("wonderswancolor", "wsc", "wswanc"), "wonderswancolor", "wswanc",
           "WS", "WonderSwan Color (WSC)", "Bandai WonderSwan - Color", 46),
    System("colecovision", "ColecoVision", "Coleco - ColecoVision", (".col",), (".rom", ".zip", ".7z"),
           ("colecovision", "coleco"), "colecovision", "colecovision", "COLECO",
           "ColecoVision (COLECO)", "ColecoVision", 48),
    System("msx", "MSX", "Microsoft - MSX", (".mx1", ".mx2", ".dsk"), (".rom", ".zip", ".7z"),
           ("msx", "msx1", "msx2"), "msx", "msx1", "MSX", "MSX (MSX)", "Microsoft - MSX", 113),
    System("vectrex", "Vectrex", "GCE - Vectrex", (".vec",), (".bin", ".zip", ".7z"),
           ("vectrex",), "vectrex", "vectrex", "VECTREX", "Vectrex (VECTREX)",
           "GCE-Vectrex", 102),
    System("pico8", "PICO-8", None, (".p8",), (), ("pico8", "pico"), "pico8", "pico8",
           "PICO", "Pico-8 (P8)", "PICO-8", 234),
    System("arcade", "Arcade (MAME / FBNeo)", None, (), (".zip", ".7z"),
           ("arcade", "mame", "fbneo", "fba", "finalburn", "cps1", "cps2", "cps3", "neogeo"),
           "arcade", "mame", "ARCADE", "Arcade (FBN)", "Arcade", 75),
]

BY_ID: dict[str, System] = {s.id: s for s in SYSTEMS}


def _norm(text: str) -> str:
    return re.sub(r"[^a-z0-9]", "", text.lower())


def _build_alias_map() -> dict[str, System]:
    out: dict[str, System] = {}
    for s in SYSTEMS:
        names = {s.id, s.name, s.es, s.batocera, s.onion, *s.aliases}
        if s.libretro:
            names.add(s.libretro)
        # "Game Boy (GB)" -> also index the tag "GB"
        m = re.search(r"\(([^)]+)\)$", s.nextui)
        if m:
            names.add(m.group(1))
        names.add(s.nextui)
        for n in names:
            key = _norm(n)
            if key and key not in out:
                out[key] = s
    return out


ALIASES = _build_alias_map()

EXT_MAP: dict[str, System] = {}
for _s in SYSTEMS:
    for _e in _s.exts:
        EXT_MAP[_e] = _s


def system_from_folder_name(name: str) -> System | None:
    """Map a folder name like 'Game Boy (GB)', 'megadrive' or 'PS1' to a system."""
    key = _norm(name)
    if key in ALIASES:
        return ALIASES[key]
    m = re.search(r"\(([^)]+)\)\s*$", name)
    if m and _norm(m.group(1)) in ALIASES:
        return ALIASES[_norm(m.group(1))]
    return None


def get(system_id: str) -> System:
    try:
        return BY_ID[system_id]
    except KeyError:
        raise KeyError(f"Unknown system '{system_id}'. Run 'rom-manager systems' to list them.")
