use openclaw_capture as capture;
use openclaw_kit::device_identity::load_or_create_device_identity;
use openclaw_kit::gateway_config::GatewayConnectionSettings;
use openclaw_kit::linux_node_advertisement;
use openclaw_protocol::device_auth::DeviceSigningMaterial;
use openclaw_node_host::{run_command, which, ExecSocketConfig, ExecSocketServer};
use openclaw_protocol::client::{ConnectParams, GatewayClient, GatewayClientConfig, GatewayRole};
use openclaw_protocol::frames::EventFrame;
use serde_json::{json, Value};
use std::sync::Arc;
use crate::canvas;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct NodeRuntimeHandle {
    shutdown: mpsc::Sender<()>,
}

impl NodeRuntimeHandle {
    pub fn stop(&self) {
        let _ = self.shutdown.try_send(());
    }
}

impl NodeRuntimeHandle {
    pub async fn start(
        app: AppHandle,
        settings: GatewayConnectionSettings,
        exec_broker: Arc<crate::exec_broker::ExecApprovalBroker>,
    ) -> Result<Arc<Self>, String> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let url = settings.gateway_ws_url();
        let (camera, location, screen, talk) = settings.node_advertisement_flags();
        let (caps, commands, permissions) =
            linux_node_advertisement(camera, location, screen, talk);
        let identity = load_or_create_device_identity().map_err(|e| e.to_string())?;
        let material = DeviceSigningMaterial {
            device_id: identity.device_id,
            public_key_pem: identity.public_key_pem,
            private_key_pem: identity.private_key_pem,
        };
        let connect = ConnectParams {
            role: GatewayRole::Node,
            client_version: env!("CARGO_PKG_VERSION").into(),
            platform: "linux".into(),
            mode: "node",
            caps,
            commands,
            permissions,
            scopes: vec![],
            auth_token: settings.token.clone(),
            auth_password: settings.password.clone(),
            device_signing: Some(material),
        };
        let (client, mut events) = GatewayClient::connect(GatewayClientConfig { url, connect })
            .await
            .map_err(|e| e.to_string())?;

        let socket_config = ExecSocketConfig::default_paths();
        let broker = exec_broker.clone();
        let socket_server = ExecSocketServer::new(socket_config, move |id, req| {
            let broker = broker.clone();
            Box::pin(async move { broker.prompt(id, req).await })
        });
        tauri::async_runtime::spawn(async move {
            let _ = socket_server.run().await;
        });

        let app_events = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    ev = events.recv() => {
                        if let Some(frame) = ev {
                            handle_node_event(&app_events, &client, frame).await;
                        } else {
                            break;
                        }
                    }
                }
            }
            if let Some(state) = app_events.try_state::<crate::state::AppState>() {
                state.clear_node_runtime_slot().await;
            }
            crate::gateway_runtime::publish_connection_status(
                &app_events,
                "node disconnected; reconnecting…",
            );
        });

        Ok(Arc::new(Self {
            shutdown: shutdown_tx,
        }))
    }
}

async fn handle_node_event(app: &AppHandle, client: &GatewayClient, frame: EventFrame) {
    if frame.event != "node.invoke.request" {
        if frame.event.contains("pair") {
            if let Some(payload) = &frame.payload {
                crate::pairing::emit_pairing_request(app, payload);
            }
        }
        return;
    }
    let Some(payload) = frame.payload else {
        return;
    };
    let command = payload.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let params = payload.get("params").cloned().unwrap_or(Value::Null);
    let result = dispatch_node_command(app, command, &params).await;
    let _ = client
        .request(
            "node.invoke.result",
            Some(json!({
                "id": id,
                "ok": result.is_ok(),
                "payload": result.unwrap_or_else(|e| json!({ "error": e })),
            })),
        )
        .await;
}

async fn dispatch_node_command(
    app: &AppHandle,
    command: &str,
    params: &Value,
) -> Result<Value, String> {
    match command {
        "system.which" => {
            let bin = params.get("binary").and_then(|v| v.as_str()).unwrap_or("");
            let res = which(bin).await;
            Ok(json!({ "path": res.path }))
        }
        "system.run" => {
            let cmd = params.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let cwd = params.get("cwd").and_then(|v| v.as_str());
            let (code, stdout, stderr) = run_command(cmd, cwd)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "code": code, "stdout": stdout, "stderr": stderr }))
        }
        "system.notify" => {
            let title = params
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("OpenClaw");
            let body = params.get("body").and_then(|v| v.as_str()).unwrap_or("");
            notify_rust::Notification::new()
                .summary(title)
                .body(body)
                .show()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "screen.snapshot" => match capture::screen_snapshot().await {
            Ok(bytes) => Ok(json!({
                "format": "png",
                "base64": capture::bytes_to_base64(&bytes),
            })),
            Err(e) => Err(e.to_string()),
        },
        "camera.snap" => match capture::camera_snap().await {
            Ok(bytes) => Ok(json!({
                "format": "jpeg",
                "base64": capture::bytes_to_base64(&bytes),
            })),
            Err(e) => Err(e.to_string()),
        },
        "camera.clip" => match capture::camera_clip().await {
            Ok(bytes) => Ok(json!({
                "format": "jpeg",
                "base64": capture::bytes_to_base64(&bytes),
                "durationMs": 1500,
                "hasAudio": false,
            })),
            Err(e) => Err(e.to_string()),
        },
        "screen.record" => match capture::screen_record().await {
            Ok(rec) => Ok(json!({
                "format": rec.format,
                "base64": capture::bytes_to_base64(&rec.bytes),
                "durationMs": rec.duration_ms,
                "hasAudio": rec.has_audio,
            })),
            Err(e) => Err(e.to_string()),
        },
        "camera.list" => match capture::list_cameras().await {
            Ok(list) => Ok(json!({ "devices": list })),
            Err(e) => Err(e.to_string()),
        },
        "location.get" => match capture::location_get().await {
            Ok(payload) => Ok(payload),
            Err(e) => Err(e.to_string()),
        },
        other if other.starts_with("canvas.")
            || other.starts_with("talk.ptt.")
            || other == "system.execApprovals.get"
            || other == "system.execApprovals.set"
            || other == "system.run.prepare" =>
        {
            canvas::handle_node_command(app, other, params).await
        }
        other => Err(format!("unsupported command: {other}")),
    }
}
