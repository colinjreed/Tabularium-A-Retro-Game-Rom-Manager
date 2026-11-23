# Tabularium

> **Disclaimer: this is a vibe-coded project.** The code was written by an
> AI assistant (Claude) from my descriptions and feedback. I steer, test it
> on my own collection and handhelds, and decide what goes in, but I didn't
> write the code by hand and haven't reviewed every line. It has automated
> tests, but expect rough edges. Keep backups of your ROMs and SD cards,
> and judge for yourself whether that's something you want to use.

A desktop app for Windows, macOS and Linux that organises a retro game ROM
collection, weeds out duplicates and junk, downloads artwork, checks BIOS
files, and builds ready-to-use SD cards for handhelds (Onion OS, muOS,
ArkOS/ROCKNIX, Knulli/Batocera, NextUI/MinUI and ES-DE).

### Why "Tabularium"?

The Tabularium was ancient Rome's public records office. It was built on
the Capitoline Hill in 78 BC, overlooking the Forum, to hold the state's
official records. The name comes from *tabulae*, the tablets those records
were written on. This app does the same job for a game collection: one
carefully catalogued archive of your ROMs, artwork and BIOS files, from
which you can carry a selection out to each handheld.

![Library view](docs/screenshots/library.png)

## Download

Builds for every platform are produced by GitHub Actions:

* **Releases page**: tagged versions (`v0.2.0`, …) attach installers to a GitHub release.
* **Actions tab**: every push has downloadable build artifacts.

| Platform | File | Notes |
|---|---|---|
| Windows 10/11 | `tabularium-windows-x86_64.zip` | Unzip and run `Tabularium.exe`. The app isn't code-signed, so SmartScreen may warn: choose *More info → Run anyway*. |
| macOS 11+ (Apple Silicon & Intel) *(paused: build by hand from the Actions tab)* | `tabularium-macos-universal.dmg` | Drag *Tabularium* to Applications. It isn't notarised: the first time, right-click the app → *Open* → *Open* (or allow it in System Settings → Privacy & Security). |
| Linux x86_64 | `Tabularium-x86_64.AppImage` | `chmod +x` it and run. Or use the `.tar.gz` and run `./install.sh` to add it to your applications menu. |

## How it works

```
your messy folders ──Import──▶ LIBRARY (sorted by system, indexed)
                                  │  Clean up  → duplicates / 1G1R / betas… → trash (restorable)
                                  │  Artwork   → box art downloaded once
                                  │  BIOS      → verified BIOS files
                                  ▼
                       SD cards (one per handheld) ──Copy to card──▶ microSD
```

1. **Create a library.** Pick a folder with plenty of space. ROMs get copied
   (or moved) into it, sorted by system.
2. **Import.** Add or drag in the folders where your ROMs live now. The
   system is worked out from the file extension, what's inside zips, folder
   names ("PS1", "megadrive", "Game Boy (GB)") or the file's header (Genesis,
   32X, PS1, PSP, Sega CD, Saturn). Byte-identical copies are skipped, even a
   zipped and an unzipped copy of the same game. Use *Preview* first to see
   what will happen.
   By default, files are renamed to their **official No-Intro names**,
   looked up by checksum for cartridge systems. So `003 sonic.md` becomes
   `Sonic The Hedgehog (USA, Europe).md`, and games filed under the wrong
   system (e.g. Game Gear games saved as `.sms`) are moved. Files not in
   the lists still lose numbering like `003 - ` when most of their folder
   is numbered that way. For a library you've already imported, use
   **Clean up → File names → Find names to fix**, check the preview, then
   press *Rename them*.
3. **Clean up** (optional). Tick what you don't want (betas, prototypes,
   demos, hacks, bad dumps…) and turn on *one game, one ROM* to keep only
   the best region and latest revision of each game. Results go to the trash,
   and nothing is deleted until you empty it.
