use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use futures::stream::{self, StreamExt};
use reqwest::{header::CONTENT_TYPE, Client, Method, Url};
use sentinel_db::{ApiInventoryEndpointRequestRow, DatabaseService, SurfaceObservationRow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

const API_MONITOR_PLUGIN_ID: &str = "api_monitor";
const API_SNAPSHOT_ARTIFACT_TYPE: &str = "api_snapshot";
const API_ENDPOINT_REQUEST_TIMEOUT_SECS: u64 = 15;
const API_ENDPOINT_REQUEST_CONCURRENCY: usize = 8;
const API_ENDPOINT_REQUEST_BODY_PREVIEW_LIMIT: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryEndpointPayload {
    path: String,
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiInventorySnapshotPayload {
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(rename = "apiEndpoints")]
    api_endpoints: Vec<ApiInventoryEndpointPayload>,
    #[serde(rename = "lastChecked")]
    last_checked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiInventoryObservationPayload {
    #[serde(rename = "baseUrl")]
    base_url: String,
    success: bool,
    snapshot: Option<ApiInventorySnapshotPayload>,
    #[serde(rename = "addedApiEndpoints")]
    added_endpoints: Option<Vec<ApiInventoryEndpointPayload>>,
    #[serde(rename = "removedApiEndpoints")]
    removed_endpoints: Option<Vec<ApiInventoryEndpointPayload>>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInventoryTargetSummary {
    pub program_id: String,
    pub base_url: String,
    pub success: bool,
    pub endpoint_count: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryEndpointRequestItem {
    pub path: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryEndpointRequest {
    pub program_id: String,
    pub base_url: String,
    pub method: String,
    pub endpoints: Vec<ApiInventoryEndpointRequestItem>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInventoryEndpointRequestResult {
    pub path: String,
    pub source: Option<String>,
    pub method: String,
    pub url: String,
    pub success: bool,
    pub status: Option<u16>,
    pub status_text: Option<String>,
    pub duration_ms: u128,
    pub response_bytes: usize,
    pub response_content_type: Option<String>,
    pub body_preview: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
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
    }
}

fn parse_endpoint_request_method(method: &str) -> Result<Method, String> {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        _ => Err("API endpoint requests only support GET or POST".to_string()),
    }
}

fn resolve_endpoint_request_url(base_url: &str, path: &str) -> Result<String, String> {
    let base = Url::parse(base_url).map_err(|e| format!("Invalid base URL: {e}"))?;
    base.join(path)
        .map(|url| url.to_string())
        .map_err(|e| format!("Invalid endpoint path: {e}"))
}

fn body_preview(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    let limit = bytes.len().min(API_ENDPOINT_REQUEST_BODY_PREVIEW_LIMIT);
    Some(String::from_utf8_lossy(&bytes[..limit]).to_string())
}

async fn request_api_inventory_endpoint(
    client: Client,
    method: Method,
    base_url: String,
    item: ApiInventoryEndpointRequestItem,
    body: Option<String>,
) -> ApiInventoryEndpointRequestResult {
    let started = Instant::now();
    let method_label = method.as_str().to_string();
    let url = match resolve_endpoint_request_url(&base_url, &item.path) {
        Ok(url) => url,
        Err(error) => {
            return ApiInventoryEndpointRequestResult {
                path: item.path,
                source: item.source,
                method: method_label,
                url: String::new(),
                success: false,
                status: None,
                status_text: None,
                duration_ms: started.elapsed().as_millis(),
                response_bytes: 0,
                response_content_type: None,
                body_preview: None,
                error: Some(error),
                created_at: Utc::now().to_rfc3339(),
            };
        }
    };

    let mut builder = client.request(method.clone(), &url);
    if method == Method::POST {
        builder = builder.body(body.unwrap_or_default());
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let response_content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            match response.bytes().await {
                Ok(bytes) => ApiInventoryEndpointRequestResult {
                    path: item.path,
                    source: item.source,
                    method: method_label,
                    url,
                    success: true,
                    status: Some(status.as_u16()),
                    status_text: status.canonical_reason().map(str::to_string),
                    duration_ms: started.elapsed().as_millis(),
                    response_bytes: bytes.len(),
                    response_content_type,
                    body_preview: body_preview(&bytes),
                    error: None,
                    created_at: Utc::now().to_rfc3339(),
                },
                Err(error) => ApiInventoryEndpointRequestResult {
                    path: item.path,
                    source: item.source,
                    method: method_label,
                    url,
                    success: false,
                    status: Some(status.as_u16()),
                    status_text: status.canonical_reason().map(str::to_string),
                    duration_ms: started.elapsed().as_millis(),
                    response_bytes: 0,
                    response_content_type,
                    body_preview: None,
                    error: Some(format!("Failed to read response body: {error}")),
                    created_at: Utc::now().to_rfc3339(),
                },
            }
        }
        Err(error) => ApiInventoryEndpointRequestResult {
            path: item.path,
            source: item.source,
            method: method_label,
            url,
            success: false,
            status: None,
            status_text: None,
            duration_ms: started.elapsed().as_millis(),
            response_bytes: 0,
            response_content_type: None,
            body_preview: None,
            error: Some(error.to_string()),
            created_at: Utc::now().to_rfc3339(),
        },
    }
}

fn request_result_to_history_row(
    program_id: &str,
    base_url: &str,
    request_body: Option<&str>,
    result: &ApiInventoryEndpointRequestResult,
) -> ApiInventoryEndpointRequestRow {
    ApiInventoryEndpointRequestRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        base_url: base_url.to_string(),
        endpoint_path: result.path.clone(),
        endpoint_source: result.source.clone(),
        method: result.method.clone(),
        request_url: result.url.clone(),
        success: result.success,
        status_code: result.status.map(i32::from),
        status_text: result.status_text.clone(),
        duration_ms: i64::try_from(result.duration_ms).unwrap_or(i64::MAX),
        response_bytes: i64::try_from(result.response_bytes).unwrap_or(i64::MAX),
        response_content_type: result.response_content_type.clone(),
        body_preview: result.body_preview.clone(),
        error_message: result.error.clone(),
        request_body: request_body.map(str::to_string),
        created_at: Utc::now(),
    }
}

fn history_row_to_request_result(
    row: ApiInventoryEndpointRequestRow,
) -> ApiInventoryEndpointRequestResult {
    ApiInventoryEndpointRequestResult {
        path: row.endpoint_path,
        source: row.endpoint_source,
        method: row.method,
        url: row.request_url,
        success: row.success,
        status: row
            .status_code
            .and_then(|status| u16::try_from(status).ok()),
        status_text: row.status_text,
        duration_ms: row.duration_ms.max(0) as u128,
        response_bytes: row.response_bytes.max(0) as usize,
        response_content_type: row.response_content_type,
        body_preview: row.body_preview,
        error: row.error_message,
        created_at: row.created_at.to_rfc3339(),
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
        .map(|value| value.api_endpoints.as_slice())
        .unwrap_or(&[]);
    let sample_endpoints = endpoints
        .iter()
        .take(8)
        .map(|endpoint| endpoint.path.clone())
        .collect();

    ApiInventoryTargetSummary {
        program_id: observation.program_id.clone(),
        base_url: payload.base_url,
        success: payload.success,
        endpoint_count: endpoints.len(),
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
        .map(|value| value.api_endpoints.clone())
        .unwrap_or_default();

    ApiInventoryTargetDetail {
        program_id: observation.program_id.clone(),
        base_url: payload.base_url,
        success: payload.success,
        endpoint_count: endpoints.len(),
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
    let filter = filter.unwrap_or_default();
    let observations = db_service
        .list_latest_surface_observations_by_target(
            filter.program_id.as_deref(),
            Some(API_MONITOR_PLUGIN_ID),
            Some(API_SNAPSHOT_ARTIFACT_TYPE),
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
pub async fn bounty_request_api_inventory_endpoints(
    db_service: State<'_, Arc<DatabaseService>>,
    request: ApiInventoryEndpointRequest,
) -> Result<Vec<ApiInventoryEndpointRequestResult>, String> {
    ensure_bug_bounty_access()?;

    if request.endpoints.is_empty() {
        return Ok(Vec::new());
    }

    let method = parse_endpoint_request_method(&request.method)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(API_ENDPOINT_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
    let program_id = request.program_id;
    let base_url = request.base_url;
    let body = request.body;

    let results: Vec<ApiInventoryEndpointRequestResult> =
        stream::iter(request.endpoints.into_iter().map(|item| {
            request_api_inventory_endpoint(
                client.clone(),
                method.clone(),
                base_url.clone(),
                item,
                body.clone(),
            )
        }))
        .buffer_unordered(API_ENDPOINT_REQUEST_CONCURRENCY)
        .collect()
        .await;

    let request_body = if method == Method::POST {
        body.as_deref()
    } else {
        None
    };
    for result in &results {
        let row = request_result_to_history_row(&program_id, &base_url, request_body, result);
        db_service
            .create_api_inventory_endpoint_request(&row)
            .await
            .map_err(|e| format!("Failed to persist API endpoint request history: {e}"))?;
    }

    Ok(results)
}

#[tauri::command]
pub async fn bounty_list_api_inventory_endpoint_request_history(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    base_url: String,
    method: String,
    paths: Option<Vec<String>>,
) -> Result<Vec<ApiInventoryEndpointRequestResult>, String> {
    let method = parse_endpoint_request_method(&method)?;
    let rows = db_service
        .list_latest_api_inventory_endpoint_requests(
            &program_id,
            &base_url,
            method.as_str(),
            paths.as_deref(),
            Some(2_000),
        )
        .await
        .map_err(|e| format!("Failed to load API endpoint request history: {e}"))?;

    Ok(rows
        .into_iter()
        .map(history_row_to_request_result)
        .collect())
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
    let filter = filter.unwrap_or_default();
    let observations = db_service
        .list_latest_surface_observations_by_target(
            filter.program_id.as_deref(),
            Some(API_MONITOR_PLUGIN_ID),
            Some(API_SNAPSHOT_ARTIFACT_TYPE),
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
