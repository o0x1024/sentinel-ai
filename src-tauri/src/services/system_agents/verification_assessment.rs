use crate::services::system_agents::verification_plan::{VerificationBaseline, VerificationPlan};
use crate::services::system_agents::verification_strategy::VerificationExecutionMode;

pub struct VerificationAssessment {
    pub verified: bool,
    pub matched_status: bool,
    pub matched_body: bool,
    pub outcome: &'static str,
    pub reasons: Vec<String>,
}

pub fn assess_verification_result(
    baseline: &VerificationBaseline,
    plan: Option<&VerificationPlan>,
    strategy_used: &str,
    response_status: u16,
    response_headers_json: &str,
    response_body: &str,
    response_body_matches: bool,
    attempt_status_codes: &[u16],
    parallel_status_codes: &[u16],
    sequence_status_codes: &[u16],
    execution_mode: VerificationExecutionMode,
) -> VerificationAssessment {
    let matched_status = baseline
        .response_status
        .map(|status| status == response_status as i32)
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let verified = match strategy_used {
        "repeat_action" => assess_repeat_action(
            matched_status,
            response_body_matches,
            response_status,
            attempt_status_codes,
            &mut reasons,
        ),
        "concurrent_submit" => assess_concurrent_submit(
            matched_status,
            response_body_matches,
            response_status,
            parallel_status_codes,
            &mut reasons,
        ),
        "swap_identity" => assess_identity_or_resource_swap(
            "identity",
            matched_status,
            response_body_matches,
            response_status,
            &mut reasons,
        ),
        "swap_resource_reference" => assess_identity_or_resource_swap(
            "resource",
            matched_status,
            response_body_matches,
            response_status,
            &mut reasons,
        ),
        "mutate_business_parameter" => assess_business_parameter_mutation(
            baseline,
            response_status,
            response_body,
            response_body_matches,
            &mut reasons,
        ),
        "input_probe" => assess_input_probe(response_status, response_body, &mut reasons),
        "path_traversal_probe" => {
            assess_path_traversal_probe(response_status, response_body, &mut reasons)
        }
        "cors_origin_probe" => {
            assess_cors_origin_probe(response_status, response_headers_json, &mut reasons)
        }
        "header_policy_probe" => {
            assess_header_policy_probe(response_status, response_headers_json, &mut reasons)
        }
        "oast_probe" => assess_oast_probe(
            response_status,
            response_headers_json,
            response_body,
            &mut reasons,
        ),
        "skip_prerequisite" => assess_sequence_bypass(
            "prerequisite",
            matched_status,
            response_body_matches,
            response_status,
            sequence_status_codes,
            &mut reasons,
        ),
        "reorder_sequence" => assess_sequence_bypass(
            "ordering",
            matched_status,
            response_body_matches,
            response_status,
            sequence_status_codes,
            &mut reasons,
        ),
        _ => assess_replay_as_is(
            matched_status,
            response_body_matches,
            response_status,
            &mut reasons,
        ),
    };

    if let Some(plan) = plan {
        reasons.extend(
            plan.notes
                .iter()
                .filter(|note| !note.trim().is_empty())
                .cloned(),
        );
    }

    let outcome = if verified {
        "verification_confirmed"
    } else if execution_mode == VerificationExecutionMode::ConcurrentDuplicate
        && parallel_status_codes
            .iter()
            .any(|status| is_success_status(*status))
    {
        "duplicate_request_not_confirmed"
    } else {
        "verification_not_confirmed"
    };

    VerificationAssessment {
        verified,
        matched_status,
        matched_body: response_body_matches,
        outcome,
        reasons,
    }
}

fn assess_replay_as_is(
    matched_status: bool,
    matched_body: bool,
    response_status: u16,
    reasons: &mut Vec<String>,
) -> bool {
    if matched_status || matched_body {
        reasons.push(format!(
            "Replay matched the original response characteristics (statusMatch={}, bodyMatch={}).",
            matched_status, matched_body
        ));
        return true;
    }
    reasons.push(format!(
        "Replay diverged from baseline (status={}, statusMatch={}, bodyMatch={}).",
        response_status, matched_status, matched_body
    ));
    false
}

