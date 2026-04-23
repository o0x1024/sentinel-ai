use crate::crypto::{sign_detached_payload, verify_detached_payload, CryptoError};
use crate::machine_id::MachineId;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const ENTITLEMENT_TOKEN_FILE: &str = ".sentinel_entitlement";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementClaims {
    #[serde(default)]
    pub license_id: Option<String>,
    pub machine_id: String,
    pub tier: String,
    #[serde(default)]
    pub feature_ids: Vec<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    #[serde(default)]
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEntitlementToken {
    pub claims: EntitlementClaims,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementTokenStatus {
    pub exists: bool,
    pub valid: bool,
    pub tier: Option<String>,
    pub license_id: Option<String>,
    pub feature_ids: Vec<String>,
    pub issued_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub expires_in_seconds: Option<i64>,
    pub machine_id_match: bool,
    pub error: Option<String>,
}

#[derive(Error, Debug)]
pub enum EntitlementTokenError {
    #[error("Invalid token format")]
    InvalidFormat,
    #[error("Invalid token signature")]
    InvalidSignature,
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Entitlement token machine mismatch")]
    MachineMismatch,
    #[error("Entitlement token expired")]
    Expired,
    #[error("Entitlement token is not yet valid")]
    NotYetValid,
    #[error("{0}")]
    Crypto(#[from] CryptoError),
    #[error("{0}")]
    Serialization(#[from] serde_json::Error),
}

impl SignedEntitlementToken {
    fn signature_bytes(&self) -> Result<[u8; 64], EntitlementTokenError> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let bytes = BASE64.decode(self.signature.trim())?;
        bytes
            .try_into()
            .map_err(|_| EntitlementTokenError::InvalidSignature)
    }
}

impl FromStr for SignedEntitlementToken {
    type Err = EntitlementTokenError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let decoded = BASE64
            .decode(value.trim())
            .map_err(|_| EntitlementTokenError::InvalidFormat)?;
        let json = String::from_utf8(decoded).map_err(|_| EntitlementTokenError::InvalidFormat)?;
        serde_json::from_str(&json).map_err(|_| EntitlementTokenError::InvalidFormat)
    }
}

impl fmt::Display for SignedEntitlementToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", BASE64.encode(json.as_bytes()))
    }
}

impl EntitlementClaims {
    pub fn expires_in_seconds(&self) -> i64 {
        self.expires_at - current_unix_timestamp()
    }

    pub fn has_feature(&self, feature_id: &str) -> bool {
        self.feature_ids.iter().any(|feature| {
            feature == "*" || feature == "all" || feature == "all_features" || feature == feature_id
        })
    }
}

pub fn sign_entitlement_token(
    claims: EntitlementClaims,
    signing_key: &SigningKey,
) -> Result<SignedEntitlementToken, EntitlementTokenError> {
    let payload = canonical_claims_bytes(&claims)?;

    Ok(SignedEntitlementToken {
        claims,
        signature: sign_detached_payload(&payload, signing_key),
    })
}

pub fn validate_entitlement_token(
    token: &SignedEntitlementToken,
) -> Result<EntitlementClaims, EntitlementTokenError> {
    let payload = canonical_claims_bytes(&token.claims)?;
    let signature = token.signature_bytes()?;
    let valid_signature = verify_detached_payload(&payload, &signature)?;
    if !valid_signature {
        return Err(EntitlementTokenError::InvalidSignature);
    }

    let current_machine_id = MachineId::generate().to_full_hex();
    if token.claims.machine_id != current_machine_id {
        return Err(EntitlementTokenError::MachineMismatch);
    }

    let now = current_unix_timestamp();
    if token.claims.issued_at > now + 300 {
        return Err(EntitlementTokenError::NotYetValid);
    }
    if token.claims.expires_at <= now {
        return Err(EntitlementTokenError::Expired);
    }

    Ok(token.claims.clone())
}

pub fn store_entitlement_token(token_str: &str) -> Result<EntitlementClaims, String> {
    let token = SignedEntitlementToken::from_str(token_str).map_err(|e| e.to_string())?;
    let claims = validate_entitlement_token(&token).map_err(|e| e.to_string())?;
    let path = entitlement_token_path()
        .ok_or_else(|| "Cannot determine entitlement token storage path".to_string())?;

    let encrypted = encrypt_storage(token_str);
    fs::write(&path, encrypted)
        .map_err(|e| format!("Failed to write entitlement token file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&path, permissions);
    }

    Ok(claims)
}

pub fn clear_entitlement_token() -> Result<(), String> {
    let path = entitlement_token_path()
        .ok_or_else(|| "Cannot determine entitlement token storage path".to_string())?;

    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove entitlement token file: {}", e))?;
    }

    Ok(())
}

