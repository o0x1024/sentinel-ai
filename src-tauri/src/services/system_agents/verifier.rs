use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::future::join_all;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use sentinel_db::{DatabaseService, TrafficEvidenceRecord};

use crate::services::system_agents::finding_lifecycle::TrafficFindingLifecycle;
use crate::services::system_agents::language::{is_chinese_ui_language, resolve_ui_language};
use crate::services::system_agents::prompts::verification_followup_planner_prompt;
use crate::services::system_agents::safety::SystemAgentSafetyPolicy;
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::system_agents::verification_assessment::assess_verification_result;
use crate::services::system_agents::verification_hypothesis_memory::{
    extract_hypothesis_state, normalize_hypothesis_state, VerificationHypothesisState,
};
use crate::services::system_agents::verification_plan::{
    build_baseline_from_context_payload, build_baseline_from_evidence, build_baseline_from_proxy_request,
    extract_context_output, extract_context_payload, extract_target_request_id,
    extract_verification_plan, normalize_verification_plan, select_fallback_evidence,
    VerificationBaseline, VerificationPlan,
};
use crate::services::system_agents::verification_request_diff::{
    build_request_diff_summary, RequestDiffSummary,
};
use crate::services::system_agents::verification_response_diff::{
    build_response_diff_summary, ResponseDiffSummary,
};
use crate::services::system_agents::verification_strategy::{
    prepare_verification_request, PreparedVerificationRequest, VerificationExecutionMode,
    VerificationSequenceMode,
};
use crate::services::system_agents::verification_mutation::AppliedVerificationMutation;
use crate::services::SystemAgentRuntime;

const TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID: &str = "traffic_active_verifier";
const MAX_RECORDED_BODY_LEN: usize = 16_384;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficActiveVerifierRequest {
    pub finding_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficActiveVerifierResult {
    pub run_id: String,
    pub profile_id: String,
    pub finding_id: String,
    pub strategy: String,
    pub target_request_id: Option<i64>,
    pub verified: bool,
    pub matched_status: bool,
    pub matched_body: bool,
    pub response_status: Option<u16>,
    pub summary: String,
    pub evidence_id: Option<String>,
    #[serde(default)]
    pub attempted_strategies: Vec<String>,
}

struct VerificationResponse {
    response_status: u16,
    response_headers_json: String,
    response_body: String,
    attempt_status_codes: Vec<u16>,
    parallel_status_codes: Vec<u16>,
    sequence_status_codes: Vec<u16>,
}

struct VerificationAttemptOutcome {
    strategy_used: String,
    request_url: String,
    verified: bool,
    matched_status: bool,
    matched_body: bool,
    response_status: Option<u16>,
    summary: String,
    evidence_id: String,
    blocked: bool,
    response_body_excerpt: Option<String>,
    applied_mutations: Vec<AppliedVerificationMutation>,
    request_diff_summary: RequestDiffSummary,
    response_diff_summary: ResponseDiffSummary,
    hypothesis_state: VerificationHypothesisState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificationAttemptRecord {
    strategy: String,
    verified: bool,
    blocked: bool,
    response_status: Option<u16>,
    matched_status: bool,
    matched_body: bool,
    summary: String,
    response_body_excerpt: Option<String>,
    applied_mutations: Vec<AppliedVerificationMutation>,
    request_diff_summary: RequestDiffSummary,
    response_diff_summary: ResponseDiffSummary,
    hypothesis_state: VerificationHypothesisState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationFollowupPlannerOutput {
    should_continue: bool,
    next_plan: Option<VerificationPlan>,
    #[serde(default)]
    hypothesis_state: VerificationHypothesisState,
}

pub async fn run_traffic_active_verifier(
    runtime: &Arc<SystemAgentRuntime>,
    db: &Arc<DatabaseService>,
    request: TrafficActiveVerifierRequest,
) -> Result<TrafficActiveVerifierResult> {
    let payload = serde_json::to_value(&request)?;
    let run = runtime
        .start_external_run(
            TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID,
            payload,
            Some("manual".to_string()),
        )
        .await?;

    match execute_verification(
        runtime,
        db.as_ref(),
        &runtime.app_handle(),
        &request.finding_id,
        &run.id,
        None,
        None,
    )
    .await
    {
        Ok(result) => {
            runtime
                .complete_external_run_success(
                    &run.id,
                    TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID,
                    serde_json::to_value(&result)?,
                )
                .await?;
            Ok(result)
        }
        Err(error) => {
            runtime
                .complete_external_run_failure(
                    &run.id,
                    TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID,
                    error.to_string(),
                )
                .await?;
            Err(error)
        }
    }
}

pub async fn verify_finding_for_runtime(
    runtime: &SystemAgentRuntime,
    db: &DatabaseService,
    app_handle: &AppHandle,
    payload: &Value,
    run_id: &str,
) -> Result<Value> {
    let finding_id = payload
        .get("findingId")
        .or_else(|| payload.get("finding_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing findingId in traffic hypothesis payload"))?;
    let plan_hint = payload
        .get("verificationPlan")
        .and_then(|raw| serde_json::from_value::<VerificationPlan>(raw.clone()).ok());
    let hypothesis_state_hint = extract_hypothesis_state(payload);

    let result =
        execute_verification(
            runtime,
            db,
            app_handle,
            finding_id,
            run_id,
            plan_hint,
            hypothesis_state_hint,
        )
        .await?;
    Ok(serde_json::to_value(result)?)
}

async fn execute_verification(
    runtime: &SystemAgentRuntime,
    db: &DatabaseService,
    app_handle: &AppHandle,
    finding_id: &str,
    run_id: &str,
    plan_hint: Option<VerificationPlan>,
    hypothesis_state_hint: Option<VerificationHypothesisState>,
) -> Result<TrafficActiveVerifierResult> {
    let ui_language = resolve_ui_language(db).await;
    let profile = runtime
        .get_profile(TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "System agent profile not found: {}",
                TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID
            )
        })?;
    let safety_policy = SystemAgentSafetyPolicy::from_profile(&profile);
    let tool_policy = SystemAgentToolPolicy::from_profile(&profile);
    tool_policy.validate()?;
    tool_policy.ensure_tool_allowed("active_replay")?;

    let finding = db
        .get_traffic_vulnerability_by_id(finding_id)
        .await?
        .ok_or_else(|| anyhow!("Traffic finding not found: {}", finding_id))?;
    let evidence = db.get_traffic_evidence_by_vuln_id(finding_id).await?;
    let context_output = extract_context_output(&evidence);
    let context_payload = extract_context_payload(&evidence);
    let (baseline, plan) =
        resolve_verification_context(db, &evidence, plan_hint, &finding.id).await?;

    if let Err(error) = safety_policy.ensure_active_replay_allowed() {
        return persist_blocked_verification_result(
            db,
            app_handle,
            &finding.id,
            &baseline,
            run_id,
            error.to_string(),
            plan.as_ref(),
        )
        .await;
    }

    let method = Method::from_bytes(baseline.method.as_bytes())
        .map_err(|_| anyhow!("Unsupported HTTP method: {}", baseline.method))?;
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut attempted_strategies = Vec::new();
    let mut attempt_records = Vec::new();
    let mut attempted_plan_signatures = BTreeSet::new();
    let mut hypothesis_state = hypothesis_state_hint
        .or_else(|| context_output.as_ref().and_then(extract_hypothesis_state))
        .unwrap_or_default();
    normalize_hypothesis_state(&mut hypothesis_state);
    let initial_strategy = plan
        .as_ref()
        .map(|item| item.preferred_strategy.clone())
        .unwrap_or_else(|| "replay_as_is".to_string());
    attempted_strategies.push(initial_strategy);
    if let Some(candidate_plan) = plan.as_ref() {
        attempted_plan_signatures.insert(plan_signature(candidate_plan));
    }
    let initial_outcome = execute_verification_attempt(
        db,
        &finding.id,
        &baseline,
        context_payload.as_ref(),
        &safety_policy,
        &client,
        &method,
        plan.as_ref(),
        &attempted_strategies,
        &hypothesis_state,
        1,
        1,
        &ui_language,
    )
    .await?;
    attempt_records.push(VerificationAttemptRecord {
        strategy: initial_outcome.strategy_used.clone(),
        verified: initial_outcome.verified,
        blocked: initial_outcome.blocked,
        response_status: initial_outcome.response_status,
        matched_status: initial_outcome.matched_status,
        matched_body: initial_outcome.matched_body,
        summary: initial_outcome.summary.clone(),
        response_body_excerpt: initial_outcome.response_body_excerpt.clone(),
        applied_mutations: initial_outcome.applied_mutations.clone(),
        request_diff_summary: initial_outcome.request_diff_summary.clone(),
        response_diff_summary: initial_outcome.response_diff_summary.clone(),
        hypothesis_state: initial_outcome.hypothesis_state.clone(),
    });
    let mut last_outcome = initial_outcome;

    if !last_outcome.verified {
        for replan_index in 0..2 {
            let planner_output = plan_followup_verification(
                runtime,
                &finding.id,
                &baseline,
                plan.as_ref(),
                context_payload.as_ref(),
                &attempt_records,
                &attempted_strategies,
                &hypothesis_state,
                replan_index + 1,
            )
            .await?;
            hypothesis_state = planner_output.hypothesis_state;
            if !planner_output.should_continue {
                break;
            }
            let Some(next_plan) = planner_output.next_plan else {
                break;
            };
            let signature = plan_signature(&next_plan);
            if attempted_plan_signatures.contains(&signature) {
                break;
            }
            attempted_plan_signatures.insert(signature);
            attempted_strategies.push(next_plan.preferred_strategy.clone());
            let attempt_index = attempt_records.len() + 1;
            let outcome = execute_verification_attempt(
                db,
                &finding.id,
                &baseline,
                context_payload.as_ref(),
                &safety_policy,
                &client,
                &method,
                Some(&next_plan),
                &attempted_strategies,
                &hypothesis_state,
                attempt_index,
                attempt_index,
                &ui_language,
            )
            .await?;
            attempt_records.push(VerificationAttemptRecord {
                strategy: outcome.strategy_used.clone(),
                verified: outcome.verified,
                blocked: outcome.blocked,
                response_status: outcome.response_status,
                matched_status: outcome.matched_status,
                matched_body: outcome.matched_body,
                summary: outcome.summary.clone(),
                response_body_excerpt: outcome.response_body_excerpt.clone(),
                applied_mutations: outcome.applied_mutations.clone(),
                request_diff_summary: outcome.request_diff_summary.clone(),
                response_diff_summary: outcome.response_diff_summary.clone(),
                hypothesis_state: outcome.hypothesis_state.clone(),
            });
            let verified = outcome.verified;
            last_outcome = outcome;
            if verified {
                break;
            }
        }
    }

    let final_outcome = last_outcome;

    if final_outcome.verified {
        db.update_traffic_vulnerability_status(
            &finding.id,
            TrafficFindingLifecycle::Verified.vulnerability_status(),
        )
        .await?;
        let _ = app_handle.emit(
            "scan:finding",
            json!({
                "vuln_id": finding.id,
                "vuln_type": finding.vuln_type,
                "severity": finding.severity,
                "status": TrafficFindingLifecycle::Verified.vulnerability_status(),
                "analysisStage": TrafficFindingLifecycle::Verified.key(),
                "url": final_outcome.request_url,
                "summary": final_outcome.summary,
                "timestamp": Utc::now().to_rfc3339(),
            }),
        );
    }

    let result = TrafficActiveVerifierResult {
        run_id: run_id.to_string(),
        profile_id: TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID.to_string(),
        finding_id: finding.id,
        strategy: final_outcome.strategy_used,
        target_request_id: baseline.source_request_id,
        verified: final_outcome.verified,
        matched_status: final_outcome.matched_status,
        matched_body: final_outcome.matched_body,
        response_status: final_outcome.response_status,
        summary: final_outcome.summary,
        evidence_id: Some(final_outcome.evidence_id),
        attempted_strategies,
    };

    let _ = app_handle.emit("system-agent:verification-complete", &result);
    Ok(result)
}

#[allow(dead_code)]
fn legacy_removed_marker() {
    let _ = compare_response_body as fn(Option<&str>, &str) -> bool;
}

async fn execute_verification_attempt(
    db: &DatabaseService,
    finding_id: &str,
    baseline: &VerificationBaseline,
    context_payload: Option<&Value>,
    safety_policy: &SystemAgentSafetyPolicy,
    client: &Client,
    method: &Method,
    plan: Option<&VerificationPlan>,
    attempted_strategies: &[String],
    hypothesis_state: &VerificationHypothesisState,
    attempt_index: usize,
    total_attempts: usize,
    ui_language: &str,
) -> Result<VerificationAttemptOutcome> {
    let active_plan = plan.cloned();
    let prepared_request = match prepare_verification_request(baseline, active_plan.as_ref()) {
        Ok(result) => result,
        Err(error) => {
            return record_blocked_verification_attempt(
                db,
                finding_id,
                baseline,
                plan,
                attempted_strategies,
                hypothesis_state,
                attempt_index,
                total_attempts,
                ui_language,
                error.to_string(),
            )
            .await;
        }
    };

    let headers = if prepared_request.strategy_used == "swap_identity" {
        match resolve_alternate_identity_headers(
            db,
            context_payload,
            baseline.source_request_id,
            baseline.request_headers.as_deref(),
        )
        .await?
        {
            Some(raw_headers) => parse_request_headers(Some(&raw_headers))?,
            None => {
                return record_blocked_verification_attempt(
                    db,
                    finding_id,
                    baseline,
                    plan,
                    attempted_strategies,
                    hypothesis_state,
                    attempt_index,
                    total_attempts,
                    ui_language,
                    "Verification plan requested identity swap, but no alternate authenticated request was available in the same cluster.".to_string(),
                )
                .await;
            }
        }
    } else {
        parse_request_headers(baseline.request_headers.as_deref())?
    };

    let PreparedVerificationRequest {
        url: request_url,
        body: request_body,
        mutated,
        notes: strategy_notes,
        strategy_used,
        execution_mode,
        execution_count,
        sequence_mode,
        applied_mutations,
    } = prepared_request;
    let request_diff_summary =
        build_request_diff_summary(baseline, &request_url, request_body.as_deref());

    if let Err(error) = safety_policy.ensure_url_in_scope(&request_url) {
        return record_blocked_verification_attempt(
            db,
            finding_id,
            baseline,
            active_plan.as_ref(),
            attempted_strategies,
            hypothesis_state,
            attempt_index,
            total_attempts,
            ui_language,
            error.to_string(),
        )
        .await;
    }

    let sequence_requests =
        resolve_sequence_requests(db, active_plan.as_ref(), baseline.source_request_id).await?;
    let verification_response = execute_verification_request(
        client,
        method,
        &request_url,
        &headers,
        request_body.as_deref(),
        execution_mode,
        execution_count,
        &sequence_requests,
        sequence_mode,
    )
    .await?;
    let matched_body = compare_response_body(
        baseline.response_body.as_deref(),
        &verification_response.response_body,
    );
    let assessment = assess_verification_result(
        baseline,
        active_plan.as_ref(),
        &strategy_used,
        verification_response.response_status,
        &verification_response.response_body,
        matched_body,
        &verification_response.attempt_status_codes,
        &verification_response.parallel_status_codes,
        &verification_response.sequence_status_codes,
        execution_mode,
    );
    let response_diff_summary = build_response_diff_summary(
        baseline,
        verification_response.response_status,
        &verification_response.response_headers_json,
        &verification_response.response_body,
    );

    let summary = format_attempt_summary(
        ui_language,
        &strategy_used,
        baseline.response_status,
        verification_response.response_status,
        &assessment.outcome,
        mutated,
        assessment.verified,
        attempt_index,
        total_attempts,
        attempted_strategies,
    );

    let evidence_id = format!("tev-{}", Uuid::new_v4());
    let verification_evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding_id.to_string(),
        url: request_url.clone(),
        method: baseline.method.clone(),
        location: "system_agent_verification".to_string(),
        evidence_snippet: summary.clone(),
        request_headers: baseline.request_headers.clone(),
        request_body: request_body.clone(),
        response_status: Some(verification_response.response_status as i32),
        response_headers: Some(
            json!({
                "analysisStage": if assessment.verified {
                    TrafficFindingLifecycle::Verified.key()
                } else {
                    "verification_attempt"
                },
                "verificationOutcome": assessment.outcome,
                "matchedStatus": assessment.matched_status,
                "matchedBody": assessment.matched_body,
                "strategyUsed": strategy_used,
                "targetRequestId": baseline.source_request_id,
                "mutated": mutated,
                "appliedMutations": applied_mutations,
                "requestDiffSummary": request_diff_summary,
                "responseDiffSummary": response_diff_summary,
                "hypothesisState": hypothesis_state,
                "notes": strategy_notes,
                "assessmentReasons": assessment.reasons,
                "executionMode": match execution_mode {
                    VerificationExecutionMode::Single => "single",
                    VerificationExecutionMode::ConcurrentDuplicate => "concurrent_duplicate",
                },
                "sequenceMode": match sequence_mode {
                    VerificationSequenceMode::None => "none",
                    VerificationSequenceMode::SkipPrerequisite => "skip_prerequisite",
                    VerificationSequenceMode::ReplayAfterTarget => "replay_after_target",
                },
                "executionCount": execution_count,
                "attemptIndex": attempt_index,
                "totalAttempts": total_attempts,
                "attemptedStrategies": attempted_strategies,
                "attemptStatusCodes": verification_response.attempt_status_codes,
                "parallelStatusCodes": verification_response.parallel_status_codes,
                "sequenceStatusCodes": verification_response.sequence_status_codes,
                "responseHeaders": serde_json::from_str::<Value>(&verification_response.response_headers_json)
                    .unwrap_or(Value::String(verification_response.response_headers_json.clone())),
            })
            .to_string(),
        ),
        response_body: truncate_text(Some(verification_response.response_body.clone())),
        timestamp: Utc::now(),
    };
    db.insert_traffic_evidence(&verification_evidence).await?;

    Ok(VerificationAttemptOutcome {
        strategy_used,
        request_url,
        verified: assessment.verified,
        matched_status: assessment.matched_status,
        matched_body: assessment.matched_body,
        response_status: Some(verification_response.response_status),
        summary,
        evidence_id,
        blocked: false,
        response_body_excerpt: truncate_text(Some(verification_response.response_body.clone())),
        applied_mutations,
        request_diff_summary,
        response_diff_summary,
        hypothesis_state: hypothesis_state.clone(),
    })
}

