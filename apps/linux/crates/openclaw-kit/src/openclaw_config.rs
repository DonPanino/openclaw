use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct OpenClawGatewayConfig {
    pub port: Option<u16>,
    pub bind: Option<String>,
    pub control_ui_base_path: Option<String>,
    pub auth_token: Option<String>,
    pub auth_password: Option<String>,
    pub tls_enabled: Option<bool>,
    pub remote_url: Option<String>,
    pub remote_ssh_target: Option<String>,
    pub remote_ssh_identity: Option<String>,
    pub remote_port: Option<u16>,
    pub remote_token: Option<String>,
    pub remote_password: Option<String>,
    pub remote_transport: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenClawConfigFile {
    #[serde(default)]
    gateway: GatewaySection,
}

#[derive(Debug, Deserialize, Default)]
struct GatewaySection {
    port: Option<u16>,
    bind: Option<String>,
    #[serde(default)]
    auth: GatewayAuth,
    #[serde(default)]
    control_ui: ControlUiSection,
    tls: Option<TlsSection>,
    remote: Option<RemoteSection>,
}

#[derive(Debug, Deserialize, Default)]
struct RemoteSection {
    url: Option<String>,
    #[serde(rename = "sshTarget")]
    ssh_target: Option<String>,
    #[serde(rename = "sshIdentity")]
    ssh_identity: Option<String>,
    #[serde(rename = "remotePort")]
    remote_port: Option<u16>,
    token: Option<String>,
    password: Option<String>,
    transport: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ControlUiSection {
    #[serde(rename = "basePath")]
    base_path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TlsSection {
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct GatewayAuth {
    token: Option<String>,
    password: Option<String>,
}

pub fn openclaw_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("openclaw.json")
}

pub fn load_gateway_config() -> OpenClawGatewayConfig {
    let path = openclaw_config_path();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return OpenClawGatewayConfig::default();
    };
    let parsed: OpenClawConfigFile = serde_json::from_str(&raw).unwrap_or_default();
    let remote = parsed.gateway.remote.unwrap_or_default();
    OpenClawGatewayConfig {
        port: parsed.gateway.port,
        bind: parsed.gateway.bind,
        control_ui_base_path: parsed.gateway.control_ui.base_path,
        auth_token: parsed.gateway.auth.token,
        auth_password: parsed.gateway.auth.password,
        tls_enabled: parsed.gateway.tls.and_then(|t| t.enabled),
        remote_url: remote.url,
        remote_ssh_target: remote.ssh_target,
        remote_ssh_identity: remote.ssh_identity,
        remote_port: remote.remote_port,
        remote_token: remote.token,
        remote_password: remote.password,
        remote_transport: remote.transport,
    }
}

pub fn load_gateway_auth() -> (Option<String>, Option<String>) {
    let cfg = load_gateway_config();
    (cfg.auth_token, cfg.auth_password)
}
