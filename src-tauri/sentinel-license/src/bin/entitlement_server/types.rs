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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub database_url: String,
    pub bootstrap_policy_file: Option<String>,
    pub customer_count: i64,
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
