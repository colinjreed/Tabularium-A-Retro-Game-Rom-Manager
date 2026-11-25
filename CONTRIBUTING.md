# Contributing to Tabularium

Thanks for helping. Bug reports, system or firmware requests, and pull
requests are all welcome.

## Reporting a problem

Open an issue with:

* what you did and what happened instead,
* your operating system and Tabularium version (Settings → About),
* for import or detection problems, the file names involved and the folder
  they were in. Please don't attach ROM or BIOS files.

## Building

You need Rust 1.95 or newer (`rustup update`).

```sh
cargo run --release          # run the app
cargo test --all             # run the tests (no network needed)
```

Before sending a pull request, please run:

```sh
cargo fmt --all
cargo clippy --all-targets
cargo test --all
```

The build on GitHub runs the same checks on Windows, macOS and Linux.

## Where things live

* `crates/core`: everything that isn't UI (system detection, the library
  database, cleaning, renaming, artwork, BIOS checks, SD card export), with
  tests in `crates/core/tests`.
* `crates/app`: the egui desktop app; each page is in `crates/app/src/pages`,
  and the themes are in `crates/app/src/themes`.
* `crates/core/src/systems.rs`: the list of supported systems, their folder
  names for each firmware, and their extensions. Most "please support system
  X" changes start here.

## Dependencies and licensing

Tabularium is licensed under the GPL-3.0 or later, and contributions are
accepted under the same license. New dependencies must use a license listed
in `about.toml` (MIT, Apache-2.0, BSD and similar); release packaging fails
otherwise.

Never commit ROMs, BIOS files, ScreenScraper credentials or any other
secrets.
