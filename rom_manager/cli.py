"""Command-line interface."""

from __future__ import annotations

import argparse
import os
import sys
from collections import Counter
from pathlib import Path

from . import __version__, bios, filters, naming, profiles, systems
from .collection import Collection, export, human, list_collections, parse_size, resolve
from .collection import collection_path
from .detect import scan
from .library import Library, LibraryError
from .scrape import scrape_library
from .scrapers import ScrapeError


def _csv(text: str | None) -> list[str]:
    return [t.strip() for t in text.split(",") if t.strip()] if text else []


def _systems_arg(text: str | None) -> list[str]:
    ids = _csv(text)
    for i in ids:
        systems.get(i)
    return ids


def _library(args) -> Library:
    path = args.library or os.environ.get("ROM_MANAGER_LIBRARY")
    if not path:
        raise LibraryError("Say which library to use with --library PATH "
                           "(or set ROM_MANAGER_LIBRARY).")
    return Library(Path(path))


def _filter_spec(args) -> filters.FilterSpec:
    exclude = _csv(getattr(args, "exclude", None))
    if getattr(args, "junk", False):
        exclude = sorted(set(exclude) | set(filters.DEFAULT_EXCLUDE))
    unknown = set(exclude) - set(naming.ALL_FLAGS)
    if unknown:
        raise LibraryError(f"Unknown flag(s): {', '.join(sorted(unknown))}. "
                           f"Known: {', '.join(naming.ALL_FLAGS)}")
    spec = filters.FilterSpec(
        exclude_flags=exclude,
        regions=_csv(getattr(args, "regions", None)),
        languages=_csv(getattr(args, "languages", None)),
        include_titles=getattr(args, "include", None) or [],
        exclude_titles=getattr(args, "exclude_title", None) or [],
        one_game_one_rom=getattr(args, "one_g1r", False),
    )
    if getattr(args, "region_priority", None):
        spec.region_priority = _csv(args.region_priority)
    return spec


# ------------------------------------------------------------------ commands
def cmd_init(args):
    lib = Library.create(Path(args.path))
    print(f"Library ready at {lib.root}")
    print(f"Tip: export ROM_MANAGER_LIBRARY={lib.root} to skip --library")


def cmd_import(args):
    lib = _library(args)
    forced = systems.get(args.system) if args.system else None
    sets, skipped = scan([Path(p) for p in args.sources], forced)
    counts: Counter = Counter()
    unknown = []
    for rs in sets:
        if rs.system is None:
            unknown.append(rs)
            continue
        outcome, detail = lib.add(rs, move=args.move, dry_run=args.dry_run)
        counts[outcome] += 1
        if outcome == "added":
            counts[f"sys:{rs.system.id}"] += 1
        if args.verbose or outcome in ("conflict", "error"):
            print(f"{outcome:9} {rs.system.id:12} {rs.main.name}  -> {detail}")
    for rs in unknown:
        if args.unsorted and not args.dry_run:
            lib.add_unsorted(rs, move=args.move)
        if args.verbose:
            print(f"unknown   {'?':12} {rs.main}  ({rs.reason})")

    verb = "Would add" if args.dry_run else "Added"
    print(f"\n{verb} {counts['added']} ROM(s); {counts['duplicate']} duplicate(s) skipped; "
          f"{counts['conflict']} name conflict(s); {counts['error']} error(s).")
    by_sys = sorted((k[4:], v) for k, v in counts.items() if k.startswith("sys:"))
    if by_sys:
        print("  " + ", ".join(f"{s}: {n}" for s, n in by_sys))
    if unknown:
        where = "copied to unsorted/" if args.unsorted and not args.dry_run else "left in place"
        print(f"{len(unknown)} file(s) with an unknown system ({where}). Re-run with "
              f"--system ID on a folder of them, or put them in a folder named after the system.")
        for rs in unknown[:10]:
            print(f"   ? {rs.main}  ({rs.reason})")
        if len(unknown) > 10:
            print(f"   ... and {len(unknown) - 10} more (use -v to list all)")
    if skipped and args.verbose:
        print(f"Ignored {len(skipped)} non-ROM file(s) (.txt, .sav, images, ...)")


