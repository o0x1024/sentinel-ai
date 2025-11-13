//! 被动扫描工具集成
//!
//! 提供被动扫描工具的延迟注册功能

use super::*;
use crate::commands::passive_scan_commands::PassiveScanState;
use anyhow::Result;
use std::sync::Arc;

/// 注册被动扫描工具提供者到全局工具系统
pub async fn register_passive_tools(
    passive_state: Arc<PassiveScanState>,
    app_handle: tauri::AppHandle,
) -> Result<()> {
    let tool_system = crate::tools::unified_manager::get_global_tool_system()?;
    
    // 获取管理器的写锁
    let manager_lock = tool_system.get_manager();
    let mut manager = manager_lock.write().await;

    // 创建并注册 PassiveToolProvider (with AppHandle for start_passive_scan tool)
    let passive_provider = Box::new(
        PassiveToolProvider::new(passive_state)
            .with_app_handle(app_handle)
    );
    manager.register_provider(passive_provider).await?;

    tracing::info!("Passive scan tools registered successfully");
    Ok(())
}
