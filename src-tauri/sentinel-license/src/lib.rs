//! Sentinel License - Enhanced license management with anti-crack features
//!
//! Features:
//! - Ed25519 asymmetric signature verification
//! - Multi-point distributed validation
//! - Anti-debugging detection
//! - String encryption
//! - Validation logic obfuscation

mod anti_debug;
mod crypto;
mod integrity;
mod machine_id;
mod obfuscate;
mod storage;
mod validator;

pub use anti_debug::is_debugger_present;
pub use crypto::{generate_keypair, sign_license, KeyPair, LicenseKey};
pub use integrity::{
    function_checksum, is_integrity_ok, verify_function_checksum, verify_integrity,
};
pub use machine_id::MachineId;
pub use storage::LicenseStorage;
pub use validator::{LicenseStatus, LicenseValidator, ValidationResult};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Global license state
static LICENSE_VALID: AtomicBool = AtomicBool::new(false);
static VALIDATION_TOKEN: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicensedFeature {
    AiRuntime,
    BugBounty,
    BotConsole,
    PluginCatalogRead,
    PluginCatalogWrite,
    PluginCatalogDelete,
    Rag,
    ToolExecution,
    TrafficAnalysis,
    WorkflowExecution,
}

impl LicensedFeature {
    pub fn display_name(self) -> &'static str {
        match self {
            LicensedFeature::AiRuntime => "AI runtime",
            LicensedFeature::BugBounty => "bug bounty",
            LicensedFeature::BotConsole => "Bot Console",
            LicensedFeature::PluginCatalogRead => "plugin access",
            LicensedFeature::PluginCatalogWrite => "plugin catalog management",
            LicensedFeature::PluginCatalogDelete => "plugin deletion",
            LicensedFeature::Rag => "RAG",
            LicensedFeature::ToolExecution => "tool execution",
            LicensedFeature::TrafficAnalysis => "traffic analysis",
            LicensedFeature::WorkflowExecution => "workflow execution",
        }
    }

    pub fn license_denial_message(self) -> String {
        format!("License required for {}", self.display_name())
    }
}

/// Controls whether release builds require local license activation.
///
/// Driven by the `enforce-license` Cargo feature (off by default).
/// To re-enable: add `enforce-license` to the crate's active features.
/// Debug builds always bypass enforcement regardless of this flag.
pub const LICENSE_ENFORCEMENT_ENABLED: bool = cfg!(feature = "enforce-license");

/// Whether license enforcement is enabled.
#[inline]
pub fn is_enforcement_enabled() -> bool {
    LICENSE_ENFORCEMENT_ENABLED
}

/// Initialize license system (call once at startup)
pub fn initialize() -> ValidationResult {
    if !is_enforcement_enabled() {
        LICENSE_VALID.store(true, Ordering::SeqCst);
        VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
        return ValidationResult::Valid;
    }

    // Skip in debug builds
    #[cfg(debug_assertions)]
    {
        LICENSE_VALID.store(true, Ordering::SeqCst);
        VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
        ValidationResult::Valid
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

        // Check 3: Load and validate local license.
        if has_valid_local_license() {
            LICENSE_VALID.store(true, Ordering::SeqCst);
            VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
            ValidationResult::Valid
        } else {
            LICENSE_VALID.store(false, Ordering::SeqCst);
            VALIDATION_TOKEN.store(0, Ordering::SeqCst);
            ValidationResult::NotActivated
        }
    }
}

/// Quick check if license is valid (for multi-point validation)
#[inline]
pub fn is_licensed() -> bool {
    if !is_enforcement_enabled() {
        return true;
    }

    #[cfg(debug_assertions)]
    return true;

    #[cfg(not(debug_assertions))]
    {
        if has_valid_local_license() {
            LICENSE_VALID.store(true, Ordering::SeqCst);
            VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
        } else {
            LICENSE_VALID.store(false, Ordering::SeqCst);
            VALIDATION_TOKEN.store(0, Ordering::SeqCst);
        }

        // Verify token integrity
        let stored_token = VALIDATION_TOKEN.load(Ordering::SeqCst);
        let expected_token = compute_valid_token();

        LICENSE_VALID.load(Ordering::SeqCst) && stored_token == expected_token
    }
}

#[inline]
pub fn ensure_feature_access(feature: LicensedFeature) -> Result<(), String> {
    if cfg!(debug_assertions) || !is_enforcement_enabled() {
        return Ok(());
    }

    if !is_licensed() {
        return Err(feature.license_denial_message());
    }

    Ok(())
}

#[inline]
pub fn has_feature_access(feature: LicensedFeature) -> bool {
    ensure_feature_access(feature).is_ok()
}

/// Require license for critical operations (returns derived key for obfuscation)
#[inline]
pub fn require_license() -> Option<u64> {
    if !is_enforcement_enabled() {
        return Some(compute_valid_token());
    }

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

/// Activate license with locally signed key bound to this machine.
pub fn activate(license_key: &str) -> ValidationResult {
    if !is_enforcement_enabled() {
        return ValidationResult::Valid;
    }

    if license_key.trim().is_empty() {
        return ValidationResult::Invalid("License key is required".to_string());
    }

    let validator = LicenseValidator::new();
    match validator.validate_str(license_key) {
        ValidationResult::Valid => {
            if let Err(error) = LicenseStorage::save(license_key) {
                return ValidationResult::Invalid(error);
            }

            LICENSE_VALID.store(true, Ordering::SeqCst);
            VALIDATION_TOKEN.store(compute_valid_token(), Ordering::SeqCst);
            ValidationResult::Valid
        }
        other => other,
    }
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
    if !is_enforcement_enabled() {
        return false;
    }

    #[cfg(debug_assertions)]
    return false;

    #[cfg(not(debug_assertions))]
    !is_licensed()
}

/// Whether a valid local license is present for the current machine.
pub fn has_valid_local_license() -> bool {
    let Some(license) = LicenseStorage::load() else {
        return false;
    };

    let validator = LicenseValidator::new();
    matches!(validator.validate(&license), ValidationResult::Valid)
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
