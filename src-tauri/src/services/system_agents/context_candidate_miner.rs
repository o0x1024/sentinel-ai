use std::collections::{BTreeSet, HashMap};

use serde_json::Value;
use url::Url;

use sentinel_traffic::HttpRequestRecord;

use crate::services::system_agents::context_candidate_rules::{
    annotate_existing_coverage, build_candidate_aggregate, derive_behavior_evidence,
    derive_behavior_reason, normalize_lookup_key, score_action_alias_candidate,
    score_auth_header_candidate, score_cookie_hint_candidate, score_field_candidate,
    CandidateAggregate, CandidateBehaviorHint, CandidateObservation, CandidateSource,
};
use crate::services::system_agents::context_candidate_types::{
    ContextDictionaryCandidate, ContextDictionaryCandidateCategory,
    ContextDictionaryCandidateEvidenceRequest, RecommendTrafficContextDictionaryCandidatesResponse,
};
use crate::services::system_agents::TrafficContextExtractionSettings;

pub fn recommend_traffic_context_dictionary_candidates(
    records: &[HttpRequestRecord],
    behavior_contexts: Option<&HashMap<i64, Value>>,
    existing_settings: TrafficContextExtractionSettings,
    max_candidates_per_category: usize,
) -> RecommendTrafficContextDictionaryCandidatesResponse {
    let request_summaries = records
        .iter()
        .map(|record| {
            (
                record.id,
                CandidateRequestSummary {
                    method: record
                        .edited_method
                        .clone()
                        .unwrap_or_else(|| record.method.clone()),
                    url: record
                        .edited_url
                        .clone()
                        .unwrap_or_else(|| record.url.clone()),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let mut field_observations: HashMap<String, Vec<CandidateObservation>> = HashMap::new();
    let mut header_observations: HashMap<String, Vec<CandidateObservation>> = HashMap::new();
    let mut cookie_observations: HashMap<String, Vec<CandidateObservation>> = HashMap::new();
    let mut action_observations: HashMap<String, Vec<CandidateObservation>> = HashMap::new();

    for record in records {
        collect_request_observations(
            record,
            behavior_contexts.and_then(|contexts| contexts.get(&record.id)),
            &mut field_observations,
            &mut header_observations,
            &mut cookie_observations,
            &mut action_observations,
        );
    }

    let mut candidates = Vec::new();
    candidates.extend(build_field_candidates(
        field_observations,
        &request_summaries,
        &existing_settings,
    ));
    candidates.extend(build_header_candidates(
        header_observations,
        &request_summaries,
        &existing_settings,
    ));
    candidates.extend(build_cookie_candidates(
        cookie_observations,
        &request_summaries,
        &existing_settings,
    ));
    candidates.extend(build_action_candidates(
        action_observations,
        &request_summaries,
        &existing_settings,
    ));

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.category.cmp(&right.category))
            .then_with(|| left.key.cmp(&right.key))
    });

    let mut trimmed_by_category: HashMap<String, usize> = HashMap::new();
    let candidates = candidates
        .into_iter()
        .filter(|candidate| {
            let counter = trimmed_by_category
                .entry(candidate.category.clone())
                .or_insert(0);
            if *counter >= max_candidates_per_category {
                return false;
            }
            *counter += 1;
            true
        })
        .collect::<Vec<_>>();

    RecommendTrafficContextDictionaryCandidatesResponse {
        candidates,
        analyzed_request_count: records.len(),
        skipped_request_count: 0,
        existing_settings,
    }
}

#[derive(Debug, Clone)]
struct CandidateRequestSummary {
    method: String,
    url: String,
}

fn collect_request_observations(
    record: &HttpRequestRecord,
    behavior_context: Option<&Value>,
    field_observations: &mut HashMap<String, Vec<CandidateObservation>>,
    header_observations: &mut HashMap<String, Vec<CandidateObservation>>,
    cookie_observations: &mut HashMap<String, Vec<CandidateObservation>>,
    action_observations: &mut HashMap<String, Vec<CandidateObservation>>,
) {
    let behavior_hint = summarize_behavior_hint(behavior_context);
    let raw_url = effective_url(record);
    let parsed_url = Url::parse(&raw_url).ok();

    if let Some(url) = &parsed_url {
        for (key, value) in url.query_pairs() {
            push_observation(
                field_observations,
                key.to_string(),
                CandidateSource::Query,
                Some(value.to_string()),
                format!("query.{key}"),
                record.id,
                behavior_hint.clone(),
            );
        }
    }

    for (key, value) in extract_body_fields(record) {
        push_observation(
            field_observations,
            key.clone(),
            CandidateSource::Body,
            Some(value),
            format!("body.{key}"),
            record.id,
            behavior_hint.clone(),
        );
    }

    let headers = extract_headers(record);
    for (key, value) in &headers {
        push_observation(
            header_observations,
            key.clone(),
            CandidateSource::Header,
            value.clone(),
            format!("header.{key}"),
            record.id,
            behavior_hint.clone(),
        );
    }

    if let Some(cookie_header) = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("cookie"))
        .and_then(|(_, value)| value.clone())
    {
        for entry in cookie_header.split(';') {
            let cookie_name = entry.split('=').next().unwrap_or("").trim();
            if cookie_name.is_empty() {
                continue;
            }
            push_observation(
                cookie_observations,
                cookie_name.to_string(),
                CandidateSource::Cookie,
                Some(entry.trim().to_string()),
                format!("cookie.{cookie_name}"),
                record.id,
                behavior_hint.clone(),
            );
        }
    }

    let path = parsed_url
        .as_ref()
        .map(|url| url.path().to_string())
        .unwrap_or_else(|| raw_url.clone());
    for token in extract_action_tokens(&path) {
        push_observation(
            action_observations,
            token.clone(),
            CandidateSource::Path,
            None,
            format!("path.{token}"),
            record.id,
            behavior_hint.clone(),
        );
    }
}

