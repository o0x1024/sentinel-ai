pub mod ai;
pub mod asset_service;
pub mod mcp;
pub mod database {
    pub use sentinel_db::DatabaseService;
    pub use sentinel_db::Database;
}
pub mod prompt_db;
pub mod scan;
pub mod scan_session;
pub mod vulnerability;
pub mod prompt_service;

// 从 sentinel-services crate 重新导出已迁移的模块
pub use sentinel_services::message_emitter;
pub use sentinel_services::performance;
pub use sentinel_services::dictionary;

// 导出所有服务
pub use ai::{AiService, AiServiceManager};
pub use asset_service::AssetService;
pub use database::DatabaseService;
pub use mcp::McpService;

// 从 sentinel-services 重新导出
pub use sentinel_services::performance::{
    PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
};
pub use sentinel_services::message_emitter::TauriMessageEmitter;
pub use sentinel_services::dictionary::DictionaryService;


pub use prompt_service::{
    PromptService, PromptServiceConfig, PromptSession, ExecutionRecord,
    SessionPerformanceStats, PromptBuildRequest, PromptBuildType,
    PromptBuildResponse, OptimizationRequest, ValidationSettings,
    ServiceStats, HealthStatus,
};

// Re-export from commands module
pub use scan::ScanService;
pub use scan_session::ScanSessionService;
pub use vulnerability::VulnerabilityService;

// 创建Database包装器以简化AI服务的使用
use crate::models::database::{AiConversation, AiMessage};
use anyhow::Result;
use std::sync::Arc;
use sentinel_core::models::database as core_db;
use serde_json;

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
        let v = serde_json::to_value(conversation)?;
        let core: core_db::AiConversation = serde_json::from_value(v)?;
        self.service.create_conversation(&core).await
    }

    pub async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>> {
        let data: Vec<core_db::AiConversation> = self.service.get_conversations().await?;
        let mut out = Vec::with_capacity(data.len());
        for x in data {
            let v = serde_json::to_value(x)?;
            let local: AiConversation = serde_json::from_value(v)?;
            out.push(local);
        }
        Ok(out)
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
        let v = serde_json::to_value(message)?;
        let core: core_db::AiMessage = serde_json::from_value(v)?;
        self.service.create_message(&core).await
    }

    pub async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessage>> {
        let data: Vec<core_db::AiMessage> = self.service.get_messages(conversation_id).await?;
        let mut out = Vec::with_capacity(data.len());
        for x in data {
            let v = serde_json::to_value(x)?;
            let local: AiMessage = serde_json::from_value(v)?;
            out.push(local);
        }
        Ok(out)
    }

    // 配置相关方法
    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        self.service.get_config(category, key).await
    }

    pub async fn get_configs_by_category(
        &self,
        category: &str,
    ) -> Result<Vec<crate::models::database::Configuration>> {
        let data: Vec<core_db::Configuration> = self.service.get_configs_by_category(category).await?;
        let mut out = Vec::with_capacity(data.len());
        for x in data {
            let v = serde_json::to_value(x)?;
            let local: crate::models::database::Configuration = serde_json::from_value(v)?;
            out.push(local);
        }
        Ok(out)
    }

    pub async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.service
            .set_config(category, key, value, description)
            .await
    }
}
