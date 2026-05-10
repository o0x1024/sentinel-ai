use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{
    DatabaseService, SurfaceAssetFilter, SurfaceAssetRow, SurfaceSeedCandidateRow, SurfaceSeedRow,
};
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
    pub refreshed: usize,
    pub pending_review: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceLegacySeedRetireResult {
    pub matched: usize,
    pub retired: usize,
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

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceSeedCandidateReviewRequest {
    pub candidate_ids: Vec<String>,
    pub decision: String,
}

#[derive(Debug, Clone)]
struct PendingSeedCandidateInput {
    seed_type: String,
    seed_value: String,
    source_asset_id: String,
    source_asset_type: String,
    source_detail_key: String,
    source_display_value: String,
    source_canonical_url: Option<String>,
    confidence_score: Option<f64>,
    metadata_json: Option<String>,
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

fn matches_seed_filter(
    seed: &SurfaceSeedRow,
    search: Option<&str>,
    seed_type: Option<&str>,
    status: Option<&str>,
    source: Option<&str>,
) -> bool {
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
    match value
        .unwrap_or("active")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
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

fn merge_metadata_json(existing: Option<&str>, patch: Value) -> Option<String> {
    let mut base = existing
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .unwrap_or_else(|| json!({}));
    match (&mut base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                base_map.insert(key, value);
            }
            Some(Value::Object(base_map.clone()).to_string())
        }
        (_, value) => Some(value.to_string()),
    }
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

async fn upsert_pending_seed_candidate(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
    candidate: PendingSeedCandidateInput,
    now: &str,
) -> Result<bool, String> {
    if let Some(mut existing) = db_service
        .get_surface_seed_candidate_by_identity(
            program_id,
            &candidate.seed_type,
            &candidate.seed_value,
            &candidate.source_asset_id,
            &candidate.source_detail_key,
        )
        .await
        .map_err(|e| e.to_string())?
    {
        existing.source_asset_type = candidate.source_asset_type;
        existing.source_display_value = candidate.source_display_value;
        existing.source_canonical_url = candidate.source_canonical_url;
        existing.confidence_score = candidate.confidence_score;
        existing.observed_at = now.to_string();
        existing.metadata_json = candidate.metadata_json;
        existing.updated_at = now.to_string();
        db_service
            .update_surface_seed_candidate(&existing)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(false);
    }

    let row = SurfaceSeedCandidateRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        seed_type: candidate.seed_type,
        seed_value: candidate.seed_value,
        status: "pending".to_string(),
        source_asset_id: candidate.source_asset_id,
        source_asset_type: candidate.source_asset_type,
        source_detail_key: candidate.source_detail_key,
        source_display_value: candidate.source_display_value,
        source_canonical_url: candidate.source_canonical_url,
        confidence_score: candidate.confidence_score,
        observed_at: now.to_string(),
        reviewed_at: None,
        metadata_json: candidate.metadata_json,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    };

    db_service
        .create_surface_seed_candidate(&row)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
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
            favicon_hash: None,
            has_favicon_hash: None,
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
pub async fn surface_list_seed_candidates(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    status: Option<String>,
) -> Result<Vec<SurfaceSeedCandidateRow>, String> {
    db_service
        .list_surface_seed_candidates(program_id.as_deref(), status.as_deref())
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

    if let Some(seed_id) = request
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
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
pub async fn surface_batch_delete_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    seed_ids: Vec<String>,
) -> Result<usize, String> {
    let mut deleted = 0usize;
    let mut seen = HashSet::new();

    for seed_id in seed_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if !seen.insert(seed_id.clone()) {
            continue;
        }

        if db_service
            .delete_surface_seed(&seed_id)
            .await
            .map_err(|e| e.to_string())?
        {
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub async fn surface_delete_filtered_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceSeedQueryRequest,
) -> Result<usize, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let program_id = request
        .program_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Program is required".to_string())?;

    let total = db_service
        .count_surface_seeds_filtered(
            Some(program_id),
            request.search.as_deref(),
            request.seed_type.as_deref(),
            request.status.as_deref(),
            request.source.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if total <= 0 {
        return Ok(0);
    }

    let seeds = db_service
        .list_surface_seeds_filtered(
            Some(program_id),
            request.search.as_deref(),
            request.seed_type.as_deref(),
            request.status.as_deref(),
            request.source.as_deref(),
            Some(total as u32),
            Some(0),
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted = 0usize;
    for seed in seeds {
        if db_service
            .delete_surface_seed(&seed.id)
            .await
            .map_err(|e| e.to_string())?
        {
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub async fn surface_review_seed_candidates(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SurfaceSeedCandidateReviewRequest,
) -> Result<usize, String> {
    let decision = request.decision.trim().to_ascii_lowercase();
    if decision != "approved" && decision != "rejected" {
        return Err("Seed candidate decision must be 'approved' or 'rejected'".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let mut reviewed = 0usize;
    let mut seen = HashSet::new();

    for candidate_id in request
        .candidate_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if !seen.insert(candidate_id.clone()) {
            continue;
        }

        let Some(mut candidate) = db_service
            .get_surface_seed_candidate_by_id(&candidate_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if candidate.status != "pending" {
            continue;
        }

        if decision == "approved" {
            let seed_metadata = Some(
                json!({
                    "review_source": "surface_seed_candidate",
                    "candidate_id": candidate.id,
                    "source_asset_id": candidate.source_asset_id,
                    "source_asset_type": candidate.source_asset_type,
                    "source_display_value": candidate.source_display_value,
                    "source_canonical_url": candidate.source_canonical_url,
                })
                .to_string(),
            );

            if let Some(mut seed) = db_service
                .get_surface_seed_by_identity(
                    &candidate.program_id,
                    &candidate.seed_type,
                    &candidate.seed_value,
                )
                .await
                .map_err(|e| e.to_string())?
            {
                seed.status = "active".to_string();
                seed.source = Some("candidate_review".to_string());
                seed.confidence_score = Some(
                    seed.confidence_score
                        .unwrap_or(0.0)
                        .max(candidate.confidence_score.unwrap_or(0.9)),
                );
                seed.metadata_json = seed_metadata;
                seed.updated_at = now.clone();
                db_service
                    .update_surface_seed(&seed)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let seed = SurfaceSeedRow {
                    id: Uuid::new_v4().to_string(),
                    program_id: candidate.program_id.clone(),
                    seed_type: candidate.seed_type.clone(),
                    seed_value: candidate.seed_value.clone(),
                    status: "active".to_string(),
                    source: Some("candidate_review".to_string()),
                    confidence_score: candidate.confidence_score.or(Some(0.9)),
                    last_run_at: None,
                    metadata_json: seed_metadata,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                db_service
                    .create_surface_seed(&seed)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        candidate.status = decision.clone();
        candidate.reviewed_at = Some(now.clone());
        candidate.updated_at = now.clone();
        db_service
            .update_surface_seed_candidate(&candidate)
            .await
            .map_err(|e| e.to_string())?;
        reviewed += 1;
    }

    Ok(reviewed)
}

#[tauri::command]
pub async fn surface_count_legacy_synced_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<usize, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let program_id = program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let seeds = db_service
        .list_surface_seeds(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(seeds
        .iter()
        .filter(|seed| seed.source.as_deref() == Some("surface_asset_sync"))
        .count())
}

#[tauri::command]
pub async fn surface_retire_legacy_synced_seeds(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<SurfaceLegacySeedRetireResult, String> {
    migrate_legacy_surface_seed_types(db_service.inner()).await?;
    let program_id = program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let seeds = db_service
        .list_surface_seeds(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;
    let mut matched = 0usize;
    let mut retired = 0usize;

    for mut seed in seeds {
        if seed.source.as_deref() != Some("surface_asset_sync") {
            continue;
        }
        matched += 1;
        seed.status = "disabled".to_string();
        seed.source = Some("legacy_surface_asset_sync".to_string());
        seed.metadata_json = merge_metadata_json(
            seed.metadata_json.as_deref(),
            json!({
                "legacy_review_required": true,
                "legacy_retired_at": now,
                "legacy_retire_reason": "historical asset-synced seed lacks source evidence after candidate review migration"
            }),
        );
        seed.updated_at = now.clone();
        if db_service
            .update_surface_seed(&seed)
            .await
            .map_err(|e| e.to_string())?
        {
            retired += 1;
        }
    }

    Ok(SurfaceLegacySeedRetireResult { matched, retired })
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
            let key = format!("root_domain::{root_domain}::{}", asset.id);
            if seen.insert(key) {
                desired.push(PendingSeedCandidateInput {
                    seed_type: "root_domain".to_string(),
                    seed_value: root_domain,
                    source_asset_id: asset.id.clone(),
                    source_asset_type: asset.asset_type.clone(),
                    source_detail_key: "root_domain".to_string(),
                    source_display_value: asset.asset_name.clone(),
                    source_canonical_url: None,
                    confidence_score: Some(0.9),
                    metadata_json: Some(
                        json!({
                            "sync": "surface_asset_candidate_sync",
                            "source_asset_name": asset.asset_name,
                        })
                        .to_string(),
                    ),
                });
            }
        }
    }

    for asset in &web_assets {
        let typed_details = web_details_by_id.get(&asset.id).and_then(Value::as_object);
        let favicon_hash = collect_favicon_seed(typed_details);
        if let Some(favicon_hash) = favicon_hash {
            let canonical_url = typed_details
                .and_then(|details| details.get("canonical_url"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let key = format!("favicon_hash::{favicon_hash}::{}", asset.id);
            if seen.insert(key) {
                desired.push(PendingSeedCandidateInput {
                    seed_type: "favicon_hash".to_string(),
                    seed_value: favicon_hash,
                    source_asset_id: asset.id.clone(),
                    source_asset_type: asset.asset_type.clone(),
                    source_detail_key: "favicon_hash".to_string(),
                    source_display_value: canonical_url
                        .clone()
                        .unwrap_or_else(|| asset.asset_name.clone()),
                    source_canonical_url: canonical_url.clone(),
                    confidence_score: Some(0.9),
                    metadata_json: Some(
                        json!({
                            "sync": "surface_asset_candidate_sync",
                            "source_asset_name": asset.asset_name,
                            "source_canonical_url": canonical_url,
                        })
                        .to_string(),
                    ),
                });
            }
        }
    }

    let requested = desired.len();
    let now = Utc::now().to_rfc3339();
    let mut created = 0usize;
    let mut refreshed = 0usize;

    for candidate in desired {
        if upsert_pending_seed_candidate(db_service.inner(), program_id, candidate, &now).await? {
            created += 1;
        } else {
            refreshed += 1;
        }
    }

    let pending_review = db_service
        .list_surface_seed_candidates(Some(program_id), Some("pending"))
        .await
        .map_err(|e| e.to_string())?
        .len();

    Ok(SurfaceSeedSyncResult {
        requested,
        created,
        refreshed,
        pending_review,
    })
}
