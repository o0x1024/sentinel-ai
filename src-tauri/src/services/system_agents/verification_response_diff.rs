use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::services::system_agents::verification_plan::VerificationBaseline;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResponseDiffSummary {
    pub response_status_changed: bool,
    pub response_headers_changed: bool,
    pub response_body_changed: bool,
    #[serde(default)]
    pub changed_targets: Vec<ResponseDiffTarget>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseDiffTarget {
    pub location: String,
    pub selector: String,
    pub change_kind: String,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

pub fn build_response_diff_summary(
    baseline: &VerificationBaseline,
    response_status: u16,
    response_headers: &str,
    response_body: &str,
) -> ResponseDiffSummary {
    let baseline_status = baseline.response_status.map(|value| value as u16);
    let mut summary = ResponseDiffSummary {
        response_status_changed: baseline_status != Some(response_status),
        response_headers_changed: baseline.response_headers.as_deref() != Some(response_headers),
        response_body_changed: baseline.response_body.as_deref() != Some(response_body),
        changed_targets: Vec::new(),
        notes: Vec::new(),
    };

    if summary.response_status_changed {
        summary.changed_targets.push(ResponseDiffTarget {
            location: "status".to_string(),
            selector: "status".to_string(),
            change_kind: classify_change_kind(baseline_status.map(|v| v.to_string()).as_deref(), Some(&response_status.to_string())).to_string(),
            before: baseline_status.map(|value| value.to_string()),
            after: Some(response_status.to_string()),
        });
    }

    diff_headers(
        baseline.response_headers.as_deref(),
        Some(response_headers),
        &mut summary.changed_targets,
        &mut summary.notes,
    );
    diff_body(
        baseline.response_body.as_deref(),
        Some(response_body),
        &mut summary.changed_targets,
        &mut summary.notes,
    );

    summary
}

fn diff_headers(
    baseline_headers: Option<&str>,
    response_headers: Option<&str>,
    changed_targets: &mut Vec<ResponseDiffTarget>,
    notes: &mut Vec<String>,
) {
    if baseline_headers == response_headers {
        return;
    }

    let baseline_map = baseline_headers.and_then(parse_json_object);
    let response_map = response_headers.and_then(parse_json_object);
    if baseline_map.is_some() || response_map.is_some() {
        diff_string_map(
            "responseHeaders",
            baseline_map.unwrap_or_default(),
            response_map.unwrap_or_default(),
            changed_targets,
        );
        return;
    }

    notes.push(
        "Response headers changed but could not be parsed as a structured JSON object."
            .to_string(),
    );
    changed_targets.push(ResponseDiffTarget {
        location: "responseHeadersRaw".to_string(),
        selector: "headers".to_string(),
        change_kind: classify_change_kind(baseline_headers, response_headers).to_string(),
        before: baseline_headers.map(str::to_string),
        after: response_headers.map(str::to_string),
    });
}

fn diff_body(
    baseline_body: Option<&str>,
    response_body: Option<&str>,
    changed_targets: &mut Vec<ResponseDiffTarget>,
    notes: &mut Vec<String>,
) {
    if baseline_body == response_body {
        return;
    }

    let baseline_json = baseline_body.and_then(parse_json_object);
    let response_json = response_body.and_then(parse_json_object);
    if baseline_json.is_some() || response_json.is_some() {
        diff_string_map(
            "responseJsonBody",
            baseline_json.unwrap_or_default(),
            response_json.unwrap_or_default(),
            changed_targets,
        );
        return;
    }

    notes.push(
        "Response body changed but could not be parsed as a JSON object; stored as raw diff."
            .to_string(),
    );
    changed_targets.push(ResponseDiffTarget {
        location: "responseBodyRaw".to_string(),
        selector: "body".to_string(),
        change_kind: classify_change_kind(baseline_body, response_body).to_string(),
        before: baseline_body.map(str::to_string),
        after: response_body.map(str::to_string),
    });
}

fn diff_string_map(
    location: &str,
    baseline: BTreeMap<String, String>,
    response: BTreeMap<String, String>,
    changed_targets: &mut Vec<ResponseDiffTarget>,
) {
    let keys = baseline
        .keys()
        .chain(response.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        let before = baseline.get(&key).cloned();
        let after = response.get(&key).cloned();
        if before == after {
            continue;
        }
        changed_targets.push(ResponseDiffTarget {
            location: location.to_string(),
            selector: key,
            change_kind: classify_change_kind(before.as_deref(), after.as_deref()).to_string(),
            before,
            after,
        });
    }
}

fn parse_json_object(raw: &str) -> Option<BTreeMap<String, String>> {
    let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    let object = value.as_object()?;
    Some(
        object
            .iter()
            .map(|(key, value)| {
                let rendered = match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                (key.clone(), rendered)
            })
            .collect(),
    )
}

fn classify_change_kind(before: Option<&str>, after: Option<&str>) -> &'static str {
    match (before, after) {
        (None, Some(_)) => "added",
        (Some(_), None) => "removed",
        _ => "updated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> VerificationBaseline {
        VerificationBaseline {
            source_request_id: Some(1),
            url: "https://shop.test/api/checkout".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some(r#"{"quantity":1}"#.to_string()),
            response_status: Some(402),
            response_headers: Some(r#"{"content-type":"application/json"}"#.to_string()),
            response_body: Some(r#"{"error":"INSUFFICIENT_FUNDS"}"#.to_string()),
        }
    }

    #[test]
    fn captures_status_header_and_json_body_changes() {
        let summary = build_response_diff_summary(
            &baseline(),
            200,
            r#"{"content-type":"application/json","x-trace":"1"}"#,
            r#"{"success":true}"#,
        );

        assert!(summary.response_status_changed);
        assert!(summary.response_headers_changed);
        assert!(summary.response_body_changed);
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "status"));
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "responseHeaders" && item.selector == "x-trace"));
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "responseJsonBody" && item.selector == "success"));
    }

    #[test]
    fn falls_back_to_raw_response_body_note_when_not_json() {
        let mut baseline = baseline();
        baseline.response_body = None;
        let summary =
            build_response_diff_summary(&baseline, 402, r#"{"content-type":"application/json"}"#, "");

        assert!(summary.response_body_changed);
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "responseBodyRaw"));
        assert!(!summary.notes.is_empty());
    }
}
