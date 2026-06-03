//! Run state checkpoint storage.

use anyhow::Result;
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as TokioMutex;

use crate::agents::context_engineering::artifact_readback::{
    apply_tool_digest_to_tracked_artifacts, TrackedArtifact,
};
use crate::agents::context_engineering::policy::ContextPolicy;
use crate::agents::context_engineering::sentinel::{
    SentinelClarificationState, SentinelCompressionState, SentinelIntentState,
    SentinelIntentStatus, SentinelIntentTransition, SentinelPinnedContext,
};
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
    pub open_tasks: Vec<String>,
    #[serde(default)]
    pub user_preferences: Vec<String>,
    #[serde(default)]
    pub current_plan: Option<String>,
    pub last_tool_digests: Vec<ToolDigest>,
    #[serde(default)]
    pub tracked_artifacts: Vec<TrackedArtifact>,
    #[serde(default)]
    pub memory_items: Vec<ContextMemoryItem>,
    #[serde(default)]
    pub sentinel_active_intent: Option<SentinelIntentState>,
    #[serde(default)]
    pub sentinel_intent_registry: Vec<SentinelIntentState>,
    #[serde(default)]
    pub sentinel_pinned_context: SentinelPinnedContext,
    #[serde(default)]
    pub sentinel_compression_state: SentinelCompressionState,
    #[serde(default)]
    pub sentinel_last_clarification: Option<SentinelClarificationState>,
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
    apply_tool_digest_to_tracked_artifacts(&mut state.tracked_artifacts, &digest);
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
    for digest in &digests {
        apply_tool_digest_to_tracked_artifacts(&mut state.tracked_artifacts, digest);
    }
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

pub async fn apply_sentinel_execution_outcome(
    app_handle: &AppHandle,
    execution_id: &str,
    success: bool,
    response_excerpt: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    let lock = get_state_lock(execution_id);
    let _guard = lock.lock().await;

    let Some(mut state) = load_run_state(app_handle, execution_id).await? else {
        return Ok(());
    };
    let Some(active_intent) = state.sentinel_active_intent.clone() else {
        return Ok(());
    };

    if !should_resolve_active_intent(&state, &active_intent, success, response_excerpt, error) {
        return Ok(());
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    if let Some(intent) = state
        .sentinel_intent_registry
        .iter_mut()
        .find(|intent| intent.intent_id == active_intent.intent_id)
    {
        intent.status = SentinelIntentStatus::Resolved;
        intent.last_transition = SentinelIntentTransition::Resolved;
        intent.updated_at_ms = now_ms;
    }

    if !state
        .sentinel_compression_state
        .resolved_intent_ids
        .iter()
        .any(|item| item == &active_intent.intent_id)
    {
        state
            .sentinel_compression_state
            .resolved_intent_ids
            .push(active_intent.intent_id.clone());
    }
    state
        .sentinel_compression_state
        .active_intent_ids
        .retain(|item| item != &active_intent.intent_id);
    state
        .sentinel_compression_state
        .open_loops
        .retain(|item| !item.contains(&active_intent.intent_id));
    state.sentinel_active_intent = None;
    state.sentinel_last_clarification = None;
    state.run_state_version += 1;
    state.last_updated_at_ms = now_ms;
    save_run_state(app_handle, execution_id, &state).await?;
    Ok(())
}

pub async fn suspend_sentinel_active_intent(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Result<()> {
    let lock = get_state_lock(execution_id);
    let _guard = lock.lock().await;

    let Some(mut state) = load_run_state(app_handle, execution_id).await? else {
        return Ok(());
    };
    let Some(active_intent) = state.sentinel_active_intent.clone() else {
        return Ok(());
    };
    let now_ms = chrono::Utc::now().timestamp_millis();

    if let Some(intent) = state
        .sentinel_intent_registry
        .iter_mut()
        .find(|intent| intent.intent_id == active_intent.intent_id)
    {
        intent.status = SentinelIntentStatus::Suspended;
        intent.last_transition = SentinelIntentTransition::Suspended;
        intent.updated_at_ms = now_ms;
    }
    state.sentinel_active_intent = None;
    state.sentinel_last_clarification = None;
    state
        .sentinel_compression_state
        .active_intent_ids
        .retain(|item| item != &active_intent.intent_id);
    state.run_state_version += 1;
    state.last_updated_at_ms = now_ms;
    save_run_state(app_handle, execution_id, &state).await?;
    Ok(())
}

fn should_resolve_active_intent(
    state: &ContextRunState,
    active_intent: &SentinelIntentState,
    success: bool,
    response_excerpt: Option<&str>,
    error: Option<&str>,
) -> bool {
    if !success || error.is_some() {
        return false;
    }

    if matches!(
        active_intent.task_type.as_str(),
        "explanation" | "comparison" | "review"
    ) {
        return true;
    }

    if state.open_tasks.is_empty()
        && matches!(
            active_intent.task_type.as_str(),
            "general" | "continuation" | "fix" | "implementation"
        )
    {
        if let Some(text) = response_excerpt {
            let lower = text.to_lowercase();
            let completion_hint = [
                "done",
                "completed",
                "implemented",
                "fixed",
                "resolved",
                "已完成",
                "完成了",
                "已经完成",
                "已解决",
                "已实现",
                "修复完成",
            ]
            .iter()
            .any(|needle| lower.contains(needle));
            let continuation_hint = [
                "next step",
                "remaining",
                "still need",
                "task",
                "follow-up",
                "后续",
                "下一步",
                "未完成",
                "继续",
            ]
            .iter()
            .any(|needle| lower.contains(needle));
            if completion_hint && !continuation_hint {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_helper_marks_explanation_intents_complete() {
        let mut state = ContextRunState::default();
        state.sentinel_active_intent = Some(SentinelIntentState {
            intent_id: "intent-1".to_string(),
            task_type: "explanation".to_string(),
            ..SentinelIntentState::default()
        });
        let active = state.sentinel_active_intent.clone().unwrap();
        assert!(should_resolve_active_intent(
            &state,
            &active,
            true,
            Some("I explained the issue."),
            None
        ));
    }

    #[test]
    fn resolve_helper_keeps_multi_step_work_when_follow_up_remains() {
        let mut state = ContextRunState::default();
        state.sentinel_active_intent = Some(SentinelIntentState {
            intent_id: "intent-1".to_string(),
            task_type: "implementation".to_string(),
            ..SentinelIntentState::default()
        });
        let active = state.sentinel_active_intent.clone().unwrap();
        assert!(!should_resolve_active_intent(
            &state,
            &active,
            true,
            Some("Implemented the first part. Next step is wiring the API."),
            None
        ));
    }

    #[test]
    fn suspend_helper_clears_active_intent_fields() {
        let mut state = ContextRunState::default();
        state.sentinel_active_intent = Some(SentinelIntentState {
            intent_id: "intent-1".to_string(),
            ..SentinelIntentState::default()
        });
        state.sentinel_compression_state.active_intent_ids = vec!["intent-1".to_string()];

        let active = state.sentinel_active_intent.clone().unwrap();
        state.sentinel_active_intent = None;
        state.sentinel_last_clarification = None;
        state
            .sentinel_compression_state
            .active_intent_ids
            .retain(|item| item != &active.intent_id);

        assert!(state.sentinel_active_intent.is_none());
        assert!(state
            .sentinel_compression_state
            .active_intent_ids
            .is_empty());
    }
}
