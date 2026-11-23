#!/bin/bash
# Build "ROM Manager.app" and a .dmg on your own Mac (Apple Silicon or Intel).
# Needs: Xcode command-line tools (xcode-select --install) and Rust (https://rustup.rs).
# Usage: ./packaging/macos/build-local.sh        -> dist/rom-manager-macos-universal.dmg
set -euo pipefail
cd "$(dirname "$0")/../.."
export MACOSX_DEPLOYMENT_TARGET=11.0

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin

rm -rf dist && mkdir -p dist
APP="dist/ROM Manager.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/rom-manager" \
  target/aarch64-apple-darwin/release/rom-manager \
  target/x86_64-apple-darwin/release/rom-manager
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
sed "s/VERSION/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp assets/icon.icns "$APP/Contents/Resources/icon.icns"
codesign --force --deep --sign - "$APP"   # ad-hoc signature, not notarised

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "ROM Manager" -srcfolder "$STAGE" -ov -format UDZO dist/rom-manager-macos-universal.dmg
rm -rf "$STAGE"
echo "Done: dist/ROM Manager.app and dist/rom-manager-macos-universal.dmg"
