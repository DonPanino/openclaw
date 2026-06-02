# Linux app security review gate

Changes in these areas require explicit security review before merge:

- `crates/node-host/` exec socket protocol, HMAC/TTL, or allowlist persistence
- `system.run` / `system.which` command resolution and environment merging
- Widening `linux` entries in `src/gateway/node-command-policy.ts` or `extensions/canvas` default platforms
- Portal capture fallbacks that bypass user consent (X11 `grim`, etc.)
- Deep link handlers that trigger agent runs without user confirmation
- Auto-approval defaults in exec-approvals.json migration

## Review checklist

1. Default deny for `system.run`; allowlist + ask-on-miss matches macOS contract.
2. No secret logging in gateway WS debug paths from the app.
3. Remote gateway mode does not disable TLS without explicit operator opt-in.
4. UDS socket path and token are user-only (`0700` dir).
5. Child processes spawned for capture/ASR inherit OOM bias (see gateway Linux OOM doc).

Contact: tag `@openclaw/openclaw-secops` on PRs touching the above.
