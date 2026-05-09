use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{DatabaseService, SurfaceAssetFilter, SurfaceAssetRow, SurfaceSeedRow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use uuid::Uuid;

use crate::commands::surface_scope_sync_support::infer_root_domain;

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceSeedUpsertRequest {
    pub id: Option<String>,
    pub program_id: String,
    pub seed_type: String,
    pub seed_value: String,
    pub status: Option<String>,
    pub source: Option<String>,
    pub confidence_score: Option<f64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceSeedSyncResult {
    pub requested: usize,
    pub created: usize,
    pub reactivated: usize,
    pub disabled: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceSeedDistributionBucket {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceSeedDistribution {
    pub types: Vec<SurfaceSeedDistributionBucket>,
    pub statuses: Vec<SurfaceSeedDistributionBucket>,
    pub sources: Vec<SurfaceSeedDistributionBucket>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceSeedQueryRequest {
    pub program_id: Option<String>,
    pub search: Option<String>,
    pub seed_type: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceSeedQueryResult {
    pub items: Vec<SurfaceSeedRow>,
    pub total: usize,
}

async fn migrate_legacy_surface_seed_types(
    db_service: &Arc<DatabaseService>,
) -> Result<(), String> {
    db_service
        .rename_surface_seed_type("fofa_icon_hash", "favicon_hash")
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn matches_seed_filter(seed: &SurfaceSeedRow, search: Option<&str>, seed_type: Option<&str>, status: Option<&str>, source: Option<&str>) -> bool {
    if let Some(seed_type_filter) = seed_type
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        if seed.seed_type != seed_type_filter {
            return false;
        }
    }

    if let Some(status_filter) = status
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        if seed.status != status_filter {
            return false;
        }
    }

    if let Some(source_filter) = source
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        if seed.source.as_deref().unwrap_or("unknown") != source_filter {
            return false;
        }
    }

    if let Some(keyword) = search
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
    {
        let matched = [
            seed.seed_value.as_str(),
            seed.seed_type.as_str(),
            seed.status.as_str(),
            seed.source.as_deref().unwrap_or("unknown"),
        ]
        .into_iter()
        .any(|value| value.to_ascii_lowercase().contains(&keyword));
        if !matched {
            return false;
        }
    }

    true
}

fn build_distribution_buckets<I>(
    seeds: I,
    bucket_of: impl Fn(&SurfaceSeedRow) -> String,
) -> Vec<SurfaceSeedDistributionBucket>
where
    I: IntoIterator<Item = SurfaceSeedRow>,
{
    let mut counts: HashMap<String, usize> = HashMap::new();
    for seed in seeds {
        *counts.entry(bucket_of(&seed)).or_insert(0) += 1;
    }

    let mut buckets: Vec<SurfaceSeedDistributionBucket> = counts
        .into_iter()
        .map(|(key, count)| SurfaceSeedDistributionBucket { key, count })
        .collect();
    buckets.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    buckets
}

fn normalize_seed_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "domain" => Some("domain"),
        "root_domain" => Some("root_domain"),
        "brand_keyword" => Some("brand_keyword"),
        "favicon_hash" => Some("favicon_hash"),
        "org_name" => Some("org_name"),
        "asn" => Some("asn"),
        "cname_keyword" => Some("cname_keyword"),
        "title_keyword" => Some("title_keyword"),
        "body_keyword" => Some("body_keyword"),
        "header_keyword" => Some("header_keyword"),
        _ => None,
    }
}

fn normalize_seed_status(value: Option<&str>) -> &'static str {
    match value.unwrap_or("active").trim().to_ascii_lowercase().as_str() {
        "disabled" => "disabled",
        _ => "active",
    }
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

fn normalize_seed_value(seed_type: &str, value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = match seed_type {
        "domain" | "root_domain" | "cname_keyword" => trimmed.to_ascii_lowercase(),
        "asn" => {
            if !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            trimmed.to_string()
        }
        _ => trimmed.to_string(),
    };

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn collect_root_domain_seed(
    asset: &SurfaceAssetRow,
    typed_details: Option<&serde_json::Map<String, Value>>,
) -> Option<String> {
    typed_details
        .and_then(|details| details.get("root_domain"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| infer_root_domain(&asset.asset_name))
}

fn collect_favicon_seed(typed_details: Option<&serde_json::Map<String, Value>>) -> Option<String> {
    typed_details
        .and_then(|details| details.get("favicon_hash"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn list_program_surface_assets(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
) -> Result<Vec<SurfaceAssetRow>, String> {
    db_service
        .list_surface_assets(&SurfaceAssetFilter {
            program_id: Some(program_id.to_string()),
            asset_type: None,
            status: None,
            search: None,
            service_name: None,
            transport_protocol: None,
            view_state: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_list_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    status: Option<String>,
) -> Result<Vec<SurfaceSeedRow>, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    db_service
        .list_surface_seeds(program_id.as_deref(), status.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_query_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceSeedQueryRequest,
) -> Result<SurfaceSeedQueryResult, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let total = db_service
        .count_surface_seeds_filtered(
            request.program_id.as_deref(),
            request.search.as_deref(),
            request.seed_type.as_deref(),
            request.status.as_deref(),
            request.source.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())? as usize;

    let items = db_service
        .list_surface_seeds_filtered(
            request.program_id.as_deref(),
            request.search.as_deref(),
            request.seed_type.as_deref(),
            request.status.as_deref(),
            request.source.as_deref(),
            request.limit,
            request.offset,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(SurfaceSeedQueryResult { items, total })
}

#[tauri::command]
pub async fn surface_get_seed_distribution(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    search: Option<String>,
    seed_type: Option<String>,
    status: Option<String>,
    source: Option<String>,
) -> Result<SurfaceSeedDistribution, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let seeds = db_service
        .list_surface_seeds(program_id.as_deref(), None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(SurfaceSeedDistribution {
        types: build_distribution_buckets(
            seeds
                .iter()
                .filter(|seed| {
                    matches_seed_filter(
                        seed,
                        search.as_deref(),
                        None,
                        status.as_deref(),
                        source.as_deref(),
                    )
                })
                .cloned(),
            |seed| seed.seed_type.clone(),
        ),
        statuses: build_distribution_buckets(
            seeds
                .iter()
                .filter(|seed| {
                    matches_seed_filter(
                        seed,
                        search.as_deref(),
                        seed_type.as_deref(),
                        None,
                        source.as_deref(),
                    )
                })
                .cloned(),
            |seed| seed.status.clone(),
        ),
        sources: build_distribution_buckets(
            seeds
                .iter()
                .filter(|seed| {
                    matches_seed_filter(
                        seed,
                        search.as_deref(),
                        seed_type.as_deref(),
                        status.as_deref(),
                        None,
                    )
                })
                .cloned(),
            |seed| seed.source.clone().unwrap_or_else(|| "unknown".to_string()),
        ),
    })
}

#[tauri::command]
pub async fn surface_upsert_seed(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceSeedUpsertRequest,
) -> Result<SurfaceSeedRow, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let program_id = request.program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let Some(seed_type) = normalize_seed_type(&request.seed_type) else {
        return Err("Unsupported seed type".to_string());
    };
    let Some(seed_value) = normalize_seed_value(seed_type, &request.seed_value) else {
        return Err("Invalid seed value".to_string());
    };

    let now = Utc::now().to_rfc3339();
    let status = normalize_seed_status(request.status.as_deref()).to_string();
    let source = normalize_optional_string(request.source).unwrap_or_else(|| "manual".to_string());
    let confidence_score = request.confidence_score.or(Some(1.0));
    let metadata_json = request.metadata.map(|value| value.to_string());

    if let Some(seed_id) = request.id.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        let Some(mut existing) = db_service
            .get_surface_seed_by_id(seed_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Err("Seed not found".to_string());
        };
        existing.program_id = program_id.to_string();
        existing.seed_type = seed_type.to_string();
        existing.seed_value = seed_value;
        existing.status = status;
        existing.source = Some(source);
        existing.confidence_score = confidence_score;
        existing.metadata_json = metadata_json;
        existing.updated_at = now;

        db_service
            .update_surface_seed(&existing)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(existing);
    }

    if let Some(mut existing) = db_service
        .get_surface_seed_by_identity(program_id, seed_type, &seed_value)
        .await
        .map_err(|e| e.to_string())?
    {
        existing.status = status;
        existing.source = Some(source);
        existing.confidence_score = confidence_score;
        existing.metadata_json = metadata_json;
        existing.updated_at = now;
        db_service
            .update_surface_seed(&existing)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(existing);
    }

    let seed = SurfaceSeedRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        seed_type: seed_type.to_string(),
        seed_value,
        status,
        source: Some(source),
        confidence_score,
        last_run_at: None,
        metadata_json,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_surface_seed(&seed)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_delete_seed(
    db_service: State<'_, Arc<DatabaseService>>,
    seed_id: String,
) -> Result<bool, String> {
    db_service
        .delete_surface_seed(&seed_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_sync_program_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<SurfaceSeedSyncResult, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let program_id = program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let assets = list_program_surface_assets(db_service.inner(), program_id).await?;
    let domain_assets: Vec<SurfaceAssetRow> = assets
        .iter()
        .filter(|asset| asset.asset_type == "domain")
        .cloned()
        .collect();
    let web_assets: Vec<SurfaceAssetRow> = assets
        .iter()
        .filter(|asset| matches!(asset.asset_type.as_str(), "web" | "url" | "website"))
        .cloned()
        .collect();

    let domain_details_by_id = if domain_assets.is_empty() {
        std::collections::HashMap::new()
    } else {
        db_service
            .list_surface_typed_details_map(&domain_assets)
            .await
            .map_err(|e| e.to_string())?
    };
    let web_details_by_id = if web_assets.is_empty() {
        std::collections::HashMap::new()
    } else {
        db_service
            .list_surface_typed_details_map(&web_assets)
            .await
            .map_err(|e| e.to_string())?
    };

    let mut desired = Vec::new();
    let mut seen = HashSet::new();

    for asset in &domain_assets {
        let root_domain = collect_root_domain_seed(
            asset,
            domain_details_by_id
                .get(&asset.id)
                .and_then(Value::as_object),
        );
        if let Some(root_domain) = root_domain {
            let key = format!("root_domain::{root_domain}");
            if seen.insert(key) {
                desired.push(("root_domain".to_string(), root_domain));
            }
        }
    }

    for asset in &web_assets {
        let favicon_hash = collect_favicon_seed(
            web_details_by_id
                .get(&asset.id)
                .and_then(Value::as_object),
        );
        if let Some(favicon_hash) = favicon_hash {
            let key = format!("favicon_hash::{favicon_hash}");
            if seen.insert(key) {
                desired.push(("favicon_hash".to_string(), favicon_hash));
            }
        }
    }

    let requested = desired.len();
    let now = Utc::now().to_rfc3339();
    let existing = db_service
        .list_surface_seeds(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;

    let managed_types = HashSet::from([
        "root_domain".to_string(),
        "favicon_hash".to_string(),
    ]);
    let desired_keys: HashSet<String> = desired
        .iter()
        .map(|(seed_type, seed_value)| format!("{seed_type}::{seed_value}"))
        .collect();

    let mut created = 0usize;
    let mut reactivated = 0usize;
    let mut disabled = 0usize;

    for (seed_type, seed_value) in desired {
        if let Some(mut current) = existing
            .iter()
            .find(|seed| seed.seed_type == seed_type && seed.seed_value == seed_value)
            .cloned()
        {
            let was_active = current.status == "active";
            current.status = "active".to_string();
            current.source = Some("surface_asset_sync".to_string());
            current.confidence_score = Some(0.9);
            current.updated_at = now.clone();
            current.metadata_json = Some(json!({ "sync": "surface_asset_sync" }).to_string());
            db_service
                .update_surface_seed(&current)
                .await
                .map_err(|e| e.to_string())?;
            if !was_active {
                reactivated += 1;
            }
            continue;
        }

        let seed = SurfaceSeedRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            seed_type,
            seed_value,
            status: "active".to_string(),
            source: Some("surface_asset_sync".to_string()),
            confidence_score: Some(0.9),
            last_run_at: None,
            metadata_json: Some(json!({ "sync": "surface_asset_sync" }).to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        db_service
            .create_surface_seed(&seed)
            .await
            .map_err(|e| e.to_string())?;
        created += 1;
    }

    for mut seed in existing.into_iter().filter(|seed| {
        seed.source.as_deref() == Some("surface_asset_sync")
            && managed_types.contains(&seed.seed_type)
            && !desired_keys.contains(&format!("{}::{}", seed.seed_type, seed.seed_value))
            && seed.status == "active"
    }) {
        seed.status = "disabled".to_string();
        seed.updated_at = now.clone();
        db_service
            .update_surface_seed(&seed)
            .await
            .map_err(|e| e.to_string())?;
        disabled += 1;
    }

    Ok(SurfaceSeedSyncResult {
        requested,
        created,
        reactivated,
        disabled,
    })
}
