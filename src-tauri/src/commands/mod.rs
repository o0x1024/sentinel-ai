pub mod agent_commands;
pub mod agent_plugin_commands;
pub mod ai;
pub mod ai_commands;
pub mod asset;
pub mod config;
pub mod config_commands;
pub mod database;
pub mod dictionary;
pub mod mcp;
pub mod passive_scan_commands;
pub mod performance;
pub mod plan_execute_commands;
pub mod plugin_review_commands;
pub mod prompt_commands;
pub mod prompt_api;
pub mod proxifier_commands;
pub mod rag_commands;
pub mod react_commands;
pub mod rewoo_commands;
pub mod role;
pub mod scan;
pub mod scan_commands;
pub mod scan_session_commands;
pub mod notifications;
pub mod test_mcp;
pub mod test_proxy;
pub mod test_agent_flow;
pub mod unified_tools;
pub mod vulnerability;
pub mod window;
pub mod workflow_catalog;

// 重新导出所有命令
pub use agent_commands::*;
pub use agent_plugin_commands::*;
pub use ai::*;
pub use ai_commands::*;
pub use asset::*;
pub use config::*;
pub use config_commands::*;
pub use database::*;
pub use dictionary::*;
pub use mcp::*;
pub use passive_scan_commands::*;
pub use performance::*;
pub use plan_execute_commands::*;
pub use plugin_review_commands::*;
pub use prompt_commands::*;
pub use prompt_api::*;
pub use proxifier_commands::*;
pub use rag_commands::*;
pub use react_commands::*;
pub use rewoo_commands::*;
pub use role::*;
pub use scan::*;
pub use scan_commands::*;
pub use scan_session_commands::*;
pub use notifications::*;
pub use test_mcp::*;
pub use unified_tools::*;
pub use vulnerability::*;
pub use window::*;
pub use workflow_catalog::*;



// 在现有的use语句后添加新的MCP命令导出
pub use self::mcp::{
    mcp_check_server_status, mcp_connect_server, mcp_create_custom_tool, mcp_delete_server_config,
    mcp_disconnect_server, mcp_get_connections, mcp_install_tool, mcp_install_tool_from_github,
    mcp_install_tool_from_registry, mcp_install_tool_from_url, mcp_list_tools, mcp_restart_tool,
    mcp_start_tool, mcp_stop_tool, mcp_uninstall_tool, start_mcp_server, stop_mcp_server,
};

use std::process::Command;

/// 检查命令是否存在
#[tauri::command]
pub fn check_command_exists(command: String) -> bool {
    // Windows上使用where命令，Unix系统上使用which命令
    let (check_cmd, args) = if cfg!(target_os = "windows") {
        ("where", vec![&command])
    } else {
        ("which", vec![&command])
    };

    Command::new(check_cmd)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
