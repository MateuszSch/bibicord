#!/bin/sh
# Builds dist/BibiCord.app (release binary + bundled FFmpeg).
# Usage: scripts/bundle-macos.sh [path/to/ffmpeg]
set -eu
cd "$(dirname "$0")/.."
cargo build --release
APP=dist/BibiCord.app
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/bibicord "$APP/Contents/MacOS/bibicord"
cp packaging/Info.plist "$APP/Contents/Info.plist"

FF="${1:-}"
if [ -z "$FF" ]; then
  case "$(uname -m)" in
    arm64) URL=https://www.osxexperts.net/ffmpeg9arm.zip ;;
    *)     URL=https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip ;;
  esac
  TMP=$(mktemp -d)
  curl -fsSL -o "$TMP/ffmpeg.zip" "$URL"
  unzip -q -o "$TMP/ffmpeg.zip" -d "$TMP"
  FF="$TMP/ffmpeg"
fi
cp "$FF" "$APP/Contents/Resources/ffmpeg"
chmod +x "$APP/Contents/Resources/ffmpeg"
codesign --force --deep --sign - "$APP"
echo "OK: $APP"
