/// Tray tooltip text from the latest gateway/operator connection status line.
pub fn format_tray_tooltip(connection_status: &str) -> String {
    let status = connection_status.trim();
    if status.is_empty() {
        "OpenClaw".to_string()
    } else {
        format!("OpenClaw — {status}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_tray_tooltip;

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
}
