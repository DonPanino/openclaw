//! Device auth payload signing (matches `packages/gateway-client/src/device-auth.ts`).

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

const ED25519_SPKI_PREFIX: &[u8] = &[
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
const ED25519_PKCS8_PRIVATE_PREFIX: &[u8] = &[
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
    0x20,
];

#[derive(Debug, Clone)]
pub struct DeviceSigningMaterial {
    pub device_id: String,
    pub public_key_pem: String,
    pub private_key_pem: String,
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn normalize_metadata(value: &str) -> String {
    value.trim().chars().flat_map(|c| c.to_lowercase()).collect()
}

pub fn build_device_auth_payload_v3(
    device_id: &str,
    client_id: &str,
    client_mode: &str,
    role: &str,
    scopes: &[String],
    signed_at_ms: u64,
    token: Option<&str>,
    nonce: &str,
    platform: Option<&str>,
    device_family: Option<&str>,
) -> String {
    let scopes_joined = scopes.join(",");
    let token = token.unwrap_or("");
    let platform = normalize_metadata(platform.unwrap_or(""));
    let device_family = normalize_metadata(device_family.unwrap_or(""));
    [
        "v3",
        device_id,
        client_id,
        client_mode,
        role,
        scopes_joined.as_str(),
        &signed_at_ms.to_string(),
        token,
        nonce,
        platform.as_str(),
        device_family.as_str(),
    ]
    .join("|")
}

fn extract_public_key_raw(public_key_pem: &str) -> Option<[u8; 32]> {
    let body: String = public_key_pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    use base64::Engine;
    let der = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .ok()?;
    if der.len() == ED25519_SPKI_PREFIX.len() + 32 && der.starts_with(ED25519_SPKI_PREFIX) {
        let mut raw = [0u8; 32];
        raw.copy_from_slice(&der[ED25519_SPKI_PREFIX.len()..]);
        return Some(raw);
    }
    None
}

fn extract_signing_key(private_key_pem: &str) -> Option<SigningKey> {
    let body: String = private_key_pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    use base64::Engine;
    let der = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .ok()?;
    if der.len() == ED25519_PKCS8_PRIVATE_PREFIX.len() + 32
        && der.starts_with(ED25519_PKCS8_PRIVATE_PREFIX)
    {
        let mut raw = [0u8; 32];
        raw.copy_from_slice(&der[ED25519_PKCS8_PRIVATE_PREFIX.len()..]);
        return Some(SigningKey::from_bytes(&raw));
    }
    None
}

pub fn public_key_raw_base64_url(public_key_pem: &str) -> Option<String> {
    extract_public_key_raw(public_key_pem).map(|raw| base64_url_encode(&raw))
}

pub fn sign_payload(private_key_pem: &str, payload: &str) -> Option<String> {
    let key = extract_signing_key(private_key_pem)?;
    let sig = key.sign(payload.as_bytes());
    Some(base64_url_encode(sig.to_bytes().as_ref()))
}

pub fn build_connect_device(
    material: &DeviceSigningMaterial,
    client_id: &str,
    client_mode: &str,
    role: &str,
    scopes: &[String],
    token: Option<&str>,
    nonce: &str,
    platform: Option<&str>,
) -> Option<serde_json::Value> {
    let signed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as u64;
    let payload = build_device_auth_payload_v3(
        &material.device_id,
        client_id,
        client_mode,
        role,
        scopes,
        signed_at_ms,
        token,
        nonce,
        platform,
        None,
    );
    let signature = sign_payload(&material.private_key_pem, &payload)?;
    let public_key = public_key_raw_base64_url(&material.public_key_pem)?;
    Some(serde_json::json!({
        "id": material.device_id,
        "publicKey": public_key,
        "signature": signature,
        "signedAt": signed_at_ms,
        "nonce": nonce,
    }))
}

#[allow(dead_code)]
pub fn fingerprint_pem(public_key_pem: &str) -> Option<String> {
    let raw = extract_public_key_raw(public_key_pem)?;
    Some(hex::encode(Sha256::digest(raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3_payload_format_matches_gateway_client() {
        let payload = build_device_auth_payload_v3(
            "device123",
            "openclaw-linux",
            "node",
            "node",
            &[],
            1_700_000_000_000,
            Some("tok"),
            "nonce-xyz",
            Some("linux"),
            None,
        );
        assert_eq!(
            payload,
            "v3|device123|openclaw-linux|node|node||1700000000000|tok|nonce-xyz|linux|"
        );
    }
}
