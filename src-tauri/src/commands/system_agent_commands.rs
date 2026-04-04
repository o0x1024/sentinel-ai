use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::services::system_agents::tool_policy::validate_profile_tool_policy;
use crate::services::system_agents::{
    ensure_default_system_agent_profiles, run_plugin_fix_agent, run_traffic_active_verifier,
    PluginFixAgentRequest, PluginFixAgentResult, SystemAgentDispatchResult, SystemAgentRuntime,
    TrafficActiveVerifierRequest, TrafficActiveVerifierResult,
};
use crate::services::AiServiceManager;
use sentinel_db::{
    Database, DatabaseService, SystemAgentBindingRecord, SystemAgentProfileRecord,
    SystemAgentProfileVersionRecord, SystemAgentRunRecord, TrafficEvidenceRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: impl ToString) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentBindingPayload {
    pub id: Option<String>,
    pub profile_id: Option<String>,
    pub event_name: String,
    pub filter: Option<Value>,
    pub priority: i64,
    pub enabled: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentProfilePayload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: String,
    pub capability: String,
    pub enabled: bool,
    pub trigger_mode: String,
    pub base_prompt_id: Option<String>,
    pub prompt_patch: Option<String>,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
    pub required_tools: Vec<String>,
    pub optional_tools: Vec<String>,
    pub forbidden_tools: Vec<String>,
    pub trigger_events: Vec<String>,
    pub budget: Option<Value>,
    pub safety_policy: Option<Value>,
    pub cooldown_secs: i64,
    pub max_concurrency: i64,
    pub risk_level: String,
    pub visibility: String,
    pub bindings: Vec<SystemAgentBindingPayload>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentProfileSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: String,
    pub capability: String,
    pub enabled: bool,
    pub trigger_mode: String,
    pub risk_level: String,
    pub visibility: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentRunPayload {
    pub id: String,
    pub profile_id: String,
    pub trigger_event: Option<String>,
    pub status: String,
    pub input_summary: Option<Value>,
    pub output: Option<Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentProfileVersionPayload {
    pub id: String,
    pub profile_id: String,
    pub snapshot: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentFindingFeedbackRequest {
    pub finding_id: String,
    pub feedback_type: String,
    pub notes: Option<String>,
}

fn parse_json_value(raw: &str) -> Option<Value> {
    serde_json::from_str(raw).ok()
}

fn parse_json_array(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn to_json_object_string(value: Option<&Value>) -> Result<String, String> {
    match value {
        Some(value) => serde_json::to_string(value).map_err(|e| e.to_string()),
        None => Ok("{}".to_string()),
    }
}

fn to_json_array_string(values: &[String]) -> Result<String, String> {
    serde_json::to_string(values).map_err(|e| e.to_string())
}

fn binding_from_record(record: SystemAgentBindingRecord) -> SystemAgentBindingPayload {
    SystemAgentBindingPayload {
        id: Some(record.id),
        profile_id: Some(record.profile_id),
        event_name: record.event_name,
        filter: parse_json_value(&record.filter_json),
        priority: record.priority,
        enabled: record.enabled,
        created_at: Some(record.created_at),
        updated_at: Some(record.updated_at),
    }
}

fn summary_from_record(record: SystemAgentProfileRecord) -> SystemAgentProfileSummary {
    SystemAgentProfileSummary {
        id: record.id,
        name: record.name,
        description: record.description,
        mode: record.mode,
        capability: record.capability,
        enabled: record.enabled,
        trigger_mode: record.trigger_mode,
        risk_level: record.risk_level,
        visibility: record.visibility,
        updated_at: record.updated_at,
    }
}

fn run_from_record(record: SystemAgentRunRecord) -> SystemAgentRunPayload {
    SystemAgentRunPayload {
        id: record.id,
        profile_id: record.profile_id,
        trigger_event: record.trigger_event,
        status: record.status,
        input_summary: parse_json_value(&record.input_summary_json),
        output: record.output_json.as_deref().and_then(parse_json_value),
        error_message: record.error_message,
        started_at: record.started_at,
        finished_at: record.finished_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn version_from_record(record: SystemAgentProfileVersionRecord) -> SystemAgentProfileVersionPayload {
    SystemAgentProfileVersionPayload {
        id: record.id,
        profile_id: record.profile_id,
        snapshot: parse_json_value(&record.snapshot_json),
        created_at: record.created_at,
    }
}

fn detail_from_record(
    record: SystemAgentProfileRecord,
    bindings: Vec<SystemAgentBindingRecord>,
) -> SystemAgentProfilePayload {
    SystemAgentProfilePayload {
        id: record.id,
        name: record.name,
        description: record.description,
        mode: record.mode,
        capability: record.capability,
        enabled: record.enabled,
        trigger_mode: record.trigger_mode,
        base_prompt_id: record.base_prompt_id,
        prompt_patch: record.prompt_patch,
        input_schema: parse_json_value(&record.input_schema_json),
        output_schema: parse_json_value(&record.output_schema_json),
        required_tools: parse_json_array(&record.required_tools_json),
        optional_tools: parse_json_array(&record.optional_tools_json),
        forbidden_tools: parse_json_array(&record.forbidden_tools_json),
        trigger_events: parse_json_array(&record.trigger_events_json),
        budget: parse_json_value(&record.budget_json),
        safety_policy: parse_json_value(&record.safety_policy_json),
        cooldown_secs: record.cooldown_secs,
        max_concurrency: record.max_concurrency,
        risk_level: record.risk_level,
        visibility: record.visibility,
        bindings: bindings.into_iter().map(binding_from_record).collect(),
        created_at: Some(record.created_at),
        updated_at: Some(record.updated_at),
    }
}

fn record_from_payload(
    payload: &SystemAgentProfilePayload,
) -> Result<SystemAgentProfileRecord, String> {
    Ok(SystemAgentProfileRecord {
        id: payload.id.clone(),
        name: payload.name.clone(),
        description: payload.description.clone(),
        mode: payload.mode.clone(),
        capability: payload.capability.clone(),
        enabled: payload.enabled,
        trigger_mode: payload.trigger_mode.clone(),
        base_prompt_id: payload.base_prompt_id.clone(),
        prompt_patch: payload.prompt_patch.clone(),
        input_schema_json: to_json_object_string(payload.input_schema.as_ref())?,
        output_schema_json: to_json_object_string(payload.output_schema.as_ref())?,
        required_tools_json: to_json_array_string(&payload.required_tools)?,
        optional_tools_json: to_json_array_string(&payload.optional_tools)?,
        forbidden_tools_json: to_json_array_string(&payload.forbidden_tools)?,
        trigger_events_json: to_json_array_string(&payload.trigger_events)?,
        budget_json: to_json_object_string(payload.budget.as_ref())?,
        safety_policy_json: to_json_object_string(payload.safety_policy.as_ref())?,
        cooldown_secs: payload.cooldown_secs,
        max_concurrency: payload.max_concurrency,
        risk_level: payload.risk_level.clone(),
        visibility: payload.visibility.clone(),
        created_at: payload.created_at.unwrap_or_else(Utc::now),
        updated_at: payload.updated_at.unwrap_or_else(Utc::now),
    })
}

fn bindings_from_payload(
    profile_id: &str,
    bindings: &[SystemAgentBindingPayload],
) -> Result<Vec<SystemAgentBindingRecord>, String> {
    bindings
        .iter()
        .map(|binding| {
            Ok(SystemAgentBindingRecord {
                id: binding
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("sab-{}", Uuid::new_v4())),
                profile_id: binding
                    .profile_id
                    .clone()
                    .unwrap_or_else(|| profile_id.to_string()),
                event_name: binding.event_name.clone(),
                filter_json: to_json_object_string(binding.filter.as_ref())?,
                priority: binding.priority,
                enabled: binding.enabled,
                created_at: binding.created_at.unwrap_or_else(Utc::now),
                updated_at: binding.updated_at.unwrap_or_else(Utc::now),
            })
        })
        .collect()
}

#[tauri::command]
pub async fn list_system_agent_profiles(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<CommandResponse<Vec<SystemAgentProfileSummary>>, String> {
    let profiles = db_service
        .list_system_agent_profiles()
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(
        profiles.into_iter().map(summary_from_record).collect(),
    ))
}

#[tauri::command]
pub async fn get_system_agent_profile(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<CommandResponse<Option<SystemAgentProfilePayload>>, String> {
    let profile = db_service
        .get_system_agent_profile(&id)
        .await
        .map_err(|e| e.to_string())?;

    let detail = if let Some(profile) = profile {
        let bindings = db_service
            .list_system_agent_bindings(Some(&id))
            .await
            .map_err(|e| e.to_string())?;
        Some(detail_from_record(profile, bindings))
    } else {
        None
    };

    Ok(CommandResponse::ok(detail))
}

#[tauri::command]
pub async fn save_system_agent_profile(
    db_service: State<'_, Arc<DatabaseService>>,
    profile: SystemAgentProfilePayload,
) -> Result<CommandResponse<SystemAgentProfilePayload>, String> {
    let profile_record = record_from_payload(&profile)?;
    validate_profile_tool_policy(&profile_record).map_err(|e| e.to_string())?;
    let binding_records = bindings_from_payload(&profile.id, &profile.bindings)?;

    db_service
        .save_system_agent_profile(&profile_record, &binding_records)
        .await
        .map_err(|e| e.to_string())?;

    let saved_profile = db_service
        .get_system_agent_profile(&profile.id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "保存后未找到 system agent profile".to_string())?;
    let saved_bindings = db_service
        .list_system_agent_bindings(Some(&profile.id))
        .await
        .map_err(|e| e.to_string())?;

    Ok(CommandResponse::ok(detail_from_record(
        saved_profile,
        saved_bindings,
    )))
}

#[tauri::command]
pub async fn delete_system_agent_profile(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<CommandResponse<()>, String> {
    db_service
        .delete_system_agent_profile(&id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn list_system_agent_runs(
    db_service: State<'_, Arc<DatabaseService>>,
    profile_id: Option<String>,
    limit: Option<u32>,
) -> Result<CommandResponse<Vec<SystemAgentRunPayload>>, String> {
    let runs = db_service
        .list_system_agent_runs(profile_id.as_deref(), limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(
        runs.into_iter().map(run_from_record).collect(),
    ))
}

#[tauri::command]
pub async fn list_system_agent_profile_versions(
    db_service: State<'_, Arc<DatabaseService>>,
    profile_id: String,
    limit: Option<u32>,
) -> Result<CommandResponse<Vec<SystemAgentProfileVersionPayload>>, String> {
    let versions = db_service
        .list_system_agent_profile_versions(&profile_id, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(
        versions.into_iter().map(version_from_record).collect(),
    ))
}

#[tauri::command]
pub async fn trigger_system_agent_profile(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    profile_id: String,
    input_summary: Option<Value>,
) -> Result<CommandResponse<SystemAgentRunPayload>, String> {
    let run = runtime
        .trigger_profile_now(
            &profile_id,
            input_summary.unwrap_or_else(|| json!({})),
            Some("manual".to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(run_from_record(run)))
}

#[tauri::command]
pub async fn dispatch_system_agent_event(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    event_name: String,
    payload: Value,
    source: Option<String>,
) -> Result<CommandResponse<SystemAgentDispatchResult>, String> {
    let result = runtime
        .dispatch_event(
            &event_name,
            payload,
            source.as_deref().unwrap_or("manual_dispatch"),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn seed_system_agent_profiles(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<CommandResponse<usize>, String> {
    ensure_default_system_agent_profiles(db_service.inner().clone())
        .await
        .map_err(|e| e.to_string())?;
    let count = db_service
        .list_system_agent_profiles()
        .await
        .map_err(|e| e.to_string())?
        .len();
    Ok(CommandResponse::ok(count))
}

#[tauri::command]
pub async fn fix_plugin_with_system_agent(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    request: PluginFixAgentRequest,
) -> Result<CommandResponse<PluginFixAgentResult>, String> {
    let result = run_plugin_fix_agent(runtime.inner(), ai_manager.inner(), request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn verify_finding_with_system_agent(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    db_service: State<'_, Arc<DatabaseService>>,
    request: TrafficActiveVerifierRequest,
) -> Result<CommandResponse<TrafficActiveVerifierResult>, String> {
    let result = run_traffic_active_verifier(runtime.inner(), db_service.inner(), request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn submit_system_agent_finding_feedback(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SystemAgentFindingFeedbackRequest,
) -> Result<CommandResponse<String>, String> {
    let finding = db_service
        .get_traffic_vulnerability_by_id(&request.finding_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "未找到对应的风险记录".to_string())?;

    if !finding.plugin_id.starts_with("agent:") {
        return Ok(CommandResponse::err("仅支持对 System Agent 发现结果提交反馈"));
    }

    let next_status = match request.feedback_type.as_str() {
        "confirm" => "reviewed",
        "false_positive" => "false_positive",
        "reopen" => "open",
        other => {
            return Ok(CommandResponse::err(format!(
                "不支持的反馈类型: {}",
                other
            )))
        }
    };

    db_service
        .update_traffic_vulnerability_status(&request.finding_id, next_status)
        .await
        .map_err(|e| e.to_string())?;

    let evidence = TrafficEvidenceRecord {
        id: format!("tef-{}", Uuid::new_v4()),
        vuln_id: request.finding_id.clone(),
        url: String::new(),
        method: "SYSTEM".to_string(),
        location: "system_agent_feedback".to_string(),
        evidence_snippet: request.notes.clone().unwrap_or_else(|| match request.feedback_type.as_str() {
            "confirm" => "用户确认该 System Agent 发现值得跟进。".to_string(),
            "false_positive" => "用户将该 System Agent 发现标记为误报。".to_string(),
            _ => "用户重新打开该 System Agent 发现。".to_string(),
        }),
        request_headers: None,
        request_body: Some(
            json!({
                "feedbackType": request.feedback_type,
                "notes": request.notes,
            })
            .to_string(),
        ),
        response_status: None,
        response_headers: None,
        response_body: None,
        timestamp: Utc::now(),
    };
    db_service
        .insert_traffic_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CommandResponse::ok(format!(
        "反馈已记录，状态更新为 {}",
        next_status
    )))
}
