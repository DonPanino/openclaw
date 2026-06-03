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
    /// Last WebChat session key (Control UI `/chat?session=`); restored when opening WebChat without an explicit session.
    #[serde(default)]
    pub last_webchat_session: Option<String>,
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
            last_webchat_session: None,
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

    /// Explicit session wins; otherwise returns persisted `last_webchat_session`.
    pub fn resolved_webchat_session(explicit: Option<&str>) -> Option<String> {
        explicit
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| {
                Self::load()
                    .last_webchat_session
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    }

    /// Persists a non-empty session key; does not clear when `session` is absent.
    pub fn remember_webchat_session(session: Option<&str>) -> std::io::Result<()> {
        let Some(trimmed) = session.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(());
        };
        let mut settings = Self::load();
        let next = trimmed.to_string();
        if settings.last_webchat_session.as_deref() == Some(trimmed) {
            return Ok(());
        }
        settings.last_webchat_session = Some(next);
        settings.save()
    }

    pub fn webchat_window_title(session: Option<&str>) -> String {
        let Some(key) = session.map(str::trim).filter(|s| !s.is_empty()) else {
            return "OpenClaw WebChat".into();
        };
        let short = if key.len() > 48 {
            format!("{}…", &key[..45])
        } else {
            key.to_string()
        };
        format!("OpenClaw WebChat — {short}")
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

    #[test]
    fn resolved_webchat_session_prefers_explicit() {
        assert_eq!(
            GatewayConnectionSettings::resolved_webchat_session(Some("agent:other:main")),
            Some("agent:other:main".into())
        );
        assert_eq!(
            GatewayConnectionSettings::resolved_webchat_session(Some("  ")),
            None
        );
    }

    #[test]
    fn remember_webchat_session_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());
        let result = std::panic::catch_unwind(|| {
            GatewayConnectionSettings::default()
                .save()
                .expect("seed settings");
            GatewayConnectionSettings::remember_webchat_session(Some("agent:main:main"))
                .expect("remember");
            let loaded = GatewayConnectionSettings::load();
            assert_eq!(
                loaded.last_webchat_session.as_deref(),
                Some("agent:main:main")
            );
            assert_eq!(
                GatewayConnectionSettings::resolved_webchat_session(None),
                Some("agent:main:main".into())
            );
            GatewayConnectionSettings::remember_webchat_session(Some("agent:main:main"))
                .expect("remember noop");
            assert_eq!(
                GatewayConnectionSettings::resolved_webchat_session(Some("agent:other:main")),
                Some("agent:other:main".into())
            );
            GatewayConnectionSettings::remember_webchat_session(None).expect("absent noop");
            assert_eq!(
                GatewayConnectionSettings::load()
                    .last_webchat_session
                    .as_deref(),
                Some("agent:main:main")
            );
        });
        match prev_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        result.expect("remember_webchat_session_round_trip");
    }

    #[test]
    fn webchat_window_title_includes_session() {
        assert_eq!(
            GatewayConnectionSettings::webchat_window_title(Some("agent:main:main")),
            "OpenClaw WebChat — agent:main:main"
        );
        assert_eq!(
            GatewayConnectionSettings::webchat_window_title(None),
            "OpenClaw WebChat"
        );
    }

    #[test]
    fn gateway_ws_url_uses_wss_when_tls_enabled() {
        let settings = GatewayConnectionSettings {
            host: Some("example.com".into()),
            port: 443,
            use_tls: true,
            ..GatewayConnectionSettings::default()
        };
        let url = settings.gateway_ws_url();
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.host_str(), Some("example.com"));
        assert!(url.port().unwrap_or(443) == 443);
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
