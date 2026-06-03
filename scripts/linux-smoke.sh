#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "Linux companion smoke: protocol version contract"
cargo test -p openclaw-protocol --manifest-path apps/linux/Cargo.toml protocol_version_matches_gateway_package

echo "Linux companion smoke: gateway_config last_webchat_session round-trip"
cargo test -p openclaw-kit --manifest-path apps/linux/Cargo.toml remember_webchat_session_round_trip

echo "Linux companion smoke: workspace unit tests"
cargo test --manifest-path apps/linux/Cargo.toml --exclude openclaw-linux-app-tauri

if command -v openclaw >/dev/null 2>&1; then
  echo "Linux companion smoke: gateway status"
  openclaw gateway status --json || true
else
  echo "skip gateway status (openclaw CLI not installed)"
fi

echo "linux-smoke: ok"
