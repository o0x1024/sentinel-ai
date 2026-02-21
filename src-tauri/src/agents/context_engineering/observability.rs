//! Context observability helpers.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSnapshot {
    pub execution_id: String,
    pub system_tokens: usize,
    pub run_state_tokens: usize,
    pub window_tokens: usize,
    pub retrieval_tokens: usize,
    pub tool_digest_tokens: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub trim_trace: Vec<String>,
    pub retrieval_ids: Vec<String>,
}

pub fn record_context_snapshot(app_handle: &AppHandle, snapshot: &ContextSnapshot) {
    tracing::info!(
        "Context snapshot execution_id={} total={} max={} sections(system={},state={},window={},retrieval={},tool={}) trim={:?}",
        snapshot.execution_id,
        snapshot.total_tokens,
        snapshot.max_tokens,
        snapshot.system_tokens,
        snapshot.run_state_tokens,
        snapshot.window_tokens,
        snapshot.retrieval_tokens,
        snapshot.tool_digest_tokens,
        snapshot.trim_trace
    );

    let _ = app_handle.emit(
        "agent:context_snapshot",
        &json!({
            "execution_id": snapshot.execution_id,
            "system_tokens": snapshot.system_tokens,
            "run_state_tokens": snapshot.run_state_tokens,
            "window_tokens": snapshot.window_tokens,
            "retrieval_tokens": snapshot.retrieval_tokens,
            "tool_digest_tokens": snapshot.tool_digest_tokens,
            "total_tokens": snapshot.total_tokens,
            "max_tokens": snapshot.max_tokens,
            "trim_trace": snapshot.trim_trace,
            "retrieval_ids": snapshot.retrieval_ids,
        }),
    );
}
