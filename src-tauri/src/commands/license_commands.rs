//! License management Tauri commands

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub machine_id: String,
    pub is_licensed: bool,
    pub needs_activation: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivationResult {
    pub success: bool,
    pub message: String,
}

/// Get license information
#[tauri::command]
pub fn get_license_info() -> LicenseInfo {
    LicenseInfo {
        machine_id: sentinel_license::get_machine_id(),
        is_licensed: sentinel_license::is_licensed(),
        needs_activation: sentinel_license::needs_activation(),
    }
}

/// Activate license with key
#[tauri::command]
pub fn activate_license(license_key: String) -> ActivationResult {
    use sentinel_license::ValidationResult;
    
    match sentinel_license::activate(&license_key) {
        ValidationResult::Valid => ActivationResult {
            success: true,
            message: "License activated successfully".to_string(),
        },
        ValidationResult::Invalid(reason) => ActivationResult {
            success: false,
            message: reason,
        },
        ValidationResult::NotActivated => ActivationResult {
            success: false,
            message: "Activation failed".to_string(),
        },
    }
}

/// Check if license is valid (quick check for multi-point validation)
#[tauri::command]
pub fn check_license() -> bool {
    sentinel_license::is_licensed()
}

/// Get machine ID for display
#[tauri::command]
pub fn get_machine_id() -> String {
    sentinel_license::get_machine_id()
}

/// Get full machine ID hash (for license generation)
#[tauri::command]
pub fn get_machine_id_full() -> String {
    sentinel_license::get_machine_id_full()
}

/// Deactivate license (remove stored license)
#[tauri::command]
pub fn deactivate_license() -> ActivationResult {
    if let Err(e) = sentinel_license::LicenseStorage::remove() {
        return ActivationResult {
            success: false,
            message: format!("Failed to deactivate: {}", e),
        };
    }
    
    ActivationResult {
        success: true,
        message: "License deactivated".to_string(),
    }
}
