#!/bin/bash
# Build "Tabularium.app" and a .dmg on your own Mac (Apple Silicon or Intel).
# Needs: Xcode command-line tools (xcode-select --install) and Rust (https://rustup.rs).
# Usage: ./packaging/macos/build-local.sh        -> dist/tabularium-macos-universal.dmg
set -euo pipefail
cd "$(dirname "$0")/../.."
export MACOSX_DEPLOYMENT_TARGET=11.0

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin

rm -rf dist && mkdir -p dist
APP="dist/Tabularium.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/tabularium" \
  target/aarch64-apple-darwin/release/tabularium \
  target/x86_64-apple-darwin/release/tabularium
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
sed "s/VERSION/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp assets/icon.icns "$APP/Contents/Resources/icon.icns"
codesign --force --deep --sign - "$APP"   # ad-hoc signature, not notarised

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Tabularium" -srcfolder "$STAGE" -ov -format UDZO dist/tabularium-macos-universal.dmg
rm -rf "$STAGE"
echo "Done: dist/Tabularium.app and dist/tabularium-macos-universal.dmg"
