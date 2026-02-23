//! Agents Module - Agent 相关操作

pub mod agent_builder;
pub mod context_engineering;
pub mod executor;
pub mod tool_router;
pub mod tenth_man;
pub mod tenth_man_executor;
pub mod subagent_executor;
pub mod sliding_window;
pub mod types;

#[cfg(test)]
mod tenth_man_tests;

pub use agent_builder::{SecurityAgent, SecurityAgentConfig, CTF_SECURITY_PREAMBLE, DEFAULT_SECURITY_PREAMBLE};
pub use context_engineering::{
    append_tool_digest, append_tool_digests, build_context, build_tool_digest, condense_text,
    ingest_memory_items, ingest_memory_items_persistent,
    load_run_state, record_context_snapshot,
    retrieve_memory_items, retrieve_memory_items_hybrid, evict_low_value_items,
    save_run_state, ContextBuildInput, ContextBuildResult, ContextPacket, ContextPolicy,
    ContextRunState, ContextScope, ContextSection, ContextSnapshot, MemoryQuery,
    RetrievedMemoryItem, ToolDigest, ToolDigestEntry,
};
pub use executor::{execute_agent, AgentExecuteParams};
pub use tool_router::{ToolConfig, ToolRouter, ToolSelectionStrategy, ToolSelectionPlan, SelectedSkill};
pub use types::DocumentAttachmentInfo;
