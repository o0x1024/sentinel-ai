use anyhow::{anyhow, Result};
use serde_json::Value;
use url::Url;

use crate::services::system_agents::verification_plan::{VerificationBaseline, VerificationPlan};

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
    pub mutated: bool,
    pub notes: Vec<String>,
    pub strategy_used: String,
    pub execution_mode: VerificationExecutionMode,
    pub execution_count: usize,
    pub sequence_mode: VerificationSequenceMode,
}

pub fn prepare_verification_request(
    baseline: &VerificationBaseline,
    plan: Option<&VerificationPlan>,
) -> Result<PreparedVerificationRequest> {
    let Some(plan) = plan else {
        return Ok(PreparedVerificationRequest {
            url: baseline.url.clone(),
            body: baseline.request_body.clone(),
            mutated: false,
            notes: vec![
                "No verification plan was provided; replayed the baseline request as-is."
                    .to_string(),
            ],
            strategy_used: "replay_as_is".to_string(),
            execution_mode: VerificationExecutionMode::Single,
            execution_count: 1,
            sequence_mode: VerificationSequenceMode::None,
        });
    };

    match plan.preferred_strategy.as_str() {
        "replay_as_is" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            false,
            plan,
            "Replayed the baseline request without mutation.",
            1,
            VerificationSequenceMode::None,
        )),
        "repeat_action" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            false,
            plan,
            "Repeated the original action with the same request payload.",
            normalize_repeat_count(plan.replay_count),
            VerificationSequenceMode::None,
        )),
        "skip_prerequisite" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            false,
            plan,
            "Replayed the target action independently to check whether prerequisite steps are enforced server-side.",
            1,
            VerificationSequenceMode::SkipPrerequisite,
        )),
        "reorder_sequence" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            false,
            plan,
            "Replayed the selected action out of its original request sequence to test ordering constraints.",
            1,
            VerificationSequenceMode::ReplayAfterTarget,
        )),
        "concurrent_submit" => Ok(PreparedVerificationRequest {
            url: baseline.url.clone(),
            body: baseline.request_body.clone(),
            mutated: false,
            notes: collect_strategy_notes(
                plan,
                "Replayed the target request concurrently to probe duplicate-success or race-sensitive behavior.",
            ),
            strategy_used: plan.preferred_strategy.clone(),
            execution_mode: VerificationExecutionMode::ConcurrentDuplicate,
            execution_count: normalize_concurrent_count(plan.concurrent_requests),
            sequence_mode: VerificationSequenceMode::None,
        }),
        "swap_identity" => Ok(single_request(
            baseline.url.clone(),
            baseline.request_body.clone(),
            false,
            plan,
            "Replayed the target request with an alternate authentication context from the same traffic cluster.",
            1,
            VerificationSequenceMode::None,
        )),
        "swap_resource_reference" => {
            let (mutated_url, mutated_body) = mutate_request_reference(
                &baseline.url,
                baseline.request_body.as_deref(),
                &plan.candidate_parameters,
            )?;
            Ok(single_request(
                mutated_url,
                mutated_body,
                true,
                plan,
                "Mutated a candidate resource reference before replay.",
                1,
                VerificationSequenceMode::None,
            ))
        }
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
    mutated: bool,
    plan: &VerificationPlan,
    summary: &str,
    execution_count: usize,
    sequence_mode: VerificationSequenceMode,
) -> PreparedVerificationRequest {
    PreparedVerificationRequest {
        url,
        body,
        mutated,
        notes: collect_strategy_notes(plan, summary),
        strategy_used: plan.preferred_strategy.clone(),
        execution_mode: VerificationExecutionMode::Single,
        execution_count,
        sequence_mode,
    }
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
    candidate_parameters: &[String],
) -> Result<(String, Option<String>)> {
    if let Some(mutated_url) = mutate_url_reference(url, candidate_parameters)? {
        return Ok((mutated_url, body.map(str::to_string)));
    }

    if let Some(mutated_body) = mutate_json_body_reference(body, candidate_parameters)? {
        return Ok((url.to_string(), Some(mutated_body)));
    }

    Err(anyhow!(
        "Verification plan requested resource mutation but no candidate parameter could be mutated"
    ))
}

fn mutate_url_reference(url: &str, candidate_parameters: &[String]) -> Result<Option<String>> {
    let mut parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let query_items = parsed.query_pairs().collect::<Vec<_>>();
    if !query_items.is_empty() {
        let mut updated = Vec::with_capacity(query_items.len());
        let mut mutated = false;
        for (key, value) in query_items {
            let key_text = key.to_string();
            let value_text = value.to_string();
            if !mutated && parameter_matches(&key_text, candidate_parameters) {
                updated.push((key_text, mutate_scalar_value(&value_text)));
                mutated = true;
            } else {
                updated.push((key_text, value_text));
            }
        }

        if mutated {
            parsed.query_pairs_mut().clear().extend_pairs(updated);
            return Ok(Some(parsed.to_string()));
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
        if let Some(mutated_segment) = mutate_path_segment(&candidate) {
            updated_segments[index] = mutated_segment;
            let next_path = format!("/{}", updated_segments.join("/"));
            parsed.set_path(&next_path);
            return Ok(Some(parsed.to_string()));
        }
    }

    Ok(None)
}

fn mutate_json_body_reference(
    body: Option<&str>,
    candidate_parameters: &[String],
) -> Result<Option<String>> {
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

    for key in prioritized_candidate_keys(candidate_parameters) {
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
            return Ok(Some(serde_json::to_string(&value)?));
        }
    }

    Ok(None)
}

fn prioritized_candidate_keys(candidate_parameters: &[String]) -> Vec<String> {
    let mut keys = candidate_parameters
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    for fallback in [
        "id",
        "userId",
        "user_id",
        "orderId",
        "order_id",
        "projectId",
        "resourceId",
    ] {
        if !keys.iter().any(|item| item == fallback) {
            keys.push(fallback.to_string());
        }
    }
    keys
}

fn parameter_matches(key: &str, candidate_parameters: &[String]) -> bool {
    if candidate_parameters.is_empty() {
        return matches!(
            key,
            "id" | "userId" | "user_id" | "orderId" | "order_id" | "projectId" | "resourceId"
        );
    }

    candidate_parameters
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(key))
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
