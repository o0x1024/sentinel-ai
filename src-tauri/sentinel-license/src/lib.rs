//! Sentinel License - Enhanced license management with anti-crack features
//!
//! Features:
//! - Ed25519 asymmetric signature verification
//! - Multi-point distributed validation
//! - Anti-debugging detection
//! - String encryption
//! - Validation logic obfuscation

mod machine_id;
mod crypto;
mod anti_debug;
mod obfuscate;
mod validator;
mod storage;
mod integrity;

pub use machine_id::MachineId;
pub use crypto::{LicenseKey, generate_keypair, sign_license, KeyPair};
pub use validator::{LicenseValidator, LicenseStatus, ValidationResult};
pub use storage::LicenseStorage;
pub use anti_debug::is_debugger_present;
pub use integrity::{verify_integrity, is_integrity_ok, function_checksum, verify_function_checksum};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Global license state
static LICENSE_VALID: AtomicBool = AtomicBool::new(false);
static VALIDATION_TOKEN: AtomicU64 = AtomicU64::new(0);

/// Initialize license system (call once at startup)
pub fn initialize() -> ValidationResult {
    // Skip in debug builds
    #[cfg(debug_assertions)]
    {
        LICENSE_VALID.store(true, Ordering::SeqCst);
        VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
        return ValidationResult::Valid;
    }

    #[cfg(not(debug_assertions))]
    {
        // Check 1: Code integrity
        if !integrity::verify_integrity() {
            tracing::warn!("Code integrity check failed");
            return ValidationResult::Invalid(obfuscate::decrypt_str("tampered"));
        }

        // Check 2: Anti-debugging
        if anti_debug::is_debugger_present() {
            tracing::warn!("Debugger detected during license initialization");
            return ValidationResult::Invalid(obfuscate::decrypt_str("debug_detected"));
        }

        // Check 3: Load and validate license
        match LicenseStorage::load() {
            Some(license_key) => {
                let validator = LicenseValidator::new();
                let result = validator.validate(&license_key);
                
                if matches!(result, ValidationResult::Valid) {
                    LICENSE_VALID.store(true, Ordering::SeqCst);
                    VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
                }
                
                result
            }
            None => ValidationResult::NotActivated,
        }
    }
}

/// Quick check if license is valid (for multi-point validation)
#[inline]
pub fn is_licensed() -> bool {
    #[cfg(debug_assertions)]
    return true;

    #[cfg(not(debug_assertions))]
    {
        // Verify token integrity
        let stored_token = VALIDATION_TOKEN.load(Ordering::SeqCst);
        let expected_token = compute_valid_token();
        
        LICENSE_VALID.load(Ordering::SeqCst) && stored_token == expected_token
    }
}

/// Require license for critical operations (returns derived key for obfuscation)
#[inline]
pub fn require_license() -> Option<u64> {
    #[cfg(debug_assertions)]
    return Some(compute_valid_token());

    #[cfg(not(debug_assertions))]
    {
        if is_licensed() {
            Some(VALIDATION_TOKEN.load(Ordering::SeqCst))
        } else {
            None
        }
    }
}

/// Activate license with key
pub fn activate(license_key: &str) -> ValidationResult {
    let validator = LicenseValidator::new();
    let result = validator.validate_str(license_key);
    
    if matches!(result, ValidationResult::Valid) {
        // Save license
        if let Err(e) = LicenseStorage::save(license_key) {
            tracing::error!("Failed to save license: {}", e);
            return ValidationResult::Invalid("Failed to save license".to_string());
        }
        
        LICENSE_VALID.store(true, Ordering::SeqCst);
        VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
    }
    
    result
}

/// Get current machine ID (display format: XXXX-XXXX-XXXX-XXXX)
pub fn get_machine_id() -> String {
    MachineId::generate().to_display_string()
}

/// Get full machine ID hash (64-char hex, for license generation)
pub fn get_machine_id_full() -> String {
    MachineId::generate().to_full_hex()
}

/// Check if application needs activation
pub fn needs_activation() -> bool {
    #[cfg(debug_assertions)]
    return false;

    #[cfg(not(debug_assertions))]
    !is_licensed()
}

/// Compute validation token based on machine characteristics
fn compute_valid_token() -> u64 {
    let machine_id = MachineId::generate();
    let hash = machine_id.to_hash();
    
    // Take first 8 bytes as u64
    let bytes: [u8; 8] = hash[..8].try_into().unwrap_or([0; 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_id_generation() {
        let id1 = get_machine_id();
        let id2 = get_machine_id();
        assert_eq!(id1, id2, "Machine ID should be deterministic");
        assert!(!id1.is_empty(), "Machine ID should not be empty");
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_debug_mode_licensed() {
        // In debug mode, should always be licensed
        assert!(is_licensed());
    }
}
