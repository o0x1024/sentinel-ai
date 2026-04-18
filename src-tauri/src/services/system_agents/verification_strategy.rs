use anyhow::{anyhow, Result};
use serde_json::Value;
use url::Url;

use crate::services::system_agents::verification_mutation::{
    mutate_business_parameter_request, AppliedVerificationMutation, VerificationMutationSelectors,
};
use crate::services::system_agents::verification_plan::{
    VerificationBaseline, VerificationPlan, VerificationProbePayload, VerificationTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationExecutionMode {
    Single,
    ConcurrentDuplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationSequenceMode {
    None,
    SkipPrerequisite,
    ReplayAfterTarget,
}

#[derive(Debug, Clone)]
pub struct PreparedVerificationRequest {
    pub url: String,
    pub body: Option<String>,
    pub header_overrides: Vec<(String, String)>,
    pub mutated: bool,
    pub notes: Vec<String>,
    pub strategy_used: String,
    pub execution_mode: VerificationExecutionMode,
    pub execution_count: usize,
    pub sequence_mode: VerificationSequenceMode,
    pub applied_mutations: Vec<AppliedVerificationMutation>,
}

pub fn prepare_verification_request(
    baseline: &VerificationBaseline,
    plan: Option<&VerificationPlan>,
) -> Result<PreparedVerificationRequest> {
    let Some(plan) = plan else {
        return Ok(PreparedVerificationRequest {
            url: baseline.url.clone(),
            body: baseline.request_body.clone(),
            header_overrides: Vec::new(),
            mutated: false,
            notes: vec![
                "No verification plan was provided; replayed the baseline request as-is."
                    .to_string(),
            ],
            strategy_used: "replay_as_is".to_string(),
            execution_mode: VerificationExecutionMode::Single,
            execution_count: 1,
            sequence_mode: VerificationSequenceMode::None,
            applied_mutations: Vec::new(),
        });
    };

    match plan.preferred_strategy.as_str() {
        "replay_as_is" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Replayed the baseline request without mutation.",
            1,
            VerificationSequenceMode::None,
        )),
        "repeat_action" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Repeated the original action with the same request payload.",
            normalize_repeat_count(plan.replay_count),
            VerificationSequenceMode::None,
        )),
        "skip_prerequisite" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Replayed the target action independently to check whether prerequisite steps are enforced server-side.",
            1,
            VerificationSequenceMode::SkipPrerequisite,
        )),
        "reorder_sequence" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Replayed the selected action out of its original request sequence to test ordering constraints.",
            1,
            VerificationSequenceMode::ReplayAfterTarget,
        )),
        "concurrent_submit" => Ok(PreparedVerificationRequest {
            url: baseline.url.clone(),
            body: baseline.request_body.clone(),
            header_overrides: Vec::new(),
            mutated: false,
            notes: collect_strategy_notes(
                plan,
                "Replayed the target request concurrently to probe duplicate-success or race-sensitive behavior.",
            ),
            strategy_used: plan.preferred_strategy.clone(),
            execution_mode: VerificationExecutionMode::ConcurrentDuplicate,
            execution_count: normalize_concurrent_count(plan.concurrent_requests),
            sequence_mode: VerificationSequenceMode::None,
            applied_mutations: Vec::new(),
        }),
        "swap_identity" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Replayed the target request with an alternate authentication context from the same traffic cluster.",
            1,
            VerificationSequenceMode::None,
        )),
        "swap_resource_reference" => {
            let (mutated_url, mutated_body, applied_mutations) = mutate_request_reference(
                &baseline.url,
                baseline.request_body.as_deref(),
                &plan.candidate_targets,
                &plan.candidate_parameters,
            )?;
            let mut prepared = single_request(
                mutated_url,
                mutated_body,
                Vec::new(),
                true,
                plan,
                "Mutated a candidate resource reference before replay.",
                1,
                VerificationSequenceMode::None,
            );
            prepared.applied_mutations = applied_mutations;
            Ok(prepared)
        }
        "mutate_business_parameter" => {
            if plan.parameter_mutations.is_empty() {
                return Err(anyhow!(
                    "Business-parameter mutation strategy requires parameterMutations"
                ));
            }
            let (mutated_url, mutated_body, applied_mutations) = mutate_business_parameter_request(
                &baseline.url,
                baseline.request_body.as_deref(),
                &plan.parameter_mutations,
                &build_mutation_selectors(plan),
            )?;
            let mut prepared = single_request(
                mutated_url,
                mutated_body,
                Vec::new(),
                true,
                plan,
                "Mutated a business-sensitive request parameter before replay.",
                1,
                VerificationSequenceMode::None,
            );
            prepared.applied_mutations = applied_mutations;
            Ok(prepared)
        }
        "input_probe" => prepare_probe_strategy(
            baseline,
            plan,
            default_probe_payload("input_probe"),
            "Injected a low-risk input probe into an explicit candidate target before replay.",
        ),
        "path_traversal_probe" => prepare_probe_strategy(
            baseline,
            plan,
            default_probe_payload("path_traversal_probe"),
            "Injected a path traversal probe into a candidate path-like target before replay.",
        ),
        "cors_origin_probe" => {
            let origin = plan
                .probe_payloads
                .iter()
                .find(|probe| probe.location.eq_ignore_ascii_case("header"))
                .map(|probe| probe.payload.clone())
                .filter(|payload| !payload.trim().is_empty())
                .unwrap_or_else(|| default_probe_payload("cors_origin_probe"));
            Ok(single_request(
                baseline.url.clone(),
                baseline.request_body.clone(),
                vec![("Origin".to_string(), origin)],
                true,
                plan,
                "Replayed the target request with a synthetic cross-origin Origin header to assess CORS policy behavior.",
                1,
                VerificationSequenceMode::None,
            ))
        }
        "header_policy_probe" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            Vec::new(),
            false,
            plan,
            "Replayed the target request and inspected response security headers for policy weaknesses.",
            1,
            VerificationSequenceMode::None,
        )),
        "oast_probe" => prepare_probe_strategy(
            baseline,
            plan,
            default_probe_payload("oast_probe"),
            "Injected an outbound-callback style probe into an explicit candidate target before replay.",
        ),
        "manual_review" => Err(anyhow!(
            "Verification plan requires manual review and was not auto-replayed"
        )),
        other => Err(anyhow!(
            "Unsupported verification strategy for auto verifier: {}",
            other
        )),
    }
}

