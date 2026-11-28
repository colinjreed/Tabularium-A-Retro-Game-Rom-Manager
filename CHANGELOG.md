# Changelog

## 0.2.6

### New
- **Autoscroll with the middle mouse button.** Hold the scroll wheel and move
  the mouse to scroll any list or page; the further you move, the faster it
  goes. A quick middle-click keeps it on until your next click, and that click
  only stops scrolling.
- **ScummVM game IDs from ScummVM itself.** If ScummVM is installed,
  Tabularium asks it to recognise each game from its files, as ScummVM's "Add
  Game" does, before falling back to matching names. Clean up shows how many
  ScummVM games still lack an ID, with a **Find ScummVM IDs** button. Any
  that remain can be set by hand in the Library.

### Improved
- **Numbers in front of names are found more reliably.** Clean up now judges
  whether names are numbered by the folder they were imported from, not the
  whole system, so numbered games are fixed even when others for the same
  system already have official names. Four-digit numbers with a leading zero
  ("0042 - ") are always removed. "007" is never treated as numbering, so the
  Bond games keep their names.
- **More NES games get official names.** Games are also looked up without the
  16-byte header most .nes files start with.
- **Folders of DOS and ScummVM games are searched.** A folder inside a DOS,
  ScummVM or similar system folder that only holds other folders (for example
  `DOS/Sierra/…`) is now searched for games, instead of being imported as one
  game.
- **Move imports empty the source folder.** With "Move into the library",
  games the library already has now go to the library's Trash, marked
  "duplicate of …", instead of being left behind. Restore or empty them from
  Library → Trash. Copy imports still leave the source untouched.
- **Imports go in alphabetical order**, so which copy of a duplicate is kept is
  the same on every computer.
- **Atomic Purple theme:**
  - a detailed see-through circuit board inspired by translucent handhelds,
    with copper ribbon cables, chips, button contacts and screw posts;
  - frosted, matte plastic in place of the glassy look;
  - a speaker grille in the sidebar, with a speaker wired to the board
    underneath.

### Fixed
- The Linux build failed intermittently because of the import-order issue
  above.

### After updating
1. **Clean up → Find names to fix** to remove leftover numbers from names.
2. If a grouping folder (such as "Sierra") was imported as a single game
   before, move it to the Trash and import that folder again so the games
   inside are found.
3. **Clean up → Find ScummVM IDs**, with ScummVM installed, for ScummVM games
   without an ID.
4. To empty a folder you imported earlier with Move, import it again with
   Move: the games left over go to the Trash.

## Earlier versions

See the [Releases page](https://github.com/colinjreed/Tabularium-A-Retro-Game-Rom-Manager/releases).
