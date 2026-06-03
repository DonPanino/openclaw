use openclaw_kit::gateway_status_indicates_running;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

const GATEWAY_START_POLL_MS: u64 = 500;
const GATEWAY_START_TIMEOUT_MS: u64 = 45_000;

pub struct GatewayCli;

impl GatewayCli {
    pub fn new() -> Self {
        Self
    }

    fn openclaw_bin() -> String {
        openclaw_kit::resolve_openclaw_bin()
    }

    pub async fn gateway_status_json(&self) -> Result<String, String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["gateway", "status", "--json", "--no-probe"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    pub async fn is_gateway_running(&self) -> Result<bool, String> {
        match self.gateway_status_json().await {
            Ok(json) => Ok(gateway_status_indicates_running(&json)),
            Err(_) => Ok(false),
        }
    }

    pub async fn gateway_install(&self) -> Result<String, String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["gateway", "install"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    pub async fn stop_gateway_service(&self) -> Result<(), String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["gateway", "stop"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }

    pub async fn restart_gateway_service(&self) -> Result<(), String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["gateway", "restart"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }

    pub async fn start_gateway_service(&self) -> Result<(), String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["gateway", "start"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }

    pub async fn node_install(&self) -> Result<String, String> {
        let output = Command::new(Self::openclaw_bin())
            .args(["node", "install"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    /// Start the gateway user service only when it is not already running; wait until status reports up.
    pub async fn ensure_gateway_running(&self) -> Result<(), String> {
        if self.is_gateway_running().await? {
            tracing::info!("gateway already running; skipping start");
            return Ok(());
        }
        tracing::info!("gateway not running; starting via openclaw gateway start");
        if let Err(err) = self.start_gateway_service().await {
            tracing::warn!("openclaw gateway start: {err}; trying systemctl");
            let status = Command::new("systemctl")
                .args(["--user", "start", "openclaw-gateway.service"])
                .status()
                .await
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!(
                    "failed to start gateway: {err} (systemctl exit {:?})",
                    status.code()
                ));
            }
        }
        let deadline = Duration::from_millis(GATEWAY_START_TIMEOUT_MS);
        let started = std::time::Instant::now();
        while started.elapsed() < deadline {
            if self.is_gateway_running().await? {
                tracing::info!("gateway is running");
                return Ok(());
            }
            sleep(Duration::from_millis(GATEWAY_START_POLL_MS)).await;
        }
        Err(
            "gateway did not become ready in time; run openclaw gateway install && openclaw gateway status"
                .into(),
        )
    }
}
