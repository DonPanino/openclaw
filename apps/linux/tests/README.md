# Linux companion tests

| ID | Command |
|----|---------|
| linux-tray-001 | Manual: tray icon + menu on Wayland session |
| linux-gateway-001 | `openclaw gateway status --json` after app install service |
| linux-dashboard-001 | `pnpm linux:dev` → Open Dashboard loads `dist/control-ui` |
| linux-node-001 | `cargo test -p openclaw-protocol --manifest-path apps/linux/Cargo.toml` |
| linux-exec-001 | Edit exec tab → saves `~/.openclaw/exec-approvals.json` |

| linux-gateway-002 | App running → Settings conn bar shows operator/node; survives gateway restart |
| linux-pair-002 | Connection → Pairing → Approve/Reject buttons on pending rows |
| linux-canvas-003 | Debug → Open test canvas; agent `canvas.a2ui.push` updates A2UI host |
| linux-exec-003 | Node `system.execApprovals.get` returns ~/.openclaw/exec-approvals.json |

CI: `pnpm linux:test` runs protocol contract + kit unit tests when Rust is available.
