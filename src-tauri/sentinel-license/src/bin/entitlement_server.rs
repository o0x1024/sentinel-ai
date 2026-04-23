#[path = "entitlement_server/admin_ui.rs"]
mod admin_ui;
#[path = "entitlement_server/auth.rs"]
mod auth;
#[path = "entitlement_server/config.rs"]
mod config;
#[path = "entitlement_server/error.rs"]
mod error;
#[path = "entitlement_server/rate_limit.rs"]
mod rate_limit;
#[path = "entitlement_server/store.rs"]
mod store;
#[path = "entitlement_server/types.rs"]
mod types;

use anyhow::{anyhow, Context, Result};
use auth::{
    authenticate_admin, authenticate_refresh_request, ensure_admin_permission,
    map_admin_user_response, AdminPermission, AuthenticatedAdmin,
};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use config::{clamp_ttl, ServerConfig};
use ed25519_dalek::SigningKey;
use error::ApiError;
use rand::RngCore;
use rate_limit::RateLimiter;
use sentinel_license::{sign_entitlement_token, EntitlementClaims};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use store::{
    CustomerRefreshApiKeyUpdate, EntitlementStore, NewAuditEvent, StoredAuditEvent, StoredCustomer,
};
use types::{
    AdminUserListResponse, AdminUserResponse, AuditEventListQuery, AuditEventListResponse,
    AuditEventResponse, CustomerListResponse, CustomerResponse, DeviceBindingListResponse,
    DeviceBindingResponse, HealthResponse, KeyPairStore, RefreshRequest, RefreshResponse,
    UpsertAdminUserRequest, UpsertCustomerRequest,
};

#[derive(Clone)]
struct AppState {
    config: ServerConfig,
    signing_key: Arc<SigningKey>,
    store: EntitlementStore,
    rate_limiter: RateLimiter,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "entitlement_server=info,tower_http=info".to_string()),
        )
        .init();

    let config = ServerConfig::from_env()?;
    let signing_key = Arc::new(load_signing_key()?);
    let store = EntitlementStore::new(&config.database_url).await?;

    if let Some(policy_file) = &config.bootstrap_policy_file {
        store.bootstrap_from_policy_file(policy_file).await?;
    }
    if let (Some(admin_id), Some(admin_api_key)) = (
        config.bootstrap_admin_id.as_deref(),
        config.bootstrap_admin_api_key.as_deref(),
    ) {
        store
            .ensure_bootstrap_admin(admin_id, &config.bootstrap_admin_role, admin_api_key)
            .await?;
    }

    let app_state = AppState {
        config: config.clone(),
        signing_key,
        store,
        rate_limiter: RateLimiter::default(),
    };

    let app = Router::new()
        .route("/", get(admin_ui::admin_page))
        .route("/admin", get(admin_ui::admin_page))
        .route("/healthz", get(healthz))
        .route("/api/entitlements/refresh", post(refresh_entitlement))
        .route("/api/admin/users", get(list_admin_users))
        .route("/api/admin/users/:admin_id", put(upsert_admin_user))
        .route("/api/admin/customers", get(list_customers))
        .route(
            "/api/admin/customers/:customer_id",
            get(get_customer).put(upsert_customer),
        )
        .route("/api/admin/audit", get(list_audit_events))
        .route(
            "/api/admin/customers/:customer_id/devices",
            get(list_customer_devices),
        )
        .route(
            "/api/admin/customers/:customer_id/devices/:machine_id",
            delete(delete_customer_device),
        )
        .with_state(app_state);

    tracing::info!(
        "entitlement server listening on {} using database {}",
        config.bind_addr,
        config.database_url
    );

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .context("failed to bind entitlement server listener")?;
    axum::serve(listener, app)
        .await
        .context("entitlement server exited unexpectedly")?;
    Ok(())
}

async fn healthz(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let customer_count = state
        .store
        .customer_count()
        .await
        .map_err(|error| ApiError::internal("customer_count_failed", error.to_string()))?;
    let device_binding_count = state
        .store
        .device_binding_count()
        .await
        .map_err(|error| ApiError::internal("device_count_failed", error.to_string()))?;
    let audit_event_count = state
        .store
        .audit_event_count()
        .await
        .map_err(|error| ApiError::internal("audit_count_failed", error.to_string()))?;
    let admin_user_count = state
        .store
        .admin_user_count()
        .await
        .map_err(|error| ApiError::internal("admin_count_failed", error.to_string()))?;

    Ok(Json(HealthResponse {
        ok: true,
        database_url: state.config.database_url.clone(),
        bootstrap_policy_file: state
            .config
            .bootstrap_policy_file
            .as_ref()
            .map(|path| path.display().to_string()),
        customer_count,
        device_binding_count,
        audit_event_count,
        admin_user_count,
    }))
}