def cmd_stats(args):
    lib = _library(args)
    rows = lib.stats()
    if not rows:
        print("Library is empty. Add ROMs with: rom-manager import <folder>")
        return
    total_n = total_b = 0
    print(f"{'System':34} {'ROMs':>7} {'Size':>10}   Trash")
    agg: dict[str, dict] = {}
    for r in rows:
        agg.setdefault(r["system"], {})[r["status"]] = (r["n"], r["bytes"] or 0)
    for sid, st in agg.items():
        n, b = st.get("active", (0, 0))
        t = st.get("trashed", (0, 0))[0]
        total_n += n
        total_b += b
        print(f"{systems.BY_ID[sid].name if sid in systems.BY_ID else sid:34} {n:7} "
              f"{human(b):>10}   {t or ''}")
    print(f"{'Total':34} {total_n:7} {human(total_b):>10}")


def cmd_list(args):
    lib = _library(args)
    roms = lib.roms(_systems_arg(args.systems), status="trashed" if args.trashed else "active")
    if args.search:
        needle = args.search.lower()
        roms = [r for r in roms if needle in r.stem.lower()]
    for r in roms:
        extra = f"  [{r.status_reason}]" if args.trashed and r.status_reason else ""
        print(f"{r.id:6}  {r.system:12} {human(r.total_size):>9}  {r.stem}{extra}")
    print(f"\n{len(roms)} ROM(s)")


def cmd_clean(args):
    lib = _library(args)
    spec = _filter_spec(args)
    if not (spec.exclude_flags or spec.regions or spec.languages or spec.exclude_titles
            or spec.one_game_one_rom or args.duplicates):
        raise LibraryError("Nothing to do: pass --duplicates, --1g1r, --junk, --exclude, "
                           "--regions or --exclude-title.")
    roms = lib.roms(_systems_arg(args.systems))
    _, rejected = filters.apply(roms, spec)
    if not rejected:
        print("Nothing to remove.")
        return
    reasons = Counter(why.split(":")[0] for _, why in rejected)
    for rom, why in rejected:
        if args.verbose or not args.apply:
            print(f"{rom.system:12} {rom.stem}\n{'':12}   -> {why}")
    size = sum(r.total_size for r, _ in rejected)
    print(f"\n{len(rejected)} ROM(s), {human(size)}: "
          + ", ".join(f"{k}: {v}" for k, v in reasons.most_common()))
    if args.apply:
        for rom, why in rejected:
            lib.trash(rom, why)
        print(f"Moved to {lib.root / 'trash'}. Undo with 'rom-manager trash restore ID' "
              "or delete for good with 'rom-manager trash empty'.")
    else:
        print("Dry run: nothing changed. Re-run with --apply to move these to the trash.")


def cmd_trash(args):
    lib = _library(args)
    if args.action == "list":
        args.trashed, args.search, args.systems = True, None, None
        cmd_list(args)
    elif args.action == "restore":
        ids = [int(i) for i in args.ids] if args.ids else [r.id for r in lib.roms(status="trashed")]
        for i in ids:
            rom = lib.get(i)
            if rom.status != "trashed":
                print(f"{i} is not in the trash")
                continue
            lib.restore(rom)
            print(f"Restored {rom.stem}")
    elif args.action == "empty":
        n = len(lib.roms(status="trashed"))
        if not args.yes:
            reply = input(f"Permanently delete {n} ROM(s) from the trash? [y/N] ")
            if reply.strip().lower() != "y":
                print("Cancelled.")
                return
        print(f"Deleted {lib.empty_trash()} ROM(s).")


