use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecApprovalsFile {
    pub version: u32,
    #[serde(default)]
    pub defaults: ExecDefaults,
    #[serde(default)]
    pub agents: HashMap<String, ExecAgentPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecDefaults {
    #[serde(default)]
    pub security: String,
    #[serde(default)]
    pub ask: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecAgentPolicy {
    #[serde(default)]
    pub security: String,
    #[serde(default)]
    pub ask: String,
    #[serde(default)]
    pub allowlist: Vec<ExecAllowlistEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecAllowlistEntry {
    pub pattern: String,
}

impl ExecApprovalsFile {
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw")
            .join("exec-approvals.json")
    }

    pub fn load() -> std::io::Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            return Ok(Self {
                version: 1,
                ..Default::default()
            });
        }
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
}
