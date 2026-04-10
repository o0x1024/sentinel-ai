use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};

use sentinel_db::{Database, SystemAgentRunRecord, TrafficEvidenceRecord};
use sentinel_traffic::{EvidenceRecord, VulnerabilityFilters};

use crate::commands::command_response_support::CommandResponse;
use crate::commands::traffic::TrafficAnalysisState;
use crate::commands::security_workbench_storage_support::{
    create_workbench_id, default_baseline_evidence_id, load_finding_snapshot,
    load_workbench_ignored_finding_ids,
    load_workbench_activities, load_workbench_cases, load_workbench_execution_drafts,
    load_workbench_execution_runs, load_workbench_notes, payload_changed,
    refresh_workbench_case_snapshot, save_workbench_activities, save_workbench_cases,
    save_workbench_execution_drafts, save_workbench_execution_runs,
    save_workbench_ignored_finding_ids, save_workbench_notes,
    workbench_priority_for_severity,
};
use crate::services::system_agents::finding_lifecycle::TrafficFindingLifecycle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchFindingSnapshotPayload {
    pub id: String,
    pub title: String,
    pub vuln_type: String,
    pub severity: String,
    pub confidence: String,
    pub status: String,
    pub plugin_id: String,
    pub url: String,
    pub method: Option<String>,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCasePayload {
    pub id: String,
    pub finding_id: String,
    pub title: String,
    pub status: String,
    pub current_conclusion: String,
    pub priority: String,
    pub baseline_evidence_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
    pub finding: WorkbenchFindingSnapshotPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchNotePayload {
    pub id: String,
    pub case_id: String,
    pub kind: String,
    pub body: String,
    pub created_at: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchActivityPayload {
    pub id: String,
    pub case_id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub created_at: String,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseListItemPayload {
    #[serde(flatten)]
    pub case_item: WorkbenchCasePayload,
    pub note_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseListResponsePayload {
    pub items: Vec<WorkbenchCaseListItemPayload>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseDetailPayload {
    pub case_item: WorkbenchCasePayload,
    pub activities: Vec<WorkbenchActivityPayload>,
    pub notes: Vec<WorkbenchNotePayload>,
    pub execution_drafts: Vec<WorkbenchExecutionDraftPayload>,
    pub execution_runs: Vec<WorkbenchExecutionRunPayload>,
    pub verifier_runs: Vec<WorkbenchVerifierRunPayload>,
    pub assessment_suggestion: Option<WorkbenchAssessmentSuggestionPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchFindingSyncResultPayload {
    pub case_id: String,
    pub finding_id: String,
    pub case_status: String,
    pub previous_finding_status: String,
    pub next_finding_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanStepPayload {
    pub id: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanStopConditionPayload {
    pub id: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanPayload {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub strategy: String,
    pub severity: String,
    pub read_only: bool,
    pub target_evidence_id: String,
    pub target_field: String,
    pub target_method: String,
    pub target_url: String,
    pub candidate_values: Vec<String>,
    pub steps: Vec<WorkbenchReplayPlanStepPayload>,
    pub stop_conditions: Vec<WorkbenchReplayPlanStopConditionPayload>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionDraftPayload {
    pub id: String,
    pub case_id: String,
    pub plan_id: String,
    pub title: String,
    pub status: String,
    pub read_only: bool,
    pub severity: String,
    pub target_evidence_id: String,
    pub target_field: String,
    pub target_method: String,
    pub target_url: String,
    pub candidate_values: Vec<String>,
    pub steps: Vec<WorkbenchReplayPlanStepPayload>,
    pub stop_conditions: Vec<WorkbenchReplayPlanStopConditionPayload>,
    pub rationale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionAttemptPayload {
    pub id: String,
    pub candidate_value: String,
    pub mutated_url: String,
    pub response_status: Option<i32>,
    pub response_snippet: String,
    pub outcome: String,
    pub diff: WorkbenchExecutionDiffPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionDiffPayload {
    pub matched_status: bool,
    pub matched_body: bool,
    pub similarity_level: String,
    pub baseline_status: Option<i32>,
    pub baseline_length: usize,
    pub response_length: usize,
    pub changed_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionRunPayload {
    pub id: String,
    pub case_id: String,
    pub draft_id: String,
    pub status: String,
    pub method: String,
    pub baseline_url: String,
    pub target_field: String,
    pub summary: String,
    pub attempts: Vec<WorkbenchExecutionAttemptPayload>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchVerifierRunPayload {
    pub id: String,
    pub status: String,
    pub trigger_event: Option<String>,
    pub strategy: Option<String>,
    pub verified: bool,
    pub response_status: Option<i32>,
    pub summary: String,
    pub evidence_id: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAssessmentSuggestionPayload {
    pub title: String,
    pub summary: String,
    pub suggested_status: String,
    pub suggested_conclusion: String,
    pub confidence: String,
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListSecurityWorkbenchCasesRequest {
    pub search: Option<String>,
    pub status: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOrCreateSecurityWorkbenchCaseForFindingRequest {
    pub finding_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityWorkbenchCaseDetailRequest {
    pub case_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchCasePatch {
    pub status: Option<String>,
    pub current_conclusion: Option<String>,
    pub priority: Option<String>,
    pub baseline_evidence_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchCaseRequest {
    pub case_id: String,
    pub patch: UpdateSecurityWorkbenchCasePatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSecurityWorkbenchNoteRequest {
    pub case_id: String,
    pub kind: String,
    pub body: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSecurityWorkbenchCaseToFindingRequest {
    pub case_id: String,
    pub apply_suggestion_to_case: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecurityWorkbenchExecutionDraftRequest {
    pub case_id: String,
    pub plan: WorkbenchReplayPlanPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchExecutionDraftRequest {
    pub draft_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteSecurityWorkbenchExecutionDraftRequest {
    pub draft_id: String,
    pub confirm_non_readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSecurityWorkbenchCasesRequest {
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSecurityWorkbenchCasesResultPayload {
    pub deleted_case_ids: Vec<String>,
    pub deleted_case_count: usize,
    pub deleted_note_count: usize,
    pub deleted_activity_count: usize,
    pub deleted_execution_draft_count: usize,
    pub deleted_execution_run_count: usize,
}

fn emit_workbench_changed(
    app_handle: &AppHandle,
    reason: &str,
    case_ids: &[String],
    finding_ids: &[String],
) {
    let _ = app_handle.emit(
        "security-workbench:changed",
        json!({
            "reason": reason,
            "caseIds": case_ids,
            "findingIds": finding_ids,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );
}

fn emit_finding_updated(
    app_handle: &AppHandle,
    reason: &str,
    finding_ids: &[String],
) {
    let _ = app_handle.emit(
        "scan:finding-updated",
        json!({
            "reason": reason,
            "findingIds": finding_ids,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );
}

fn normalize_case_status(value: Option<&str>) -> String {
    match value.unwrap_or("new").trim() {
        "new" => "new".to_string(),
        "investigating" => "investigating".to_string(),
        "awaiting_verification" => "awaiting_verification".to_string(),
        "verified" => "verified".to_string(),
        "false_positive" => "false_positive".to_string(),
        "archived" => "archived".to_string(),
        _ => "new".to_string(),
    }
}

fn normalize_priority(value: Option<&str>) -> String {
    match value.unwrap_or("medium").trim() {
        "low" => "low".to_string(),
        "medium" => "medium".to_string(),
        "high" => "high".to_string(),
        _ => "medium".to_string(),
    }
}

fn normalize_execution_draft_status(value: Option<&str>) -> String {
    match value.unwrap_or("draft").trim() {
        "draft" => "draft".to_string(),
        "ready" => "ready".to_string(),
        "paused" => "paused".to_string(),
        "archived" => "archived".to_string(),
        _ => "draft".to_string(),
    }
}

fn normalize_execution_run_status(value: Option<&str>) -> String {
    match value.unwrap_or("completed").trim() {
        "completed" => "completed".to_string(),
        "blocked" => "blocked".to_string(),
        "failed" => "failed".to_string(),
        _ => "failed".to_string(),
    }
}

fn normalize_attempt_outcome(value: &str) -> String {
    match value.trim() {
        "changed" => "changed".to_string(),
        "same" => "same".to_string(),
        "blocked" => "blocked".to_string(),
        "not_found" => "not_found".to_string(),
        _ => "error".to_string(),
    }
}

fn normalize_similarity_level(value: &str) -> String {
    match value.trim() {
        "high" => "high".to_string(),
        "medium" => "medium".to_string(),
        _ => "low".to_string(),
    }
}

fn normalize_confidence(value: &str) -> String {
    match value.trim() {
        "high" => "high".to_string(),
        "medium" => "medium".to_string(),
        _ => "low".to_string(),
    }
}

fn map_case_status_to_finding_status(case_status: &str, current_finding_status: &str) -> String {
    match case_status {
        "new" | "investigating" | "awaiting_verification" => "candidate".to_string(),
        "verified" => "reviewed".to_string(),
        "false_positive" => "false_positive".to_string(),
        // 归档不是漏洞生命周期概念，保留当前 finding 状态。
        "archived" => current_finding_status.to_string(),
        _ => current_finding_status.to_string(),
    }
}

fn parse_header_map(raw: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let Some(raw) = raw else {
        return headers;
    };

    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return headers;
    };
    let Some(object) = value.as_object() else {
        return headers;
    };

    for (key, value) in object {
        let header_name = key.trim().to_ascii_lowercase();
        if matches!(header_name.as_str(), "host" | "content-length") {
            continue;
        }
        let Some(raw_value) = value.as_str() else {
            continue;
        };
        let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) else {
            continue;
        };
        let Ok(parsed_value) = HeaderValue::from_str(raw_value) else {
            continue;
        };
        headers.insert(name, parsed_value);
    }

    headers
}

fn normalize_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(512)
        .collect()
}

fn compare_response_body(expected: Option<&str>, actual: &str) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    let normalized_expected = normalize_text(expected);
    let normalized_actual = normalize_text(actual);
    if normalized_expected.is_empty() || normalized_actual.is_empty() {
        return false;
    }
    if normalized_expected == normalized_actual {
        return true;
    }

    let prefix_chars = normalized_expected
        .chars()
        .zip(normalized_actual.chars())
        .take(256)
        .take_while(|(left, right)| left == right)
        .count();
    prefix_chars >= 64
}

fn truncate_body(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn parse_form_pairs(input: &str) -> Vec<(String, String)> {
    input
        .split('&')
        .filter_map(|item| {
            let mut segments = item.splitn(2, '=');
            let key = segments.next()?.trim();
            let value = segments.next().unwrap_or_default().trim();
            if key.is_empty() {
                return None;
            }
            let decoded_key = urlencoding::decode(key).ok()?.into_owned();
            let decoded_value = urlencoding::decode(value).ok()?.into_owned();
            Some((decoded_key, decoded_value))
        })
        .collect()
}

fn build_form_body(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn parse_json_value(input: &str) -> Option<Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() || (!trimmed.starts_with('{') && !trimmed.starts_with('[')) {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn extract_system_agent_run_finding_id(run: &SystemAgentRunRecord) -> Option<String> {
    let input_finding_id = parse_json_value(&run.input_summary_json).and_then(|value| {
        value
            .get("findingId")
            .or_else(|| value.get("finding_id"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    if input_finding_id.is_some() {
        return input_finding_id;
    }

    run.output_json
        .as_deref()
        .and_then(parse_json_value)
        .and_then(|value| {
            value
                .get("findingId")
                .or_else(|| value.get("finding_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn map_verifier_run_payload(run: &SystemAgentRunRecord) -> WorkbenchVerifierRunPayload {
    let output = run.output_json.as_deref().and_then(parse_json_value);
    let summary = output
        .as_ref()
        .and_then(|value| value.get("summary").and_then(Value::as_str))
        .map(str::to_string)
        .or_else(|| run.error_message.clone())
        .unwrap_or_else(|| "System Agent verifier run completed.".to_string());

    WorkbenchVerifierRunPayload {
        id: run.id.clone(),
        status: run.status.clone(),
        trigger_event: run.trigger_event.clone(),
        strategy: output
            .as_ref()
            .and_then(|value| value.get("strategy").and_then(Value::as_str))
            .map(str::to_string),
        verified: output
            .as_ref()
            .and_then(|value| value.get("verified").and_then(Value::as_bool))
            .unwrap_or(false),
        response_status: output
            .as_ref()
            .and_then(|value| value.get("responseStatus").and_then(Value::as_i64))
            .and_then(|value| i32::try_from(value).ok()),
        summary,
        evidence_id: output
            .as_ref()
            .and_then(|value| value.get("evidenceId").and_then(Value::as_str))
            .map(str::to_string),
        error_message: run.error_message.clone(),
        started_at: run.started_at.to_rfc3339(),
        finished_at: run.finished_at.map(|value| value.to_rfc3339()),
    }
}

fn collect_changed_paths(
    left: &Value,
    right: &Value,
    prefix: &str,
    output: &mut Vec<String>,
    limit: usize,
) {
    if output.len() >= limit {
        return;
    }

    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            let mut keys = left_map.keys().cloned().collect::<Vec<_>>();
            for key in right_map.keys() {
                if !keys.iter().any(|existing| existing == key) {
                    keys.push(key.clone());
                }
            }
            keys.sort();
            keys.dedup();

            for key in keys {
                if output.len() >= limit {
                    break;
                }
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                match (left_map.get(&key), right_map.get(&key)) {
                    (Some(left_item), Some(right_item)) => {
                        collect_changed_paths(left_item, right_item, &next_prefix, output, limit);
                    }
                    _ => output.push(next_prefix),
                }
            }
        }
        (Value::Array(left_items), Value::Array(right_items)) => {
            let min_len = left_items.len().min(right_items.len());
            for index in 0..min_len {
                if output.len() >= limit {
                    break;
                }
                let next_prefix = if prefix.is_empty() {
                    format!("[{index}]")
                } else {
                    format!("{prefix}[{index}]")
                };
                collect_changed_paths(
                    &left_items[index],
                    &right_items[index],
                    &next_prefix,
                    output,
                    limit,
                );
            }
            if left_items.len() != right_items.len() && output.len() < limit {
                output.push(if prefix.is_empty() {
                    "[] length".to_string()
                } else {
                    format!("{prefix} length")
                });
            }
        }
        _ => {
            if left != right {
                output.push(if prefix.is_empty() {
                    "<root>".to_string()
                } else {
                    prefix.to_string()
                });
            }
        }
    }
}

fn analyze_response_difference(
    baseline_status: Option<i32>,
    baseline_body: Option<&str>,
    response_status: i32,
    response_body: &str,
) -> WorkbenchExecutionDiffPayload {
    let matched_status = baseline_status == Some(response_status);
    let matched_body = compare_response_body(baseline_body, response_body);
    let baseline_length = baseline_body.unwrap_or_default().chars().count();
    let response_length = response_body.chars().count();
    let ratio = if baseline_length == 0 || response_length == 0 {
        0.0
    } else {
        let min = baseline_length.min(response_length) as f64;
        let max = baseline_length.max(response_length) as f64;
        min / max
    };

    let similarity_level = if matched_body || ratio >= 0.9 {
        "high"
    } else if ratio >= 0.6 {
        "medium"
    } else {
        "low"
    };

    let mut changed_signals = Vec::new();
    if !matched_status {
        changed_signals.push(format!(
            "状态码变化: {} -> {}",
            baseline_status
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            response_status
        ));
    }

    if baseline_length != response_length {
        changed_signals.push(format!(
            "响应长度变化: {} -> {}",
            baseline_length, response_length
        ));
    }

    if let Some(baseline_body) = baseline_body {
        if let (Some(left), Some(right)) = (
            parse_json_value(baseline_body),
            parse_json_value(response_body),
        ) {
            let mut paths = Vec::new();
            collect_changed_paths(&left, &right, "", &mut paths, 6);
            if !paths.is_empty() {
                changed_signals.push(format!("字段变化: {}", paths.join(", ")));
            }
        } else if !matched_body {
            changed_signals.push("响应文本出现明显变化".to_string());
        }
    }

    WorkbenchExecutionDiffPayload {
        matched_status,
        matched_body,
        similarity_level: normalize_similarity_level(similarity_level),
        baseline_status,
        baseline_length,
        response_length,
        changed_signals,
    }
}

#[derive(Debug, Clone)]
struct MutatedRequest {
    url: String,
    body: Option<String>,
}

#[derive(Debug, Clone)]
enum BodyPathToken {
    Key(String),
    Index(usize),
}

fn parse_body_path_tokens(path: &str) -> Vec<BodyPathToken> {
    let mut tokens = Vec::new();
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        let mut rest = segment;
        loop {
            if let Some(index_start) = rest.find('[') {
                let key = &rest[..index_start];
                if !key.is_empty() {
                    tokens.push(BodyPathToken::Key(key.to_string()));
                }
                let Some(index_end) = rest[index_start + 1..].find(']') else {
                    break;
                };
                let raw_index = &rest[index_start + 1..index_start + 1 + index_end];
                if let Ok(index) = raw_index.parse::<usize>() {
                    tokens.push(BodyPathToken::Index(index));
                }
                let next_offset = index_start + index_end + 2;
                if next_offset >= rest.len() {
                    break;
                }
                rest = &rest[next_offset..];
            } else {
                if !rest.is_empty() {
                    tokens.push(BodyPathToken::Key(rest.to_string()));
                }
                break;
            }
        }
    }
    tokens
}

fn set_json_value_by_tokens(
    value: &mut Value,
    tokens: &[BodyPathToken],
    candidate_value: &str,
) -> bool {
    if tokens.is_empty() {
        return false;
    }

    match &tokens[0] {
        BodyPathToken::Key(key) => {
            let Value::Object(map) = value else {
                return false;
            };
            let Some(next) = map.get_mut(key) else {
                return false;
            };
            if tokens.len() == 1 {
                *next = Value::String(candidate_value.to_string());
                return true;
            }
            set_json_value_by_tokens(next, &tokens[1..], candidate_value)
        }
        BodyPathToken::Index(index) => {
            let Value::Array(items) = value else {
                return false;
            };
            let Some(next) = items.get_mut(*index) else {
                return false;
            };
            if tokens.len() == 1 {
                *next = Value::String(candidate_value.to_string());
                return true;
            }
            set_json_value_by_tokens(next, &tokens[1..], candidate_value)
        }
    }
}

fn mutate_body_for_candidate(
    request_body: Option<&str>,
    target_field: &str,
    candidate_value: &str,
) -> Result<Option<String>, String> {
    let Some(body_field) = target_field.strip_prefix("body.") else {
        return Ok(request_body.map(|item| item.to_string()));
    };
    let Some(request_body) = request_body else {
        return Err("Baseline request body is empty".to_string());
    };

    if let Some(mut json) = parse_json_value(request_body) {
        let tokens = parse_body_path_tokens(body_field);
        if tokens.is_empty() {
            return Err(format!("Unsupported body target field: {target_field}"));
        }
        if !set_json_value_by_tokens(&mut json, &tokens, candidate_value) {
            return Err(format!(
                "Body path not found in JSON payload: {target_field}"
            ));
        }
        return serde_json::to_string(&json)
            .map(Some)
            .map_err(|error| format!("Failed to serialize mutated JSON body: {error}"));
    }

    let mut pairs = parse_form_pairs(request_body);
    if !pairs.is_empty() {
        let mut replaced = false;
        for (key, value) in &mut pairs {
            if key == body_field {
                *value = candidate_value.to_string();
                replaced = true;
            }
        }
        if !replaced {
            pairs.push((body_field.to_string(), candidate_value.to_string()));
        }
        return Ok(Some(build_form_body(&pairs)));
    }

    Err(format!(
        "Only JSON and form body payloads are supported for read-only execution: {target_field}"
    ))
}

fn mutate_request_for_candidate(
    target_url: &str,
    request_body: Option<&str>,
    target_field: &str,
    candidate_value: &str,
) -> Result<MutatedRequest, String> {
    let mut url = reqwest::Url::parse(target_url)
        .map_err(|error| format!("Target URL is not absolute: {error}"))?;

    if let Some(query_key) = target_field.strip_prefix("query.") {
        let mut replaced = false;
        let pairs = url
            .query_pairs()
            .map(|(key, value)| {
                if key == query_key {
                    replaced = true;
                    (key.into_owned(), candidate_value.to_string())
                } else {
                    (key.into_owned(), value.into_owned())
                }
            })
            .collect::<Vec<_>>();
        url.query_pairs_mut().clear();
        for (key, value) in pairs {
            url.query_pairs_mut().append_pair(&key, &value);
        }
        if !replaced {
            url.query_pairs_mut()
                .append_pair(query_key, candidate_value);
        }
        return Ok(MutatedRequest {
            url: url.to_string(),
            body: request_body.map(|item| item.to_string()),
        });
    }

    if let Some(path_hint) = target_field.strip_prefix("path.") {
        let mut segments = url
            .path_segments()
            .map(|items| items.map(|item| item.to_string()).collect::<Vec<_>>())
            .ok_or_else(|| "Target URL does not support path mutation".to_string())?;

        let mut replaced = false;
        for index in 1..segments.len() {
            if segments[index - 1].eq_ignore_ascii_case(path_hint) {
                segments[index] = candidate_value.to_string();
                replaced = true;
                break;
            }
        }

        if !replaced {
            return Err(format!("Unsupported path target field: {target_field}"));
        }

        let next_path = format!("/{}", segments.join("/"));
        url.set_path(&next_path);
        return Ok(MutatedRequest {
            url: url.to_string(),
            body: request_body.map(|item| item.to_string()),
        });
    }

    if target_field.starts_with("body.") {
        return Ok(MutatedRequest {
            url: url.to_string(),
            body: mutate_body_for_candidate(request_body, target_field, candidate_value)?,
        });
    }

    Err(format!(
        "Only path/query/body targets are supported for read-only execution: {target_field}"
    ))
}

async fn execute_readonly_attempt(
    client: &Client,
    draft: &WorkbenchExecutionDraftPayload,
    baseline: &TrafficEvidenceRecord,
    candidate_value: &str,
) -> WorkbenchExecutionAttemptPayload {
    let mutated_request = match mutate_request_for_candidate(
        &draft.target_url,
        baseline.request_body.as_deref(),
        &draft.target_field,
        candidate_value,
    ) {
        Ok(request) => request,
        Err(error) => {
            return WorkbenchExecutionAttemptPayload {
                id: create_workbench_id("attempt"),
                candidate_value: candidate_value.to_string(),
                mutated_url: draft.target_url.clone(),
                response_status: None,
                response_snippet: error,
                outcome: "error".to_string(),
                diff: WorkbenchExecutionDiffPayload {
                    matched_status: false,
                    matched_body: false,
                    similarity_level: "low".to_string(),
                    baseline_status: baseline.response_status,
                    baseline_length: baseline
                        .response_body
                        .as_deref()
                        .unwrap_or_default()
                        .chars()
                        .count(),
                    response_length: 0,
                    changed_signals: vec!["目标字段不支持自动只读变异".to_string()],
                },
            };
        }
    };

    let method = match Method::from_bytes(draft.target_method.as_bytes()) {
        Ok(method) => method,
        Err(error) => {
            return WorkbenchExecutionAttemptPayload {
                id: create_workbench_id("attempt"),
                candidate_value: candidate_value.to_string(),
                mutated_url: mutated_request.url.clone(),
                response_status: None,
                response_snippet: format!("Invalid HTTP method: {error}"),
                outcome: "error".to_string(),
                diff: WorkbenchExecutionDiffPayload {
                    matched_status: false,
                    matched_body: false,
                    similarity_level: "low".to_string(),
                    baseline_status: baseline.response_status,
                    baseline_length: baseline
                        .response_body
                        .as_deref()
                        .unwrap_or_default()
                        .chars()
                        .count(),
                    response_length: 0,
                    changed_signals: vec!["HTTP 方法无效".to_string()],
                },
            };
        }
    };

    let headers = parse_header_map(baseline.request_headers.as_deref());
    let mut request = client
        .request(method.clone(), &mutated_request.url)
        .headers(headers);
    if !matches!(method.as_str(), "GET" | "HEAD") {
        if let Some(body) = mutated_request.body.clone() {
            request = request.body(body);
        }
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return WorkbenchExecutionAttemptPayload {
                id: create_workbench_id("attempt"),
                candidate_value: candidate_value.to_string(),
                mutated_url: mutated_request.url.clone(),
                response_status: None,
                response_snippet: error.to_string(),
                outcome: "error".to_string(),
                diff: WorkbenchExecutionDiffPayload {
                    matched_status: false,
                    matched_body: false,
                    similarity_level: "low".to_string(),
                    baseline_status: baseline.response_status,
                    baseline_length: baseline
                        .response_body
                        .as_deref()
                        .unwrap_or_default()
                        .chars()
                        .count(),
                    response_length: 0,
                    changed_signals: vec!["请求发送失败".to_string()],
                },
            };
        }
    };

    let status_code = i32::from(response.status().as_u16());
    let body = response.text().await.unwrap_or_default();
    let outcome = if matches!(status_code, 401 | 403) {
        "blocked"
    } else if status_code == 404 {
        "not_found"
    } else if compare_response_body(baseline.response_body.as_deref(), &body)
        || baseline.response_status == Some(status_code)
    {
        "same"
    } else {
        "changed"
    };
    let diff = analyze_response_difference(
        baseline.response_status,
        baseline.response_body.as_deref(),
        status_code,
        &body,
    );

    WorkbenchExecutionAttemptPayload {
        id: create_workbench_id("attempt"),
        candidate_value: candidate_value.to_string(),
        mutated_url: mutated_request.url,
        response_status: Some(status_code),
        response_snippet: truncate_body(&body, 1200),
        outcome: normalize_attempt_outcome(outcome),
        diff,
    }
}

fn build_assessment_changed_signals(attempts: &[WorkbenchExecutionAttemptPayload]) -> Vec<String> {
    let mut signals = Vec::new();
    for attempt in attempts {
        for signal in &attempt.diff.changed_signals {
            if !signal.trim().is_empty() && !signals.iter().any(|item| item == signal) {
                signals.push(signal.clone());
            }
            if signals.len() >= 4 {
                return signals;
            }
        }
    }
    signals
}

fn build_workbench_assessment_suggestion(
    case_item: &WorkbenchCasePayload,
    execution_runs: &[WorkbenchExecutionRunPayload],
) -> Option<WorkbenchAssessmentSuggestionPayload> {
    if execution_runs.is_empty() {
        return None;
    }

    let attempts = execution_runs
        .iter()
        .flat_map(|run| run.attempts.iter().cloned())
        .collect::<Vec<_>>();

    if attempts.is_empty() {
        return Some(WorkbenchAssessmentSuggestionPayload {
            title: "建议继续人工分析".to_string(),
            summary: "当前只有阻断或未执行的草案，没有足够的自动执行结果来支持状态推进。".to_string(),
            suggested_status: normalize_case_status(Some("investigating")),
            suggested_conclusion: "工作台已有执行草案，但当前执行结果不足以确认对象边界是否被突破，建议继续补充基线请求和候选对象，再做人工复核。".to_string(),
            confidence: normalize_confidence("low"),
            signals: vec!["当前没有可用于归因的自动执行尝试结果".to_string()],
        });
    }

    let changed_attempts = attempts
        .iter()
        .filter(|item| item.outcome == "changed")
        .cloned()
        .collect::<Vec<_>>();
    let blocked_attempts = attempts
        .iter()
        .filter(|item| item.outcome == "blocked")
        .count();
    let same_attempts = attempts
        .iter()
        .filter(|item| item.outcome == "same")
        .count();
    let error_attempts = attempts
        .iter()
        .filter(|item| item.outcome == "error")
        .count();
    let not_found_attempts = attempts
        .iter()
        .filter(|item| item.outcome == "not_found")
        .count();

    if !changed_attempts.is_empty() {
        let mut signals = vec![format!(
            "{} 次只读替换后返回了变化后的对象结果",
            changed_attempts.len()
        )];
        signals.extend(build_assessment_changed_signals(&changed_attempts));
        return Some(WorkbenchAssessmentSuggestionPayload {
            title: "建议推进到待验证".to_string(),
            summary: "自动执行已经观察到对象边界变化，建议把案件状态推进到待验证，并补充人工确认。".to_string(),
            suggested_status: normalize_case_status(Some("awaiting_verification")),
            suggested_conclusion: [
                format!(
                    "工作台只读执行共观察到 {} 次对象变化响应，当前案件更像对象边界异常，需要人工进一步确认是否构成水平越权。",
                    changed_attempts.len()
                ),
                changed_attempts
                    .first()
                    .map(|item| item.diff.changed_signals.join("；"))
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("关键差异：{value}"))
                    .unwrap_or_default(),
            ]
            .into_iter()
            .filter(|item| !item.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
            confidence: if changed_attempts.len() >= 2 {
                normalize_confidence("high")
            } else {
                normalize_confidence("medium")
            },
            signals,
        });
    }

    if blocked_attempts == attempts.len() {
        return Some(WorkbenchAssessmentSuggestionPayload {
            title: "建议维持调查中".to_string(),
            summary: "自动执行全部被权限阻断，当前更像边界正常生效，但还不足以直接归为误报。".to_string(),
            suggested_status: normalize_case_status(Some("investigating")),
            suggested_conclusion: "当前自动只读执行全部被权限拒绝，暂未观察到对象边界突破。建议继续结合更多对象池样本或其它入口做复核，不建议直接标记为误报。".to_string(),
            confidence: normalize_confidence("medium"),
            signals: vec![format!("{} 次尝试全部被权限阻断", blocked_attempts)],
        });
    }

    if same_attempts > 0 {
        let mut signals = vec![format!("{} 次尝试返回相似响应", same_attempts)];
        if not_found_attempts > 0 {
            signals.push(format!("{} 次尝试返回资源不存在", not_found_attempts));
        }
        if error_attempts > 0 {
            signals.push(format!("{} 次尝试执行异常", error_attempts));
        }

        return Some(WorkbenchAssessmentSuggestionPayload {
            title: "建议继续调查".to_string(),
            summary: "当前自动执行未观察到明显对象变化，但存在相似响应，建议继续补充样本或更换验证入口。".to_string(),
            suggested_status: normalize_case_status(Some("investigating")),
            suggested_conclusion: "工作台自动执行暂未观察到明确的对象边界突破，当前结果更接近相似响应或无显著差异。建议继续补充候选对象、切换入口或结合人工验证判断。".to_string(),
            confidence: if same_attempts >= 2 {
                normalize_confidence("medium")
            } else {
                normalize_confidence("low")
            },
            signals,
        });
    }

    let mut signals = Vec::new();
    if not_found_attempts > 0 {
        signals.push(format!("{} 次尝试返回资源不存在", not_found_attempts));
    }
    if error_attempts > 0 {
        signals.push(format!("{} 次尝试执行异常", error_attempts));
    }

    Some(WorkbenchAssessmentSuggestionPayload {
        title: "建议保持调查中".to_string(),
        summary: "当前自动执行结果偏弱，建议继续补更多执行样本后再推进案件状态。".to_string(),
        suggested_status: if case_item.status == "new" {
            normalize_case_status(Some("investigating"))
        } else {
            normalize_case_status(Some(&case_item.status))
        },
        suggested_conclusion: "当前工作台自动执行结果不足以形成稳定结论，建议继续补充可替换对象、基线请求和相关证据，再决定是否推进状态或回写漏洞。".to_string(),
        confidence: normalize_confidence("low"),
        signals,
    })
}

async fn append_workbench_activity(
    state: &TrafficAnalysisState,
    case_id: &str,
    kind: &str,
    title: impl Into<String>,
    summary: impl Into<String>,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<WorkbenchActivityPayload, String> {
    let next_activity = WorkbenchActivityPayload {
        id: create_workbench_id("activity"),
        case_id: case_id.to_string(),
        kind: kind.to_string(),
        title: title.into(),
        summary: summary.into(),
        before,
        after,
        created_at: chrono::Utc::now().to_rfc3339(),
        actor: "系统".to_string(),
    };

    let mut activities = load_workbench_activities(state).await?;
    activities.insert(0, next_activity.clone());
    save_workbench_activities(state, &activities).await?;
    Ok(next_activity)
}

async fn delete_workbench_cases_by_ids(
    state: &TrafficAnalysisState,
    case_ids: &[String],
) -> Result<DeleteSecurityWorkbenchCasesResultPayload, String> {
    let normalized_case_ids = case_ids
        .iter()
        .map(|case_id| case_id.trim())
        .filter(|case_id| !case_id.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if normalized_case_ids.is_empty() {
        return Ok(DeleteSecurityWorkbenchCasesResultPayload {
            deleted_case_ids: Vec::new(),
            deleted_case_count: 0,
            deleted_note_count: 0,
            deleted_activity_count: 0,
            deleted_execution_draft_count: 0,
            deleted_execution_run_count: 0,
        });
    }

    let mut cases = load_workbench_cases(state).await?;
    let deleted_cases = cases
        .iter()
        .filter(|item| normalized_case_ids.iter().any(|case_id| case_id == &item.id))
        .cloned()
        .collect::<Vec<_>>();
    let deleted_case_ids = deleted_cases
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    if deleted_case_ids.is_empty() {
        return Ok(DeleteSecurityWorkbenchCasesResultPayload {
            deleted_case_ids: Vec::new(),
            deleted_case_count: 0,
            deleted_note_count: 0,
            deleted_activity_count: 0,
            deleted_execution_draft_count: 0,
            deleted_execution_run_count: 0,
        });
    }

    cases.retain(|item| !deleted_case_ids.iter().any(|case_id| case_id == &item.id));

    let mut ignored_finding_ids = load_workbench_ignored_finding_ids(state).await?;
    for finding_id in deleted_cases.iter().map(|item| item.finding_id.clone()) {
        if !ignored_finding_ids.iter().any(|item| item == &finding_id) {
            ignored_finding_ids.push(finding_id);
        }
    }

    let mut notes = load_workbench_notes(state).await?;
    let notes_before = notes.len();
    notes.retain(|item| !deleted_case_ids.iter().any(|case_id| case_id == &item.case_id));

    let mut activities = load_workbench_activities(state).await?;
    let activities_before = activities.len();
    activities.retain(|item| !deleted_case_ids.iter().any(|case_id| case_id == &item.case_id));

    let mut drafts = load_workbench_execution_drafts(state).await?;
    let drafts_before = drafts.len();
    drafts.retain(|item| !deleted_case_ids.iter().any(|case_id| case_id == &item.case_id));

    let mut runs = load_workbench_execution_runs(state).await?;
    let runs_before = runs.len();
    runs.retain(|item| !deleted_case_ids.iter().any(|case_id| case_id == &item.case_id));

    save_workbench_cases(state, &cases).await?;
    save_workbench_notes(state, &notes).await?;
    save_workbench_activities(state, &activities).await?;
    save_workbench_execution_drafts(state, &drafts).await?;
    save_workbench_execution_runs(state, &runs).await?;
    save_workbench_ignored_finding_ids(state, &ignored_finding_ids).await?;

    Ok(DeleteSecurityWorkbenchCasesResultPayload {
        deleted_case_count: deleted_case_ids.len(),
        deleted_case_ids,
        deleted_note_count: notes_before.saturating_sub(notes.len()),
        deleted_activity_count: activities_before.saturating_sub(activities.len()),
        deleted_execution_draft_count: drafts_before.saturating_sub(drafts.len()),
        deleted_execution_run_count: runs_before.saturating_sub(runs.len()),
    })
}

fn default_workbench_case_status_from_finding_status(status: &str) -> String {
    match status {
        "false_positive" => "false_positive".to_string(),
        _ => "new".to_string(),
    }
}

async fn intake_non_formal_findings_into_workbench_cases(
    state: &TrafficAnalysisState,
    cases: &mut Vec<WorkbenchCasePayload>,
) -> Result<bool, String> {
    let existing_finding_ids = cases
        .iter()
        .map(|item| item.finding_id.clone())
        .collect::<std::collections::HashSet<_>>();

    let formal_statuses = TrafficFindingLifecycle::formal_statuses();
    let ignored_finding_ids = load_workbench_ignored_finding_ids(state)
        .await?
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let findings = state
        .get_db_service()
        .list_traffic_vulnerabilities(VulnerabilityFilters::default())
        .await
        .map_err(|error| format!("Failed to load workbench intake findings: {error}"))?;

    let mut changed = false;
    for finding in findings {
        if formal_statuses
            .iter()
            .any(|formal_status| formal_status == &finding.status)
        {
            continue;
        }
        if existing_finding_ids.contains(&finding.id) {
            continue;
        }
        if ignored_finding_ids.contains(&finding.id) {
            continue;
        }

        let Some(snapshot) = load_finding_snapshot(state, &finding.id).await? else {
            continue;
        };

        let now = chrono::Utc::now().to_rfc3339();
        cases.push(WorkbenchCasePayload {
            id: create_workbench_id("case"),
            finding_id: finding.id.clone(),
            title: snapshot.title.clone(),
            status: default_workbench_case_status_from_finding_status(&finding.status),
            current_conclusion: String::new(),
            priority: workbench_priority_for_severity(&snapshot.severity),
            baseline_evidence_id: default_baseline_evidence_id(&snapshot),
            created_at: now.clone(),
            updated_at: now.clone(),
            last_activity_at: now,
            finding: snapshot,
        });
        changed = true;
    }

    if changed {
        cases.sort_by(|left, right| right.last_activity_at.cmp(&left.last_activity_at));
    }

    Ok(changed)
}

#[tauri::command]
pub async fn security_workbench_list_cases(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: Option<ListSecurityWorkbenchCasesRequest>,
) -> Result<CommandResponse<WorkbenchCaseListResponsePayload>, String> {
    let request = request.unwrap_or_default();
    let search = request.search.unwrap_or_default().trim().to_lowercase();
    let status_filter = request.status.unwrap_or_default().trim().to_string();
    let page_size = request.page_size.unwrap_or(20).max(1);
    let page = request.page.unwrap_or(1).max(1);

    let mut raw_cases = load_workbench_cases(&state).await?;
    let intake_changed = intake_non_formal_findings_into_workbench_cases(&state, &mut raw_cases).await?;
    let notes = load_workbench_notes(&state).await?;

    let mut refreshed_cases = Vec::with_capacity(raw_cases.len());
    for case_item in &raw_cases {
        refreshed_cases.push(refresh_workbench_case_snapshot(&state, case_item).await?);
    }

    if intake_changed || payload_changed(&refreshed_cases, &raw_cases) {
        save_workbench_cases(&state, &refreshed_cases).await?;
        if intake_changed {
            let case_ids = refreshed_cases
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>();
            let finding_ids = refreshed_cases
                .iter()
                .map(|item| item.finding_id.clone())
                .collect::<Vec<_>>();
            emit_workbench_changed(&app_handle, "intake", &case_ids, &finding_ids);
        }
    }

    let mut items = refreshed_cases
        .into_iter()
        .filter(|item| {
            if !status_filter.is_empty() && item.status != status_filter {
                return false;
            }

            if search.is_empty() {
                return true;
            }

            let haystack = [
                item.title.as_str(),
                item.finding.title.as_str(),
                item.finding.vuln_type.as_str(),
                item.finding.url.as_str(),
                item.current_conclusion.as_str(),
            ]
            .join("\n")
            .to_lowercase();

            haystack.contains(&search)
        })
        .map(|case_item| WorkbenchCaseListItemPayload {
            note_count: notes
                .iter()
                .filter(|note| note.case_id == case_item.id)
                .count(),
            case_item,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .case_item
            .last_activity_at
            .cmp(&left.case_item.last_activity_at)
    });

    let total = items.len();
    let start = page_size.saturating_mul(page.saturating_sub(1));
    let paged_items = items
        .into_iter()
        .skip(start)
        .take(page_size)
        .collect::<Vec<_>>();

    Ok(CommandResponse::ok(WorkbenchCaseListResponsePayload {
        items: paged_items,
        total,
        page,
        page_size,
    }))
}

#[tauri::command]
pub async fn security_workbench_get_or_create_case_for_finding(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: GetOrCreateSecurityWorkbenchCaseForFindingRequest,
) -> Result<CommandResponse<WorkbenchCasePayload>, String> {
    let snapshot = load_finding_snapshot(&state, &request.finding_id)
        .await?
        .ok_or_else(|| format!("Finding {} not found", request.finding_id))?;

    let mut ignored_finding_ids = load_workbench_ignored_finding_ids(&state).await?;
    let ignored_before = ignored_finding_ids.len();
    ignored_finding_ids.retain(|finding_id| finding_id != &request.finding_id);
    if ignored_finding_ids.len() != ignored_before {
        save_workbench_ignored_finding_ids(&state, &ignored_finding_ids).await?;
    }

    let mut cases = load_workbench_cases(&state).await?;
    if let Some(index) = cases
        .iter()
        .position(|item| item.finding_id == request.finding_id)
    {
        let mut existing = cases[index].clone();
        existing.title = snapshot.title.clone();
        existing.finding = snapshot.clone();
        existing.priority = workbench_priority_for_severity(&snapshot.severity);
        if existing.baseline_evidence_id.is_none() {
            existing.baseline_evidence_id = default_baseline_evidence_id(&snapshot);
        }
        cases[index] = existing.clone();
        save_workbench_cases(&state, &cases).await?;
        emit_workbench_changed(
            &app_handle,
            "case_updated",
            &[existing.id.clone()],
            &[existing.finding_id.clone()],
        );
        return Ok(CommandResponse::ok(existing));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let next_case = WorkbenchCasePayload {
        id: create_workbench_id("case"),
        finding_id: request.finding_id.clone(),
        title: snapshot.title.clone(),
        status: "new".to_string(),
        current_conclusion: String::new(),
        priority: workbench_priority_for_severity(&snapshot.severity),
        baseline_evidence_id: default_baseline_evidence_id(&snapshot),
        created_at: now.clone(),
        updated_at: now.clone(),
        last_activity_at: now,
        finding: snapshot,
    };

    cases.insert(0, next_case.clone());
    save_workbench_cases(&state, &cases).await?;
    emit_workbench_changed(
        &app_handle,
        "case_created",
        &[next_case.id.clone()],
        &[next_case.finding_id.clone()],
    );

    Ok(CommandResponse::ok(next_case))
}

#[tauri::command]
pub async fn security_workbench_get_case_detail(
    state: State<'_, TrafficAnalysisState>,
    request: GetSecurityWorkbenchCaseDetailRequest,
) -> Result<CommandResponse<Option<WorkbenchCaseDetailPayload>>, String> {
    let mut cases = load_workbench_cases(&state).await?;
    let Some(index) = cases.iter().position(|item| item.id == request.case_id) else {
        return Ok(CommandResponse::ok(None));
    };

    let refreshed = refresh_workbench_case_snapshot(&state, &cases[index]).await?;
    if payload_changed(&refreshed, &cases[index]) {
        cases[index] = refreshed.clone();
        save_workbench_cases(&state, &cases).await?;
    }

    let notes = load_workbench_notes(&state)
        .await?
        .into_iter()
        .filter(|note| note.case_id == request.case_id)
        .collect::<Vec<_>>();

    let activities = load_workbench_activities(&state)
        .await?
        .into_iter()
        .filter(|activity| activity.case_id == request.case_id)
        .collect::<Vec<_>>();

    let execution_drafts = load_workbench_execution_drafts(&state)
        .await?
        .into_iter()
        .filter(|draft| draft.case_id == request.case_id)
        .collect::<Vec<_>>();

    let execution_runs = load_workbench_execution_runs(&state)
        .await?
        .into_iter()
        .filter(|run| run.case_id == request.case_id)
        .collect::<Vec<_>>();
    let verifier_runs = state
        .get_db_service()
        .list_system_agent_runs(Some("traffic_active_verifier"), Some(200))
        .await
        .map_err(|error| format!("Failed to load verifier runs for workbench case: {error}"))?
        .into_iter()
        .filter(|run| extract_system_agent_run_finding_id(run).as_deref() == Some(&refreshed.finding_id))
        .map(|run| map_verifier_run_payload(&run))
        .collect::<Vec<_>>();
    let assessment_suggestion = build_workbench_assessment_suggestion(&refreshed, &execution_runs);

    Ok(CommandResponse::ok(Some(WorkbenchCaseDetailPayload {
        case_item: refreshed,
        activities,
        notes,
        execution_drafts,
        execution_runs,
        verifier_runs,
        assessment_suggestion,
    })))
}

#[tauri::command]
pub async fn security_workbench_update_case(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: UpdateSecurityWorkbenchCaseRequest,
) -> Result<CommandResponse<Option<WorkbenchCasePayload>>, String> {
    let mut cases = load_workbench_cases(&state).await?;
    let Some(index) = cases.iter().position(|item| item.id == request.case_id) else {
        return Ok(CommandResponse::ok(None));
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut next_case = cases[index].clone();
    if let Some(status) = request.patch.status.as_deref() {
        next_case.status = normalize_case_status(Some(status));
    }
    if let Some(current_conclusion) = request.patch.current_conclusion {
        next_case.current_conclusion = current_conclusion.trim().to_string();
    }
    if let Some(priority) = request.patch.priority.as_deref() {
        next_case.priority = normalize_priority(Some(priority));
    }
    if request.patch.baseline_evidence_id.is_some() {
        next_case.baseline_evidence_id = request.patch.baseline_evidence_id;
    }
    next_case.updated_at = now.clone();
    next_case.last_activity_at = now;

    cases[index] = next_case.clone();
    save_workbench_cases(&state, &cases).await?;
    emit_workbench_changed(
        &app_handle,
        "case_updated",
        &[next_case.id.clone()],
        &[next_case.finding_id.clone()],
    );

    Ok(CommandResponse::ok(Some(next_case)))
}

#[tauri::command]
pub async fn security_workbench_delete_cases(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: DeleteSecurityWorkbenchCasesRequest,
) -> Result<CommandResponse<DeleteSecurityWorkbenchCasesResultPayload>, String> {
    let cases = load_workbench_cases(&state).await?;
    let finding_ids = cases
        .iter()
        .filter(|item| request.case_ids.iter().any(|case_id| case_id == &item.id))
        .map(|item| item.finding_id.clone())
        .collect::<Vec<_>>();
    let result = delete_workbench_cases_by_ids(&state, &request.case_ids).await?;
    if !result.deleted_case_ids.is_empty() {
        emit_workbench_changed(
            &app_handle,
            "case_deleted",
            &result.deleted_case_ids,
            &finding_ids,
        );
    }
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn security_workbench_add_note(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: AddSecurityWorkbenchNoteRequest,
) -> Result<CommandResponse<Option<WorkbenchNotePayload>>, String> {
    let trimmed = request.body.trim();
    if trimmed.is_empty() {
        return Ok(CommandResponse::ok(None));
    }

    let mut notes = load_workbench_notes(&state).await?;
    let next_note = WorkbenchNotePayload {
        id: create_workbench_id("note"),
        case_id: request.case_id.clone(),
        kind: request.kind.trim().to_string(),
        body: trimmed.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        author: request
            .author
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "当前用户".to_string()),
    };
    notes.insert(0, next_note.clone());
    save_workbench_notes(&state, &notes).await?;

    let mut cases = load_workbench_cases(&state).await?;
    if let Some(index) = cases.iter().position(|item| item.id == request.case_id) {
        let now = chrono::Utc::now().to_rfc3339();
        cases[index].updated_at = now.clone();
        cases[index].last_activity_at = now;
        save_workbench_cases(&state, &cases).await?;
        emit_workbench_changed(
            &app_handle,
            "note_added",
            &[cases[index].id.clone()],
            &[cases[index].finding_id.clone()],
        );
    }

    Ok(CommandResponse::ok(Some(next_note)))
}

#[tauri::command]
pub async fn security_workbench_create_execution_draft(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: CreateSecurityWorkbenchExecutionDraftRequest,
) -> Result<CommandResponse<Option<WorkbenchExecutionDraftPayload>>, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let next_draft = WorkbenchExecutionDraftPayload {
        id: create_workbench_id("draft"),
        case_id: request.case_id.clone(),
        plan_id: request.plan.id.clone(),
        title: request.plan.title.clone(),
        status: "draft".to_string(),
        read_only: request.plan.read_only,
        severity: request.plan.severity.clone(),
        target_evidence_id: request.plan.target_evidence_id.clone(),
        target_field: request.plan.target_field.clone(),
        target_method: request.plan.target_method.clone(),
        target_url: request.plan.target_url.clone(),
        candidate_values: request.plan.candidate_values.clone(),
        steps: request.plan.steps.clone(),
        stop_conditions: request.plan.stop_conditions.clone(),
        rationale: request.plan.rationale.clone(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let mut drafts = load_workbench_execution_drafts(&state).await?;
    drafts.insert(0, next_draft.clone());
    save_workbench_execution_drafts(&state, &drafts).await?;

    let mut cases = load_workbench_cases(&state).await?;
    if let Some(index) = cases.iter().position(|item| item.id == request.case_id) {
        cases[index].updated_at = now.clone();
        cases[index].last_activity_at = now;
        save_workbench_cases(&state, &cases).await?;
        emit_workbench_changed(
            &app_handle,
            "draft_created",
            &[cases[index].id.clone()],
            &[cases[index].finding_id.clone()],
        );
    }

    Ok(CommandResponse::ok(Some(next_draft)))
}

#[tauri::command]
pub async fn security_workbench_update_execution_draft(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: UpdateSecurityWorkbenchExecutionDraftRequest,
) -> Result<CommandResponse<Option<WorkbenchExecutionDraftPayload>>, String> {
    let mut drafts = load_workbench_execution_drafts(&state).await?;
    let Some(index) = drafts.iter().position(|item| item.id == request.draft_id) else {
        return Ok(CommandResponse::ok(None));
    };

    let now = chrono::Utc::now().to_rfc3339();
    drafts[index].status = normalize_execution_draft_status(Some(&request.status));
    drafts[index].updated_at = now.clone();
    let next_draft = drafts[index].clone();
    save_workbench_execution_drafts(&state, &drafts).await?;

    let mut cases = load_workbench_cases(&state).await?;
    if let Some(case_index) = cases.iter().position(|item| item.id == next_draft.case_id) {
        cases[case_index].updated_at = now.clone();
        cases[case_index].last_activity_at = now;
        save_workbench_cases(&state, &cases).await?;
        emit_workbench_changed(
            &app_handle,
            "draft_updated",
            &[cases[case_index].id.clone()],
            &[cases[case_index].finding_id.clone()],
        );
    }

    Ok(CommandResponse::ok(Some(next_draft)))
}

#[tauri::command]
pub async fn security_workbench_execute_execution_draft(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: ExecuteSecurityWorkbenchExecutionDraftRequest,
) -> Result<CommandResponse<Option<WorkbenchExecutionRunPayload>>, String> {
    let drafts = load_workbench_execution_drafts(&state).await?;
    let Some(draft) = drafts.into_iter().find(|item| item.id == request.draft_id) else {
        return Ok(CommandResponse::ok(None));
    };

    let started_at = chrono::Utc::now().to_rfc3339();
    let completed_at;
    let status;
    let summary;
    let attempts;

    if !draft.read_only && !request.confirm_non_readonly.unwrap_or(false) {
        completed_at = chrono::Utc::now().to_rfc3339();
        status = "blocked".to_string();
        summary = "当前仅支持执行只读草案；该草案需要人工确认，未自动执行。".to_string();
        attempts = Vec::new();
    } else if draft.read_only && !matches!(draft.target_method.as_str(), "GET" | "HEAD" | "POST") {
        completed_at = chrono::Utc::now().to_rfc3339();
        status = "blocked".to_string();
        summary = "当前只允许自动执行 GET/HEAD/POST 的只读请求。".to_string();
        attempts = Vec::new();
    } else {
        let cases = load_workbench_cases(&state).await?;
        let Some(case_item) = cases.iter().find(|item| item.id == draft.case_id) else {
            return Ok(CommandResponse::ok(None));
        };

        let baseline = case_item
            .finding
            .evidence
            .iter()
            .find(|item| item.id == draft.target_evidence_id)
            .cloned()
            .or_else(|| {
                case_item.baseline_evidence_id.as_ref().and_then(|id| {
                    case_item
                        .finding
                        .evidence
                        .iter()
                        .find(|item| &item.id == id)
                        .cloned()
                })
            });

        let Some(baseline) = baseline else {
            completed_at = chrono::Utc::now().to_rfc3339();
            status = "failed".to_string();
            summary = "未找到可用于执行的基线证据。".to_string();
            attempts = Vec::new();
            let run = WorkbenchExecutionRunPayload {
                id: create_workbench_id("run"),
                case_id: draft.case_id.clone(),
                draft_id: draft.id.clone(),
                status: normalize_execution_run_status(Some(&status)),
                method: draft.target_method.clone(),
                baseline_url: draft.target_url.clone(),
                target_field: draft.target_field.clone(),
                summary,
                attempts,
                started_at,
                finished_at: completed_at,
            };

            let mut runs = load_workbench_execution_runs(&state).await?;
            runs.insert(0, run.clone());
            save_workbench_execution_runs(&state, &runs).await?;
            return Ok(CommandResponse::ok(Some(run)));
        };

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|error| format!("Failed to build workbench HTTP client: {error}"))?;

        let mut next_attempts = Vec::new();
        let attempt_limit = if draft.read_only { 5 } else { 1 };
        for candidate_value in draft.candidate_values.iter().take(attempt_limit) {
            next_attempts
                .push(execute_readonly_attempt(&client, &draft, &baseline, candidate_value).await);
        }

        let changed_count = next_attempts
            .iter()
            .filter(|item| item.outcome == "changed")
            .count();
        let blocked_count = next_attempts
            .iter()
            .filter(|item| item.outcome == "blocked")
            .count();
        let error_count = next_attempts
            .iter()
            .filter(|item| item.outcome == "error")
            .count();

        completed_at = chrono::Utc::now().to_rfc3339();
        status = if error_count == next_attempts.len() {
            "failed".to_string()
        } else {
            "completed".to_string()
        };
        summary = if changed_count > 0 {
            if draft.read_only {
                format!(
                    "已执行 {} 次只读替换，其中 {} 次返回了变化后的对象内容。",
                    next_attempts.len(),
                    changed_count
                )
            } else {
                format!(
                    "已执行 {} 次人工确认的非只读替换，其中 {} 次返回了变化后的对象内容。",
                    next_attempts.len(),
                    changed_count
                )
            }
        } else if blocked_count == next_attempts.len() {
            if draft.read_only {
                format!(
                    "已执行 {} 次只读替换，全部被权限拒绝。",
                    next_attempts.len()
                )
            } else {
                format!(
                    "已执行 {} 次人工确认的非只读替换，全部被权限拒绝。",
                    next_attempts.len()
                )
            }
        } else if error_count > 0 {
            if draft.read_only {
                format!(
                    "已执行 {} 次只读替换，{} 次执行异常。",
                    next_attempts.len(),
                    error_count
                )
            } else {
                format!(
                    "已执行 {} 次人工确认的非只读替换，{} 次执行异常。",
                    next_attempts.len(),
                    error_count
                )
            }
        } else {
            if draft.read_only {
                format!(
                    "已执行 {} 次只读替换，暂未观察到明显对象变化。",
                    next_attempts.len()
                )
            } else {
                format!(
                    "已执行 {} 次人工确认的非只读替换，暂未观察到明显对象变化。",
                    next_attempts.len()
                )
            }
        };
        attempts = next_attempts;
    }

    let run = WorkbenchExecutionRunPayload {
        id: create_workbench_id("run"),
        case_id: draft.case_id.clone(),
        draft_id: draft.id.clone(),
        status: normalize_execution_run_status(Some(&status)),
        method: draft.target_method.clone(),
        baseline_url: draft.target_url.clone(),
        target_field: draft.target_field.clone(),
        summary,
        attempts,
        started_at,
        finished_at: completed_at.clone(),
    };

    let mut runs = load_workbench_execution_runs(&state).await?;
    runs.insert(0, run.clone());
    save_workbench_execution_runs(&state, &runs).await?;
    let changed_count = run
        .attempts
        .iter()
        .filter(|item| item.outcome == "changed")
        .count();
    let blocked_count = run
        .attempts
        .iter()
        .filter(|item| item.outcome == "blocked")
        .count();
    let same_count = run
        .attempts
        .iter()
        .filter(|item| item.outcome == "same")
        .count();
    let error_count = run
        .attempts
        .iter()
        .filter(|item| item.outcome == "error")
        .count();
    let not_found_count = run
        .attempts
        .iter()
        .filter(|item| item.outcome == "not_found")
        .count();
    let _ = append_workbench_activity(
        &state,
        &draft.case_id,
        "draft_execution",
        "执行只读草案",
        format!("已执行草案「{}」，结果摘要：{}", draft.title, run.summary),
        Some(json!({
            "draftId": draft.id,
            "draftStatus": draft.status,
            "targetEvidenceId": draft.target_evidence_id,
            "targetMethod": draft.target_method,
            "targetField": draft.target_field,
            "candidateCount": draft.candidate_values.len(),
            "readOnly": draft.read_only,
            "confirmNonReadonly": request.confirm_non_readonly.unwrap_or(false),
        })),
        Some(json!({
            "runId": run.id,
            "runStatus": run.status,
            "attemptCount": run.attempts.len(),
            "changedCount": changed_count,
            "blockedCount": blocked_count,
            "sameCount": same_count,
            "notFoundCount": not_found_count,
            "errorCount": error_count,
        })),
    )
    .await;

    let mut cases = load_workbench_cases(&state).await?;
    if let Some(index) = cases.iter().position(|item| item.id == draft.case_id) {
        cases[index].updated_at = completed_at.clone();
        cases[index].last_activity_at = completed_at;
        save_workbench_cases(&state, &cases).await?;
        emit_workbench_changed(
            &app_handle,
            "draft_executed",
            &[cases[index].id.clone()],
            &[cases[index].finding_id.clone()],
        );
    }

    Ok(CommandResponse::ok(Some(run)))
}

#[tauri::command]
pub async fn security_workbench_sync_case_to_finding(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: SyncSecurityWorkbenchCaseToFindingRequest,
) -> Result<CommandResponse<Option<WorkbenchFindingSyncResultPayload>>, String> {
    let mut cases = load_workbench_cases(&state).await?;
    let Some(index) = cases.iter().position(|item| item.id == request.case_id) else {
        return Ok(CommandResponse::ok(None));
    };

    let case_item = cases[index].clone();
    let execution_runs = load_workbench_execution_runs(&state)
        .await?
        .into_iter()
        .filter(|run| run.case_id == request.case_id)
        .collect::<Vec<_>>();
    let suggestion = build_workbench_assessment_suggestion(&case_item, &execution_runs);
    let mut next_case = case_item.clone();
    if request.apply_suggestion_to_case.unwrap_or(false) {
        if let Some(suggestion) = suggestion {
            next_case.status = normalize_case_status(Some(&suggestion.suggested_status));
            next_case.current_conclusion = suggestion.suggested_conclusion;
        }
    }

    let db = state.get_db_service();
    let finding = db
        .get_traffic_vulnerability_by_id(&next_case.finding_id)
        .await
        .map_err(|error| format!("Failed to fetch finding before workbench sync: {error}"))?;
    let Some(finding) = finding else {
        return Ok(CommandResponse::ok(None));
    };

    let previous_status = finding.status.clone();
    let next_status = map_case_status_to_finding_status(&next_case.status, &previous_status);

    if next_status != previous_status {
        db.update_traffic_vulnerability_status(&next_case.finding_id, &next_status)
            .await
            .map_err(|error| format!("Failed to sync workbench case to finding: {error}"))?;
    }

    let refreshed = refresh_workbench_case_snapshot(&state, &next_case).await?;
    let mut synced_case = refreshed.clone();
    synced_case.finding.status = next_status.clone();
    synced_case.updated_at = chrono::Utc::now().to_rfc3339();
    synced_case.last_activity_at = synced_case.updated_at.clone();
    cases[index] = synced_case;
    save_workbench_cases(&state, &cases).await?;

    let activity_kind = if request.apply_suggestion_to_case.unwrap_or(false) {
        "suggestion_sync"
    } else {
        "finding_sync"
    };
    let activity_title = if request.apply_suggestion_to_case.unwrap_or(false) {
        "按建议回写漏洞"
    } else {
        "回写漏洞状态"
    };
    let activity_summary = if request.apply_suggestion_to_case.unwrap_or(false) {
        format!(
            "已采用建议状态「{}」并同步漏洞状态：{} -> {}。",
            next_case.status, previous_status, next_status
        )
    } else {
        format!(
            "已按当前案件状态同步漏洞状态：{} -> {}。",
            previous_status, next_status
        )
    };
    let _ = append_workbench_activity(
        &state,
        &request.case_id,
        activity_kind,
        activity_title,
        activity_summary,
        Some(json!({
            "caseStatus": case_item.status,
            "caseConclusion": case_item.current_conclusion,
            "findingId": next_case.finding_id,
            "findingStatus": previous_status,
        })),
        Some(json!({
            "caseStatus": next_case.status,
            "caseConclusion": next_case.current_conclusion,
            "findingId": next_case.finding_id,
            "findingStatus": next_status,
            "syncMode": if request.apply_suggestion_to_case.unwrap_or(false) {
                "suggestion"
            } else {
                "manual"
            },
        })),
    )
    .await;

    emit_workbench_changed(
        &app_handle,
        if request.apply_suggestion_to_case.unwrap_or(false) {
            "finding_synced_with_suggestion"
        } else {
            "finding_synced"
        },
        &[request.case_id.clone()],
        &[next_case.finding_id.clone()],
    );
    emit_finding_updated(&app_handle, "status_sync", &[next_case.finding_id.clone()]);

    Ok(CommandResponse::ok(Some(
        WorkbenchFindingSyncResultPayload {
            case_id: request.case_id,
            finding_id: next_case.finding_id,
            case_status: next_case.status,
            previous_finding_status: previous_status,
            next_finding_status: next_status,
        },
    )))
}
