//! Run state checkpoint storage.

use anyhow::Result;
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as TokioMutex;

use crate::agents::context_engineering::policy::ContextPolicy;
use crate::agents::context_engineering::tool_digest::ToolDigest;

/// Per-execution-id lock to prevent concurrent read-modify-write races on RunState.
fn get_state_lock(execution_id: &str) -> Arc<TokioMutex<()>> {
    static LOCKS: std::sync::LazyLock<StdMutex<HashMap<String, Arc<TokioMutex<()>>>>> =
        std::sync::LazyLock::new(|| StdMutex::new(HashMap::new()));

    let mut map = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    map.entry(execution_id.to_string())
        .or_insert_with(|| Arc::new(TokioMutex::new(())))
        .clone()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMemoryItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub importance: u8,
    pub created_at_ms: i64,
    pub last_used_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextRunState {
    pub task: String,
    pub task_brief: String,
    pub selected_tools: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub open_todos: Vec<String>,
    #[serde(default)]
    pub user_preferences: Vec<String>,
    #[serde(default)]
    pub current_plan: Option<String>,
    pub last_tool_digests: Vec<ToolDigest>,
    #[serde(default)]
    pub memory_items: Vec<ContextMemoryItem>,
    #[serde(default)]
    pub run_state_version: i64,
    pub last_updated_at_ms: i64,
}

pub async fn load_run_state(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Result<Option<ContextRunState>> {
    let db = app_handle.state::<Arc<dyn Database>>().inner().clone();

    let state_json_opt = db.get_agent_run_state(execution_id).await?;

    if let Some(state_json) = state_json_opt {
        let state: ContextRunState = serde_json::from_str(&state_json).unwrap_or_default();
        return Ok(Some(state));
    }
    Ok(None)
}

pub async fn save_run_state(
    app_handle: &AppHandle,
    execution_id: &str,
    state: &ContextRunState,
) -> Result<()> {
    let db = app_handle.state::<Arc<dyn Database>>().inner().clone();

    let state_json = serde_json::to_string(state)?;
    db.save_agent_run_state(execution_id, &state_json).await?;

    Ok(())
}

pub async fn load_or_init_run_state(
    app_handle: &AppHandle,
    execution_id: &str,
    init_state: ContextRunState,
) -> Result<ContextRunState> {
    let lock = get_state_lock(execution_id);
    let _guard = lock.lock().await;

    if let Some(existing) = load_run_state(app_handle, execution_id).await? {
        return Ok(existing);
    }
    save_run_state(app_handle, execution_id, &init_state).await?;
    Ok(init_state)
}

pub async fn append_tool_digest(
    app_handle: &AppHandle,
    execution_id: &str,
    digest: ToolDigest,
    policy: &ContextPolicy,
) -> Result<()> {
    let lock = get_state_lock(execution_id);
    let _guard = lock.lock().await;

    let mut state = load_run_state(app_handle, execution_id)
        .await?
        .unwrap_or_default();
    state.last_tool_digests.push(digest);
    if state.last_tool_digests.len() > policy.run_state_max_digests {
        let keep_from = state.last_tool_digests.len() - policy.run_state_max_digests;
        state.last_tool_digests = state.last_tool_digests.split_off(keep_from);
    }
    state.run_state_version += 1;
    state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
    save_run_state(app_handle, execution_id, &state).await?;
    Ok(())
}

pub async fn append_tool_digests(
    app_handle: &AppHandle,
    execution_id: &str,
    digests: Vec<ToolDigest>,
    policy: &ContextPolicy,
) -> Result<()> {
    if digests.is_empty() {
        return Ok(());
    }
    let lock = get_state_lock(execution_id);
    let _guard = lock.lock().await;

    let mut state = load_run_state(app_handle, execution_id)
        .await?
        .unwrap_or_default();
    state.last_tool_digests.extend(digests);
    if state.last_tool_digests.len() > policy.run_state_max_digests {
        let keep_from = state.last_tool_digests.len() - policy.run_state_max_digests;
        state.last_tool_digests = state.last_tool_digests.split_off(keep_from);
    }
    state.run_state_version += 1;
    state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
    save_run_state(app_handle, execution_id, &state).await?;
    Ok(())
}
