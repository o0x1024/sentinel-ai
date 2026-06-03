use serde::{Deserialize, Serialize};
use serde_json::Value;

use sentinel_traffic::HttpRequestRecord;

use crate::services::system_agents::context::build_traffic_context_snapshot;
use crate::services::system_agents::TrafficContextExtractionSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficContextExtractionPreviewSample {
    pub request_id: i64,
    pub method: String,
    pub url: String,
    pub added_principal_keys: Vec<String>,
    pub added_resource_keys: Vec<String>,
    pub added_auth_headers: Vec<String>,
    pub added_auth_tokens: Vec<String>,
    pub added_cookie_keys: Vec<String>,
    pub action_kind_before: Option<String>,
    pub action_kind_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficContextExtractionPreviewResponse {
    pub analyzed_request_count: usize,
    pub changed_request_count: usize,
    pub principal_match_delta_count: usize,
    pub resource_match_delta_count: usize,
    pub auth_header_match_delta_count: usize,
    pub auth_token_match_delta_count: usize,
    pub cookie_match_delta_count: usize,
    pub action_kind_change_count: usize,
    pub samples: Vec<TrafficContextExtractionPreviewSample>,
}

pub fn preview_traffic_context_extraction_changes(
    records: &[HttpRequestRecord],
    current_settings: &TrafficContextExtractionSettings,
    preview_settings: &TrafficContextExtractionSettings,
    sample_limit: usize,
) -> TrafficContextExtractionPreviewResponse {
    let mut changed_request_count = 0usize;
    let mut principal_match_delta_count = 0usize;
    let mut resource_match_delta_count = 0usize;
    let mut auth_header_match_delta_count = 0usize;
    let mut auth_token_match_delta_count = 0usize;
    let mut cookie_match_delta_count = 0usize;
    let mut action_kind_change_count = 0usize;
    let mut samples = Vec::new();

    for record in records {
        let before = build_traffic_context_snapshot(record, &[], current_settings);
        let after = build_traffic_context_snapshot(record, &[], preview_settings);

        let added_principal_keys = added_object_keys(
            before.payload.get("principalContext"),
            after.payload.get("principalContext"),
        );
        let added_resource_keys = added_object_keys(
            before.payload.get("resourceKeys"),
            after.payload.get("resourceKeys"),
        );
        let added_auth_headers = added_match_keys(
            before
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("authHeaderMatches")),
            after
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("authHeaderMatches")),
        );
        let added_auth_tokens = added_match_keys(
            before
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("authTokenMatches")),
            after
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("authTokenMatches")),
        );
        let added_cookie_keys = added_match_keys(
            before
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("cookieMatches")),
            after
                .payload
                .get("contextExtraction")
                .and_then(|value| value.get("cookieMatches")),
        );
        let action_kind_before = before
            .payload
            .get("actionKind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let action_kind_after = after
            .payload
            .get("actionKind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let action_changed = action_kind_before != action_kind_after;
        let has_delta = !added_principal_keys.is_empty()
            || !added_resource_keys.is_empty()
            || !added_auth_headers.is_empty()
            || !added_auth_tokens.is_empty()
            || !added_cookie_keys.is_empty()
            || action_changed;

        if !has_delta {
            continue;
        }

        changed_request_count += 1;
        principal_match_delta_count += added_principal_keys.len();
        resource_match_delta_count += added_resource_keys.len();
        auth_header_match_delta_count += added_auth_headers.len();
        auth_token_match_delta_count += added_auth_tokens.len();
        cookie_match_delta_count += added_cookie_keys.len();
        if action_changed {
            action_kind_change_count += 1;
        }

        if samples.len() < sample_limit {
            samples.push(TrafficContextExtractionPreviewSample {
                request_id: record.id,
                method: record
                    .edited_method
                    .clone()
                    .unwrap_or_else(|| record.method.clone()),
                url: record
                    .edited_url
                    .clone()
                    .unwrap_or_else(|| record.url.clone()),
                added_principal_keys,
                added_resource_keys,
                added_auth_headers,
                added_auth_tokens,
                added_cookie_keys,
                action_kind_before,
                action_kind_after,
            });
        }
    }

    TrafficContextExtractionPreviewResponse {
        analyzed_request_count: records.len(),
        changed_request_count,
        principal_match_delta_count,
        resource_match_delta_count,
        auth_header_match_delta_count,
        auth_token_match_delta_count,
        cookie_match_delta_count,
        action_kind_change_count,
        samples,
    }
}

fn added_object_keys(before: Option<&Value>, after: Option<&Value>) -> Vec<String> {
    let before_keys = before
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let after_keys = after
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    after_keys
        .into_iter()
        .filter(|key| !before_keys.iter().any(|existing| existing == key))
        .collect()
}

fn added_match_keys(before: Option<&Value>, after: Option<&Value>) -> Vec<String> {
    let before_keys = extract_match_keys(before);
    extract_match_keys(after)
        .into_iter()
        .filter(|key| !before_keys.iter().any(|existing| existing == key))
        .collect()
}

fn extract_match_keys(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("matchedKey").and_then(Value::as_str))
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::preview_traffic_context_extraction_changes;
    use crate::services::system_agents::TrafficContextExtractionSettings;
    use chrono::Utc;
    use sentinel_traffic::HttpRequestRecord;

    fn build_record(url: &str) -> HttpRequestRecord {
        HttpRequestRecord {
            id: 1,
            db_request_id: Some(1001),
            traffic_request_id: None,
            origin_kind: None,
            origin_ref_id: None,
            parent_request_id: None,
            source_draft_revision_id: None,
            url: url.to_string(),
            host: "example.com".to_string(),
            scheme: "https".to_string(),
            http_version_observed: Some("HTTP/1.1".to_string()),
            method: "GET".to_string(),
            status_code: 200,
            request_headers: None,
            request_body: None,
            response_headers: None,
            response_body: None,
            response_size: 0,
            response_time: 50,
            timestamp: Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_request_headers: None,
            edited_request_body: None,
            edited_response_headers: None,
            edited_response_body: None,
            edited_status_code: None,
        }
    }

    #[test]
    fn preview_reports_added_resource_key_matches() {
        let record = build_record("https://example.com/api/workflows/detail?fleof=CASE-9");
        let current = TrafficContextExtractionSettings::default();
        let preview = TrafficContextExtractionSettings {
            resource_key_hints: vec!["fleof".to_string()],
            ..TrafficContextExtractionSettings::default()
        };

        let response = preview_traffic_context_extraction_changes(&[record], &current, &preview, 4);

        assert_eq!(response.changed_request_count, 1);
        assert_eq!(response.resource_match_delta_count, 1);
        assert_eq!(response.samples.len(), 1);
        assert_eq!(
            response.samples[0].added_resource_keys,
            vec!["fleof".to_string()]
        );
    }
}
