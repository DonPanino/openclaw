#!/usr/bin/env bash
# Regenerate Tauri app icons from apps/macos/Icon.icon/Assets/openclaw-mac.png
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${ROOT_DIR}/apps/macos/Icon.icon/Assets/openclaw-mac.png"
DST_DIR="${ROOT_DIR}/apps/linux/openclaw-app/src-tauri/icons"

if [[ ! -f "$SRC" ]]; then
  echo "linux-app-icons: source missing: $SRC" >&2
  exit 0
fi

mkdir -p "$DST_DIR"

if command -v magick >/dev/null 2>&1; then
  magick "$SRC" -alpha on -resize 512x512 PNG32:"${DST_DIR}/icon.png"
  magick "$SRC" -alpha on -resize 32x32 PNG32:"${DST_DIR}/32x32.png"
  magick "$SRC" -alpha on -resize 128x128 PNG32:"${DST_DIR}/128x128.png"
  echo "linux-app-icons: wrote icons via ImageMagick magick" >&2
elif command -v convert >/dev/null 2>&1; then
  convert "$SRC" -alpha on -resize 512x512 PNG32:"${DST_DIR}/icon.png"
  convert "$SRC" -alpha on -resize 32x32 PNG32:"${DST_DIR}/32x32.png"
  convert "$SRC" -alpha on -resize 128x128 PNG32:"${DST_DIR}/128x128.png"
  echo "linux-app-icons: wrote icons via ImageMagick convert" >&2
else
  cp -f "$SRC" "${DST_DIR}/icon.png"
  cp -f "$SRC" "${DST_DIR}/32x32.png"
  cp -f "$SRC" "${DST_DIR}/128x128.png"
  echo "linux-app-icons: ImageMagick not found; copied source PNG to all sizes" >&2
fi
