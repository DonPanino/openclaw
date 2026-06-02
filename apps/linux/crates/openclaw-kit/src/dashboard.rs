use crate::gateway_config::{ConnectionMode, GatewayConnectionSettings};
use crate::openclaw_config::OpenClawGatewayConfig;
use url::Url;

#[derive(Debug, Clone)]
pub struct DashboardAuth {
    pub http_url: Url,
    pub ws_url: String,
    pub token: Option<String>,
    pub password: Option<String>,
}

fn normalize_base_path(raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or("/").trim();
    if trimmed.is_empty() {
        return "/".into();
    }
    let with_slash = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    if with_slash.len() > 1 && with_slash.ends_with('/') {
        with_slash.trim_end_matches('/').to_string()
    } else {
        with_slash
    }
}

/// Build the Control UI HTTP URL and companion auth material (matches macOS dashboard flow).
pub fn resolve_dashboard_auth(
    settings: &GatewayConnectionSettings,
    gateway: &OpenClawGatewayConfig,
) -> Result<DashboardAuth, String> {
    let host = settings
        .host
        .clone()
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| default_http_host(settings.mode, gateway.bind.as_deref()));

    let port = if settings.port != 0 {
        settings.port
    } else {
        gateway.port.unwrap_or(18_789)
    };

    let tls = settings.use_tls || gateway.tls_enabled.unwrap_or(false);
    let scheme = if tls { "https" } else { "http" };

    let base_path = if settings.mode == ConnectionMode::Local {
        normalize_base_path(gateway.control_ui_base_path.as_deref())
    } else {
        "/".into()
    };

    let http_url = Url::parse(&format!("{scheme}://{host}:{port}{base_path}"))
        .map_err(|e| format!("invalid dashboard url: {e}"))?;

    let ws_scheme = if tls { "wss" } else { "ws" };
    let ws_url = format!("{ws_scheme}://{host}:{port}{base_path}");

    let token = settings
        .token
        .clone()
        .or_else(|| gateway.auth_token.clone())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    let password = settings
        .password
        .clone()
        .or_else(|| gateway.auth_password.clone())
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    Ok(DashboardAuth {
        http_url,
        ws_url,
        token,
        password,
    })
}

fn default_http_host(mode: ConnectionMode, bind: Option<&str>) -> String {
    if mode == ConnectionMode::Remote {
        return "127.0.0.1".into();
    }
    match bind.unwrap_or("loopback") {
        "lan" | "custom" => "127.0.0.1".into(),
        _ => "127.0.0.1".into(),
    }
}

pub fn dashboard_url_with_token_fragment(mut url: Url, token: Option<&str>) -> Url {
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        let encoded: String = url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
        url.set_fragment(Some(&format!("token={encoded}")));
    }
    url
}

/// Control UI chat tab (`/chat` under the configured base path).
pub fn control_ui_chat_url(auth: &DashboardAuth) -> Url {
    let mut url = auth.http_url.clone();
    let path = url.path().trim_end_matches('/');
    let chat_path = if path.is_empty() || path == "/" {
        "/chat".to_string()
    } else {
        format!("{path}/chat")
    };
    url.set_path(&chat_path);
    dashboard_url_with_token_fragment(url, auth.token.as_deref())
}

pub fn native_control_auth_init_script(auth: &DashboardAuth) -> String {
    let payload = serde_json::json!({
        "gatewayUrl": auth.ws_url,
        "token": auth.token,
        "password": auth.password,
    });
    let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
    format!(
        "(function(){{try{{Object.defineProperty(window,\"__OPENCLAW_NATIVE_CONTROL_AUTH__\",{{value:{json},configurable:true}});}}catch(e){{}}}})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_config::GatewayConnectionSettings;

    #[test]
    fn local_dashboard_uses_control_ui_base_path() {
        let settings = GatewayConnectionSettings {
            mode: ConnectionMode::Local,
            port: 18789,
            host: None,
            token: Some("sekrit".into()),
            password: None,
            use_tls: false,
            ..Default::default()
        };
        let gateway = OpenClawGatewayConfig {
            port: Some(18789),
            bind: Some("loopback".into()),
            control_ui_base_path: Some("/openclaw".into()),
            ..Default::default()
        };
        let auth = resolve_dashboard_auth(&settings, &gateway).unwrap();
        assert_eq!(auth.http_url.as_str(), "http://127.0.0.1:18789/openclaw");
        assert_eq!(auth.ws_url, "ws://127.0.0.1:18789/openclaw");
        let with_frag = dashboard_url_with_token_fragment(auth.http_url.clone(), auth.token.as_deref());
        assert_eq!(with_frag.fragment(), Some("token=sekrit"));
        let chat = control_ui_chat_url(&auth);
        assert_eq!(chat.path(), "/openclaw/chat");
        assert_eq!(chat.fragment(), Some("token=sekrit"));
    }
}
