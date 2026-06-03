# Linux companion tests

| ID | Command |
|----|---------|
| linux-tray-001 | Manual: tray icon + menu on Wayland session |
| linux-tray-002 | Tray hover tooltip shows `operator ✓/✗ · node ✓/✗` (+ tunnel when active) |
| linux-gateway-001 | `openclaw gateway status --json` after app install service |
| linux-dashboard-001 | `pnpm linux:dev` → Open Dashboard loads `dist/control-ui` |
| linux-node-001 | `cargo test -p openclaw-protocol --manifest-path apps/linux/Cargo.toml` |
| linux-exec-001 | Edit exec tab → saves `~/.openclaw/exec-approvals.json` |

| linux-gateway-002 | App running → conn bar + Debug health line show operator/node (+ health probe); survives gateway restart |
| linux-settings-013 | Operator tabs: Instances (system-presence), Skills counts, Sessions table, Config snippet + raw JSON toggle |
| linux-settings-020 | Operator tabs: filter search (Skills/Instances/Sessions), Copy JSON, raw JSON collapsed by default |
| linux-settings-014 | Connection tab: prominent status, tunnel line, Test operator connection, remote_direct checkbox |
| linux-voice-006 | Voice tab Save → `voicewake.set` when operator connected (gateway settings/voicewake.json) |
| linux-pair-002 | Connection → Pairing → Approve/Reject buttons on pending rows |
| linux-pair-003 | `pairing-request` event switches to Connection tab and refreshes pending list before confirm |
| linux-pair-004 | Connection → Pairing → **Approve all device/node** batch buttons |
| linux-pair-005 | Connection → Pairing → **Reject all device/node** batch buttons |
| linux-screen-003 | Node `screen.record` partial (`wf-recorder`/`ffmpeg`; see PARITY) |
| linux-canvas-003 | Debug → Open test canvas; agent `canvas.a2ui.push` updates A2UI host |
| linux-canvas-004 | `canvas.snapshot` retries webview PNG (3×); returns `source: webview|screen` + fallback `note` |
| linux-exec-003 | Node `system.execApprovals.get` returns ~/.openclaw/exec-approvals.json |

| linux-cli-002 | General → Install CLI; `cli_installed_location` shows ~/.openclaw/bin/openclaw |
| linux-cli-004 | General → failed CLI install shows stderr/exit on status line (not toast-only) |
| linux-deeplink-002 | `openclaw://settings` opens settings; `openclaw://gateway?host=…` saves + reconnects |
| linux-discover-001 | Discover → deduped list with friendly service name + LAN hint |
| linux-discover-002 | Discover → **Use & save** on LAN host sets local mode + direct WS and reconnects |
| linux-exec-004 | Exec approval notification focuses Exec tab + confirm |
| linux-exec-006 | Exec tab pending list (Allow/Deny cards) + highlight on `exec-approval-request` |
| linux-tray-003 | Tray → Reconnect gateway; Connection/Debug reconnect buttons |
| linux-tray-005 | Tray → About opens Settings on About tab |
| linux-voice-007 | Voice → Load gateway talk.config (operator connected) |
| linux-bridge-001 | Debug → Bridge status JSON (`planned` state, socket path) |
| linux-deeplink-003 | `openclaw://agent?message=…` opens dashboard + toast |
| linux-deeplink-004 | `openclaw://agent?message=…&session=…` opens WebChat; `openclaw://canvas?session=…` opens canvas |
| linux-exec-005 | Exec tab shows node exec-approvals Unix socket path |
| linux-voice-008 | Voice tab PTT cancel |
| linux-cron-001 | Cron tab → Run job (operator `cron.run`) + `cron.status` |
| linux-cron-002 | Cron tab job list shows enabled/disabled status badges |
| linux-gateway-003 | Save connection / Reconnect restarts operator+node; gateway CLI uses `~/.openclaw/bin/openclaw` when present |
| linux-node-002 | Connection tab shows node `deviceId` |
| linux-gateway-004 | Node WS drop clears slot + status bar shows operator/node/tunnel |
| linux-channels-001 | Channels tab Start/Stop per channel id |
| linux-channels-002 | Channels tab summary + action rows show status badges |
| linux-tray-004 | Header conn bar: `operator ✓ · node ✓` from `get_connection_health` |
| linux-settings-015 | Connection tab TLS checkbox; pairing list auto-refresh every 15s |
| linux-sessions-001 | Sessions tab WebChat + `sessions.preview` per row |
| linux-sessions-002 | Sessions tab Describe → `sessions.describe` (metadata + last message) |
| linux-settings-016 | Connection tab: live `ws`/`wss` URL hint updates with host/port/TLS checkbox |
| linux-settings-017 | General/About: gateway version from `get_gateway_version_info` (CLI status or operator hello) |
| linux-settings-018 | Connection: tunnel failures surface on tunnel status line; `gateway-connection-status` updates prominent status |
| linux-settings-019 | Connection: **Restart SSH tunnel** button; reconnecting state uses warn styling on tunnel line |
| linux-webchat-003 | General/Sessions/Voice → Open chat in Dashboard (`open_dashboard_chat`); Sessions row Dashboard chat |
| linux-deeplink-005 | `openclaw://webchat?session=…` opens WebChat with session query |
| linux-webchat-004 | WebChat window title shows session; tray/general reopen restores `last_webchat_session` in linux-app-settings |
| linux-webchat-005 | General → WebChat session picker (sessions.list dropdown) + Open WebChat / Dashboard chat |
| linux-settings-012 | Permissions tab: capability probe refreshes after toggling camera/screen/location |
| linux-voice-009 | Voice tab: wake (not implemented) vs PTT (implemented) vs talk (WebChat link) fieldsets |
| linux-bridge-002 | Debug → Bridge status shows inline JSON under Automation bridge |

CI: `pnpm linux:test` runs protocol contract + kit unit tests when Rust is available.

Kit: `cargo test -p openclaw-kit remember_webchat_session_round_trip` (also in `scripts/linux-smoke.sh`) verifies `last_webchat_session` save/load round-trip in `linux-app-settings.json`.
