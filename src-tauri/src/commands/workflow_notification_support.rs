use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};

pub const WORKFLOW_RESULT_SUMMARY_EVENT: &str = "workflow:result-summary";

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResultSummaryEvent {
    pub execution_id: String,
    pub workflow_name: String,
    pub status: String,
    pub findings_count: u32,
    pub asset_count: u32,
    pub errors_count: u32,
    pub completed_at: String,
}

pub fn summarize_workflow_results(
    results: &HashMap<String, Value>,
    errors_count: usize,
) -> Option<(usize, usize)> {
    let mut findings_count = 0usize;
    let mut asset_count = 0usize;

    for value in results.values() {
        visit_result_value(value, None, &mut findings_count, &mut asset_count);
    }

    if findings_count == 0 && asset_count == 0 && errors_count == 0 {
        return None;
    }

    Some((findings_count, asset_count))
}

fn visit_result_value(
    value: &Value,
    key: Option<&str>,
    findings_count: &mut usize,
    asset_count: &mut usize,
) {
    let normalized_key = key.unwrap_or_default().trim().to_ascii_lowercase();

    match value {
        Value::Array(items) => {
            if is_finding_collection_key(&normalized_key) {
                *findings_count += items.len();
                return;
            }
            if is_asset_collection_key(&normalized_key) {
                *asset_count += items.len();
                return;
            }

            for item in items {
                visit_result_value(item, None, findings_count, asset_count);
            }
        }
        Value::Object(map) => {
            let has_finding_marker = map.get("title").and_then(Value::as_str).is_some()
                && (map.get("finding_type").and_then(Value::as_str).is_some()
                    || map
                        .get("vulnerability_type")
                        .and_then(Value::as_str)
                        .is_some()
                    || map.get("severity").and_then(Value::as_str).is_some());

            if has_finding_marker {
                *findings_count += 1;
                return;
            }

            if !map.contains_key("findings") {
                if let Some(raw_count) = map.get("findings_count").and_then(Value::as_u64) {
                    *findings_count += raw_count as usize;
                }
            }

            for (child_key, child_value) in map {
                visit_result_value(
                    child_value,
                    Some(child_key.as_str()),
                    findings_count,
                    asset_count,
                );
            }
        }
        _ => {}
    }
}

fn is_finding_collection_key(key: &str) -> bool {
    matches!(key, "findings")
}

fn is_asset_collection_key(key: &str) -> bool {
    matches!(
        key,
        "assets" | "subdomains" | "domains" | "urls" | "ips" | "hosts" | "live_hosts" | "services"
    )
}

pub fn build_workflow_result_summary_event(
    execution_id: &str,
    workflow_name: &str,
    status: &str,
    findings_count: usize,
    asset_count: usize,
    errors_count: usize,
) -> WorkflowResultSummaryEvent {
    WorkflowResultSummaryEvent {
        execution_id: execution_id.to_string(),
        workflow_name: workflow_name.to_string(),
        status: status.to_string(),
        findings_count: findings_count as u32,
        asset_count: asset_count as u32,
        errors_count: errors_count as u32,
        completed_at: Utc::now().to_rfc3339(),
    }
}

pub fn emit_workflow_result_summary(app: &AppHandle, payload: &WorkflowResultSummaryEvent) {
    let _ = app.emit(WORKFLOW_RESULT_SUMMARY_EVENT, payload);
}
