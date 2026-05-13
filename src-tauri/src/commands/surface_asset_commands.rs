use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{DatabaseService, SurfaceAssetFilter, SurfaceAssetRow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::State;
use uuid::Uuid;

use super::surface_scope_sync_support::{create_missing_in_scope_domains, infer_root_domain};

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceAssetUpdateRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub status: Option<String>,
    pub internet_exposure: Option<String>,
    pub criticality: Option<String>,
    pub risk_level: Option<String>,
    pub typed_details: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceAssetManualImportRequest {
    pub program_id: String,
    pub asset_type: String,
    pub raw_input: String,
    pub source: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub internet_exposure: Option<String>,
    pub criticality: Option<String>,
    pub risk_level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceAssetManualImportResult {
    pub requested: usize,
    pub created: usize,
    pub skipped: usize,
    pub scopes_created: usize,
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_asset_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "org" => Some("org"),
        "domain" => Some("domain"),
        "ip" => Some("ip"),
        "host" => Some("host"),
        "web" => Some("web"),
        _ => None,
    }
}

fn normalize_asset_name(asset_type: &str, raw_value: &str) -> Option<String> {
    let value = raw_value.trim();
    if value.is_empty() {
        return None;
    }

    let normalized = match asset_type {
        "org" | "domain" | "host" => value.to_ascii_lowercase(),
        _ => value.to_string(),
    };

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn build_manual_import_artifact(asset_type: &str, asset_name: &str) -> Option<Value> {
    match asset_type {
        "domain" => {
            let root_domain = infer_root_domain(asset_name);
            let subdomain_level = asset_name
                .split('.')
                .filter(|part| !part.is_empty())
                .count()
                .checked_sub(2)
                .map(|value| value as i32);

            Some(json!({
                "fqdn": asset_name,
                "main_domain": root_domain.clone(),
                "root_domain": root_domain,
                "subdomain_level": subdomain_level,
            }))
        }
        "ip" => Some(json!({
            "ip_address": asset_name,
            "ip_version": if asset_name.contains(':') { "ipv6" } else { "ipv4" },
        })),
        "host" => Some(json!({
            "hostname": asset_name,
            "fqdn": if asset_name.contains('.') { Some(asset_name) } else { None::<&str> },
        })),
        "web" => Some(json!({
            "canonical_url": asset_name,
            "scheme": asset_name.split("://").next().filter(|scheme| *scheme != asset_name),
        })),
        _ => None,
    }
}

fn normalize_json_value(value: Value) -> Option<Value> {
    match value {
        Value::Null => None,
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(Value::String(trimmed.to_string()))
            }
        }
        Value::Array(values) => Some(Value::Array(
            values
                .into_iter()
                .filter_map(normalize_json_value)
                .collect(),
        )),
        Value::Object(values) => {
            let normalized: Map<String, Value> = values
                .into_iter()
                .filter_map(|(key, value)| normalize_json_value(value).map(|item| (key, item)))
                .collect();
            if normalized.is_empty() {
                None
            } else {
                Some(Value::Object(normalized))
            }
        }
        other => Some(other),
    }
}

fn normalize_typed_details(value: Option<Value>) -> Result<Option<Value>, String> {
    match value {
        None => Ok(None),
        Some(Value::Object(map)) => Ok(normalize_json_value(Value::Object(map))),
        Some(_) => Err("typed_details must be an object".to_string()),
    }
}

