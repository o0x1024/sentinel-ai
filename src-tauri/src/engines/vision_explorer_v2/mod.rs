//! Vision Explorer V2 - ReAct Architecture
//!
//! A simplified web exploration engine using the ReAct (Reasoning + Acting) pattern.
//! 
//! ## Architecture
//! 
//! The engine follows a simple loop:
//! 1. **Observe**: Analyze the current page using Vision LLM
//! 2. **Think**: Use LLM to reason about what action to take next
//! 3. **Act**: Execute the chosen action via MCP Playwright
//! 4. **Update**: Record results and update exploration state
//! 
//! ## Modules
//! 
//! - `types`: Core data structures
//! - `graph`: Simple exploration graph for tracking visited pages
//! - `perception`: Page analysis using Vision LLM
//! - `action_executor`: Browser action execution via MCP
//! - `react_engine`: Main ReAct loop implementation
//! - `tool`: Rig tool interface for agent integration

pub mod action_executor;
pub mod graph;
pub mod perception;
pub mod react_engine;
pub mod tool;
pub mod types;

// Re-export key items
pub use graph::{ExplorationGraph, GraphEdge, GraphNode};
pub use react_engine::ReActEngine;
pub use tool::VisionExplorerV2Tool;
pub use types::{
    Action, ActionResult, AIConfig, AuthStatus, Element, ExplorationResult, ExplorationState,
    FormField, FormInfo, Observation, PageContext, PageType, ReActDecision, ScrollDirection,
    Step, VisionExplorerV2Config, VisionMessage,
};