fn push_observation(
    store: &mut HashMap<String, Vec<CandidateObservation>>,
    key: String,
    source: CandidateSource,
    value_sample: Option<String>,
    location: String,
    request_id: i64,
    behavior_hint: Option<CandidateBehaviorHint>,
) {
    let normalized_key = normalize_lookup_key(&key);
    if normalized_key.is_empty() {
        return;
    }

    store
        .entry(normalized_key.clone())
        .or_default()
        .push(CandidateObservation {
            key,
            normalized_key,
            source,
            value_sample,
            location,
            request_id,
            behavior_hint,
        });
}

fn build_field_candidates(
    grouped_observations: HashMap<String, Vec<CandidateObservation>>,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    settings: &TrafficContextExtractionSettings,
) -> Vec<ContextDictionaryCandidate> {
    grouped_observations
        .into_values()
        .filter_map(build_candidate_aggregate)
        .filter_map(|aggregate| {
            candidate_from_field_aggregate(aggregate, request_summaries, settings)
        })
        .collect()
}

fn candidate_from_field_aggregate(
    aggregate: CandidateAggregate,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    settings: &TrafficContextExtractionSettings,
) -> Option<ContextDictionaryCandidate> {
    let (category, score, rule_reasons, suggested_canonical_action, _) =
        score_field_candidate(&aggregate)?;
    let mut candidate = candidate_from_aggregate(
        aggregate,
        request_summaries,
        category,
        score,
        rule_reasons,
        suggested_canonical_action,
    );
    annotate_existing_coverage(&mut candidate, settings);
    Some(candidate)
}

fn build_header_candidates(
    grouped_observations: HashMap<String, Vec<CandidateObservation>>,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    settings: &TrafficContextExtractionSettings,
) -> Vec<ContextDictionaryCandidate> {
    grouped_observations
        .into_values()
        .filter_map(build_candidate_aggregate)
        .filter_map(|aggregate| {
            let (score, rule_reasons) = score_auth_header_candidate(&aggregate)?;
            let mut candidate = candidate_from_aggregate(
                aggregate,
                request_summaries,
                ContextDictionaryCandidateCategory::AuthHeader,
                score,
                rule_reasons,
                None,
            );
            annotate_existing_coverage(&mut candidate, settings);
            Some(candidate)
        })
        .collect()
}

