#!/bin/sh
# Builds dist/BibiCord-<version>.dmg: one-file macOS installer (Apple Silicon,
# macOS 14+). Open it and drag BibiCord into Applications.
# Usage: scripts/build-macos-dmg.sh [path/to/ffmpeg]
set -eu
cd "$(dirname "$0")/.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
scripts/bundle-macos.sh "$@"

# dmgbuild lays out the Finder window without scripting Finder.
# Prefer an installed one, else uv, else a venv.
DMGBUILD=$(command -v dmgbuild || true)
if [ -z "$DMGBUILD" ] && command -v uvx >/dev/null 2>&1; then
  DMGBUILD="uvx --python 3.12 dmgbuild"
fi
if [ -z "$DMGBUILD" ]; then
  VENV=target/pkg/venv
  if [ ! -x "$VENV/bin/dmgbuild" ]; then
    rm -rf "$VENV"
    PY=$(command -v python3.12 || command -v python3.13 || command -v python3)
    "$PY" -m venv "$VENV"
    "$VENV/bin/pip" -q install dmgbuild
  fi
  DMGBUILD="$VENV/bin/dmgbuild"
fi
OUT="dist/BibiCord-$VERSION.dmg"
rm -f "$OUT"
$DMGBUILD \
  -s packaging/macos/dmg_settings.py \
  -D app="$(pwd)/dist/BibiCord.app" \
  -D background="$(pwd)/packaging/macos/dmg-background.png" \
  -D volicon="$(pwd)/packaging/icon/BibiCord.icns" \
  "BibiCord" "$OUT"
hdiutil verify -quiet "$OUT"
ls -lh "$OUT"