fn single_request(
    url: String,
    body: Option<String>,
    header_overrides: Vec<(String, String)>,
    mutated: bool,
    plan: &VerificationPlan,
    summary: &str,
    execution_count: usize,
    sequence_mode: VerificationSequenceMode,
) -> PreparedVerificationRequest {
    PreparedVerificationRequest {
        url,
        body,
        header_overrides,
        mutated,
        notes: collect_strategy_notes(plan, summary),
        strategy_used: plan.preferred_strategy.clone(),
        execution_mode: VerificationExecutionMode::Single,
        execution_count,
        sequence_mode,
        applied_mutations: Vec::new(),
    }
}

fn prepare_probe_strategy(
    baseline: &VerificationBaseline,
    plan: &VerificationPlan,
    default_payload: String,
    summary: &str,
) -> Result<PreparedVerificationRequest> {
    let (mutated_url, mutated_body, header_overrides, applied_mutations) =
        apply_probe_payloads_to_request(
            &baseline.url,
            baseline.request_body.as_deref(),
            &plan.candidate_targets,
            &plan.candidate_parameters,
            &plan.probe_payloads,
            &default_payload,
        )?;
    let mut prepared = single_request(
        mutated_url,
        mutated_body,
        header_overrides,
        true,
        plan,
        summary,
        1,
        VerificationSequenceMode::None,
    );
    prepared.applied_mutations = applied_mutations;
    Ok(prepared)
}

