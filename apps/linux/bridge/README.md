# Linux automation bridge (Peekaboo parity)

Planned host for UI automation on Linux: AT-SPI, xdg-desktop-portal screenshots, window listing.

Agents should treat this as the Linux equivalent of macOS PeekabooBridge when the companion app is running and the bridge socket is enabled.

Status: **partial** — `crates/bridge` exports `bridge_status()` and default socket path (`~/.openclaw/bridge/peekaboo.sock`). Host server not started yet; Debug tab shows status via `get_automation_bridge_status`.
