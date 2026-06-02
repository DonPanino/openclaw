use openclaw_kit::gateway_config::GatewayConnectionSettings;
use openclaw_kit::remote_tunnel::RemoteTunnel;
use openclaw_kit::OperatorGateway;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    settings: Mutex<GatewayConnectionSettings>,
    node: Mutex<Option<Arc<crate::node_runtime::NodeRuntimeHandle>>>,
    operator: Mutex<Option<Arc<OperatorGateway>>>,
    remote_tunnel: Mutex<Option<RemoteTunnel>>,
    connection_status: Mutex<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(GatewayConnectionSettings::load()),
            node: Mutex::new(None),
            operator: Mutex::new(None),
            remote_tunnel: Mutex::new(None),
            connection_status: Mutex::new(String::new()),
        }
    }
}

impl AppState {
    pub async fn set_connection_status(&self, message: impl Into<String>) {
        *self.connection_status.lock().await = message.into();
    }

    pub async fn connection_status(&self) -> String {
        self.connection_status.lock().await.clone()
    }

    pub async fn stop_node_runtime(&self) {
        let mut guard = self.node.lock().await;
        if let Some(handle) = guard.take() {
            handle.stop();
        }
    }

    pub async fn has_node_runtime(&self) -> bool {
        self.node.lock().await.is_some()
    }
}

impl AppState {
    pub async fn settings(&self) -> GatewayConnectionSettings {
        self.settings.lock().await.clone()
    }

    pub async fn set_settings(&self, settings: GatewayConnectionSettings) {
        *self.settings.lock().await = settings;
    }

    pub async fn set_node_runtime(&self, handle: Arc<crate::node_runtime::NodeRuntimeHandle>) {
        *self.node.lock().await = Some(handle);
    }

    pub async fn set_operator(&self, gateway: Arc<OperatorGateway>) {
        *self.operator.lock().await = Some(gateway);
    }

    pub async fn operator(&self) -> Option<Arc<OperatorGateway>> {
        self.operator.lock().await.clone()
    }

    pub async fn set_remote_tunnel(&self, tunnel: Option<RemoteTunnel>) {
        let mut guard = self.remote_tunnel.lock().await;
        if let Some(existing) = guard.take() {
            existing.stop().await;
        }
        *guard = tunnel;
    }

    pub async fn stop_remote_tunnel(&self) {
        let mut guard = self.remote_tunnel.lock().await;
        if let Some(tunnel) = guard.take() {
            tunnel.stop().await;
        }
    }

    pub async fn clear_operator(&self) {
        *self.operator.lock().await = None;
    }
}