4. **Artwork.** Download box art, screenshots or title screens.
5. **BIOS.** See which systems need a BIOS, import the files (they're
   recognised by MD5, whatever they're named), and check an SD card.
6. **SD cards.** Make one entry per handheld:
   * the firmware it runs (sets the folder layout);
   * the systems to include;
   * which versions of each game (the same filters as Clean up, without
     changing your library);
   * the card size: games are spread fairly across systems if it won't all
     fit, with any you mark as priorities going first.

   A live preview shows what fits. *Copy to card* then writes the ROMs,
   resized artwork, `.m3u` playlists for multi-disc games, `gamelist.xml`
   where the firmware uses one, and the BIOS files those systems need.
   Copying again only transfers what changed. *Remove games I've taken out*
   deletes only files the app itself put on the card, never your saves.

![SD card setup](docs/screenshots/sd-card.png)

### Mixtapes, tags and play tracking

* **Hand-picked games:** select games in the Library and use *Add to SD
  card*, or search for them in a card's *Hand-picked games* section.
  Hand-picked games always go on the card. Untick *Also add games by
  rules* for a card that holds only your picks, like a mixtape, or keep
  rules on to combine both: e.g. 12 favourites plus 30 random unplayed
  RPGs.
* **Selecting several games:** Ctrl-click (⌘-click on a Mac) or Shift-click
  in the Library. Then tag them, mark them played, add them to a card or
  trash them all at once.
* **Genres:** filled in automatically for cartridge systems from libretro's
  genre lists (looked up by checksum), and editable by hand. For games
  imported before this feature, use *Artwork → Look up genres*.
* **Tags:** your own labels (`#co-op`, `#short`, `#comfort`…). Filter by
  them in the Library, and use them in a card's rules: include games with
  a tag, or leave them out.
* **Played / Finished:** mark games yourself, or let Tabularium read play
  history back from your cards. EmulationStation-based firmware (ArkOS,
  ROCKNIX, Knulli/Batocera, ES-DE) records it in `gamelist.xml`, and Onion
  OS in its play-activity database. The history is read every time you
  copy to a card, or on demand with *Read play history now*. It only ever
  adds to what the library knows, and never marks a game unplayed.
* **Random picks:** a card can take *N random games* from whatever its
  rules match. Combine it with *Only games I haven't played* for a
  rotating selection, and press *Reshuffle* for a different set. In the
  Library, *🎲 Surprise me* picks a random unplayed game from the current
  list.

## Supported handheld firmware

| Firmware | Devices | ROMs | Artwork | BIOS |
|---|---|---|---|---|
| Onion OS | Miyoo Mini / Mini+ / Flip | `Roms/GB/` | `Roms/GB/Imgs/` (250px) | `BIOS/` |
| muOS | Anbernic RG35XX/40XX/CubeXX, TrimUI | `ROMS/gb/` | `MUOS/info/catalogue/<System>/box/` | `MUOS/bios/` |
| EmulationStation | ArkOS, ROCKNIX, AmberELEC | `roms/gb/` | `roms/gb/images/` + `gamelist.xml` | `roms/bios/` |
| Knulli / Batocera | Anbernic RG35XX family, PCs | `roms/gb/` (`megadrive`, `lynx`…) | `roms/gb/images/` + `gamelist.xml` | `bios/` |
| NextUI / MinUI | TrimUI Brick / Smart Pro, Miyoo | `Roms/Game Boy (GB)/` | `Roms/…/.media/` (300px) | `Bios/GB/` |
| ES-DE | Retroid Pocket, AYN Odin, Ayaneo, Steam Deck | `ROMs/gb/` | `ES-DE/downloaded_media/gb/covers/` | `RetroArch/system/` |

PS2, GameCube, 3DS and Xbox need powerful hardware (Retroid / Odin-class
Android devices, or a PC running Batocera or ES-DE). Low-power handhelds
like the Miyoo Mini can't run them, but their folder names are still defined.

BIOS checks include every PS2 BIOS dump known to libretro (by hash), the
Xbox files xemu needs (`mcpx_1.0.bin`, `Complex_4627.bin`,
`xbox_hdd.qcow2`, by name), and the optional GameCube IPL and DOS MT-32 ROMs.

Firmware updates sometimes rename folders. Every system's folder name can be
overridden per SD card under *Artwork and extras → Folder names on the card*.

44 systems are supported: NES, Famicom Disk System, SNES, Game Boy / Color /
Advance, N64, DS, 3DS, GameCube, Virtual Boy, Pokémon Mini, Genesis, 32X,
Master System, Game Gear, SG-1000, Sega CD, Saturn, Dreamcast, PlayStation
1 and 2, PSP, Xbox, PC Engine (and CD), Atari 2600 / 7800 / Lynx, Neo Geo
Pocket (Color), WonderSwan (Color), ColecoVision, MSX, Vectrex, PICO-8, DOS,
ScummVM, and arcade (FBNeo, MAME, CPS-1, CPS-2, CPS-3).

### Disc systems: PS1 vs PS2, GameCube, Xbox

Disc images are identified from their contents, so a PS2 `.iso` sitting in a
"PS1" folder still lands in PS2. PS1 and PS2 discs are told apart by the
`SYSTEM.CNF` file on the disc. GameCube discs are recognised from the disc
header, including inside Dolphin's `.rvz` / `.gcz` formats (Wii discs are
skipped). Xbox discs are recognised as XISO images, the format xemu needs.

