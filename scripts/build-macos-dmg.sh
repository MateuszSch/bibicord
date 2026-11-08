#!/bin/sh
# Builds dist/BibiCord-<version>.dmg: one-file macOS installer (Apple Silicon,
# macOS 14+). Open it and drag BibiCord into Applications.
# Usage: scripts/build-macos-dmg.sh [path/to/ffmpeg]
set -eu
cd "$(dirname "$0")/.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
scripts/bundle-macos.sh "$@"

# dmgbuild lays out the Finder window without scripting Finder.
VENV=target/pkg/venv
if [ ! -x "$VENV/bin/dmgbuild" ]; then
  python3 -m venv "$VENV"
  "$VENV/bin/pip" -q install dmgbuild
fi
OUT="dist/BibiCord-$VERSION.dmg"
rm -f "$OUT"
"$VENV/bin/dmgbuild" \
  -s packaging/macos/dmg_settings.py \
  -D app="$(pwd)/dist/BibiCord.app" \
  -D background="$(pwd)/packaging/macos/dmg-background.png" \
  -D volicon="$(pwd)/packaging/icon/BibiCord.icns" \
  "BibiCord" "$OUT"
hdiutil verify -quiet "$OUT"
ls -lh "$OUT"
