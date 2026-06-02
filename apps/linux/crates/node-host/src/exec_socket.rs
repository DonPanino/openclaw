use openclaw_kit::exec_approvals::ExecApprovalsFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[derive(Debug, Clone)]
pub struct ExecSocketConfig {
    pub socket_path: PathBuf,
    pub token: String,
}

impl ExecSocketConfig {
    pub fn default_paths() -> Self {
        let dir = dirs::runtime_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("openclaw");
        let _ = std::fs::create_dir_all(&dir);
        Self {
            socket_path: dir.join("exec-approvals.sock"),
            token: uuid::Uuid::new_v4().to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExecSocketRequest {
    #[serde(rename = "type")]
    kind: String,
    token: String,
    id: String,
    request: ExecPromptRequest,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecPromptRequest {
    pub command: String,
    pub cwd: Option<String>,
    #[serde(alias = "agentId")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExecSocketDecision {
    #[serde(rename = "type")]
    kind: String,
    id: String,
    decision: String,
}

pub struct ExecSocketServer {
    config: ExecSocketConfig,
    prompt: std::sync::Arc<dyn Fn(String, ExecPromptRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> + Send + Sync>,
}

impl ExecSocketServer {
    pub fn new(
        config: ExecSocketConfig,
        prompt: impl Fn(String, ExecPromptRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            config,
            prompt: std::sync::Arc::new(prompt),
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let _ = std::fs::remove_file(&self.config.socket_path);
        let listener = UnixListener::bind(&self.config.socket_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &self.config.socket_path,
                std::fs::Permissions::from_mode(0o600),
            )?;
        }
        loop {
            let (stream, _) = listener.accept().await?;
            let token = self.config.token.clone();
            let prompt = self.prompt.clone();
            if let Err(err) = handle_client(stream, token, prompt).await {
                tracing::warn!("exec socket client error: {err}");
            }
        }
    }
}

async fn handle_client(
    stream: UnixStream,
    expected_token: String,
    prompt: std::sync::Arc<dyn Fn(String, ExecPromptRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> + Send + Sync>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let req: ExecSocketRequest = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if req.kind != "exec.approval.request" || req.token != expected_token {
            continue;
        }
        let _approvals = ExecApprovalsFile::load().unwrap_or_default();
        let decision = prompt(req.id.clone(), req.request).await;
        let body = ExecSocketDecision {
            kind: "exec.approval.decision".into(),
            id: req.id,
            decision,
        };
        writer
            .write_all(format!("{}\n", serde_json::to_string(&body).unwrap()).as_bytes())
            .await?;
    }
    Ok(())
}
