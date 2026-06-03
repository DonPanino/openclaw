use crate::gateway_config::GatewayConnectionSettings;
use crate::device_identity::load_or_create_device_identity;
use openclaw_protocol::client::{
    ConnectParams, GatewayClient, GatewayClientConfig, GatewayRole,
};
use openclaw_protocol::device_auth::DeviceSigningMaterial;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct OperatorGateway {
    client: Arc<Mutex<GatewayClient>>,
}

impl OperatorGateway {
    pub async fn connect(settings: &GatewayConnectionSettings) -> Result<Self, String> {
        let identity = load_or_create_device_identity().map_err(|e| e.to_string())?;
        let material = DeviceSigningMaterial {
            device_id: identity.device_id,
            public_key_pem: identity.public_key_pem,
            private_key_pem: identity.private_key_pem,
        };
        let connect = ConnectParams {
            role: GatewayRole::Operator,
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: "linux".to_string(),
            mode: "ui",
            caps: vec![],
            commands: vec![],
            permissions: Default::default(),
            scopes: vec![
                "operator.read".into(),
                "operator.write".into(),
                "operator.admin".into(),
                "operator.pairing".into(),
            ],
            auth_token: settings.token.clone(),
            auth_password: settings.password.clone(),
            device_signing: Some(material),
        };
        let (client, _events) = GatewayClient::connect(GatewayClientConfig {
            url: settings.gateway_ws_url(),
            connect,
        })
        .await
        .map_err(|e| e.to_string())?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    pub async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, String> {
        self.client
            .lock()
            .await
            .request(method, params)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn health(&self) -> Result<Value, String> {
        self.call("health", None).await
    }

    pub async fn server_version(&self) -> Option<String> {
        self.client.lock().await.server_version().await
    }

    pub async fn config_get(&self) -> Result<Value, String> {
        self.call("config.get", None).await
    }

    pub async fn channels_status(&self) -> Result<Value, String> {
        self.call("channels.status", None).await
    }

    pub async fn channel_start(&self, channel: &str) -> Result<Value, String> {
        self.call(
            "channels.start",
            Some(json!({ "channel": channel })),
        )
        .await
    }

    pub async fn channel_stop(&self, channel: &str) -> Result<Value, String> {
        self.call(
            "channels.stop",
            Some(json!({ "channel": channel })),
        )
        .await
    }

    pub async fn skills_status(&self) -> Result<Value, String> {
        self.call("skills.status", None).await
    }

    pub async fn cron_list(&self) -> Result<Value, String> {
        self.call("cron.list", None).await
    }

    pub async fn cron_status(&self) -> Result<Value, String> {
        self.call("cron.status", None).await
    }

    pub async fn cron_run(&self, job_id: &str) -> Result<Value, String> {
        self.call("cron.run", Some(json!({ "id": job_id }))).await
    }

    pub async fn sessions_list(&self) -> Result<Value, String> {
        self.call("sessions.list", None).await
    }

    pub async fn sessions_preview(
        &self,
        keys: &[String],
        limit: Option<u32>,
    ) -> Result<Value, String> {
        let mut params = json!({ "keys": keys });
        if let Some(limit) = limit {
            params["limit"] = json!(limit);
        }
        self.call("sessions.preview", Some(params)).await
    }

    pub async fn sessions_describe(
        &self,
        key: &str,
        include_derived_titles: bool,
        include_last_message: bool,
    ) -> Result<Value, String> {
        let key = key.trim();
        if key.is_empty() {
            return Err("session key required".into());
        }
        let mut params = json!({ "key": key });
        if include_derived_titles {
            params["includeDerivedTitles"] = json!(true);
        }
        if include_last_message {
            params["includeLastMessage"] = json!(true);
        }
        self.call("sessions.describe", Some(params)).await
    }

    pub async fn node_list(&self) -> Result<Value, String> {
        self.call("node.list", None).await
    }

    pub async fn system_presence(&self) -> Result<Value, String> {
        self.call("system-presence", None).await
    }

    pub async fn voicewake_get(&self) -> Result<Value, String> {
        self.call("voicewake.get", None).await
    }

    pub async fn voicewake_set(&self, triggers: &[String]) -> Result<Value, String> {
        self.call(
            "voicewake.set",
            Some(json!({ "triggers": triggers })),
        )
        .await
    }

    pub async fn talk_config(&self) -> Result<Value, String> {
        self.call("talk.config", None).await
    }

    pub async fn device_pair_list(&self) -> Result<Value, String> {
        self.call("device.pair.list", None).await
    }

    pub async fn device_pair_approve(&self, request_id: &str) -> Result<Value, String> {
        self.call(
            "device.pair.approve",
            Some(json!({ "requestId": request_id })),
        )
        .await
    }

    pub async fn device_pair_reject(&self, request_id: &str) -> Result<Value, String> {
        self.call(
            "device.pair.reject",
            Some(json!({ "requestId": request_id })),
        )
        .await
    }

    pub async fn node_pair_list(&self) -> Result<Value, String> {
        self.call("node.pair.list", None).await
    }

    pub async fn node_pair_approve(&self, request_id: &str) -> Result<Value, String> {
        self.call(
            "node.pair.approve",
            Some(json!({ "requestId": request_id })),
        )
        .await
    }

    pub async fn node_pair_reject(&self, request_id: &str) -> Result<Value, String> {
        self.call(
            "node.pair.reject",
            Some(json!({ "requestId": request_id })),
        )
        .await
    }
}