async fn record_blocked_verification_attempt(
    db: &DatabaseService,
    finding_id: &str,
    baseline: &VerificationBaseline,
    plan: Option<&VerificationPlan>,
    attempted_strategies: &[String],
    hypothesis_state: &VerificationHypothesisState,
    attempt_index: usize,
    total_attempts: usize,
    ui_language: &str,
    reason: String,
) -> Result<VerificationAttemptOutcome> {
    let strategy = plan
        .map(|item| item.preferred_strategy.clone())
        .unwrap_or_else(|| "replay_as_is".to_string());
    let summary = if is_chinese_ui_language(ui_language) {
        format!(
            "系统 Agent 第 {}/{} 次验证尝试（{}）被阻断：{}。已尝试策略：{}。",
            attempt_index,
            total_attempts,
            strategy,
            reason,
            attempted_strategies.join(" -> ")
        )
    } else {
        format!(
            "System agent verification attempt {}/{} using {} was blocked: {}. Attempted strategies: {}.",
            attempt_index,
            total_attempts,
            strategy,
            reason,
            attempted_strategies.join(" -> ")
        )
    };
    let evidence_id = format!("tev-{}", Uuid::new_v4());
    let evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding_id.to_string(),
        url: baseline.url.clone(),
        method: baseline.method.clone(),
        location: "system_agent_verification".to_string(),
        evidence_snippet: summary.clone(),
        request_headers: baseline.request_headers.clone(),
        request_body: baseline.request_body.clone(),
        response_status: None,
        response_headers: Some(
            json!({
                "analysisStage": "verification_blocked",
                "blocked": true,
                "reason": reason,
                "targetRequestId": baseline.source_request_id,
                "verificationPlan": plan,
                "strategyUsed": strategy,
                "appliedMutations": [],
                "requestDiffSummary": RequestDiffSummary::default(),
                "responseDiffSummary": ResponseDiffSummary::default(),
                "hypothesisState": hypothesis_state,
                "attemptIndex": attempt_index,
                "totalAttempts": total_attempts,
                "attemptedStrategies": attempted_strategies,
            })
            .to_string(),
        ),
        response_body: None,
        timestamp: Utc::now(),
    };
    db.insert_traffic_evidence(&evidence).await?;

    Ok(VerificationAttemptOutcome {
        strategy_used: strategy,
        request_url: baseline.url.clone(),
        verified: false,
        matched_status: false,
        matched_body: false,
        response_status: None,
        summary,
        evidence_id,
        blocked: true,
        response_body_excerpt: None,
        applied_mutations: Vec::new(),
        request_diff_summary: RequestDiffSummary::default(),
        response_diff_summary: ResponseDiffSummary::default(),
        hypothesis_state: hypothesis_state.clone(),
    })
}

