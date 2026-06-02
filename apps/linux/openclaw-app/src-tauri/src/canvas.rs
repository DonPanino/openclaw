//! Canvas window + node canvas.* / canvas.a2ui.* commands (macOS CanvasManager parity).

use openclaw_capture as capture;
use openclaw_kit::exec_approvals::ExecApprovalsFile;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn resolve_canvas_scaffold() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/canvas-scaffold/scaffold.html"),
        PathBuf::from("apps/linux/resources/canvas-scaffold/scaffold.html"),
        PathBuf::from(
            "apps/shared/OpenClawKit/Sources/OpenClawKit/Resources/CanvasScaffold/scaffold.html",
        ),
        PathBuf::from("dist/canvas-host/a2ui/index.html"),
    ];
    if let Ok(root) = std::env::var("OPENCLAW_REPO_ROOT") {
        candidates.insert(
            0,
            PathBuf::from(&root)
                .join("apps/linux/resources/canvas-scaffold/scaffold.html"),
        );
        candidates.insert(
            1,
            PathBuf::from(&root).join(
                "apps/shared/OpenClawKit/Sources/OpenClawKit/Resources/CanvasScaffold/scaffold.html",
            ),
        );
        candidates.insert(
            2,
            PathBuf::from(&root).join("dist/canvas-host/a2ui/index.html"),
        );
    }
    candidates.into_iter().find(|p| p.exists())
}

/// Bundled A2UI host (`pnpm canvas:a2ui:bundle` → `dist/canvas-host/a2ui`).
pub fn resolve_a2ui_index() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/a2ui/index.html"),
        PathBuf::from("apps/linux/resources/a2ui/index.html"),
        PathBuf::from("dist/canvas-host/a2ui/index.html"),
        PathBuf::from("extensions/canvas/src/host/a2ui/index.html"),
    ];
    if let Ok(root) = std::env::var("OPENCLAW_REPO_ROOT") {
        candidates.insert(0, PathBuf::from(&root).join("apps/linux/resources/a2ui/index.html"));
        candidates.insert(1, PathBuf::from(&root).join("dist/canvas-host/a2ui/index.html"));
    }
    candidates.into_iter().find(|p| p.exists())
}

fn canvas_label(session: &str) -> String {
    format!("canvas-{session}")
}

fn session_id(params: &Value) -> String {
    params
        .get("sessionKey")
        .or_else(|| params.get("sessionId"))
        .or_else(|| params.get("session"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string()
}

fn file_webview_url(path: PathBuf, session_id: &str) -> Result<WebviewUrl, String> {
    let path = path.canonicalize().unwrap_or(path);
    let mut file_url = url::Url::from_file_path(&path).map_err(|_| "invalid canvas path")?;
    file_url
        .query_pairs_mut()
        .append_pair("session", session_id)
        .append_pair("platform", "linux");
    Ok(WebviewUrl::External(
        tauri::Url::parse(file_url.as_str()).map_err(|e| e.to_string())?,
    ))
}

fn run_on_main<R>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> Result<R, String> + Send + 'static,
) -> Result<R, String>
where
    R: Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel(1);
    let app = app.clone();
    app.clone()
        .run_on_main_thread(move || {
            let _ = tx.send(f(&app));
        })
    .map_err(|e| e.to_string())?;
    rx.recv()
        .map_err(|_| "main thread canvas channel closed".to_string())?
}

fn eval_with_result(app: &AppHandle, session: &str, js: &str) -> Result<String, String> {
    let label = canvas_label(session);
    let js = js.to_string();
    let (tx, rx) = mpsc::sync_channel::<Result<String, String>>(1);
    let app = app.clone();
    let session = session.to_string();
    app.clone()
        .run_on_main_thread(move || {
            let Some(win) = app.get_webview_window(&label) else {
                let _ = tx.send(Err(format!("canvas not open for session {session}")));
                return;
            };
            let _ = win.eval_with_callback(js, move |result| {
                let _ = tx.send(Ok(result));
            });
        })
        .map_err(|e| e.to_string())?;
    match rx.recv_timeout(Duration::from_secs(15)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("canvas eval timed out".into()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("canvas eval failed (window closed or script error)".into())
        }
    }
}

