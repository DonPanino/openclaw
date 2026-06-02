use std::process::Stdio;
use tokio::process::{Child, Command};

/// `ssh -N -L` port forward for remote gateway mode (macOS RemotePortTunnel parity).
pub struct RemoteTunnel {
    child: Child,
    pub local_port: u16,
}

impl RemoteTunnel {
    pub async fn start(
        ssh_target: &str,
        ssh_identity: Option<&str>,
        remote_port: u16,
        local_port: u16,
    ) -> Result<Self, String> {
        let target = ssh_target.trim();
        if target.is_empty() {
            return Err("gateway.remote.sshTarget is empty".into());
        }

        let forward = format!("{local_port}:127.0.0.1:{remote_port}");
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            "TCPKeepAlive=yes",
            "-n",
            "-N",
            "-L",
            &forward,
        ]);
        if let Some(identity) = ssh_identity.filter(|s| !s.trim().is_empty()) {
            cmd.arg("-i").arg(identity.trim());
        }
        cmd.arg(target);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(Self { child, local_port })
    }

    pub async fn stop(mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

impl Drop for RemoteTunnel {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
