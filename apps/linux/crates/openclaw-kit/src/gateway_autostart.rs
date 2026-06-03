use crate::gateway_config::{ConnectionMode, GatewayConnectionSettings};

/// macOS `GatewayAutostartPolicy` parity: local mode + user opt-in.
pub fn should_autostart_gateway(settings: &GatewayConnectionSettings) -> bool {
    settings.mode == ConnectionMode::Local && settings.gateway_autostart
}

/// Gateway semver from `openclaw gateway status --json` (probe or daemon gather).
pub fn gateway_version_from_daemon_status_json(json: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
    gateway_version_from_status_value(&value)
}

pub fn gateway_version_from_status_value(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/gateway/version")
        .or_else(|| value.pointer("/rpc/server/version"))
        .or_else(|| value.pointer("/rpc/version"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `openclaw gateway status --json` shape (daemon status gather).
pub fn gateway_status_indicates_running(json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return json.contains("\"status\":\"running\"")
            || json.contains("\"status\": \"running\"");
    };
    if value
        .get("rpc")
        .and_then(|rpc| rpc.get("ok"))
        .and_then(|ok| ok.as_bool())
        .unwrap_or(false)
    {
        return true;
    }
    value
        .pointer("/service/runtime/status")
        .and_then(|s| s.as_str())
        == Some("running")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_config::{ConnectionMode, GatewayConnectionSettings};

    #[test]
    fn autostart_only_for_local_when_enabled() {
        let mut s = GatewayConnectionSettings::default();
        s.gateway_autostart = true;
        s.mode = ConnectionMode::Local;
        assert!(should_autostart_gateway(&s));
        s.mode = ConnectionMode::Remote;
        assert!(!should_autostart_gateway(&s));
        s.mode = ConnectionMode::Local;
        s.gateway_autostart = false;
        assert!(!should_autostart_gateway(&s));
    }

    #[test]
    fn parses_running_from_daemon_status_json() {
        let json = r#"{"service":{"runtime":{"status":"running"}},"rpc":{"ok":false}}"#;
        assert!(gateway_status_indicates_running(json));
        let rpc_ok = r#"{"service":{"runtime":{"status":"stopped"}},"rpc":{"ok":true}}"#;
        assert!(gateway_status_indicates_running(rpc_ok));
    }

    #[test]
    fn parses_gateway_version_from_status_json() {
        let json = r#"{"gateway":{"version":"2026.6.2"},"rpc":{"ok":true,"server":{"version":"2026.6.2"}}}"#;
        assert_eq!(
            gateway_version_from_daemon_status_json(json).as_deref(),
            Some("2026.6.2")
        );
        let rpc_only = r#"{"rpc":{"version":"1.2.3"}}"#;
        assert_eq!(
            gateway_version_from_daemon_status_json(rpc_only).as_deref(),
            Some("1.2.3")
        );
    }
}