fn decode_jsonl_messages(jsonl: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        out.push(serde_json::from_str(line).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn a2ui_messages_from_params(params: &Value, jsonl_mode: bool) -> Result<Vec<Value>, String> {
    if jsonl_mode {
        let jsonl = params
            .get("jsonl")
            .and_then(|v| v.as_str())
            .ok_or("jsonl required")?;
        return decode_jsonl_messages(jsonl);
    }
    if let Some(messages) = params.get("messages") {
        if let Some(arr) = messages.as_array() {
            return Ok(arr.clone());
        }
    }
    if let Some(jsonl) = params.get("jsonl").and_then(|v| v.as_str()) {
        return decode_jsonl_messages(jsonl);
    }
    Err("messages or jsonl required".into())
}

fn parse_eval_json_result(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(json!({ "raw": raw }))
}

pub fn open_canvas_window(app: &AppHandle, session_id: &str) -> Result<(), String> {
    let session_id = session_id.to_string();
    run_on_main(app, move |app| open_canvas_window_inner(app, &session_id))
}

fn open_canvas_window_inner(app: &AppHandle, session_id: &str) -> Result<(), String> {
    let label = canvas_label(session_id);
    if let Some(win) = app.get_webview_window(&label) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url = if let Some(a2ui) = resolve_a2ui_index() {
        file_webview_url(a2ui, session_id)?
    } else if let Some(scaffold) = resolve_canvas_scaffold() {
        file_webview_url(scaffold, session_id)?
    } else {
        WebviewUrl::App(format!("canvas.html?session={session_id}").into())
    };

    let win = WebviewWindowBuilder::new(app, &label, url)
        .title("OpenClaw Canvas")
        .inner_size(960.0, 720.0)
        .visible(true)
        .focused(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn handle_canvas_command(
    app: &AppHandle,
    command: &str,
    params: &Value,
) -> Result<Value, String> {
    let session = session_id(params);
    match command {
        "canvas.present" => {
            open_canvas_window(app, &session)?;
            Ok(json!({ "ok": true, "sessionId": session }))
        }
        "canvas.hide" => {
            let label = canvas_label(&session);
            run_on_main(app, move |app| {
                if let Some(win) = app.get_webview_window(&label) {
                    win.hide().map_err(|e| e.to_string())?;
                }
                Ok(())
            })?;
            Ok(json!({ "ok": true }))
        }
        "canvas.navigate" => {
            let url = params
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("url required")?;
            let js = format!(
                "window.location.href = {};",
                serde_json::to_string(url).map_err(|e| e.to_string())?
            );
            eval_with_result(app, &session, &js)?;
            Ok(json!({ "ok": true, "url": url }))
        }
        "canvas.eval" => {
            let js = params
                .get("javaScript")
                .or_else(|| params.get("js"))
                .and_then(|v| v.as_str())
                .ok_or("javaScript required")?;
            let raw = eval_with_result(app, &session, js)?;
            Ok(parse_eval_json_result(&raw))
        }
        "canvas.snapshot" => {
            open_canvas_window(app, &session)?;
            // Webview capture is not wired yet; screen snapshot supports agent vision on Linux.
            let bytes = capture::screen_snapshot()
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "ok": true,
                "format": "png",
                "bytes": bytes,
                "note": "screen fallback until webview snapshot is implemented"
            }))
        }
        "canvas.a2ui.reset" => {
            open_canvas_window(app, &session)?;
            let raw = eval_with_result(
                app,
                &session,
                r#"(() => {
                  const host = globalThis.openclawA2UI;
                  if (!host) return JSON.stringify({ ok: false, error: "missing openclawA2UI" });
                  return JSON.stringify(host.reset());
                })()"#,
            )?;
            Ok(parse_eval_json_result(&raw))
        }
        "canvas.a2ui.push" => {
            open_canvas_window(app, &session)?;
            let messages = a2ui_messages_from_params(params, false)?;
            let messages_json = serde_json::to_string(&messages).map_err(|e| e.to_string())?;
            let js = format!(
                r#"(() => {{
                  try {{
                    const host = globalThis.openclawA2UI;
                    if (!host) return JSON.stringify({{ ok: false, error: "missing openclawA2UI" }});
                    const messages = {messages_json};
                    return JSON.stringify(host.applyMessages(messages));
                  }} catch (e) {{
                    return JSON.stringify({{ ok: false, error: String(e?.message ?? e) }});
                  }}
                }})()"#
            );
            let raw = eval_with_result(app, &session, &js)?;
            Ok(parse_eval_json_result(&raw))
        }
        "canvas.a2ui.pushJSONL" => {
            open_canvas_window(app, &session)?;
            let messages = a2ui_messages_from_params(params, true)?;
            let messages_json = serde_json::to_string(&messages).map_err(|e| e.to_string())?;
            let js = format!(
                r#"(() => {{
                  try {{
                    const host = globalThis.openclawA2UI;
                    if (!host) return JSON.stringify({{ ok: false, error: "missing openclawA2UI" }});
                    const messages = {messages_json};
                    return JSON.stringify(host.applyMessages(messages));
                  }} catch (e) {{
                    return JSON.stringify({{ ok: false, error: String(e?.message ?? e) }});
                  }}
                }})()"#
            );
            let raw = eval_with_result(app, &session, &js)?;
            Ok(parse_eval_json_result(&raw))
        }
        other => Err(format!("unsupported canvas command: {other}")),
    }
}

