//! Context Engineering module.

pub mod builder;
pub mod checkpoint;
pub mod policy;
pub mod tool_digest;

pub use builder::{build_context, ContextBuildInput, ContextBuildResult};
pub use checkpoint::{append_tool_digest, load_run_state, save_run_state, ContextRunState};
pub use policy::{ContextPolicy, ContextScope};
pub use tool_digest::{build_tool_digest, condense_text, ToolDigest};