def _build_scrapers(args, lib):
    from .scrapers.libretro import LibretroScraper
    out = []
    for src in _csv(args.source):
        if src == "libretro":
            out.append(LibretroScraper(lib.root / "media" / ".cache", cutoff=args.match_cutoff,
                                       log=print))
        elif src == "screenscraper":
            from .scrapers.screenscraper import ScreenScraper
            out.append(ScreenScraper(log=print))
        else:
            raise LibraryError(f"Unknown art source '{src}' (libretro, screenscraper)")
    return out


def cmd_scrape(args):
    lib = _library(args)
    kinds = _csv(args.art)
    try:
        scrapers = _build_scrapers(args, lib)
    except ScrapeError as e:
        raise LibraryError(str(e))
    unsupported = [k for k in kinds if not any(k in s.kinds for s in scrapers)]
    if unsupported:
        print(f"Note: {', '.join(unsupported)} art only comes from ScreenScraper.")
    log = print if args.verbose else (lambda msg: print(msg) if "✓" not in msg and "✗" not in msg
                                      else None)
    rep = scrape_library(lib, scrapers, kinds, _systems_arg(args.systems), args.refresh, log=log)
    print(f"\nArtwork: {rep.found} downloaded, {rep.missing} not found, "
          f"{rep.skipped} already done.")
    if rep.missing_list and not args.verbose:
        print("Not found (first 15):")
        for m in rep.missing_list[:15]:
            print("   " + m)
    if rep.stopped:
        print(f"Stopped early: {rep.stopped}")


def cmd_collection(args):
    lib = _library(args)
    if args.action == "list":
        names = list_collections(lib)
        if not names:
            print("No collections yet. Create one with: rom-manager collection create NAME "
                  "--profile onion --systems gb,gba")
        for n in names:
            c = Collection.load(lib, n)
            print(f"{n:24} {c.profile:16} {','.join(c.systems) or 'all systems'}")
        return
    if args.action == "create":
        if collection_path(lib, args.name).exists() and not args.overwrite:
            raise LibraryError(f"Collection '{args.name}' exists; pass --overwrite to replace it.")
        profiles.get(args.profile)
        overrides = {}
        for item in args.folder or []:
            k, _, v = item.partition("=")
            systems.get(k)
            overrides[k] = v
        col = Collection(
            name=args.name, profile=args.profile, systems=_systems_arg(args.systems),
            filters=_filter_spec(args).to_dict(),
            max_size=parse_size(args.max_size) if args.max_size else None,
            priority_titles=args.priority or [], art_kind=args.art_kind,
            multidisc_m3u=not args.no_m3u, folder_overrides=overrides)
        col.save(lib)
        print(f"Saved collection '{col.name}' -> {collection_path(lib, col.name)}")
        args.action = "show"
    col = Collection.load(lib, args.name)
    if args.action == "delete":
        collection_path(lib, args.name).unlink()
        print(f"Deleted collection '{args.name}' (the library is unchanged).")
        return
    sel = resolve(lib, col)
    profile = profiles.get(col.profile)
    if args.action == "preview":
        for r in sel.roms:
            print(f"{r.system:12} {human(r.total_size):>9}  {r.stem}")
        if args.verbose:
            for r, why in sel.rejected:
                print(f"  - {r.system:12} {r.stem}  ({why})")
    print(f"\nCollection '{col.name}' for {profile.label}")
    print(f"  systems: {', '.join(col.systems) or 'all'}; "
          f"1G1R: {'on' if col.filter_spec.one_game_one_rom else 'off'}; "
          f"excluded flags: {', '.join(col.filter_spec.exclude_flags) or 'none'}")
    per = Counter(r.system for r in sel.roms)
    size = Counter()
    for r in sel.roms:
        size[r.system] += r.total_size
    for sid in sorted(per):
        folder = profile.rom_dir(systems.BY_ID[sid], col.folder_overrides)
        print(f"  {sid:14} {per[sid]:6} ROMs  {human(size[sid]):>10}  -> {folder}/")
    budget = f" of {human(col.max_size)} budget" if col.max_size else ""
    print(f"  total: {len(sel.roms)} ROMs, {human(sel.size)}{budget}; "
          f"{len(sel.rejected)} filtered out; {len(sel.over_budget)} over budget")


