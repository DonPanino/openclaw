use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConnectionSettings {
    #[serde(default)]
    pub mode: ConnectionMode,
    #[serde(default = "default_port")]
    pub port: u16,
    pub host: Option<String>,
    pub token: Option<String>,
    pub password: Option<String>,
    pub use_tls: bool,
    #[serde(default)]
    pub ssh_target: Option<String>,
    #[serde(default)]
    pub ssh_identity: Option<String>,
    /// When true, connect WS to host/port directly (gateway.remote.url) without SSH tunnel.
    #[serde(default)]
    pub remote_direct: bool,
    #[serde(default)]
    pub voice_wake_enabled: bool,
    #[serde(default)]
    pub talk_enabled: bool,
    #[serde(default)]
    pub voice_wake_phrases: Vec<String>,
    #[serde(default = "default_node_capture_enabled")]
    pub camera_enabled: bool,
    #[serde(default = "default_node_capture_enabled")]
    pub screen_enabled: bool,
    #[serde(default = "default_node_capture_enabled")]
    pub location_enabled: bool,
    /// When true (default), local mode starts the systemd gateway user unit on app launch if it is not already running.
    #[serde(default = "default_gateway_autostart")]
    pub gateway_autostart: bool,
}

fn default_gateway_autostart() -> bool {
    true
}

fn default_node_capture_enabled() -> bool {
    true
}

impl Default for GatewayConnectionSettings {
    fn default() -> Self {
        Self {
            mode: ConnectionMode::default(),
            port: default_port(),
            host: None,
            token: None,
            password: None,
            use_tls: false,
            ssh_target: None,
            ssh_identity: None,
            remote_direct: false,
            voice_wake_enabled: false,
            talk_enabled: false,
            voice_wake_phrases: Vec::new(),
            camera_enabled: true,
            screen_enabled: true,
            location_enabled: true,
            gateway_autostart: default_gateway_autostart(),
        }
    }
}

fn default_port() -> u16 {
    18789
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionMode {
    #[default]
    Local,
    Remote,
}

impl GatewayConnectionSettings {
    pub fn settings_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".openclaw")
            .join("linux-app-settings.json")
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        let gateway = crate::openclaw_config::load_gateway_config();
        let mut settings = if !path.exists() {
            Self::default()
        } else {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default()
        };
        if settings.port == default_port() {
            if let Some(port) = gateway.port {
                settings.port = port;
            }
        }
        if settings.ssh_target.is_none() {
            settings.ssh_target = gateway.remote_ssh_target.clone();
        }
        if settings.ssh_identity.is_none() {
            settings.ssh_identity = gateway.remote_ssh_identity.clone();
        }
        if settings.mode == ConnectionMode::Remote {
            if settings.token.is_none() {
                settings.token = gateway.remote_token.clone().or(gateway.auth_token.clone());
            }
            if settings.password.is_none() {
                settings.password = gateway
                    .remote_password
                    .clone()
                    .or(gateway.auth_password.clone());
            }
            apply_remote_url(&gateway, &mut settings);
        } else if settings.token.is_none() && settings.password.is_none() {
            settings.token = gateway.auth_token.clone();
            settings.password = gateway.auth_password.clone();
        }
        settings
    }

    pub fn remote_gateway_port(gateway: &crate::openclaw_config::OpenClawGatewayConfig) -> u16 {
        gateway
            .remote_port
            .or(gateway.port)
            .unwrap_or(default_port())
    }

    pub fn uses_ssh_tunnel(&self) -> bool {
        self.mode == ConnectionMode::Remote
            && !self.remote_direct
            && self
                .ssh_target
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }

    /// `(camera, location, screen, talk)` flags for node capability advertisement.
    pub fn node_advertisement_flags(&self) -> (bool, bool, bool, bool) {
        (
            self.camera_enabled,
            self.location_enabled,
            self.screen_enabled,
            self.talk_enabled,
        )
    }

    pub fn gateway_ws_url(&self) -> url::Url {
        let host = self
            .host
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let scheme = if self.use_tls { "wss" } else { "ws" };
        url::Url::parse(&format!("{scheme}://{host}:{}", self.port)).expect("valid gateway url")
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayConnectionSettings;

    #[test]
    fn default_node_advertisement_flags_enable_capture() {
        let settings = GatewayConnectionSettings::default();
        let (camera, location, screen, talk) = settings.node_advertisement_flags();
        assert!(camera && location && screen);
        assert!(!talk);
    }
}

fn apply_remote_url(
    gateway: &crate::openclaw_config::OpenClawGatewayConfig,
    settings: &mut GatewayConnectionSettings,
) {
    let transport = gateway.remote_transport.as_deref().unwrap_or("ssh");
    let Some(raw) = gateway.remote_url.as_ref().filter(|u| !u.trim().is_empty()) else {
        settings.remote_direct = transport == "direct";
        return;
    };
    let Ok(url) = url::Url::parse(raw.trim()) else {
        return;
    };
    let host = url.host_str().unwrap_or("127.0.0.1");
    let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";
    if transport == "direct" || !loopback {
        settings.host = Some(host.to_string());
        settings.port = url.port().unwrap_or(settings.port);
        settings.use_tls = url.scheme() == "wss";
        settings.remote_direct = true;
        return;
    }
    settings.remote_direct = false;
    settings.host = Some("127.0.0.1".into());
}
