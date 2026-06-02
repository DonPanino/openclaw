mod canvas;
mod exec_broker;
mod gateway_cli;
mod gateway_runtime;
mod node_runtime;
mod pairing;
mod state;
mod voice;

use gateway_cli::GatewayCli;
use openclaw_kit::gateway_config::{ConnectionMode, GatewayConnectionSettings};
use openclaw_kit::{load_gateway_config, should_autostart_gateway, RemoteTunnel};
use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_shell::ShellExt;
use tracing_subscriber::EnvFilter;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .manage(voice::VoiceService::new())
        .setup(|app| {
            setup_tray(app.handle())?;
            let exec_broker = Arc::new(exec_broker::ExecApprovalBroker::new(app.handle().clone()));
            app.manage(exec_broker.clone());
            let settings = GatewayConnectionSettings::load();
            if should_autostart_gateway(&settings) {
                let cli = GatewayCli::new();
                if let Err(err) = tauri::async_runtime::block_on(cli.ensure_gateway_running()) {
                    tracing::warn!("gateway autostart before UI: {err}");
                    emit_connection_error(app.handle(), err);
                }
            }
            let app_handle = app.handle().clone();
            let settings_spawn = settings.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = ensure_connection_mode(&app_handle, &settings_spawn).await {
                    tracing::warn!("connection mode setup: {err}");
                }
                gateway_runtime::spawn_background_services(
                    app_handle.clone(),
                    settings_spawn,
                    exec_broker,
                );
            });
            pairing::start_pairing_poll(app.handle().clone());
            start_tray_status_poll(app.handle().clone());
            register_deep_links(app.handle());
            // Open dashboard on the GTK main thread (Wayland-safe); settings if URL/auth fails.
            if let Err(err) = show_dashboard_window(app.handle()) {
                tracing::error!("failed to show dashboard on launch: {err}");
                emit_connection_error(app.handle(), err.clone());
                if let Err(fallback) = show_settings_window(app.handle()) {
                    tracing::error!("failed to show settings fallback: {fallback}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_gateway_status,
            install_gateway_service,
            open_dashboard,
            open_webchat,
            open_settings,
            save_connection_settings,
            get_connection_settings,
            discover_gateways,
            get_control_ui_path,
            get_exec_approvals,
            save_exec_approvals,
            install_cli,
            open_canvas_window,
            open_test_canvas,
            operator_health,
            operator_channels_status,
            operator_skills_status,
            operator_cron_list,
            operator_sessions_list,
            operator_config_get,
            operator_connect,
            install_node_service,
            pairing_list_devices,
            pairing_list_nodes,
            pairing_approve_device,
            pairing_reject_device,
            pairing_approve_node,
            pairing_reject_node,
            operator_instances,
            resolve_exec_approval,
            start_remote_tunnel,
            stop_remote_tunnel,
            get_connection_status,
            get_voice_settings,
            save_voice_settings,
            set_gateway_autostart,
            ensure_gateway_running_cmd,
            get_capture_diagnostics,
            get_voice_diagnostics,
            restart_gateway_service_cmd,
            stop_gateway_service_cmd,
            voice_ptt_start,
            voice_ptt_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error running OpenClaw Linux app");
}

/// GTK/WebKit window ops must run on the main thread (avoids Wayland EPROTO crashes).
fn run_window_on_main<R>(app: &AppHandle, f: impl FnOnce(&AppHandle) -> Result<R, String> + Send + 'static) -> Result<R, String>
where
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let app = app.clone();
    app.clone().run_on_main_thread(move || {
        let _ = tx.send(f(&app));
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|_| "main thread window channel closed".to_string())?
}

fn schedule_window_on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) -> Result<(), String> + Send + 'static) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Err(err) = f(&app) {
            tracing::error!("window action failed: {err}");
        }
    });
}