fn value_to_trimmed_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|item| match item {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

fn derive_asset_name_from_typed_details(
    asset_type: &str,
    typed_details: &Value,
) -> Result<String, String> {
    let name = match asset_type {
        "org" => value_to_trimmed_string(typed_details.get("org_name")),
        "domain" => value_to_trimmed_string(typed_details.get("fqdn"))
            .and_then(|value| normalize_asset_name("domain", &value)),
        "ip" => value_to_trimmed_string(typed_details.get("ip_address"))
            .and_then(|value| normalize_asset_name("ip", &value)),
        "host" => value_to_trimmed_string(typed_details.get("hostname"))
            .or_else(|| value_to_trimmed_string(typed_details.get("fqdn")))
            .and_then(|value| normalize_asset_name("host", &value)),
        "port" | "service" => {
            let host = value_to_trimmed_string(typed_details.get("ip_address"))
                .ok_or_else(|| "IP address is required".to_string())?;
            let port = value_to_trimmed_string(typed_details.get("port"))
                .ok_or_else(|| "Port is required".to_string())?;
            let protocol = value_to_trimmed_string(typed_details.get("transport_protocol"))
                .ok_or_else(|| "Transport protocol is required".to_string())?;
            Some(format!("{host}:{port}/{}", protocol.to_ascii_lowercase()))
        }
        "web" => value_to_trimmed_string(typed_details.get("canonical_url")),
        "certificate" => value_to_trimmed_string(typed_details.get("sha256")),
        _ => None,
    };

    name.ok_or_else(|| "Missing primary typed field for asset identity".to_string())
}

#[tauri::command]
pub async fn surface_update_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
    request: SurfaceAssetUpdateRequest,
) -> Result<bool, String> {
    let SurfaceAssetUpdateRequest {
        display_name,
        description,
        owner,
        status,
        internet_exposure,
        criticality,
        risk_level,
        typed_details,
    } = request;

    let Some(mut asset) = db_service
        .get_surface_asset_by_id(&asset_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Asset not found".to_string());
    };

    let typed_details = normalize_typed_details(typed_details)?;
    if let Some(ref typed_details_value) = typed_details {
        let next_asset_name =
            derive_asset_name_from_typed_details(&asset.asset_type, typed_details_value)?;
        if next_asset_name != asset.asset_name {
            if let Some(existing) = db_service
                .get_surface_asset_by_identity(
                    &asset.program_id,
                    &asset.asset_type,
                    &next_asset_name,
                )
                .await
                .map_err(|e| e.to_string())?
            {
                if existing.id != asset.id {
                    return Err("Asset identity already exists".to_string());
                }
            }
            asset.asset_name = next_asset_name;
        }
    }

    asset.display_name = normalize_optional_string(display_name);
    asset.description = normalize_optional_string(description);
    asset.owner = normalize_optional_string(owner);
    asset.internet_exposure = normalize_optional_string(internet_exposure);
    asset.criticality = normalize_optional_string(criticality);
    asset.risk_level = normalize_optional_string(risk_level);
    if let Some(status) = normalize_optional_string(status) {
        asset.status = status;
    }
    asset.updated_at = Utc::now().to_rfc3339();
    asset.updated_by = Some("surface_inventory".to_string());

    let updated = db_service
        .update_surface_asset(&asset)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref typed_details_value) = typed_details {
        db_service
            .upsert_surface_extension_from_artifact(
                &asset.asset_type,
                &asset.id,
                typed_details_value,
            )
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(updated)
}