def cmd_export(args):
    lib = _library(args)
    col = Collection.load(lib, args.name)
    profile = profiles.get(col.profile)
    rep = export(lib, col, Path(args.dest), dry_run=args.dry_run, sync=args.sync,
                 art=not args.no_art, force=args.force, bios=not args.no_bios)
    if args.verbose or args.dry_run:
        for p in rep.planned[:200 if not args.verbose else None]:
            print(("would write " if args.dry_run else "wrote ") + p)
    verb = "Would copy" if args.dry_run else "Copied"
    print(f"\n{verb} {rep.copied} file(s) ({human(rep.bytes_copied)}), "
          f"{rep.unchanged} already up to date, {rep.art} image(s), "
          f"{rep.art_missing} without art" + (f", removed {rep.removed} stale file(s)"
                                               if rep.removed else "") + ".")
    if rep.bios_copied:
        print(f"BIOS: {'would copy' if args.dry_run else 'copied'} "
              + ", ".join(rep.bios_copied))
    for m in rep.bios_missing_required:
        print(f"warning: missing REQUIRED BIOS for {m} — add it with 'rom-manager bios import'")
    if rep.bios_missing_optional and args.verbose:
        print("optional BIOS not in library: " + "; ".join(rep.bios_missing_optional))
    for w in rep.warnings:
        print("warning: " + w)
    if profile.notes and not args.dry_run:
        print(f"\n{profile.label}: {profile.notes}")


def _print_bios_status(statuses):
    marks = {"ok": "✓", "missing": "✗", "bad_hash": "!", "misnamed": "~"}
    problems = 0
    for rs in statuses:
        kind = "required" if rs.req.required else "optional"
        any_note = " (any one)" if rs.req.any_of else ""
        verdict = "OK" if rs.satisfied else ("MISSING" if rs.req.required else "not present")
        if not rs.satisfied and rs.req.required:
            problems += 1
        print(f"{systems.BY_ID[rs.req.system].name} — {kind}{any_note}: {verdict}")
        for fs in rs.files:
            extra = f"  {fs.detail}" if fs.detail else ""
            print(f"   {marks[fs.state]} {fs.file.name}{extra}")
        if rs.req.note and not rs.satisfied:
            print(f"     {rs.req.note}")
    return problems


def cmd_bios(args):
    if args.action == "list":
        for r in bios.requirements_for(_systems_arg(args.systems) or None):
            files = (" or " if r.any_of else ", ").join(f.name for f in r.files)
            print(f"{r.system:14} {'required' if r.required else 'optional':9} {files}")
            if r.note:
                print(f"{'':25}{r.note}")
        return
    if args.action == "import":
        lib = _library(args)
        if not args.paths:
            raise LibraryError("Give one or more files/folders to import BIOS files from.")
        results = bios.import_files([Path(p) for p in args.paths], lib.root / "bios",
                                    args.dry_run)
        for src, outcome, detail in results:
            if outcome != "present" or args.verbose:
                print(f"{outcome:9} {src}  ({detail})")
        n = sum(1 for _, o, _ in results if o == "imported")
        print(f"\n{'Would import' if args.dry_run else 'Imported'} {n} BIOS file(s) into "
              f"{lib.root / 'bios'}. Files that aren't known BIOS dumps are ignored.")
        return
    # check
    if args.paths:
        folder = Path(args.paths[0]).expanduser().resolve()
    else:
        folder = _library(args).root / "bios"
    statuses = bios.check(folder, _systems_arg(args.systems) or None, layout=args.profile)
    where = f"{folder} ({profiles.get(args.profile).label} layout)" if args.profile else folder
    print(f"BIOS check: {where}\n")
    problems = _print_bios_status(statuses)
    print(f"\n{problems} system(s) missing a required BIOS." if problems
          else "\nAll required BIOS files are present.")


