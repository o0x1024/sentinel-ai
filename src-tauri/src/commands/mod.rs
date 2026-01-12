//! Commands module - Tauri commands
#![allow(ambiguous_glob_reexports)]

pub mod ai;
pub mod aisettings;
pub mod asset;
pub mod cache_commands;
pub mod config;
pub mod config_commands;
pub mod database;
pub mod dictionary;
pub mod license_commands;
pub mod mcp_commands;
pub mod notifications;
pub mod packet_capture_commands;
pub mod plugin_generation_commands;
pub mod task_tool_commands;
pub mod test_tracking_commands;
pub mod traffic_analysis_commands;
pub mod performance;
pub mod plugin_review_commands;
pub mod proxifier_commands;
pub mod rag_commands;
pub mod role;
pub mod scan_session_commands;
pub mod scan_task_commands;
pub mod shell_commands;
pub mod terminal_commands;
pub mod test_proxy;
pub mod tool_commands;
pub mod vision_explorer_v2;
pub mod window;

// Re-export commands
#[allow(ambiguous_glob_reexports)]
pub use ai::*;
pub use aisettings::*;
pub use asset::*;
pub use cache_commands::*;
pub use config::*;
pub use config_commands::*;
pub use database::*;
pub use dictionary::*;
pub use license_commands::*;
pub use mcp_commands::*;
pub use notifications::*;
pub use packet_capture_commands::*;
pub use plugin_generation_commands::*;
pub use task_tool_commands::*;
pub use test_tracking_commands::*;
pub use traffic_analysis_commands::*;
pub use performance::*;
pub use plugin_review_commands::*;
#[allow(ambiguous_glob_reexports)]
pub use proxifier_commands::*;
pub use rag_commands::*;
pub use role::*;
pub use scan_session_commands::*;
pub use scan_task_commands::*;
pub use shell_commands::*;
pub use terminal_commands::*;
pub use tool_commands::*;
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
