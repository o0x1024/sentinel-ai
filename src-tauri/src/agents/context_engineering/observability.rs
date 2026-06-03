//! Context observability helpers.

use crate::memory::MemoryRetrievalTrace;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSnapshot {
    pub execution_id: String,
    pub generation: Option<u64>,
    pub system_tokens: usize,
    pub run_state_tokens: usize,
    pub window_tokens: usize,
    pub retrieval_tokens: usize,
    pub tool_digest_tokens: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub trim_trace: Vec<String>,
    pub retrieval_ids: Vec<String>,
    pub memory_retrieval: Option<MemoryRetrievalTrace>,
    pub sentinel_mode: bool,
    pub sentinel_intent_id: Option<String>,
    pub sentinel_intent_confidence: Option<f32>,
    pub sentinel_intent_relation: Option<String>,
    pub sentinel_intent_transition: Option<String>,
    pub sentinel_parent_intent_id: Option<String>,
    pub sentinel_clarification_needed: bool,
    pub sentinel_compression_aggressiveness: Option<String>,
    pub sentinel_clarification_status: Option<String>,
}

pub fn record_context_snapshot(app_handle: &AppHandle, snapshot: &ContextSnapshot) {
    tracing::info!(
        "Context snapshot execution_id={} total={} max={} sections(system={},state={},window={},retrieval={},tool={}) trim={:?} retrieval_trace={:?} sentinel_mode={} intent={:?} clarification_needed={} aggressiveness={:?}",
        snapshot.execution_id,
        snapshot.total_tokens,
        snapshot.max_tokens,
        snapshot.system_tokens,
        snapshot.run_state_tokens,
        snapshot.window_tokens,
        snapshot.retrieval_tokens,
        snapshot.tool_digest_tokens,
        snapshot.trim_trace,
        snapshot.memory_retrieval,
        snapshot.sentinel_mode,
        snapshot.sentinel_intent_id,
        snapshot.sentinel_clarification_needed,
        snapshot.sentinel_compression_aggressiveness
    );

    let _ = app_handle.emit(
        "agent:context_snapshot",
        &json!({
            "execution_id": snapshot.execution_id,
            "generation": snapshot.generation,
            "system_tokens": snapshot.system_tokens,
            "run_state_tokens": snapshot.run_state_tokens,
            "window_tokens": snapshot.window_tokens,
            "retrieval_tokens": snapshot.retrieval_tokens,
            "tool_digest_tokens": snapshot.tool_digest_tokens,
            "total_tokens": snapshot.total_tokens,
            "max_tokens": snapshot.max_tokens,
            "trim_trace": snapshot.trim_trace,
            "retrieval_ids": snapshot.retrieval_ids,
            "memory_retrieval": snapshot.memory_retrieval,
            "sentinel_mode": snapshot.sentinel_mode,
            "sentinel_intent_id": snapshot.sentinel_intent_id,
            "sentinel_intent_confidence": snapshot.sentinel_intent_confidence,
            "sentinel_intent_relation": snapshot.sentinel_intent_relation,
            "sentinel_intent_transition": snapshot.sentinel_intent_transition,
            "sentinel_parent_intent_id": snapshot.sentinel_parent_intent_id,
            "sentinel_clarification_needed": snapshot.sentinel_clarification_needed,
            "sentinel_compression_aggressiveness": snapshot.sentinel_compression_aggressiveness,
            "sentinel_clarification_status": snapshot.sentinel_clarification_status,
        }),
    );
}
