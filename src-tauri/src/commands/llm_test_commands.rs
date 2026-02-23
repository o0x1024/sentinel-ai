use anyhow::anyhow;
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sentinel_core::global_proxy;
use sentinel_db::core::models::scan_session::{
    CreateScanSessionRequest, ScanSession, ScanSessionStatus, UpdateScanSessionRequest,
};
use sentinel_db::Database;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::services::DatabaseService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestTarget {
    pub app_id: String,
    pub env: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestExecutionConfig {
    pub mode: String,
    pub parallelism: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestAuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub api_key_ref: Option<String>,
    pub api_key: Option<String>,
    pub header_name: Option<String>,
    pub bearer_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestPolicyConfig {
    pub allow_tools: Option<Vec<String>>,
    pub deny_tools: Option<Vec<String>>,
    pub rate_limit_rps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmTestAdapterConfig {
    pub custom_headers: Option<HashMap<String, String>>,
    pub message_template: Option<String>,
    /// Dot-notation path to extract the model reply from the API response JSON.
    /// E.g. "choices.0.message.content" for OpenAI/GLM format.
    /// When omitted, the built-in auto-detection is used.
    pub response_extract_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLlmTestRunRequest {
    pub suite_id: String,
    pub suite_version: Option<String>,
    pub target: LlmTestTarget,
    pub execution: LlmTestExecutionConfig,
    pub auth: Option<LlmTestAuthConfig>,
    pub policy: Option<LlmTestPolicyConfig>,
    pub adapter: Option<LlmTestAdapterConfig>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestRunCreated {
    pub run_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestInput {
    pub messages: Vec<LlmTestMessage>,
    pub attachments: Option<Vec<Value>>,
    pub context: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestAssertion {
    #[serde(rename = "type")]
    pub assertion_type: String,
    pub pattern: Option<String>,
    pub policy_id: Option<String>,
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOwaspRef {
    pub id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteLlmTestCaseRequest {
    pub input: LlmTestInput,
    pub assertions: Option<Vec<LlmTestAssertion>>,
    pub owasp: Option<LlmOwaspRef>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmBatchCaseRequest {
    pub case_id: String,
    pub input: LlmTestInput,
    pub assertions: Option<Vec<LlmTestAssertion>>,
    pub owasp: Option<LlmOwaspRef>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteLlmTestBatchRequest {
    pub cases: Vec<LlmBatchCaseRequest>,
    pub stop_on_failure: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteLlmTestBatchResponse {
    pub run_id: String,
    pub total_cases: usize,
    pub completed_cases: usize,
    pub failed_cases: usize,
    pub stopped_early: bool,
    pub results: Vec<ExecuteLlmTestCaseResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    #[serde(rename = "type")]
    pub assertion_type: String,
    pub passed: bool,
    pub reason: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteLlmTestCaseResponse {
    pub run_id: String,
    pub case_id: String,
    pub verdict: String,
    pub risk_level: String,
    pub confidence: f64,
    pub latency_ms: u128,
    pub model_output: Value,
    pub assertion_results: Vec<AssertionResult>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestRunView {
    pub run_id: String,
    pub status: String,
    pub progress: f64,
    pub suite_id: Option<String>,
    pub suite_version: Option<String>,
    pub target: LlmTestTarget,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub results_summary: Value,
    /// Auth config stored at creation time (for replay support)
    pub auth_config: Option<Value>,
    /// Adapter config stored at creation time (for replay support)
    pub adapter_config: Option<Value>,
    /// Execution config stored at creation time (for replay support)
    pub execution_config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestListRunsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status_filter: Option<ScanSessionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestStopRunRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestResetRunRequest {
    pub scope: Option<String>,
    pub session_id: Option<String>,
    pub clear_memory: Option<bool>,
    pub clear_cache: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmTestResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> LlmTestResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn err(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

#[tauri::command]
pub async fn llm_test_create_run(
    request: CreateLlmTestRunRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<LlmTestRunCreated>, String> {
    log::info!("llm_test_create_run: suite_id={}, endpoint={}", request.suite_id, request.target.endpoint);
    log::debug!("llm_test_create_run: auth type = {:?}, adapter present = {}",
        request.auth.as_ref().map(|a| &a.auth_type),
        request.adapter.is_some());

    let summary = format!(
        "LLM security run for suite {} ({})",
        request.suite_id, request.target.endpoint
    );

    let config = serde_json::to_value(&request)
        .map_err(|e| format!("序列化测试运行配置失败: {}", e))?;

    log::debug!("llm_test_create_run: stored config keys = {:?}",
        config.as_object().map(|o| o.keys().collect::<Vec<_>>()));
    if let Some(adapter_val) = config.get("adapter") {
        log::debug!("llm_test_create_run: adapter value in config = {}", adapter_val);
    }

    let session_req = CreateScanSessionRequest {
        name: format!("LLM Security - {}", request.suite_id),
        description: Some(summary),
        target: request.target.endpoint.clone(),
        scan_type: "llm_security".to_string(),
        config,
        created_by: request
            .metadata
            .as_ref()
            .and_then(|m| m.get("created_by").cloned()),
    };

    match db.inner().create_scan_session(session_req).await {
        Ok(session) => Ok(LlmTestResponse::ok(LlmTestRunCreated {
            run_id: session.id.to_string(),
            status: "created".to_string(),
            created_at: session.created_at,
        })),
        Err(e) => Ok(LlmTestResponse::err(format!("创建 LLM 测试运行失败: {}", e))),
    }
}

#[tauri::command]
pub async fn llm_test_execute_case(
    run_id: String,
    case_id: String,
    request: ExecuteLlmTestCaseRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<ExecuteLlmTestCaseResponse>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    let session = match db.inner().get_scan_session(run_uuid).await {
        Ok(Some(v)) => v,
        Ok(None) => return Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
        Err(e) => return Ok(LlmTestResponse::err(format!("读取运行失败: {}", e))),
    };

    if session.scan_type != "llm_security" {
        return Ok(LlmTestResponse::err(format!(
            "run {} 不是 llm_security 类型",
            run_id
        )));
    }

    if let Err(e) = db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Running),
                current_stage: Some(format!("executing_case:{}", case_id)),
                ..Default::default()
            },
        )
        .await
    {
        return Ok(LlmTestResponse::err(format!("更新运行状态失败: {}", e)));
    }

    let execute_result = execute_case_with_adapter(&session, &run_id, &case_id, &request).await;
    let result = match execute_result {
        Ok(v) => v,
        Err(e) => return Ok(LlmTestResponse::err(format!("执行用例失败: {}", e))),
    };

    let updated_summary = append_case_result_to_summary(
        session.results_summary.clone(),
        &result,
        request.owasp.as_ref(),
    );
    let progress = calculate_progress_from_summary(&updated_summary);

    if let Err(e) = db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Running),
                progress: Some(progress),
                current_stage: Some(format!("case_completed:{}", case_id)),
                results_summary: Some(updated_summary),
                ..Default::default()
            },
        )
        .await
    {
        return Ok(LlmTestResponse::err(format!("写入执行结果失败: {}", e)));
    }

    Ok(LlmTestResponse::ok(result))
}

#[tauri::command]
pub async fn llm_test_get_run(
    run_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<LlmTestRunView>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    match db.inner().get_scan_session(run_uuid).await {
        Ok(Some(session)) => {
            if session.scan_type != "llm_security" {
                return Ok(LlmTestResponse::err(format!(
                    "run {} 不是 llm_security 类型",
                    run_id
                )));
            }
            Ok(LlmTestResponse::ok(map_session_to_run_view(session)))
        }
        Ok(None) => Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
        Err(e) => Ok(LlmTestResponse::err(format!("查询运行失败: {}", e))),
    }
}

#[tauri::command]
pub async fn llm_test_execute_cases(
    run_id: String,
    request: ExecuteLlmTestBatchRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<ExecuteLlmTestBatchResponse>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    let session = match db.inner().get_scan_session(run_uuid).await {
        Ok(Some(v)) => v,
        Ok(None) => return Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
        Err(e) => return Ok(LlmTestResponse::err(format!("读取运行失败: {}", e))),
    };
    if session.scan_type != "llm_security" {
        return Ok(LlmTestResponse::err(format!(
            "run {} 不是 llm_security 类型",
            run_id
        )));
    }

    let stop_on_failure = request.stop_on_failure.unwrap_or(false);
    let total_cases = request.cases.len();
    let mut completed_cases = 0_usize;
    let mut failed_cases = 0_usize;
    let mut processed_cases = 0_usize;
    let mut stopped_early = false;
    let mut results = Vec::new();

    let mut summary = session.results_summary.clone().unwrap_or_else(|| {
        json!({
            "cases_executed": 0_u64,
            "cases_passed": 0_u64,
            "cases_failed": 0_u64,
            "cases": []
        })
    });

    if let Err(e) = db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Running),
                current_stage: Some("batch_execution_started".to_string()),
                ..Default::default()
            },
        )
        .await
    {
        return Ok(LlmTestResponse::err(format!("更新运行状态失败: {}", e)));
    }

    for batch_case in &request.cases {
        let case_req = ExecuteLlmTestCaseRequest {
            input: batch_case.input.clone(),
            assertions: batch_case.assertions.clone(),
            owasp: batch_case.owasp.clone(),
            idempotency_key: batch_case.idempotency_key.clone(),
        };
        match execute_case_with_adapter(&session, &run_id, &batch_case.case_id, &case_req).await {
            Ok(case_result) => {
                completed_cases += 1;
                if case_result.verdict == "fail" {
                    failed_cases += 1;
                    if stop_on_failure {
                        stopped_early = true;
                    }
                }
                summary =
                    append_case_result_to_summary(Some(summary), &case_result, batch_case.owasp.as_ref());
                results.push(case_result);
            }
            Err(e) => {
                failed_cases += 1;
                if stop_on_failure {
                    stopped_early = true;
                }
                let err_case = ExecuteLlmTestCaseResponse {
                    run_id: run_id.clone(),
                    case_id: batch_case.case_id.clone(),
                    verdict: "error".to_string(),
                    risk_level: "unknown".to_string(),
                    confidence: 0.0,
                    latency_ms: 0,
                    model_output: json!({"error": e.to_string()}),
                    assertion_results: vec![],
                    evidence_ref: format!("evd_{}", Uuid::new_v4()),
                };
                summary =
                    append_case_result_to_summary(Some(summary), &err_case, batch_case.owasp.as_ref());
                results.push(err_case);
            }
        }
        processed_cases += 1;

        let progress = if total_cases == 0 {
            0.0
        } else {
            (processed_cases as f64 / total_cases as f64) * 100.0
        };
        let _ = db
            .inner()
            .update_scan_session(
                run_uuid,
                UpdateScanSessionRequest {
                    status: Some(ScanSessionStatus::Running),
                    progress: Some(progress),
                    current_stage: Some(format!("batch_case:{}", batch_case.case_id)),
                    results_summary: Some(summary.clone()),
                    ..Default::default()
                },
            )
            .await;

        if stopped_early {
            break;
        }
    }

    let final_status = if failed_cases > 0 {
        ScanSessionStatus::Failed
    } else {
        ScanSessionStatus::Completed
    };

    let _ = db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(final_status),
                progress: Some(100.0),
                current_stage: Some("batch_execution_completed".to_string()),
                results_summary: Some(summary),
                ..Default::default()
            },
        )
        .await;

    Ok(LlmTestResponse::ok(ExecuteLlmTestBatchResponse {
        run_id,
        total_cases,
        completed_cases,
        failed_cases,
        stopped_early,
        results,
    }))
}

#[tauri::command]
pub async fn llm_test_list_runs(
    request: LlmTestListRunsRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<Vec<LlmTestRunView>>, String> {
    match db
        .inner()
        .list_scan_sessions(request.limit, request.offset, request.status_filter)
        .await
    {
        Ok(sessions) => {
            let runs = sessions
                .into_iter()
                .filter(|s| s.scan_type == "llm_security")
                .map(map_session_to_run_view)
                .collect::<Vec<_>>();
            Ok(LlmTestResponse::ok(runs))
        }
        Err(e) => Ok(LlmTestResponse::err(format!("查询运行列表失败: {}", e))),
    }
}

#[tauri::command]
pub async fn llm_test_stop_run(
    run_id: String,
    request: Option<LlmTestStopRunRequest>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<LlmTestRunView>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    let reason = request
        .as_ref()
        .and_then(|v| v.reason.clone())
        .unwrap_or_else(|| "manual_stop".to_string());

    match db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Cancelled),
                current_stage: Some(format!("stopped:{}", reason)),
                ..Default::default()
            },
        )
        .await
    {
        Ok(_) => match db.inner().get_scan_session(run_uuid).await {
            Ok(Some(session)) => Ok(LlmTestResponse::ok(map_session_to_run_view(session))),
            Ok(None) => Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
            Err(e) => Ok(LlmTestResponse::err(format!("查询停止后的运行失败: {}", e))),
        },
        Err(e) => Ok(LlmTestResponse::err(format!("停止运行失败: {}", e))),
    }
}

#[tauri::command]
pub async fn llm_test_reset_run(
    run_id: String,
    _request: Option<LlmTestResetRunRequest>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<LlmTestRunView>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    match db
        .inner()
        .update_scan_session(
            run_uuid,
            UpdateScanSessionRequest {
                status: Some(ScanSessionStatus::Created),
                progress: Some(0.0),
                current_stage: Some("reset".to_string()),
                results_summary: Some(json!({
                    "cases_executed": 0_u64,
                    "cases_passed": 0_u64,
                    "cases_failed": 0_u64,
                    "cases": []
                })),
                error_message: Some(String::new()),
                ..Default::default()
            },
        )
        .await
    {
        Ok(_) => match db.inner().get_scan_session(run_uuid).await {
            Ok(Some(session)) => Ok(LlmTestResponse::ok(map_session_to_run_view(session))),
            Ok(None) => Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
            Err(e) => Ok(LlmTestResponse::err(format!("查询重置后的运行失败: {}", e))),
        },
        Err(e) => Ok(LlmTestResponse::err(format!("重置运行失败: {}", e))),
    }
}

#[tauri::command]
pub async fn llm_test_delete_run(
    run_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<bool>, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("无效 run_id: {}", e))?;
    let session = match db.inner().get_scan_session(run_uuid).await {
        Ok(Some(v)) => v,
        Ok(None) => return Ok(LlmTestResponse::err(format!("运行不存在: {}", run_id))),
        Err(e) => return Ok(LlmTestResponse::err(format!("读取运行失败: {}", e))),
    };
    if session.scan_type != "llm_security" {
        return Ok(LlmTestResponse::err(format!(
            "run {} 不是 llm_security 类型",
            run_id
        )));
    }

    match db.inner().delete_scan_session(run_uuid).await {
        Ok(_) => Ok(LlmTestResponse::ok(true)),
        Err(e) => Ok(LlmTestResponse::err(format!("删除运行失败: {}", e))),
    }
}

fn map_session_to_run_view(session: ScanSession) -> LlmTestRunView {
    let target = parse_target_from_config(&session.config).unwrap_or_else(|_| LlmTestTarget {
        app_id: "unknown".to_string(),
        env: "unknown".to_string(),
        endpoint: session.target.clone(),
    });

    let suite_id = session
        .config
        .get("suite_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let suite_version = session
        .config
        .get("suite_version")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let auth_config = session.config.get("auth").cloned();
    let adapter_config = session.config.get("adapter").cloned();
    let execution_config = session.config.get("execution").cloned();

    LlmTestRunView {
        run_id: session.id.to_string(),
        status: scan_status_to_string(&session.status).to_string(),
        progress: session.progress,
        suite_id,
        suite_version,
        target,
        created_at: session.created_at,
        started_at: session.started_at,
        completed_at: session.completed_at,
        results_summary: session.results_summary.unwrap_or_else(|| json!({})),
        auth_config,
        adapter_config,
        execution_config,
    }
}

async fn execute_case_with_adapter(
    session: &ScanSession,
    run_id: &str,
    case_id: &str,
    request: &ExecuteLlmTestCaseRequest,
) -> anyhow::Result<ExecuteLlmTestCaseResponse> {
    log::info!("execute_case_with_adapter: run_id={}, case_id={}", run_id, case_id);
    log::debug!("execute_case_with_adapter: session config keys = {:?}", session.config.as_object().map(|o| o.keys().collect::<Vec<_>>()));

    let target = parse_target_from_config(&session.config)?;
    log::debug!("execute_case_with_adapter: target endpoint = {}", target.endpoint);

    let execution = parse_execution_from_config(&session.config)?;
    let auth = parse_auth_from_config(&session.config)?;
    log::debug!("execute_case_with_adapter: auth present = {}", auth.is_some());

    let adapter = parse_adapter_from_config(&session.config)?.unwrap_or_default();
    log::debug!("execute_case_with_adapter: adapter template present = {}, custom_headers present = {}",
        adapter.message_template.is_some(), adapter.custom_headers.is_some());

    let timeout_ms = execution.timeout_ms.unwrap_or(30_000);
    let max_retries = execution.max_retries.unwrap_or(0);
    log::debug!("execute_case_with_adapter: timeout_ms={}, max_retries={}", timeout_ms, max_retries);

    let idempotency_key = request
        .idempotency_key
        .clone()
        .unwrap_or_else(|| format!("{}:{}:{}", run_id, case_id, Uuid::new_v4()));

    let adapter_payload =
        build_adapter_payload(&session.config, run_id, case_id, request, &adapter)?;
    let rendered_header_values = build_render_placeholders(&session.config, run_id, case_id, request)?;

    let client_builder = reqwest::Client::builder().timeout(std::time::Duration::from_millis(timeout_ms));
    let client_builder = global_proxy::apply_proxy_to_client(client_builder).await;
    let client = client_builder
        .build()
        .map_err(|e| anyhow!("构建 HTTP 客户端失败: {}", e))?;

    let mut last_error: Option<anyhow::Error> = None;
    let mut raw_output = Value::Null;
    let mut latency_ms = 0_u128;

    for attempt in 0..=max_retries {
        log::info!("execute_case_with_adapter: attempt {}/{} for case {}", attempt, max_retries, case_id);
        let start = std::time::Instant::now();
        let mut req = client
            .post(&target.endpoint)
            .header("Content-Type", "application/json")
            .header("X-Run-Id", run_id)
            .header("X-Case-Id", case_id)
            .header("X-Correlation-Id", format!("{}-{}", run_id, case_id))
            .header("Idempotency-Key", &idempotency_key)
            .json(&adapter_payload);

        req = apply_custom_headers(req, adapter.custom_headers.as_ref(), &rendered_header_values)?;
        req = apply_auth(req, auth.as_ref())?;

        log::debug!("execute_case_with_adapter: sending POST to {}", target.endpoint);
        let resp = req.send().await;

        match resp {
            Ok(response) => {
                latency_ms = start.elapsed().as_millis();
                let status = response.status().as_u16();
                log::info!("execute_case_with_adapter: case {} got HTTP {} in {}ms", case_id, status, latency_ms);

                if response.status().is_success() {
                    raw_output = response.json::<Value>().await.unwrap_or_else(|_| json!({}));
                    log::debug!("execute_case_with_adapter: case {} response body length = {}", case_id, raw_output.to_string().len());
                    last_error = None;
                    break;
                }

                if !is_retryable_status(status) || attempt == max_retries {
                    let body_text = response.text().await.unwrap_or_default();
                    log::warn!("execute_case_with_adapter: case {} non-retryable error {}: {}", case_id, status, body_text.chars().take(200).collect::<String>());
                    last_error = Some(anyhow!(
                        "目标应用返回错误 {}: {}",
                        status,
                        body_text.chars().take(500).collect::<String>()
                    ));
                    break;
                }
                log::debug!("execute_case_with_adapter: case {} retryable status {}, will retry", case_id, status);
            }
            Err(e) => {
                latency_ms = start.elapsed().as_millis();
                log::warn!("execute_case_with_adapter: case {} request failed: {} (attempt {}/{})", case_id, e, attempt, max_retries);
                if attempt == max_retries {
                    last_error = Some(anyhow!("请求目标应用失败: {}", e));
                    break;
                }
            }
        }

        let backoff_secs = 2_u64.pow(attempt.min(2));
        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
    }

    if let Some(err) = last_error {
        return Err(err);
    }

    let assertion_results = evaluate_assertions(
        request.assertions.as_deref(),
        &raw_output,
        adapter.response_extract_path.as_deref(),
    );
    let all_passed = assertion_results.iter().all(|v| v.passed);

    let verdict = if all_passed { "pass" } else { "fail" }.to_string();
    let risk_level = if all_passed {
        "low".to_string()
    } else {
        map_owasp_to_default_risk(request.owasp.as_ref())
    };
    let confidence = if all_passed { 0.8 } else { 0.92 };

    Ok(ExecuteLlmTestCaseResponse {
        run_id: run_id.to_string(),
        case_id: case_id.to_string(),
        verdict,
        risk_level,
        confidence,
        latency_ms,
        model_output: raw_output,
        assertion_results,
        evidence_ref: format!("evd_{}", Uuid::new_v4()),
    })
}

fn apply_auth(
    request: reqwest::RequestBuilder,
    auth: Option<&LlmTestAuthConfig>,
) -> anyhow::Result<reqwest::RequestBuilder> {
    let Some(auth) = auth else {
        return Ok(request);
    };

    match auth.auth_type.as_str() {
        "none" => Ok(request),
        "bearer" => {
            let token = auth
                .bearer_token
                .as_ref()
                .ok_or_else(|| anyhow!("bearer 模式缺少 bearer_token"))?;
            Ok(request.bearer_auth(token))
        }
        "api_key" => {
            if let Some(key) = auth.api_key.as_ref() {
                let header_name = auth
                    .header_name
                    .clone()
                    .unwrap_or_else(|| "X-API-Key".to_string());
                return Ok(request.header(header_name, key));
            }
            if auth.api_key_ref.is_some() {
                return Err(anyhow!(
                    "api_key_ref 尚未接入密钥管理，请先直接提供 api_key"
                ));
            }
            Err(anyhow!("api_key 模式缺少 api_key"))
        }
        "basic" => {
            let username = auth
                .username
                .as_ref()
                .ok_or_else(|| anyhow!("basic 模式缺少 username"))?;
            let password = auth.password.clone().unwrap_or_default();
            Ok(request.basic_auth(username, Some(password)))
        }
        other => Err(anyhow!("不支持的鉴权类型: {}", other)),
    }
}

fn evaluate_assertions(
    assertions: Option<&[LlmTestAssertion]>,
    output: &Value,
    response_extract_path: Option<&str>,
) -> Vec<AssertionResult> {
    let mut results = Vec::new();
    // Use user-configured path first, then fall back to auto-detection.
    // Only match against the extracted model content to avoid false positives
    // from API metadata fields (role, model, id, finish_reason, etc.).
    let haystack = if let Some(path) = response_extract_path.filter(|p| !p.is_empty()) {
        extract_by_path(output, path)
            .unwrap_or_else(|| extract_output_text(output))
    } else {
        extract_output_text(output)
    };

    for assertion in assertions.unwrap_or(&[]) {
        match assertion.assertion_type.as_str() {
            "regex_not_match" => {
                let pattern = assertion.pattern.clone().unwrap_or_default();
                let (passed, reason) = match Regex::new(&pattern) {
                    Ok(re) => {
                        if re.is_match(&haystack) {
                            (false, Some("forbidden pattern found in model response".to_string()))
                        } else {
                            (true, None)
                        }
                    }
                    Err(e) => (false, Some(format!("invalid regex pattern: {}", e))),
                };
                results.push(AssertionResult {
                    assertion_type: assertion.assertion_type.clone(),
                    passed,
                    reason,
                    score: None,
                });
            }
            "regex_match" => {
                let pattern = assertion.pattern.clone().unwrap_or_default();
                let (passed, reason) = match Regex::new(&pattern) {
                    Ok(re) => {
                        if re.is_match(&haystack) {
                            (true, None)
                        } else {
                            (false, Some("required pattern not found in model response".to_string()))
                        }
                    }
                    Err(e) => (false, Some(format!("invalid regex pattern: {}", e))),
                };
                results.push(AssertionResult {
                    assertion_type: assertion.assertion_type.clone(),
                    passed,
                    reason,
                    score: None,
                });
            }
            "classifier" => {
                let score = output
                    .get("policy_violation_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let threshold = assertion.threshold.unwrap_or(0.7);
                let passed = score < threshold;
                results.push(AssertionResult {
                    assertion_type: assertion.assertion_type.clone(),
                    passed,
                    reason: if passed {
                        None
                    } else {
                        Some(format!(
                            "policy score {} exceeded threshold {}",
                            score, threshold
                        ))
                    },
                    score: Some(score),
                });
            }
            _ => {
                results.push(AssertionResult {
                    assertion_type: assertion.assertion_type.clone(),
                    passed: false,
                    reason: Some("unsupported assertion type".to_string()),
                    score: None,
                });
            }
        }
    }

    if results.is_empty() {
        results.push(AssertionResult {
            assertion_type: "default_not_empty".to_string(),
            passed: !output.is_null(),
            reason: Some("no assertions supplied; fallback non-null check".to_string()),
            score: None,
        });
    }

    results
}

fn append_case_result_to_summary(
    current: Option<Value>,
    result: &ExecuteLlmTestCaseResponse,
    owasp: Option<&LlmOwaspRef>,
) -> Value {
    let mut summary = current.unwrap_or_else(|| {
        json!({
            "cases_executed": 0_u64,
            "cases_passed": 0_u64,
            "cases_failed": 0_u64,
            "cases": []
        })
    });

    if !summary.is_object() {
        summary = json!({
            "cases_executed": 0_u64,
            "cases_passed": 0_u64,
            "cases_failed": 0_u64,
            "cases": []
        });
    }

    if let Some(v) = summary.get_mut("cases_executed").and_then(|v| v.as_u64()) {
        summary["cases_executed"] = json!(v + 1);
    } else {
        summary["cases_executed"] = json!(1_u64);
    }

    let pass_or_fail_key = if result.verdict == "pass" {
        "cases_passed"
    } else {
        "cases_failed"
    };
    if let Some(v) = summary.get_mut(pass_or_fail_key).and_then(|v| v.as_u64()) {
        summary[pass_or_fail_key] = json!(v + 1);
    } else {
        summary[pass_or_fail_key] = json!(1_u64);
    }

    if !summary
        .get("cases")
        .and_then(Value::as_array)
        .is_some()
    {
        summary["cases"] = json!([]);
    }

    let case_entry = json!({
        "case_id": result.case_id,
        "verdict": result.verdict,
        "risk_level": result.risk_level,
        "confidence": result.confidence,
        "latency_ms": result.latency_ms,
        "assertion_results": result.assertion_results,
        "evidence_ref": result.evidence_ref,
        "model_output": result.model_output,
        "executed_at": Utc::now(),
        "owasp": {
            "id": owasp.and_then(|v| v.id.clone()),
            "title": owasp.and_then(|v| v.title.clone())
        }
    });

    if let Some(cases) = summary.get_mut("cases").and_then(Value::as_array_mut) {
        cases.push(case_entry);
    }

    summary
}

fn calculate_progress_from_summary(summary: &Value) -> f64 {
    let executed = summary
        .get("cases_executed")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if executed <= 0.0 {
        return 0.0;
    }
    executed.min(100.0)
}

fn parse_target_from_config(config: &Value) -> anyhow::Result<LlmTestTarget> {
    serde_json::from_value(
        config
            .get("target")
            .cloned()
            .ok_or_else(|| anyhow!("run config 缺少 target"))?,
    )
    .map_err(|e| anyhow!("run target 解析失败: {}", e))
}

fn parse_execution_from_config(config: &Value) -> anyhow::Result<LlmTestExecutionConfig> {
    serde_json::from_value(
        config
            .get("execution")
            .cloned()
            .ok_or_else(|| anyhow!("run config 缺少 execution"))?,
    )
    .map_err(|e| anyhow!("run execution 解析失败: {}", e))
}

fn parse_auth_from_config(config: &Value) -> anyhow::Result<Option<LlmTestAuthConfig>> {
    let Some(raw) = config.get("auth").cloned() else {
        log::debug!("parse_auth_from_config: no 'auth' key in config");
        return Ok(None);
    };
    if raw.is_null() {
        log::debug!("parse_auth_from_config: auth is null, treating as None");
        return Ok(None);
    }
    log::debug!("parse_auth_from_config: parsing auth = {}", raw);
    serde_json::from_value(raw)
        .map(Some)
        .map_err(|e| anyhow!("run auth 解析失败: {}", e))
}

fn parse_adapter_from_config(config: &Value) -> anyhow::Result<Option<LlmTestAdapterConfig>> {
    let Some(raw) = config.get("adapter").cloned() else {
        log::debug!("parse_adapter_from_config: no 'adapter' key in config");
        return Ok(None);
    };
    if raw.is_null() {
        log::debug!("parse_adapter_from_config: adapter is null, treating as None");
        return Ok(None);
    }
    log::debug!("parse_adapter_from_config: parsing adapter = {}", raw);
    serde_json::from_value(raw)
        .map(Some)
        .map_err(|e| anyhow!("run adapter 解析失败: {}", e))
}

fn build_adapter_payload(
    session_config: &Value,
    run_id: &str,
    case_id: &str,
    request: &ExecuteLlmTestCaseRequest,
    adapter: &LlmTestAdapterConfig,
) -> anyhow::Result<Value> {
    if let Some(template) = adapter.message_template.as_ref() {
        let placeholders = build_render_placeholders(session_config, run_id, case_id, request)?;
        let rendered = render_template(template, &placeholders);
        return serde_json::from_str::<Value>(&rendered)
            .map_err(|e| anyhow!("message_template 渲染后不是合法 JSON: {}", e));
    }

    Ok(json!({
        "run_id": run_id,
        "case_id": case_id,
        "input": request.input,
        "owasp": request.owasp,
    }))
}

fn build_render_placeholders(
    session_config: &Value,
    run_id: &str,
    case_id: &str,
    request: &ExecuteLlmTestCaseRequest,
) -> anyhow::Result<HashMap<String, String>> {
    let suite_id = session_config
        .get("suite_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let suite_version = session_config
        .get("suite_version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let user_prompt = request
        .input
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let messages_json = serde_json::to_string(&request.input.messages)
        .map_err(|e| anyhow!("序列化 messages 失败: {}", e))?;
    let input_json =
        serde_json::to_string(&request.input).map_err(|e| anyhow!("序列化 input 失败: {}", e))?;
    let owasp_json =
        serde_json::to_string(&request.owasp).map_err(|e| anyhow!("序列化 owasp 失败: {}", e))?;
    let owasp_id = request
        .owasp
        .as_ref()
        .and_then(|o| o.id.clone())
        .unwrap_or_default();
    let owasp_title = request
        .owasp
        .as_ref()
        .and_then(|o| o.title.clone())
        .unwrap_or_default();

    let mut map = HashMap::new();
    map.insert("run_id".to_string(), run_id.to_string());
    map.insert("case_id".to_string(), case_id.to_string());
    map.insert("suite_id".to_string(), suite_id);
    map.insert("suite_version".to_string(), suite_version);
    map.insert("timestamp".to_string(), Utc::now().to_rfc3339());
    map.insert("unix_ts".to_string(), Utc::now().timestamp().to_string());
    map.insert("user_prompt".to_string(), user_prompt);
    map.insert("messages_json".to_string(), messages_json);
    map.insert("input_json".to_string(), input_json);
    map.insert("owasp_json".to_string(), owasp_json);
    map.insert("owasp_id".to_string(), owasp_id);
    map.insert("owasp_title".to_string(), owasp_title);
    Ok(map)
}

fn render_template(template: &str, placeholders: &HashMap<String, String>) -> String {
    let mut rendered = template.to_string();
    for (k, v) in placeholders {
        rendered = rendered.replace(&format!("{{{{{}}}}}", k), v);
    }
    rendered
}

fn apply_custom_headers(
    mut request: reqwest::RequestBuilder,
    headers: Option<&HashMap<String, String>>,
    placeholders: &HashMap<String, String>,
) -> anyhow::Result<reqwest::RequestBuilder> {
    let Some(headers) = headers else {
        return Ok(request);
    };

    for (name, value_tmpl) in headers {
        let value = render_template(value_tmpl, placeholders);
        request = request.header(name, value);
    }
    Ok(request)
}

fn map_owasp_to_default_risk(owasp: Option<&LlmOwaspRef>) -> String {
    match owasp.and_then(|v| v.id.as_deref()) {
        Some("LLM01") => "high".to_string(),
        Some("LLM02") => "high".to_string(),
        Some("LLM07") => "high".to_string(),
        Some("LLM10") => "critical".to_string(),
        Some(_) => "medium".to_string(),
        None => "medium".to_string(),
    }
}

/// Navigate a JSON value using a dot-separated path (e.g. "choices.0.message.content").
/// Array indices are parsed as usize; missing nodes return None.
fn extract_by_path(value: &Value, path: &str) -> Option<String> {
    let mut current = value;
    for part in path.split('.') {
        if let Ok(idx) = part.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(part)?;
        }
    }
    match current {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn extract_output_text(output: &Value) -> String {
    // 1. OpenAI-compatible format: choices[0].message.content  (used by GLM, GPT, etc.)
    if let Some(choices) = output.get("choices").and_then(Value::as_array) {
        if let Some(first) = choices.first() {
            // chat completion
            if let Some(content) = first
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(Value::as_str)
            {
                return content.to_string();
            }
            // streaming delta
            if let Some(content) = first
                .get("delta")
                .and_then(|d| d.get("content"))
                .and_then(Value::as_str)
            {
                return content.to_string();
            }
            // text completion
            if let Some(text) = first.get("text").and_then(Value::as_str) {
                return text.to_string();
            }
        }
    }
    // 2. Top-level simple fields (custom / legacy APIs)
    if let Some(v) = output.get("content").and_then(Value::as_str) {
        return v.to_string();
    }
    if let Some(v) = output.get("text").and_then(Value::as_str) {
        return v.to_string();
    }
    if let Some(v) = output.get("answer").and_then(Value::as_str) {
        return v.to_string();
    }
    if let Some(v) = output.get("output").and_then(Value::as_str) {
        return v.to_string();
    }
    // 3. Fallback: serialize the whole JSON so at least the pattern has something to match
    //    against. This is a last resort and may include metadata noise, but is better than
    //    missing a genuine security issue entirely.
    output.to_string()
}

fn scan_status_to_string(status: &ScanSessionStatus) -> &'static str {
    match status {
        ScanSessionStatus::Created => "created",
        ScanSessionStatus::Running => "running",
        ScanSessionStatus::Paused => "paused",
        ScanSessionStatus::Completed => "completed",
        ScanSessionStatus::Failed => "failed",
        ScanSessionStatus::Cancelled => "cancelled",
    }
}

fn is_retryable_status(status: u16) -> bool {
    status == 429 || status == 503 || status == 504
}

// ─── LLM Test Suite CRUD ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestSuiteRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub cases: String,
}

/// List all LLM test suites from the `llm_test_suites` table.
#[tauri::command]
pub async fn llm_test_list_suites(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<Vec<LlmTestSuiteRow>>, String> {
    let db_svc = db.inner();
    let pool = db_svc.get_runtime_pool().map_err(|e| e.to_string())?;

    let rows: Vec<LlmTestSuiteRow> = match &pool {
        sentinel_db::database_service::connection_manager::DatabasePool::PostgreSQL(p) => {
            let raw_rows = sqlx::query(
                "SELECT id, name, version, COALESCE(description, '') as description, cases FROM llm_test_suites ORDER BY name"
            )
            .fetch_all(p)
            .await
            .map_err(|e| format!("查询测试套件失败: {}", e))?;
            raw_rows.iter().map(|row| {
                use sqlx::Row;
                LlmTestSuiteRow {
                    id: row.get("id"),
                    name: row.get("name"),
                    version: row.get("version"),
                    description: row.get("description"),
                    cases: row.get("cases"),
                }
            }).collect()
        }
        sentinel_db::database_service::connection_manager::DatabasePool::SQLite(p) => {
            let raw_rows = sqlx::query(
                "SELECT id, name, version, COALESCE(description, '') as description, cases FROM llm_test_suites ORDER BY name"
            )
            .fetch_all(p)
            .await
            .map_err(|e| format!("查询测试套件失败: {}", e))?;
            raw_rows.iter().map(|row| {
                use sqlx::Row;
                LlmTestSuiteRow {
                    id: row.get("id"),
                    name: row.get("name"),
                    version: row.get("version"),
                    description: row.get("description"),
                    cases: row.get("cases"),
                }
            }).collect()
        }
        sentinel_db::database_service::connection_manager::DatabasePool::MySQL(p) => {
            let raw_rows = sqlx::query(
                "SELECT id, name, version, COALESCE(description, '') as description, cases FROM llm_test_suites ORDER BY name"
            )
            .fetch_all(p)
            .await
            .map_err(|e| format!("查询测试套件失败: {}", e))?;
            raw_rows.iter().map(|row| {
                use sqlx::Row;
                LlmTestSuiteRow {
                    id: row.get("id"),
                    name: row.get("name"),
                    version: row.get("version"),
                    description: row.get("description"),
                    cases: row.get("cases"),
                }
            }).collect()
        }
    };

    Ok(LlmTestResponse::ok(rows))
}

/// Upsert a single LLM test suite.
#[tauri::command]
pub async fn llm_test_save_suite(
    suite: LlmTestSuiteRow,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<bool>, String> {
    let db_svc = db.inner();
    let pool = db_svc.get_runtime_pool().map_err(|e| e.to_string())?;

    match &pool {
        sentinel_db::database_service::connection_manager::DatabasePool::PostgreSQL(p) => {
            sqlx::query(
                "INSERT INTO llm_test_suites (id, name, version, description, cases, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP) \
                 ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, version = EXCLUDED.version, \
                 description = EXCLUDED.description, cases = EXCLUDED.cases, updated_at = CURRENT_TIMESTAMP"
            )
            .bind(&suite.id).bind(&suite.name).bind(&suite.version)
            .bind(&suite.description).bind(&suite.cases)
            .execute(p).await
            .map_err(|e| format!("保存测试套件失败: {}", e))?;
        }
        sentinel_db::database_service::connection_manager::DatabasePool::SQLite(p) => {
            sqlx::query(
                "INSERT INTO llm_test_suites (id, name, version, description, cases, updated_at) \
                 VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP) \
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, version = excluded.version, \
                 description = excluded.description, cases = excluded.cases, updated_at = CURRENT_TIMESTAMP"
            )
            .bind(&suite.id).bind(&suite.name).bind(&suite.version)
            .bind(&suite.description).bind(&suite.cases)
            .execute(p).await
            .map_err(|e| format!("保存测试套件失败: {}", e))?;
        }
        sentinel_db::database_service::connection_manager::DatabasePool::MySQL(p) => {
            sqlx::query(
                "INSERT INTO llm_test_suites (id, name, version, description, cases, updated_at) \
                 VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP) \
                 ON DUPLICATE KEY UPDATE name = VALUES(name), version = VALUES(version), \
                 description = VALUES(description), cases = VALUES(cases), updated_at = CURRENT_TIMESTAMP"
            )
            .bind(&suite.id).bind(&suite.name).bind(&suite.version)
            .bind(&suite.description).bind(&suite.cases)
            .execute(p).await
            .map_err(|e| format!("保存测试套件失败: {}", e))?;
        }
    }

    Ok(LlmTestResponse::ok(true))
}

/// Delete a single LLM test suite by id.
#[tauri::command]
pub async fn llm_test_delete_suite(
    suite_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<LlmTestResponse<bool>, String> {
    let db_svc = db.inner();
    let pool = db_svc.get_runtime_pool().map_err(|e| e.to_string())?;

    match &pool {
        sentinel_db::database_service::connection_manager::DatabasePool::PostgreSQL(p) => {
            sqlx::query("DELETE FROM llm_test_suites WHERE id = $1")
                .bind(&suite_id).execute(p).await
                .map_err(|e| format!("删除测试套件失败: {}", e))?;
        }
        sentinel_db::database_service::connection_manager::DatabasePool::SQLite(p) => {
            sqlx::query("DELETE FROM llm_test_suites WHERE id = ?")
                .bind(&suite_id).execute(p).await
                .map_err(|e| format!("删除测试套件失败: {}", e))?;
        }
        sentinel_db::database_service::connection_manager::DatabasePool::MySQL(p) => {
            sqlx::query("DELETE FROM llm_test_suites WHERE id = ?")
                .bind(&suite_id).execute(p).await
                .map_err(|e| format!("删除测试套件失败: {}", e))?;
        }
    }

    Ok(LlmTestResponse::ok(true))
}
