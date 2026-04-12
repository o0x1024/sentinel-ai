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
    let baseline_denied = baseline
        .response_status
        .map(|status| !is_success_status(status as u16))
        .unwrap_or(false)
        || contains_denial_marker(baseline.response_body.as_deref());
    let mutation_became_more_favorable = is_success_status(response_status)
        || (!contains_denial_marker(Some(response_body)) && !response_body_matches);

    if baseline_denied && mutation_became_more_favorable {
        reasons.push(format!(
            "Business-parameter mutation changed a baseline denial into a more favorable response (status={}, bodyMatch={}).",
            response_status, response_body_matches
        ));
        return true;
    }

    reasons.push(format!(
        "Business-parameter mutation did not overturn the baseline denial semantics (status={}, bodyMatch={}, denialStillPresent={}).",
        response_status,
        response_body_matches,
        contains_denial_marker(Some(response_body))
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

fn is_success_status(status: u16) -> bool {
    (200..400).contains(&status)
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
            r#"{"success":true}"#,
            false,
            &[],
            &[],
            &[],
            VerificationExecutionMode::Single,
        );

        assert!(result.verified);
    }
}
