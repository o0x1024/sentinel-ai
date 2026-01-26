use anyhow::Result;
use async_trait::async_trait;

use crate::core::models::ai::AiRole;
use crate::core::models::database::{
    AiConversation, AiMessage, Configuration, NotificationRule, ScanTask, Vulnerability, ToolExecution, DatabaseStats, ExecutionStatistics, McpServerConfig, MemoryExecution
};
use crate::core::models::agent::{
    AgentTask, AgentSessionData, AgentExecutionResult, SessionLog,
};
use crate::core::models::workflow::WorkflowStepDetail;
use sentinel_plugins::PluginRecord;
use crate::core::models::rag_config::RagConfig;
use crate::core::models::asset::*;
use crate::database_service::rag::{RagCollectionRow, RagDocumentSourceRow, RagChunkRow};
use crate::database_service::proxifier::{ProxifierProxyRecord, ProxifierRuleRecord};
use crate::database_service::ability::{AbilityGroup, AbilityGroupDetail, AbilityGroupSummary, CreateAbilityGroup, UpdateAbilityGroup};
use crate::core::models::scan_session::{
    ScanSession, ScanStage, ScanProgress, CreateScanSessionRequest, UpdateScanSessionRequest,
    ScanSessionStatus,
};

#[async_trait]
pub trait Database: Send + Sync + std::fmt::Debug {
    // AI相关方法
    async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>>;
    async fn get_ai_conversations_paginated(&self, limit: i64, offset: i64) -> Result<Vec<AiConversation>>;
    async fn get_ai_conversations_count(&self) -> Result<i64>;
    async fn get_ai_conversation(&self, id: &str) -> Result<Option<AiConversation>>;
    async fn update_ai_conversation(&self, conversation: &AiConversation) -> Result<()>;
    async fn delete_ai_conversation(&self, id: &str) -> Result<()>;
    async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()>;
    async fn archive_ai_conversation(&self, id: &str) -> Result<()>;
    async fn create_ai_message(&self, message: &AiMessage) -> Result<()>;
    async fn upsert_ai_message_append(&self, message: &AiMessage) -> Result<()>;
    async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AiMessage>>;
    async fn delete_ai_message(&self, message_id: &str) -> Result<()>;
    async fn delete_ai_messages_by_conversation(&self, conversation_id: &str) -> Result<()>;
    async fn delete_ai_messages_after(&self, conversation_id: &str, message_id: &str) -> Result<u64>;
    async fn update_ai_usage(&self, provider: &str, model: &str, input_tokens: i32, output_tokens: i32, cost: f64) -> Result<()>;
    async fn get_ai_usage_stats(&self) -> Result<Vec<crate::core::models::database::AiUsageStats>>;
    async fn clear_ai_usage_stats(&self) -> Result<()>;
    async fn get_aggregated_ai_usage(&self) -> Result<std::collections::HashMap<String, crate::core::models::database::AiUsageStats>>;
    async fn get_ai_roles(&self) -> Result<Vec<AiRole>>;
    async fn create_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn update_ai_role(&self, role: &AiRole) -> Result<()>;
    async fn delete_ai_role(&self, role_id: &str) -> Result<()>;
    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()>;
    async fn get_current_ai_role(&self) -> Result<Option<AiRole>>;
    async fn save_agent_run_state(&self, execution_id: &str, state_json: &str) -> Result<()>;
    async fn get_agent_run_state(&self, execution_id: &str) -> Result<Option<String>>;
    async fn delete_agent_run_state(&self, execution_id: &str) -> Result<()>;

    // 扫描会话相关方法
    async fn create_scan_session(&self, request: CreateScanSessionRequest) -> Result<ScanSession>;
    async fn get_scan_session(&self, session_id: uuid::Uuid) -> Result<Option<ScanSession>>;
    async fn update_scan_session(&self, session_id: uuid::Uuid, request: UpdateScanSessionRequest) -> Result<()>;
    async fn list_scan_sessions(&self, limit: Option<i64>, offset: Option<i64>, status_filter: Option<ScanSessionStatus>) -> Result<Vec<ScanSession>>;
    async fn delete_scan_session(&self, session_id: uuid::Uuid) -> Result<()>;
    async fn create_scan_stage(&self, stage: ScanStage) -> Result<()>;
    async fn update_scan_stage(&self, stage: &ScanStage) -> Result<()>;
    async fn get_scan_session_stages(&self, session_id: uuid::Uuid) -> Result<Vec<ScanStage>>;
    async fn get_scan_progress(&self, session_id: uuid::Uuid) -> Result<Option<ScanProgress>>;

