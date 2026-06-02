# Linux companion app

Telegraph style. Native desktop companion for OpenClaw on Linux (CachyOS/Arch-first, Wayland-first).

## Boundary

- Gateway runtime stays in Node (`openclaw` CLI + systemd user unit). The app does not embed Bun/Node or spawn the gateway as a child process.
- App connects as **operator** (Control UI, settings, WebChat) and **node** (canvas, screen, exec) over the Gateway WebSocket protocol.
- Protocol types live in `crates/openclaw-protocol`; behavioral helpers in `crates/openclaw-kit`.
- Core gateway policy changes (`src/gateway/node-command-policy.ts`, `extensions/canvas`) require coordinated PRs when adding node commands.

## Build

```bash
# from repo root
pnpm install
pnpm ui:build
pnpm build
pnpm linux:build
```

Dev:

```bash
pnpm linux:dev
```

## CachyOS deps

`webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator3`, `pipewire`, `xdg-desktop-portal`, `libnotify`, `systemd`, `rust` (stable).

## Security

Exec approvals and `system.run` changes need review per `apps/linux/SECURITY.md`. Do not widen `gateway.nodes.allowCommands` defaults without security sign-off.

## Parity

Feature parity vs macOS is tracked in `apps/linux/PARITY.md`. Mark rows `done` only with test ID proof.

## Docs

- User: `docs/platforms/linux.md`
- Dev: `docs/platforms/linux/dev-setup.md`
