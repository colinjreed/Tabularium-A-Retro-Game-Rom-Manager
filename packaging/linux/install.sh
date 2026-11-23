#!/bin/sh
# Install Tabularium for the current user (no root needed).
set -e
cd "$(dirname "$0")"
BIN="${HOME}/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 755 tabularium "$BIN/tabularium"
install -m 644 tabularium.png "$ICONS/tabularium.png"
sed "s|^Exec=tabularium|Exec=$BIN/tabularium|" tabularium.desktop > "$APPS/tabularium.desktop"
echo "Installed. Find 'Tabularium' in your applications menu, or run $BIN/tabularium"
