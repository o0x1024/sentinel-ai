//! Context Engineering module.

pub mod builder;
pub mod checkpoint;
pub mod memory_index;
pub mod observability;
pub mod policy;
pub mod tool_digest;
pub mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod long_context_regression;

pub use builder::{build_context, ContextBuildInput, ContextBuildResult};
pub use checkpoint::{
    append_tool_digest, append_tool_digests, load_run_state, save_run_state, ContextRunState,
};
pub use memory_index::{ingest_memory_items, retrieve_memory_items, MemoryQuery, RetrievedMemoryItem};
pub use observability::{record_context_snapshot, ContextSnapshot};
pub use policy::{ContextPolicy, ContextScope};
pub use tool_digest::{build_tool_digest, condense_text, ToolDigest};
pub use types::{ContextPacket, ContextSection, ToolDigestEntry, trim_history_preserve_tool_pairs};
