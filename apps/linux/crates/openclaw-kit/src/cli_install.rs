//! CLI install/detect helpers (macOS `CLIInstaller.swift` parity).

use std::path::PathBuf;

const INSTALL_SCRIPT_URL: &str = "https://openclaw.bot/install-cli.sh";

/// Search paths for an existing `openclaw` binary (same order as typical login shells).
pub fn preferred_cli_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(&home);
        paths.push(home.join(".openclaw/bin"));
        paths.push(home.join(".local/bin"));
    }
    if let Ok(path) = std::env::var("PATH") {
        for part in path.split(':').filter(|p| !p.is_empty()) {
            paths.push(PathBuf::from(part));
        }
    }
    paths.push(PathBuf::from("/usr/local/bin"));
    paths.push(PathBuf::from("/usr/bin"));
    paths
}

/// Absolute path to `openclaw` when executable on disk.
/// CLI binary for gateway subprocesses (`OPENCLAW_BIN` overrides).
pub fn resolve_openclaw_bin() -> String {
    if let Ok(bin) = std::env::var("OPENCLAW_BIN") {
        if !bin.trim().is_empty() {
            return bin;
        }
    }
    installed_location()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "openclaw".into())
}

pub fn installed_location() -> Option<PathBuf> {
    for base in preferred_cli_paths() {
        let candidate = base.join("openclaw");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn default_install_prefix() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
}

/// Shell command run under `bash -lc` (JSON install events on stdout).
pub fn install_shell_command(version: &str) -> String {
    let version = shell_escape(version);
    let prefix = shell_escape(&default_install_prefix().to_string_lossy());
    format!(
        "curl -fsSL {INSTALL_SCRIPT_URL} | bash -s -- --json --no-onboard --prefix {prefix} --version {version}"
    )
}

fn shell_escape(raw: &str) -> String {
    if raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | '+'))
    {
        return raw.to_string();
    }
    format!("'{}'", raw.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_command_includes_version_and_prefix() {
        let cmd = install_shell_command("2026.6.2");
        assert!(cmd.contains("install-cli.sh"));
        assert!(cmd.contains("2026.6.2"));
        assert!(cmd.contains("--no-onboard"));
    }

    #[test]
    fn resolve_openclaw_bin_prefers_env() {
        std::env::set_var("OPENCLAW_BIN", "/tmp/custom-openclaw");
        assert_eq!(resolve_openclaw_bin(), "/tmp/custom-openclaw");
        std::env::remove_var("OPENCLAW_BIN");
    }

    #[test]
    fn shell_escape_quotes_spaces() {
        assert_eq!(shell_escape("latest"), "latest");
        assert_eq!(shell_escape("a b"), "'a b'");
    }
}