fn assess_repeat_action(
    matched_status: bool,
    matched_body: bool,
    response_status: u16,
    attempt_status_codes: &[u16],
    reasons: &mut Vec<String>,
) -> bool {
    let success_count = attempt_status_codes
        .iter()
        .filter(|status| is_success_status(**status))
        .count();
    let repeated_success = success_count >= 2
        && is_success_status(response_status)
        && (matched_status || matched_body);
    if repeated_success {
        reasons.push(format!(
            "Repeated action kept succeeding across multiple sequential attempts (attemptStatuses={:?}, statusMatch={}, bodyMatch={}).",
            attempt_status_codes, matched_status, matched_body
        ));
        return true;
    }
    reasons.push(format!(
        "Repeated action did not preserve the original success pattern across attempts (attemptStatuses={:?}, statusMatch={}, bodyMatch={}).",
        attempt_status_codes, matched_status, matched_body
    ));
    false
}

fn assess_concurrent_submit(
    matched_status: bool,
    matched_body: bool,
    response_status: u16,
    parallel_status_codes: &[u16],
    reasons: &mut Vec<String>,
) -> bool {
    let success_count = parallel_status_codes
        .iter()
        .filter(|status| is_success_status(**status))
        .count();
    if success_count >= 2 && (matched_status || matched_body) {
        reasons.push(format!(
            "Concurrent duplicate submissions both succeeded (parallelStatuses={:?}, bodyMatch={}).",
            parallel_status_codes, matched_body
        ));
        return true;
    }
    reasons.push(format!(
        "Concurrent duplicate submissions were not both accepted like the baseline (status={}, parallelStatuses={:?}, bodyMatch={}).",
        response_status, parallel_status_codes, matched_body
    ));
    false
}

fn assess_identity_or_resource_swap(
    swap_kind: &str,
    matched_status: bool,
    matched_body: bool,
    response_status: u16,
    reasons: &mut Vec<String>,
) -> bool {
    let still_accessible = is_success_status(response_status) && (matched_status || matched_body);
    if still_accessible {
        reasons.push(format!(
            "Swapped {} request still returned a baseline-like successful response (status={}, statusMatch={}, bodyMatch={}).",
            swap_kind, response_status, matched_status, matched_body
        ));
        return true;
    }
    reasons.push(format!(
        "Swapped {} request no longer resembled the baseline response (status={}, statusMatch={}, bodyMatch={}).",
        swap_kind, response_status, matched_status, matched_body
    ));
    false
}

fn assess_business_parameter_mutation(
    baseline: &VerificationBaseline,
    response_status: u16,
    response_body: &str,
    response_body_matches: bool,
    reasons: &mut Vec<String>,
) -> bool {
    let baseline_status = baseline.response_status.map(|status| status as u16);
    let baseline_denial_marker = contains_denial_marker(baseline.response_body.as_deref());
    let replay_denial_marker = contains_denial_marker(Some(response_body));
    let baseline_denied = baseline
        .response_status
        .map(|status| !is_success_status(status as u16))
        .unwrap_or(false)
        || baseline_denial_marker;
    let mutation_became_more_favorable =
        is_success_status(response_status) || (!replay_denial_marker && !response_body_matches);

    if baseline_denied && mutation_became_more_favorable {
        reasons.push(format!(
            "Business-parameter mutation changed a baseline denial into a more favorable response (status={}, bodyMatch={}).",
            response_status, response_body_matches
        ));
        return true;
    }

    let baseline_success =
        baseline_status.map(is_success_status).unwrap_or(false) && !baseline_denial_marker;
    let success_semantics_preserved = baseline_status
        .map(|status| same_success_family(status, response_status))
        .unwrap_or(false)
        || response_body_matches;
    if baseline_success
        && is_success_status(response_status)
        && !replay_denial_marker
        && success_semantics_preserved
    {
        reasons.push(format!(
            "Business-parameter mutation was still accepted after changing a sensitive client-controlled parameter (baselineStatus={:?}, replayStatus={}, bodyMatch={}).",
            baseline.response_status, response_status, response_body_matches
        ));
        return true;
    }

    reasons.push(format!(
        "Business-parameter mutation was not rejected strongly enough to confirm a server-side validation gap (baselineStatus={:?}, replayStatus={}, bodyMatch={}, denialStillPresent={}).",
        baseline.response_status,
        response_status,
        response_body_matches,
        replay_denial_marker
    ));
    false
}

