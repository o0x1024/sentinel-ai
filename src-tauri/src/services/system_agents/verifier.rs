use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use sentinel_db::{DatabaseService, TrafficEvidenceRecord};

use crate::services::system_agents::safety::SystemAgentSafetyPolicy;
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::system_agents::verification_plan::{
    build_baseline_from_evidence, build_baseline_from_proxy_request, extract_context_output,
    extract_context_payload, extract_target_request_id, extract_verification_plan,
    select_fallback_evidence, VerificationBaseline, VerificationPlan,
};
use crate::services::system_agents::verification_strategy::{
    prepare_verification_request, PreparedVerificationRequest, VerificationExecutionMode,
};
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
}

struct VerificationResponse {
    response_status: u16,
    response_headers_json: String,
    response_body: String,
    parallel_status_codes: Vec<u16>,
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

    let result = execute_verification(runtime, db, app_handle, finding_id, run_id, plan_hint).await?;
    Ok(serde_json::to_value(result)?)
}

async fn execute_verification(
    runtime: &SystemAgentRuntime,
    db: &DatabaseService,
    app_handle: &AppHandle,
    finding_id: &str,
    run_id: &str,
    plan_hint: Option<VerificationPlan>,
) -> Result<TrafficActiveVerifierResult> {
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
    let context_payload = extract_context_payload(&evidence);
    let (baseline, plan) = resolve_verification_context(db, &evidence, plan_hint, &finding.id).await?;

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

    let prepared_request = match prepare_verification_request(&baseline, plan.as_ref()) {
        Ok(result) => result,
        Err(error) => {
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
    };
    let PreparedVerificationRequest {
        url: request_url,
        body: request_body,
        mutated,
        notes: strategy_notes,
        strategy_used,
        execution_mode,
    } = prepared_request;

    if let Err(error) = safety_policy.ensure_url_in_scope(&request_url) {
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
    let headers = if strategy_used == "swap_identity" {
        match resolve_alternate_identity_headers(
            db,
            context_payload.as_ref(),
            baseline.source_request_id,
            baseline.request_headers.as_deref(),
        )
        .await?
        {
            Some(raw_headers) => parse_request_headers(Some(&raw_headers))?,
            None => {
                return persist_blocked_verification_result(
                    db,
                    app_handle,
                    &finding.id,
                    &baseline,
                    run_id,
                    "Verification plan requested identity swap, but no alternate authenticated request was available in the same cluster.".to_string(),
                    plan.as_ref(),
                )
                .await;
            }
        }
    } else {
        parse_request_headers(baseline.request_headers.as_deref())?
    };

    let verification_response = execute_verification_request(
        &client,
        &method,
        &request_url,
        &headers,
        request_body.as_deref(),
        execution_mode,
    )
    .await?;
    let matched_status = baseline
        .response_status
        .map(|status| status == verification_response.response_status as i32)
        .unwrap_or(false);
    let matched_body = compare_response_body(
        baseline.response_body.as_deref(),
        &verification_response.response_body,
    );
    let verified = matched_status || matched_body;

    let summary = if verified {
        format!(
            "System agent verification using {} matched baseline response (status={}, bodyMatch={}, mutated={}).",
            strategy_used, verification_response.response_status, matched_body, mutated
        )
    } else {
        format!(
            "System agent verification using {} diverged from baseline response (baselineStatus={:?}, replayStatus={}, bodyMatch={}, mutated={}).",
            strategy_used,
            baseline.response_status,
            verification_response.response_status,
            matched_body,
            mutated
        )
    };

    let evidence_id = format!("tev-{}", Uuid::new_v4());
    let verification_evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding.id.clone(),
        url: request_url.clone(),
        method: baseline.method.clone(),
        location: "system_agent_verification".to_string(),
        evidence_snippet: summary.clone(),
        request_headers: baseline.request_headers.clone(),
        request_body: request_body.clone(),
        response_status: Some(verification_response.response_status as i32),
        response_headers: Some(
            json!({
                "strategyUsed": strategy_used,
                "targetRequestId": baseline.source_request_id,
                "mutated": mutated,
                "notes": strategy_notes,
                "executionMode": match execution_mode {
                    VerificationExecutionMode::Single => "single",
                    VerificationExecutionMode::ConcurrentDuplicate => "concurrent_duplicate",
                },
                "parallelStatusCodes": verification_response.parallel_status_codes,
                "responseHeaders": serde_json::from_str::<Value>(&verification_response.response_headers_json)
                    .unwrap_or(Value::String(verification_response.response_headers_json.clone())),
            })
            .to_string(),
        ),
        response_body: truncate_text(Some(verification_response.response_body.clone())),
        timestamp: Utc::now(),
    };
    db.insert_traffic_evidence(&verification_evidence).await?;

    if verified {
        db.update_traffic_vulnerability_status(&finding.id, "reviewed")
            .await?;
    }

    let result = TrafficActiveVerifierResult {
        run_id: run_id.to_string(),
        profile_id: TRAFFIC_ACTIVE_VERIFIER_PROFILE_ID.to_string(),
        finding_id: finding.id,
        strategy: strategy_used,
        target_request_id: baseline.source_request_id,
        verified,
        matched_status,
        matched_body,
        response_status: Some(verification_response.response_status),
        summary,
        evidence_id: Some(evidence_id),
    };

    let _ = app_handle.emit("system-agent:verification-complete", &result);
    Ok(result)
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
    let target_request_id = extract_target_request_id(context_output.as_ref(), context_payload.as_ref())
        .or_else(|| plan.as_ref().and_then(|item| item.target_request_id));

    if let Some(request_id) = target_request_id {
        if let Some(record) = db.get_proxy_request_by_id(request_id).await? {
            return Ok((build_baseline_from_proxy_request(&record), plan));
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
) -> Result<VerificationResponse> {
    match execution_mode {
        VerificationExecutionMode::Single => {
            let response =
                send_single_request(client, method, request_url, headers, request_body).await?;
            Ok(VerificationResponse {
                response_status: response.status().as_u16(),
                response_headers_json: headers_to_json(response.headers())?,
                response_body: response.text().await.unwrap_or_default(),
                parallel_status_codes: vec![],
            })
        }
        VerificationExecutionMode::ConcurrentDuplicate => {
            let (first, second) = tokio::try_join!(
                send_single_request(client, method, request_url, headers, request_body),
                send_single_request(client, method, request_url, headers, request_body)
            )?;
            let first_status = first.status().as_u16();
            let second_status = second.status().as_u16();
            let first_headers = headers_to_json(first.headers())?;
            let first_body = first.text().await.unwrap_or_default();
            let _second_body = second.text().await.unwrap_or_default();
            Ok(VerificationResponse {
                response_status: first_status,
                response_headers_json: first_headers,
                response_body: first_body,
                parallel_status_codes: vec![first_status, second_status],
            })
        }
    }
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
    let evidence_id = format!("tev-{}", Uuid::new_v4());
    let evidence = TrafficEvidenceRecord {
        id: evidence_id.clone(),
        vuln_id: finding_id.to_string(),
        url: baseline.url.clone(),
        method: baseline.method.clone(),
        location: "system_agent_verification".to_string(),
        evidence_snippet: format!("System agent verification was blocked: {reason}"),
        request_headers: baseline.request_headers.clone(),
        request_body: baseline.request_body.clone(),
        response_status: None,
        response_headers: Some(
            json!({
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
        summary: format!("System agent verification was blocked: {reason}"),
        evidence_id: Some(evidence_id),
    };
    let _ = app_handle.emit("system-agent:verification-complete", &result);
    Ok(result)
}