fn format_attempt_summary(
    ui_language: &str,
    strategy_used: &str,
    baseline_status: Option<i32>,
    replay_status: u16,
    outcome: &str,
    mutated: bool,
    verified: bool,
    attempt_index: usize,
    total_attempts: usize,
    attempted_strategies: &[String],
) -> String {
    if is_chinese_ui_language(ui_language) {
        if verified {
            return format!(
                "系统 Agent 第 {}/{} 次验证使用 {} 完成确认（状态码={}, 结果={}, 请求变异={}，策略链={}）。",
                attempt_index,
                total_attempts,
                strategy_used,
                replay_status,
                outcome,
                mutated,
                attempted_strategies.join(" -> ")
            );
        }
        return format!(
            "系统 Agent 第 {}/{} 次验证使用 {} 尚未确认该假设（基线状态={:?}, 回放状态={}, 结果={}, 请求变异={}，策略链={}）。",
            attempt_index,
            total_attempts,
            strategy_used,
            baseline_status,
            replay_status,
            outcome,
            mutated,
            attempted_strategies.join(" -> ")
        );
    }

    if verified {
        return format!(
            "System agent attempt {}/{} using {} confirmed the hypothesis (status={}, outcome={}, mutated={}, strategyChain={}).",
            attempt_index,
            total_attempts,
            strategy_used,
            replay_status,
            outcome,
            mutated,
            attempted_strategies.join(" -> ")
        );
    }
    format!(
        "System agent attempt {}/{} using {} did not confirm the hypothesis (baselineStatus={:?}, replayStatus={}, outcome={}, mutated={}, strategyChain={}).",
        attempt_index,
        total_attempts,
        strategy_used,
        baseline_status,
        replay_status,
        outcome,
        mutated,
        attempted_strategies.join(" -> ")
    )
}

