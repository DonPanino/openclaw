//! KDE/Wayland taskbars resolve icons via GApplication app id + `.desktop` + hicolor theme,
//! not only the GTK window icon. Install a user-level desktop entry in dev/uninstalled runs.

use std::fs;
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "ai.openclaw.linux";

const ICON_NAME: &str = "ai.openclaw.linux";
const SKIP_ENV: &str = "OPENCLAW_SKIP_DESKTOP_INTEGRATION";

/// Register icon + `.desktop` under `~/.local/share` when not installed system-wide.
pub fn ensure_desktop_integration() {
    if std::env::var_os(SKIP_ENV).is_some() {
        return;
    }
    if system_desktop_installed() {
        return;
    }
    let Some(icon_src) = resolve_icon_png() else {
        tracing::debug!("desktop integration: icon source not found");
        return;
    };
    if let Err(err) = install_user_icons(&icon_src) {
        tracing::warn!("desktop integration: failed to install icons: {err}");
        return;
    }
    if let Err(err) = install_user_desktop_file() {
        tracing::warn!("desktop integration: failed to install desktop entry: {err}");
    }
}

fn system_desktop_installed() -> bool {
    Path::new("/usr/share/applications")
        .join(format!("{APP_ID}.desktop"))
        .is_file()
}

fn resolve_icon_png() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("OPENCLAW_REPO_ROOT") {
        let path = PathBuf::from(root).join("apps/linux/openclaw-app/src-tauri/icons/icon.png");
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for candidate in [
        dir.join("icons/icon.png"),
        dir.join("../../openclaw-app/src-tauri/icons/icon.png"),
        dir.join("../share/icons/hicolor/512x512/apps/ai.openclaw.linux.png"),
    ] {
        if candidate.is_file() {
            return candidate.canonicalize().ok();
        }
    }
    None
}

fn user_data_dir(sub: &str) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(sub))
}

fn install_user_icons(src: &Path) -> Result<(), String> {
    let base = user_data_dir(".local/share/icons/hicolor")?;
    for size in ["32x32", "128x128", "256x256", "512x512"] {
        let dest_dir = base.join(format!("{size}/apps"));
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        fs::copy(src, dest_dir.join(format!("{ICON_NAME}.png"))).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn install_user_desktop_file() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exec = shell_escape(&exe.display().to_string());
    let desktop = format!(
        r#"[Desktop Entry]
Type=Application
Version=1.0
Name=OpenClaw
GenericName=OpenClaw Linux companion
Comment=OpenClaw Linux companion
Exec={exec} %U
Icon={ICON_NAME}
Terminal=false
Categories=Network;Chat;
MimeType=x-scheme-handler/openclaw;
StartupWMClass={APP_ID}
StartupNotify=true
"#
    );
    let apps_dir = user_data_dir(".local/share/applications")?;
    fs::create_dir_all(&apps_dir).map_err(|e| e.to_string())?;
    fs::write(apps_dir.join(format!("{APP_ID}.desktop")), desktop).map_err(|e| e.to_string())?;
    Ok(())
}

fn shell_escape(path: &str) -> String {
    if path.contains(char::is_whitespace) || path.contains('"') {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_quotes_paths_with_spaces() {
        assert_eq!(shell_escape("/tmp/a"), "/tmp/a");
        assert_eq!(shell_escape("/tmp/a b"), "\"/tmp/a b\"");
    }
}
