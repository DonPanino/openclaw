//! Background operator/node connections with retry and health probes.

use crate::exec_broker::ExecApprovalBroker;
use crate::node_runtime::NodeRuntimeHandle;
use crate::AppState;
use openclaw_kit::gateway_config::GatewayConnectionSettings;
use openclaw_kit::format_tray_tooltip;
use openclaw_kit::OperatorGateway;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

const HEALTH_PROBE_SECS: u64 = 30;
const MAX_BACKOFF_SECS: u64 = 30;

pub fn apply_tray_tooltip(app: &AppHandle, status: &str) {
    let tooltip = format_tray_tooltip(status);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

pub fn publish_connection_status(app: &AppHandle, message: impl Into<String>) {
    let msg = message.into();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(state) = app.try_state::<AppState>() {
            state.set_connection_status(msg.clone()).await;
        }
        apply_tray_tooltip(&app, &msg);
        let _ = app.emit("gateway-connection-status", msg);
    });
}

pub fn spawn_operator_loop(app: AppHandle, settings: GatewayConnectionSettings) {
    tauri::async_runtime::spawn(async move {
        let mut backoff = Duration::from_secs(2);
        loop {
            if app.try_state::<AppState>().is_none() {
                break;
            }
            if let Some(state) = app.try_state::<AppState>() {
                if let Some(op) = state.operator().await {
                    sleep(Duration::from_secs(HEALTH_PROBE_SECS)).await;
                    if op.health().await.is_ok() {
                        continue;
                    }
                    tracing::warn!("operator health probe failed; reconnecting");
                    state.clear_operator().await;
                }
            }
            match OperatorGateway::connect(&settings).await {
                Ok(op) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.set_operator(Arc::new(op)).await;
                    }
                    publish_connection_status(&app, "operator connected");
                    backoff = Duration::from_secs(2);
                }
                Err(err) => {
                    tracing::warn!("operator connect: {err}");
                    publish_connection_status(&app, format!("operator: {err}"));
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(MAX_BACKOFF_SECS));
                }
            }
        }
    });
}

pub fn spawn_node_loop(
    app: AppHandle,
    settings: GatewayConnectionSettings,
    exec_broker: Arc<ExecApprovalBroker>,
) {
    tauri::async_runtime::spawn(async move {
        let mut backoff = Duration::from_secs(2);
        loop {
            if app.try_state::<AppState>().is_none() {
                break;
            }
            if let Some(state) = app.try_state::<AppState>() {
                if state.has_node_runtime().await {
                    sleep(Duration::from_secs(HEALTH_PROBE_SECS)).await;
                    continue;
                }
            }
            match NodeRuntimeHandle::start(app.clone(), settings.clone(), exec_broker.clone()).await
            {
                Ok(handle) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.set_node_runtime(handle).await;
                        let base = state.connection_status().await;
                        let status = if base.is_empty() {
                            "node connected".into()
                        } else {
                            format!("{base}; node connected")
                        };
                        publish_connection_status(&app, status);
                    }
                    backoff = Duration::from_secs(2);
                }
                Err(err) => {
                    tracing::warn!("node connect: {err}");
                    publish_connection_status(&app, format!("node: {err}"));
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(MAX_BACKOFF_SECS));
                }
            }
        }
    });
}

pub fn spawn_background_services(
    app: AppHandle,
    settings: GatewayConnectionSettings,
    exec_broker: Arc<ExecApprovalBroker>,
) {
    spawn_operator_loop(app.clone(), settings.clone());
    spawn_node_loop(app, settings, exec_broker);
}