fn build_cookie_candidates(
    grouped_observations: HashMap<String, Vec<CandidateObservation>>,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    settings: &TrafficContextExtractionSettings,
) -> Vec<ContextDictionaryCandidate> {
    grouped_observations
        .into_values()
        .filter_map(build_candidate_aggregate)
        .filter_map(|aggregate| {
            let (score, rule_reasons) = score_cookie_hint_candidate(&aggregate)?;
            let mut candidate = candidate_from_aggregate(
                aggregate,
                request_summaries,
                ContextDictionaryCandidateCategory::CookieHint,
                score,
                rule_reasons,
                None,
            );
            annotate_existing_coverage(&mut candidate, settings);
            Some(candidate)
        })
        .collect()
}

fn build_action_candidates(
    grouped_observations: HashMap<String, Vec<CandidateObservation>>,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    settings: &TrafficContextExtractionSettings,
) -> Vec<ContextDictionaryCandidate> {
    grouped_observations
        .into_values()
        .filter_map(build_candidate_aggregate)
        .filter_map(|aggregate| {
            let (score, rule_reasons, suggested_canonical_action) =
                score_action_alias_candidate(&aggregate)?;
            let mut candidate = candidate_from_aggregate(
                aggregate,
                request_summaries,
                ContextDictionaryCandidateCategory::ActionAlias,
                score,
                rule_reasons,
                suggested_canonical_action,
            );
            annotate_existing_coverage(&mut candidate, settings);
            Some(candidate)
        })
        .collect()
}

fn candidate_from_aggregate(
    aggregate: CandidateAggregate,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
    category: ContextDictionaryCandidateCategory,
    score: i32,
    rule_reasons: Vec<String>,
    suggested_canonical_action: Option<String>,
) -> ContextDictionaryCandidate {
    let preferred_key = aggregate.preferred_key.clone();
    let behavior_reason = derive_behavior_reason(
        &aggregate,
        &category,
        &preferred_key,
        suggested_canonical_action.as_deref(),
    );
    let behavior_evidence = derive_behavior_evidence(&aggregate);
    let evidence_requests = derive_evidence_requests(&aggregate, request_summaries);
    ContextDictionaryCandidate {
        key: preferred_key,
        normalized_key: aggregate.normalized_key,
        category: category.as_str().to_string(),
        suggested_canonical_action,
        confidence: if score >= 60 {
            "high".to_string()
        } else if score >= 45 {
            "medium".to_string()
        } else {
            "low".to_string()
        },
        score,
        evidence_count: aggregate.evidence_count,
        distinct_value_count: aggregate.distinct_value_count,
        sources: aggregate.sources,
        example_values: aggregate.example_values,
        example_locations: aggregate.example_locations,
        rule_reasons,
        ai_reason: None,
        behavior_reason,
        behavior_evidence,
        evidence_requests,
        conflict_with_existing: false,
        already_covered_by: None,
    }
}

fn derive_evidence_requests(
    aggregate: &CandidateAggregate,
    request_summaries: &HashMap<i64, CandidateRequestSummary>,
) -> Vec<ContextDictionaryCandidateEvidenceRequest> {
    let mut request_locations = HashMap::<i64, BTreeSet<String>>::new();
    for observation in &aggregate.observations {
        request_locations
            .entry(observation.request_id)
            .or_default()
            .insert(observation.location.clone());
    }

    let mut request_ids = request_locations.keys().copied().collect::<Vec<_>>();
    request_ids.sort_unstable();

    request_ids
        .into_iter()
        .take(4)
        .filter_map(|request_id| {
            let summary = request_summaries.get(&request_id)?;
            let matched_locations = request_locations
                .remove(&request_id)
                .map(|items| items.into_iter().take(3).collect::<Vec<_>>())
                .unwrap_or_default();
            Some(ContextDictionaryCandidateEvidenceRequest {
                request_id,
                method: summary.method.clone(),
                url: summary.url.clone(),
                matched_locations,
            })
        })
        .collect()
}

