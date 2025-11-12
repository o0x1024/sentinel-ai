//! 内置工具模块
//!
//! 提供系统内置的安全扫描和分析工具

pub mod port_scan;
pub mod subdomain_scan;
// 使用 sentinel-tools 中的实现
pub use sentinel_tools::builtin::{
    HttpRequestConfig, HttpRequestResult, HttpRequestTool, HttpResponse, LocalTimeTool,
};

// 重新导出主要的工具结构体
pub use port_scan::{PortResult, PortScanResults, PortScanTool, PortStatus, ScanConfig};
pub use subdomain_scan::{RSubdomainTool, SubdomainResult, SubdomainScanResults};

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