async fn plan_followup_verification(
    runtime: &SystemAgentRuntime,
    finding_id: &str,
    baseline: &VerificationBaseline,
    original_plan: Option<&VerificationPlan>,
    context_payload: Option<&Value>,
    attempt_records: &[VerificationAttemptRecord],
    attempted_strategies: &[String],
    hypothesis_state: &VerificationHypothesisState,
    replan_round: usize,
) -> Result<VerificationFollowupPlannerOutput> {
    if attempt_records.is_empty() {
        return Ok(VerificationFollowupPlannerOutput {
            should_continue: false,
            next_plan: None,
            hypothesis_state: hypothesis_state.clone(),
        });
    }

    let input = json!({
        "findingId": finding_id,
        "baseline": {
            "requestId": baseline.source_request_id,
            "url": baseline.url,
            "method": baseline.method,
            "requestHeaders": baseline.request_headers,
            "requestBody": baseline.request_body,
            "responseStatus": baseline.response_status,
            "responseHeaders": baseline.response_headers,
            "responseBody": baseline.response_body,
        },
        "originalPlan": original_plan,
        "contextPayload": context_payload,
        "attemptHistory": attempt_records,
        "attemptedStrategies": attempted_strategies,
        "hypothesisState": hypothesis_state,
        "replanRound": replan_round,
    });
    let user_input = serde_json::to_string_pretty(&input)?;
    let output = runtime
        .run_ad_hoc_json_llm(verification_followup_planner_prompt(), &user_input)
        .await?;
    let mut parsed = serde_json::from_value::<VerificationFollowupPlannerOutput>(output)
        .map_err(|error| anyhow!("Invalid follow-up verification planner output: {}", error))?;
    normalize_hypothesis_state(&mut parsed.hypothesis_state);
    if let Some(plan) = parsed.next_plan.as_mut() {
        normalize_verification_plan(plan);
    }
    Ok(parsed)
}

