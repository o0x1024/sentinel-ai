//! License storage and persistence

use std::fs;
use std::path::PathBuf;
use crate::crypto::LicenseKey;
use crate::obfuscate;

/// License file name (obfuscated)
const LICENSE_FILE: &str = ".sentinel_auth";

/// License storage manager
pub struct LicenseStorage;

impl LicenseStorage {
    /// Get license file path
    fn get_license_path() -> Option<PathBuf> {
        // Store in app data directory
        let data_dir = dirs::data_local_dir()?;
        let app_dir = data_dir.join("sentinel-ai");
        
        // Create directory if not exists
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).ok()?;
        }
        
        Some(app_dir.join(LICENSE_FILE))
    }
    
    /// Load license from disk
    pub fn load() -> Option<LicenseKey> {
        let path = Self::get_license_path()?;
        
        if !path.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&path).ok()?;
        let decrypted = Self::decrypt_storage(&content)?;
        
        LicenseKey::from_str(&decrypted).ok()
    }
    
    /// Save license to disk
    pub fn save(license_str: &str) -> Result<(), String> {
        let path = Self::get_license_path()
            .ok_or_else(|| "Cannot determine license storage path".to_string())?;
        
        let encrypted = Self::encrypt_storage(license_str);
        
        fs::write(&path, encrypted)
            .map_err(|e| format!("Failed to write license file: {}", e))?;
        
        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&path, permissions);
        }
        
        Ok(())
    }
    
    /// Remove license from disk
    pub fn remove() -> Result<(), String> {
        let path = Self::get_license_path()
            .ok_or_else(|| "Cannot determine license storage path".to_string())?;
        
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove license file: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Check if license file exists
    pub fn exists() -> bool {
        Self::get_license_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
    
    /// Encrypt license for storage (simple XOR + base64)
    fn encrypt_storage(data: &str) -> String {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        
        let key = Self::get_storage_key();
        let encrypted: Vec<u8> = data.as_bytes()
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        
        BASE64.encode(&encrypted)
    }
    
    /// Decrypt stored license
    fn decrypt_storage(data: &str) -> Option<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        
        let encrypted = BASE64.decode(data.trim()).ok()?;
        let key = Self::get_storage_key();
        
        let decrypted: Vec<u8> = encrypted
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        
        String::from_utf8(decrypted).ok()
    }
    
    /// Get storage encryption key (derived from machine ID)
    fn get_storage_key() -> Vec<u8> {
        use sha2::{Sha256, Digest};
        
        let machine_id = crate::MachineId::generate();
        let mut hasher = Sha256::new();
        hasher.update(&machine_id.to_hash());
        hasher.update(b"storage_key_salt_v1");
        
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_encryption() {
        let original = "test_license_data";
        let encrypted = LicenseStorage::encrypt_storage(original);
        let decrypted = LicenseStorage::decrypt_storage(&encrypted).unwrap();
        
        assert_eq!(decrypted, original);
        assert_ne!(encrypted, original);
    }
    
    #[test]
    fn test_license_path() {
        let path = LicenseStorage::get_license_path();
        assert!(path.is_some());
    }
}