async fn refresh_entitlement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let customer_id = request
        .customer_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request("missing_customer_id", "customer_id is required"))?;
    let machine_id_full = normalize_machine_id(
        request
            .machine_id_full
            .as_deref()
            .or(request.machine_id.as_deref()),
    )?;

    if !request.license_present.unwrap_or(false) {
        return Err(ApiError::forbidden(
            "local_license_required",
            "local license is required before entitlement refresh",
        ));
    }

    let customer = state
        .store
        .get_customer(customer_id)
        .await
        .map_err(|error| ApiError::internal("customer_lookup_failed", error.to_string()))?
        .ok_or_else(|| ApiError::forbidden("customer_not_entitled", "customer is not entitled"))?;
    let refresh_actor = authenticate_refresh_request(&state.config, &headers, &customer)?;
    state.rate_limiter.check(
        format!("refresh:{}", customer.customer_id),
        state.config.refresh_rate_limit_per_minute,
    )?;
    state
        .store
        .touch_customer_refresh_api_key_last_used(customer_id)
        .await
        .map_err(|error| {
            ApiError::internal("customer_refresh_last_used_touch_failed", error.to_string())
        })?;

    validate_customer_access(&customer, &machine_id_full)?;

    state
        .store
        .ensure_machine_binding(customer_id, &machine_id_full, customer.device_limit)
        .await
        .map_err(|error| ApiError::forbidden("device_limit_exceeded", error.to_string()))?;

    log_refresh_context(customer_id, &machine_id_full, &request);

    let resolved_ttl = clamp_ttl(customer.ttl_seconds.unwrap_or(state.config.token_ttl_secs));
    let issued_at = current_unix_timestamp();
    let expires_at = issued_at + resolved_ttl;
    let claims = EntitlementClaims {
        license_id: Some(customer.license_id.clone()),
        machine_id: machine_id_full.clone(),
        tier: customer.tier.clone(),
        feature_ids: customer.feature_ids.clone(),
        issued_at,
        expires_at,
        nonce: Some(generate_nonce()),
    };
    let signed = sign_entitlement_token(claims, &state.signing_key).map_err(|error| {
        ApiError::internal(
            "token_sign_failed",
            format!("failed to sign token: {}", error),
        )
    })?;
    state
        .store
        .touch_customer_entitlement_issued(customer_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "customer_entitlement_issued_touch_failed",
                error.to_string(),
            )
        })?;

    state
        .store
        .insert_audit_event(NewAuditEvent {
            event_type: "entitlement_issued".to_string(),
            actor: refresh_actor,
            customer_id: Some(customer_id.to_string()),
            machine_id: Some(machine_id_full.clone()),
            details: Some("issued entitlement token".to_string()),
            metadata: Some(json!({
                "license_id": customer.license_id,
                "tier": customer.tier,
                "feature_ids": customer.feature_ids,
                "expires_at": expires_at,
            })),
        })
        .await
        .map_err(|error| ApiError::internal("audit_insert_failed", error.to_string()))?;

    Ok(Json(RefreshResponse {
        entitlement_token: signed.to_string(),
        license_id: customer.license_id,
        tier: customer.tier,
        feature_ids: customer.feature_ids,
        expires_at,
    }))
}

async fn list_customers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CustomerListResponse>, ApiError> {
    let _admin = authorize_admin_read(&state, &headers).await?;

    let customers = state
        .store
        .list_customers()
        .await
        .map_err(|error| ApiError::internal("customer_list_failed", error.to_string()))?
        .into_iter()
        .map(map_customer_response)
        .collect();

    Ok(Json(CustomerListResponse { customers }))
}

async fn get_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(customer_id): Path<String>,
) -> Result<Json<CustomerResponse>, ApiError> {
    let _admin = authorize_admin_read(&state, &headers).await?;
    let normalized_customer_id = normalize_customer_id(&customer_id)?;
    let customer = state
        .store
        .get_customer(&normalized_customer_id)
        .await
        .map_err(|error| ApiError::internal("customer_lookup_failed", error.to_string()))?
        .ok_or_else(|| ApiError::not_found("customer_not_found", "customer does not exist"))?;

    Ok(Json(map_customer_response(customer)))
}

