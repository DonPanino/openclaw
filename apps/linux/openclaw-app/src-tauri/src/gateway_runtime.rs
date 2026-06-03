//! Background operator/node connections with retry and health probes.

use crate::exec_broker::ExecApprovalBroker;
use crate::node_runtime::NodeRuntimeHandle;
use crate::AppState;
use openclaw_kit::gateway_config::{ConnectionMode, GatewayConnectionSettings};
use openclaw_kit::format_tray_tooltip_health;
use openclaw_kit::OperatorGateway;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

const HEALTH_PROBE_SECS: u64 = 30;
const TUNNEL_WATCH_SECS: u64 = 15;
const MAX_BACKOFF_SECS: u64 = 30;

pub async fn refresh_tray_tooltip(app: &AppHandle) {
    let tooltip = if let Some(state) = app.try_state::<AppState>() {
        let operator = state.operator().await.is_some();
        let node = state.has_node_runtime().await;
        let tunnel = state.has_remote_tunnel().await;
        let message = state.connection_status().await;
        format_tray_tooltip_health(operator, node, tunnel, &message)
    } else {
        format_tray_tooltip_health(false, false, false, "")
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

pub fn schedule_tray_tooltip_refresh(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        refresh_tray_tooltip(&app).await;
    });
}

pub fn publish_connection_status(app: &AppHandle, message: impl Into<String>) {
    let msg = message.into();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(state) = app.try_state::<AppState>() {
            state.set_connection_status(msg.clone()).await;
        }
        refresh_tray_tooltip(&app).await;
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

pub fn spawn_remote_tunnel_watchdog(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut backoff = Duration::from_secs(2);
        loop {
            sleep(Duration::from_secs(TUNNEL_WATCH_SECS)).await;
            let Some(state) = app.try_state::<AppState>() else {
                break;
            };
            let settings = state.settings().await;
            if settings.mode != ConnectionMode::Remote || !settings.uses_ssh_tunnel() {
                backoff = Duration::from_secs(2);
                continue;
            }
            if !state.has_remote_tunnel().await {
                continue;
            }
            if state.remote_tunnel_is_alive().await {
                backoff = Duration::from_secs(2);
                continue;
            }
            tracing::warn!("SSH tunnel process exited; restarting");
            publish_connection_status(&app, "SSH tunnel disconnected; reconnecting…");
            state.stop_remote_tunnel().await;
            match crate::ensure_remote_tunnel(&app, &settings).await {
                Ok(()) => {
                    publish_connection_status(
                        &app,
                        format!("SSH tunnel active on port {}", settings.port),
                    );
                    backoff = Duration::from_secs(2);
                }
                Err(err) => {
                    tracing::warn!("SSH tunnel restart: {err}");
                    publish_connection_status(&app, format!("SSH tunnel restart failed: {err}"));
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
    spawn_remote_tunnel_watchdog(app.clone());
    spawn_operator_loop(app.clone(), settings.clone());
    spawn_node_loop(app, settings, exec_broker);
}
