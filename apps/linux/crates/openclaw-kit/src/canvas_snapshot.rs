//! Canvas webview snapshot helpers (PNG data URL → base64).

/// Extract standard/base64 PNG payload from a webview `canvas.toDataURL` or A2UI snapshot string.
pub fn png_base64_from_data_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"');
    let payload = trimmed
        .strip_prefix("data:image/png;base64,")
        .or_else(|| trimmed.strip_prefix("data:image/png;charset=utf-8;base64,"))?;
    if payload.is_empty() {
        return None;
    }
    Some(payload.to_string())
}

#[cfg(test)]
mod tests {
    use super::png_base64_from_data_url;

    #[test]
    fn parses_standard_png_data_url() {
        assert_eq!(
            png_base64_from_data_url("data:image/png;base64,abc123"),
            Some("abc123".into())
        );
    }

    #[test]
    fn parses_quoted_and_charset_variants() {
        assert_eq!(
            png_base64_from_data_url(r#""data:image/png;charset=utf-8;base64,xyz""#),
            Some("xyz".into())
        );
    }

    #[test]
    fn rejects_empty_or_non_png() {
        assert_eq!(png_base64_from_data_url("data:image/png;base64,"), None);
        assert_eq!(png_base64_from_data_url("data:image/jpeg;base64,abc"), None);
        assert_eq!(png_base64_from_data_url(""), None);
    }
}