fn show_settings_window(app: &AppHandle) -> Result<(), String> {
    const LABEL: &str = "main";
    if let Some(win) = app.get_webview_window(LABEL) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Some(win) = app.get_webview_window("settings") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("OpenClaw Settings")
        .inner_size(1000.0, 760.0)
        .visible(true)
        .focused(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?
        .set_focus()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn show_dashboard_window(app: &AppHandle) -> Result<(), String> {
    let settings = GatewayConnectionSettings::load();
    let gateway = openclaw_kit::load_gateway_config();
    let auth = openclaw_kit::resolve_dashboard_auth(&settings, &gateway)?;
    let dashboard_url =
        openclaw_kit::dashboard_url_with_token_fragment(auth.http_url.clone(), auth.token.as_deref());
    let init_script = openclaw_kit::native_control_auth_init_script(&auth);
    let label = "dashboard";
    if let Some(win) = app.get_webview_window(label) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        let _ = win.eval(&init_script);
        hide_settings_windows_if_visible(app);
        return Ok(());
    }
    let external =
        tauri::Url::parse(dashboard_url.as_str()).map_err(|e: url::ParseError| e.to_string())?;
    let win = WebviewWindowBuilder::new(app, label, WebviewUrl::External(external))
        .title("OpenClaw Dashboard")
        .inner_size(1200.0, 900.0)
        .visible(true)
        .focused(true)
        .center()
        .initialization_script(init_script)
        .build()
        .map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    hide_settings_windows_if_visible(app);
    Ok(())
}

fn hide_settings_windows_if_visible(app: &AppHandle) {
    for label in ["main", "settings"] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.hide();
        }
    }
}

fn show_webchat_window(app: &AppHandle) -> Result<(), String> {
    let settings = GatewayConnectionSettings::load();
    let gateway = openclaw_kit::load_gateway_config();
    let auth = openclaw_kit::resolve_dashboard_auth(&settings, &gateway)?;
    let webchat_url = openclaw_kit::control_ui_chat_url(&auth);
    let init_script = openclaw_kit::native_control_auth_init_script(&auth);
    let label = "webchat";
    if let Some(win) = app.get_webview_window(label) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        let _ = win.eval(&init_script);
        return Ok(());
    }
    let external =
        tauri::Url::parse(webchat_url.as_str()).map_err(|e: url::ParseError| e.to_string())?;
    let win = WebviewWindowBuilder::new(app, label, WebviewUrl::External(external))
        .title("OpenClaw WebChat")
        .inner_size(900.0, 800.0)
        .visible(true)
        .focused(true)
        .center()
        .initialization_script(init_script)
        .build()
        .map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let dashboard_menu = MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?;
    let webchat_menu = MenuItem::with_id(app, "webchat", "Open WebChat", true, None::<&str>)?;
    let settings_menu = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&dashboard_menu, &webchat_menu, &settings_menu, &quit])?;

    let mut tray_builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("OpenClaw");
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }
    let _tray = tray_builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "dashboard" => {
                schedule_window_on_main(app, show_dashboard_window);
            }
            "webchat" => {
                schedule_window_on_main(app, show_webchat_window);
            }
            "settings" => {
                schedule_window_on_main(app, show_settings_window);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                schedule_window_on_main(tray.app_handle(), show_dashboard_window);
            }
        })
        .build(app)?;

    Ok(())
}

async fn ensure_connection_mode(
    app: &AppHandle,
    settings: &GatewayConnectionSettings,
) -> Result<(), String> {
    if settings.mode == ConnectionMode::Local {
        if let Some(state) = app.try_state::<AppState>() {
            state.stop_remote_tunnel().await;
        }
        if should_autostart_gateway(settings) {
            GatewayCli::new().ensure_gateway_running().await?;
        }
        return Ok(());
    }
    if settings.uses_ssh_tunnel() {
        start_remote_tunnel_inner(app, settings).await?;
    } else if let Some(state) = app.try_state::<AppState>() {
        state.stop_remote_tunnel().await;
    }
    Ok(())
}

