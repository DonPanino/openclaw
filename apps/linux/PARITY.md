# macOS → Linux parity matrix

Status: `planned` | `partial` | `done`. Test ID links to `apps/linux/tests/` or gateway integration tests.

| Feature | macOS reference | Linux status | Test |
|---------|-----------------|--------------|------|
| Tray + status (menu: reconnect, About; tooltip operator/node/tunnel) | MenuBar.swift | partial | linux-tray-001, linux-tray-002, linux-tray-003, linux-tray-005 |
| Local gateway attach | GatewayProcessManager.swift | done | linux-gateway-001 |
| Remote gateway + tunnel | ConnectionModeCoordinator.swift | partial | linux-remote-001, linux-settings-014 |
| SSH tunnel (`ssh -N -L`) | RemotePortTunnel.swift | partial | linux-remote-002, linux-settings-014 |
| Remote reconnect on save/tunnel | ConnectionModeCoordinator.swift | partial | linux-remote-003, linux-settings-014, linux-gateway-003 |
| Control UI / Dashboard | DashboardManager.swift | done | linux-dashboard-001 |
| Dashboard auth (HTTP + native inject) | DashboardWindowController.swift | done | linux-dashboard-002 |
| WebChat | WebChatSwiftUI.swift | partial | linux-webchat-001, linux-webchat-003, linux-webchat-004 |
| WebChat (Control UI `/chat` + `session` query; title + last session + picker) | WebChatSwiftUI.swift | partial | linux-webchat-002, linux-deeplink-004, linux-deeplink-005, linux-webchat-004, linux-webchat-005 |
| Device identity + WS auth | OpenClawKit device | done | linux-node-001 |
| Operator gateway RPC | ChannelsStore.swift | partial | linux-settings-004, linux-settings-013 |
| Operator/node reconnect + health probe (30s probe loop) | ConnectionModeCoordinator | partial | linux-gateway-002, linux-gateway-004 |
| Pairing approvals (notify-rust on pending; approve/reject all) | DevicePairingApprovalPrompter.swift | partial | linux-pair-001, linux-pair-002, linux-pair-003, linux-pair-004, linux-pair-005 |
| Settings: General (gateway version + autostart) | GeneralSettings.swift | partial | linux-settings-001, linux-settings-017 |
| Settings: Connection (TLS + WSS hint + pairing poll) | ConnectionModeCoordinator | partial | linux-settings-002, linux-settings-014, linux-settings-015, linux-settings-016 |
| Connection errors in UI (status line + tunnel detail) | ConnectionModeCoordinator | partial | linux-settings-011, linux-settings-018 |
| Operator tab summaries | ChannelsStore / sessions / config UIs | partial | linux-settings-013, linux-settings-020 |
| Operator connection test | ConnectionModeCoordinator | partial | linux-settings-014 |
| Settings: Permissions (cap toggles + probe on save) | PermissionsSettings.swift | partial | linux-settings-003, linux-settings-012 |
| Permissions capability probe | PermissionsSettings.swift | partial | linux-settings-012 |
| Settings: Voice (wake settings-only; PTT; talk partial) | VoiceWakeSettings.swift | partial | linux-voice-001, linux-voice-009 |
| Node caps from linux-app-settings | MacNodeModeCoordinator | partial | linux-node-001 |
| Settings: Channels (status + start/stop) | ChannelsSettings.swift | partial | linux-settings-004, linux-channels-001, linux-channels-002 |
| Settings: Skills | SkillsSettings.swift | partial | linux-settings-005, linux-settings-013 |
| Settings: Cron (list + status + run) | CronSettings.swift | partial | linux-settings-006, linux-cron-001, linux-cron-002 |
| Settings: Exec | SystemRunSettingsView.swift | partial | linux-exec-001, linux-exec-006 |
| Exec approval prompts (socket → UI; focuses Exec tab) | ExecApprovalPrompter | partial | linux-exec-002, linux-exec-004, linux-exec-006 |
| Settings: Sessions (list + preview + describe + WebChat) | SessionsSettings.swift | partial | linux-settings-007, linux-settings-013, linux-sessions-001, linux-sessions-002 |
| Settings: Instances | InstancesSettings.swift | partial | linux-settings-008, linux-settings-013 |
| Settings: Config | ConfigSettings.swift | partial | linux-settings-009, linux-settings-013 |
| Settings: Debug | DebugSettings.swift | partial | linux-settings-010, linux-gateway-002 |
| linux:dev prep script | dev-setup.md | done | — |
| Debug: stop gateway | DebugSettings.swift | partial | linux-settings-010 |
| Node: system.run | MacNodeRuntime | partial | linux-node-001 |
| Node: system.notify | NotificationManager.swift | partial | linux-node-002 |
| Node: canvas.* | CanvasManager.swift | partial | linux-canvas-001 |
| Canvas scaffold (shared kit + dist/canvas-host/a2ui) | CanvasManager.swift | partial | linux-canvas-002 |
| Canvas A2UI push/reset/eval | CanvasManager.swift | partial | linux-canvas-003 |
| Canvas snapshot (webview PNG retry; `source` + screen fallback `note`) | CanvasManager.swift | partial | linux-canvas-004 |
| talk.ptt.* (pw-record/parecord WAV; cancel; no STT yet) | VoicePushToTalk.swift | partial | linux-voice-003 |
| system.execApprovals.get/set | MacNodeRuntime | partial | linux-exec-003 |
| Node: screen.* (portal + grim; base64 PNG) | ScreenSnapshotService.swift | partial | linux-screen-001 |
| Node: camera.* (base64 JPEG) | CameraCaptureService.swift | partial | linux-camera-001 |
| Node: camera.clip (ffmpeg/fswebcam burst; base64 + durationMs) | CameraCaptureService.swift | partial | linux-camera-001 |
| Node: screen.record (`wf-recorder`/`ffmpeg`; `{ format, base64, durationMs, hasAudio }`) | ScreenCaptureKit.swift | partial | linux-screen-002, linux-screen-003 |
| Remote tunnel watchdog (ssh child restart; errors in Connection UI) | ConnectionModeCoordinator.swift | partial | linux-remote-004, linux-settings-018, linux-settings-019 |
| Launch: hide settings after dashboard | DashboardManager.swift | done | linux-dashboard-001 |
| Node: location.get | MacNodeLocationService.swift | partial | linux-location-001 |
| Node: location.get (gpspipe/GeoClue) | MacNodeLocationService.swift | partial | linux-location-002 |
| Voice wake | VoiceWakeRuntime.swift | partial | linux-voice-002 |
| Voice settings persistence | VoiceWakeSettings.swift | partial | linux-voice-005, linux-voice-006 |
| Voice wake triggers gateway sync | VoiceWakeGlobalSettingsSync.swift | partial | linux-voice-006 |
| PTT (WAV capture; gateway STT TBD) | VoicePushToTalk.swift | partial | linux-voice-003 |
| Talk mode (talk.config probe + WebChat; no managed-room yet) | TalkModeController.swift | partial | linux-voice-004, linux-voice-007 |
| Pairing prompts | NodePairingApprovalPrompter.swift | partial | linux-pair-001, linux-pair-003 |
| mDNS discovery (deduped host:port; friendly name; Use & save) | GatewayDiscoveryModel.swift | partial | linux-discover-001, linux-discover-002 |
| Deep links (`dashboard`, `webchat`, `settings`, `gateway`, `agent`, `canvas`) | DeepLinks.swift | partial | linux-deeplink-001, linux-deeplink-004 |
| Tray status tooltip (operator/node/tunnel + status line) | MenuBar.swift | partial | linux-tray-002, linux-tray-004 |
| CLI install (official install-cli.sh + PATH detect + install errors in UI) | CLIInstaller.swift | partial | linux-cli-001, linux-cli-002, linux-cli-003, linux-cli-004 |
| Automation bridge (status stub + socket path; Debug inline JSON) | PeekabooBridgeHostCoordinator.swift | partial | linux-bridge-001, linux-bridge-002 |
| MLX local TTS | TalkMLXSpeechSynthesizer.swift | n/a (Piper/gateway) | — |
