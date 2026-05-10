//! Commands module - Tauri commands
#![allow(ambiguous_glob_reexports)]

pub(crate) mod agent_continue_resolver;
pub mod agent_task_commands;
pub(crate) mod agent_turn_completion;
pub mod ai;
pub(crate) mod ai_conversation_binding_support;
pub(crate) mod ai_execution_state_support;
pub mod ai_parallel_commands;
pub(crate) mod ai_runtime_command_support;
pub(crate) mod ai_runtime_commands;
pub(crate) mod ai_runtime_harness;
pub(crate) mod ai_runtime_rag_context;
pub(crate) mod ai_runtime_task_config;
pub mod ai_task_commands;
pub(crate) mod ai_task_support;
pub mod ai_turn_logs;
pub mod aisettings;
pub mod api_inventory_commands;
pub mod asset;
pub mod asset_enrichment_commands;
pub(crate) mod assistant_profile_commands;
pub(crate) mod assistant_profile_team_cleanup;
pub mod bot_console_commands;
pub mod bounty_asset_commands;
pub mod bounty_asset_hierarchy_commands;
pub mod bounty_commands;
pub mod bounty_knowledge_commands;
pub mod bounty_report_commands;
pub mod bounty_scope_sync_commands;
pub mod bounty_submission_timeline_commands;
pub mod bounty_workflow_binding_support;
pub mod bounty_workflow_event_support;
pub mod bounty_workflow_orchestration_commands;
pub mod bounty_workflow_template_commands;
pub mod cache_commands;
pub(crate) mod command_response_support;
pub mod config;
pub mod config_commands;
pub mod database;
pub mod dictionary;
pub mod document_commands;
pub mod http_gateway_commands;
pub mod license_commands;
pub mod llm_test_commands;
pub mod mcp_commands;
pub mod memory_commands;
pub mod mention_commands;
pub mod monitor_commands;
pub(crate) mod monitor_config_support;
pub mod monitor_discovery_commands;
pub(crate) mod monitor_execution_heartbeat_support;
pub(crate) mod monitor_finding_support;
pub mod monitor_history_commands;
pub(crate) mod monitor_notification_support;
pub mod monitor_plugin_commands;
pub(crate) mod monitor_plugin_execution_support;
pub(crate) mod monitor_plugin_output_support;
pub(crate) mod monitor_progress_support;
pub(crate) mod monitor_snapshot_support;
mod monitor_surface;
mod monitor_surface_support;
pub mod monitor_task_commands;
pub mod monitor_trigger_commands;
pub mod notifications;
pub mod packet_capture_commands;
pub mod performance;
pub mod plugin_authoring_commands;
pub mod plugin_generation_commands;
pub mod plugin_review_commands;
pub mod plugin_upload_commands;
pub mod proxifier_commands;
pub mod rag_commands;
pub mod role;
pub mod scan_session_commands;
pub mod scan_task_commands;
pub mod security_workbench_commands;
pub mod security_workbench_ignore_commands;
pub(crate) mod security_workbench_storage_support;
pub mod shell_commands;
pub mod surface_asset_commands;
pub mod surface_commands;
pub(crate) mod surface_scope_sync_support;
pub mod surface_seed_commands;
pub mod system_agent_commands;
pub mod task_tool_commands;
pub(crate) mod team_v4_api;
pub(crate) mod team_v4_bootstrap;
pub(crate) mod team_v4_harness;
pub(crate) mod team_v4_mapping;
pub(crate) mod team_v4_schema;
pub mod terminal_commands;
pub mod test_proxy;
pub mod test_tracking_commands;
pub mod tool_commands;
pub mod traffic;
pub mod weixin_gateway_commands;
pub mod window;
pub(crate) mod workflow_notification_support;

// Re-export commands
pub use agent_task_commands::*;
#[allow(ambiguous_glob_reexports)]
pub use ai::*;
pub use ai_parallel_commands::*;
pub use ai_task_commands::*;
pub use ai_turn_logs::*;
pub use aisettings::*;
pub use api_inventory_commands::*;
pub use asset::*;
pub use bot_console_commands::*;
pub use bounty_asset_commands::*;
pub use bounty_asset_hierarchy_commands::*;
pub use bounty_commands::*;
pub use bounty_knowledge_commands::*;
pub use bounty_report_commands::*;
pub use bounty_scope_sync_commands::*;
pub use bounty_submission_timeline_commands::*;
pub use bounty_workflow_binding_support::*;
pub use bounty_workflow_event_support::*;
pub use bounty_workflow_orchestration_commands::*;
pub use bounty_workflow_template_commands::*;
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
pub use memory_commands::*;
pub use mention_commands::*;
pub use monitor_commands::*;
pub use monitor_discovery_commands::*;
pub use monitor_history_commands::*;
pub use monitor_plugin_commands::*;
pub use monitor_task_commands::*;
pub use monitor_trigger_commands::*;
pub use notifications::*;
pub use packet_capture_commands::*;
pub use performance::*;
pub use plugin_authoring_commands::*;
pub use plugin_generation_commands::*;
pub use plugin_review_commands::*;
pub use plugin_upload_commands::*;
#[allow(ambiguous_glob_reexports)]
pub use proxifier_commands::*;
pub use rag_commands::*;
pub use role::*;
pub use scan_session_commands::*;
pub use scan_task_commands::*;
pub use security_workbench_commands::*;
pub use security_workbench_ignore_commands::*;
pub use shell_commands::*;
pub use surface_asset_commands::*;
pub use surface_commands::*;
pub use surface_seed_commands::*;
pub use system_agent_commands::*;
pub use task_tool_commands::*;
pub use terminal_commands::*;
pub use test_tracking_commands::*;
pub use tool_commands::*;
pub use traffic::*;
pub use weixin_gateway_commands::*;
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
