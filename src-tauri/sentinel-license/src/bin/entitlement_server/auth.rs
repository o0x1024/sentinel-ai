use crate::config::ServerConfig;
use crate::error::ApiError;
use crate::store::{EntitlementStore, StoredAdminUser, StoredCustomer};
use axum::http::{header::AUTHORIZATION, HeaderMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminPermission {
    Read,
    Write,
    ManageAdmins,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedAdmin {
    pub admin_id: String,
    pub role: String,
}

impl AuthenticatedAdmin {
    pub fn allows(&self, permission: AdminPermission) -> bool {
        match normalize_role(&self.role).as_str() {
            "admin" => true,
            "operator" => matches!(permission, AdminPermission::Read | AdminPermission::Write),
            "viewer" => matches!(permission, AdminPermission::Read),
            _ => false,
        }
    }
}

pub async fn authenticate_admin(
    _config: &ServerConfig,
    headers: &HeaderMap,
    store: &EntitlementStore,
) -> Result<AuthenticatedAdmin, ApiError> {
    let api_key = extract_bearer_token(headers)?;
    let key_hash = EntitlementStore::hash_api_key(api_key);
    let admin = store
        .find_admin_by_api_key_hash(&key_hash)
        .await
        .map_err(|error| ApiError::internal("admin_auth_lookup_failed", error.to_string()))?
        .ok_or_else(|| ApiError::unauthorized("admin_auth_failed", "invalid admin API key"))?;
    if !admin.active {
        return Err(ApiError::forbidden(
            "admin_inactive",
            "admin account is inactive",
        ));
    }

    store
        .touch_admin_last_used(&admin.admin_id)
        .await
        .map_err(|error| ApiError::internal("admin_touch_failed", error.to_string()))?;

    Ok(AuthenticatedAdmin {
        admin_id: admin.admin_id,
        role: normalize_role(&admin.role),
    })
}

pub fn authenticate_refresh_request(
    config: &ServerConfig,
    headers: &HeaderMap,
    customer: &StoredCustomer,
) -> Result<String, ApiError> {
    let api_key = extract_bearer_token(headers)?;
    let api_key_hash = EntitlementStore::hash_api_key(api_key);

    if let Some(customer_refresh_hash) = customer.refresh_api_key_hash.as_deref() {
        if customer_refresh_hash == api_key_hash {
            return Ok(format!("refresh_customer:{}", customer.customer_id));
        }

        return Err(ApiError::unauthorized(
            "refresh_auth_failed",
            "invalid customer refresh API key",
        ));
    }

    if let Some(global_api_key) = config.api_key.as_deref() {
        if EntitlementStore::hash_api_key(global_api_key) == api_key_hash {
            return Ok("refresh_global_api_key".to_string());
        }
    }

    Err(ApiError::unauthorized(
        "refresh_auth_not_configured",
        "refresh API key is not configured for this customer",
    ))
}

pub fn ensure_admin_permission(
    admin: &AuthenticatedAdmin,
    permission: AdminPermission,
) -> Result<(), ApiError> {
    if admin.allows(permission) {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "admin_permission_denied",
        "admin permission denied",
    ))
}

pub fn normalize_role(role: &str) -> String {
    let normalized = role.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "admin" | "operator" | "viewer" => normalized,
        _ => "viewer".to_string(),
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            ApiError::unauthorized("missing_authorization", "missing Authorization header")
        })?;
    authorization
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::unauthorized(
                "invalid_authorization_header",
                "invalid Authorization header",
            )
        })
}

pub fn map_admin_user_response(admin: StoredAdminUser) -> crate::types::AdminUserResponse {
    crate::types::AdminUserResponse {
        admin_id: admin.admin_id,
        role: normalize_role(&admin.role),
        active: admin.active,
        created_at: admin.created_at,
        updated_at: admin.updated_at,
        last_used_at: admin.last_used_at,
    }
}
