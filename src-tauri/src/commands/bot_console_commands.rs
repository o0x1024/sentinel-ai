use std::sync::Arc;

use tauri::State;

use crate::models::database::{
    BotAccount, BotExecutionRun, BotMessage, BotPeer, BotSchedule, BotScheduleRun,
};
use crate::services::database::DatabaseService;

const DEFAULT_PEER_LIMIT: i64 = 200;
const DEFAULT_MESSAGE_LIMIT: i64 = 200;
const DEFAULT_EXECUTION_LIMIT: i64 = 100;
const DEFAULT_SCHEDULE_RUN_LIMIT: i64 = 100;
const MAX_LIST_LIMIT: i64 = 500;

fn resolve_limit(requested: Option<u32>, default_value: i64) -> i64 {
    requested
        .map(i64::from)
        .unwrap_or(default_value)
        .clamp(1, MAX_LIST_LIMIT)
}

#[tauri::command]
pub async fn list_bot_accounts(
    transport: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotAccount>, String> {
    db_service
        .list_bot_accounts(transport.as_deref())
        .await
        .map_err(|e| format!("Failed to load bot accounts: {}", e))
}

#[tauri::command]
pub async fn list_bot_peers(
    transport: Option<String>,
    account_id: Option<String>,
    peer_type: Option<String>,
    limit: Option<u32>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotPeer>, String> {
    db_service
        .list_bot_peers(
            transport.as_deref(),
            account_id.as_deref(),
            peer_type.as_deref(),
            resolve_limit(limit, DEFAULT_PEER_LIMIT),
        )
        .await
        .map_err(|e| format!("Failed to load bot peers: {}", e))
}

#[tauri::command]
pub async fn list_bot_messages_for_peer(
    transport: String,
    account_id: String,
    peer_type: String,
    peer_id: String,
    limit: Option<u32>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotMessage>, String> {
    db_service
        .list_bot_messages_for_peer(
            &transport,
            &account_id,
            &peer_type,
            &peer_id,
            resolve_limit(limit, DEFAULT_MESSAGE_LIMIT),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to load bot messages for {}:{}:{}:{}: {}",
                transport, account_id, peer_type, peer_id, e
            )
        })
}

#[tauri::command]
pub async fn list_bot_execution_runs_for_peer(
    transport: String,
    account_id: String,
    peer_type: String,
    peer_id: String,
    limit: Option<u32>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotExecutionRun>, String> {
    db_service
        .list_bot_execution_runs_for_peer(
            &transport,
            &account_id,
            &peer_type,
            &peer_id,
            resolve_limit(limit, DEFAULT_EXECUTION_LIMIT),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to load bot execution runs for {}:{}:{}:{}: {}",
                transport, account_id, peer_type, peer_id, e
            )
        })
}

#[tauri::command]
pub async fn list_bot_schedules(
    transport: String,
    account_id: String,
    peer_type: Option<String>,
    peer_id: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotSchedule>, String> {
    match (peer_type.as_deref(), peer_id.as_deref()) {
        (Some(peer_type), Some(peer_id)) => db_service
            .list_bot_schedules_for_peer(&transport, &account_id, peer_type, peer_id)
            .await
            .map_err(|e| {
                format!(
                    "Failed to load bot schedules for {}:{}:{}:{}: {}",
                    transport, account_id, peer_type, peer_id, e
                )
            }),
        (None, None) => db_service
            .list_bot_schedules_by_transport_account(&transport, &account_id)
            .await
            .map_err(|e| {
                format!(
                    "Failed to load bot schedules for {}:{}: {}",
                    transport, account_id, e
                )
            }),
        _ => Err("peer_type and peer_id must both be provided or both be omitted".to_string()),
    }
}

#[tauri::command]
pub async fn list_bot_schedule_runs(
    schedule_id: String,
    limit: Option<u32>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BotScheduleRun>, String> {
    db_service
        .list_bot_schedule_runs(
            &schedule_id,
            resolve_limit(limit, DEFAULT_SCHEDULE_RUN_LIMIT),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to load bot schedule runs for {}: {}",
                schedule_id, e
            )
        })
}