fn assess_sequence_bypass(
    bypass_kind: &str,
    matched_status: bool,
    matched_body: bool,
    response_status: u16,
    sequence_status_codes: &[u16],
    reasons: &mut Vec<String>,
) -> bool {
    let target_succeeded = is_success_status(response_status) && (matched_status || matched_body);
    let follow_up_success_count = sequence_status_codes
        .iter()
        .filter(|status| is_success_status(**status))
        .count();
    let follow_up_ratio = if sequence_status_codes.is_empty() {
        None
    } else {
        Some(follow_up_success_count as f32 / sequence_status_codes.len() as f32)
    };
    let bypass_succeeded = match bypass_kind {
        "ordering" => target_succeeded && follow_up_ratio.map(|ratio| ratio >= 0.5).unwrap_or(true),
        _ => target_succeeded,
    };
    if bypass_succeeded {
        reasons.push(format!(
            "The {}-sensitive action still succeeded outside its expected sequence (status={}, statusMatch={}, bodyMatch={}, followUpStatuses={:?}).",
            bypass_kind, response_status, matched_status, matched_body, sequence_status_codes
        ));
        return true;
    }
    reasons.push(format!(
        "The {}-sensitive action did not preserve the baseline response when replayed out of sequence (status={}, statusMatch={}, bodyMatch={}, followUpStatuses={:?}).",
        bypass_kind, response_status, matched_status, matched_body, sequence_status_codes
    ));
    false
}

fn assess_input_probe(
    response_status: u16,
    response_body: &str,
    reasons: &mut Vec<String>,
) -> bool {
    let normalized = response_body.to_ascii_lowercase();
    let matched = is_success_status(response_status)
        && [
            "sql syntax",
            "syntax error",
            "unterminated",
            "database error",
            "odbc",
            "mysql",
            "postgres",
            "sqlite",
            "oracle",
            "template error",
            "sentinel-probe",
        ]
        .iter()
        .any(|marker| normalized.contains(marker));
    if matched {
        reasons.push(format!(
            "Input probe triggered a parser-like error or direct reflection in the replay response (status={}).",
            response_status
        ));
        return true;
    }
    reasons.push(format!(
        "Input probe did not trigger a stable parser, database, or reflection signal (status={}).",
        response_status
    ));
    false
}

fn assess_path_traversal_probe(
    response_status: u16,
    response_body: &str,
    reasons: &mut Vec<String>,
) -> bool {
    let normalized = response_body.to_ascii_lowercase();
    let matched = is_success_status(response_status)
        && [
            "root:x:",
            "[extensions]",
            "[fonts]",
            "/bin/bash",
            "drivers/etc/hosts",
        ]
        .iter()
        .any(|marker| normalized.contains(marker));
    if matched {
        reasons.push(format!(
            "Traversal probe returned content resembling a sensitive local file (status={}).",
            response_status
        ));
        return true;
    }
    reasons.push(format!(
        "Traversal probe did not return a recognizable sensitive-file signature (status={}).",
        response_status
    ));
    false
}

fn assess_cors_origin_probe(
    response_status: u16,
    response_headers_json: &str,
    reasons: &mut Vec<String>,
) -> bool {
    let headers = parse_headers(response_headers_json);
    let allow_origin = header_value(&headers, "access-control-allow-origin").unwrap_or_default();
    let allow_credentials = header_value(&headers, "access-control-allow-credentials")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let matched = is_success_status(response_status)
        && (allow_origin == "https://sentinel.invalid"
            || (allow_origin == "*" && allow_credentials));
    if matched {
        reasons.push(format!(
            "CORS probe observed a permissive ACAO response for the synthetic Origin header (acao={}, credentials={}).",
            allow_origin, allow_credentials
        ));
        return true;
    }
    reasons.push(format!(
        "CORS probe did not observe an overly permissive ACAO policy (acao={}, credentials={}).",
        allow_origin, allow_credentials
    ));
    false
}

fn assess_header_policy_probe(
    response_status: u16,
    response_headers_json: &str,
    reasons: &mut Vec<String>,
) -> bool {
    let headers = parse_headers(response_headers_json);
    let has_xfo = header_value(&headers, "x-frame-options").is_some();
    let has_frame_ancestors = header_value(&headers, "content-security-policy")
        .map(|value| value.to_ascii_lowercase().contains("frame-ancestors"))
        .unwrap_or(false);
    let matched = is_success_status(response_status) && !has_xfo && !has_frame_ancestors;
    if matched {
        reasons.push(
            "Header policy probe found neither X-Frame-Options nor CSP frame-ancestors."
                .to_string(),
        );
        return true;
    }
    reasons.push(format!(
        "Header policy probe found framing protections or the response was not replay-successful (xfo={}, frameAncestors={}, status={}).",
        has_xfo, has_frame_ancestors, response_status
    ));
    false
}

