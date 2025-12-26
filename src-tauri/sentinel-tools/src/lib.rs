//! Sentinel Tools Library
//!
//! Security scanning tools using rig-core Tool trait.
//!
//! # Modules
//! - `buildin_tools`: Built-in tools (port_scan, http_request, local_time, shell)
//! - `dynamic_tool`: Dynamic tool registration and Rig Tool trait adaptation
//! - `tool_server`: Tool server for managing all tools
//! - `mcp_adapter`: MCP tool adapter
//! - `plugin_adapter`: Plugin tool adapter
//! - `workflow_adapter`: Workflow tool adapter

pub mod buildin_tools;
pub mod batch_progress_manager;
pub mod error_classifier;
pub mod error_config_loader;
pub mod global_proxy;
pub mod dynamic_tool;
pub mod tool_server;
pub mod mcp_adapter;
pub mod plugin_adapter;
pub mod workflow_adapter;

pub use buildin_tools::*;
pub use batch_progress_manager::*;
pub use error_classifier::*;
pub use error_config_loader::*;
pub use global_proxy::*;
pub use dynamic_tool::*;
pub use tool_server::*;
pub use mcp_adapter::*;
pub use plugin_adapter::*;
pub use workflow_adapter::*;

use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Global tool execution configuration
#[derive(Debug, Clone, Copy)]
pub struct ToolExecutionConfig {
    /// Max output size in chars for any tool execution (to prevent context overflow)
    pub max_output_chars: usize,
}

impl Default for ToolExecutionConfig {
    fn default() -> Self {
        Self {
            max_output_chars: 50_000, // Default 50K chars (approx 10K-15K tokens)
        }
    }
}

static TOOL_EXECUTION_CONFIG: Lazy<RwLock<ToolExecutionConfig>> = Lazy::new(|| RwLock::new(ToolExecutionConfig::default()));

pub fn get_tool_execution_config() -> ToolExecutionConfig {
    *TOOL_EXECUTION_CONFIG.read().unwrap()
}

pub fn set_tool_execution_config(config: ToolExecutionConfig) {
    if let Ok(mut c) = TOOL_EXECUTION_CONFIG.write() {
        *c = config;
    }
}
