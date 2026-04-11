//! Agents Module - Agent 相关操作

pub mod agent_builder;
pub mod context_engineering;
pub mod control_plane;
pub mod executor;
pub mod sliding_window;
pub mod subagent_executor;
pub mod tenth_man;
pub mod tenth_man_executor;
pub mod tool_router;
pub mod types;

#[cfg(test)]
mod tenth_man_tests;

pub use agent_builder::{
    SecurityAgent, SecurityAgentConfig, CTF_SECURITY_PREAMBLE, DEFAULT_SECURITY_PREAMBLE,
};
pub use context_engineering::{
    append_tool_digest, append_tool_digests, build_context, build_tool_digest, condense_text,
    evict_low_value_items, ingest_memory_items, ingest_memory_items_persistent, load_run_state,
    record_context_snapshot, resolve_context_policy, retrieve_memory_items,
    retrieve_memory_items_hybrid, save_run_state, ContextBuildInput, ContextBuildResult,
    ContextEngineMode, ContextPacket, ContextPolicy, ContextRunState, ContextScope, ContextSection,
    ContextSnapshot, MemoryQuery, RetrievedMemoryItem, ToolDigest, ToolDigestEntry,
};
pub use control_plane::{close_agent, list_agents, spawn_agent, wait_agents, AgentHandleSummary};
pub use executor::{execute_agent, AgentExecuteParams};
pub use subagent_executor::{
    clear_parent_context, set_parent_context, ControlPlaneSpawnRequest, SubagentParentContext,
};
pub use tool_router::{
    SelectedSkill, ToolConfig, ToolRouter, ToolSelectionPlan, ToolSelectionStrategy,
};
pub use types::DocumentAttachmentInfo;
