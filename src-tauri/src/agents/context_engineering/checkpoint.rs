//! Run state checkpoint storage.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use sentinel_db::Database;

use crate::agents::context_engineering::policy::ContextPolicy;
use crate::agents::context_engineering::tool_digest::ToolDigest;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextRunState {
    pub task: String,
    pub task_brief: String,
    pub selected_tools: Vec<String>,
    pub last_tool_digests: Vec<ToolDigest>,
    pub last_updated_at_ms: i64,
}

pub async fn load_run_state(app_handle: &AppHandle, execution_id: &str) -> Result<Option<ContextRunState>> {
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
    let mut state = load_run_state(app_handle, execution_id)
        .await?
        .unwrap_or_default();
    state.last_tool_digests.push(digest);
    if state.last_tool_digests.len() > policy.run_state_max_digests {
        let keep_from = state.last_tool_digests.len() - policy.run_state_max_digests;
        state.last_tool_digests = state.last_tool_digests.split_off(keep_from);
    }
    state.last_updated_at_ms = chrono::Utc::now().timestamp_millis();
    save_run_state(app_handle, execution_id, &state).await?;
    Ok(())
}

