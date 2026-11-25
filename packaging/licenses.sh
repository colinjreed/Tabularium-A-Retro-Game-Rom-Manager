#!/usr/bin/env bash
# Put the license files every download must carry into the folder given:
# Tabularium's own license (GPL-3.0), the licenses of the Rust libraries it's
# built with, and the bundled fonts' licenses.
# Needs cargo-about (cargo install cargo-about --locked --features cli).
set -euo pipefail
out="$1"
mkdir -p "$out/licenses"
cp LICENSE "$out/LICENSE.txt"
cargo about generate --locked --fail about.hbs -o "$out/licenses/third-party-licenses.html"
cp assets/fonts/OFL-*.txt "$out/licenses/"
