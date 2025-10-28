//! Sentinel AI Command Handlers
//! 
//! Tauri命令处理器

pub mod agent_commands;
pub mod ai;
pub mod ai_commands;
pub mod asset;
pub mod config;
pub mod database;
pub mod dictionary;
pub mod mcp;
pub mod performance;
pub mod plan_execute_commands;
pub mod prompt_api;
pub mod prompt_commands;
pub mod rag_commands;
pub mod rewoo_commands;
pub mod role;
pub mod scan;
pub mod scan_commands;
pub mod scan_session_commands;
pub mod subdomain_dictionary;
pub mod test_agent_flow;
pub mod test_mcp;
pub mod test_proxy;
pub mod unified_tools;
pub mod vulnerability;
pub mod window;

// 重新导出所有命令
pub use agent_commands::*;
pub use ai::*;
pub use ai_commands::*;
pub use asset::*;
pub use config::*;
pub use database::*;
pub use dictionary::*;
pub use mcp::*;
pub use performance::*;
pub use plan_execute_commands::*;
pub use prompt_api::*;
pub use prompt_commands::*;
pub use rag_commands::*;
pub use rewoo_commands::*;
pub use role::*;
pub use scan::*;
pub use scan_commands::*;
pub use scan_session_commands::*;
pub use unified_tools::*;
pub use vulnerability::*;
pub use window::*;
