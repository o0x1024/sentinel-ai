use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub mod port_scanner;
pub mod subdomain_scanner;
pub mod tool_manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub target: String,
    pub timeout: u64,
    pub threads: usize,
    pub options: HashMap<String, serde_json::Value>,
}

impl ScanConfig {
    pub fn new(target: String) -> Self {
        Self {
            target,
            timeout: 30,
            threads: 10,
            options: HashMap::new(),
        }
    }

    pub fn set_parameter(&mut self, key: String, value: serde_json::Value) {
        self.options.insert(key, value);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: Uuid,
    pub tool_name: String,
    pub target: String,
    pub status: ScanStatus,
    pub results: serde_json::Value,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[async_trait]
pub trait ScanTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;

    async fn validate_config(&self, config: &ScanConfig) -> anyhow::Result<()>;
    async fn scan(&self, config: ScanConfig) -> anyhow::Result<ScanResult>;
    async fn cancel(&self, scan_id: Uuid) -> anyhow::Result<()>;

    fn default_config(&self) -> ScanConfig;
    fn supported_options(&self) -> Vec<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub supported_options: Vec<String>,
    pub default_config: ScanConfig,
}
