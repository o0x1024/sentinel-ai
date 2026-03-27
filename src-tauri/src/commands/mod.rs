//! Commands module - Tauri commands
#![allow(ambiguous_glob_reexports)]

pub mod ai;
pub(crate) mod ai_execution_state_support;
pub mod aisettings;
pub mod asset;
pub mod asset_enrichment_commands;
pub mod bounty_commands;
pub mod bounty_workflow_binding_support;
pub mod bounty_workflow_event_support;
pub mod cache_commands;
pub mod config;
pub mod config_commands;
pub mod database;
pub mod dictionary;
pub mod document_commands;
pub mod http_gateway_commands;
pub mod license_commands;
pub mod llm_test_commands;
pub mod mcp_commands;
pub mod monitor_commands;
pub(crate) mod monitor_execution_heartbeat_support;
pub(crate) mod monitor_finding_support;
pub(crate) mod monitor_notification_support;
pub(crate) mod monitor_plugin_output_support;
pub(crate) mod monitor_progress_support;
mod monitor_surface;
mod monitor_surface_support;
pub mod notifications;
pub mod packet_capture_commands;
pub mod performance;
pub mod plugin_generation_commands;
pub mod plugin_review_commands;
pub mod proxifier_commands;
pub mod rag_commands;
pub mod role;
pub mod scan_session_commands;
pub mod scan_task_commands;
pub mod shell_commands;
pub mod surface_asset_commands;
pub mod surface_commands;
pub mod task_tool_commands;
pub mod team_v3_artifact_store;
pub mod team_v3_commands;
pub mod terminal_commands;
pub mod test_proxy;
pub mod test_tracking_commands;
pub mod tool_commands;
pub mod traffic_analysis_commands;
pub mod window;
pub(crate) mod workflow_notification_support;

// Re-export commands
#[allow(ambiguous_glob_reexports)]
pub use ai::*;
pub use aisettings::*;
pub use asset::*;
pub use bounty_commands::*;
pub use bounty_workflow_binding_support::*;
pub use bounty_workflow_event_support::*;
pub use cache_commands::*;
pub use config::*;
pub use config_commands::*;
pub use database::*;
pub use dictionary::*;
pub use document_commands::*;
pub use http_gateway_commands::*;
pub use license_commands::*;
pub use llm_test_commands::*;
pub use mcp_commands::*;
pub use monitor_commands::*;
pub use notifications::*;
pub use packet_capture_commands::*;
pub use performance::*;
pub use plugin_generation_commands::*;
pub use plugin_review_commands::*;
#[allow(ambiguous_glob_reexports)]
pub use proxifier_commands::*;
pub use rag_commands::*;
pub use role::*;
pub use scan_session_commands::*;
pub use scan_task_commands::*;
pub use shell_commands::*;
pub use surface_asset_commands::*;
pub use surface_commands::*;
pub use task_tool_commands::*;
pub use team_v3_commands::*;
pub use terminal_commands::*;
pub use test_tracking_commands::*;
pub use tool_commands::*;
pub use traffic_analysis_commands::*;
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
