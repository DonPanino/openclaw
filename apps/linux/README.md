# OpenClaw Linux companion

Native menu-bar/tray companion for OpenClaw on Linux (CachyOS/Arch, Wayland-first).

## Prerequisites

- Node 22.19+ (24 recommended) and pnpm for the gateway/Control UI
- Rust stable, Tauri 2 system deps (see [dev setup](../../docs/platforms/linux/dev-setup.md))

## Build

From repo root:

```bash
pnpm install
pnpm ui:build
pnpm build
pnpm linux:build
```

Binary: `apps/linux/target/release/openclaw-linux-app` (or debug under `target/debug/`).

## Run

```bash
pnpm linux:dev
```

Ensure a gateway is reachable (`openclaw gateway status`) or use the app to install the systemd user service.

## Architecture

- **Tauri 2** shell (tray, windows, CLI integration)
- **Rust crates** for gateway protocol, node host, capture, voice
- **WebKitGTK** for Control UI, WebChat, and Canvas surfaces
- Gateway lifecycle delegated to `openclaw` CLI + systemd

See [PARITY.md](./PARITY.md) for macOS feature tracking.
