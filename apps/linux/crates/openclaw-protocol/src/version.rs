//! Must match `packages/gateway-protocol/src/version.ts`.

pub const PROTOCOL_VERSION: u32 = 4;
pub const MIN_CLIENT_PROTOCOL_VERSION: u32 = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn protocol_version_matches_gateway_package() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repo root");
        let version_ts = std::fs::read_to_string(
            root.join("packages/gateway-protocol/src/version.ts"),
        )
        .expect("read version.ts");
        assert!(
            version_ts.contains(&format!("PROTOCOL_VERSION = {}", super::PROTOCOL_VERSION)),
            "Rust PROTOCOL_VERSION must match packages/gateway-protocol/src/version.ts"
        );
    }
}
