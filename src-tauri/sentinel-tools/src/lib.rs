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
//! - `docker_sandbox`: Docker sandbox for secure shell execution
//! - `terminal`: Interactive terminal with WebSocket support

pub mod buildin_tools;
pub mod batch_progress_manager;
pub mod error_classifier;
pub mod error_config_loader;
pub mod dynamic_tool;
pub mod tool_server;
pub mod mcp_adapter;
pub mod plugin_adapter;
pub mod workflow_adapter;
pub mod docker_sandbox;
pub mod terminal;
pub mod output_storage;

pub use buildin_tools::*;
pub use batch_progress_manager::*;
pub use error_classifier::*;
pub use error_config_loader::*;
pub use dynamic_tool::*;
pub use tool_server::*;
pub use mcp_adapter::*;
pub use plugin_adapter::*;
pub use workflow_adapter::*;
pub use docker_sandbox::*;
pub use terminal::*;
pub use output_storage::*;

// ToolExecutionConfig removed - now using output_storage::get_storage_threshold() instead
