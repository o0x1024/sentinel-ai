use std::collections::HashMap;
use std::sync::Arc;

use crate::services::ensure_bug_bounty_access;
use sentinel_db::{DatabaseService, SurfaceObservationRow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

const API_MONITOR_PLUGIN_ID: &str = "api_monitor";
const API_SNAPSHOT_ARTIFACT_TYPE: &str = "api_snapshot";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryEndpointPayload {
    path: String,
    method: Option<String>,
    source: Option<String>,
    #[serde(rename = "requestUrl")]
    request_url: Option<String>,
    #[serde(rename = "requestMethod")]
    request_method: Option<String>,
    #[serde(rename = "responseStatus")]
    response_status: Option<u16>,
    #[serde(rename = "responseContentType")]
    response_content_type: Option<String>,
    #[serde(rename = "responsePreview")]
    response_preview: Option<String>,
    #[serde(rename = "responseFetchedAt")]
    response_fetched_at: Option<String>,
    #[serde(rename = "responseError")]
    response_error: Option<String>,
    #[serde(rename = "responseSize")]
    response_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiInventorySnapshotPayload {
    #[serde(rename = "baseUrl")]
    base_url: String,
    endpoints: Vec<ApiInventoryEndpointPayload>,
    #[serde(rename = "graphqlEndpoint")]
    graphql_endpoint: Option<String>,
    #[serde(rename = "openApiSpec")]
    open_api_spec: Option<String>,
    #[serde(rename = "lastChecked")]
    last_checked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiInventoryObservationPayload {
    #[serde(rename = "baseUrl")]
    base_url: String,
    success: bool,
    snapshot: Option<ApiInventorySnapshotPayload>,
    #[serde(rename = "addedEndpoints")]
    added_endpoints: Option<Vec<ApiInventoryEndpointPayload>>,
    #[serde(rename = "removedEndpoints")]
    removed_endpoints: Option<Vec<ApiInventoryEndpointPayload>>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryTargetSummary {
    pub program_id: String,
    pub base_url: String,
    pub success: bool,
    pub endpoint_count: usize,
    pub graphql_endpoint: Option<String>,
    pub open_api_spec: Option<String>,
    pub last_checked: Option<String>,
    pub observed_at: String,
    pub run_id: String,
    pub task_name: Option<String>,
    pub execution_mode: Option<String>,
    pub added_endpoints_count: usize,
    pub removed_endpoints_count: usize,
    pub sample_endpoints: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryTargetDetail {
    pub program_id: String,
    pub base_url: String,
    pub success: bool,
    pub endpoint_count: usize,
    pub graphql_endpoint: Option<String>,
    pub open_api_spec: Option<String>,
    pub last_checked: Option<String>,
    pub observed_at: String,
    pub run_id: String,
    pub task_name: Option<String>,
    pub execution_mode: Option<String>,
    pub endpoints: Vec<ApiInventoryEndpointPayload>,
    pub added_endpoints: Vec<ApiInventoryEndpointPayload>,
    pub removed_endpoints: Vec<ApiInventoryEndpointPayload>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryDeleteTarget {
    #[serde(alias = "programId")]
    pub program_id: String,
    #[serde(alias = "baseUrl")]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryListFilter {
    pub program_id: Option<String>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub execution_mode: Option<String>,
    pub capability: Option<String>,
    pub sort_by: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryListStats {
    pub total_endpoints: usize,
    pub successful_targets: usize,
    pub failed_targets: usize,
    pub changed_targets: usize,
    pub graphql_targets: usize,
    pub open_api_targets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryListResponse {
    pub items: Vec<ApiInventoryTargetSummary>,
    pub total: usize,
    pub filtered_total: usize,
    pub has_more: bool,
    pub stats: ApiInventoryListStats,
}

fn parse_observation_payload(
    observation: &SurfaceObservationRow,
) -> Option<ApiInventoryObservationPayload> {
    serde_json::from_str(&observation.payload_json).ok()
}

fn parse_metadata(metadata_json: &Option<String>) -> Value {
    metadata_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null)
}

fn task_name_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .get("task_name")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn execution_mode_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .get("execution_mode")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn normalize_filter_value(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
}

fn has_changes(target: &ApiInventoryTargetSummary) -> bool {
    target.added_endpoints_count > 0 || target.removed_endpoints_count > 0
}

fn matches_search(target: &ApiInventoryTargetSummary, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    target.base_url.to_lowercase().contains(needle)
        || target
            .task_name
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(needle)
        || target
            .error_message
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(needle)
        || target
            .sample_endpoints
            .iter()
            .any(|endpoint| endpoint.to_lowercase().contains(needle))
}

fn apply_inventory_filters(
    rows: Vec<ApiInventoryTargetSummary>,
    filter: &ApiInventoryListFilter,
) -> Vec<ApiInventoryTargetSummary> {
    let search = normalize_filter_value(&filter.search).unwrap_or_default();
    let status = normalize_filter_value(&filter.status);
    let execution_mode = normalize_filter_value(&filter.execution_mode);
    let capability = normalize_filter_value(&filter.capability);

    rows.into_iter()
        .filter(|target| {
            filter
                .program_id
                .as_deref()
                .map(|program_id| target.program_id == program_id)
                .unwrap_or(true)
        })
        .filter(|target| match status.as_deref() {
            Some("success") => target.success,
            Some("failed") => !target.success,
            _ => true,
        })
        .filter(|target| match execution_mode.as_deref() {
            Some("scheduler") | Some("manual") => target
                .execution_mode
                .as_deref()
                .map(|value| {
                    value.eq_ignore_ascii_case(execution_mode.as_deref().unwrap_or_default())
                })
                .unwrap_or(false),
            _ => true,
        })
        .filter(|target| match capability.as_deref() {
            Some("changed") => has_changes(target),
            Some("graphql") => target.graphql_endpoint.is_some(),
            Some("openapi") => target.open_api_spec.is_some(),
            Some("errors") => target.error_message.is_some(),
            _ => true,
        })
        .filter(|target| matches_search(target, &search))
        .collect()
}

fn sort_inventory_rows(rows: &mut [ApiInventoryTargetSummary], sort_by: &Option<String>) {
    match normalize_filter_value(sort_by).as_deref() {
        Some("endpoint_desc") => rows.sort_by(|left, right| {
            right
                .endpoint_count
                .cmp(&left.endpoint_count)
                .then_with(|| right.observed_at.cmp(&left.observed_at))
        }),
        Some("changes_desc") => rows.sort_by(|left, right| {
            let left_changes = left.added_endpoints_count + left.removed_endpoints_count;
            let right_changes = right.added_endpoints_count + right.removed_endpoints_count;
            right_changes
                .cmp(&left_changes)
                .then_with(|| right.observed_at.cmp(&left.observed_at))
        }),
        Some("base_url_asc") => rows.sort_by(|left, right| left.base_url.cmp(&right.base_url)),
        _ => rows.sort_by(|left, right| {
            right
                .observed_at
                .cmp(&left.observed_at)
                .then_with(|| left.base_url.cmp(&right.base_url))
        }),
    }
}

fn build_inventory_stats(rows: &[ApiInventoryTargetSummary]) -> ApiInventoryListStats {
    ApiInventoryListStats {
        total_endpoints: rows.iter().map(|target| target.endpoint_count).sum(),
        successful_targets: rows.iter().filter(|target| target.success).count(),
        failed_targets: rows.iter().filter(|target| !target.success).count(),
        changed_targets: rows.iter().filter(|target| has_changes(target)).count(),
        graphql_targets: rows
            .iter()
            .filter(|target| target.graphql_endpoint.is_some())
            .count(),
        open_api_targets: rows
            .iter()
            .filter(|target| target.open_api_spec.is_some())
            .count(),
    }
}

fn collect_latest_targets(
    observations: Vec<SurfaceObservationRow>,
) -> Vec<ApiInventoryTargetSummary> {
    let mut latest_by_target: HashMap<
        String,
        (SurfaceObservationRow, ApiInventoryObservationPayload),
    > = HashMap::new();

    for observation in observations {
        let Some(payload) = parse_observation_payload(&observation) else {
            continue;
        };
        let target_key = format!(
            "{}::{}",
            observation.program_id,
            observation
                .object_key
                .clone()
                .unwrap_or_else(|| payload.base_url.clone())
        );
        latest_by_target
            .entry(target_key)
            .or_insert((observation, payload));
    }

    latest_by_target
        .into_values()
        .map(|(observation, payload)| observation_to_summary(&observation, payload))
        .collect()
}

fn observation_to_summary(
    observation: &SurfaceObservationRow,
    payload: ApiInventoryObservationPayload,
) -> ApiInventoryTargetSummary {
    let metadata = parse_metadata(&observation.metadata_json);
    let snapshot = payload.snapshot;
    let endpoints = snapshot
        .as_ref()
        .map(|value| value.endpoints.as_slice())
        .unwrap_or(&[]);
    let sample_endpoints = endpoints
        .iter()
        .take(8)
        .map(|endpoint| match endpoint.method.as_deref() {
            Some(method) if !method.trim().is_empty() => format!("{method} {}", endpoint.path),
            _ => endpoint.path.clone(),
        })
        .collect();

    ApiInventoryTargetSummary {
        program_id: observation.program_id.clone(),
        base_url: payload.base_url,
        success: payload.success,
        endpoint_count: endpoints.len(),
        graphql_endpoint: snapshot
            .as_ref()
            .and_then(|value| value.graphql_endpoint.clone()),
        open_api_spec: snapshot
            .as_ref()
            .and_then(|value| value.open_api_spec.clone()),
        last_checked: snapshot.map(|value| value.last_checked),
        observed_at: observation.observed_at.clone(),
        run_id: observation.run_id.clone(),
        task_name: task_name_from_metadata(&metadata),
        execution_mode: execution_mode_from_metadata(&metadata),
        added_endpoints_count: payload
            .added_endpoints
            .as_ref()
            .map(|value| value.len())
            .unwrap_or(0),
        removed_endpoints_count: payload
            .removed_endpoints
            .as_ref()
            .map(|value| value.len())
            .unwrap_or(0),
        sample_endpoints,
        error_message: payload.error,
    }
}

fn observation_to_detail(
    observation: &SurfaceObservationRow,
    payload: ApiInventoryObservationPayload,
) -> ApiInventoryTargetDetail {
    let metadata = parse_metadata(&observation.metadata_json);
    let snapshot = payload.snapshot;
    let endpoints = snapshot
        .as_ref()
        .map(|value| value.endpoints.clone())
        .unwrap_or_default();

    ApiInventoryTargetDetail {
        program_id: observation.program_id.clone(),
        base_url: payload.base_url,
        success: payload.success,
        endpoint_count: endpoints.len(),
        graphql_endpoint: snapshot
            .as_ref()
            .and_then(|value| value.graphql_endpoint.clone()),
        open_api_spec: snapshot
            .as_ref()
            .and_then(|value| value.open_api_spec.clone()),
        last_checked: snapshot.map(|value| value.last_checked),
        observed_at: observation.observed_at.clone(),
        run_id: observation.run_id.clone(),
        task_name: task_name_from_metadata(&metadata),
        execution_mode: execution_mode_from_metadata(&metadata),
        endpoints,
        added_endpoints: payload.added_endpoints.unwrap_or_default(),
        removed_endpoints: payload.removed_endpoints.unwrap_or_default(),
        error_message: payload.error,
    }
}

#[tauri::command]
pub async fn bounty_list_api_inventory_targets(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ApiInventoryListFilter>,
) -> Result<ApiInventoryListResponse, String> {
    ensure_bug_bounty_access()?;

    let filter = filter.unwrap_or_default();
    let observations = db_service
        .list_surface_observations_filtered(
            filter.program_id.as_deref(),
            Some(API_MONITOR_PLUGIN_ID),
            Some(API_SNAPSHOT_ARTIFACT_TYPE),
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    let all_rows = collect_latest_targets(observations);
    let total = all_rows.len();
    let mut filtered_rows = apply_inventory_filters(all_rows, &filter);
    sort_inventory_rows(&mut filtered_rows, &filter.sort_by);

    let stats = build_inventory_stats(&filtered_rows);
    let filtered_total = filtered_rows.len();
    let offset = filter.offset.unwrap_or(0).max(0) as usize;
    let limit = filter.limit.unwrap_or(50).max(1) as usize;
    let has_more = offset.saturating_add(limit) < filtered_total;
    let items = filtered_rows.into_iter().skip(offset).take(limit).collect();

    Ok(ApiInventoryListResponse {
        items,
        total,
        filtered_total,
        has_more,
        stats,
    })
}

#[tauri::command]
pub async fn bounty_get_api_inventory_target(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    base_url: String,
) -> Result<Option<ApiInventoryTargetDetail>, String> {
    ensure_bug_bounty_access()?;

    let observations = db_service
        .list_surface_observations_filtered(
            program_id.as_deref(),
            Some(API_MONITOR_PLUGIN_ID),
            Some(API_SNAPSHOT_ARTIFACT_TYPE),
            Some(&base_url),
            Some(20),
        )
        .await
        .map_err(|e| e.to_string())?;

    for observation in observations {
        let Some(payload) = parse_observation_payload(&observation) else {
            continue;
        };
        return Ok(Some(observation_to_detail(&observation, payload)));
    }

    Ok(None)
}

#[tauri::command]
pub async fn bounty_batch_delete_api_inventory_targets(
    db_service: State<'_, Arc<DatabaseService>>,
    targets: Vec<ApiInventoryDeleteTarget>,
) -> Result<usize, String> {
    ensure_bug_bounty_access()?;

    let delete_targets: Vec<(String, String)> = targets
        .into_iter()
        .map(|target| (target.program_id, target.base_url))
        .collect();

    db_service
        .delete_surface_observations_by_targets(
            API_MONITOR_PLUGIN_ID,
            API_SNAPSHOT_ARTIFACT_TYPE,
            &delete_targets,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_list_api_inventory_target_keys(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ApiInventoryListFilter>,
) -> Result<Vec<ApiInventoryDeleteTarget>, String> {
    ensure_bug_bounty_access()?;

    let filter = filter.unwrap_or_default();
    let observations = db_service
        .list_surface_observations_filtered(
            filter.program_id.as_deref(),
            Some(API_MONITOR_PLUGIN_ID),
            Some(API_SNAPSHOT_ARTIFACT_TYPE),
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut filtered_rows = apply_inventory_filters(collect_latest_targets(observations), &filter);
    sort_inventory_rows(&mut filtered_rows, &filter.sort_by);

    Ok(filtered_rows
        .into_iter()
        .map(|target| ApiInventoryDeleteTarget {
            program_id: target.program_id,
            base_url: target.base_url,
        })
        .collect())
}