    // 扫描任务相关方法
    async fn create_scan_task(&self, task: &ScanTask) -> Result<()>;
    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>>;
    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>>;
    async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>>;
    async fn update_scan_task_status(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()>;
    async fn delete_scan_task(&self, id: &str) -> Result<()>;
    async fn stop_scan_task(&self, id: &str) -> Result<()>;

    // 漏洞相关方法
    async fn create_vulnerability(&self, v: &Vulnerability) -> Result<()>;
    async fn get_vulnerabilities(&self, project_id: Option<&str>) -> Result<Vec<Vulnerability>>;
    async fn get_vulnerability(&self, id: &str) -> Result<Option<Vulnerability>>;
    async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()>;

    // 工具执行相关方法
    async fn create_tool_execution(&self, exec: &ToolExecution) -> Result<()>;
    async fn update_tool_execution_status(&self, id: &str, status: &str, progress: Option<f64>, end_time: Option<chrono::DateTime<chrono::Utc>>, execution_time: Option<i32>) -> Result<()>;
    async fn get_tool_executions_by_tool(&self, tool_id: &str) -> Result<Vec<ToolExecution>>;

    // Agent任务相关方法
    async fn create_agent_task(&self, task: &AgentTask) -> Result<()>;
    async fn get_agent_task(&self, id: &str) -> Result<Option<AgentTask>>;
    async fn get_agent_tasks(&self, user_id: Option<&str>) -> Result<Vec<AgentTask>>;
    async fn update_agent_task_status(&self, id: &str, status: &str, agent_name: Option<&str>, architecture: Option<&str>) -> Result<()>;
    async fn update_agent_task_timing(&self, id: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, execution_time_ms: Option<u64>) -> Result<()>;
    async fn update_agent_task_error(&self, id: &str, error_message: &str) -> Result<()>;

    // Agent会话相关方法
    async fn create_agent_session(&self, session_id: &str, task_id: &str, agent_name: &str) -> Result<()>;
    async fn update_agent_session_status(&self, session_id: &str, status: &str) -> Result<()>;
    async fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSessionData>>;
    async fn list_agent_sessions(&self) -> Result<Vec<AgentSessionData>>;
    async fn delete_agent_session(&self, session_id: &str) -> Result<()>;
    async fn delete_agent_execution_steps(&self, session_id: &str) -> Result<()>;

    // Agent执行日志相关方法
    async fn add_agent_session_log(&self, session_id: &str, level: &str, message: &str, source: &str) -> Result<()>;
    async fn get_agent_session_logs(&self, session_id: &str) -> Result<Vec<SessionLog>>;

    // Agent执行结果相关方法
    async fn save_agent_execution_result(&self, session_id: &str, result: &AgentExecutionResult) -> Result<()>;
    async fn get_agent_execution_result(&self, session_id: &str) -> Result<Option<AgentExecutionResult>>;

    // Agent执行步骤相关方法
    async fn save_agent_execution_step(&self, session_id: &str, step: &WorkflowStepDetail) -> Result<()>;
    async fn get_agent_execution_steps(&self, session_id: &str) -> Result<Vec<WorkflowStepDetail>>;
    async fn update_agent_execution_step_status(&self, step_id: &str, status: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, duration_ms: Option<u64>, error_message: Option<&str>) -> Result<()>;

    // Memory
    async fn create_memory_execution(&self, record: &MemoryExecution) -> Result<()>;
    async fn get_memory_executions_since(&self, since: Option<chrono::DateTime<chrono::Utc>>, limit: i64) -> Result<Vec<MemoryExecution>>;

    // Workflow相关方法
    async fn create_workflow_run(&self, id: &str, workflow_id: &str, workflow_name: &str, version: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()>;
    async fn update_workflow_run_status(&self, id: &str, status: &str, completed_at: Option<chrono::DateTime<chrono::Utc>>, error_message: Option<&str>) -> Result<()>;
    async fn update_workflow_run_progress(&self, id: &str, progress: u32, completed_steps: u32, total_steps: u32) -> Result<()>;
    async fn save_workflow_run_step(&self, run_id: &str, step_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()>;
    async fn update_workflow_run_step_status(&self, run_id: &str, step_id: &str, status: &str, completed_at: chrono::DateTime<chrono::Utc>, result_json: Option<String>, error_message: Option<&str>) -> Result<()>;
    async fn list_workflow_runs(&self) -> Result<Vec<serde_json::Value>>;
    async fn list_workflow_runs_paginated(&self, page: i64, page_size: i64, search: Option<&str>, workflow_id: Option<&str>) -> Result<(Vec<serde_json::Value>, i64)>;
    async fn get_workflow_run_detail(&self, run_id: &str) -> Result<Option<serde_json::Value>>;
    async fn delete_workflow_run(&self, run_id: &str) -> Result<()>;

    // Workflow Definition相关方法
    async fn save_workflow_definition(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        graph_data: &str,
        is_template: bool,
        is_tool: bool,
        category: Option<&str>,
        tags: Option<&str>,
        version: &str,
        created_by: &str,
    ) -> Result<()>;
    async fn get_workflow_definition(&self, id: &str) -> Result<Option<serde_json::Value>>;
    async fn list_workflow_definitions(&self, is_template: bool) -> Result<Vec<serde_json::Value>>;
    async fn delete_workflow_definition(&self, id: &str) -> Result<()>;
    async fn list_workflow_tools(&self) -> Result<Vec<serde_json::Value>>;

    // 插件相关方法
    async fn get_plugins_from_registry(&self, user_id: Option<&str>) -> Result<Vec<PluginRecord>>;
    async fn get_active_agent_plugins(&self) -> Result<Vec<PluginRecord>>;
    async fn update_plugin_status(&self, plugin_id: &str, status: &str) -> Result<()>;
    async fn update_plugin(&self, metadata: &serde_json::Value, code: &str) -> Result<()>;
    async fn get_plugin_from_registry(&self, plugin_id: &str) -> Result<Option<PluginRecord>>;
    async fn get_plugin_code(&self, plugin_id: &str) -> Result<Option<String>>;
    async fn delete_plugin_from_registry(&self, plugin_id: &str) -> Result<()>;
    async fn get_plugins_paginated(
        &self,
        page: i64,
        page_size: i64,
        status_filter: Option<&str>,
        search_text: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn toggle_plugin_favorite(&self, plugin_id: &str, user_id: Option<&str>) -> Result<bool>;
    async fn get_favorited_plugins(&self, user_id: Option<&str>) -> Result<Vec<String>>;
    async fn get_plugin_review_stats(&self) -> Result<serde_json::Value>;
    async fn update_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()>;
    async fn get_plugin_name(&self, plugin_id: &str) -> Result<Option<String>>;
    async fn get_plugin_summary(&self, plugin_id: &str) -> Result<Option<(String, bool)>>;
    async fn get_plugin_tags(&self, plugin_id: &str) -> Result<Vec<String>>;

    // 配置相关方法
    async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>>;
    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>>;
    async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()>;
    async fn create_notification_rule(&self, rule: &NotificationRule) -> Result<()>;
    async fn get_notification_rules(&self) -> Result<Vec<NotificationRule>>;
    async fn get_notification_rule(&self, id: &str) -> Result<Option<NotificationRule>>;
    async fn update_notification_rule(&self, rule: &NotificationRule) -> Result<()>;
    async fn delete_notification_rule(&self, id: &str) -> Result<()>;
    async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String>;
    async fn get_all_mcp_server_configs(&self) -> Result<Vec<McpServerConfig>>;
    async fn get_auto_connect_mcp_servers(&self) -> Result<Vec<McpServerConfig>>;
    async fn update_mcp_server_auto_connect(&self, id: &str, auto_connect: bool) -> Result<()>;
    async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()>;
    async fn delete_mcp_server_config(&self, id: &str) -> Result<()>;
    async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<McpServerConfig>>;
    async fn update_mcp_server_config(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        enabled: bool,
    ) -> Result<()>;
    async fn get_rag_config(&self) -> Result<Option<RagConfig>>;
    async fn save_rag_config(&self, config: &RagConfig) -> Result<()>;
    async fn get_subdomain_dictionary(&self) -> Result<Vec<String>>;
    async fn set_subdomain_dictionary(&self, dictionary: &[String]) -> Result<()>;
    async fn add_subdomain_words(&self, words: &[String]) -> Result<()>;
    async fn remove_subdomain_words(&self, words: &[String]) -> Result<()>;

    // Cache operations
    async fn get_cache(&self, key: &str) -> Result<Option<String>>;
    async fn set_cache(&self, key: &str, value: &str, cache_type: &str, expires_at: Option<chrono::DateTime<chrono::Utc>>) -> Result<()>;
    async fn delete_cache(&self, key: &str) -> Result<()>;
    async fn cleanup_expired_cache(&self) -> Result<u64>;
    async fn get_all_cache_keys(&self, cache_type: Option<String>) -> Result<Vec<String>>;

    // Sliding Window Memory
    async fn ensure_sliding_window_tables_exist(&self) -> Result<()>;
    async fn get_sliding_window_summaries(&self, conversation_id: &str) -> Result<(Option<crate::core::models::database::GlobalSummary>, Vec<crate::core::models::database::ConversationSegment>)>;
    async fn save_conversation_segment(&self, segment: &crate::core::models::database::ConversationSegment) -> Result<()>;
    async fn upsert_global_summary(&self, summary: &crate::core::models::database::GlobalSummary) -> Result<()>;
    async fn delete_conversation_segments(&self, segment_ids: &[String]) -> Result<()>;

    // RAG相关方法
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String>;
    async fn get_rag_collections(&self) -> Result<Vec<RagCollectionRow>>;
    async fn get_rag_collection_by_id(&self, id: &str) -> Result<Option<RagCollectionRow>>;
    async fn get_rag_collection_by_name(&self, name: &str) -> Result<Option<RagCollectionRow>>;
    async fn update_rag_collection(&self, id: &str, name: &str, description: Option<&str>) -> Result<()>;
    async fn delete_rag_collection(&self, id: &str) -> Result<()>;
    async fn set_rag_collection_active(&self, id: &str, active: bool) -> Result<()>;
    async fn update_collection_stats(&self, id: &str) -> Result<()>;
    async fn get_documents_by_collection_name(&self, name: &str) -> Result<Vec<RagDocumentSourceRow>>;
    async fn get_documents_by_collection_id(&self, id: &str) -> Result<Vec<RagDocumentSourceRow>>;
    async fn insert_document_source(&self, id: &str, collection_id: &str, file_path: &str, file_name: &str, file_type: &str, file_size: i64, file_hash: &str, content_hash: &str, status: &str, metadata: &str, created_at: &str, updated_at: &str) -> Result<()>;
    async fn delete_document_cascade(&self, id: &str) -> Result<()>;
    async fn delete_rag_document(&self, id: &str) -> Result<()>;
    async fn get_collection_id_by_document_id(&self, id: &str) -> Result<Option<String>>;
    async fn insert_chunk(&self, id: &str, document_id: &str, collection_id: &str, content: &str, content_hash: &str, chunk_index: i32, char_count: i32, embedding_bytes: Option<Vec<u8>>, metadata_json: &str, created_at_ts: i64, updated_at_ts: i64) -> Result<()>;
    async fn get_chunks_by_document_id(&self, id: &str) -> Result<Vec<RagChunkRow>>;
    async fn get_rag_documents(&self, collection_id: &str) -> Result<Vec<sentinel_rag::models::DocumentSource>>;
    async fn get_rag_chunks(&self, document_id: &str) -> Result<Vec<sentinel_rag::models::DocumentChunk>>;

    // 资产相关方法
    async fn create_asset(&self, request: CreateAssetRequest, created_by: String) -> Result<Asset>;
    async fn get_asset_by_id(&self, id: &str) -> Result<Option<Asset>>;
    async fn find_asset_by_type_and_value(&self, asset_type: &AssetType, value: &str) -> Result<Option<Asset>>;
    async fn update_asset(&self, id: &str, request: UpdateAssetRequest) -> Result<bool>;
    async fn delete_asset(&self, id: &str) -> Result<bool>;
    async fn list_assets(&self, filter: Option<AssetFilter>, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<Asset>>;
    async fn get_asset_stats(&self) -> Result<AssetStats>;
    async fn create_relationship(&self, source_asset_id: String, target_asset_id: String, relationship_type: RelationshipType, created_by: String) -> Result<AssetRelationship>;
    async fn get_asset_relationships(&self, asset_id: &str) -> Result<(Vec<AssetRelationship>, Vec<AssetRelationship>)>;
    async fn import_assets(&self, request: ImportAssetsRequest, created_by: String) -> Result<ImportResult>;

    // 能力组相关方法
    async fn list_ability_groups_summary(&self) -> Result<Vec<AbilityGroupSummary>>;
    async fn list_ability_groups_summary_by_ids(&self, ids: &[String]) -> Result<Vec<AbilityGroupSummary>>;
    async fn get_ability_group_detail(&self, id: &str) -> Result<Option<AbilityGroupDetail>>;
    async fn get_ability_group(&self, id: &str) -> Result<Option<AbilityGroup>>;
    async fn get_ability_group_by_name(&self, name: &str) -> Result<Option<AbilityGroup>>;
    async fn list_all_ability_groups(&self) -> Result<Vec<AbilityGroup>>;
    async fn create_ability_group(&self, payload: &CreateAbilityGroup) -> Result<AbilityGroup>;
    async fn update_ability_group(&self, id: &str, payload: &UpdateAbilityGroup) -> Result<bool>;
    async fn delete_ability_group(&self, id: &str) -> Result<bool>;

    // Proxifier相关方法
    async fn get_all_proxies(&self) -> Result<Vec<ProxifierProxyRecord>>;
    async fn get_proxy_by_id(&self, id: &str) -> Result<Option<ProxifierProxyRecord>>;
    async fn create_proxy(&self, id: &str, name: &str, host: &str, port: u16, proxy_type: &str, username: Option<&str>, password: Option<&str>, enabled: bool) -> Result<()>;
    async fn update_proxy(&self, id: &str, name: &str, host: &str, port: u16, proxy_type: &str, username: Option<&str>, password: Option<&str>, enabled: bool) -> Result<()>;
    async fn delete_proxy(&self, id: &str) -> Result<()>;
    async fn save_all_proxies(&self, proxies: &[ProxifierProxyRecord]) -> Result<()>;
    async fn get_all_rules(&self) -> Result<Vec<ProxifierRuleRecord>>;
    async fn get_rule_by_id(&self, id: &str) -> Result<Option<ProxifierRuleRecord>>;
    async fn create_rule(&self, id: &str, name: &str, enabled: bool, applications: &str, target_hosts: &str, target_ports: &str, action: &str, proxy_id: Option<&str>) -> Result<()>;
    async fn update_rule(&self, id: &str, name: &str, enabled: bool, applications: &str, target_hosts: &str, target_ports: &str, action: &str, proxy_id: Option<&str>) -> Result<()>;
    async fn delete_rule(&self, id: &str) -> Result<()>;
    async fn save_all_rules(&self, rules: &[ProxifierRuleRecord]) -> Result<()>;

    // Plan-and-Execute
    async fn save_execution_plan(&self, id: &str, name: &str, description: &str, estimated_duration: u64, metadata: &serde_json::Value) -> Result<()>;
    async fn get_execution_plan(&self, id: &str) -> Result<Option<serde_json::Value>>;
    async fn save_execution_session(&self, id: &str, plan_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>, completed_at: Option<chrono::DateTime<chrono::Utc>>, current_step: Option<i32>, progress: f64, context: &serde_json::Value, metadata: &serde_json::Value) -> Result<()>;
    async fn get_execution_session(&self, id: &str) -> Result<Option<serde_json::Value>>;
    async fn list_execution_sessions(&self) -> Result<Vec<serde_json::Value>>;
    async fn delete_execution_session(&self, session_id: &str) -> Result<()>;

    // 统计相关方法
    async fn get_stats(&self) -> Result<DatabaseStats>;
    async fn get_execution_statistics(&self) -> Result<ExecutionStatistics>;
}