fn summarize_behavior_hint(context: Option<&Value>) -> Option<CandidateBehaviorHint> {
    let context = context?;
    let intent_hints = extract_behavior_string_array(context.get("intentHints"));
    let behavior_steps = extract_behavior_string_array(context.get("behaviorSteps"));
    let last_page_title = context
        .get("lastPageTitle")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let last_page_url = context
        .get("lastPageUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if intent_hints.is_empty()
        && behavior_steps.is_empty()
        && last_page_title.is_none()
        && last_page_url.is_none()
    {
        return None;
    }

    Some(CandidateBehaviorHint {
        intent_hints,
        behavior_steps,
        last_page_title,
        last_page_url,
    })
}

fn extract_behavior_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .take(4)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::recommend_traffic_context_dictionary_candidates;
    use crate::services::system_agents::TrafficContextExtractionSettings;
    use chrono::Utc;
    use sentinel_traffic::HttpRequestRecord;

    fn build_record(id: i64, url: &str) -> HttpRequestRecord {
        HttpRequestRecord {
            id,
            db_request_id: Some(1000 + id),
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
    fn candidates_include_evidence_request_samples() {
        let records = vec![
            build_record(1, "https://example.com/api/workflows/detail?biz_ref=CASE-9"),
            build_record(
                2,
                "https://example.com/api/workflows/detail?biz_ref=CASE-10",
            ),
        ];

        let response = recommend_traffic_context_dictionary_candidates(
            &records,
            None,
            TrafficContextExtractionSettings::default(),
            12,
        );

        let candidate = response
            .candidates
            .iter()
            .find(|item| item.key == "biz_ref")
            .expect("expected biz_ref candidate");

        assert!(!candidate.evidence_requests.is_empty());
        assert_eq!(candidate.evidence_requests[0].request_id, 1);
        assert!(candidate.evidence_requests[0]
            .matched_locations
            .iter()
            .any(|item| item == "query.biz_ref"));
    }
}

fn effective_url(record: &HttpRequestRecord) -> String {
    record
        .edited_url
        .clone()
        .unwrap_or_else(|| record.url.clone())
}

fn extract_body_fields(record: &HttpRequestRecord) -> Vec<(String, String)> {
    let body = record
        .edited_request_body
        .as_deref()
        .or(record.request_body.as_deref());
    let Some(body) = body else {
        return Vec::new();
    };

    if let Ok(value) = serde_json::from_str::<Value>(body) {
        let mut fields = Vec::new();
        flatten_json_value(&value, "", &mut fields);
        return fields;
    }

    url::form_urlencoded::parse(body.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn flatten_json_value(value: &Value, path: &str, output: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let next_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                flatten_json_value(nested, &next_path, output);
            }
        }
        Value::Array(items) => {
            for item in items {
                flatten_json_value(item, path, output);
            }
        }
        Value::String(text) => {
            if !path.is_empty() {
                output.push((leaf_field_name(path), text.clone()));
            }
        }
        Value::Number(number) => {
            if !path.is_empty() {
                output.push((leaf_field_name(path), number.to_string()));
            }
        }
        Value::Bool(flag) => {
            if !path.is_empty() {
                output.push((leaf_field_name(path), flag.to_string()));
            }
        }
        Value::Null => {}
    }
}

fn leaf_field_name(path: &str) -> String {
    path.split('.').last().unwrap_or(path).to_string()
}

fn extract_headers(record: &HttpRequestRecord) -> Vec<(String, Option<String>)> {
    let raw = record
        .edited_request_headers
        .as_deref()
        .or(record.request_headers.as_deref());
    let Some(raw) = raw else {
        return Vec::new();
    };

    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .map(|map| {
            map.into_iter()
                .map(|(key, value)| (key, Some(render_value(&value))))
                .collect()
        })
        .unwrap_or_default()
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items
            .iter()
            .map(render_value)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) | Value::Null => String::new(),
    }
}

fn extract_action_tokens(path: &str) -> Vec<String> {
    path.split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .filter(|segment| !matches!(segment.as_str(), "api" | "v1" | "v2" | "v3"))
        .filter(|segment| !segment.chars().all(|ch| ch.is_ascii_digit()))
        .filter(|segment| !segment.contains('{'))
        .filter(|segment| segment.len() >= 3)
        .collect()
}
