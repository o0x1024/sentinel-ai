use anyhow::Result;
use sqlx::sqlite::SqlitePool;

use crate::database::*;
use sentinel_core::models::prompt::{
    PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem,
    PromptCategory, TemplateType,
};

#[derive(Clone, Debug)]
pub struct DatabaseClient {
    pool: SqlitePool,
}

impl DatabaseClient {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // Prompt templates
    pub async fn insert_default_templates(&self) -> Result<()> {
        prompt_template_dao::insert_default_templates(self.pool()).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // Config
    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        config_dao::get_config(self.pool(), category, key).await
    }
    pub async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        config_dao::set_config(self.pool(), category, key, value, description).await
    }
    pub async fn get_configs_by_category(
        &self,
        category: &str,
    ) -> Result<Vec<sentinel_core::models::database::Configuration>> {
        config_dao::get_configs_by_category(self.pool(), category).await
    }

    // MCP
    pub async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        mcp_dao::create_mcp_server_config(self.pool(), name, description, command, args).await
    }
    pub async fn get_all_mcp_server_configs(
        &self,
    ) -> Result<Vec<sentinel_core::models::database::McpServerConfig>> {
        mcp_dao::get_all_mcp_server_configs(self.pool()).await
    }
    pub async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        mcp_dao::update_mcp_server_config_enabled(self.pool(), id, enabled).await
    }
    pub async fn delete_mcp_server_config(&self, id: &str) -> Result<()> {
        mcp_dao::delete_mcp_server_config(self.pool(), id).await
    }
    pub async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<sentinel_core::models::database::McpServerConfig>> {
        mcp_dao::get_mcp_server_config_by_name(self.pool(), name).await
    }
    pub async fn update_mcp_server_config(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        enabled: bool,
    ) -> Result<()> {
        mcp_dao::update_mcp_server_config(self.pool(), id, name, description, command, args, enabled)
            .await
    }

    // Conversations & Messages
    pub async fn create_ai_conversation(
        &self,
        c: &sentinel_core::models::database::AiConversation,
    ) -> Result<()> {
        ai_conversation_dao::create_ai_conversation(self.pool(), c).await
    }
    pub async fn get_ai_conversations(
        &self,
    ) -> Result<Vec<sentinel_core::models::database::AiConversation>> {
        ai_conversation_dao::get_ai_conversations(self.pool()).await
    }
    pub async fn get_ai_conversation(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::AiConversation>> {
        ai_conversation_dao::get_ai_conversation(self.pool(), id).await
    }
    pub async fn update_ai_conversation(
        &self,
        c: &sentinel_core::models::database::AiConversation,
    ) -> Result<()> {
        ai_conversation_dao::update_ai_conversation(self.pool(), c).await
    }
    pub async fn delete_ai_conversation(&self, id: &str) -> Result<()> {
        ai_conversation_dao::delete_ai_conversation(self.pool(), id).await
    }
    pub async fn update_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        ai_conversation_dao::update_conversation_title(self.pool(), id, title).await
    }
    pub async fn create_ai_message(
        &self,
        m: &sentinel_core::models::database::AiMessage,
    ) -> Result<()> {
        ai_conversation_dao::create_ai_message(self.pool(), m).await
    }
    pub async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<sentinel_core::models::database::AiMessage>> {
        ai_conversation_dao::get_ai_messages_by_conversation(self.pool(), conversation_id).await
    }

    // Scan tasks
    pub async fn create_scan_task(
        &self,
        t: &sentinel_core::models::database::ScanTask,
    ) -> Result<()> {
        scan_task_dao::create_scan_task(self.pool(), t).await
    }
    pub async fn get_scan_tasks(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<sentinel_core::models::database::ScanTask>> {
        scan_task_dao::get_scan_tasks(self.pool(), project_id).await
    }
    pub async fn get_scan_task(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::ScanTask>> {
        scan_task_dao::get_scan_task(self.pool(), id).await
    }
    pub async fn update_scan_task_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
    ) -> Result<()> {
        scan_task_dao::update_scan_task_status(self.pool(), id, status, progress).await
    }

    // Vulnerabilities
    pub async fn create_vulnerability(
        &self,
        v: &sentinel_core::models::database::Vulnerability,
    ) -> Result<()> {
        vulnerability_dao::create_vulnerability(self.pool(), v).await
    }
    pub async fn get_vulnerabilities(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<sentinel_core::models::database::Vulnerability>> {
        vulnerability_dao::get_vulnerabilities(self.pool(), project_id).await
    }
    pub async fn get_vulnerability(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::Vulnerability>> {
        vulnerability_dao::get_vulnerability(self.pool(), id).await
    }
    pub async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()> {
        vulnerability_dao::update_vulnerability_status(self.pool(), id, status).await
    }

    // RAG Collections
    pub async fn create_rag_collection(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<String> {
        rag_collection_dao::create_rag_collection(self.pool(), name, description).await
    }
    pub async fn get_rag_collections(&self) -> Result<Vec<rag_collection_dao::RagCollectionRow>> {
        rag_collection_dao::get_rag_collections(self.pool()).await
    }
    pub async fn get_rag_collection_by_id(
        &self,
        id: &str,
    ) -> Result<Option<rag_collection_dao::RagCollectionRow>> {
        rag_collection_dao::get_rag_collection_by_id(self.pool(), id).await
    }
    pub async fn get_rag_collection_by_name(
        &self,
        name: &str,
    ) -> Result<Option<rag_collection_dao::RagCollectionRow>> {
        rag_collection_dao::get_rag_collection_by_name(self.pool(), name).await
    }
    pub async fn delete_rag_collection(&self, id: &str) -> Result<()> {
        rag_collection_dao::delete_rag_collection(self.pool(), id).await
    }
    pub async fn set_rag_collection_active(&self, id: &str, active: bool) -> Result<()> {
        rag_collection_dao::set_rag_collection_active(self.pool(), id, active).await
    }
    pub async fn update_collection_stats(&self, id: &str) -> Result<()> {
        rag_collection_dao::update_collection_stats(self.pool(), id).await
    }

    // RAG Docs/Chunks
    pub async fn get_documents_by_collection_name(
        &self,
        collection_name: &str,
    ) -> Result<Vec<rag_doc_dao::RagDocumentSourceRow>> {
        rag_doc_dao::get_documents_by_collection_name(self.pool(), collection_name).await
    }
    pub async fn get_documents_by_collection_id(
        &self,
        collection_id: &str,
    ) -> Result<Vec<rag_doc_dao::RagDocumentSourceRow>> {
        rag_doc_dao::get_documents_by_collection_id(self.pool(), collection_id).await
    }
    pub async fn insert_document_source(
        &self,
        id: &str,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        file_type: &str,
        file_size: i64,
        file_hash: &str,
        content_hash: &str,
        metadata: &str,
        created_at: &str,
        updated_at: &str,
    ) -> Result<()> {
        rag_doc_dao::insert_document_source(
            self.pool(),
            id,
            collection_id,
            file_path,
            file_name,
            file_type,
            file_size,
            file_hash,
            content_hash,
            metadata,
            created_at,
            updated_at,
        )
        .await
    }
    pub async fn delete_document_cascade(&self, document_id: &str) -> Result<()> {
        rag_doc_dao::delete_document_cascade(self.pool(), document_id).await
    }
    pub async fn get_collection_id_by_document_id(
        &self,
        document_id: &str,
    ) -> Result<Option<String>> {
        rag_doc_dao::get_collection_id_by_document_id(self.pool(), document_id).await
    }
    pub async fn insert_chunk(
        &self,
        id: &str,
        document_id: &str,
        collection_id: &str,
        content: &str,
        content_hash: &str,
        chunk_index: i32,
        char_count: i32,
        embedding_bytes: Option<Vec<u8>>,
        embedding_model: &str,
        embedding_dimension: i32,
        metadata_json: &str,
        created_at_ts: i64,
        updated_at_ts: i64,
    ) -> Result<()> {
        rag_doc_dao::insert_chunk(
            self.pool(),
            id,
            document_id,
            collection_id,
            content,
            content_hash,
            chunk_index,
            char_count,
            embedding_bytes,
            embedding_model,
            embedding_dimension,
            metadata_json,
            created_at_ts,
            updated_at_ts,
        )
        .await
    }
    pub async fn get_chunks_by_document_id(
        &self,
        document_id: &str,
    ) -> Result<Vec<rag_doc_dao::RagChunkRow>> {
        rag_doc_dao::get_chunks_by_document_id(self.pool(), document_id).await
    }

    // Tool executions
    pub async fn create_tool_execution(
        &self,
        exec: &sentinel_core::models::database::ToolExecution,
    ) -> Result<()> {
        tool_execution_dao::create_tool_execution(self.pool(), exec).await
    }

    // Prompt templates
    pub async fn list_templates(&self) -> Result<Vec<sentinel_core::models::prompt::PromptTemplate>> {
        prompt_dao::list_templates(self.pool()).await
    }
    pub async fn get_template(&self, id: i64) -> Result<Option<sentinel_core::models::prompt::PromptTemplate>> {
        prompt_dao::get_template(self.pool(), id).await
    }
    pub async fn get_template_by_arch_stage(
        &self,
        arch: ArchitectureType,
        stage: StageType,
    ) -> Result<Option<PromptTemplate>> {
        prompt_dao::get_template_by_arch_stage(self.pool(), arch, stage).await
    }
    pub async fn create_template(&self, t: &PromptTemplate) -> Result<i64> {
        prompt_dao::create_template(self.pool(), t).await
    }
    pub async fn update_template(&self, id: i64, t: &PromptTemplate) -> Result<()> {
        prompt_dao::update_template(self.pool(), id, t).await
    }
    pub async fn delete_template(&self, id: i64) -> Result<()> {
        prompt_dao::delete_template(self.pool(), id).await
    }

    // User prompt config
    pub async fn get_user_configs(&self) -> Result<Vec<UserPromptConfig>> {
        prompt_dao::get_user_configs(self.pool()).await
    }
    pub async fn update_user_config(&self, arch: ArchitectureType, stage: StageType, template_id: i64) -> Result<()> {
        prompt_dao::update_user_config(self.pool(), arch, stage, template_id).await
    }
    pub async fn get_active_prompt(&self, arch: ArchitectureType, stage: StageType) -> Result<Option<String>> {
        prompt_dao::get_active_prompt(self.pool(), arch, stage).await
    }

    // Groups
    pub async fn list_groups(&self, arch: Option<ArchitectureType>) -> Result<Vec<PromptGroup>> {
        prompt_dao::list_groups(self.pool(), arch).await
    }
    pub async fn create_group(&self, g: &PromptGroup) -> Result<i64> {
        prompt_dao::create_group(self.pool(), g).await
    }
    pub async fn update_group(&self, id: i64, g: &PromptGroup) -> Result<()> {
        prompt_dao::update_group(self.pool(), id, g).await
    }
    pub async fn delete_group(&self, id: i64) -> Result<()> {
        prompt_dao::delete_group(self.pool(), id).await
    }
    pub async fn set_arch_default_group(&self, arch: ArchitectureType, group_id: i64) -> Result<()> {
        prompt_dao::set_arch_default_group(self.pool(), arch, group_id).await
    }
    pub async fn upsert_group_item(&self, group_id: i64, stage: StageType, template_id: i64) -> Result<()> {
        prompt_dao::upsert_group_item(self.pool(), group_id, stage, template_id).await
    }
    pub async fn list_group_items(&self, group_id: i64) -> Result<Vec<PromptGroupItem>> {
        prompt_dao::list_group_items(self.pool(), group_id).await
    }
    pub async fn remove_group_item(&self, group_id: i64, stage: StageType) -> Result<()> {
        prompt_dao::remove_group_item(self.pool(), group_id, stage).await
    }

    pub async fn list_templates_filtered(
        &self,
        category: Option<PromptCategory>,
        template_type: Option<TemplateType>,
        architecture: Option<ArchitectureType>,
        is_system: Option<bool>,
    ) -> Result<Vec<PromptTemplate>> {
        prompt_dao::list_templates_filtered(self.pool(), category, template_type, architecture, is_system).await
    }
    pub async fn duplicate_template(&self, id: i64, new_name: Option<String>) -> Result<i64> {
        prompt_dao::duplicate_template(self.pool(), id, new_name).await
    }
    pub async fn update_tool_execution_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        execution_time: Option<i32>,
    ) -> Result<()> {
        tool_execution_dao::update_tool_execution_status(
            self.pool(),
            id,
            status,
            progress,
            end_time,
            execution_time,
        )
        .await
    }
    pub async fn get_tool_executions_by_tool(
        &self,
        tool_id: &str,
    ) -> Result<Vec<sentinel_core::models::database::ToolExecution>> {
        tool_execution_dao::get_tool_executions_by_tool(self.pool(), tool_id).await
    }
}


