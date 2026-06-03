use std::sync::Arc;

use tauri::State;

use crate::models::database::{BotAccount, BotExecutionRun, BotMessage, BotPeer};
use crate::services::database::DatabaseService;

const DEFAULT_PEER_LIMIT: i64 = 200;
const DEFAULT_MESSAGE_LIMIT: i64 = 200;
const DEFAULT_EXECUTION_LIMIT: i64 = 100;
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