async fn list_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditEventListQuery>,
) -> Result<Json<AuditEventListResponse>, ApiError> {
    let _admin = authorize_admin_read(&state, &headers).await?;

    let customer_id = query
        .customer_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let event_type = query
        .event_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let events = state
        .store
        .list_audit_events(customer_id, event_type, limit, offset)
        .await
        .map_err(|error| ApiError::internal("audit_list_failed", error.to_string()))?
        .into_iter()
        .map(map_audit_event_response)
        .collect();

    Ok(Json(AuditEventListResponse { events }))
}

async fn upsert_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(customer_id): Path<String>,
    Json(request): Json<UpsertCustomerRequest>,
) -> Result<Json<CustomerResponse>, ApiError> {
    let admin = authorize_admin_write(&state, &headers).await?;

    let normalized_customer_id = normalize_customer_id(&customer_id)?;
    let stored_customer = StoredCustomer {
        customer_id: normalized_customer_id.clone(),
        license_id: trim_required(
            &request.license_id,
            "missing_license_id",
            "license_id is required",
        )?
        .to_string(),
        tier: request
            .tier
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("pro")
            .to_string(),
        feature_ids: request
            .feature_ids
            .unwrap_or_else(default_admin_feature_ids),
        device_limit: request.device_limit.unwrap_or(3).clamp(1, 64),
        revoked: request.revoked.unwrap_or(false),
        allowed_machine_ids: normalize_machine_ids(
            request.allowed_machine_ids.unwrap_or_default(),
        )?,
        ttl_seconds: request.ttl_seconds.map(clamp_ttl),
        refresh_api_key_hash: request
            .refresh_api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(EntitlementStore::hash_api_key),
        refresh_api_key_last_used_at: None,
        last_entitlement_issued_at: None,
    };
    let refresh_api_key_changed = request
        .refresh_api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let clear_refresh_api_key = request.clear_refresh_api_key.unwrap_or(false);
    let refresh_api_key_update = if clear_refresh_api_key {
        CustomerRefreshApiKeyUpdate::Clear
    } else if let Some(hash) = stored_customer.refresh_api_key_hash.clone() {
        CustomerRefreshApiKeyUpdate::Set(hash)
    } else {
        CustomerRefreshApiKeyUpdate::Preserve
    };

    state
        .store
        .upsert_customer(&stored_customer)
        .await
        .map_err(|error| ApiError::internal("customer_upsert_failed", error.to_string()))?;
    state
        .store
        .upsert_customer_refresh_api_key(&normalized_customer_id, refresh_api_key_update)
        .await
        .map_err(|error| {
            ApiError::internal("customer_refresh_key_upsert_failed", error.to_string())
        })?;
    let stored_customer = state
        .store
        .get_customer(&normalized_customer_id)
        .await
        .map_err(|error| ApiError::internal("customer_lookup_failed", error.to_string()))?
        .ok_or_else(|| {
            ApiError::internal(
                "customer_missing_after_upsert",
                "customer missing after upsert",
            )
        })?;

    state
        .store
        .insert_audit_event(NewAuditEvent {
            event_type: "customer_upserted".to_string(),
            actor: admin.admin_id,
            customer_id: Some(normalized_customer_id),
            machine_id: None,
            details: Some("created or updated entitlement customer".to_string()),
            metadata: Some(json!({
                "license_id": stored_customer.license_id,
                "tier": stored_customer.tier,
                "feature_ids": stored_customer.feature_ids,
                "device_limit": stored_customer.device_limit,
                "revoked": stored_customer.revoked,
                "allowed_machine_ids": stored_customer.allowed_machine_ids,
                "ttl_seconds": stored_customer.ttl_seconds,
                "refresh_api_key_configured": stored_customer.refresh_api_key_hash.is_some(),
                "refresh_api_key_changed": refresh_api_key_changed,
                "refresh_api_key_cleared": clear_refresh_api_key,
            })),
        })
        .await
        .map_err(|error| ApiError::internal("audit_insert_failed", error.to_string()))?;

    Ok(Json(map_customer_response(stored_customer)))
}