fn collect_strategy_notes(plan: &VerificationPlan, summary: &str) -> Vec<String> {
    let mut notes = vec![summary.to_string()];
    if let Some(replay_count) = plan.replay_count.filter(|count| *count > 1) {
        notes.push(format!("Requested repeat replay count: {}", replay_count));
    }
    if let Some(concurrent_requests) = plan.concurrent_requests.filter(|count| *count > 1) {
        notes.push(format!(
            "Requested concurrent submission count: {}",
            concurrent_requests
        ));
    }
    if !plan.sequence_request_ids.is_empty() {
        notes.push(format!(
            "Related sequence request ids: {}",
            plan.sequence_request_ids
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    notes.extend(plan.notes.clone());
    notes
}

fn normalize_repeat_count(value: Option<u32>) -> usize {
    value.unwrap_or(2).clamp(2, 5) as usize
}

fn normalize_concurrent_count(value: Option<u32>) -> usize {
    value.unwrap_or(2).clamp(2, 5) as usize
}

fn mutate_request_reference(
    url: &str,
    body: Option<&str>,
    candidate_targets: &[VerificationTarget],
    candidate_parameters: &[String],
) -> Result<(String, Option<String>, Vec<AppliedVerificationMutation>)> {
    let query_keys = target_selectors(candidate_targets, "query");
    let path_values = target_selectors(candidate_targets, "pathSegment");
    let json_body_keys = target_selectors(candidate_targets, "jsonBody");
    if let Some((mutated_url, applied_mutation)) =
        mutate_url_reference(url, &query_keys, &path_values, candidate_parameters)?
    {
        return Ok((
            mutated_url,
            body.map(str::to_string),
            vec![applied_mutation],
        ));
    }

    if let Some((mutated_body, applied_mutation)) =
        mutate_json_body_reference(body, &json_body_keys, candidate_parameters)?
    {
        return Ok((url.to_string(), Some(mutated_body), vec![applied_mutation]));
    }

    Err(anyhow!(
        "Verification plan requested resource mutation but no candidate parameter could be mutated"
    ))
}

fn apply_probe_payloads_to_request(
    url: &str,
    body: Option<&str>,
    candidate_targets: &[VerificationTarget],
    candidate_parameters: &[String],
    probe_payloads: &[VerificationProbePayload],
    default_payload: &str,
) -> Result<(
    String,
    Option<String>,
    Vec<(String, String)>,
    Vec<AppliedVerificationMutation>,
)> {
    let mut current_url = url.to_string();
    let mut current_body = body.map(str::to_string);
    let mut header_overrides = Vec::new();
    let mut applied_mutations = Vec::new();
    let effective_payloads = effective_probe_payloads(
        probe_payloads,
        candidate_targets,
        candidate_parameters,
        default_payload,
    );
    if effective_payloads.is_empty() {
        return Err(anyhow!(
            "Verification plan requested probe execution but no explicit probe target could be resolved"
        ));
    }

    for probe in effective_payloads {
        match probe.location.as_str() {
            "query" => {
                let (next_url, selector) =
                    inject_query_probe(&current_url, &probe.selector, &probe.payload)?.ok_or_else(
                        || anyhow!("Probe target not found in query: {}", probe.selector),
                    )?;
                current_url = next_url;
                applied_mutations.push(AppliedVerificationMutation {
                    location: "query".to_string(),
                    selector,
                    mutation_kind: format!("probe:{}", payload_kind_or_default(&probe)),
                });
            }
            "pathSegment" => {
                let (next_url, selector) =
                    inject_path_probe(&current_url, &probe.selector, &probe.payload)?.ok_or_else(
                        || anyhow!("Probe target not found in path: {}", probe.selector),
                    )?;
                current_url = next_url;
                applied_mutations.push(AppliedVerificationMutation {
                    location: "pathSegment".to_string(),
                    selector,
                    mutation_kind: format!("probe:{}", payload_kind_or_default(&probe)),
                });
            }
            "jsonBody" => {
                let (next_body, selector) =
                    inject_json_probe(current_body.as_deref(), &probe.selector, &probe.payload)?
                        .ok_or_else(|| {
                            anyhow!("Probe target not found in JSON body: {}", probe.selector)
                        })?;
                current_body = Some(next_body);
                applied_mutations.push(AppliedVerificationMutation {
                    location: "jsonBody".to_string(),
                    selector,
                    mutation_kind: format!("probe:{}", payload_kind_or_default(&probe)),
                });
            }
            "formBody" => {
                let (next_body, selector) =
                    inject_form_probe(current_body.as_deref(), &probe.selector, &probe.payload)?
                        .ok_or_else(|| {
                            anyhow!("Probe target not found in form body: {}", probe.selector)
                        })?;
                current_body = Some(next_body);
                applied_mutations.push(AppliedVerificationMutation {
                    location: "formBody".to_string(),
                    selector,
                    mutation_kind: format!("probe:{}", payload_kind_or_default(&probe)),
                });
            }
            "header" => {
                header_overrides.push((probe.selector.clone(), probe.payload.clone()));
                applied_mutations.push(AppliedVerificationMutation {
                    location: "header".to_string(),
                    selector: probe.selector.clone(),
                    mutation_kind: format!("probe:{}", payload_kind_or_default(&probe)),
                });
            }
            other => {
                return Err(anyhow!("Unsupported probe payload location: {}", other));
            }
        }
    }

    Ok((
        current_url,
        current_body,
        header_overrides,
        applied_mutations,
    ))
}

fn mutate_url_reference(
    url: &str,
    query_keys: &[String],
    path_values: &[String],
    candidate_parameters: &[String],
) -> Result<Option<(String, AppliedVerificationMutation)>> {
    let mut parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let query_items = parsed.query_pairs().collect::<Vec<_>>();
    if !query_items.is_empty() {
        let mut updated = Vec::with_capacity(query_items.len());
        let mut mutated = false;
        let mut mutated_selector = None;
        for (key, value) in query_items {
            let key_text = key.to_string();
            let value_text = value.to_string();
            if !mutated && parameter_matches(&key_text, query_keys, candidate_parameters) {
                mutated_selector = Some(key_text.clone());
                updated.push((key_text, mutate_scalar_value(&value_text)));
                mutated = true;
            } else {
                updated.push((key_text, value_text));
            }
        }

        if mutated {
            parsed.query_pairs_mut().clear().extend_pairs(updated);
            return Ok(Some((
                parsed.to_string(),
                AppliedVerificationMutation {
                    location: "query".to_string(),
                    selector: mutated_selector.unwrap_or_default(),
                    mutation_kind: "mutate_reference".to_string(),
                },
            )));
        }
    }

    let path_segments = parsed
        .path_segments()
        .map(|items| items.map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    if path_segments.is_empty() {
        return Ok(None);
    }

    let mut updated_segments = path_segments.clone();
    for index in (0..updated_segments.len()).rev() {
        let candidate = updated_segments[index].clone();
        if path_segment_matches(&candidate, path_values, candidate_parameters) {
            let Some(mutated_segment) = mutate_path_segment(&candidate) else {
                continue;
            };
            updated_segments[index] = mutated_segment;
            let next_path = format!("/{}", updated_segments.join("/"));
            parsed.set_path(&next_path);
            return Ok(Some((
                parsed.to_string(),
                AppliedVerificationMutation {
                    location: "pathSegment".to_string(),
                    selector: candidate,
                    mutation_kind: "mutate_reference".to_string(),
                },
            )));
        }
    }

    Ok(None)
}

fn mutate_json_body_reference(
    body: Option<&str>,
    json_body_keys: &[String],
    candidate_parameters: &[String],
) -> Result<Option<(String, AppliedVerificationMutation)>> {
    let Some(body) = body else {
        return Ok(None);
    };

    let mut value: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(map) = value.as_object_mut() else {
        return Ok(None);
    };

    for key in prioritized_candidate_keys(json_body_keys, candidate_parameters) {
        if let Some(field) = map.get_mut(&key) {
            let replacement = match field {
                Value::String(text) => Value::String(mutate_scalar_value(text)),
                Value::Number(number) => number
                    .as_i64()
                    .map(|value| Value::Number((value + 1).into()))
                    .unwrap_or_else(|| Value::String(format!("{}-alt", number))),
                _ => continue,
            };
            *field = replacement;
            return Ok(Some((
                serde_json::to_string(&value)?,
                AppliedVerificationMutation {
                    location: "jsonBody".to_string(),
                    selector: key,
                    mutation_kind: "mutate_reference".to_string(),
                },
            )));
        }
    }

    Ok(None)
}

fn prioritized_candidate_keys(explicit_keys: &[String], legacy_keys: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    for item in explicit_keys.iter().chain(legacy_keys.iter()) {
        let trimmed = item.trim();
        if trimmed.is_empty() || keys.iter().any(|existing| existing == trimmed) {
            continue;
        }
        keys.push(trimmed.to_string());
    }
    keys
}

fn effective_probe_payloads(
    explicit_payloads: &[VerificationProbePayload],
    candidate_targets: &[VerificationTarget],
    candidate_parameters: &[String],
    default_payload: &str,
) -> Vec<VerificationProbePayload> {
    if !explicit_payloads.is_empty() {
        return explicit_payloads.to_vec();
    }

    candidate_targets
        .iter()
        .map(|target| VerificationProbePayload {
            location: target.location.clone(),
            selector: target.selector.clone(),
            payload: default_payload.to_string(),
            payload_kind: "default_probe".to_string(),
        })
        .chain(
            candidate_parameters
                .iter()
                .map(|parameter| VerificationProbePayload {
                    location: "query".to_string(),
                    selector: parameter.clone(),
                    payload: default_payload.to_string(),
                    payload_kind: "default_probe".to_string(),
                }),
        )
        .fold(Vec::new(), |mut acc, payload| {
            if !acc.iter().any(|item| {
                item.location.eq_ignore_ascii_case(&payload.location)
                    && item.selector.eq_ignore_ascii_case(&payload.selector)
            }) {
                acc.push(payload);
            }
            acc
        })
}

fn parameter_matches(key: &str, explicit_keys: &[String], legacy_keys: &[String]) -> bool {
    explicit_keys
        .iter()
        .chain(legacy_keys.iter())
        .any(|candidate| candidate.eq_ignore_ascii_case(key))
}

fn path_segment_matches(
    segment: &str,
    explicit_values: &[String],
    legacy_values: &[String],
) -> bool {
    explicit_values
        .iter()
        .chain(legacy_values.iter())
        .any(|candidate| candidate.eq_ignore_ascii_case(segment))
}

fn target_selectors(candidate_targets: &[VerificationTarget], location: &str) -> Vec<String> {
    candidate_targets
        .iter()
        .filter(|target| target.location.eq_ignore_ascii_case(location))
        .map(|target| target.selector.trim().to_string())
        .filter(|selector| !selector.is_empty())
        .collect()
}

fn build_mutation_selectors(plan: &VerificationPlan) -> VerificationMutationSelectors {
    VerificationMutationSelectors {
        query: target_selectors(&plan.candidate_targets, "query"),
        json_body: target_selectors(&plan.candidate_targets, "jsonBody"),
        form_body: target_selectors(&plan.candidate_targets, "formBody"),
        legacy: plan.candidate_parameters.clone(),
    }
}

fn inject_query_probe(
    url: &str,
    selector: &str,
    payload: &str,
) -> Result<Option<(String, String)>> {
    let mut parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let query_items = parsed.query_pairs().collect::<Vec<_>>();
    if query_items.is_empty() {
        return Ok(None);
    }
    let mut updated = Vec::with_capacity(query_items.len());
    let mut mutated = false;
    for (key, value) in query_items {
        let key_text = key.to_string();
        if !mutated && key_text.eq_ignore_ascii_case(selector) {
            updated.push((key_text.clone(), payload.to_string()));
            mutated = true;
        } else {
            updated.push((key_text, value.to_string()));
        }
    }
    if !mutated {
        return Ok(None);
    }
    parsed.query_pairs_mut().clear().extend_pairs(updated);
    Ok(Some((parsed.to_string(), selector.to_string())))
}

fn inject_path_probe(url: &str, selector: &str, payload: &str) -> Result<Option<(String, String)>> {
    let mut parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let mut segments = parsed
        .path_segments()
        .map(|items| items.map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut mutated = false;
    for segment in &mut segments {
        if !mutated && segment.eq_ignore_ascii_case(selector) {
            *segment = payload.to_string();
            mutated = true;
        }
    }
    if !mutated {
        return Ok(None);
    }
    parsed.set_path(&format!("/{}", segments.join("/")));
    Ok(Some((parsed.to_string(), selector.to_string())))
}

fn inject_json_probe(
    body: Option<&str>,
    selector: &str,
    payload: &str,
) -> Result<Option<(String, String)>> {
    let Some(body) = body else {
        return Ok(None);
    };
    let mut value: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(map) = value.as_object_mut() else {
        return Ok(None);
    };
    let Some(field) = map.get_mut(selector) else {
        return Ok(None);
    };
    *field = Value::String(payload.to_string());
    Ok(Some((serde_json::to_string(&value)?, selector.to_string())))
}

fn inject_form_probe(
    body: Option<&str>,
    selector: &str,
    payload: &str,
) -> Result<Option<(String, String)>> {
    let Some(body) = body else {
        return Ok(None);
    };
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    let mut mutated = false;
    for (key, value) in url::form_urlencoded::parse(body.as_bytes()) {
        if !mutated && key.eq_ignore_ascii_case(selector) {
            serializer.append_pair(&key, payload);
            mutated = true;
        } else {
            serializer.append_pair(&key, &value);
        }
    }
    if !mutated {
        return Ok(None);
    }
    Ok(Some((serializer.finish(), selector.to_string())))
}

fn default_probe_payload(strategy: &str) -> String {
    match strategy {
        "path_traversal_probe" => "../../../../etc/passwd".to_string(),
        "cors_origin_probe" => "https://sentinel.invalid".to_string(),
        "oast_probe" => "https://oast.invalid/sentinel".to_string(),
        _ => "sentinel-probe".to_string(),
    }
}

fn payload_kind_or_default(probe: &VerificationProbePayload) -> &str {
    let trimmed = probe.payload_kind.trim();
    if trimmed.is_empty() {
        "generic"
    } else {
        trimmed
    }
}

fn mutate_path_segment(segment: &str) -> Option<String> {
    if segment.chars().all(|item| item.is_ascii_digit()) {
        return segment
            .parse::<i64>()
            .ok()
            .map(|value| (value + 1).to_string());
    }

    if segment.len() >= 4 && segment.chars().filter(|item| item.is_ascii_digit()).count() >= 2 {
        return Some(mutate_scalar_value(segment));
    }

    None
}

fn mutate_scalar_value(value: &str) -> String {
    if value.chars().all(|item| item.is_ascii_digit()) {
        return value
            .parse::<i64>()
            .map(|number| (number + 1).to_string())
            .unwrap_or_else(|_| format!("{}-alt", value));
    }

    if let Some((prefix, suffix)) = split_numeric_suffix(value) {
        return format!("{}{}", prefix, suffix + 1);
    }

    format!("{}-agent", value)
}

fn split_numeric_suffix(value: &str) -> Option<(&str, i64)> {
    let split_index = value
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(index, _)| index)?;
    let (prefix, suffix) = value.split_at(split_index);
    suffix.parse::<i64>().ok().map(|number| (prefix, number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_reference_mutation_requires_explicit_candidates() {
        let result = mutate_request_reference(
            "https://example.test/api/orders/123",
            Some(r#"{"orderId":"123"}"#),
            &[],
            &[],
        );

        assert!(result.is_err());
    }

    #[test]
    fn resource_reference_mutates_explicit_path_candidate() {
        let (url, body, _applied_mutations) = mutate_request_reference(
            "https://example.test/api/orders/123",
            Some(r#"{"note":"keep"}"#),
            &[VerificationTarget {
                location: "pathSegment".to_string(),
                selector: "123".to_string(),
            }],
            &["123".to_string()],
        )
        .expect("mutation to succeed");

        assert_eq!(url, "https://example.test/api/orders/124");
        assert_eq!(body.as_deref(), Some(r#"{"note":"keep"}"#));
    }

    #[test]
    fn input_probe_mutates_explicit_json_target() {
        let plan = VerificationPlan {
            preferred_strategy: "input_probe".to_string(),
            candidate_targets: vec![VerificationTarget {
                location: "jsonBody".to_string(),
                selector: "email".to_string(),
            }],
            ..VerificationPlan::default()
        };
        let baseline = VerificationBaseline {
            source_request_id: Some(1),
            url: "https://example.test/api/profile".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some(r#"{"email":"user@example.test"}"#.to_string()),
            response_status: Some(200),
            response_headers: None,
            response_body: None,
        };

        let prepared = prepare_verification_request(&baseline, Some(&plan)).expect("probe");

        assert_eq!(prepared.strategy_used, "input_probe");
        assert_eq!(
            prepared.body.as_deref(),
            Some(r#"{"email":"sentinel-probe"}"#)
        );
        assert!(prepared.mutated);
    }

    #[test]
    fn cors_probe_sets_origin_header_override() {
        let plan = VerificationPlan {
            preferred_strategy: "cors_origin_probe".to_string(),
            ..VerificationPlan::default()
        };
        let baseline = VerificationBaseline {
            source_request_id: Some(1),
            url: "https://example.test/api/profile".to_string(),
            method: "GET".to_string(),
            request_headers: None,
            request_body: None,
            response_status: Some(200),
            response_headers: None,
            response_body: None,
        };

        let prepared = prepare_verification_request(&baseline, Some(&plan)).expect("probe");

        assert_eq!(
            prepared.header_overrides,
            vec![("Origin".to_string(), "https://sentinel.invalid".to_string())]
        );
    }
}
