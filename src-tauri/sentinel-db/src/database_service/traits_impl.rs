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
use chrono::DateTime;
use chrono::Utc;

use crate::database_service::service::DatabaseService;
use crate::database_service::traits::Database;
use sentinel_rag::db::RagDatabase;

#[async_trait]
impl Database for DatabaseService {
    // AI
    async fn create_ai_conversation(&self, conversation: &AiConversation) -> Result<()> {
        Self::create_ai_conversation_internal(self, conversation).await
    }
    async fn get_ai_conversations(&self) -> Result<Vec<AiConversation>> {
        Self::get_ai_conversations_internal(self).await
    }
    async fn get_ai_conversations_paginated(&self, limit: i64, offset: i64) -> Result<Vec<AiConversation>> {
        Self::get_ai_conversations_paginated_internal(self, limit, offset).await
    }
    async fn get_ai_conversations_count(&self) -> Result<i64> {
        Self::get_ai_conversations_count_internal(self).await
    }
    async fn get_ai_conversation(&self, id: &str) -> Result<Option<AiConversation>> {
        Self::get_ai_conversation_internal(self, id).await
    }
    async fn update_ai_conversation(&self, conversation: &AiConversation) -> Result<()> {
        Self::update_ai_conversation_internal(self, conversation).await
    }
    async fn delete_ai_conversation(&self, id: &str) -> Result<()> {
        Self::delete_ai_conversation_internal(self, id).await
    }
    async fn update_ai_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        Self::update_ai_conversation_title_internal(self, id, title).await
    }
    async fn archive_ai_conversation(&self, id: &str) -> Result<()> {
        Self::archive_ai_conversation_internal(self, id).await
    }
    async fn create_ai_message(&self, message: &AiMessage) -> Result<()> {
        Self::create_ai_message_internal(self, message).await
    }
    async fn upsert_ai_message_append(&self, message: &AiMessage) -> Result<()> {
        Self::upsert_ai_message_append_internal(self, message).await
    }
    async fn get_ai_messages_by_conversation(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        Self::get_ai_messages_by_conversation_internal(self, conversation_id).await
    }
    async fn delete_ai_message(&self, message_id: &str) -> Result<()> {
        Self::delete_ai_message_internal(self, message_id).await
    }
    async fn delete_ai_messages_by_conversation(&self, conversation_id: &str) -> Result<()> {
        Self::delete_ai_messages_by_conversation_internal(self, conversation_id).await
    }
    async fn delete_ai_messages_after(&self, conversation_id: &str, message_id: &str) -> Result<u64> {
        Self::delete_ai_messages_after_internal(self, conversation_id, message_id).await
    }
    async fn update_ai_usage(&self, provider: &str, model: &str, input_tokens: i32, output_tokens: i32, cost: f64) -> Result<()> {
        Self::update_ai_usage_internal(self, provider, model, input_tokens, output_tokens, cost).await
    }
    async fn get_ai_usage_stats(&self) -> Result<Vec<crate::core::models::database::AiUsageStats>> {
        Self::get_ai_usage_stats_internal(self).await
    }
    async fn get_aggregated_ai_usage(&self) -> Result<std::collections::HashMap<String, crate::core::models::database::AiUsageStats>> {
        Self::get_aggregated_ai_usage_internal(self).await
    }
    async fn get_ai_roles(&self) -> Result<Vec<AiRole>> {
        Self::get_ai_roles_internal(self).await
    }
    async fn create_ai_role(&self, role: &AiRole) -> Result<()> {
        Self::create_ai_role_internal(self, role).await
    }
    async fn update_ai_role(&self, role: &AiRole) -> Result<()> {
        Self::update_ai_role_internal(self, role).await
    }
    async fn delete_ai_role(&self, role_id: &str) -> Result<()> {
        Self::delete_ai_role_internal(self, role_id).await
    }
    async fn set_current_ai_role(&self, role_id: Option<&str>) -> Result<()> {
        Self::set_current_ai_role_internal(self, role_id).await
    }
    async fn get_current_ai_role(&self) -> Result<Option<AiRole>> {
        Self::get_current_ai_role_internal(self).await
    }

    // Scan Session
    async fn create_scan_session(&self, request: CreateScanSessionRequest) -> Result<ScanSession> {
        Self::create_scan_session_internal(self, request).await
    }
    async fn get_scan_session(&self, session_id: uuid::Uuid) -> Result<Option<ScanSession>> {
        Self::get_scan_session_internal(self, session_id).await
    }
    async fn update_scan_session(&self, session_id: uuid::Uuid, request: UpdateScanSessionRequest) -> Result<()> {
        Self::update_scan_session_internal(self, session_id, request).await
    }
    async fn list_scan_sessions(&self, limit: Option<i64>, offset: Option<i64>, status_filter: Option<ScanSessionStatus>) -> Result<Vec<ScanSession>> {
        Self::list_scan_sessions_internal(self, limit, offset, status_filter).await
    }
    async fn delete_scan_session(&self, session_id: uuid::Uuid) -> Result<()> {
        Self::delete_scan_session_internal(self, session_id).await
    }
    async fn create_scan_stage(&self, stage: ScanStage) -> Result<()> {
        Self::create_scan_stage_internal(self, stage).await
    }
    async fn update_scan_stage(&self, stage: &ScanStage) -> Result<()> {
        Self::update_scan_stage_internal(self, stage).await
    }
    async fn get_scan_session_stages(&self, session_id: uuid::Uuid) -> Result<Vec<ScanStage>> {
        Self::get_scan_session_stages_internal(self, session_id).await
    }
    async fn get_scan_progress(&self, session_id: uuid::Uuid) -> Result<Option<ScanProgress>> {
        Self::get_scan_progress_internal(self, session_id).await
    }

    // Scan
    async fn create_scan_task(&self, task: &ScanTask) -> Result<()> {
        Self::create_scan_task_internal(self, task).await
    }
    async fn get_scan_tasks(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        Self::get_scan_tasks_internal(self, project_id).await
    }
    async fn get_scan_task(&self, id: &str) -> Result<Option<ScanTask>> {
        Self::get_scan_task_internal(self, id).await
    }
    async fn get_scan_tasks_by_target(&self, target: &str) -> Result<Vec<ScanTask>> {
        Self::get_scan_tasks_by_target_internal(self, target).await
    }
    async fn update_scan_task_status(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()> {
        Self::update_scan_task_status_internal(self, id, status, progress).await
    }
    async fn delete_scan_task(&self, id: &str) -> Result<()> {
        Self::delete_scan_task_internal(self, id).await
    }
    async fn stop_scan_task(&self, id: &str) -> Result<()> {
        Self::stop_scan_task_internal(self, id).await
    }

    // Vulnerability
    async fn create_vulnerability(&self, v: &Vulnerability) -> Result<()> {
        Self::create_vulnerability_internal(self, v).await
    }
    async fn get_vulnerabilities(&self, project_id: Option<&str>) -> Result<Vec<Vulnerability>> {
        Self::get_vulnerabilities_internal(self, project_id).await
    }
    async fn get_vulnerability(&self, id: &str) -> Result<Option<Vulnerability>> {
        Self::get_vulnerability_internal(self, id).await
    }
    async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()> {
        Self::update_vulnerability_status_internal(self, id, status).await
    }

    // Tool Execution
    async fn create_tool_execution(&self, exec: &ToolExecution) -> Result<()> {
        Self::create_tool_execution_internal(self, exec).await
    }
    async fn update_tool_execution_status(&self, id: &str, status: &str, progress: Option<f64>, end_time: Option<chrono::DateTime<chrono::Utc>>, execution_time: Option<i32>) -> Result<()> {
        Self::update_tool_execution_status_internal(self, id, status, progress, end_time, execution_time).await
    }
    async fn get_tool_executions_by_tool(&self, tool_id: &str) -> Result<Vec<ToolExecution>> {
        Self::get_tool_executions_by_tool_internal(self, tool_id).await
    }

    // Agent Task
    async fn create_agent_task(&self, task: &AgentTask) -> Result<()> {
        Self::create_agent_task_internal(self, task).await
    }
    async fn get_agent_task(&self, id: &str) -> Result<Option<AgentTask>> {
        Self::get_agent_task_internal(self, id).await
    }
    async fn get_agent_tasks(&self, user_id: Option<&str>) -> Result<Vec<AgentTask>> {
        Self::get_agent_tasks_internal(self, user_id).await
    }
    async fn update_agent_task_status(&self, id: &str, status: &str, agent_name: Option<&str>, architecture: Option<&str>) -> Result<()> {
        Self::update_agent_task_status_internal(self, id, status, agent_name, architecture).await
    }
    async fn update_agent_task_timing(&self, id: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, execution_time_ms: Option<u64>) -> Result<()> {
        Self::update_agent_task_timing_internal(self, id, started_at, completed_at, execution_time_ms).await
    }
    async fn update_agent_task_error(&self, id: &str, error_message: &str) -> Result<()> {
        Self::update_agent_task_error_internal(self, id, error_message).await
    }

    // Agent Session
    async fn create_agent_session(&self, session_id: &str, task_id: &str, agent_name: &str) -> Result<()> {
        Self::create_agent_session_internal(self, session_id, task_id, agent_name).await
    }
    async fn update_agent_session_status(&self, session_id: &str, status: &str) -> Result<()> {
        Self::update_agent_session_status_internal(self, session_id, status).await
    }
    async fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSessionData>> {
        Self::get_agent_session_internal(self, session_id).await
    }
    async fn list_agent_sessions(&self) -> Result<Vec<AgentSessionData>> {
        Self::list_agent_sessions_internal(self).await
    }
    async fn delete_agent_session(&self, session_id: &str) -> Result<()> {
        Self::delete_agent_session_internal(self, session_id).await
    }
    async fn delete_agent_execution_steps(&self, session_id: &str) -> Result<()> {
        Self::delete_agent_execution_steps_internal(self, session_id).await
    }

    // Agent Session Log
    async fn add_agent_session_log(&self, session_id: &str, level: &str, message: &str, source: &str) -> Result<()> {
        Self::add_agent_session_log_internal(self, session_id, level, message, source).await
    }
    async fn get_agent_session_logs(&self, session_id: &str) -> Result<Vec<SessionLog>> {
        Self::get_agent_session_logs_internal(self, session_id).await
    }

    // Agent Execution Result
    async fn save_agent_execution_result(&self, session_id: &str, result: &AgentExecutionResult) -> Result<()> {
        Self::save_agent_execution_result_internal(self, session_id, result).await
    }
    async fn get_agent_execution_result(&self, session_id: &str) -> Result<Option<AgentExecutionResult>> {
        Self::get_agent_execution_result_internal(self, session_id).await
    }

    // Agent Execution Step
    async fn save_agent_execution_step(&self, session_id: &str, step: &WorkflowStepDetail) -> Result<()> {
        Self::save_agent_execution_step_internal(self, session_id, step).await
    }
    async fn get_agent_execution_steps(&self, session_id: &str) -> Result<Vec<WorkflowStepDetail>> {
        Self::get_agent_execution_steps_internal(self, session_id).await
    }
    async fn update_agent_execution_step_status(&self, step_id: &str, status: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, duration_ms: Option<u64>, error_message: Option<&str>) -> Result<()> {
        Self::update_agent_execution_step_status_internal(self, step_id, status, started_at, completed_at, duration_ms, error_message).await
    }

    // Memory
    async fn create_memory_execution(&self, record: &MemoryExecution) -> Result<()> {
        Self::create_memory_execution_internal(self, record).await
    }
    async fn get_memory_executions_since(&self, since: Option<DateTime<Utc>>, limit: i64) -> Result<Vec<MemoryExecution>> {
        Self::get_memory_executions_since_internal(self, since, limit).await
    }

    // Workflow Run
    async fn create_workflow_run(&self, id: &str, workflow_id: &str, workflow_name: &str, version: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        Self::create_workflow_run_internal(self, id, workflow_id, workflow_name, version, status, started_at).await
    }
    async fn update_workflow_run_status(&self, id: &str, status: &str, completed_at: Option<chrono::DateTime<chrono::Utc>>, error_message: Option<&str>) -> Result<()> {
        Self::update_workflow_run_status_internal(self, id, status, completed_at, error_message).await
    }
    async fn update_workflow_run_progress(&self, id: &str, progress: u32, completed_steps: u32, total_steps: u32) -> Result<()> {
        Self::update_workflow_run_progress_internal(self, id, progress, completed_steps, total_steps).await
    }
    async fn save_workflow_run_step(&self, run_id: &str, step_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        Self::save_workflow_run_step_internal(self, run_id, step_id, status, started_at).await
    }
    async fn update_workflow_run_step_status(&self, run_id: &str, step_id: &str, status: &str, completed_at: chrono::DateTime<chrono::Utc>, result_json: Option<String>, error_message: Option<&str>) -> Result<()> {
        Self::update_workflow_run_step_status_internal(self, run_id, step_id, status, completed_at, result_json, error_message).await
    }
    async fn list_workflow_runs(&self) -> Result<Vec<serde_json::Value>> {
        Self::list_workflow_runs_internal(self).await
    }
    async fn list_workflow_runs_paginated(&self, page: i64, page_size: i64, search: Option<&str>, workflow_id: Option<&str>) -> Result<(Vec<serde_json::Value>, i64)> {
        Self::list_workflow_runs_paginated_internal(self, page, page_size, search, workflow_id).await
    }
    async fn get_workflow_run_detail(&self, run_id: &str) -> Result<Option<serde_json::Value>> {
        Self::get_workflow_run_detail_internal(self, run_id).await
    }
    async fn delete_workflow_run(&self, run_id: &str) -> Result<()> {
        Self::delete_workflow_run_internal(self, run_id).await
    }

    // Workflow Definition
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
    ) -> Result<()> {
        Self::save_workflow_definition_internal(self, id, name, description, graph_data, is_template, is_tool, category, tags, version, created_by).await
    }
    async fn get_workflow_definition(&self, id: &str) -> Result<Option<serde_json::Value>> {
        Self::get_workflow_definition_internal(self, id).await
    }
    async fn list_workflow_definitions(&self, is_template: bool) -> Result<Vec<serde_json::Value>> {
        Self::list_workflow_definitions_internal(self, is_template).await
    }
    async fn delete_workflow_definition(&self, id: &str) -> Result<()> {
        Self::delete_workflow_definition_internal(self, id).await
    }
    async fn list_workflow_tools(&self) -> Result<Vec<serde_json::Value>> {
        Self::list_workflow_tools_internal(self).await
    }

    // Plugin
    async fn get_plugins_from_registry(&self) -> Result<Vec<PluginRecord>> {
        Self::get_plugins_from_registry_internal(self).await
    }
    async fn update_plugin_status(&self, plugin_id: &str, status: &str) -> Result<()> {
        Self::update_plugin_status_internal(self, plugin_id, status).await
    }
    async fn update_plugin(&self, metadata: &serde_json::Value, code: &str) -> Result<()> {
        Self::update_plugin_internal(self, metadata, code).await
    }
    async fn get_plugin_from_registry(&self, plugin_id: &str) -> Result<Option<PluginRecord>> {
        Self::get_plugin_from_registry_internal(self, plugin_id).await
    }
    async fn get_plugin_code(&self, plugin_id: &str) -> Result<Option<String>> {
        Self::get_plugin_code_internal(self, plugin_id).await
    }
    async fn delete_plugin_from_registry(&self, plugin_id: &str) -> Result<()> {
        Self::delete_plugin_from_registry_internal(self, plugin_id).await
    }
    async fn get_plugins_paginated(
        &self,
        page: i64,
        page_size: i64,
        status_filter: Option<&str>,
        search_text: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        Self::get_plugins_paginated_internal(self, page, page_size, status_filter, search_text, user_id).await
    }
    async fn toggle_plugin_favorite(&self, plugin_id: &str, user_id: Option<&str>) -> Result<bool> {
        Self::toggle_plugin_favorite_internal(self, plugin_id, user_id).await
    }
    async fn get_favorited_plugins(&self, user_id: Option<&str>) -> Result<Vec<String>> {
        Self::get_favorited_plugins_internal(self, user_id).await
    }
    async fn get_plugin_review_stats(&self) -> Result<serde_json::Value> {
        Self::get_plugin_review_stats_internal(self).await
    }

    // Config
    async fn get_configs_by_category(&self, category: &str) -> Result<Vec<Configuration>> {
        Self::get_configs_by_category_internal(self, category).await
    }
    async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        Self::get_config_internal(self, category, key).await
    }
    async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        Self::set_config_internal(self, category, key, value, description).await
    }
    async fn create_notification_rule(&self, rule: &NotificationRule) -> Result<()> {
        Self::create_notification_rule_internal(self, rule).await
    }
    async fn get_notification_rules(&self) -> Result<Vec<NotificationRule>> {
        Self::get_notification_rules_internal(self).await
    }
    async fn get_notification_rule(&self, id: &str) -> Result<Option<NotificationRule>> {
        Self::get_notification_rule_internal(self, id).await
    }
    async fn update_notification_rule(&self, rule: &NotificationRule) -> Result<()> {
        Self::update_notification_rule_internal(self, rule).await
    }
    async fn delete_notification_rule(&self, id: &str) -> Result<()> {
        Self::delete_notification_rule_internal(self, id).await
    }
    async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        Self::create_mcp_server_config_internal(self, name, description, command, args).await
    }
    async fn get_all_mcp_server_configs(&self) -> Result<Vec<McpServerConfig>> {
        Self::get_all_mcp_server_configs_internal(self).await
    }
    async fn get_auto_connect_mcp_servers(&self) -> Result<Vec<McpServerConfig>> {
        Self::get_auto_connect_mcp_servers_internal(self).await
    }
    async fn update_mcp_server_auto_connect(&self, id: &str, auto_connect: bool) -> Result<()> {
        Self::update_mcp_server_auto_connect_internal(self, id, auto_connect).await
    }
    async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        Self::update_mcp_server_config_enabled_internal(self, id, enabled).await
    }
    async fn delete_mcp_server_config(&self, id: &str) -> Result<()> {
        Self::delete_mcp_server_config_internal(self, id).await
    }
    async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<McpServerConfig>> {
        Self::get_mcp_server_config_by_name_internal(self, name).await
    }
    async fn update_mcp_server_config(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        enabled: bool,
    ) -> Result<()> {
        Self::update_mcp_server_config_internal(self, id, name, description, command, args, enabled).await
    }
    async fn get_rag_config(&self) -> Result<Option<RagConfig>> {
        Self::get_rag_config_internal(self).await
    }
    async fn save_rag_config(&self, config: &RagConfig) -> Result<()> {
        Self::save_rag_config_internal(self, config).await
    }
    async fn get_subdomain_dictionary(&self) -> Result<Vec<String>> {
        Self::get_subdomain_dictionary_internal(self).await
    }
    async fn set_subdomain_dictionary(&self, dictionary: &[String]) -> Result<()> {
        Self::set_subdomain_dictionary_internal(self, dictionary).await
    }
    async fn add_subdomain_words(&self, words: &[String]) -> Result<()> {
        Self::add_subdomain_words_internal(self, words).await
    }
    async fn remove_subdomain_words(&self, words: &[String]) -> Result<()> {
        Self::remove_subdomain_words_internal(self, words).await
    }

    // RAG
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String> {
        Self::create_rag_collection_internal(self, name, description).await
    }
    async fn get_rag_collections(&self) -> Result<Vec<RagCollectionRow>> {
        Self::get_rag_collections_internal(self).await
    }
    async fn get_rag_collection_by_id(&self, id: &str) -> Result<Option<RagCollectionRow>> {
        Self::get_rag_collection_by_id_internal(self, id).await
    }
    async fn get_rag_collection_by_name(&self, name: &str) -> Result<Option<RagCollectionRow>> {
        Self::get_rag_collection_by_name_internal(self, name).await
    }
    async fn update_rag_collection(&self, id: &str, name: &str, description: Option<&str>) -> Result<()> {
        Self::update_rag_collection_internal(self, id, name, description).await
    }
    async fn delete_rag_collection(&self, id: &str) -> Result<()> {
        Self::delete_rag_collection_internal(self, id).await
    }
    async fn set_rag_collection_active(&self, id: &str, active: bool) -> Result<()> {
        Self::set_rag_collection_active_internal(self, id, active).await
    }
    async fn update_collection_stats(&self, id: &str) -> Result<()> {
        Self::update_collection_stats_internal(self, id).await
    }
    async fn get_documents_by_collection_name(&self, name: &str) -> Result<Vec<RagDocumentSourceRow>> {
        Self::get_documents_by_collection_name_internal(self, name).await
    }
    async fn get_documents_by_collection_id(&self, id: &str) -> Result<Vec<RagDocumentSourceRow>> {
        Self::get_documents_by_collection_id_internal(self, id).await
    }
    async fn insert_document_source(&self, id: &str, collection_id: &str, file_path: &str, file_name: &str, file_type: &str, file_size: i64, file_hash: &str, content_hash: &str, status: &str, metadata: &str, created_at: &str, updated_at: &str) -> Result<()> {
        Self::insert_document_source_internal(self, id, collection_id, file_path, file_name, file_type, file_size, file_hash, content_hash, status, metadata, created_at, updated_at).await
    }
    async fn delete_document_cascade(&self, id: &str) -> Result<()> {
        Self::delete_document_cascade_internal(self, id).await
    }
    async fn delete_rag_document(&self, id: &str) -> Result<()> {
        Self::delete_rag_document_internal(self, id).await
    }
    async fn get_collection_id_by_document_id(&self, id: &str) -> Result<Option<String>> {
        Self::get_collection_id_by_document_id_internal(self, id).await
    }
    async fn insert_chunk(&self, id: &str, document_id: &str, collection_id: &str, content: &str, content_hash: &str, chunk_index: i32, char_count: i32, embedding_bytes: Option<Vec<u8>>, metadata_json: &str, created_at_ts: i64, updated_at_ts: i64) -> Result<()> {
        Self::insert_chunk_internal(self, id, document_id, collection_id, content, content_hash, chunk_index, char_count, embedding_bytes, metadata_json, created_at_ts, updated_at_ts).await
    }
    async fn get_chunks_by_document_id(&self, id: &str) -> Result<Vec<RagChunkRow>> {
        Self::get_chunks_by_document_id_internal(self, id).await
    }
    async fn get_rag_documents(&self, collection_id: &str) -> Result<Vec<sentinel_rag::models::DocumentSource>> {
        Self::get_rag_documents_internal(self, collection_id).await
    }
    async fn get_rag_chunks(&self, document_id: &str) -> Result<Vec<sentinel_rag::models::DocumentChunk>> {
        Self::get_rag_chunks_internal(self, document_id).await
    }

    // Asset
    async fn create_asset(&self, request: CreateAssetRequest, created_by: String) -> Result<Asset> {
        Self::create_asset_internal(self, request, created_by).await
    }
    async fn get_asset_by_id(&self, id: &str) -> Result<Option<Asset>> {
        Self::get_asset_by_id_internal(self, id).await
    }
    async fn find_asset_by_type_and_value(&self, asset_type: &AssetType, value: &str) -> Result<Option<Asset>> {
        Self::find_asset_by_type_and_value_internal(self, asset_type, value).await
    }
    async fn update_asset(&self, id: &str, request: UpdateAssetRequest) -> Result<bool> {
        Self::update_asset_internal(self, id, request).await
    }
    async fn delete_asset(&self, id: &str) -> Result<bool> {
        Self::delete_asset_internal(self, id).await
    }
    async fn list_assets(&self, filter: Option<AssetFilter>, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<Asset>> {
        Self::list_assets_internal(self, filter, limit, offset).await
    }
    async fn get_asset_stats(&self) -> Result<AssetStats> {
        Self::get_asset_stats_internal(self).await
    }
    async fn create_relationship(&self, source_asset_id: String, target_asset_id: String, relationship_type: RelationshipType, created_by: String) -> Result<AssetRelationship> {
        Self::create_relationship_internal(self, source_asset_id, target_asset_id, relationship_type, created_by).await
    }
    async fn get_asset_relationships(&self, asset_id: &str) -> Result<(Vec<AssetRelationship>, Vec<AssetRelationship>)> {
        Self::get_asset_relationships_internal(self, asset_id).await
    }
    async fn import_assets(&self, request: ImportAssetsRequest, created_by: String) -> Result<ImportResult> {
        Self::import_assets_internal(self, request, created_by).await
    }

    // Ability
    async fn list_ability_groups_summary(&self) -> Result<Vec<AbilityGroupSummary>> {
        Self::list_ability_groups_summary_internal(self).await
    }
    async fn list_ability_groups_summary_by_ids(&self, ids: &[String]) -> Result<Vec<AbilityGroupSummary>> {
        Self::list_ability_groups_summary_by_ids_internal(self, ids).await
    }
    async fn get_ability_group_detail(&self, id: &str) -> Result<Option<AbilityGroupDetail>> {
        Self::get_ability_group_detail_internal(self, id).await
    }
    async fn get_ability_group(&self, id: &str) -> Result<Option<AbilityGroup>> {
        Self::get_ability_group_internal(self, id).await
    }
    async fn get_ability_group_by_name(&self, name: &str) -> Result<Option<AbilityGroup>> {
        Self::get_ability_group_by_name_internal(self, name).await
    }
    async fn list_all_ability_groups(&self) -> Result<Vec<AbilityGroup>> {
        Self::list_all_ability_groups_internal(self).await
    }
    async fn create_ability_group(&self, payload: &CreateAbilityGroup) -> Result<AbilityGroup> {
        Self::create_ability_group_internal(self, payload).await
    }
    async fn update_ability_group(&self, id: &str, payload: &UpdateAbilityGroup) -> Result<bool> {
        Self::update_ability_group_internal(self, id, payload).await
    }
    async fn delete_ability_group(&self, id: &str) -> Result<bool> {
        Self::delete_ability_group_internal(self, id).await
    }

    // Proxifier
    async fn get_all_proxies(&self) -> Result<Vec<ProxifierProxyRecord>> {
        Self::get_all_proxies_internal(self).await
    }
    async fn get_proxy_by_id(&self, id: &str) -> Result<Option<ProxifierProxyRecord>> {
        Self::get_proxy_by_id_internal(self, id).await
    }
    async fn create_proxy(&self, id: &str, name: &str, host: &str, port: u16, proxy_type: &str, username: Option<&str>, password: Option<&str>, enabled: bool) -> Result<()> {
        Self::create_proxy_internal(self, id, name, host, port, proxy_type, username, password, enabled).await
    }
    async fn update_proxy(&self, id: &str, name: &str, host: &str, port: u16, proxy_type: &str, username: Option<&str>, password: Option<&str>, enabled: bool) -> Result<()> {
        Self::update_proxy_internal(self, id, name, host, port, proxy_type, username, password, enabled).await
    }
    async fn delete_proxy(&self, id: &str) -> Result<()> {
        Self::delete_proxy_internal(self, id).await
    }
    async fn save_all_proxies(&self, proxies: &[ProxifierProxyRecord]) -> Result<()> {
        Self::save_all_proxies_internal(self, proxies).await
    }
    async fn get_all_rules(&self) -> Result<Vec<ProxifierRuleRecord>> {
        Self::get_all_rules_internal(self).await
    }
    async fn get_rule_by_id(&self, id: &str) -> Result<Option<ProxifierRuleRecord>> {
        Self::get_rule_by_id_internal(self, id).await
    }
    async fn create_rule(&self, id: &str, name: &str, enabled: bool, applications: &str, target_hosts: &str, target_ports: &str, action: &str, proxy_id: Option<&str>) -> Result<()> {
        Self::create_rule_internal(self, id, name, enabled, applications, target_hosts, target_ports, action, proxy_id).await
    }
    async fn update_rule(&self, id: &str, name: &str, enabled: bool, applications: &str, target_hosts: &str, target_ports: &str, action: &str, proxy_id: Option<&str>) -> Result<()> {
        Self::update_rule_internal(self, id, name, enabled, applications, target_hosts, target_ports, action, proxy_id).await
    }
    async fn delete_rule(&self, id: &str) -> Result<()> {
        Self::delete_rule_internal(self, id).await
    }
    async fn save_all_rules(&self, rules: &[ProxifierRuleRecord]) -> Result<()> {
        Self::save_all_rules_internal(self, rules).await
    }

    // Stats
    async fn get_stats(&self) -> Result<DatabaseStats> {
        Self::get_stats_internal(self).await
    }
    async fn get_execution_statistics(&self) -> Result<ExecutionStatistics> {
        Self::get_execution_statistics_internal(self).await
    }

    // Plan-and-Execute
    async fn save_execution_plan(&self, id: &str, name: &str, description: &str, estimated_duration: u64, metadata: &serde_json::Value) -> Result<()> {
        Self::save_execution_plan_internal(self, id, name, description, estimated_duration, metadata).await
    }
    async fn get_execution_plan(&self, id: &str) -> Result<Option<serde_json::Value>> {
        Self::get_execution_plan_internal(self, id).await
    }
    async fn save_execution_session(&self, id: &str, plan_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>, completed_at: Option<chrono::DateTime<chrono::Utc>>, current_step: Option<i32>, progress: f64, context: &serde_json::Value, metadata: &serde_json::Value) -> Result<()> {
        Self::save_execution_session_internal(self, id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata).await
    }
    async fn get_execution_session(&self, id: &str) -> Result<Option<serde_json::Value>> {
        Self::get_execution_session_internal(self, id).await
    }
    async fn list_execution_sessions(&self) -> Result<Vec<serde_json::Value>> {
        Self::list_execution_sessions_internal(self).await
    }
    async fn delete_execution_session(&self, session_id: &str) -> Result<()> {
        Self::delete_execution_session_internal(self, session_id).await
    }
}

