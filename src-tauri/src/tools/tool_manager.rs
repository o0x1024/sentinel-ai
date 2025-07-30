use super::port_scanner::PortScanner;
use super::subdomain_scanner::SubdomainScanner;
use super::{ScanConfig, ScanResult, ScanTool, ToolInfo};
use crate::services::database::DatabaseService;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct ToolManager {
    tools: Arc<RwLock<HashMap<String, Arc<dyn ScanTool>>>>,
    running_scans: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<anyhow::Result<ScanResult>>>>>,
}

impl ToolManager {
    pub async fn new(db_service: Arc<DatabaseService>) -> anyhow::Result<Self> {
        let manager = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            running_scans: Arc::new(RwLock::new(HashMap::new())),
        };

        // 注册默认工具
        let subdomain_scanner = Arc::new(SubdomainScanner::new(db_service)?);
        let port_scanner = Arc::new(PortScanner::new());

        let mut tools = manager.tools.write().await;
        tools.insert("subdomain_scanner".to_string(), subdomain_scanner);
        tools.insert("port_scanner".to_string(), port_scanner);
        drop(tools);

        Ok(manager)
    }

    pub async fn register_tool(&self, tool: Arc<dyn ScanTool>) -> anyhow::Result<()> {
        let mut tools = self.tools.write().await;
        tools.insert(tool.name().to_string(), tool);
        Ok(())
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn ScanTool>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let tools = self.tools.read().await;
        let mut tool_infos = Vec::new();

        for tool in tools.values() {
            tool_infos.push(ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                version: tool.version().to_string(),
                category: "scanner".to_string(), // TODO: 添加分类支持
                supported_options: tool.supported_options(),
                default_config: tool.default_config(),
            });
        }

        tool_infos
    }

    pub async fn start_scan(&self, tool_name: &str, config: ScanConfig) -> anyhow::Result<Uuid> {
        let tool = self
            .get_tool(tool_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("工具 '{}' 未找到", tool_name))?;

        tool.validate_config(&config).await?;

        let scan_id = Uuid::new_v4();
        let tool_clone = tool.clone();

        let handle = tokio::spawn(async move { tool_clone.scan(config).await });

        let mut running_scans = self.running_scans.write().await;
        running_scans.insert(scan_id, handle);

        Ok(scan_id)
    }

    pub async fn get_scan_result(&self, scan_id: Uuid) -> anyhow::Result<Option<ScanResult>> {
        let mut running_scans = self.running_scans.write().await;

        if let Some(handle) = running_scans.remove(&scan_id) {
            match handle.await {
                Ok(result) => Ok(Some(result?)),
                Err(e) => Err(anyhow::anyhow!("扫描任务执行失败: {}", e)),
            }
        } else {
            Ok(None)
        }
    }

    pub async fn cancel_scan(&self, scan_id: Uuid) -> anyhow::Result<()> {
        let mut running_scans = self.running_scans.write().await;

        if let Some(handle) = running_scans.remove(&scan_id) {
            handle.abort();
            Ok(())
        } else {
            Err(anyhow::anyhow!("扫描任务 {} 未找到", scan_id))
        }
    }

    pub async fn list_running_scans(&self) -> Vec<Uuid> {
        let running_scans = self.running_scans.read().await;
        running_scans.keys().cloned().collect()
    }
}

impl ToolManager {
    pub fn default_sync() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            running_scans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
