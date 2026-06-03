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

Optional capture tools: `fswebcam` (camera), `gnome-screenshot`, ImageMagick (`import`), `wf-recorder` (Wayland screen record), `ffmpeg` (X11 `x11grab` screen record).

Optional voice capture: `pipewire` (`pw-record`) or `pulseaudio-utils` (`parecord`) for push-to-talk.

Optional location tools: `gpsd` + `gpspipe`, or GeoClue2 (`geoclue` + `busctl`).

The dashboard opens the gateway Control UI at `http://127.0.0.1:<port>/` (respecting `gateway.controlUi.basePath` from `~/.openclaw/openclaw.json`) and injects `__OPENCLAW_NATIVE_CONTROL_AUTH__` like the macOS app.

**Remote mode:** set `gateway.remote.sshTarget` (and optional `gateway.remote.sshIdentity`) in `openclaw.json`, switch Connection → Remote, then **Save** or **Start SSH tunnel**. Saving connection settings or starting the tunnel restarts the operator and node WebSocket clients. Direct `gateway.remote.url` (`transport: direct` or non-loopback host) skips the tunnel. While remote + SSH tunnel is active, the app watches the `ssh` child every 15s and restarts it with backoff if it exits.

**Screen capture:** node `screen.snapshot` tries xdg-desktop-portal Screenshot (session D-Bus via `zbus`), then `grim`, `gnome-screenshot`, and ImageMagick `import`.

**Media payloads:** node `screen.snapshot`, `camera.snap`, `camera.clip`, and `canvas.snapshot` return `{ format, base64 }` (plus `durationMs` / `hasAudio` on `camera.clip` and `screen.record`). `screen.record` prefers `wf-recorder` (~2s WebM) on Wayland, else `ffmpeg` x11grab (MP4 on X11). `canvas.snapshot` retries webview PNG capture up to three times, returns `source: webview` on success, or `source: screen` with a `note` explaining the webview fallback. This matches `openclaw nodes screen/camera` and [Nodes](/nodes/index.md).

**App icons:** optional `scripts/linux-app-icons.sh` (ImageMagick `magick`/`convert`) regenerates `apps/linux/openclaw-app/src-tauri/icons/` from `apps/macos/Icon.icon/Assets/openclaw-mac.png`. `scripts/package-linux-app.sh` runs it when present; missing ImageMagick copies the source PNG without failing the build.

**Push-to-talk:** `talk.ptt.*` records microphone audio via `pw-record` (PipeWire) or `parecord` (PulseAudio). Install `pipewire` / `pulseaudio-utils` on the host; transcription is not wired yet — stop payloads include `transcript: ""` and `audioBase64` when capture succeeds.

**Canvas A2UI:** run `pnpm canvas:a2ui:bundle` (builds `extensions/canvas/src/host/a2ui` and copies to `dist/canvas-host/a2ui`). The Linux app also resolves the plugin source tree, so dev works after bundle alone. For packaged/offline builds, `scripts/package-linux-app.sh` copies `dist/canvas-host/a2ui` → `apps/linux/resources/a2ui`.

**App settings:** `~/.openclaw/linux-app-settings.json` stores connection mode/host/port/token, SSH fields, voice wake/talk flags, and `last_webchat_session` (restored when opening WebChat from the tray or General without an explicit session).

**Operator health:** background 30s health probes reconnect operator/node WebSocket clients after gateway restarts (see `linux-gateway-002`, `linux-gateway-004` in `apps/linux/tests/README.md`). Debug tab shows full `get_connection_health` JSON.

**CLI install:** Settings → General → **Install CLI (official script)** runs `https://openclaw.bot/install-cli.sh` into `~/.openclaw` (same flow as the macOS app). `cli_installed_location` checks `~/.openclaw/bin/openclaw` and common PATH entries. Failures show exit code and stderr on the General tab status line (not toast-only).

**Deep links:** `openclaw://dashboard`, `openclaw://webchat`, `openclaw://webchat?session=…`, `openclaw://settings`, `openclaw://gateway?host=…&port=…`, `openclaw://agent?message=…`, `openclaw://agent?message=…&session=…` (WebChat with session query), `openclaw://canvas?session=…`. Gateway links on LAN hosts set local mode + direct WebSocket and restart operator/node clients. Agent links without `session` open the dashboard; with `session` they open WebChat.

**Gateway version:** Settings → General and About show the running gateway version from `openclaw gateway status --json` when available, or from the operator WebSocket hello when connected. Use **Refresh gateway version** on General after upgrades.

**Dashboard chat:** General, Sessions, and Voice include **Open chat in Dashboard** (`/chat` with optional `session` query), separate from the dedicated WebChat window.

**Tray:** menu includes **Reconnect gateway** (restarts operator/node clients and reconnects the operator WS) and **About** (opens Settings on the About tab). The tray tooltip shows `operator ✓/✗ · node ✓/✗` (and tunnel when active) plus the latest status line. Connection and Debug tabs expose the same reconnect action.

**Debug:** shows a live connection health summary (`operatorHealthOk` from the 30s health probe) and **Connection health (JSON)** for the full payload.

**WebChat:** dedicated window loads Control UI `/chat` with optional `session` query. Window title includes the session key when set. Last session persists in `linux-app-settings.json` and is restored from tray **Open WebChat** or General **Open WebChat** when no session is passed. General tab includes a **WebChat session** picker (dropdown from `sessions.list` when the operator is connected) for **Open WebChat** and **Open chat in Dashboard**.

**Connection tab:** optional **Use TLS** for `wss`/`https` with a live WebSocket URL hint; pairing pending list refreshes every 15s while the tab is open. **Approve all** / **Reject all** device/node batch buttons process every pending row of that kind. mDNS **Discover gateways** lists deduped gateways with friendly names; **Use & save** applies host/port/mode and reconnects. SSH tunnel state and watchdog errors (disconnect, restart failed, reconnecting) appear on the tunnel status line and the prominent connection status bar; use **Restart SSH tunnel** for a manual stop/start cycle. **Sessions** tab can open WebChat or Dashboard chat, load `sessions.preview`, or **Describe** (`sessions.describe`) per row.

**Operator tabs (Skills, Instances, Sessions, Config, …):** structured summaries with **Filter list** search on Skills/Instances/Sessions, **Copy JSON**, and raw JSON in a collapsed **View raw JSON** section.

**Permissions tab:** camera/screen/location toggles save to `linux-app-settings.json` and refresh the capability probe JSON after each change.

**Exec tab:** pending gateway CLI exec prompts list with Allow/Deny cards while the node is connected; new requests focus the Exec tab and highlight the matching row (desktop notification + confirm dialog still fire).

**Channels / Cron tabs:** operator summaries show colored status badges; Channels action rows include Start/Stop per channel id.

**Voice tab:** three sections — wake settings only (no Linux listener), push-to-talk capture (implemented), talk mode partial (gateway flags + WebChat/Dashboard chat links).

Install the gateway separately (`openclaw` on PATH or **Install CLI** above).

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