#[async_trait]
impl RagDatabase for DatabaseService {
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String> {
        Self::create_rag_collection_internal(self, name, description).await
    }
    async fn get_rag_collections(&self) -> Result<Vec<sentinel_rag::models::CollectionInfo>> {
        let rows = Self::get_rag_collections_internal(self).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(sentinel_rag::models::CollectionInfo {
                id: r.id,
                name: r.name,
                description: r.description,
                is_active: r.is_active,
                document_count: r.document_count as usize,
                chunk_count: r.chunk_count as usize,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap_or_default().with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap_or_default().with_timezone(&chrono::Utc),
            });
        }
        Ok(out)
    }
    async fn get_rag_collection_by_id(&self, id: &str) -> Result<Option<sentinel_rag::models::CollectionInfo>> {
        let r = Self::get_rag_collection_by_id_internal(self, id).await?;
        Ok(r.map(|r| sentinel_rag::models::CollectionInfo {
            id: r.id,
            name: r.name,
            description: r.description,
            is_active: r.is_active,
            document_count: r.document_count as usize,
            chunk_count: r.chunk_count as usize,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap_or_default().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap_or_default().with_timezone(&chrono::Utc),
        }))
    }
    async fn get_rag_collection_by_name(&self, name: &str) -> Result<Option<sentinel_rag::models::CollectionInfo>> {
        let r = Self::get_rag_collection_by_name_internal(self, name).await?;
        Ok(r.map(|r| sentinel_rag::models::CollectionInfo {
            id: r.id,
            name: r.name,
            description: r.description,
            is_active: r.is_active,
            document_count: r.document_count as usize,
            chunk_count: r.chunk_count as usize,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap_or_default().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap_or_default().with_timezone(&chrono::Utc),
        }))
    }
    async fn delete_rag_collection(&self, id: &str) -> Result<()> {
        Self::delete_rag_collection_internal(self, id).await
    }
    async fn create_rag_document(&self, collection_id: &str, file_path: &str, file_name: &str, content: &str, metadata: &str) -> Result<String> {
        Self::create_rag_document_internal(self, collection_id, file_path, file_name, content, metadata).await
    }
    async fn insert_document_source(&self, id: &str, collection_id: &str, file_path: &str, file_name: &str, file_type: &str, file_size: i64, file_hash: &str, content_hash: &str, status: &str, metadata: &str, created_at: &str, updated_at: &str) -> Result<()> {
        Self::insert_document_source_internal(self, id, collection_id, file_path, file_name, file_type, file_size, file_hash, content_hash, status, metadata, created_at, updated_at).await
    }
    async fn create_rag_chunk(&self, document_id: &str, collection_id: &str, content: &str, chunk_index: i32, embedding: Option<&[f32]>, metadata_json: &str) -> Result<String> {
        Self::create_rag_chunk_internal(self, document_id, collection_id, content, chunk_index, embedding, metadata_json).await
    }
    async fn update_collection_stats(&self, collection_id: &str) -> Result<()> {
        Self::update_collection_stats_internal(self, collection_id).await
    }
    async fn get_rag_documents(&self, collection_id: &str) -> Result<Vec<sentinel_rag::models::DocumentSource>> {
        Self::get_rag_documents_internal(self, collection_id).await
    }
    async fn get_rag_documents_paginated(&self, collection_id: &str, limit: i64, offset: i64, search_query: Option<&str>) -> Result<(Vec<sentinel_rag::models::DocumentSource>, i64)> {
        Self::get_rag_documents_paginated_internal(self, collection_id, limit, offset, search_query).await
    }
    async fn get_rag_chunks(&self, document_id: &str) -> Result<Vec<sentinel_rag::models::DocumentChunk>> {
        Self::get_rag_chunks_internal(self, document_id).await
    }
    async fn delete_rag_document(&self, document_id: &str) -> Result<()> {
        Self::delete_rag_document_internal(self, document_id).await
    }
    async fn save_rag_query(&self, collection_id: Option<&str>, conversation_id: Option<&str>, query: &str, response: &str, processing_time_ms: u64) -> Result<()> {
        Self::save_rag_query_internal(self, collection_id, conversation_id, query, response, processing_time_ms).await
    }
    async fn get_rag_query_history(&self, collection_id: Option<&str>, limit: Option<i32>) -> Result<Vec<sentinel_rag::models::QueryResult>> {
        Self::get_rag_query_history_internal(self, collection_id, limit).await
    }
}
