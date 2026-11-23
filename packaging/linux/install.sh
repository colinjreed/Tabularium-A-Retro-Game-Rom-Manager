#!/bin/sh
# Install ROM Manager for the current user (no root needed).
set -e
cd "$(dirname "$0")"
BIN="${HOME}/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 755 rom-manager "$BIN/rom-manager"
install -m 644 rom-manager.png "$ICONS/rom-manager.png"
sed "s|^Exec=rom-manager|Exec=$BIN/rom-manager|" rom-manager.desktop > "$APPS/rom-manager.desktop"
echo "Installed. Find 'ROM Manager' in your applications menu, or run $BIN/rom-manager"
