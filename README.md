# ROM Manager

Organise a large retro ROM collection, weed out duplicates and junk, scrape
artwork, check BIOS files, and export ready-to-copy SD card collections for
handhelds (Onion OS, muOS, ArkOS/ROCKNIX, Knulli/Batocera, NextUI/MinUI,
ES-DE on Android).

Pure Python 3.10+, no required dependencies. Installing
[Pillow](https://pypi.org/project/Pillow/) lets it resize artwork for small
screens.

```sh
pip install .            # or: pip install '.[images]' to include Pillow
rom-manager --help       # or: python -m rom_manager --help
```

## How it works

```
messy folders ──import──▶ LIBRARY (organised by system, indexed)
                               │  clean   → duplicates / 1G1R / betas … → trash
                               │  scrape  → box art cached once
                               │  bios    → verified BIOS files
                               ▼
                     collections (saved recipes) ──export──▶ SD card(s)
```

You keep **one master library**. Each handheld gets a **collection**: a saved
recipe of target firmware, systems, filters and size budget. Exporting a
collection never changes the library, and re-exporting only copies what
changed.

## Quick start

```sh
export ROM_MANAGER_LIBRARY=~/Games/library
rom-manager init ~/Games/library

# 1. Sort everything into the library (copies by default; --move to move)
rom-manager import ~/Downloads/roms /mnt/old-sd/Roms --dry-run
rom-manager import ~/Downloads/roms /mnt/old-sd/Roms
rom-manager stats

# 2. Weed out junk and keep one ROM per game (reports first; --apply to act)
rom-manager clean --junk --1g1r
rom-manager clean --junk --1g1r --apply      # moved to LIBRARY/trash, restorable
rom-manager trash list
rom-manager trash restore 123

# 3. Artwork (downloaded once, reused for every card)
rom-manager scrape                            # libretro box art
rom-manager scrape --source screenscraper,libretro --art boxart,mix

# 4. BIOS files
rom-manager bios list                         # what each system needs
rom-manager bios import ~/Downloads/bios      # recognised by MD5, renamed correctly
rom-manager bios check                        # what's present / missing / bad

# 5. A collection per device, exported to its SD card
rom-manager collection create miyoo --profile onion \
    --systems gb,gbc,gba,nes,snes,genesis,psx --junk --1g1r --max-size 120G
rom-manager collection preview miyoo
rom-manager export miyoo /media/SDCARD --dry-run
rom-manager export miyoo /media/SDCARD
rom-manager bios check /media/SDCARD --profile onion
```

## Importing

System detection, in order:

1. **Unique extension**: `.gba`, `.sfc`, `.md`, `.pce`, …
2. **Inside zips**: `mario.zip` containing a `.gba` file goes to GBA.
3. **Folder names** anywhere in the path: `PS1`, `megadrive`, `Game Boy (GB)`,
   `FC`, `psx`, … (every firmware's folder names are recognised).
4. **File headers** for `.bin`/`.iso`/`.cue`: Genesis, 32X, PS1, PSP, Sega CD, Saturn.

`.cue`/`.gdi` sheets bring their track files along. Files that still can't be
placed are listed (or copied to `LIBRARY/unsorted` with `--unsorted`). Put
them in a folder named after the system, or use `--system psx`.

Byte-identical copies are skipped during import, including a zipped and an
unzipped copy of the same ROM.

## Cleaning: duplicates and unwanted ROMs

`clean` reports what it would remove and only changes anything with
`--apply`. Even then, files are moved to `LIBRARY/trash/`, and only
`rom-manager trash empty` deletes them.

| Option | Removes |
|---|---|
| (always) | byte-identical duplicates |
| `--1g1r` | other versions of the same game, keeping the best region (`--region-priority USA,World,Europe,Japan` is the default), then verified dumps, then the latest revision. Multi-disc games are kept or dropped together. |
| `--junk` | beta, proto, demo, sample, kiosk, program, BIOS, bad dumps, overdumps, trainers, pirate |
| `--exclude hack,unlicensed,translation,virtualconsole,…` | any of the flags in `rom-manager clean --help` |
| `--regions USA,Europe` | anything not from these regions |
| `--languages en` | anything without English listed (when languages are tagged) |
| `--exclude-title '*Pinball*'` | titles matching a pattern |

These work best with No-Intro / Redump names, and also understand
GoodTools tags (`[!]`, `[b1]`, `[h]`, `[T+Eng]`, `(U)`).

You don't have to clean the library at all. The same filters can be set on a
collection instead, so the library keeps everything and each card gets a
curated subset.

## Artwork sources

| | libretro-thumbnails (default) | ScreenScraper |
|---|---|---|
| Account | none | free account, plus developer credentials |
| Matches by | filename (with fuzzy matching) | file hash (CRC/size), then name |
| Good for | No-Intro/Redump-named sets | badly named files, arcade, hacks |
| Art types | `boxart`, `snap`, `title` | `boxart`, `snap`, `title`, `mix`, `wheel` |
| Limits | none | daily quota and request rate per account |

List sources in priority order with `--source screenscraper,libretro`, and
anything the first source misses is tried on the next.

ScreenScraper's API needs **developer credentials** (request them on the
ScreenScraper forum for this program) as well as your user account. Provide
them as environment variables, never on the command line:

```sh
export SCREENSCRAPER_DEVID=...  SCREENSCRAPER_DEVPASSWORD=...
export SCREENSCRAPER_USER=...   SCREENSCRAPER_PASSWORD=...
```

Scraping is resumable: ROMs already tried are skipped unless you pass `--refresh`.

## BIOS files

`rom-manager bios list` shows what each system needs, using MD5 hashes from
libretro's `System.dat`. *Required* means games won't boot without it.
*Optional* means the emulator has a built-in substitute.

* `bios import <folder>` finds known BIOS dumps by hash, whatever they're
  named, and stores them under the right name in `LIBRARY/bios/`.
* `bios check` checks the library. `bios check <card> --profile onion`
  checks a card in that firmware's BIOS folder, and flags bad hashes and
  files in the wrong place.
* `export` copies the BIOS files a collection's systems use, and warns about
  missing required ones.

## Collections and SD cards

```sh
rom-manager collection create NAME --profile PROFILE [options]
  --systems gb,gba,psx           default: all systems
  --junk --1g1r --exclude … --regions … --include 'Pokemon*'   same filters as clean
  --max-size 64G                 fits the card; spread fairly across systems
  --priority 'Zelda*'            fit these first when space is tight
  --art-kind boxart|snap|title|mix|wheel
  --folder genesis=MD            override a folder name for your firmware version
  --no-m3u                       leave multi-disc games as separate entries
```

On export:

* multi-disc games become one `.m3u` entry, with their discs in a hidden
  `.Game Name/` folder;
* art is resized for the device (Onion 250px, NextUI 300px) when Pillow is
  installed;
* EmulationStation-based profiles get a `gamelist.xml`, merged with any
  existing one so play counts and favourites are kept;
* only new or changed files are copied, and the card's free space is checked first;
* `--sync` removes files that this collection put on the card earlier but no
  longer includes. It never touches your own files or saves (tracked in
  `.rom-manager/` on the card).

### Profiles

| Profile | Firmware / devices | ROMs | Art | BIOS |
|---|---|---|---|---|
| `onion` | Onion OS: Miyoo Mini / Mini+ / Flip | `Roms/GB/` | `Roms/GB/Imgs/` | `BIOS/` |
| `muos` | muOS: Anbernic RG35XX/40XX/CubeXX, TrimUI | `ROMS/gb/` | `MUOS/info/catalogue/<System>/box/` | `MUOS/bios/` |
| `emulationstation` | ArkOS, ROCKNIX, AmberELEC, dArkOS | `roms/gb/` | `roms/gb/images/` + gamelist.xml | `roms/bios/` |
| `batocera` | Knulli, Batocera | `roms/gb/` (`megadrive`, `lynx`, …) | `roms/gb/images/` + gamelist.xml | `bios/` |
| `nextui` | NextUI / MinUI: TrimUI Brick, Miyoo | `Roms/Game Boy (GB)/` | `Roms/…/.media/` | `Bios/GB/` |
| `esde` | ES-DE: Retroid, Odin, Ayaneo, Steam Deck | `ROMs/gb/` | `ES-DE/downloaded_media/gb/covers/` | `RetroArch/system/` |

`rom-manager systems --profile onion` shows every system's folder for a
firmware. Folder names change between firmware releases, so check against
your device and use `--folder` to adjust.

## Library layout

```
LIBRARY/
  library.db              SQLite index (hashes, parsed names, status)
  roms/<system>/          organised ROMs
  media/<system>/<kind>/  artwork cache
  bios/                   verified BIOS files
  trash/<system>/         cleaned-out ROMs (restorable)
  unsorted/               files whose system couldn't be identified
  collections/*.json      collection recipes (editable by hand)
```

## Development

```sh
pip install -e '.[test,images]'
python -m pytest
```

Tests use generated fake ROMs and never touch the network.
