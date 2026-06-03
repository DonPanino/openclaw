//! Gateway device/node pairing approval (operator RPC + desktop notifications).

use serde_json::Value;
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Duration};

static SEEN_PAIRING_IDS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub fn emit_pairing_request(app: &AppHandle, payload: &Value) {
    let _ = app.emit("pairing-request", payload.clone());
}

pub fn start_pairing_poll(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            let Some(state) = app.try_state::<crate::state::AppState>() else {
                continue;
            };
            let Some(op) = state.operator().await else {
                continue;
            };
            poll_pairing_lists(&app, &op).await;
        }
    });
}

async fn poll_pairing_lists(app: &AppHandle, op: &openclaw_kit::OperatorGateway) {
    if let Ok(devices) = op.device_pair_list().await {
        notify_pending(app, "device", &devices);
    }
    if let Ok(nodes) = op.node_pair_list().await {
        notify_pending(app, "node", &nodes);
    }
}

fn notify_pending(app: &AppHandle, kind: &str, list: &Value) {
    let Some(pending) = list.get("pending").and_then(|v| v.as_array()) else {
        return;
    };
    let mut guard = SEEN_PAIRING_IDS.lock().expect("pairing ids lock");
    let seen = guard.get_or_insert_with(HashSet::new);
    for req in pending {
        let request_id = req
            .get("requestId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if request_id.is_empty() || !seen.insert(request_id.to_string()) {
            continue;
        }
        let name = req
            .get("displayName")
            .or_else(|| req.get("deviceId"))
            .or_else(|| req.get("nodeId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let summary = "OpenClaw pairing request";
        let body = format!("{kind} \"{name}\" — open Settings to approve");
        let _ = notify_rust::Notification::new()
            .summary(&summary)
            .body(&body)
            .show();
        let _ = app.emit(
            "pairing-request",
            serde_json::json!({ "kind": kind, "request": req }),
        );
    }
}
