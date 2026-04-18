//! Context Engineering module.

pub mod artifact_readback;
pub mod builder;
pub mod checkpoint;
pub mod engine;
pub mod memory_index;
pub mod observability;
pub mod policy;
pub mod reflection;
pub mod sentinel;
pub mod token_utils;
pub mod tool_digest;
pub mod types;

#[cfg(test)]
mod long_context_regression;
#[cfg(test)]
mod tests;

pub use artifact_readback::{
    apply_tool_digest_artifact_updates, apply_tool_digest_to_tracked_artifacts,
    has_incomplete_host_artifacts, render_artifact_readback_summary, TrackedArtifact,
};
pub use builder::{build_context, ContextBuildInput, ContextBuildResult};
pub use checkpoint::{
    append_tool_digest, append_tool_digests, apply_sentinel_execution_outcome, load_run_state,
    save_run_state, suspend_sentinel_active_intent, ContextRunState,
};
pub use engine::{resolve_context_policy, ContextEngineMode};
pub use memory_index::{
    evict_low_value_items, ingest_memory_items, ingest_memory_items_persistent,
    keyword_score_value, retrieve_memory_items, retrieve_memory_items_hybrid, MemoryQuery,
    RetrievedMemoryItem,
};
pub use observability::{record_context_snapshot, ContextSnapshot};
pub use policy::{ContextMessageLayout, ContextPolicy, ContextScope};
pub use reflection::{record_execution_reflection, ExecutionOutcome};
pub use sentinel::{
    analyze_intent, apply_sentinel_history_selection, build_sentinel_clarification_state,
    focus_compression_state_for_intent, reconcile_sentinel_clarification, render_sentinel_context,
    restore_pinned_context_for_intent, update_intent_registry, update_pinned_context,
    SentinelClarificationState, SentinelCompressionAggressiveness, SentinelCompressionState,
    SentinelIntentRelation, SentinelIntentState, SentinelIntentStatus, SentinelIntentTransition,
    SentinelPinnedContext,
};
pub use token_utils::{
    estimate_message_tokens, estimate_tokens, MESSAGE_OVERHEAD_TOKENS,
    SYSTEM_MESSAGE_OVERHEAD_TOKENS, TOOL_CALLS_OVERHEAD_TOKENS,
};
pub use tool_digest::{build_tool_digest, condense_text, ToolDigest};
pub use types::{trim_history_preserve_tool_pairs, ContextPacket, ContextSection, ToolDigestEntry};
