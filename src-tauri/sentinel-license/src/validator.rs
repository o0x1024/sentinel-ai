//! License validation with multiple checks

use crate::machine_id::MachineId;
use crate::crypto::{LicenseKey, verify_license};
use crate::obfuscate;
use std::str::FromStr;

#[cfg(not(debug_assertions))]
use crate::anti_debug;

/// License validation result
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// License is valid
    Valid,
    /// License is invalid with reason
    Invalid(String),
    /// No license found, activation required
    NotActivated,
}

/// License status for UI display
#[derive(Debug, Clone, serde::Serialize)]
pub enum LicenseStatus {
    Licensed,
    NotLicensed,
    Error(String),
}

/// License validator with multiple verification points
pub struct LicenseValidator {
    machine_id: MachineId,
    #[allow(dead_code)]
    check_count: u32,
}

impl LicenseValidator {
    /// Create new validator
    pub fn new() -> Self {
        Self {
            machine_id: MachineId::generate(),
            check_count: 0,
        }
    }
    
    /// Validate license from string
    pub fn validate_str(&self, license_str: &str) -> ValidationResult {
        // Parse license
        let license = match LicenseKey::from_str(license_str) {
            Ok(l) => l,
            Err(_) => {
                return ValidationResult::Invalid(
                    obfuscate::decrypt_str("license_invalid")
                );
            }
        };
        
        self.validate(&license)
    }
    
    /// Validate license key
    pub fn validate(&self, license: &LicenseKey) -> ValidationResult {
        // Check 1: Anti-debug
        #[cfg(not(debug_assertions))]
        if anti_debug::is_debugger_present() {
            return ValidationResult::Invalid(
                obfuscate::decrypt_str("debug_detected")
            );
        }
        
        // Check 2: Machine ID match
        let machine_hash = self.machine_id.to_hash();
        let expected_id = hex::encode(machine_hash);
        
        // Support both full hash and partial hash (display format + zeros)
        let id_match = if license.machine_id == expected_id {
            true
        } else if license.machine_id.len() == 64 {
            // Check if first 16 chars match (display format: 8 bytes = 16 hex chars)
            license.machine_id[..16] == expected_id[..16] && 
            license.machine_id[16..].chars().all(|c| c == '0')
        } else {
            false
        };
        
        if !id_match {
            return ValidationResult::Invalid(
                obfuscate::decrypt_str("machine_mismatch")
            );
        }
        
        // Get the machine_id bytes from license for signature verification
        let license_machine_bytes: [u8; 32] = match hex::decode(&license.machine_id) {
            Ok(bytes) if bytes.len() == 32 => bytes.try_into().unwrap(),
            _ => return ValidationResult::Invalid(obfuscate::decrypt_str("license_invalid")),
        };
        
        // Check 3: Signature verification (use the machine_id from license)
        match verify_license(license, &license_machine_bytes) {
            Ok(true) => {}
            Ok(false) => {
                return ValidationResult::Invalid(
                    obfuscate::decrypt_str("license_invalid")
                );
            }
            Err(e) => {
                tracing::debug!("License verification error: {:?}", e);
                return ValidationResult::Invalid(
                    obfuscate::decrypt_str("license_invalid")
                );
            }
        }
        
        // Check 4: Timing check (detect single-step debugging)
        #[cfg(not(debug_assertions))]
        if anti_debug::timing_check() {
            tracing::warn!("Timing anomaly detected during license validation");
            // Don't immediately fail, just log
        }
        
        ValidationResult::Valid
    }
    
    /// Get machine ID for display
    pub fn get_machine_id(&self) -> String {
        self.machine_id.to_display_string()
    }
    
    /// Get full machine ID hash (for license generation)
    pub fn get_machine_id_hash(&self) -> String {
        self.machine_id.to_full_hex()
    }
    
    /// Perform quick validation (for multi-point checks)
    #[inline]
    pub fn quick_check(&self, license: &LicenseKey) -> bool {
        // Simple machine ID check without full signature verification
        let machine_hash = self.machine_id.to_hash();
        let expected_id = hex::encode(machine_hash);
        
        license.machine_id == expected_id
    }
}

impl Default for LicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick check function for multi-point validation (uses global state)
#[inline]
#[allow(dead_code)]
pub fn quick_license_check() -> bool {
    crate::is_licensed()
}

/// Get feature key for obfuscated validation
#[inline]
#[allow(dead_code)]
pub fn get_feature_key(feature_id: u32) -> u64 {
    let licensed = crate::is_licensed();
    obfuscate::derive_feature_key(licensed, feature_id)
}

/// Verify feature key
#[inline]
#[allow(dead_code)]
pub fn verify_feature_key(key: u64, feature_id: u32) -> bool {
    obfuscate::check_feature_key(key, feature_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validator_creation() {
        let validator = LicenseValidator::new();
        let machine_id = validator.get_machine_id();
        
        assert!(!machine_id.is_empty());
    }
    
    #[test]
    fn test_validation_result() {
        let validator = LicenseValidator::new();
        
        // Invalid license string should fail
        let result = validator.validate_str("invalid_license");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }
}
