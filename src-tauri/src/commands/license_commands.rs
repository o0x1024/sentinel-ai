//! License management Tauri commands

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

use crate::services::{build_app_entitlements, AppEntitlements};
use sentinel_db::Database;
use sentinel_db::DatabaseService;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementRefreshConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub api_key: String,
    pub customer_id: String,
    pub timeout_secs: u64,
    pub api_key_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementRefreshConfigInput {
    pub enabled: bool,
    pub endpoint: String,
    pub api_key: String,
    pub customer_id: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementRefreshResult {
    pub success: bool,
    pub configured: bool,
    pub message: String,
    pub error_code: Option<String>,
    pub retry_after_secs: Option<u64>,
    pub token_stored: bool,
    pub status: sentinel_license::EntitlementTokenStatus,
}

const LICENSE_CONFIG_CATEGORY: &str = "license";
const ENTITLEMENT_REFRESH_ENABLED_KEY: &str = "entitlement_refresh_enabled";
const ENTITLEMENT_REFRESH_ENDPOINT_KEY: &str = "entitlement_refresh_endpoint";
const ENTITLEMENT_REFRESH_API_KEY_KEY: &str = "entitlement_refresh_api_key";
const ENTITLEMENT_REFRESH_CUSTOMER_ID_KEY: &str = "entitlement_refresh_customer_id";
const ENTITLEMENT_REFRESH_TIMEOUT_SECS_KEY: &str = "entitlement_refresh_timeout_secs";

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

    let _ = sentinel_license::clear_entitlement_token();

    ActivationResult {
        success: true,
        message: "License deactivated".to_string(),
    }
}

/// Get current app entitlements for feature gating.
#[tauri::command]
pub fn get_app_entitlements() -> AppEntitlements {
    build_app_entitlements()
}

/// Store a signed entitlement token for short-lived feature access.
#[tauri::command]
pub fn store_entitlement_token(token: String) -> ActivationResult {
    match sentinel_license::store_entitlement_token(&token) {
        Ok(claims) => ActivationResult {
            success: true,
            message: format!(
                "Entitlement token stored successfully (tier={}, expires_at={})",
                claims.tier, claims.expires_at
            ),
        },
        Err(error) => ActivationResult {
            success: false,
            message: error,
        },
    }
}

/// Clear any locally cached entitlement token.
#[tauri::command]
pub fn clear_entitlement_token() -> ActivationResult {
    match sentinel_license::clear_entitlement_token() {
        Ok(_) => ActivationResult {
            success: true,
            message: "Entitlement token cleared".to_string(),
        },
        Err(error) => ActivationResult {
            success: false,
            message: error,
        },
    }
}

/// Get the current entitlement token validation status.
#[tauri::command]
pub fn get_entitlement_token_status() -> sentinel_license::EntitlementTokenStatus {
    sentinel_license::get_entitlement_token_status()
}

#[tauri::command]
pub async fn get_entitlement_refresh_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<EntitlementRefreshConfig, String> {
    load_entitlement_refresh_config(db.inner().as_ref()).await
}

#[tauri::command]
pub async fn save_entitlement_refresh_config(
    config: EntitlementRefreshConfigInput,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<EntitlementRefreshConfig, String> {
    let timeout_secs = config.timeout_secs.unwrap_or(15).clamp(5, 120);
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        ENTITLEMENT_REFRESH_ENABLED_KEY,
        if config.enabled { "true" } else { "false" },
        Some("Whether automatic entitlement token refresh is enabled"),
    )
    .await
    .map_err(|e| e.to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        ENTITLEMENT_REFRESH_ENDPOINT_KEY,
        config.endpoint.trim(),
        Some("Entitlement token refresh endpoint"),
    )
    .await
    .map_err(|e| e.to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        ENTITLEMENT_REFRESH_API_KEY_KEY,
        config.api_key.trim(),
        Some("Entitlement refresh API key"),
    )
    .await
    .map_err(|e| e.to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        ENTITLEMENT_REFRESH_CUSTOMER_ID_KEY,
        config.customer_id.trim(),
        Some("Entitlement refresh customer identifier"),
    )
    .await
    .map_err(|e| e.to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        ENTITLEMENT_REFRESH_TIMEOUT_SECS_KEY,
        &timeout_secs.to_string(),
        Some("Entitlement refresh request timeout in seconds"),
    )
    .await
    .map_err(|e| e.to_string())?;

    load_entitlement_refresh_config(db.inner().as_ref()).await
}

