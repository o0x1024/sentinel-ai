use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

use crate::services::system_agents::verification_plan::VerificationBaseline;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestDiffSummary {
    pub request_url_changed: bool,
    pub request_body_changed: bool,
    #[serde(default)]
    pub changed_targets: Vec<RequestDiffTarget>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestDiffTarget {
    pub location: String,
    pub selector: String,
    pub change_kind: String,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

pub fn build_request_diff_summary(
    baseline: &VerificationBaseline,
    request_url: &str,
    request_body: Option<&str>,
) -> RequestDiffSummary {
    let mut summary = RequestDiffSummary {
        request_url_changed: baseline.url != request_url,
        request_body_changed: baseline.request_body.as_deref() != request_body,
        changed_targets: Vec::new(),
        notes: Vec::new(),
    };

    diff_url(&baseline.url, request_url, &mut summary);
    diff_body(
        baseline.request_body.as_deref(),
        request_body,
        &mut summary.changed_targets,
        &mut summary.notes,
    );

    summary
}

fn diff_url(baseline_url: &str, request_url: &str, summary: &mut RequestDiffSummary) {
    let baseline = match Url::parse(baseline_url) {
        Ok(value) => value,
        Err(_) => {
            if baseline_url != request_url {
                summary.notes.push(
                    "URL changed but baseline URL could not be parsed structurally.".to_string(),
                );
            }
            return;
        }
    };
    let request = match Url::parse(request_url) {
        Ok(value) => value,
        Err(_) => {
            if baseline_url != request_url {
                summary.notes.push(
                    "URL changed but replay URL could not be parsed structurally.".to_string(),
                );
            }
            return;
        }
    };

    diff_string_map(
        "query",
        query_map(&baseline),
        query_map(&request),
        &mut summary.changed_targets,
    );

    let baseline_segments = baseline
        .path_segments()
        .map(|segments| segments.map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let request_segments = request
        .path_segments()
        .map(|segments| segments.map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let max_len = baseline_segments.len().max(request_segments.len());
    for index in 0..max_len {
        let before = baseline_segments.get(index).cloned();
        let after = request_segments.get(index).cloned();
        if before == after {
            continue;
        }
        summary.changed_targets.push(RequestDiffTarget {
            location: "pathSegment".to_string(),
            selector: index.to_string(),
            change_kind: classify_change_kind(before.as_deref(), after.as_deref()).to_string(),
            before,
            after,
        });
    }
}

fn diff_body(
    baseline_body: Option<&str>,
    request_body: Option<&str>,
    changed_targets: &mut Vec<RequestDiffTarget>,
    notes: &mut Vec<String>,
) {
    if baseline_body == request_body {
        return;
    }

    let baseline_json = baseline_body.and_then(parse_json_object);
    let request_json = request_body.and_then(parse_json_object);
    if baseline_json.is_some() || request_json.is_some() {
        diff_string_map(
            "jsonBody",
            baseline_json.unwrap_or_default(),
            request_json.unwrap_or_default(),
            changed_targets,
        );
        return;
    }

    let baseline_form = baseline_body.and_then(parse_form_object);
    let request_form = request_body.and_then(parse_form_object);
    if baseline_form.is_some() || request_form.is_some() {
        diff_string_map(
            "formBody",
            baseline_form.unwrap_or_default(),
            request_form.unwrap_or_default(),
            changed_targets,
        );
        return;
    }

    notes.push(
        "Request body changed but could not be parsed as JSON object or form body.".to_string(),
    );
    changed_targets.push(RequestDiffTarget {
        location: "rawBody".to_string(),
        selector: "body".to_string(),
        change_kind: classify_change_kind(baseline_body, request_body).to_string(),
        before: baseline_body.map(str::to_string),
        after: request_body.map(str::to_string),
    });
}

fn diff_string_map(
    location: &str,
    baseline: BTreeMap<String, String>,
    request: BTreeMap<String, String>,
    changed_targets: &mut Vec<RequestDiffTarget>,
) {
    let keys = baseline
        .keys()
        .chain(request.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        let before = baseline.get(&key).cloned();
        let after = request.get(&key).cloned();
        if before == after {
            continue;
        }
        changed_targets.push(RequestDiffTarget {
            location: location.to_string(),
            selector: key,
            change_kind: classify_change_kind(before.as_deref(), after.as_deref()).to_string(),
            before,
            after,
        });
    }
}

fn classify_change_kind(before: Option<&str>, after: Option<&str>) -> &'static str {
    match (before, after) {
        (None, Some(_)) => "added",
        (Some(_), None) => "removed",
        _ => "updated",
    }
}

fn query_map(url: &Url) -> BTreeMap<String, String> {
    url.query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn parse_json_object(body: &str) -> Option<BTreeMap<String, String>> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
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

fn parse_form_object(body: &str) -> Option<BTreeMap<String, String>> {
    let pairs = url::form_urlencoded::parse(body.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return None;
    }
    Some(pairs.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> VerificationBaseline {
        VerificationBaseline {
            source_request_id: Some(1),
            url: "https://shop.test/api/orders/123?quantity=1".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some(r#"{"coupon":"SAVE10","note":"keep"}"#.to_string()),
            response_status: Some(402),
            response_headers: None,
            response_body: Some(r#"{"error":"INSUFFICIENT_FUNDS"}"#.to_string()),
        }
    }

    #[test]
    fn captures_query_path_and_json_diffs() {
        let summary = build_request_diff_summary(
            &baseline(),
            "https://shop.test/api/orders/124?quantity=0",
            Some(r#"{"note":"keep"}"#),
        );

        assert!(summary.request_url_changed);
        assert!(summary.request_body_changed);
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "query" && item.selector == "quantity"));
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "pathSegment" && item.before.as_deref() == Some("123")));
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "jsonBody"
                && item.selector == "coupon"
                && item.change_kind == "removed"));
    }

    #[test]
    fn falls_back_to_raw_body_note_when_structure_unknown() {
        let mut baseline = baseline();
        baseline.request_body = None;
        let summary = build_request_diff_summary(&baseline, &baseline.url, Some(""));

        assert!(summary.request_body_changed);
        assert!(summary
            .changed_targets
            .iter()
            .any(|item| item.location == "rawBody"));
        assert!(!summary.notes.is_empty());
    }
}
