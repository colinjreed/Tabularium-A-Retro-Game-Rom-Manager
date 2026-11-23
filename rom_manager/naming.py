"""Parse No-Intro / Redump / GoodTools style ROM filenames.

Examples it understands::

    Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2).gb
    Legend of Zelda, The - Link's Awakening DX (USA, Europe) (Rev 2) (SGB Enhanced).gbc
    Final Fantasy VII (USA) (Disc 2).chd
    Sonic the Hedgehog (Beta) [!].md
    Pokemon - Red Version (USA, Europe) (SGB Enhanced) [T+Fre].gb
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

REGIONS = {
    "usa": "USA", "us": "USA", "u": "USA", "america": "USA",
    "world": "World", "w": "World",
    "europe": "Europe", "eu": "Europe", "e": "Europe",
    "japan": "Japan", "jp": "Japan", "j": "Japan",
    "uk": "UK", "united kingdom": "UK",
    "germany": "Germany", "g": "Germany", "france": "France", "f": "France",
    "spain": "Spain", "s": "Spain", "italy": "Italy", "i": "Italy",
    "netherlands": "Netherlands", "sweden": "Sweden", "denmark": "Denmark",
    "norway": "Norway", "finland": "Finland", "portugal": "Portugal",
    "korea": "Korea", "k": "Korea", "china": "China", "c": "China",
    "taiwan": "Taiwan", "hong kong": "Hong Kong", "asia": "Asia",
    "australia": "Australia", "a": "Australia", "brazil": "Brazil", "b": "Brazil",
    "canada": "Canada", "russia": "Russia", "scandinavia": "Scandinavia",
    "latin america": "Latin America", "unknown": "Unknown",
}

LANG_CODES = {
    "en", "ja", "fr", "de", "es", "it", "nl", "pt", "sv", "no", "da", "fi",
    "zh", "ko", "pl", "ru", "el", "tr", "cs", "hu", "ca", "hr",
}

# Flags users commonly want to filter out. Keys are what you pass on the
# command line (--exclude beta,proto,...).
FLAG_PATTERNS: dict[str, re.Pattern] = {
    "beta": re.compile(r"^beta( \d+)?$|^beta \S+$", re.I),
    "proto": re.compile(r"^proto(type)?( \d+)?$", re.I),
    "demo": re.compile(r"^(demo|trial|taikenban|playable demo|tech demo)( \d+)?$", re.I),
    "sample": re.compile(r"^sample( \d+)?$", re.I),
    "kiosk": re.compile(r"^kiosk( \S+)?$", re.I),
    "promo": re.compile(r"^promo$", re.I),
    "unlicensed": re.compile(r"^unl$|^unlicensed$", re.I),
    "pirate": re.compile(r"^pirate$", re.I),
    "hack": re.compile(r"^hack$", re.I),
    "aftermarket": re.compile(r"^aftermarket$|^homebrew$", re.I),
    "program": re.compile(r"^program$|^test program$", re.I),
    "virtualconsole": re.compile(r"virtual console|switch online|nso|classic mini|"
                                 r"collection|evercade|gamecube|wii u|3ds", re.I),
}

# GoodTools-style bracket codes.
GOOD_FLAGS = {
    "b": "bad", "o": "overdump", "h": "hack", "t": "trainer", "f": "fixed",
    "p": "pirate", "a": "alternate", "!": "verified", "c": "checksum",
}

ALL_FLAGS = sorted(set(FLAG_PATTERNS) | {"bios", "bad", "overdump", "trainer", "translation",
                                         "alternate", "fixed"})

_TAG_RE = re.compile(r"\(([^()]*)\)|\[([^\[\]]*)\]")
_REV_RE = re.compile(r"^(?:rev\s*([0-9a-z.]+)|v\s?([0-9][0-9a-z.]*))$", re.I)
_DISC_RE = re.compile(r"^(?:disc|disk|cd)\s*([0-9]+|[a-z])(?:\s+of\s+\d+)?$", re.I)
_ARTICLE_RE = re.compile(r"^(.*?), (The|A|An|Les|Le|La|Der|Die|Das|El|Il)( - .*)?$", re.I)


@dataclass
class ParsedName:
    raw: str
    title: str                       # human title, tags removed, article restored
    key: str                         # normalised key for grouping/matching
    regions: list[str] = field(default_factory=list)
    languages: list[str] = field(default_factory=list)
    revision: tuple = (0,)
    disc: int | None = None
    flags: set[str] = field(default_factory=set)
    verified: bool = False
    tags: list[str] = field(default_factory=list)

    @property
    def is_clean(self) -> bool:
        """True for ordinary retail releases (no beta/proto/hack/bad-dump flags)."""
        return not (self.flags - {"alternate", "fixed"})


def _rev_tuple(text: str) -> tuple:
    parts = re.split(r"[.]", text)
    out = []
    for p in parts:
        if p.isdigit():
            out.append(int(p))
        elif len(p) == 1 and p.isalpha():
            out.append(ord(p.lower()) - ord("a") + 1)
        else:
            m = re.match(r"\d+", p)
            out.append(int(m.group()) if m else 0)
    return tuple(out) or (0,)


def title_key(title: str) -> str:
    """Normalised key: lowercase alnum, leading articles dropped, '&' == 'and'."""
    t = title.lower().replace("&", " and ")
    t = re.sub(r"^(the|a|an)\s+", "", t)
    t = re.sub(r"\s+-\s+(the|a|an)\s+", " - ", t)
    return re.sub(r"[^a-z0-9]", "", t)


def restore_article(title: str) -> str:
    """'Legend of Zelda, The - Link's Awakening' -> 'The Legend of Zelda - Link's Awakening'."""
    m = _ARTICLE_RE.match(title)
    if m:
        return f"{m.group(2)} {m.group(1)}{m.group(3) or ''}"
    return title


def parse(name: str) -> ParsedName:
    """Parse a filename *stem* (no extension)."""
    regions: list[str] = []
    langs: list[str] = []
    flags: set[str] = set()
    tags: list[str] = []
    revision: tuple = (0,)
    disc = None
    verified = False

    raw = name
    if re.match(r"^\[BIOS\]", name, re.I) or re.search(r"\bBIOS\b", name):
        flags.add("bios")

    for m in _TAG_RE.finditer(name):
        paren, bracket = m.group(1), m.group(2)
        if paren is not None:
            tag = paren.strip()
            tags.append(tag)
            parts = [p.strip() for p in tag.split(",")]
            low = [p.lower() for p in parts]
            if all(p in REGIONS for p in low) and not all(p in LANG_CODES for p in low):
                regions.extend(REGIONS[p] for p in low)
                continue
            if all(p in LANG_CODES or re.fullmatch(r"[a-z]{2}(\+[a-z]{2})*", p) for p in low):
                langs.extend(p for part in low for p in part.split("+"))
                continue
            rm = _REV_RE.match(tag)
            if rm:
                revision = _rev_tuple(rm.group(1) or rm.group(2))
                continue
            dm = _DISC_RE.match(tag)
            if dm:
                d = dm.group(1)
                disc = int(d) if d.isdigit() else ord(d.lower()) - ord("a") + 1
                continue
            for flag, pat in FLAG_PATTERNS.items():
                if pat.search(tag):
                    flags.add(flag)
        else:
            tag = bracket.strip()
            tags.append(f"[{tag}]")
            if tag.upper() == "BIOS":
                flags.add("bios")
            elif tag == "!":
                verified = True
            elif re.match(r"^T[+-]", tag):
                flags.add("translation")
            else:
                code = re.match(r"^([a-z!])\d*", tag)
                if code and code.group(1) in GOOD_FLAGS and len(tag) <= 4:
                    f = GOOD_FLAGS[code.group(1)]
                    if f not in ("verified", "checksum"):
                        flags.add(f)

    title = _TAG_RE.sub("", name)
    title = re.sub(r"^\s*\[BIOS\]\s*", "", title, flags=re.I)
    title = re.sub(r"\s+", " ", title).strip(" -_.")
    display = restore_article(title)
    return ParsedName(
        raw=raw, title=display, key=title_key(display), regions=regions,
        languages=langs, revision=revision, disc=disc, flags=flags,
        verified=verified, tags=tags,
    )


# Characters libretro replaces with '_' in thumbnail filenames.
_LIBRETRO_BAD = re.compile(r'[&*/:`<>?\\|"]')


def libretro_safe(name: str) -> str:
    return _LIBRETRO_BAD.sub("_", name)


def strip_disc(name: str) -> str:
    """'Final Fantasy VII (USA) (Disc 1)' -> 'Final Fantasy VII (USA)'."""
    return re.sub(r"\s*\((?:disc|disk|cd)\s*[0-9a-z]+(?:\s+of\s+\d+)?\)", "", name,
                  flags=re.I).strip()
