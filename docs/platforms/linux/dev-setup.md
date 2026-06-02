---
summary: "Build and run the OpenClaw Linux companion app from source"
read_when:
  - Developing the Linux desktop companion
  - Packaging for CachyOS/Arch
title: "Linux app dev setup"
---

# Linux companion dev setup

Native Linux companion sources live under `apps/linux/` (Tauri 2 + Rust).

## Prerequisites (CachyOS / Arch)

```bash
sudo pacman -S --needed \
  base-devel \
  rust \
  webkit2gtk-4.1 \
  gtk3 \
  libayatana-appindicator3 \
  pipewire \
  xdg-desktop-portal \
  libnotify \
  grim \
  npm \
  nodejs \
  pnpm
```

Optional capture tools: `fswebcam` (camera), `gnome-screenshot`, ImageMagick (`import`).

Optional location tools: `gpsd` + `gpspipe`, or GeoClue2 (`geoclue` + `busctl`).

The dashboard opens the gateway Control UI at `http://127.0.0.1:<port>/` (respecting `gateway.controlUi.basePath` from `~/.openclaw/openclaw.json`) and injects `__OPENCLAW_NATIVE_CONTROL_AUTH__` like the macOS app.

**Remote mode:** set `gateway.remote.sshTarget` (and optional `gateway.remote.sshIdentity`) in `openclaw.json`, switch Connection → Remote, then **Save** or **Start SSH tunnel**. Saving connection settings or starting the tunnel restarts the operator and node WebSocket clients. Direct `gateway.remote.url` (`transport: direct` or non-loopback host) skips the tunnel.

**Screen capture:** node `screen.snapshot` tries xdg-desktop-portal Screenshot (session D-Bus via `zbus`), then `grim`, `gnome-screenshot`, and ImageMagick `import`.

**Canvas A2UI:** run `pnpm canvas:a2ui:bundle` (builds `extensions/canvas/src/host/a2ui` and copies to `dist/canvas-host/a2ui`). The Linux app also resolves the plugin source tree, so dev works after bundle alone. For packaged/offline builds, `scripts/package-linux-app.sh` copies `dist/canvas-host/a2ui` → `apps/linux/resources/a2ui`.

**App settings:** `~/.openclaw/linux-app-settings.json` stores connection mode/host/port/token, SSH fields, and voice wake/talk flags.

Install the gateway separately (`npm i -g openclaw@latest` or from this repo via `pnpm build`).

## Build

From repo root:

```bash
pnpm install
pnpm ui:build
pnpm build
pnpm linux:build
```

Run in dev:

```bash
pnpm linux:dev
```

`linux:dev` serves the built UI from `apps/linux/openclaw-app/ui/dist` (not `localhost:1420`). For Vite HMR + `tauri dev`, use `pnpm linux:dev:hot` (requires `cargo install tauri-cli`).

On first launch the app opens the **Dashboard** (Control UI). If the gateway URL or auth cannot be resolved, it falls back to **Settings**. Tray → Settings or left-click tray toggles dashboard. Window creation runs on the GTK main thread to avoid Wayland `Error 71 (Protocol error)` crashes on KDE.

**Gateway autostart (default on):** In Settings → General, **Start gateway when OpenClaw launches** is stored in `~/.openclaw/linux-app-settings.json` (`gatewayAutostart`). On launch the app runs `openclaw gateway status --json` first; if the service is not running it runs `openclaw gateway start` and waits before showing the window.

**Gateway required** for Dashboard and operator tabs: install the service once with **Install gateway service** or `openclaw gateway install`. Without a running gateway, **Open Dashboard** fails on port 18789.

If the webview still fails on Wayland, try:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm linux:dev
# or
GDK_BACKEND=x11 pnpm linux:dev
```

## Environment

- `OPENCLAW_REPO_ROOT` — points the app at `dist/control-ui`, `dist/canvas-host/a2ui`, and canvas scaffold paths when running an uninstalled build.
- `OPENCLAW_BIN` — override path to `openclaw` CLI (default: `openclaw` on `PATH`).

## Packaging

- Arch/CachyOS: `apps/linux/packaging/aur/PKGBUILD`
- Flatpak: `apps/linux/packaging/flatpak/ai.openclaw.linux.yml` (stub)

## Parity tracking

See `apps/linux/PARITY.md` for macOS feature matrix and test IDs.

## Security

Exec approvals / `system.run` changes require review per `apps/linux/SECURITY.md`.