async fn start_remote_tunnel_inner(
    app: &AppHandle,
    settings: &GatewayConnectionSettings,
) -> Result<(), String> {
    let gateway = load_gateway_config();
    let ssh_target = settings
        .ssh_target
        .as_deref()
        .ok_or("remote mode requires gateway.remote.sshTarget")?;
    let remote_port = GatewayConnectionSettings::remote_gateway_port(&gateway);
    let local_port = settings.port;
    let tunnel = RemoteTunnel::start(
        ssh_target,
        settings.ssh_identity.as_deref(),
        remote_port,
        local_port,
    )
    .await?;
    if let Some(state) = app.try_state::<AppState>() {
        state.set_remote_tunnel(Some(tunnel)).await;
        let tunnel_status = format!("SSH tunnel active on port {local_port}");
        state.set_connection_status(tunnel_status.clone()).await;
        gateway_runtime::apply_tray_tooltip(app, &tunnel_status);
        let _ = app.emit("gateway-connection-status", tunnel_status);
    }
    Ok(())
}

fn emit_connection_error(app: &AppHandle, message: impl Into<String>) {
    let msg = message.into();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit("connection-error", msg.clone());
        gateway_runtime::publish_connection_status(&app, msg);
    });
}

async fn restart_gateway_runtimes(
    app: &AppHandle,
    settings: GatewayConnectionSettings,
    exec_broker: Arc<exec_broker::ExecApprovalBroker>,
) {
    if let Some(state) = app.try_state::<AppState>() {
        state.stop_node_runtime().await;
        state.clear_operator().await;
    }
    gateway_runtime::spawn_background_services(app.clone(), settings, exec_broker);
}

fn start_tray_status_poll(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            ticker.tick().await;
            let Some(state) = app.try_state::<AppState>() else {
                break;
            };
            let status = state.connection_status().await;
            gateway_runtime::apply_tray_tooltip(&app, &status);
        }
    });
}

fn register_deep_links(app: &AppHandle) {
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            let Some(link) = openclaw_kit::parse_deep_link(url.as_ref()) else {
                continue;
            };
            let app = handle.clone();
            tauri::async_runtime::spawn(async move {
                handle_deep_link(&app, link).await;
            });
        }
    });
}

async fn handle_deep_link(app: &AppHandle, link: openclaw_kit::DeepLink) {
    use openclaw_kit::DeepLink;
    match link {
        DeepLink::Dashboard => {
            let _ = open_dashboard(app.clone()).await;
        }
        DeepLink::WebChat => {
            let _ = run_window_on_main(&app, show_webchat_window);
        }
        DeepLink::Gateway { host, port } => {
            if let Some(state) = app.try_state::<AppState>() {
                let mut settings = state.settings().await;
                if let Some(host) = host.filter(|h| !h.is_empty()) {
                    settings.host = Some(host);
                }
                if let Some(port) = port {
                    settings.port = port;
                }
                let _ = settings.save();
                state.set_settings(settings.clone()).await;
                let _ = ensure_connection_mode(app, &settings).await;
            }
            let _ = open_settings(app.clone()).await;
        }
        DeepLink::Agent { message, .. } => {
            let _ = app.emit("deep-link-agent", message);
            let _ = open_dashboard(app.clone()).await;
        }
    }
}

#[tauri::command]
async fn get_gateway_status() -> Result<String, String> {
    GatewayCli::new()
        .gateway_status_json()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn install_gateway_service() -> Result<String, String> {
    GatewayCli::new()
        .gateway_install()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_gateway_autostart(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.settings().await;
    settings.gateway_autostart = enabled;
    settings.save().map_err(|e| e.to_string())?;
    state.set_settings(settings).await;
    Ok(())
}

#[tauri::command]
async fn ensure_gateway_running_cmd() -> Result<(), String> {
    GatewayCli::new().ensure_gateway_running().await
}

#[tauri::command]
async fn get_connection_settings(state: State<'_, AppState>) -> Result<GatewayConnectionSettings, String> {
    Ok(state.settings().await)
}

#[tauri::command]
async fn save_connection_settings(
    settings: GatewayConnectionSettings,
    state: State<'_, AppState>,
    app: AppHandle,
    exec_broker: State<'_, Arc<exec_broker::ExecApprovalBroker>>,
    voice: State<'_, voice::VoiceService>,
) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())?;
    voice.sync_from_connection(&settings)?;
    state.set_settings(settings.clone()).await;
    ensure_connection_mode(&app, &settings)
        .await
        .map_err(|e| {
            emit_connection_error(&app, e.clone());
            e
        })?;
    restart_gateway_runtimes(&app, settings, exec_broker.inner().clone()).await;
    Ok(())
}