fn plan_signature(plan: &VerificationPlan) -> String {
    serde_json::to_string(plan).unwrap_or_else(|_| format!("strategy:{}", plan.preferred_strategy))
}

async fn resolve_verification_context(
    db: &DatabaseService,
    evidence: &[TrafficEvidenceRecord],
    plan_hint: Option<VerificationPlan>,
    finding_id: &str,
) -> Result<(VerificationBaseline, Option<VerificationPlan>)> {
    let context_output = extract_context_output(evidence);
    let context_payload = extract_context_payload(evidence);
    let plan = plan_hint.or_else(|| context_output.as_ref().and_then(extract_verification_plan));
    let target_request_id =
        extract_target_request_id(context_output.as_ref(), context_payload.as_ref())
            .or_else(|| plan.as_ref().and_then(|item| item.target_request_id));

    if let Some(request_id) = target_request_id {
        if let Some(record) = db.get_proxy_request_by_id(request_id).await? {
            return Ok((build_baseline_from_proxy_request(&record), plan));
        }
    }

    if let Some(payload) = context_payload.as_ref() {
        if let Some(baseline) = build_baseline_from_context_payload(payload) {
            return Ok((baseline, plan));
        }
    }

    if let Some(item) = select_fallback_evidence(evidence) {
        return Ok((build_baseline_from_evidence(item), plan));
    }

    Err(anyhow!(
        "Traffic finding has no usable baseline request or evidence: {}",
        finding_id
    ))
}

