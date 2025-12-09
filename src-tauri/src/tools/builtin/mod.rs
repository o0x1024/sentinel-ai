//! 内置工具模块
//!
//! 提供系统内置的安全扫描和分析工具
//!
//! ## 工具列表
//! - port_scan: 端口扫描
//! - subdomain_scan: 子域名扫描
//! - http_request: HTTP 请求
//! - local_time: 本地时间
//! - bash: Shell 命令执行
//! - bash_output: 后台 Shell 输出
//! - kill_shell: 终止后台 Shell

pub mod port_scan;
pub mod subdomain_scan;
pub mod shell_manager;
pub mod bash_tools;

// 使用 sentinel-tools 中的实现
pub use sentinel_tools::builtin::{
    HttpRequestConfig, HttpRequestResult, HttpRequestTool, HttpResponse, LocalTimeTool,
};

// 重新导出主要的工具结构体
pub use port_scan::{PortResult, PortScanResults, PortScanTool, PortStatus, ScanConfig};
pub use subdomain_scan::{RSubdomainTool, SubdomainResult, SubdomainScanResults};

// 导出 Shell 相关
pub use shell_manager::{
    ShellManager, ShellStatus, ShellResult, ShellInfo,
    initialize_global_shell_manager, get_global_shell_manager,
};
pub use bash_tools::{
    BashTool, BashOutputTool, KillShellTool,
    create_bash_tools, create_bash_tools_with_manager,
};

use super::*;
use crate::services::database::DatabaseService;
use std::sync::Arc;

// ============================================================================
// 内置工具提供者
// ============================================================================

#[derive(Debug)]
pub struct BuiltinToolProvider {
    tools: Vec<Arc<dyn UnifiedTool>>,
}

impl BuiltinToolProvider {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();

        // 添加内置工具（只保留完全实现的工具）
        tools.push(Arc::new(PortScanTool::new()));
        tools.push(Arc::new(RSubdomainTool::new(db_service.clone())));
        tools.push(Arc::new(HttpRequestTool::new()));
        tools.push(Arc::new(LocalTimeTool::new()));

        // 添加 Bash 工具集（共享 ShellManager）
        let (bash_tool, bash_output_tool, kill_shell_tool) = create_bash_tools();
        tools.push(Arc::new(bash_tool));
        tools.push(Arc::new(bash_output_tool));
        tools.push(Arc::new(kill_shell_tool));

        Self { tools }
    }
}

#[async_trait::async_trait]
impl ToolProvider for BuiltinToolProvider {
    fn name(&self) -> &str {
        "builtin"
    }

    fn description(&self) -> &str {
        "Built-in security scanning and analysis tools"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        Ok(self.tools.clone())
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        for tool in &self.tools {
            if tool.name() == name {
                return Ok(Some(tool.clone()));
            }
        }
        Ok(None)
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        tracing::info!("Refreshing builtin tools");
        // 内置工具不需要刷新
        Ok(())
    }
}