#[tauri::command]
async fn start_remote_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    exec_broker: State<'_, Arc<exec_broker::ExecApprovalBroker>>,
) -> Result<String, String> {
    let settings = state.settings().await;
    start_remote_tunnel_inner(&app, &settings)
        .await
        .map_err(|e| {
            emit_connection_error(&app, e.clone());
            e
        })?;
    restart_gateway_runtimes(&app, settings, exec_broker.inner().clone()).await;
    Ok("tunnel started".into())
}

#[tauri::command]
async fn stop_remote_tunnel(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.stop_remote_tunnel().await;
    let msg = "SSH tunnel stopped";
    state.set_connection_status(msg).await;
    gateway_runtime::apply_tray_tooltip(&app, msg);
    let _ = app.emit("gateway-connection-status", msg);
    Ok(())
}

#[tauri::command]
async fn get_connection_status(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.connection_status().await)
}

#[tauri::command]
async fn get_voice_settings(voice: State<'_, voice::VoiceService>) -> Result<openclaw_voice::VoiceWakeConfig, String> {
    Ok(voice.config())
}

#[tauri::command]
async fn save_voice_settings(
    config: openclaw_voice::VoiceWakeConfig,
    voice: State<'_, voice::VoiceService>,
) -> Result<(), String> {
    voice.apply(config)
}

#[tauri::command]
async fn resolve_exec_approval(
    id: String,
    decision: String,
    broker: State<'_, Arc<exec_broker::ExecApprovalBroker>>,
) -> Result<(), String> {
    broker.resolve(&id, &decision).await
}

#[tauri::command]
async fn discover_gateways() -> Result<String, String> {
    let found = openclaw_kit::discovery::discover_gateways(std::time::Duration::from_secs(3)).await;
    serde_json::to_string_pretty(&found).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_control_ui_path() -> Option<String> {
    resolve_control_ui_index().map(|p| p.to_string_lossy().into_owned())
}

fn resolve_control_ui_index() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../..")
            .join("dist/control-ui/index.html"),
        PathBuf::from("dist/control-ui/index.html"),
    ];
    if let Ok(root) = std::env::var("OPENCLAW_REPO_ROOT") {
        candidates.insert(0, PathBuf::from(root).join("dist/control-ui/index.html"));
    }
    candidates.into_iter().find(|p| p.exists())
}

#[tauri::command]
async fn open_dashboard(app: AppHandle) -> Result<(), String> {
    run_window_on_main(&app, show_dashboard_window)
}

#[tauri::command]
async fn open_webchat(app: AppHandle) -> Result<(), String> {
    run_window_on_main(&app, show_webchat_window)
}

#[tauri::command]
async fn restart_gateway_service_cmd() -> Result<(), String> {
    GatewayCli::new().restart_gateway_service().await
}

#[tauri::command]
async fn stop_gateway_service_cmd() -> Result<(), String> {
    GatewayCli::new().stop_gateway_service().await
}

#[tauri::command]
fn voice_ptt_start(voice: State<'_, voice::VoiceService>) -> Result<(), String> {
    voice.start_ptt()
}

#[tauri::command]
fn voice_ptt_stop(voice: State<'_, voice::VoiceService>) -> Result<Option<String>, String> {
    voice.stop_ptt()
}