fn assess_oast_probe(
    response_status: u16,
    response_headers_json: &str,
    response_body: &str,
    reasons: &mut Vec<String>,
) -> bool {
    let headers = parse_headers(response_headers_json);
    let normalized_body = response_body.to_ascii_lowercase();
    let reflected = normalized_body.contains("oast.invalid")
        || headers
            .values()
            .any(|value| value.to_ascii_lowercase().contains("oast.invalid"));
    if is_success_status(response_status) && reflected {
        reasons.push(
            "Outbound-style probe was reflected back in the response, which is enough to keep the hypothesis active.".to_string(),
        );
        return true;
    }
    reasons.push(
        "Outbound-style probe did not leave a response-side signal; network-side correlation is still required.".to_string(),
    );
    false
}

fn is_success_status(status: u16) -> bool {
    (200..400).contains(&status)
}

fn same_success_family(left: u16, right: u16) -> bool {
    matches!(
        (left, right),
        (200..=299, 200..=299) | (300..=399, 300..=399)
    )
}

fn contains_denial_marker(text: Option<&str>) -> bool {
    let Some(text) = text else {
        return false;
    };
    let normalized = text.to_ascii_lowercase();
    [
        "insufficient_funds",
        "insufficient funds",
        "insufficient balance",
        "balance_insufficient",
        "not_enough_balance",
        "余额不足",
        "资金不足",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn parse_headers(raw: &str) -> std::collections::BTreeMap<String, String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .map(|map| {
            map.into_iter()
                .map(|(key, value)| {
                    let rendered = match value {
                        serde_json::Value::String(text) => text,
                        serde_json::Value::Array(items) => items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "),
                        other => other.to_string(),
                    };
                    (key.to_ascii_lowercase(), rendered)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn header_value<'a>(
    headers: &'a std::collections::BTreeMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    headers.get(&name.to_ascii_lowercase()).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirms_business_parameter_mutation_when_denial_disappears() {
        let baseline = VerificationBaseline {
            source_request_id: Some(1),
            url: "https://shop.test/api/checkout".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some(r#"{"quantity":1}"#.to_string()),
            response_status: Some(402),
            response_headers: None,
            response_body: Some(r#"{"error":"INSUFFICIENT_FUNDS"}"#.to_string()),
        };

        let result = assess_verification_result(
            &baseline,
            None,
            "mutate_business_parameter",
            200,
            "{}",
            r#"{"success":true}"#,
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(result.verified);
    }

    #[test]
    fn confirms_business_parameter_mutation_when_success_semantics_are_preserved() {
        let baseline = VerificationBaseline {
            source_request_id: Some(2),
            url: "https://shop.test/cart".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some("price=133700&quantity=1".to_string()),
            response_status: Some(302),
            response_headers: None,
            response_body: Some(String::new()),
        };

        let result = assess_verification_result(
            &baseline,
            None,
            "mutate_business_parameter",
            302,
            "{}",
            "",
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(result.verified);
    }

    #[test]
    fn rejects_business_parameter_mutation_when_success_semantics_do_not_hold() {
        let baseline = VerificationBaseline {
            source_request_id: Some(3),
            url: "https://shop.test/cart".to_string(),
            method: "POST".to_string(),
            request_headers: None,
            request_body: Some("price=133700&quantity=1".to_string()),
            response_status: Some(302),
            response_headers: None,
            response_body: Some(String::new()),
        };

        let result = assess_verification_result(
            &baseline,
            None,
            "mutate_business_parameter",
            500,
            "{}",
            "internal error",
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(!result.verified);
    }

    #[test]
    fn confirms_cors_probe_when_origin_is_reflected() {
        let baseline = VerificationBaseline {
            source_request_id: Some(4),
            url: "https://api.test/data".to_string(),
            method: "GET".to_string(),
            request_headers: None,
            request_body: None,
            response_status: Some(200),
            response_headers: None,
            response_body: Some(String::new()),
        };

        let result = assess_verification_result(
            &baseline,
            None,
            "cors_origin_probe",
            200,
            r#"{"access-control-allow-origin":"https://sentinel.invalid","access-control-allow-credentials":"true"}"#,
            "",
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(result.verified);
    }

    #[test]
    fn confirms_header_policy_probe_when_framing_headers_are_missing() {
        let baseline = VerificationBaseline {
            source_request_id: Some(5),
            url: "https://app.test/page".to_string(),
            method: "GET".to_string(),
            request_headers: None,
            request_body: None,
            response_status: Some(200),
            response_headers: None,
            response_body: Some(String::new()),
        };

        let result = assess_verification_result(
            &baseline,
            None,
            "header_policy_probe",
            200,
            r#"{"content-type":"text/html"}"#,
            "<html></html>",
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(result.verified);
    }
}
