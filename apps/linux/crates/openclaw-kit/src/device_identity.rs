//! Device Ed25519 identity compatible with `src/infra/device-identity.ts`.

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const ED25519_SPKI_PREFIX: &[u8] = &[
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
const ED25519_PKCS8_PRIVATE_PREFIX: &[u8] = &[
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
    0x20,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub public_key_pem: String,
    pub private_key_pem: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredIdentity {
    version: u32,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "publicKeyPem")]
    public_key_pem: String,
    #[serde(rename = "privateKeyPem")]
    private_key_pem: String,
    #[serde(rename = "createdAtMs")]
    created_at_ms: u64,
}

pub fn identity_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("identity")
        .join("device.json")
}

fn base64_standard_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn pem_encode(label: &str, der: &[u8]) -> String {
    let body = base64_standard_encode(der)
        .as_bytes()
        .chunks(64)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    format!("-----BEGIN {label}-----\n{body}\n-----END {label}-----\n")
}

fn public_key_pem_from_raw(raw: &[u8; 32]) -> String {
    let mut der = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + 32);
    der.extend_from_slice(ED25519_SPKI_PREFIX);
    der.extend_from_slice(raw);
    pem_encode("PUBLIC KEY", &der)
}

fn private_key_pem_from_raw(raw: &[u8; 32]) -> String {
    let mut der = Vec::with_capacity(ED25519_PKCS8_PRIVATE_PREFIX.len() + 32);
    der.extend_from_slice(ED25519_PKCS8_PRIVATE_PREFIX);
    der.extend_from_slice(raw);
    pem_encode("PRIVATE KEY", &der)
}

fn fingerprint_public_key_raw(raw: &[u8; 32]) -> String {
    hex::encode(Sha256::digest(raw))
}

fn generate_identity() -> DeviceIdentity {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_raw = verifying_key.to_bytes();
    let private_raw = signing_key.to_bytes();
    DeviceIdentity {
        device_id: fingerprint_public_key_raw(&public_raw),
        public_key_pem: public_key_pem_from_raw(&public_raw),
        private_key_pem: private_key_pem_from_raw(&private_raw),
    }
}

pub fn load_or_create_device_identity() -> std::io::Result<DeviceIdentity> {
    let path = identity_path();
    if path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(stored) = serde_json::from_str::<StoredIdentity>(&raw) {
                return Ok(DeviceIdentity {
                    device_id: stored.device_id,
                    public_key_pem: stored.public_key_pem,
                    private_key_pem: stored.private_key_pem,
                });
            }
        }
    }
    let identity = generate_identity();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stored = StoredIdentity {
        version: 1,
        device_id: identity.device_id.clone(),
        public_key_pem: identity.public_key_pem.clone(),
        private_key_pem: identity.private_key_pem.clone(),
        created_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    };
    std::fs::write(path, serde_json::to_string_pretty(&stored).unwrap())?;
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_pem_identity() {
        let id = generate_identity();
        assert_eq!(id.device_id.len(), 64);
        assert!(id.public_key_pem.contains("PUBLIC KEY"));
        assert!(id.private_key_pem.contains("PRIVATE KEY"));
    }
}