async fn list_customer_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(customer_id): Path<String>,
) -> Result<Json<DeviceBindingListResponse>, ApiError> {
    let _admin = authorize_admin_read(&state, &headers).await?;

    let normalized_customer_id = normalize_customer_id(&customer_id)?;
    ensure_customer_exists(&state.store, &normalized_customer_id).await?;

    let devices = state
        .store
        .list_device_bindings(&normalized_customer_id)
        .await
        .map_err(|error| ApiError::internal("device_list_failed", error.to_string()))?
        .into_iter()
        .map(|device| DeviceBindingResponse {
            customer_id: device.customer_id,
            machine_id: device.machine_id,
            bound_at: device.bound_at,
            last_seen_at: device.last_seen_at,
        })
        .collect();

    Ok(Json(DeviceBindingListResponse {
        customer_id: normalized_customer_id,
        devices,
    }))
}

async fn delete_customer_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((customer_id, machine_id)): Path<(String, String)>,
) -> Result<Json<DeviceBindingListResponse>, ApiError> {
    let admin = authorize_admin_write(&state, &headers).await?;

    let normalized_customer_id = normalize_customer_id(&customer_id)?;
    let normalized_machine_id = normalize_machine_id(Some(machine_id.as_str()))?;
    ensure_customer_exists(&state.store, &normalized_customer_id).await?;

    let deleted = state
        .store
        .delete_device_binding(&normalized_customer_id, &normalized_machine_id)
        .await
        .map_err(|error| ApiError::internal("device_delete_failed", error.to_string()))?;
    if !deleted {
        return Err(ApiError::not_found(
            "device_binding_not_found",
            "device binding does not exist",
        ));
    }

    state
        .store
        .insert_audit_event(NewAuditEvent {
            event_type: "device_binding_deleted".to_string(),
            actor: admin.admin_id,
            customer_id: Some(normalized_customer_id.clone()),
            machine_id: Some(normalized_machine_id),
            details: Some("deleted entitlement device binding".to_string()),
            metadata: None,
        })
        .await
        .map_err(|error| ApiError::internal("audit_insert_failed", error.to_string()))?;

    let devices = state
        .store
        .list_device_bindings(&normalized_customer_id)
        .await
        .map_err(|error| ApiError::internal("device_list_failed", error.to_string()))?
        .into_iter()
        .map(|device| DeviceBindingResponse {
            customer_id: device.customer_id,
            machine_id: device.machine_id,
            bound_at: device.bound_at,
            last_seen_at: device.last_seen_at,
        })
        .collect();

    Ok(Json(DeviceBindingListResponse {
        customer_id: normalized_customer_id,
        devices,
    }))
}

async fn list_admin_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminUserListResponse>, ApiError> {
    let _admin = authorize_admin_manage_admins(&state, &headers).await?;

    let users = state
        .store
        .list_admin_users()
        .await
        .map_err(|error| ApiError::internal("admin_user_list_failed", error.to_string()))?
        .into_iter()
        .map(map_admin_user_response)
        .collect();

    Ok(Json(AdminUserListResponse { users }))
}