async fn resolve_alternate_identity_headers(
    db: &DatabaseService,
    context_payload: Option<&Value>,
    baseline_request_id: Option<i64>,
    baseline_headers: Option<&str>,
) -> Result<Option<String>> {
    let Some(context_payload) = context_payload else {
        return Ok(None);
    };
    let recent_ids = context_payload
        .get("clusterSummary")
        .and_then(|value| value.get("recentRequestIds"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for item in recent_ids {
        let Some(request_id) = item.as_i64() else {
            continue;
        };
        if baseline_request_id == Some(request_id) {
            continue;
        }
        let Some(record) = db.get_proxy_request_by_id(request_id).await? else {
            continue;
        };
        let Some(candidate_headers) = record.request_headers.clone() else {
            continue;
        };
        if candidate_headers.trim().is_empty() {
            continue;
        }
        if baseline_headers
            .map(|headers| headers.trim() == candidate_headers.trim())
            .unwrap_or(false)
        {
            continue;
        }
        return Ok(Some(candidate_headers));
    }

    Ok(None)
}

async fn resolve_sequence_requests(
    db: &DatabaseService,
    plan: Option<&VerificationPlan>,
    baseline_request_id: Option<i64>,
) -> Result<Vec<ProxySequenceRequest>> {
    let mut requests = Vec::new();
    let Some(plan) = plan else {
        return Ok(requests);
    };

    for request_id in &plan.sequence_request_ids {
        if baseline_request_id == Some(*request_id) {
            continue;
        }
        let Some(record) = db.get_proxy_request_by_id(*request_id).await? else {
            continue;
        };
        requests.push(ProxySequenceRequest {
            method: Method::from_bytes(record.method.as_bytes()).map_err(|_| {
                anyhow!(
                    "Unsupported HTTP method in sequence request: {}",
                    record.method
                )
            })?,
            url: record.url,
            headers: parse_request_headers(record.request_headers.as_deref())?,
            body: record.request_body,
        });
    }

    Ok(requests)
}

struct ProxySequenceRequest {
    method: Method,
    url: String,
    headers: HeaderMap,
    body: Option<String>,
}

fn parse_request_headers(raw: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    let Some(raw) = raw else {
        return Ok(headers);
    };

    let value: Value = serde_json::from_str(raw)?;
    let Some(map) = value.as_object() else {
        return Ok(headers);
    };

    for (name, value) in map {
        let name_lc = name.to_ascii_lowercase();
        if matches!(
            name_lc.as_str(),
            "host" | "content-length" | "transfer-encoding" | "connection"
        ) {
            continue;
        }

        let header_name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let rendered_value = match value {
            Value::String(text) => text.clone(),
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
            _ => value.to_string(),
        };
        let header_value = match HeaderValue::from_str(&rendered_value) {
            Ok(value) => value,
            Err(_) => continue,
        };
        headers.insert(header_name, header_value);
    }

    Ok(headers)
}

fn headers_to_json(headers: &HeaderMap) -> Result<String> {
    let mut map = serde_json::Map::new();
    for (name, value) in headers {
        let rendered = value.to_str().unwrap_or_default().to_string();
        map.insert(name.to_string(), Value::String(rendered));
    }
    Ok(Value::Object(map).to_string())
}

async fn execute_verification_request(
    client: &Client,
    method: &Method,
    request_url: &str,
    headers: &HeaderMap,
    request_body: Option<&str>,
    execution_mode: VerificationExecutionMode,
    execution_count: usize,
    sequence_requests: &[ProxySequenceRequest],
    sequence_mode: VerificationSequenceMode,
) -> Result<VerificationResponse> {
    match execution_mode {
        VerificationExecutionMode::Single => {
            let mut attempt_status_codes = Vec::with_capacity(execution_count);
            let mut last_response = None;
            for _ in 0..execution_count {
                let response =
                    send_single_request(client, method, request_url, headers, request_body).await?;
                attempt_status_codes.push(response.status().as_u16());
                last_response = Some(response);
            }
            let response =
                last_response.ok_or_else(|| anyhow!("No verification request executed"))?;
            let sequence_status_codes = match sequence_mode {
                VerificationSequenceMode::None | VerificationSequenceMode::SkipPrerequisite => {
                    Vec::new()
                }
                VerificationSequenceMode::ReplayAfterTarget => {
                    execute_sequence_requests(client, sequence_requests).await?
                }
            };
            Ok(VerificationResponse {
                response_status: response.status().as_u16(),
                response_headers_json: headers_to_json(response.headers())?,
                response_body: response.text().await.unwrap_or_default(),
                attempt_status_codes,
                parallel_status_codes: vec![],
                sequence_status_codes,
            })
        }
        VerificationExecutionMode::ConcurrentDuplicate => {
            let tasks = (0..execution_count)
                .map(|_| send_single_request(client, method, request_url, headers, request_body))
                .collect::<Vec<_>>();
            let responses = join_all(tasks).await;
            let mut resolved = Vec::with_capacity(execution_count);
            for item in responses {
                resolved.push(item?);
            }
            let mut responses_iter = resolved.into_iter();
            let first = responses_iter
                .next()
                .ok_or_else(|| anyhow!("No concurrent verification response captured"))?;
            let first_status = first.status().as_u16();
            let first_headers = headers_to_json(first.headers())?;
            let first_body = first.text().await.unwrap_or_default();
            let mut parallel_status_codes = vec![first_status];
            for response in responses_iter {
                parallel_status_codes.push(response.status().as_u16());
            }
            Ok(VerificationResponse {
                response_status: first_status,
                response_headers_json: first_headers,
                response_body: first_body,
                attempt_status_codes: parallel_status_codes.clone(),
                parallel_status_codes,
                sequence_status_codes: Vec::new(),
            })
        }
    }
}

async fn execute_sequence_requests(
    client: &Client,
    sequence_requests: &[ProxySequenceRequest],
) -> Result<Vec<u16>> {
    let mut status_codes = Vec::with_capacity(sequence_requests.len());
    for request in sequence_requests {
        let response = send_single_request(
            client,
            &request.method,
            &request.url,
            &request.headers,
            request.body.as_deref(),
        )
        .await?;
        status_codes.push(response.status().as_u16());
    }
    Ok(status_codes)
}

async fn send_single_request(
    client: &Client,
    method: &Method,
    request_url: &str,
    headers: &HeaderMap,
    request_body: Option<&str>,
) -> Result<reqwest::Response> {
    let mut builder = client.request(method.clone(), request_url);
    if !headers.is_empty() {
        builder = builder.headers(headers.clone());
    }
    if let Some(body) = request_body {
        if !body.is_empty() && method_allows_body(method) {
            builder = builder.body(body.to_string());
        }
    }
    Ok(builder.send().await?)
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

fn normalize_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(512)
        .collect()
}

fn truncate_text(text: Option<String>) -> Option<String> {
    text.map(|text| text.chars().take(MAX_RECORDED_BODY_LEN).collect())
}

fn method_allows_body(method: &Method) -> bool {
    !matches!(method.as_str(), "GET" | "HEAD")
}

async fn persist_blocked_verification_result(
    db: &DatabaseService,
    app_handle: &AppHandle,
    finding_id: &str,
    baseline: &VerificationBaseline,
    run_id: &str,
    reason: String,
    plan: Option<&VerificationPlan>,
) -> Result<TrafficActiveVerifierResult> {
    let ui_language = resolve_ui_language(db).await;
    let blocked_summary = if is_chinese_ui_language(&ui_language) {
        format!("系统 Agent 验证已被阻断：{reason}")
    } else {
        format!("System agent verification was blocked: {reason}")
    };
    let evidence_id = format!("tev-{}", Uuid::new_v4());
    let evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding_id.to_string(),
        url: baseline.url.clone(),
        method: baseline.method.clone(),
        location: "system_agent_verification".to_string(),
        evidence_snippet: blocked_summary.clone(),
        request_headers: baseline.request_headers.clone(),
        request_body: baseline.request_body.clone(),
        response_status: None,
        response_headers: Some(
            json!({
                "analysisStage": "verification_blocked",
                "blocked": true,
                "reason": reason,
                "targetRequestId": baseline.source_request_id,
                "verificationPlan": plan,
            })
            .to_string(),
        ),
        response_body: None,
        timestamp: Utc::now(),
    };
    db.insert_traffic_evidence(&evidence).await?;

    let result = TrafficActiveVerifierResult {
        run_id: run_id.to_string(),
        profile_id: TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID.to_string(),
        finding_id: finding_id.to_string(),
        strategy: plan
            .map(|item| item.preferred_strategy.clone())
            .unwrap_or_else(|| "replay_as_is".to_string()),
        target_request_id: baseline.source_request_id,
        verified: false,
        matched_status: false,
        matched_body: false,
        response_status: None,
        summary: blocked_summary,
        evidence_id: Some(evidence_id),
        attempted_strategies: plan
            .map(|item| vec![item.preferred_strategy.clone()])
            .unwrap_or_default(),
    };
    let _ = app_handle.emit("system-agent:verification-complete", &result);
    Ok(result)
}
