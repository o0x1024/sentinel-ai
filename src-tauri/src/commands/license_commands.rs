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
    pub trial_active: bool,
    pub trial_started_at: Option<i64>,
    pub trial_expires_at: Option<i64>,
    pub trial_remaining_seconds: Option<i64>,
    pub trial_days_remaining: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseCardActivationInput {
    pub username: String,
    pub activation_key: String,
}

const LICENSE_CONFIG_CATEGORY: &str = "license";
const ENTITLEMENT_REFRESH_ENABLED_KEY: &str = "entitlement_refresh_enabled";
const ENTITLEMENT_REFRESH_ENDPOINT_KEY: &str = "entitlement_refresh_endpoint";
const ENTITLEMENT_REFRESH_API_KEY_KEY: &str = "entitlement_refresh_api_key";
const ENTITLEMENT_REFRESH_CUSTOMER_ID_KEY: &str = "entitlement_refresh_customer_id";
const ENTITLEMENT_REFRESH_TIMEOUT_SECS_KEY: &str = "entitlement_refresh_timeout_secs";
const LICENSE_CARD_USERNAME_KEY: &str = "license_card_username";
const LICENSE_CARD_DEVICE_REFRESH_TOKEN_KEY: &str = "license_card_device_refresh_token";
const DEFAULT_LICENSE_SERVER_BASE_URL: &str = "http://119.29.111.31:24001";

/// Get license information
#[tauri::command]
pub fn get_license_info() -> LicenseInfo {
    let trial_status = sentinel_license::get_or_create_trial_status();

    LicenseInfo {
        machine_id: sentinel_license::get_machine_id(),
        is_licensed: sentinel_license::is_licensed(),
        needs_activation: sentinel_license::needs_activation(),
        trial_active: trial_status.active,
        trial_started_at: trial_status.started_at,
        trial_expires_at: trial_status.expires_at,
        trial_remaining_seconds: trial_status.remaining_seconds,
        trial_days_remaining: trial_status.days_remaining,
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
pub async fn activate_with_license_card(
    db: State<'_, Arc<DatabaseService>>,
    input: LicenseCardActivationInput,
) -> Result<EntitlementRefreshResult, String> {
    let username = input.username.trim();
    let activation_key = input.activation_key.trim();
    if username.is_empty() || activation_key.is_empty() {
        return Ok(EntitlementRefreshResult {
            success: false,
            configured: true,
            message: "用户名和激活密钥不能为空".to_string(),
            error_code: Some("missing_activation_fields".to_string()),
            retry_after_secs: None,
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        });
    }

    let endpoint = format!("{}/api/licenses/activate", license_server_base_url());
    let request_body = serde_json::json!({
        "username": username,
        "activation_key": activation_key,
        "machine_id": sentinel_license::get_machine_id(),
        "machine_id_full": sentinel_license::get_machine_id_full(),
        "client": {
            "product": "sentinel-ai",
            "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS,
        }
    });

    let response = post_license_server_json(&endpoint, request_body).await?;
    if !response.status_code.is_success() {
        let error_payload = extract_refresh_error_payload(&response.text);
        return Ok(EntitlementRefreshResult {
            success: false,
            configured: true,
            message: error_payload.message.unwrap_or_else(|| {
                format!("Activation endpoint returned HTTP {}", response.status_code)
            }),
            error_code: error_payload
                .code
                .or_else(|| Some(format!("http_{}", response.status_code.as_u16()))),
            retry_after_secs: error_payload.retry_after_secs,
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        });
    }

    let token = extract_entitlement_token(&response.text).ok_or_else(|| {
        "Activation endpoint did not return a supported token payload".to_string()
    })?;
    let device_refresh_token = extract_string_field(&response.text, "device_refresh_token")
        .ok_or_else(|| "Activation endpoint did not return device_refresh_token".to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        LICENSE_CARD_USERNAME_KEY,
        username,
        Some("License card username for automatic activation refresh"),
    )
    .await
    .map_err(|e| e.to_string())?;
    db.set_config(
        LICENSE_CONFIG_CATEGORY,
        LICENSE_CARD_DEVICE_REFRESH_TOKEN_KEY,
        &device_refresh_token,
        Some("Device refresh token for license card activation"),
    )
    .await
    .map_err(|e| e.to_string())?;

    store_activation_token(token).await
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
    if let Some(result) = refresh_license_card_token(db.inner().as_ref()).await? {
        return Ok(result);
    }

    Ok(EntitlementRefreshResult {
        success: false,
        configured: false,
        message: "当前设备尚未完成卡密激活".to_string(),
        error_code: Some("license_card_not_activated".to_string()),
        retry_after_secs: None,
        token_stored: false,
        status: sentinel_license::get_entitlement_token_status(),
    })
}

struct LicenseServerResponse {
    status_code: reqwest::StatusCode,
    text: String,
}

async fn post_license_server_json(
    endpoint: &str,
    body: Value,
) -> Result<LicenseServerResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build license client: {}", e))?;
    let response = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("Failed to contact license server: {}", error))?;
    let status_code = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read license server response: {}", e))?;
    Ok(LicenseServerResponse { status_code, text })
}

async fn refresh_license_card_token(
    db: &DatabaseService,
) -> Result<Option<EntitlementRefreshResult>, String> {
    let device_refresh_token = db
        .get_config(
            LICENSE_CONFIG_CATEGORY,
            LICENSE_CARD_DEVICE_REFRESH_TOKEN_KEY,
        )
        .await
        .map_err(|e| e.to_string())?
        .filter(|value| !value.trim().is_empty());
    let Some(device_refresh_token) = device_refresh_token else {
        return Ok(None);
    };

    let endpoint = format!("{}/api/licenses/refresh", license_server_base_url());
    let request_body = serde_json::json!({
        "device_refresh_token": device_refresh_token,
        "machine_id": sentinel_license::get_machine_id(),
        "machine_id_full": sentinel_license::get_machine_id_full(),
        "client": {
            "product": "sentinel-ai",
            "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS,
        }
    });
    let response = post_license_server_json(&endpoint, request_body).await?;
    if !response.status_code.is_success() {
        let error_payload = extract_refresh_error_payload(&response.text);
        return Ok(Some(EntitlementRefreshResult {
            success: false,
            configured: true,
            message: error_payload.message.unwrap_or_else(|| {
                format!("Refresh endpoint returned HTTP {}", response.status_code)
            }),
            error_code: error_payload
                .code
                .or_else(|| Some(format!("http_{}", response.status_code.as_u16()))),
            retry_after_secs: error_payload.retry_after_secs,
            token_stored: false,
            status: sentinel_license::get_entitlement_token_status(),
        }));
    }

    let token = extract_entitlement_token(&response.text)
        .ok_or_else(|| "Refresh endpoint did not return a supported token payload".to_string())?;
    store_activation_token(token).await.map(Some)
}

async fn store_activation_token(token: String) -> Result<EntitlementRefreshResult, String> {
    match sentinel_license::store_entitlement_token(&token) {
        Ok(claims) => Ok(EntitlementRefreshResult {
            success: true,
            configured: true,
            message: format!(
                "授权激活成功 (tier={}, expires_at={})",
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

fn license_server_base_url() -> String {
    std::env::var("SENTINEL_LICENSE_SERVER_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_LICENSE_SERVER_BASE_URL.to_string())
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

fn extract_string_field(response_text: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(response_text.trim())
        .ok()
        .and_then(|value| value.get(field).and_then(Value::as_str).map(str::to_string))
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