async fn upsert_admin_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(admin_id): Path<String>,
    Json(request): Json<UpsertAdminUserRequest>,
) -> Result<Json<AdminUserResponse>, ApiError> {
    let current_admin = authorize_admin_manage_admins(&state, &headers).await?;
    let normalized_admin_id = normalize_customer_id(&admin_id)?;
    let existing = state
        .store
        .get_admin_user(&normalized_admin_id)
        .await
        .map_err(|error| ApiError::internal("admin_user_lookup_failed", error.to_string()))?;
    let role = request
        .role
        .as_deref()
        .map(auth::normalize_role)
        .unwrap_or_else(|| {
            existing
                .as_ref()
                .map(|item| auth::normalize_role(&item.role))
                .unwrap_or_else(|| "viewer".to_string())
        });
    let active = request
        .active
        .unwrap_or_else(|| existing.as_ref().map(|item| item.active).unwrap_or(true));
    let api_key_hash = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(EntitlementStore::hash_api_key);
    let existing_was_active_admin = existing
        .as_ref()
        .map(|item| item.active && auth::normalize_role(&item.role) == "admin")
        .unwrap_or(false);
    let next_is_active_admin = active && role == "admin";

    if current_admin.admin_id == normalized_admin_id && !active {
        return Err(ApiError::forbidden(
            "admin_self_deactivate_forbidden",
            "cannot deactivate the current authenticated admin",
        ));
    }

    let current_active_admin_count = state
        .store
        .active_admin_count()
        .await
        .map_err(|error| ApiError::internal("admin_user_count_failed", error.to_string()))?;
    let projected_active_admin_count = current_active_admin_count
        - if existing_was_active_admin {
            1_i64
        } else {
            0_i64
        }
        + if next_is_active_admin { 1_i64 } else { 0_i64 };
    if projected_active_admin_count < 1 {
        let (error_code, message) = if existing_was_active_admin && !active {
            (
                "last_active_admin_forbidden",
                "cannot deactivate the last active admin",
            )
        } else {
            (
                "last_admin_role_forbidden",
                "cannot remove the last active admin role",
            )
        };
        return Err(ApiError::forbidden(error_code, message));
    }

    state
        .store
        .upsert_admin_user(&normalized_admin_id, &role, active, api_key_hash.as_deref())
        .await
        .map_err(|error| ApiError::internal("admin_user_upsert_failed", error.to_string()))?;

    let stored = state
        .store
        .get_admin_user(&normalized_admin_id)
        .await
        .map_err(|error| ApiError::internal("admin_user_lookup_failed", error.to_string()))?
        .ok_or_else(|| {
            ApiError::internal(
                "admin_user_missing_after_upsert",
                "admin user missing after upsert",
            )
        })?;

    state
        .store
        .insert_audit_event(NewAuditEvent {
            event_type: "admin_user_upserted".to_string(),
            actor: current_admin.admin_id,
            customer_id: None,
            machine_id: None,
            details: Some("created or updated admin user".to_string()),
            metadata: Some(json!({
                "admin_id": stored.admin_id,
                "role": stored.role,
                "active": stored.active,
                "api_key_updated": api_key_hash.is_some(),
            })),
        })
        .await
        .map_err(|error| ApiError::internal("audit_insert_failed", error.to_string()))?;

    Ok(Json(map_admin_user_response(stored)))
}

async fn authorize_admin_read(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedAdmin, ApiError> {
    let admin = authenticate_admin(&state.config, headers, &state.store).await?;
    ensure_admin_permission(&admin, AdminPermission::Read)?;
    state.rate_limiter.check(
        format!("{}:admin_read", admin.admin_id),
        state.config.admin_read_rate_limit_per_minute,
    )?;
    Ok(admin)
}

async fn authorize_admin_write(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedAdmin, ApiError> {
    let admin = authenticate_admin(&state.config, headers, &state.store).await?;
    ensure_admin_permission(&admin, AdminPermission::Write)?;
    state.rate_limiter.check(
        format!("{}:admin_write", admin.admin_id),
        state.config.admin_write_rate_limit_per_minute,
    )?;
    Ok(admin)
}

async fn authorize_admin_manage_admins(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedAdmin, ApiError> {
    let admin = authenticate_admin(&state.config, headers, &state.store).await?;
    ensure_admin_permission(&admin, AdminPermission::ManageAdmins)?;
    state.rate_limiter.check(
        format!("{}:admin_manage_admins", admin.admin_id),
        state.config.admin_write_rate_limit_per_minute,
    )?;
    Ok(admin)
}

async fn ensure_customer_exists(
    store: &EntitlementStore,
    customer_id: &str,
) -> Result<(), ApiError> {
    let exists = store
        .get_customer(customer_id)
        .await
        .map_err(|error| ApiError::internal("customer_lookup_failed", error.to_string()))?
        .is_some();
    if !exists {
        return Err(ApiError::not_found(
            "customer_not_found",
            "customer does not exist",
        ));
    }
    Ok(())
}

fn validate_customer_access(
    customer: &StoredCustomer,
    machine_id_full: &str,
) -> Result<(), ApiError> {
    if customer.revoked {
        return Err(ApiError::forbidden(
            "customer_revoked",
            "customer entitlement is revoked",
        ));
    }

    if !customer.allowed_machine_ids.is_empty()
        && !customer
            .allowed_machine_ids
            .iter()
            .any(|item| item == machine_id_full)
    {
        return Err(ApiError::forbidden(
            "machine_not_allowed",
            "machine_id_full is not allowed for this customer",
        ));
    }

    Ok(())
}

fn load_signing_key() -> Result<SigningKey> {
    if let Ok(encoded) = std::env::var("SENTINEL_ENTITLEMENT_PRIVATE_KEY") {
        return decode_private_key(&encoded);
    }

    let keys_path = std::env::var("SENTINEL_ENTITLEMENT_KEYS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("license_keys.json"));
    let raw = std::fs::read_to_string(&keys_path)
        .with_context(|| format!("failed to read keys file {}", keys_path.display()))?;
    let key_store: KeyPairStore =
        serde_json::from_str(&raw).context("failed to parse keys file as JSON")?;
    decode_private_key(&key_store.private_key)
}

fn decode_private_key(encoded: &str) -> Result<SigningKey> {
    let bytes = BASE64
        .decode(encoded.trim())
        .context("failed to decode base64 private key")?;
    let key_array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("invalid private key length"))?;
    Ok(SigningKey::from_bytes(&key_array))
}

fn normalize_machine_id(value: Option<&str>) -> Result<String, ApiError> {
    let normalized = value
        .map(str::trim)
        .unwrap_or_default()
        .replace('-', "")
        .to_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request(
            "invalid_machine_id",
            "machine_id_full must be a 64-char lowercase hex string",
        ));
    }
    Ok(normalized)
}

fn normalize_customer_id(value: &str) -> Result<String, ApiError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_customer_id",
            "customer_id must not be empty",
        ));
    }
    Ok(normalized.to_string())
}