pub fn get_valid_entitlement_claims() -> Option<EntitlementClaims> {
    let token = load_entitlement_token().ok().flatten()?;
    validate_entitlement_token(&token).ok()
}

pub fn get_entitlement_token_status() -> EntitlementTokenStatus {
    let path = match entitlement_token_path() {
        Some(path) => path,
        None => {
            return EntitlementTokenStatus {
                exists: false,
                valid: false,
                tier: None,
                license_id: None,
                feature_ids: Vec::new(),
                issued_at: None,
                expires_at: None,
                expires_in_seconds: None,
                machine_id_match: false,
                error: Some("Cannot determine entitlement token storage path".to_string()),
            };
        }
    };

    if !path.exists() {
        return EntitlementTokenStatus {
            exists: false,
            valid: false,
            tier: None,
            license_id: None,
            feature_ids: Vec::new(),
            issued_at: None,
            expires_at: None,
            expires_in_seconds: None,
            machine_id_match: false,
            error: None,
        };
    }

    match load_entitlement_token() {
        Ok(Some(token)) => {
            let machine_id_match = token.claims.machine_id == MachineId::generate().to_full_hex();
            match validate_entitlement_token(&token) {
                Ok(claims) => EntitlementTokenStatus {
                    exists: true,
                    valid: true,
                    tier: Some(claims.tier.clone()),
                    license_id: claims.license_id.clone(),
                    feature_ids: claims.feature_ids.clone(),
                    issued_at: Some(claims.issued_at),
                    expires_at: Some(claims.expires_at),
                    expires_in_seconds: Some(claims.expires_in_seconds()),
                    machine_id_match,
                    error: None,
                },
                Err(error) => EntitlementTokenStatus {
                    exists: true,
                    valid: false,
                    tier: Some(token.claims.tier.clone()),
                    license_id: token.claims.license_id.clone(),
                    feature_ids: token.claims.feature_ids.clone(),
                    issued_at: Some(token.claims.issued_at),
                    expires_at: Some(token.claims.expires_at),
                    expires_in_seconds: Some(token.claims.expires_in_seconds()),
                    machine_id_match,
                    error: Some(error.to_string()),
                },
            }
        }
        Ok(None) => EntitlementTokenStatus {
            exists: false,
            valid: false,
            tier: None,
            license_id: None,
            feature_ids: Vec::new(),
            issued_at: None,
            expires_at: None,
            expires_in_seconds: None,
            machine_id_match: false,
            error: None,
        },
        Err(error) => EntitlementTokenStatus {
            exists: true,
            valid: false,
            tier: None,
            license_id: None,
            feature_ids: Vec::new(),
            issued_at: None,
            expires_at: None,
            expires_in_seconds: None,
            machine_id_match: false,
            error: Some(error),
        },
    }
}

fn load_entitlement_token() -> Result<Option<SignedEntitlementToken>, String> {
    let path = match entitlement_token_path() {
        Some(path) => path,
        None => return Ok(None),
    };

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read entitlement token file: {}", e))?;
    let decrypted = decrypt_storage(&content)
        .ok_or_else(|| "Failed to decrypt entitlement token".to_string())?;
    let token = SignedEntitlementToken::from_str(&decrypted).map_err(|e| e.to_string())?;

    Ok(Some(token))
}

fn entitlement_token_path() -> Option<PathBuf> {
    let data_dir = dirs::data_local_dir()?;
    let app_dir = data_dir.join("sentinel-ai");

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).ok()?;
    }

    Some(app_dir.join(ENTITLEMENT_TOKEN_FILE))
}

fn canonical_claims_bytes(claims: &EntitlementClaims) -> Result<Vec<u8>, EntitlementTokenError> {
    serde_json::to_vec(claims).map_err(EntitlementTokenError::Serialization)
}

fn encrypt_storage(data: &str) -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};

    let machine_id = MachineId::generate();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.to_hash());
    hasher.update(b"entitlement_token_storage_key_salt_v1");
    let key = hasher.finalize().to_vec();

    let encrypted: Vec<u8> = data
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect();

    BASE64.encode(encrypted)
}

fn decrypt_storage(data: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};

    let encrypted = BASE64.decode(data.trim()).ok()?;

    let machine_id = MachineId::generate();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.to_hash());
    hasher.update(b"entitlement_token_storage_key_salt_v1");
    let key = hasher.finalize().to_vec();

    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect();

    String::from_utf8(decrypted).ok()
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
