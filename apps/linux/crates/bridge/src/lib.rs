//! Peekaboo-style automation bridge stub for Linux (AT-SPI / portal planned).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PEEKABOO_SOCKET_ENV: &str = "OPENCLAW_PEEKABOO_SOCKET";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeState {
    /// Socket server not started; node agents must not assume Peekaboo is available.
    Disabled,
    /// Reserved for future `bridge/` host implementation.
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub state: BridgeState,
    pub socket_path: String,
    pub message: String,
}

pub fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("bridge")
        .join("peekaboo.sock")
}

pub fn resolve_socket_path() -> PathBuf {
    std::env::var(PEEKABOO_SOCKET_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_socket_path)
}

pub fn bridge_status() -> BridgeStatus {
    BridgeStatus {
        state: BridgeState::Planned,
        socket_path: resolve_socket_path().display().to_string(),
        message: "Linux automation bridge is not enabled yet. Planned: AT-SPI + xdg-desktop-portal."
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_socket_under_openclaw_state() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains(".openclaw"));
        assert!(path.to_string_lossy().ends_with("peekaboo.sock"));
    }
}
