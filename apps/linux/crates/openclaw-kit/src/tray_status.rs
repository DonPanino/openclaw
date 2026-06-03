/// Tray tooltip text from the latest gateway/operator connection status line.
pub fn format_tray_tooltip(connection_status: &str) -> String {
    let status = connection_status.trim();
    if status.is_empty() {
        "OpenClaw".to_string()
    } else {
        format!("OpenClaw — {status}")
    }
}

/// Tray tooltip with operator/node/tunnel slots (matches Settings conn bar).
pub fn format_tray_tooltip_health(
    operator_connected: bool,
    node_connected: bool,
    ssh_tunnel_active: bool,
    connection_status: &str,
) -> String {
    let op = if operator_connected {
        "operator ✓"
    } else {
        "operator ✗"
    };
    let node = if node_connected { "node ✓" } else { "node ✗" };
    let mut slots = vec![op, node];
    if ssh_tunnel_active {
        slots.push("tunnel ✓");
    }
    let detail = connection_status.trim();
    if detail.is_empty() {
        format!("OpenClaw — {}", slots.join(" · "))
    } else {
        format!("OpenClaw — {} — {}", slots.join(" · "), detail)
    }
}

#[cfg(test)]
mod tests {
    use super::{format_tray_tooltip, format_tray_tooltip_health};

    #[test]
    fn empty_status_uses_product_name_only() {
        assert_eq!(format_tray_tooltip(""), "OpenClaw");
        assert_eq!(format_tray_tooltip("   "), "OpenClaw");
    }

    #[test]
    fn non_empty_status_is_prefixed() {
        assert_eq!(
            format_tray_tooltip("operator connected"),
            "OpenClaw — operator connected"
        );
        assert_eq!(
            format_tray_tooltip("operator: connection refused"),
            "OpenClaw — operator: connection refused"
        );
    }

    #[test]
    fn health_tooltip_includes_slots_and_detail() {
        assert_eq!(
            format_tray_tooltip_health(true, true, false, ""),
            "OpenClaw — operator ✓ · node ✓"
        );
        assert_eq!(
            format_tray_tooltip_health(false, false, true, "SSH tunnel active"),
            "OpenClaw — operator ✗ · node ✗ · tunnel ✓ — SSH tunnel active"
        );
    }
}
