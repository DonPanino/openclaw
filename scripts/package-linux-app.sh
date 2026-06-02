#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required to build the Linux companion app" >&2
  exit 1
fi

if [[ ! -f dist/control-ui/index.html ]]; then
  echo "Building Control UI (pnpm ui:build)..." >&2
  pnpm ui:build
fi

echo "Building Linux companion UI..." >&2
pnpm --dir apps/linux/openclaw-app/ui install
pnpm --dir apps/linux/openclaw-app/ui build

CANVAS_SCAFFOLD_SRC="apps/shared/OpenClawKit/Sources/OpenClawKit/Resources/CanvasScaffold"
CANVAS_SCAFFOLD_DST="apps/linux/resources/canvas-scaffold"
if [[ -d "$CANVAS_SCAFFOLD_SRC" ]]; then
  rm -rf "$CANVAS_SCAFFOLD_DST"
  mkdir -p "$(dirname "$CANVAS_SCAFFOLD_DST")"
  cp -a "$CANVAS_SCAFFOLD_SRC" "$CANVAS_SCAFFOLD_DST"
fi

if [[ -f package.json ]]; then
  pnpm canvas:a2ui:bundle || true
fi

A2UI_SRC="dist/canvas-host/a2ui"
A2UI_DST="apps/linux/resources/a2ui"
if [[ -d "$A2UI_SRC" ]]; then
  rm -rf "$A2UI_DST"
  mkdir -p "$(dirname "$A2UI_DST")"
  cp -a "$A2UI_SRC" "$A2UI_DST"
fi

export OPENCLAW_REPO_ROOT="$ROOT_DIR"
cd apps/linux
cargo build --release -p openclaw-linux-app-tauri

echo "Built: apps/linux/target/release/openclaw-linux-app" >&2
