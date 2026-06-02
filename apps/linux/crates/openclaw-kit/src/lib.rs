pub mod dashboard;
pub mod deep_links;
pub mod remote_tunnel;
pub mod device_identity;
pub mod discovery;
pub mod exec_approvals;
pub mod gateway_autostart;
pub mod gateway_config;
pub mod gateway_operator;
pub mod node_caps;
pub mod openclaw_config;
pub mod tray_status;

pub use deep_links::{parse_deep_link, DeepLink};
pub use device_identity::{load_or_create_device_identity, DeviceIdentity};
pub use gateway_operator::OperatorGateway;
pub use exec_approvals::ExecApprovalsFile;
pub use dashboard::{
    control_ui_chat_url, dashboard_url_with_token_fragment, native_control_auth_init_script,
    resolve_dashboard_auth, DashboardAuth,
};
pub use gateway_autostart::{gateway_status_indicates_running, should_autostart_gateway};
pub use gateway_config::GatewayConnectionSettings;
pub use node_caps::linux_node_advertisement;
pub use openclaw_config::load_gateway_config;
pub use remote_tunnel::RemoteTunnel;
pub use tray_status::format_tray_tooltip;