pub async fn handle_node_command(
    app: &AppHandle,
    command: &str,
    params: &Value,
) -> Result<Value, String> {
    if command.starts_with("canvas.") {
        return handle_canvas_command(app, command, params).await;
    }
    match command {
        "system.execApprovals.get" => {
            let file = ExecApprovalsFile::load().map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(file).map_err(|e| e.to_string())?)
        }
        "system.execApprovals.set" => {
            let file: ExecApprovalsFile = if let Some(raw) = params.get("file") {
                serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?
            } else {
                serde_json::from_value(params.clone()).map_err(|e| e.to_string())?
            };
            file.save().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "system.run.prepare" => {
            let command_text = params
                .get("command")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("rawCommand").and_then(|v| v.as_str()))
                .unwrap_or("");
            Ok(json!({
                "plan": {
                    "command": command_text,
                    "cwd": params.get("cwd"),
                    "agentId": params.get("agentId"),
                    "sessionKey": params.get("sessionKey"),
                },
                "execPolicy": {
                    "security": "allow",
                    "ask": "on-miss"
                }
            }))
        }
        "talk.ptt.start" => {
            if let Some(voice) = app.try_state::<crate::voice::VoiceService>() {
                voice.start_ptt()?;
            }
            Ok(json!({ "ok": true }))
        }
        "talk.ptt.stop" => {
            let transcript = if let Some(voice) = app.try_state::<crate::voice::VoiceService>() {
                voice.stop_ptt()?
            } else {
                None
            };
            Ok(json!({ "ok": true, "transcript": transcript.unwrap_or_default() }))
        }
        "talk.ptt.cancel" => {
            if let Some(voice) = app.try_state::<crate::voice::VoiceService>() {
                let _ = voice.stop_ptt()?;
            }
            Ok(json!({ "ok": true, "cancelled": true }))
        }
        "talk.ptt.once" => Ok(json!({ "ok": true, "stub": true })),
        other => Err(format!("unsupported command: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_jsonl_messages() {
        let msgs = decode_jsonl_messages("{\"a\":1}\n\n{\"b\":2}\n").unwrap();
        assert_eq!(msgs.len(), 2);
    }
}
