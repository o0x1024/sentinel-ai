//! Sentinel Tools Library
//!
//! Security scanning tools using rig-core Tool trait.
//!
//! # Modules
//! - `buildin_tools`: Built-in tools (http_request, shell, web_search, todos)
//! - `dynamic_tool`: Dynamic tool registration and Rig Tool trait adaptation
//! - `tool_server`: Tool server for managing all tools
//! - `mcp_adapter`: MCP tool adapter
//! - `plugin_adapter`: Plugin tool adapter
//! - `workflow_adapter`: Workflow tool adapter
//! - `docker_sandbox`: Docker sandbox for secure shell execution
//! - `terminal`: Interactive terminal with WebSocket support

pub mod batch_progress_manager;
pub mod buildin_tools;
pub mod docker_sandbox;
pub mod dynamic_tool;
pub mod error_classifier;
pub mod error_config_loader;
pub mod exploitdb;
pub mod mcp_adapter;
pub mod mcp_transport;
pub mod output_storage;
#[cfg(feature = "plugins")]
pub mod plugin_adapter;
pub mod terminal;
mod terminal_output;
mod tool_search_runtime;
pub mod tool_server;
mod tool_server_knowledge;
pub mod tool_timeout;
pub mod workflow_adapter;

pub use batch_progress_manager::*;
pub use buildin_tools::*;
pub use docker_sandbox::*;
pub use dynamic_tool::*;
pub use error_classifier::*;
pub use error_config_loader::*;
pub use exploitdb::*;
pub use mcp_adapter::*;
pub use mcp_transport::*;
pub use output_storage::*;
#[cfg(feature = "plugins")]
pub use plugin_adapter::*;
pub use terminal::*;
pub use tool_server::*;
pub use tool_timeout::*;
pub use workflow_adapter::*;

// ToolExecutionConfig removed - now using output_storage::get_storage_threshold() instead