fn normalize_machine_ids(values: Vec<String>) -> Result<Vec<String>, ApiError> {
    values
        .into_iter()
        .map(|value| normalize_machine_id(Some(value.as_str())))
        .collect()
}

fn trim_required<'a>(
    value: &'a str,
    code: &'static str,
    message: &'static str,
) -> Result<&'a str, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(code, message));
    }
    Ok(trimmed)
}

fn map_customer_response(customer: StoredCustomer) -> CustomerResponse {
    CustomerResponse {
        customer_id: customer.customer_id,
        license_id: customer.license_id,
        tier: customer.tier,
        feature_ids: customer.feature_ids,
        device_limit: customer.device_limit,
        revoked: customer.revoked,
        allowed_machine_ids: customer.allowed_machine_ids,
        ttl_seconds: customer.ttl_seconds,
        refresh_api_key_configured: customer.refresh_api_key_hash.is_some(),
        refresh_api_key_last_used_at: customer.refresh_api_key_last_used_at,
        last_entitlement_issued_at: customer.last_entitlement_issued_at,
    }
}

fn map_audit_event_response(event: StoredAuditEvent) -> AuditEventResponse {
    AuditEventResponse {
        id: event.id,
        event_type: event.event_type,
        actor: event.actor,
        customer_id: event.customer_id,
        machine_id: event.machine_id,
        details: event.details,
        metadata: event.metadata,
        created_at: event.created_at,
    }
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn generate_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn log_refresh_context(customer_id: &str, machine_id_full: &str, request: &RefreshRequest) {
    let current = request.current_entitlement.as_ref();
    let client = request.client.as_ref();
    tracing::info!(
        customer_id,
        machine_id_prefix = %machine_id_full.chars().take(12).collect::<String>(),
        license_present = request.license_present.unwrap_or(false),
        current_token_exists = current.and_then(|item| item.exists).unwrap_or(false),
        current_token_valid = current.and_then(|item| item.valid).unwrap_or(false),
        current_license_id = current.and_then(|item| item.license_id.as_deref()).unwrap_or(""),
        current_tier = current.and_then(|item| item.tier.as_deref()).unwrap_or(""),
        current_feature_count = current.and_then(|item| item.feature_ids.as_ref().map(Vec::len)).unwrap_or(0),
        current_expires_at = current.and_then(|item| item.expires_at).unwrap_or_default(),
        client_product = client.and_then(|item| item.product.as_deref()).unwrap_or(""),
        client_version = client.and_then(|item| item.version.as_deref()).unwrap_or(""),
        client_platform = client.and_then(|item| item.platform.as_deref()).unwrap_or(""),
        "issued entitlement refresh"
    );
}

fn default_admin_feature_ids() -> Vec<String> {
    vec![
        "ai_runtime".to_string(),
        "bug_bounty".to_string(),
        "plugin_catalog_access".to_string(),
        "plugin_catalog_write".to_string(),
        "plugin_catalog_delete".to_string(),
    ]
}
