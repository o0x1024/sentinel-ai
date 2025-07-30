pub mod ai;
pub mod database;
pub mod mcp;
pub mod performance;
pub mod project;
pub mod scan;
pub mod vulnerability;

// 导出所有服务
pub use ai::{AiService, AiServiceManager};
pub use mcp::McpService;
pub use scan::ScanService;
pub use vulnerability::VulnerabilityService;
pub use project::ProjectService;
pub use database::DatabaseService;
pub use performance::{PerformanceMonitor, PerformanceMetrics};

// 创建Database包装器以简化AI服务的使用
use std::sync::Arc;
use anyhow::Result;
use crate::models::database::{AiConversation, AiMessage};

#[derive(Debug, Clone)]
pub struct Database {
    service: Arc<DatabaseService>,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let mut service = DatabaseService::new();
        service.initialize().await?;
        Ok(Self {
            service: Arc::new(service),
        })
    }

    // AI相关方法
    pub async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()> {
        self.service.create_conversation(conversation).await
    }

    pub async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>> {
        self.service.get_conversations().await
    }

    pub async fn delete_ai_conversation(&self, id: &str) -> Result<()> {
        self.service.delete_conversation(id).await
    }

    pub async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        self.service.update_conversation_title(id, title).await
    }

    pub async fn archive_ai_conversation(&self, _id: &str) -> Result<()> {
        // 数据库服务中还没有archive方法，我们简单地更新conversation
        // 这里需要添加archive字段的支持，暂时返回OK
        tracing::warn!("archive_ai_conversation feature not fully implemented");
        Ok(())
    }

    pub async fn create_ai_message(&self, message: &AiMessage) -> Result<()> {
        self.service.create_message(message).await
    }

    pub async fn get_ai_messages_by_conversation(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        self.service.get_messages(conversation_id).await
    }

    // 配置相关方法
    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        self.service.get_config(category, key).await
    }

    pub async fn get_configs_by_category(&self, category: &str) -> Result<Vec<crate::models::database::Configuration>> {
        self.service.get_configs_by_category(category).await
    }

    pub async fn set_config(&self, category: &str, key: &str, value: &str, description: Option<&str>) -> Result<()> {
        self.service.set_config(category, key, value, description).await
    }
} 