//! Commands module - Tauri commands

pub mod ai;
pub mod asset;
pub mod config;
pub mod config_commands;
pub mod database;
pub mod dictionary;
pub mod license_commands;
pub mod packet_capture_commands;
pub mod passive_scan_commands;
pub mod performance;
pub mod plugin_review_commands;
pub mod prompt_commands;
pub mod prompt_api;
pub mod proxifier_commands;
pub mod rag_commands;
pub mod role;
pub mod scan_session_commands;
pub mod notifications;
pub mod test_proxy;
pub mod tool_commands;
pub mod mcp_commands;
pub mod window;

// Re-export commands
pub use ai::*;
pub use asset::*;
pub use config::*;
pub use config_commands::*;
pub use database::*;
pub use dictionary::*;
pub use packet_capture_commands::*;
pub use passive_scan_commands::*;
pub use performance::*;
pub use plugin_review_commands::*;
pub use prompt_commands::*;
pub use prompt_api::*;
pub use proxifier_commands::*;
pub use rag_commands::*;
pub use role::*;
pub use scan_session_commands::*;
pub use notifications::*;
pub use tool_commands::*;
pub use mcp_commands::*;
pub use window::*;

use std::process::Command;

/// 检查命令是否存在
#[tauri::command]
pub fn check_command_exists(command: String) -> bool {
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
