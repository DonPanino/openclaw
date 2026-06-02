use serde::Serialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Serialize)]
pub struct WhichResult {
    pub path: Option<String>,
}

pub async fn which(binary: &str) -> WhichResult {
    let output = Command::new("which")
        .arg(binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok();
    let path = output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    WhichResult { path }
}

pub async fn run_command(command: &str, cwd: Option<&str>) -> Result<(i32, String, String), std::io::Error> {
    let mut cmd = if let Some(shell) = std::env::var_os("SHELL") {
        let mut c = Command::new(shell);
        c.arg("-lc").arg(command);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-lc").arg(command);
        c
    };
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().await?;
    let code = output.status.code().unwrap_or(-1);
    Ok((
        code,
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}
