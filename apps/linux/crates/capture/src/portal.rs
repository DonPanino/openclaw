//! xdg-desktop-portal Screenshot (Wayland-friendly) before grim fallbacks.

use crate::CaptureError;
use std::path::PathBuf;
use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::Connection;
use zbus_macros::proxy;

#[proxy(
    interface = "org.freedesktop.portal.Screenshot",
    default_path = "/org/freedesktop/portal/desktop",
    default_service = "org.freedesktop.portal.Desktop"
)]
trait PortalScreenshot {
    fn screenshot(
        &self,
        parent_window: &str,
        options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.portal.Request",
    default_service = "org.freedesktop.portal.Desktop"
)]
trait PortalRequest {
    #[zbus(signal)]
    fn response(
        &self,
        response: u32,
        results: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

pub async fn portal_screen_snapshot() -> Result<Vec<u8>, CaptureError> {
    let connection = Connection::session()
        .await
        .map_err(|e| CaptureError::Portal(e.to_string()))?;
    let portal = PortalScreenshotProxy::new(&connection)
        .await
        .map_err(|e| CaptureError::Portal(e.to_string()))?;

    let mut options = std::collections::HashMap::new();
    options.insert(
        "interactive",
        zbus::zvariant::Value::Bool(false),
    );
    options.insert(
        "handle_token",
        zbus::zvariant::Value::Str("openclaw".into()),
    );

    let request_path = portal
        .screenshot("", options)
        .await
        .map_err(|e| CaptureError::Portal(e.to_string()))?;

    let request = PortalRequestProxy::builder(&connection)
        .path(request_path)
        .map_err(|e| CaptureError::Portal(e.to_string()))?
        .build()
        .await
        .map_err(|e| CaptureError::Portal(e.to_string()))?;

    let mut stream = request
        .receive_response()
        .await
        .map_err(|e| CaptureError::Portal(e.to_string()))?;

    let signal = tokio::time::timeout(std::time::Duration::from_secs(30), stream.next())
        .await
        .map_err(|_| CaptureError::Portal("portal screenshot timed out".into()))?
        .ok_or_else(|| CaptureError::Portal("portal closed request stream".into()))?;

    let args = signal
        .args()
        .map_err(|e| CaptureError::Portal(e.to_string()))?;
    if args.response != 0 {
        return Err(CaptureError::Portal(format!(
            "portal denied (code {})",
            args.response
        )));
    }

    let uri = args
        .results
        .get("uri")
        .and_then(|v| {
            v.downcast_ref::<zbus::zvariant::Str<'_>>()
                .ok()
                .map(|s| s.as_str().to_string())
                .or_else(|| v.downcast_ref::<String>().ok().map(|s| s.clone()))
        })
        .ok_or_else(|| CaptureError::Portal("portal response missing uri".into()))?;

    read_portal_uri(&uri).await
}

fn portal_uri_path(uri: &str) -> PathBuf {
    let Some(rest) = uri.strip_prefix("file://") else {
        return PathBuf::from(uri);
    };
    if rest.starts_with('/') {
        return PathBuf::from(rest);
    }
    PathBuf::from(format!("/{rest}"))
}

async fn read_portal_uri(uri: &str) -> Result<Vec<u8>, CaptureError> {
    let path = portal_uri_path(uri);
    tokio::fs::read(&path)
        .await
        .map_err(|e| CaptureError::Failed(format!("read portal screenshot: {e}")))
}
