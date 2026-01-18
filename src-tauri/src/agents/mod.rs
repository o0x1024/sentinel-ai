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

pub use agent_builder::*;
pub use context_engineering::*;
pub use executor::{execute_agent, AgentExecuteParams};
pub use tool_router::{ToolConfig, ToolRouter, ToolSelectionStrategy, ToolSelectionPlan, SelectedAbilityGroup};
pub use types::DocumentAttachmentInfo;