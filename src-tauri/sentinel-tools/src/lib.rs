//! Sentinel Tools Library
//!
//! Security scanning tools using rig-core Tool trait.
//!
//! # Modules
//! - `agent_browser`: Browser automation using agent-browser daemon
//! - `buildin_tools`: Built-in tools (port_scan, http_request, local_time, shell, browser)
//! - `dynamic_tool`: Dynamic tool registration and Rig Tool trait adaptation
//! - `tool_server`: Tool server for managing all tools
//! - `mcp_adapter`: MCP tool adapter
//! - `plugin_adapter`: Plugin tool adapter
//! - `workflow_adapter`: Workflow tool adapter
//! - `docker_sandbox`: Docker sandbox for secure shell execution
//! - `terminal`: Interactive terminal with WebSocket support

pub mod agent_browser;
pub mod batch_progress_manager;
pub mod buildin_tools;
pub mod docker_sandbox;
pub mod dynamic_tool;
pub mod error_classifier;
pub mod error_config_loader;
pub mod mcp_adapter;
pub mod output_storage;
pub mod plugin_adapter;
pub mod terminal;
pub mod tool_server;
pub mod workflow_adapter;

pub use agent_browser::*;
pub use batch_progress_manager::*;
pub use buildin_tools::*;
pub use docker_sandbox::*;
pub use dynamic_tool::*;
pub use error_classifier::*;
pub use error_config_loader::*;
pub use mcp_adapter::*;
pub use output_storage::*;
pub use plugin_adapter::*;
pub use terminal::*;
pub use tool_server::*;
pub use workflow_adapter::*;

// ToolExecutionConfig removed - now using output_storage::get_storage_threshold() instead