### Arcade (FBNeo, MAME, CPS-1/2/3)

Arcade games are zips named by short name (`sf2.zip`). The app downloads
libretro's FBNeo and MAME name lists once (about 18,000 games), then:

* zips that aren't recognised any other way are identified as arcade games;
* they show their full names, e.g. "Street Fighter II: The World Warrior
  (World 910522)", and artwork is matched by that name;
* one-game-one-ROM works for arcade too, keeping e.g. the World version
  over Japanese and bootleg sets.

Which arcade system a game goes to comes from its folder name (`mame`,
`cps1`, `cps2`, `cps3`, `fbneo`…), defaulting to FBNeo. Keep each
system's romset matched to the emulator version on your device.

### DOS and ScummVM (folder games)

DOS and ScummVM games are usually folders of files, and the app treats each
folder as one game. A folder counts as a game when it ends in `.scummvm`,
`.svm`, `.pc` or `.dos`, contains a ScummVM `.scummvm` file, or sits directly
inside a folder named `DOS` or `ScummVM`. Zipped DOS games (`.zip`,
`.dosz`) work too.

ScummVM needs each game's **game ID** (e.g. `monkey2`). The app works it out
and writes it into a `<Game>.scummvm` file inside the folder. It uses, in
order:

1. an existing `.scummvm` file;
2. well-known data files (e.g. `MONKEY2.000` means `monkey2`);
3. a folder already named after the ID;
4. the title, matched against libretro's list of 11,000+ ScummVM games.

Anything still unidentified can be set by hand in the library's detail panel.
On the SD card each game becomes `<Game>.scummvm/` with the ID file inside,
the layout Batocera, ES-DE and ROCKNIX expect. Batocera's DOS folders get
the `.pc` suffix it needs.

## Artwork sources

| | libretro thumbnails (default) | ScreenScraper |
|---|---|---|
| Account | none | your account, plus developer credentials |
| Matches by | filename (with fuzzy matching) | file contents (CRC + size), then name |
| Best for | No-Intro / Redump-named sets | badly named files, arcade, hacks |
| Image types | box art, screenshot, title screen | the same, plus mix images and logos |

With both enabled, anything one source misses is tried on the other. Enter
ScreenScraper details in **Settings**. Developer credentials are requested
on the ScreenScraper forum. Passwords are kept only until you quit, unless
you tick *Remember passwords*. The `SCREENSCRAPER_USER`,
`SCREENSCRAPER_PASSWORD`, `SCREENSCRAPER_DEVID` and
`SCREENSCRAPER_DEVPASSWORD` environment variables also work.

## Library folder

```
LIBRARY/
  library.db              index (hashes, parsed names, status)
  roms/<system>/          your organised ROMs
  media/<system>/<kind>/  artwork cache
  bios/                   verified BIOS files
  trash/<system>/         cleaned-out ROMs (restorable)
  unsorted/               files whose system couldn't be identified
  collections/*.json      SD card definitions
```

You can open a library directly: `tabularium /path/to/LIBRARY`. Libraries
made with the earlier command-line (Python) version open unchanged.

## Building from source

Requires Rust 1.95 or newer (`rustup update`).

```sh
cargo run --release          # run the app
cargo test --all             # run the tests (no network needed)
```

The workspace has two crates:

* `crates/core`: all the logic (detection, library, filters, scrapers, BIOS,
  export) with no UI, covered by unit and end-to-end tests.
* `crates/app`: the desktop UI, built with [egui](https://github.com/emilk/egui).

On Linux the app needs the usual desktop libraries at runtime (X11 or
Wayland, `libxkbcommon-x11`, OpenGL), which every desktop distribution
already has. File dialogs use the XDG desktop portal. If yours has none,
type paths into the boxes next to each *Choose…* button.

To publish a release, push a tag such as `v0.2.0`. The workflow builds all
three platforms and attaches them to a GitHub release.
