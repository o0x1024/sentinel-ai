use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::State;

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::services::{
    delete_traffic_oast_token, generate_traffic_oast_token, lookup_traffic_oast_token,
    test_traffic_oast_config, OastGenerateResponse, TrafficOastConfig, TrafficOastEvent,
    TrafficOastEventKey, TrafficOastRecord, TrafficOastTestResult,
};

pub const TRAFFIC_OAST_CONFIG_KEY: &str = "traffic_oast_config";
const TRAFFIC_OAST_RECORDS_KEY: &str = "traffic_oast_records";
const TRAFFIC_OAST_HIDDEN_EVENTS_KEY: &str = "traffic_oast_hidden_events";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTrafficOastConfigPayload {
    pub config: TrafficOastConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTrafficOastConfigPayload {
    pub config: TrafficOastConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTrafficOastTokenPayload {
    pub label: Option<String>,
    pub source_tool: Option<String>,
    pub source_request_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HideTrafficOastEventsPayload {
    pub token: String,
    pub event_keys: Vec<TrafficOastEventKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTrafficOastRecordResult {
    pub token: String,
    pub remote_deleted_all: bool,
    pub local_removed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HideTrafficOastEventsResult {
    pub token: String,
    pub hidden_count: u64,
    pub visible_event_count: usize,
    pub record: TrafficOastRecord,
}

fn normalize_record_label(payload: &CreateTrafficOastTokenPayload) -> String {
    payload
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("OAST token")
        .to_string()
}

fn normalize_source_tool(payload: &CreateTrafficOastTokenPayload) -> String {
    payload
        .source_tool
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("manual")
        .to_string()
}

fn apply_lookup_to_record(
    record: &mut TrafficOastRecord,
    lookup: crate::services::traffic_oast::OastLookupResponse,
) {
    if let Some(fqdn) = lookup.fqdn {
        record.fqdn = fqdn;
    }
    if let Some(created_at) = lookup.created_at {
        record.created_at = created_at;
    }

    record.hit_count = lookup.hit_count.unwrap_or(0);
    record.last_hit_at = lookup.last_hit_at;
    record.events = lookup.events.unwrap_or_default();
}

fn build_event_key_string(key: &TrafficOastEventKey) -> String {
    [
        key.time.as_str(),
        key.method.as_str(),
        key.url.as_str(),
        key.ip.as_str(),
    ]
    .join("\n")
}

fn build_event_key_from_event(event: &TrafficOastEvent) -> TrafficOastEventKey {
    TrafficOastEventKey {
        time: event.time.clone(),
        method: event.method.clone(),
        url: event.url.clone(),
        ip: event.ip.clone(),
    }
}

fn apply_hidden_event_keys(record: &mut TrafficOastRecord, hidden_event_keys: &[TrafficOastEventKey]) {
    if hidden_event_keys.is_empty() {
        record.hit_count = record.events.len() as u64;
        record.last_hit_at = record.events.last().map(|event| event.time.clone());
        return;
    }

    let hidden = hidden_event_keys
        .iter()
        .map(build_event_key_string)
        .collect::<HashSet<_>>();
    record.events.retain(|event| !hidden.contains(&build_event_key_string(&build_event_key_from_event(event))));
    record.hit_count = record.events.len() as u64;
    record.last_hit_at = record.events.last().map(|event| event.time.clone());
}

async fn load_hidden_oast_events_from_state(
    state: &TrafficAnalysisState,
) -> Result<HashMap<String, Vec<TrafficOastEventKey>>, String> {
    let db = state.get_db_service();
    match db.load_proxy_config(TRAFFIC_OAST_HIDDEN_EVENTS_KEY).await {
        Ok(Some(raw)) => serde_json::from_str::<HashMap<String, Vec<TrafficOastEventKey>>>(&raw)
            .map_err(|error| format!("Failed to parse hidden traffic OAST events: {error}")),
        Ok(None) => Ok(HashMap::new()),
        Err(error) => Err(format!("Failed to load hidden traffic OAST events: {error}")),
    }
}

async fn save_hidden_oast_events_to_state(
    state: &TrafficAnalysisState,
    hidden_events: &HashMap<String, Vec<TrafficOastEventKey>>,
) -> Result<(), String> {
    let db = state.get_db_service();
    let raw = serde_json::to_string(hidden_events)
        .map_err(|error| format!("Failed to serialize hidden traffic OAST events: {error}"))?;
    db.save_proxy_config(TRAFFIC_OAST_HIDDEN_EVENTS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save hidden traffic OAST events: {error}"))
}

fn apply_hidden_events_to_records(
    records: &mut [TrafficOastRecord],
    hidden_events: &HashMap<String, Vec<TrafficOastEventKey>>,
) {
    for record in records {
        let hidden = hidden_events
            .get(&record.token)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        apply_hidden_event_keys(record, hidden);
    }
}

fn merge_hidden_event_keys(
    existing: &[TrafficOastEventKey],
    added: &[TrafficOastEventKey],
) -> Vec<TrafficOastEventKey> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for event_key in existing.iter().chain(added.iter()) {
        let key = build_event_key_string(event_key);
        if seen.insert(key) {
            merged.push(event_key.clone());
        }
    }

    merged
}

async fn load_traffic_oast_config_from_state(
    state: &TrafficAnalysisState,
) -> Result<TrafficOastConfig, String> {
    let db = state.get_db_service();
    match db.load_proxy_config(TRAFFIC_OAST_CONFIG_KEY).await {
        Ok(Some(raw)) => serde_json::from_str::<TrafficOastConfig>(&raw)
            .map(|config| config.sanitized())
            .map_err(|error| format!("Failed to parse traffic OAST config: {error}")),
        Ok(None) => Ok(TrafficOastConfig::default()),
        Err(error) => Err(format!("Failed to load traffic OAST config: {error}")),
    }
}

async fn load_traffic_oast_records_from_state(
    state: &TrafficAnalysisState,
) -> Result<Vec<TrafficOastRecord>, String> {
    let db = state.get_db_service();
    match db.load_proxy_config(TRAFFIC_OAST_RECORDS_KEY).await {
        Ok(Some(raw)) => serde_json::from_str::<Vec<TrafficOastRecord>>(&raw)
            .map_err(|error| format!("Failed to parse traffic OAST records: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!("Failed to load traffic OAST records: {error}")),
    }
}

async fn save_traffic_oast_records_to_state(
    state: &TrafficAnalysisState,
    records: &[TrafficOastRecord],
) -> Result<(), String> {
    let db = state.get_db_service();
    let raw = serde_json::to_string(records)
        .map_err(|error| format!("Failed to serialize traffic OAST records: {error}"))?;
    db.save_proxy_config(TRAFFIC_OAST_RECORDS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save traffic OAST records: {error}"))
}

fn build_record_from_generate_response(
    response: OastGenerateResponse,
    payload: &CreateTrafficOastTokenPayload,
) -> Result<TrafficOastRecord, String> {
    let token = response
        .token
        .ok_or_else(|| "OAST server did not return token".to_string())?;
    let fqdn = response
        .fqdn
        .ok_or_else(|| "OAST server did not return fqdn".to_string())?;
    let example_urls = response
        .example_urls
        .ok_or_else(|| "OAST server did not return example URLs".to_string())?;
    let created_at = Utc::now().to_rfc3339();

    Ok(TrafficOastRecord {
        token,
        fqdn,
        http_url: example_urls.http,
        https_url: example_urls.https,
        created_at,
        label: normalize_record_label(payload),
        source_tool: normalize_source_tool(payload),
        source_request_id: payload.source_request_id,
        hit_count: 0,
        last_hit_at: None,
        last_sync_at: None,
        events: Vec::new(),
    })
}

#[tauri::command]
pub async fn get_traffic_oast_config(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<TrafficOastConfig>, String> {
    let config = load_traffic_oast_config_from_state(&state).await?;
    Ok(CommandResponse::ok(config))
}

#[tauri::command]
pub async fn set_traffic_oast_config(
    state: State<'_, TrafficAnalysisState>,
    payload: SetTrafficOastConfigPayload,
) -> Result<CommandResponse<TrafficOastConfig>, String> {
    let db = state.get_db_service();
    let config = payload.config.sanitized();
    let raw = serde_json::to_string(&config)
        .map_err(|error| format!("Failed to serialize traffic OAST config: {error}"))?;

    db.save_proxy_config(TRAFFIC_OAST_CONFIG_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save traffic OAST config: {error}"))?;

    Ok(CommandResponse::ok(config))
}

#[tauri::command]
pub async fn test_traffic_oast_config_command(
    payload: TestTrafficOastConfigPayload,
) -> Result<CommandResponse<TrafficOastTestResult>, String> {
    let result = test_traffic_oast_config(&payload.config).await?;
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn create_traffic_oast_token(
    state: State<'_, TrafficAnalysisState>,
    payload: CreateTrafficOastTokenPayload,
) -> Result<CommandResponse<TrafficOastRecord>, String> {
    let config = load_traffic_oast_config_from_state(&state).await?;
    let generated = generate_traffic_oast_token(&config).await?;
    let record = build_record_from_generate_response(generated, &payload)?;
    let mut records = load_traffic_oast_records_from_state(&state).await?;

    records.retain(|item| item.token != record.token);
    records.insert(0, record.clone());
    save_traffic_oast_records_to_state(&state, &records).await?;

    Ok(CommandResponse::ok(record))
}

#[tauri::command]
pub async fn list_traffic_oast_records(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<TrafficOastRecord>>, String> {
    let mut records = load_traffic_oast_records_from_state(&state).await?;
    let hidden_events = load_hidden_oast_events_from_state(&state).await?;
    apply_hidden_events_to_records(&mut records, &hidden_events);
    Ok(CommandResponse::ok(records))
}

#[tauri::command]
pub async fn sync_traffic_oast_records(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<TrafficOastRecord>>, String> {
    let config = load_traffic_oast_config_from_state(&state).await?;
    let mut records = load_traffic_oast_records_from_state(&state).await?;
    let hidden_events = load_hidden_oast_events_from_state(&state).await?;
    let sync_time = Utc::now().to_rfc3339();

    for record in &mut records {
        let lookup = lookup_traffic_oast_token(&config, &record.token).await?;
        apply_lookup_to_record(record, lookup);
        let hidden = hidden_events
            .get(&record.token)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        apply_hidden_event_keys(record, hidden);
        record.last_sync_at = Some(sync_time.clone());
    }

    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    save_traffic_oast_records_to_state(&state, &records).await?;

    Ok(CommandResponse::ok(records))
}

#[tauri::command]
pub async fn delete_traffic_oast_record(
    state: State<'_, TrafficAnalysisState>,
    token: String,
) -> Result<CommandResponse<DeleteTrafficOastRecordResult>, String> {
    let config = load_traffic_oast_config_from_state(&state).await?;
    let delete_result = delete_traffic_oast_token(&config, token.trim()).await?;

    let mut records = load_traffic_oast_records_from_state(&state).await?;
    let before_len = records.len();
    records.retain(|record| record.token != token.trim());
    let removed = records.len() != before_len;
    save_traffic_oast_records_to_state(&state, &records).await?;

    let mut hidden_events = load_hidden_oast_events_from_state(&state).await?;
    hidden_events.remove(token.trim());
    save_hidden_oast_events_to_state(&state, &hidden_events).await?;

    Ok(CommandResponse::ok(DeleteTrafficOastRecordResult {
        token: token.trim().to_string(),
        remote_deleted_all: delete_result.deleted_all.unwrap_or(false),
        local_removed: removed,
    }))
}

#[tauri::command]
pub async fn hide_traffic_oast_events_command(
    state: State<'_, TrafficAnalysisState>,
    payload: HideTrafficOastEventsPayload,
) -> Result<CommandResponse<HideTrafficOastEventsResult>, String> {
    let token = payload.token.trim();
    if token.is_empty() {
        return Err("OAST token is required".to_string());
    }
    if payload.event_keys.is_empty() {
        return Err("At least one OAST event must be selected".to_string());
    }

    let mut hidden_events = load_hidden_oast_events_from_state(&state).await?;
    let existing = hidden_events.remove(token).unwrap_or_default();
    let next_hidden = merge_hidden_event_keys(&existing, &payload.event_keys);
    hidden_events.insert(token.to_string(), next_hidden.clone());
    save_hidden_oast_events_to_state(&state, &hidden_events).await?;

    let mut records = load_traffic_oast_records_from_state(&state).await?;
    let record = records
        .iter_mut()
        .find(|record| record.token == token)
        .ok_or_else(|| "OAST record not found".to_string())?;

    apply_hidden_event_keys(record, &next_hidden);
    let updated = record.clone();
    save_traffic_oast_records_to_state(&state, &records).await?;

    Ok(CommandResponse::ok(HideTrafficOastEventsResult {
        token: token.to_string(),
        hidden_count: payload.event_keys.len() as u64,
        visible_event_count: updated.events.len(),
        record: updated,
    }))
}
