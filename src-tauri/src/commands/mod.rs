pub mod ai;
pub mod config;
pub mod database;
pub mod dictionary;
pub mod mcp;
pub mod performance;
pub mod role;
pub mod scan;
pub mod scan_commands;
pub mod scan_engine_commands;
pub mod scan_session_commands;
pub mod vulnerability;

// 重新导出所有命令
pub use ai::*;
pub use config::*;
pub use database::*;
pub use dictionary::*;
pub use mcp::*;
pub use performance::*;
pub use role::*;
pub use scan::*;
pub use scan_commands::*;
pub use scan_engine_commands::*;
pub use scan_session_commands::*;
pub use vulnerability::*;

// 在现有的use语句后添加新的MCP命令导出
pub use self::mcp::{
    mcp_check_server_status, mcp_connect_server, mcp_create_custom_tool, mcp_disconnect_server,
    mcp_get_connections, mcp_install_tool, mcp_install_tool_from_github,
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
