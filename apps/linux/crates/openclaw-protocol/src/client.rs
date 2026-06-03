use crate::device_auth::{build_connect_device, DeviceSigningMaterial};
use crate::frames::{EventFrame, GatewayFrame, ResponseFrame};
use crate::version::{MIN_CLIENT_PROTOCOL_VERSION, PROTOCOL_VERSION};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use uuid::Uuid;

pub const CLIENT_ID_LINUX: &str = "openclaw-linux";
pub const CLIENT_ID_LINUX_NODE: &str = "openclaw-linux";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayRole {
    Operator,
    Node,
}

#[derive(Debug, Clone)]
pub struct ConnectParams {
    pub role: GatewayRole,
    pub client_version: String,
    pub platform: String,
    pub mode: &'static str,
    pub caps: Vec<String>,
    pub commands: Vec<String>,
    pub permissions: HashMap<String, bool>,
    pub scopes: Vec<String>,
    pub auth_token: Option<String>,
    pub auth_password: Option<String>,
    pub device_signing: Option<DeviceSigningMaterial>,
}

#[derive(Debug, Clone)]
pub struct GatewayClientConfig {
    pub url: Url,
    pub connect: ConnectParams,
}

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("connect failed: {0}")]
    ConnectFailed(String),
    #[error("request {method} failed: {message}")]
    RpcFailed { method: String, message: String },
    #[error("channel closed")]
    ChannelClosed,
}

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value, GatewayError>>>>>;

pub struct GatewayClient {
    outbound: mpsc::Sender<GatewayFrame>,
    pending: PendingMap,
    event_tx: mpsc::Sender<EventFrame>,
    server_version: Arc<Mutex<Option<String>>>,
}

impl GatewayClient {
    pub async fn connect(config: GatewayClientConfig) -> Result<(Self, mpsc::Receiver<EventFrame>), GatewayError> {
        let (ws_stream, _) = connect_async(config.url.as_str()).await?;
        let (mut write, mut read) = ws_stream.split();
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<GatewayFrame>(64);
        let (event_tx, event_rx) = mpsc::channel::<EventFrame>(128);

        let (challenge_tx, challenge_rx) = oneshot::channel::<Option<String>>();
        let challenge_once = Arc::new(Mutex::new(Some(challenge_tx)));
        let pending_reader = pending.clone();
        let event_tx_reader = event_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                let Ok(msg) = msg else { break };
                if let Message::Text(text) = msg {
                    if let Ok(frame) = serde_json::from_str::<GatewayFrame>(&text) {
                        match frame {
                            GatewayFrame::Response(ResponseFrame { id, ok, payload, error }) => {
                                if let Some(tx) = pending_reader.lock().await.remove(&id) {
                                    let result = if ok {
                                        Ok(payload.unwrap_or(Value::Null))
                                    } else {
                                        Err(GatewayError::RpcFailed {
                                            method: id.clone(),
                                            message: error
                                                .map(|e| e.message)
                                                .unwrap_or_else(|| "unknown error".into()),
                                        })
                                    };
                                    let _ = tx.send(result);
                                }
                            }
                            GatewayFrame::Event(ev) => {
                                if ev.event == "connect.challenge" {
                                    if let Some(payload) = &ev.payload {
                                        if let Some(nonce) =
                                            payload.get("nonce").and_then(|v| v.as_str())
                                        {
                                            if let Some(tx) =
                                                challenge_once.lock().await.take()
                                            {
                                                let _ = tx.send(Some(nonce.to_string()));
                                            }
                                        }
                                    }
                                }
                                let _ = event_tx_reader.send(ev).await;
                            }
                            GatewayFrame::Request(_) => {}
                        }
                    }
                }
            }
        });

        let pending_writer = pending.clone();
        tokio::spawn(async move {
            while let Some(frame) = outbound_rx.recv().await {
                if let Ok(text) = serde_json::to_string(&frame) {
                    if write.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
            }
            let mut guard = pending_writer.lock().await;
            for (_, tx) in guard.drain() {
                let _ = tx.send(Err(GatewayError::ChannelClosed));
            }
        });

        let client = Self {
            outbound: outbound_tx,
            pending,
            event_tx,
            server_version: Arc::new(Mutex::new(None)),
        };

        let challenge_nonce = tokio::time::timeout(std::time::Duration::from_secs(10), challenge_rx)
            .await
            .ok()
            .and_then(|r| r.ok())
            .flatten();
        client.handshake(config.connect, challenge_nonce).await?;
        Ok((client, event_rx))
    }

    async fn handshake(&self, params: ConnectParams, challenge_nonce: Option<String>) -> Result<(), GatewayError> {
        let role = match params.role {
            GatewayRole::Operator => "operator",
            GatewayRole::Node => "node",
        };
        let auth_token = params.auth_token.as_deref();
        let mut auth = serde_json::Map::new();
        if let Some(token) = auth_token {
            auth.insert("token".into(), json!(token));
        }
        if let Some(password) = params.auth_password.as_deref() {
            auth.insert("password".into(), json!(password));
        }
        let mut connect_params = json!({
            "minProtocol": MIN_CLIENT_PROTOCOL_VERSION,
            "maxProtocol": PROTOCOL_VERSION,
            "client": {
                "id": CLIENT_ID_LINUX,
                "version": params.client_version,
                "platform": params.platform,
                "mode": params.mode,
            },
            "role": role,
            "scopes": params.scopes,
            "caps": params.caps,
            "commands": params.commands,
            "permissions": params.permissions,
            "auth": auth,
            "locale": "en-US",
            "userAgent": format!("openclaw-linux/{}", params.client_version),
        });
        if let (Some(nonce), Some(material)) = (challenge_nonce.as_ref(), params.device_signing.as_ref()) {
            if let Some(device) = build_connect_device(
                material,
                CLIENT_ID_LINUX,
                params.mode,
                role,
                &params.scopes,
                auth_token,
                nonce,
                Some(params.platform.as_str()),
            ) {
                connect_params["device"] = device;
            }
        }
        let hello = self.request("connect", Some(connect_params)).await?;
        if hello.get("type").and_then(|v| v.as_str()) != Some("hello-ok") {
            return Err(GatewayError::ConnectFailed(format!("unexpected hello: {hello}")));
        }
        let version = hello
            .get("server")
            .and_then(|s| s.get("version"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        *self.server_version.lock().await = version;
        Ok(())
    }

    pub async fn server_version(&self) -> Option<String> {
        self.server_version.lock().await.clone()
    }

    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value, GatewayError> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        self.outbound
            .send(GatewayFrame::request(id.clone(), method, params))
            .await
            .map_err(|_| GatewayError::ChannelClosed)?;
        rx.await.map_err(|_| GatewayError::ChannelClosed)?
    }

    pub fn event_sender(&self) -> mpsc::Sender<EventFrame> {
        self.event_tx.clone()
    }
}
