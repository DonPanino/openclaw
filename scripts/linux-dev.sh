#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
export OPENCLAW_REPO_ROOT="$ROOT_DIR"

LINUX_APP_UI="$ROOT_DIR/apps/linux/openclaw-app/ui"
LINUX_RESOURCES_A2UI="$ROOT_DIR/apps/linux/resources/a2ui/index.html"
DIST_A2UI="$ROOT_DIR/dist/canvas-host/a2ui/index.html"
CONTROL_UI_INDEX="$ROOT_DIR/dist/control-ui/index.html"

echo "linux-dev: OPENCLAW_REPO_ROOT=$OPENCLAW_REPO_ROOT"

MAC_ICON="$ROOT_DIR/apps/macos/Icon.icon/Assets/openclaw-mac.png"
LINUX_ICON="$ROOT_DIR/apps/linux/openclaw-app/src-tauri/icons/icon.png"
if [[ -f "$MAC_ICON" ]] && [[ -x "$ROOT_DIR/scripts/linux-app-icons.sh" ]]; then
  if [[ ! -f "$LINUX_ICON" ]] || [[ "$MAC_ICON" -nt "$LINUX_ICON" ]]; then
    "$ROOT_DIR/scripts/linux-app-icons.sh" || true
  fi
fi

pnpm --dir "$LINUX_APP_UI" install
pnpm --dir "$LINUX_APP_UI" build

if [[ ! -f "$LINUX_RESOURCES_A2UI" && ! -f "$DIST_A2UI" ]]; then
  echo "linux-dev: A2UI bundle missing; running pnpm canvas:a2ui:bundle and copying to apps/linux/resources/a2ui"
  pnpm canvas:a2ui:bundle
  mkdir -p "$(dirname "$LINUX_RESOURCES_A2UI")"
  cp -a "$ROOT_DIR/dist/canvas-host/a2ui" "$(dirname "$LINUX_RESOURCES_A2UI")"
elif [[ ! -f "$LINUX_RESOURCES_A2UI" && -f "$DIST_A2UI" ]]; then
  echo "linux-dev: copying dist/canvas-host/a2ui -> apps/linux/resources/a2ui"
  mkdir -p "$(dirname "$LINUX_RESOURCES_A2UI")"
  cp -a "$ROOT_DIR/dist/canvas-host/a2ui" "$(dirname "$LINUX_RESOURCES_A2UI")"
fi

if [[ ! -f "$CONTROL_UI_INDEX" ]]; then
  echo "linux-dev: warning: Control UI dist missing at dist/control-ui/index.html (run pnpm ui:build from repo root); dashboard may fail until built" >&2
fi

cd "$ROOT_DIR/apps/linux"
TAURI_DEV=false cargo run -p openclaw-linux-app-tauri
