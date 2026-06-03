//! AI 服务工厂 trait

use async_trait::async_trait;
use anyhow::Result;
use crate::chunk_type::ChunkType;
use std::sync::Arc;
use crate::database::Database;

/// AI 配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// AI 服务工厂 trait
#[async_trait]
pub trait AiServiceFactory: Send + Sync {
    /// 获取提供商配置
    async fn get_provider_config(&self, provider: &str) -> Result<Option<AiConfig>>;
    
    /// 获取数据库 Arc
    fn get_db_arc(&self) -> Arc<dyn Database>;
    
    /// 获取特定阶段的AI配置
    async fn get_ai_config_for_stage(&self, stage: crate::scheduler::SchedulerStage) -> Result<Option<AiConfig>>;
}