#[tauri::command]
async fn get_capture_diagnostics() -> Result<String, String> {
    let diag = openclaw_capture::capture_diagnostics().await;
    serde_json::to_string_pretty(&diag).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_voice_diagnostics(voice: State<'_, voice::VoiceService>) -> Result<String, String> {
    let diag = voice.diagnostics().await?;
    serde_json::to_string_pretty(&diag).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_settings(app: AppHandle) -> Result<(), String> {
    run_window_on_main(&app, show_settings_window)
}

#[tauri::command]
async fn get_exec_approvals() -> Result<openclaw_kit::exec_approvals::ExecApprovalsFile, String> {
    openclaw_kit::exec_approvals::ExecApprovalsFile::load().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_exec_approvals(
    file: openclaw_kit::exec_approvals::ExecApprovalsFile,
) -> Result<(), String> {
    file.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn install_node_service() -> Result<String, String> {
    GatewayCli::new().node_install().await
}

#[tauri::command]
async fn operator_connect(state: State<'_, AppState>) -> Result<String, String> {
    let settings = state.settings().await;
    let op = openclaw_kit::OperatorGateway::connect(&settings).await?;
    state.set_operator(std::sync::Arc::new(op)).await;
    Ok("connected".into())
}

async fn with_operator<F, Fut>(state: State<'_, AppState>, f: F) -> Result<String, String>
where
    F: FnOnce(std::sync::Arc<openclaw_kit::OperatorGateway>) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>>,
{
    let op = state
        .operator()
        .await
        .ok_or("operator not connected; open Connection tab and save settings")?;
    let value = f(op).await?;
    Ok(serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into()))
}

#[tauri::command]
async fn operator_health(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.health().await }).await
}

#[tauri::command]
async fn operator_channels_status(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.channels_status().await }).await
}

#[tauri::command]
async fn operator_skills_status(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.skills_status().await }).await
}

#[tauri::command]
async fn operator_cron_list(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.cron_list().await }).await
}

#[tauri::command]
async fn operator_sessions_list(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.sessions_list().await }).await
}

#[tauri::command]
async fn operator_config_get(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.config_get().await }).await
}

#[tauri::command]
async fn operator_instances(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.node_list().await }).await
}

#[tauri::command]
async fn pairing_list_devices(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.device_pair_list().await }).await
}

#[tauri::command]
async fn pairing_list_nodes(state: State<'_, AppState>) -> Result<String, String> {
    with_operator(state, |op| async move { op.node_pair_list().await }).await
}

#[tauri::command]
async fn pairing_approve_device(state: State<'_, AppState>, request_id: String) -> Result<String, String> {
    with_operator(state, |op| async move { op.device_pair_approve(&request_id).await }).await
}

#[tauri::command]
async fn pairing_reject_device(state: State<'_, AppState>, request_id: String) -> Result<String, String> {
    with_operator(state, |op| async move { op.device_pair_reject(&request_id).await }).await
}

#[tauri::command]
async fn pairing_approve_node(state: State<'_, AppState>, request_id: String) -> Result<String, String> {
    with_operator(state, |op| async move { op.node_pair_approve(&request_id).await }).await
}

#[tauri::command]
async fn pairing_reject_node(state: State<'_, AppState>, request_id: String) -> Result<String, String> {
    with_operator(state, |op| async move { op.node_pair_reject(&request_id).await }).await
}

#[tauri::command]
async fn open_canvas_window(app: AppHandle, session_id: String) -> Result<(), String> {
    let session_id = session_id.clone();
    run_window_on_main(&app, move |app| canvas::open_canvas_window(app, &session_id))
}

#[tauri::command]
async fn open_test_canvas(app: AppHandle) -> Result<(), String> {
    run_window_on_main(&app, |app| canvas::open_canvas_window(app, "main"))
}

#[tauri::command]
async fn install_cli(app: AppHandle) -> Result<String, String> {
    let output = app
        .shell()
        .command("npm")
        .args(["install", "-g", "openclaw@latest"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