#[tauri::command]
pub async fn surface_manual_import_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceAssetManualImportRequest,
) -> Result<SurfaceAssetManualImportResult, String> {
    let program_id = request.program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let Some(asset_type) = normalize_asset_type(&request.asset_type) else {
        return Err("Unsupported asset type".to_string());
    };

    let source =
        normalize_optional_string(request.source).unwrap_or_else(|| "manual_import".to_string());
    let owner = normalize_optional_string(request.owner);
    let description = normalize_optional_string(request.description);
    let status = normalize_optional_string(request.status).unwrap_or_else(|| "active".to_string());
    let internet_exposure = normalize_optional_string(request.internet_exposure);
    let criticality = normalize_optional_string(request.criticality);
    let risk_level = normalize_optional_string(request.risk_level);

    let requested_values: Vec<String> = request
        .raw_input
        .lines()
        .filter_map(|line| normalize_asset_name(asset_type, line))
        .collect();

    if requested_values.is_empty() {
        return Err("No valid assets to import".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut seen = std::collections::HashSet::new();

    for asset_name in &requested_values {
        if !seen.insert(asset_name.clone()) {
            skipped += 1;
            continue;
        }

        if db_service
            .get_surface_asset_by_identity(program_id, asset_type, asset_name)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            skipped += 1;
            continue;
        }

        let asset = SurfaceAssetRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_type: asset_type.to_string(),
            asset_name: asset_name.clone(),
            display_name: Some(asset_name.clone()),
            description: description.clone(),
            org_id: None,
            business_unit: None,
            project: None,
            owner: owner.clone(),
            maintainer: None,
            contact: None,
            env: None,
            internet_exposure: internet_exposure.clone(),
            criticality: criticality.clone(),
            data_level: None,
            source: Some(source.clone()),
            first_seen_at: now.clone(),
            last_seen_at: now.clone(),
            last_verified_at: Some(now.clone()),
            discovery_task_id: None,
            status: status.clone(),
            alive_status: Some("alive".to_string()),
            confidence_score: Some(1.0),
            fingerprint_confidence: None,
            risk_score: None,
            risk_level: risk_level.clone(),
            vulnerabilities_count: Some(0),
            weak_password_flag: Some(false),
            expired_cert_flag: Some(false),
            exposed_to_internet_flag: internet_exposure
                .as_ref()
                .map(|value| matches!(value.as_str(), "internet" | "public")),
            viewed_at: None,
            viewed_by: None,
            is_favorite: false,
            metadata_json: Some(json!({ "import_mode": "manual" }).to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
            created_by: Some("surface_manual_import".to_string()),
            updated_by: Some("surface_manual_import".to_string()),
        };

        db_service
            .create_surface_asset(&asset)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(artifact) = build_manual_import_artifact(asset_type, asset_name) {
            db_service
                .upsert_surface_extension_from_artifact(asset_type, &asset.id, &artifact)
                .await
                .map_err(|e| e.to_string())?;
        }

        created += 1;
    }

    let scopes_created = if asset_type == "domain" {
        create_missing_in_scope_domains(
            db_service.inner().as_ref(),
            program_id,
            &requested_values,
            "Auto-created from manual domain import",
            "auto_root_domain_from_import",
        )
        .await?
    } else {
        0
    };

    Ok(SurfaceAssetManualImportResult {
        requested: requested_values.len(),
        created,
        skipped,
        scopes_created,
    })
}

#[tauri::command]
pub async fn surface_delete_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
) -> Result<bool, String> {
    db_service
        .delete_surface_assets_by_ids(&[asset_id])
        .await
        .map(|deleted| deleted > 0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_batch_delete_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_ids: Vec<String>,
) -> Result<usize, String> {
    db_service
        .delete_surface_assets_by_ids(&asset_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_delete_inventory(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceAssetFilter,
) -> Result<usize, String> {
    db_service
        .delete_surface_inventory(&filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_mark_asset_viewed(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
) -> Result<usize, String> {
    db_service
        .mark_surface_assets_viewed(&[asset_id], &Utc::now().to_rfc3339(), "surface_inventory")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_batch_mark_assets_viewed(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_ids: Vec<String>,
) -> Result<usize, String> {
    db_service
        .mark_surface_assets_viewed(&asset_ids, &Utc::now().to_rfc3339(), "surface_inventory")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_mark_inventory_viewed(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceAssetFilter,
) -> Result<usize, String> {
    db_service
        .mark_surface_inventory_viewed(&filter, &Utc::now().to_rfc3339(), "surface_inventory")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_set_asset_favorite(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
    is_favorite: bool,
) -> Result<bool, String> {
    db_service
        .set_surface_asset_favorite(
            &asset_id,
            is_favorite,
            &Utc::now().to_rfc3339(),
            "surface_inventory",
        )
        .await
        .map_err(|e| e.to_string())
}
