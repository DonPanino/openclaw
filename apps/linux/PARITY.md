# macOS → Linux parity matrix

Status: `planned` | `partial` | `done`. Test ID links to `apps/linux/tests/` or gateway integration tests.

| Feature | macOS reference | Linux status | Test |
|---------|-----------------|--------------|------|
| Tray + status | MenuBar.swift | partial | linux-tray-001 |
| Local gateway attach | GatewayProcessManager.swift | done | linux-gateway-001 |
| Remote gateway + tunnel | ConnectionModeCoordinator.swift | partial | linux-remote-001 |
| SSH tunnel (`ssh -N -L`) | RemotePortTunnel.swift | partial | linux-remote-002 |
| Remote reconnect on save/tunnel | ConnectionModeCoordinator.swift | partial | linux-remote-003 |
| Control UI / Dashboard | DashboardManager.swift | done | linux-dashboard-001 |
| Dashboard auth (HTTP + native inject) | DashboardWindowController.swift | done | linux-dashboard-002 |
| WebChat | WebChatSwiftUI.swift | partial | linux-webchat-001 |
| WebChat (Control UI `/chat`) | WebChatSwiftUI.swift | partial | linux-webchat-002 |
| Device identity + WS auth | OpenClawKit device | done | linux-node-001 |
| Operator gateway RPC | ChannelsStore.swift | partial | linux-settings-004 |
| Operator/node reconnect + health probe | ConnectionModeCoordinator | partial | linux-gateway-002 |
| Pairing approvals | DevicePairingApprovalPrompter.swift | partial | linux-pair-001 |
| Settings: General | GeneralSettings.swift | partial | linux-settings-001 |
| Settings: Connection | ConnectionModeCoordinator | partial | linux-settings-002 |
| Connection errors in UI | ConnectionModeCoordinator | partial | linux-settings-011 |
| Settings: Permissions (cap toggles + probe) | PermissionsSettings.swift | partial | linux-settings-003 |
| Permissions capability probe | PermissionsSettings.swift | partial | linux-settings-012 |
| Settings: Voice (wake/talk flags in linux-app-settings) | VoiceWakeSettings.swift | partial | linux-voice-001 |
| Node caps from linux-app-settings | MacNodeModeCoordinator | partial | linux-node-001 |
| Settings: Channels | ChannelsSettings.swift | partial | linux-settings-004 |
| Settings: Skills | SkillsSettings.swift | partial | linux-settings-005 |
| Settings: Cron | CronSettings.swift | partial | linux-settings-006 |
| Settings: Exec | SystemRunSettingsView.swift | partial | linux-exec-001 |
| Exec approval prompts (socket → UI) | ExecApprovalPrompter | partial | linux-exec-002 |
| Settings: Sessions | SessionsSettings.swift | partial | linux-settings-007 |
| Settings: Instances | InstancesSettings.swift | partial | linux-settings-008 |
| Settings: Config | ConfigSettings.swift | partial | linux-settings-009 |
| Settings: Debug | DebugSettings.swift | partial | linux-settings-010 |
| linux:dev prep script | dev-setup.md | done | — |
| Debug: stop gateway | DebugSettings.swift | partial | linux-settings-010 |
| Node: system.run | MacNodeRuntime | partial | linux-node-001 |
| Node: system.notify | NotificationManager.swift | partial | linux-node-002 |
| Node: canvas.* | CanvasManager.swift | partial | linux-canvas-001 |
| Canvas scaffold (shared kit + dist/canvas-host/a2ui) | CanvasManager.swift | partial | linux-canvas-002 |
| Canvas A2UI push/reset/eval | CanvasManager.swift | partial | linux-canvas-003 |
| Canvas snapshot (screen fallback) | CanvasManager.swift | partial | linux-canvas-004 |
| talk.ptt.* (stub) | VoicePushToTalk.swift | partial | linux-voice-003 |
| system.execApprovals.get/set | MacNodeRuntime | partial | linux-exec-003 |
| Node: screen.* (portal + grim) | ScreenSnapshotService.swift | partial | linux-screen-001 |
| Node: camera.* | CameraCaptureService.swift | partial | linux-camera-001 |
| Node: camera.clip (ffmpeg/fswebcam burst) | CameraCaptureService.swift | partial | linux-camera-001 |
| Node: screen.record | ScreenCaptureKit.swift | partial | linux-screen-001 |
| Launch: hide settings after dashboard | DashboardManager.swift | partial | linux-dashboard-001 |
| Node: location.get | MacNodeLocationService.swift | partial | linux-location-001 |
| Node: location.get (gpspipe/GeoClue) | MacNodeLocationService.swift | partial | linux-location-002 |
| Voice wake | VoiceWakeRuntime.swift | partial | linux-voice-002 |
| Voice settings persistence | VoiceWakeSettings.swift | partial | linux-voice-005 |
| PTT | VoicePushToTalk.swift | partial | linux-voice-003 |
| Talk mode | TalkModeController.swift | partial | linux-voice-004 |
| Pairing prompts | NodePairingApprovalPrompter.swift | partial | linux-pair-001 |
| mDNS discovery | GatewayDiscoveryModel.swift | partial | linux-discover-001 |
| Deep links | DeepLinks.swift | partial | linux-deeplink-001 |
| Tray status tooltip (operator/node WS status) | MenuBar.swift | partial | linux-tray-002 |
| CLI install | CLIInstaller.swift | partial | linux-cli-001 |
| Automation bridge | PeekabooBridgeHostCoordinator.swift | planned | linux-bridge-001 |
| MLX local TTS | TalkMLXSpeechSynthesizer.swift | n/a (Piper/gateway) | — |
