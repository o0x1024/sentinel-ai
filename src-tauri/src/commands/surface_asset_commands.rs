use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sentinel_db::{DatabaseService, SurfaceAssetFilter, SurfaceAssetRow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

#[tauri::command]
pub async fn surface_update_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
    request: SurfaceAssetUpdateRequest,
) -> Result<bool, String> {
    let Some(mut asset) = db_service
        .get_surface_asset_by_id(&asset_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Asset not found".to_string());
    };

    asset.display_name = normalize_optional_string(request.display_name);
    asset.description = normalize_optional_string(request.description);
    asset.owner = normalize_optional_string(request.owner);
    asset.internet_exposure = normalize_optional_string(request.internet_exposure);
    asset.criticality = normalize_optional_string(request.criticality);
    asset.risk_level = normalize_optional_string(request.risk_level);
    if let Some(status) = normalize_optional_string(request.status) {
        asset.status = status;
    }
    asset.updated_at = Utc::now().to_rfc3339();
    asset.updated_by = Some("surface_inventory".to_string());

    db_service
        .update_surface_asset(&asset)
        .await
        .map_err(|e| e.to_string())
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
    let mut view_filter = filter;
    view_filter.limit = None;
    view_filter.offset = None;

    let asset_ids = db_service
        .list_surface_assets(&view_filter)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|asset| asset.id)
        .collect::<Vec<_>>();

    db_service
        .mark_surface_assets_viewed(&asset_ids, &Utc::now().to_rfc3339(), "surface_inventory")
        .await
        .map_err(|e| e.to_string())
}