def cmd_systems(args):
    prof = profiles.get(args.profile) if args.profile else None
    print(f"{'ID':16} {'Name':34} {'Folder' if prof else 'Extensions'}")
    for s in systems.SYSTEMS:
        right = str(prof.rom_dir(s, {})) if prof else " ".join(s.exts + s.extra_exts)
        print(f"{s.id:16} {s.name:34} {right}")


def cmd_profiles(args):
    for p in profiles.PROFILES.values():
        print(f"{p.id:17} {p.label}\n{'':17} devices: {p.devices}\n{'':17} {p.notes}\n")


# ------------------------------------------------------------------ parser
def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("-L", "--library", default=argparse.SUPPRESS,
                        help="library folder (or set ROM_MANAGER_LIBRARY)")
    common.add_argument("-v", "--verbose", action="store_true")

    def add_filters(p, clean=False):
        p.add_argument("--junk", action="store_true",
                       help="exclude betas, protos, demos, samples, kiosk, BIOS, bad dumps, "
                            "trainers and pirates")
        p.add_argument("--exclude", metavar="FLAGS",
                       help=f"comma list of flags to exclude: {', '.join(naming.ALL_FLAGS)}")
        p.add_argument("--regions", help="keep only these regions, e.g. USA,World,Europe")
        p.add_argument("--languages", help="keep only these languages, e.g. en")
        p.add_argument("--exclude-title", action="append", metavar="GLOB",
                       help="exclude titles matching a glob, e.g. '*pinball*' (repeatable)")
        if not clean:
            p.add_argument("--include", action="append", metavar="GLOB",
                           help="only include titles matching a glob (repeatable)")
        p.add_argument("--1g1r", dest="one_g1r", action="store_true",
                       help="one game, one ROM: keep the best region/revision of each game")
        p.add_argument("--region-priority", metavar="LIST",
                       help=f"1G1R preference (default {','.join(filters.DEFAULT_REGION_PRIORITY)})")

    ap = argparse.ArgumentParser(
        prog="rom-manager",
        description="Organise retro game ROMs, remove duplicates, scrape artwork and export "
                    "ready-to-copy SD card collections for handhelds.")
    ap.add_argument("--version", action="version", version=__version__)
    ap.add_argument("-L", "--library", help="library folder (or set ROM_MANAGER_LIBRARY)")
    sub = ap.add_subparsers(dest="command", required=True)

    p = sub.add_parser("init", help="create a new library folder")
    p.add_argument("path")
    p.set_defaults(func=cmd_init)

    p = sub.add_parser("import", parents=[common], help="sort ROMs from folders into the library")
    p.add_argument("sources", nargs="+", help="files or folders to import")
    p.add_argument("--move", action="store_true", help="move instead of copy")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--system", help="treat every file as this system (for ambiguous files)")
    p.add_argument("--unsorted", action="store_true",
                   help="copy files with unknown systems into LIBRARY/unsorted")
    p.set_defaults(func=cmd_import)

    p = sub.add_parser("stats", parents=[common], help="ROM counts and sizes per system")
    p.set_defaults(func=cmd_stats)

    p = sub.add_parser("list", parents=[common], help="list ROMs in the library")
    p.add_argument("--systems")
    p.add_argument("--search")
    p.add_argument("--trashed", action="store_true")
    p.set_defaults(func=cmd_list)

    p = sub.add_parser("clean", parents=[common],
                       help="find duplicates and unwanted ROMs and move them to the trash")
    p.add_argument("--systems")
    p.add_argument("--duplicates", action="store_true",
                   help="byte-identical duplicates (always checked when cleaning)")
    add_filters(p, clean=True)
    p.add_argument("--apply", action="store_true", help="actually move files (default: report)")
    p.set_defaults(func=cmd_clean)

    p = sub.add_parser("trash", parents=[common], help="list, restore or empty trashed ROMs")
    p.add_argument("action", choices=["list", "restore", "empty"])
    p.add_argument("ids", nargs="*", help="ROM ids to restore (default: all)")
    p.add_argument("-y", "--yes", action="store_true")
    p.set_defaults(func=cmd_trash)

    p = sub.add_parser("scrape", parents=[common], help="download artwork into the library")
    p.add_argument("--systems")
    p.add_argument("--art", default="boxart",
                   help="comma list: boxart, snap, title, mix, wheel (default boxart)")
    p.add_argument("--source", default="libretro",
                   help="comma list in priority order: libretro, screenscraper")
    p.add_argument("--refresh", action="store_true", help="retry ROMs already scraped")
    p.add_argument("--match-cutoff", type=float, default=0.92,
                   help="fuzzy name match strictness for libretro, 0-1 (default 0.92)")
    p.set_defaults(func=cmd_scrape)

    p = sub.add_parser("collection", parents=[common], help="create and inspect SD card collections")
    p.add_argument("action", choices=["create", "list", "show", "preview", "delete"])
    p.add_argument("name", nargs="?")
    p.add_argument("--profile", choices=list(profiles.PROFILES), default="emulationstation")
    p.add_argument("--systems", help="comma list of system ids (default: all)")
    add_filters(p)
    p.add_argument("--max-size", help="size budget, e.g. 64G — the card's usable space")
    p.add_argument("--priority", action="append", metavar="GLOB",
                   help="titles to fit first under the size budget (repeatable)")
    p.add_argument("--art-kind", choices=["boxart", "snap", "title", "mix", "wheel"])
    p.add_argument("--no-m3u", action="store_true", help="don't build .m3u playlists for discs")
    p.add_argument("--folder", action="append", metavar="SYSTEM=NAME",
                   help="override a system's folder name on the card (repeatable)")
    p.add_argument("--overwrite", action="store_true")
    p.set_defaults(func=cmd_collection)

    p = sub.add_parser("export", parents=[common], help="copy a collection onto an SD card")
    p.add_argument("name")
    p.add_argument("dest", help="SD card root (or any folder)")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--sync", action="store_true",
                   help="remove files this collection put on the card earlier but no longer "
                        "includes (only touches files it wrote)")
    p.add_argument("--no-art", action="store_true")
    p.add_argument("--force", action="store_true", help="skip the free-space check")
    p.add_argument("--no-bios", action="store_true", help="don't copy BIOS files")
    p.set_defaults(func=cmd_export)

    p = sub.add_parser("bios", parents=[common],
                       help="see which BIOS files systems need, check a folder or card, "
                            "and import BIOS files into the library")
    p.add_argument("action", choices=["list", "check", "import"])
    p.add_argument("paths", nargs="*",
                   help="check: folder or SD card root (default: the library's BIOS); "
                        "import: files/folders to import from")
    p.add_argument("--systems")
    p.add_argument("--profile", choices=list(profiles.PROFILES),
                   help="check: treat the folder as an SD card for this firmware")
    p.add_argument("--dry-run", action="store_true")
    p.set_defaults(func=cmd_bios)

    p = sub.add_parser("systems", help="list supported systems")
    p.add_argument("--profile", choices=list(profiles.PROFILES),
                   help="show the folder each system uses for this firmware")
    p.set_defaults(func=cmd_systems)

    p = sub.add_parser("profiles", help="list supported handheld firmwares")
    p.set_defaults(func=cmd_profiles)
    return ap


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "collection" and args.action != "list" and not args.name:
        print("error: collection name required", file=sys.stderr)
        return 2
    try:
        args.func(args)
    except (LibraryError, KeyError, ValueError) as e:
        msg = e.args[0] if isinstance(e, KeyError) and e.args else e
        print(f"error: {msg}", file=sys.stderr)
        return 1
    except BrokenPipeError:
        sys.stderr.close()
        return 0
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        return 130
    return 0
