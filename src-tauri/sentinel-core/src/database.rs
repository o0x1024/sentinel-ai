//! Database trait - 数据库操作抽象接口
//! 
//! 定义数据库操作的统一接口，允许 engines 模块不依赖具体的 DatabaseService 实现

use async_trait::async_trait;
use anyhow::Result;
use crate::models::database::{
    AiConversation, AiMessage, Configuration, DatabaseStats, McpServerConfig,
    ScanTask, Vulnerability,
};
use crate::models::ai::AiRole;

#[async_trait]
pub trait Database: Send + Sync + std::fmt::Debug {
    // AI 对话相关
    async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>>;
    async fn get_ai_conversation(&self, id: &str) -> Result<Option<AiConversation>>;
    async fn update_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn delete_ai_conversation(&self, id: &str) -> Result<()>;
    async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()>;
    async fn archive_ai_conversation(&self, id: &str) -> Result<()>;
    
    // AI 消息相关
    async fn create_ai_message(&self, message: &AiMessage) -> Result<()>;
    async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessage>>;
    
    // 配置相关
    async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>>;
    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>>;
    async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()>;
    
    // AI 角色相关
    async fn get_ai_roles(&self) -> Result<Vec<AiRole>>;
    async fn create_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn update_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn delete_ai_role(&self, role_id: &str) -> Result<()>;
    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()>;
    async fn get_current_ai_role(&self) -> Result<Option<AiRole>>;
    
    // 扫描任务相关
    async fn create_scan_task(&self, task: &ScanTask) -> Result<()>;
    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>>;
    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>>;
    async fn update_scan_task(&self, task: &ScanTask) -> Result<()>;
    async fn delete_scan_task(&self, id: &str) -> Result<()>;
    
    // 漏洞相关
    async fn create_vulnerability(&self, vuln: &Vulnerability) -> Result<()>;
    async fn get_vulnerabilities(&self, scan_task_id: Option<&str>) -> Result<Vec<Vulnerability>>;
    async fn get_vulnerability(&self, id: &str) -> Result<Option<Vulnerability>>;
    async fn update_vulnerability(&self, vuln: &Vulnerability) -> Result<()>;
    async fn delete_vulnerability(&self, id: &str) -> Result<()>;
    
    // MCP 服务器配置相关
    async fn create_mcp_server_config(&self, config: &McpServerConfig) -> Result<()>;
    async fn get_mcp_server_configs(&self) -> Result<Vec<McpServerConfig>>;
    async fn get_mcp_server_config(&self, id: &str) -> Result<Option<McpServerConfig>>;
    async fn update_mcp_server_config(&self, config: &McpServerConfig) -> Result<()>;
    async fn delete_mcp_server_config(&self, id: &str) -> Result<()>;
    
    // RAG 相关
    async fn get_rag_config(&self) -> Result<Option<serde_json::Value>>;
    async fn get_rag_collections(&self) -> Result<Vec<serde_json::Value>>;
    
    // 数据库池访问
    fn get_pool(&self) -> Option<sqlx::sqlite::SqlitePool>;
}

/// Prompt 仓库 trait
#[async_trait]
pub trait PromptRepository: Send + Sync {
    async fn get_template(&self, id: i64) -> Result<Option<crate::models::prompt::PromptTemplate>>;
    async fn get_template_by_arch_stage(
        &self,
        arch: crate::models::prompt::ArchitectureType,
        stage: crate::models::prompt::StageType,
    ) -> Result<Option<crate::models::prompt::PromptTemplate>>;
    async fn get_active_prompt(
        &self,
        arch: crate::models::prompt::ArchitectureType,
        stage: crate::models::prompt::StageType,
    ) -> Result<Option<String>>;
    async fn list_group_items(&self, group_id: i64) -> Result<Vec<crate::models::prompt::PromptGroupItem>>;
}