#[tauri::command]
pub async fn refresh_entitlement_token(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<EntitlementRefreshResult, String> {
    let config = load_entitlement_refresh_config(db.inner().as_ref()).await?;
    if !config.enabled || config.endpoint.trim().is_empty() {
        return Ok(EntitlementRefreshResult {
            success: false,
            configured: false,
            message: "Entitlement refresh service is not configured".to_string(),
            error_code: Some("refresh_not_configured".to_string()),
            retry_after_secs: None,
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs.clamp(5, 120)))
        .build()
        .map_err(|e| format!("Failed to build refresh client: {}", e))?;

    let current_status = sentinel_license::get_entitlement_token_status();
    let request_body = serde_json::json!({
        "machine_id": sentinel_license::get_machine_id(),
        "machine_id_full": sentinel_license::get_machine_id_full(),
        "customer_id": trim_to_option(&config.customer_id),
        "license_present": sentinel_license::get_entitlement_token_status().valid,
        "current_entitlement": {
            "exists": current_status.exists,
            "license_id": current_status.license_id,
            "tier": current_status.tier,
            "feature_ids": current_status.feature_ids,
            "issued_at": current_status.issued_at,
            "expires_at": current_status.expires_at,
            "valid": current_status.valid,
        },
        "client": {
            "product": "sentinel-ai",
            "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS,
        }
    });

    let mut request = client
        .post(config.endpoint.trim())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_body);

    if !config.api_key.trim().is_empty() {
        request = request.bearer_auth(config.api_key.trim());
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(EntitlementRefreshResult {
                success: false,
                configured: true,
                message: format!("Failed to refresh entitlement token: {}", error),
                error_code: Some("network_request_failed".to_string()),
                retry_after_secs: Some(300),
                token_stored: false,
                status: sentinel_license::get_entitlement_token_status(),
            });
        }
    };
    let status_code = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read entitlement refresh response: {}", e))?;

    if !status_code.is_success() {
        let error_payload = extract_refresh_error_payload(&response_text);
        let message = error_payload
            .message
            .unwrap_or_else(|| format!("Refresh endpoint returned HTTP {}", status_code));
        return Ok(EntitlementRefreshResult {
            success: false,
            configured: true,
            message,
            error_code: error_payload
                .code
                .or_else(|| Some(format!("http_{}", status_code.as_u16()))),
            retry_after_secs: error_payload.retry_after_secs,
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        });
    }

    let token = match extract_entitlement_token(&response_text) {
        Some(token) => token,
        None => {
            return Ok(EntitlementRefreshResult {
                success: false,
                configured: true,
                message: "Refresh endpoint did not return a supported token payload".to_string(),
                error_code: Some("invalid_response_payload".to_string()),
                retry_after_secs: Some(300),
                token_stored: false,
                status: sentinel_license::get_entitlement_token_status(),
            });
        }
    };

    match sentinel_license::store_entitlement_token(&token) {
        Ok(claims) => Ok(EntitlementRefreshResult {
            success: true,
            configured: true,
            message: format!(
                "Entitlement token refreshed successfully (tier={}, expires_at={})",
                claims.tier, claims.expires_at
            ),
            error_code: None,
            retry_after_secs: None,
            token_stored: true,
            status: sentinel_license::get_entitlement_token_status(),
        }),
        Err(error) => Ok(EntitlementRefreshResult {
            success: false,
            configured: true,
            message: error,
            error_code: Some("token_store_failed".to_string()),
            retry_after_secs: Some(300),
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        }),
    }
}

async fn load_entitlement_refresh_config(
    db: &DatabaseService,
) -> Result<EntitlementRefreshConfig, String> {
    let enabled = match db
        .get_config(LICENSE_CONFIG_CATEGORY, ENTITLEMENT_REFRESH_ENABLED_KEY)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(value) => parse_bool_like(&value),
        None => std::env::var("SENTINEL_ENTITLEMENT_REFRESH_ENABLED")
            .ok()
            .is_some_and(|value| parse_bool_like(&value)),
    };

    let endpoint = db
        .get_config(LICENSE_CONFIG_CATEGORY, ENTITLEMENT_REFRESH_ENDPOINT_KEY)
        .await
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("SENTINEL_ENTITLEMENT_REFRESH_URL").ok())
        .unwrap_or_default();

    let api_key = db
        .get_config(LICENSE_CONFIG_CATEGORY, ENTITLEMENT_REFRESH_API_KEY_KEY)
        .await
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("SENTINEL_ENTITLEMENT_REFRESH_API_KEY").ok())
        .unwrap_or_default();

    let customer_id = db
        .get_config(LICENSE_CONFIG_CATEGORY, ENTITLEMENT_REFRESH_CUSTOMER_ID_KEY)
        .await
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("SENTINEL_ENTITLEMENT_CUSTOMER_ID").ok())
        .unwrap_or_default();

    let timeout_secs = db
        .get_config(
            LICENSE_CONFIG_CATEGORY,
            ENTITLEMENT_REFRESH_TIMEOUT_SECS_KEY,
        )
        .await
        .map_err(|e| e.to_string())?
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("SENTINEL_ENTITLEMENT_REFRESH_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(15)
        .clamp(5, 120);

    Ok(EntitlementRefreshConfig {
        enabled,
        endpoint,
        api_key_configured: !api_key.trim().is_empty(),
        api_key,
        customer_id,
        timeout_secs,
    })
}

fn parse_bool_like(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn trim_to_option(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn extract_entitlement_token(response_text: &str) -> Option<String> {
    let trimmed = response_text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(token) = value.as_str() {
            return Some(token.to_string());
        }

        for key in ["token", "entitlement_token", "signed_token"] {
            if let Some(token) = value.get(key).and_then(Value::as_str) {
                return Some(token.to_string());
            }
        }
    }

    Some(trimmed.to_string())
}

#[derive(Debug, Default)]
struct RefreshErrorPayload {
    message: Option<String>,
    code: Option<String>,
    retry_after_secs: Option<u64>,
}

fn extract_refresh_error_payload(response_text: &str) -> RefreshErrorPayload {
    let trimmed = response_text.trim();
    if trimmed.is_empty() {
        return RefreshErrorPayload::default();
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let retry_after_secs = value.get("retry_after_secs").and_then(Value::as_u64);
        let code = value
            .get("code")
            .and_then(Value::as_str)
            .map(|value| value.to_string());
        for key in ["message", "error", "detail"] {
            if let Some(message) = value.get(key).and_then(Value::as_str) {
                return RefreshErrorPayload {
                    message: Some(message.to_string()),
                    code,
                    retry_after_secs,
                };
            }
        }

        return RefreshErrorPayload {
            message: None,
            code,
            retry_after_secs,
        };
    }

    RefreshErrorPayload {
        message: Some(trimmed.to_string()),
        code: None,
        retry_after_secs: None,
    }
}
