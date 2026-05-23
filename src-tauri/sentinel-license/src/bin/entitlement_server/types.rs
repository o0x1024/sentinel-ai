use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub machine_id: Option<String>,
    pub machine_id_full: Option<String>,
    pub customer_id: Option<String>,
    pub license_present: Option<bool>,
    pub current_entitlement: Option<CurrentEntitlement>,
    pub client: Option<ClientInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CurrentEntitlement {
    pub exists: Option<bool>,
    pub valid: Option<bool>,
    pub license_id: Option<String>,
    pub tier: Option<String>,
    pub feature_ids: Option<Vec<String>>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientInfo {
    pub product: Option<String>,
    pub version: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub entitlement_token: String,
    pub license_id: String,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct LicenseActivateRequest {
    pub username: String,
    pub activation_key: String,
    pub machine_id: Option<String>,
    pub machine_id_full: Option<String>,
    pub client: Option<ClientInfo>,
}

#[derive(Debug, Deserialize)]
pub struct LicenseRefreshRequest {
    pub device_refresh_token: String,
    pub machine_id: Option<String>,
    pub machine_id_full: Option<String>,
    #[serde(rename = "client")]
    pub _client: Option<ClientInfo>,
}

#[derive(Debug, Serialize)]
pub struct LicenseActivateResponse {
    pub entitlement_token: String,
    pub device_refresh_token: String,
    pub license_id: String,
    pub username: String,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub expires_at: i64,
}

pub type LicenseRefreshResponse = LicenseActivateResponse;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub database_url: String,
    pub bootstrap_policy_file: Option<String>,
    pub customer_count: i64,
    pub license_card_count: i64,
    pub device_binding_count: i64,
    pub audit_event_count: i64,
    pub admin_user_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpsertCustomerRequest {
    pub license_id: String,
    pub tier: Option<String>,
    pub feature_ids: Option<Vec<String>>,
    pub device_limit: Option<usize>,
    pub revoked: Option<bool>,
    pub allowed_machine_ids: Option<Vec<String>>,
    pub ttl_seconds: Option<i64>,
    pub refresh_api_key: Option<String>,
    pub clear_refresh_api_key: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertAdminUserRequest {
    pub api_key: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub customer_id: String,
    pub license_id: String,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub device_limit: usize,
    pub revoked: bool,
    pub allowed_machine_ids: Vec<String>,
    pub ttl_seconds: Option<i64>,
    pub refresh_api_key_configured: bool,
    pub refresh_api_key_last_used_at: Option<i64>,
    pub last_entitlement_issued_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CustomerListResponse {
    pub customers: Vec<CustomerResponse>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCardBatchRequest {
    pub name: Option<String>,
    pub count: usize,
    pub tier: Option<String>,
    pub feature_ids: Option<Vec<String>>,
    pub device_limit: Option<usize>,
    pub ttl_seconds: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreatedLicenseCardResponse {
    pub id: i64,
    pub username: String,
    pub activation_key: String,
    pub tier: String,
    pub device_limit: usize,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateCardBatchResponse {
    pub batch_id: i64,
    pub cards: Vec<CreatedLicenseCardResponse>,
}

#[derive(Debug, Serialize)]
pub struct LicenseCardResponse {
    pub id: i64,
    pub batch_id: Option<i64>,
    pub username: Option<String>,
    pub activation_key: Option<String>,
    pub tier: String,
    pub feature_ids: Vec<String>,
    pub status: String,
    pub device_limit: usize,
    pub device_count: i64,
    pub expires_at: Option<i64>,
    pub first_activated_at: Option<i64>,
    pub last_activated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct LicenseCardListResponse {
    pub cards: Vec<LicenseCardResponse>,
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteLicenseCardsRequest {
    pub card_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteLicenseCardsResponse {
    pub deleted_ids: Vec<i64>,
    pub cards: Vec<LicenseCardResponse>,
}

#[derive(Debug, Serialize)]
pub struct LicenseCardDetailResponse {
    pub card: LicenseCardResponse,
    pub devices: Vec<LicenseDeviceResponse>,
}

#[derive(Debug, Serialize)]
pub struct LicenseDeviceResponse {
    pub id: i64,
    pub card_id: i64,
    pub username: String,
    pub machine_id: String,
    pub device_name: Option<String>,
    pub revoked: bool,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Serialize)]
pub struct LicenseDeviceListResponse {
    pub card_id: i64,
    pub devices: Vec<LicenseDeviceResponse>,
}

#[derive(Debug, Serialize)]
pub struct DeviceBindingResponse {
    pub customer_id: String,
    pub machine_id: String,
    pub bound_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Serialize)]
pub struct DeviceBindingListResponse {
    pub customer_id: String,
    pub devices: Vec<DeviceBindingResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub admin_id: String,
    pub role: String,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserListResponse {
    pub users: Vec<AdminUserResponse>,
}

#[derive(Debug, Deserialize)]
pub struct AuditEventListQuery {
    pub customer_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AuditEventResponse {
    pub id: i64,
    pub event_type: String,
    pub actor: String,
    pub customer_id: Option<String>,
    pub machine_id: Option<String>,
    pub details: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditEventListResponse {
    pub events: Vec<AuditEventResponse>,
}

#[derive(Debug, Deserialize)]
pub struct KeyPairStore {
    #[serde(rename = "public_key")]
    pub _public_key: String,
    pub private_key: String,
}
