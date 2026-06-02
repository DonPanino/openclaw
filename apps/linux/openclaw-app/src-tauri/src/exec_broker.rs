//! Bridges the node-host exec approval Unix socket to Tauri UI prompts.

use openclaw_node_host::ExecPromptRequest;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};

pub struct ExecApprovalBroker {
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
    inbound: mpsc::Sender<(String, ExecPromptRequest, oneshot::Sender<String>)>,
}

impl ExecApprovalBroker {
    pub fn new(app: AppHandle) -> Self {
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (inbound_tx, mut inbound_rx) =
            mpsc::channel::<(String, ExecPromptRequest, oneshot::Sender<String>)>(16);
        let pending_loop = pending.clone();
        let app_loop = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some((id, req, reply_tx)) = inbound_rx.recv().await {
                pending_loop.lock().await.insert(id.clone(), reply_tx);
                let summary = req
                    .command
                    .lines()
                    .next()
                    .unwrap_or("command")
                    .chars()
                    .take(120)
                    .collect::<String>();
                let _ = notify_rust::Notification::new()
                    .summary("OpenClaw exec approval")
                    .body(&summary)
                    .show();
                let _ = app_loop.emit(
                    "exec-approval-request",
                    json!({
                        "id": id,
                        "command": req.command,
                        "cwd": req.cwd,
                        "agentId": req.agent_id,
                    }),
                );
            }
        });
        Self {
            pending,
            inbound: inbound_tx,
        }
    }

    pub async fn prompt(&self, id: String, req: ExecPromptRequest) -> String {
        let (tx, rx) = oneshot::channel();
        if self.inbound.send((id, req, tx)).await.is_err() {
            return "deny".into();
        }
        rx.await.unwrap_or_else(|_| "deny".into())
    }

    pub async fn resolve(&self, id: &str, decision: &str) -> Result<(), String> {
        let reply = self
            .pending
            .lock()
            .await
            .remove(id)
            .ok_or("no pending exec approval for id")?;
        let normalized = match decision {
            "allow" | "allow-once" | "allow-always" => decision,
            _ => "deny",
        };
        let _ = reply.send(normalized.into());
        Ok(())
    }
}
