//! Multi-Agent Vision Explorer Architecture
//!
//! Manager-Worker pattern for parallel website exploration:
//! - Manager: Analyzes homepage navigation, divides exploration scope
//! - Workers: Explore assigned scopes with boundary constraints
//! - GlobalState: Shared state for deduplication and aggregation
//! - ElementFilter: Smart context optimization for LLM prompts

mod coordinator;
mod element_filter;
mod global_state;
mod manager_agent;
mod types;
mod worker_agent;

pub use coordinator::MultiAgentExplorer;
pub use element_filter::{
    format_filtered_for_prompt, ElementFilter, ElementFilterConfig, FilteredElements,
};
pub use global_state::{AuthSnapshot, GlobalExplorerState};
pub use manager_agent::ManagerAgent;
pub use types::*;
pub use worker_agent::WorkerAgent;
