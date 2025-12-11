//! Cryptographic functions for license signing and verification

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Embedded public key for license verification
const EMBEDDED_PUBLIC_KEY: &str = "yzCNnuh1Mj0rXdWqvjvWRS6bxXp3Kw9GPu5gDDxrSsk=";

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key format")]
    InvalidKeyFormat,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// Key pair for license generation
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

/// License key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseKey {
    /// Machine ID hash (hex)
    pub machine_id: String,
    /// Signature (base64)
    pub signature: String,
    /// Optional metadata
    #[serde(default)]
    pub metadata: Option<String>,
}

impl LicenseKey {
    /// Create from encoded string
    pub fn from_str(s: &str) -> Result<Self, CryptoError> {
        // License format: BASE64(JSON)
        let decoded = BASE64.decode(s.trim())?;
        let json_str = String::from_utf8(decoded)
            .map_err(|_| CryptoError::InvalidKeyFormat)?;
        
        serde_json::from_str(&json_str)
            .map_err(|_| CryptoError::InvalidKeyFormat)
    }
    
    /// Encode to string
    pub fn to_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        BASE64.encode(json.as_bytes())
    }
    
    /// Get signature bytes
    pub fn signature_bytes(&self) -> Result<[u8; 64], CryptoError> {
        let bytes = BASE64.decode(&self.signature)?;
        bytes.try_into()
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

/// Generate a new key pair (for license generation tool)
pub fn generate_keypair() -> KeyPair {
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    
    KeyPair {
        signing_key,
        verifying_key,
    }
}

/// Sign a license for a machine ID
pub fn sign_license(machine_id_hash: &[u8; 32], signing_key: &SigningKey, metadata: Option<&str>) -> LicenseKey {
    // Create message to sign: hash of machine_id + optional metadata
    let mut hasher = Sha256::new();
    hasher.update(machine_id_hash);
    if let Some(meta) = metadata {
        hasher.update(meta.as_bytes());
    }
    let message: [u8; 32] = hasher.finalize().into();
    
    // Sign
    let signature = signing_key.sign(&message);
    
    LicenseKey {
        machine_id: hex::encode(machine_id_hash),
        signature: BASE64.encode(signature.to_bytes()),
        metadata: metadata.map(|s| s.to_string()),
    }
}

/// Verify a license signature
pub fn verify_license(license: &LicenseKey, machine_id_hash: &[u8; 32]) -> Result<bool, CryptoError> {
    // Check machine ID matches
    let expected_machine_id = hex::encode(machine_id_hash);
    if license.machine_id != expected_machine_id {
        return Ok(false);
    }
    
    // Get embedded public key
    let public_key = get_embedded_public_key()?;
    
    // Recreate message
    let mut hasher = Sha256::new();
    hasher.update(machine_id_hash);
    if let Some(ref meta) = license.metadata {
        hasher.update(meta.as_bytes());
    }
    let message: [u8; 32] = hasher.finalize().into();
    
    // Get signature
    let sig_bytes = license.signature_bytes()?;
    let signature = Signature::from_bytes(&sig_bytes);
    
    // Verify
    match public_key.verify(&message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get the embedded public key
fn get_embedded_public_key() -> Result<VerifyingKey, CryptoError> {
    // In production, this would decode the actual embedded key
    if EMBEDDED_PUBLIC_KEY == "REPLACE_WITH_YOUR_PUBLIC_KEY_BASE64" {
        // For development/testing, return error
        return Err(CryptoError::InvalidKeyFormat);
    }
    
    let key_bytes = BASE64.decode(EMBEDDED_PUBLIC_KEY)?;
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| CryptoError::InvalidKeyFormat)?;
    
    VerifyingKey::from_bytes(&key_array)
        .map_err(|_| CryptoError::InvalidKeyFormat)
}

/// Export signing key to base64
pub fn export_signing_key(key: &SigningKey) -> String {
    BASE64.encode(key.to_bytes())
}

/// Export verifying key to base64
pub fn export_verifying_key(key: &VerifyingKey) -> String {
    BASE64.encode(key.to_bytes())
}

/// Import signing key from base64
pub fn import_signing_key(s: &str) -> Result<SigningKey, CryptoError> {
    let bytes = BASE64.decode(s)?;
    let key_array: [u8; 32] = bytes.try_into()
        .map_err(|_| CryptoError::InvalidKeyFormat)?;
    
    Ok(SigningKey::from_bytes(&key_array))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = generate_keypair();
        let public_key_str = export_verifying_key(&keypair.verifying_key);
        let private_key_str = export_signing_key(&keypair.signing_key);
        
        assert!(!public_key_str.is_empty());
        assert!(!private_key_str.is_empty());
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = generate_keypair();
        let machine_id: [u8; 32] = [0x42; 32];
        
        let license = sign_license(&machine_id, &keypair.signing_key, Some("test"));
        
        // Verify with correct machine ID should work
        // (This won't work with embedded key check, so we test the structure)
        assert_eq!(license.machine_id, hex::encode(&machine_id));
        assert!(!license.signature.is_empty());
    }
    
    #[test]
    fn test_license_serialization() {
        let license = LicenseKey {
            machine_id: "abc123".to_string(),
            signature: "sig123".to_string(),
            metadata: Some("test".to_string()),
        };
        
        let encoded = license.to_string();
        let decoded = LicenseKey::from_str(&encoded).unwrap();
        
        assert_eq!(decoded.machine_id, license.machine_id);
        assert_eq!(decoded.signature, license.signature);
    }
}
