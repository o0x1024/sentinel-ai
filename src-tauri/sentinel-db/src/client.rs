use anyhow::Result;
use sqlx::sqlite::SqlitePool;

use crate::database_service::traits::Database;
use crate::database_service::service::DatabaseService;
use sentinel_core::models::database::MemoryExecution;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct DatabaseClient {
    service: DatabaseService,
}

impl DatabaseClient {
    pub fn new(pool: SqlitePool) -> Self {
        let mut service = DatabaseService::new();
        service.pool = Some(pool);
        Self { service }
    }

    pub fn pool(&self) -> &SqlitePool {
        self.service.get_pool().expect("数据库未初始化")
    }

    // Config
    pub async fn get_config(&self, category: &str, key: &str) -> Result<Option<String>> {
        self.service.get_config(category, key).await
    }
    pub async fn set_config(
        &self,
        category: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.service.set_config(category, key, value, description).await
    }
    pub async fn get_configs_by_category(
        &self,
        category: &str,
    ) -> Result<Vec<sentinel_core::models::database::Configuration>> {
        self.service.get_configs_by_category(category).await
    }

    // MCP
    pub async fn create_mcp_server_config(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        self.service.create_mcp_server_config(name, description, command, args).await
    }
    pub async fn get_all_mcp_server_configs(
        &self,
    ) -> Result<Vec<sentinel_core::models::database::McpServerConfig>> {
        self.service.get_all_mcp_server_configs().await
    }
    pub async fn update_mcp_server_config_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        self.service.update_mcp_server_config_enabled(id, enabled).await
    }
    pub async fn update_mcp_server_auto_connect(&self, id: &str, auto_connect: bool) -> Result<()> {
        self.service.update_mcp_server_auto_connect(id, auto_connect).await
    }
    pub async fn get_auto_connect_mcp_servers(&self) -> Result<Vec<sentinel_core::models::database::McpServerConfig>> {
        self.service.get_auto_connect_mcp_servers().await
    }
    pub async fn delete_mcp_server_config(&self, id: &str) -> Result<()> {
        self.service.delete_mcp_server_config(id).await
    }
    pub async fn get_mcp_server_config_by_name(
        &self,
        name: &str,
    ) -> Result<Option<sentinel_core::models::database::McpServerConfig>> {
        self.service.get_mcp_server_config_by_name(name).await
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
        self.service.update_mcp_server_config(id, name, description, command, args, enabled).await
    }

    // Conversations & Messages
    pub async fn create_ai_conversation(
        &self,
        c: &sentinel_core::models::database::AiConversation,
    ) -> Result<()> {
        self.service.create_ai_conversation(c).await
    }
    pub async fn get_ai_conversations(
        &self,
    ) -> Result<Vec<sentinel_core::models::database::AiConversation>> {
        self.service.get_ai_conversations().await
    }
    pub async fn get_ai_conversations_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<sentinel_core::models::database::AiConversation>> {
        self.service.get_ai_conversations_paginated(limit, offset).await
    }
    pub async fn get_ai_conversations_count(&self) -> Result<i64> {
        self.service.get_ai_conversations_count().await
    }
    pub async fn get_ai_conversation(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::AiConversation>> {
        self.service.get_ai_conversation(id).await
    }
    pub async fn update_ai_conversation(
        &self,
        c: &sentinel_core::models::database::AiConversation,
    ) -> Result<()> {
        self.service.update_ai_conversation(c).await
    }
    pub async fn delete_ai_conversation(&self, id: &str) -> Result<()> {
        self.service.delete_ai_conversation(id).await
    }
    pub async fn update_conversation_title(&self, id: &str, title: &str) -> Result<()> {
        self.service.update_ai_conversation_title(id, title).await
    }
    pub async fn create_ai_message(
        &self,
        m: &sentinel_core::models::database::AiMessage,
    ) -> Result<()> {
        self.service.create_ai_message(m).await
    }
    pub async fn get_ai_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<sentinel_core::models::database::AiMessage>> {
        self.service.get_ai_messages_by_conversation(conversation_id).await
    }

    // Memory
    pub async fn create_memory_execution(&self, record: &MemoryExecution) -> Result<()> {
        self.service.create_memory_execution(record).await
    }
    pub async fn get_memory_executions_since(
        &self,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<MemoryExecution>> {
        self.service.get_memory_executions_since(since, limit).await
    }

    // Scan tasks
    pub async fn create_scan_task(
        &self,
        t: &sentinel_core::models::database::ScanTask,
    ) -> Result<()> {
        self.service.create_scan_task(t).await
    }
    pub async fn get_scan_tasks(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<sentinel_core::models::database::ScanTask>> {
        self.service.get_scan_tasks(project_id).await
    }
    pub async fn get_scan_task(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::ScanTask>> {
        self.service.get_scan_task(id).await
    }
    pub async fn update_scan_task_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
    ) -> Result<()> {
        self.service.update_scan_task_status(id, status, progress).await
    }

    // Vulnerabilities
    pub async fn create_vulnerability(
        &self,
        v: &sentinel_core::models::database::Vulnerability,
    ) -> Result<()> {
        self.service.create_vulnerability(v).await
    }
    pub async fn get_vulnerabilities(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<sentinel_core::models::database::Vulnerability>> {
        self.service.get_vulnerabilities(project_id).await
    }
    pub async fn get_vulnerability(
        &self,
        id: &str,
    ) -> Result<Option<sentinel_core::models::database::Vulnerability>> {
        self.service.get_vulnerability(id).await
    }
    pub async fn update_vulnerability_status(&self, id: &str, status: &str) -> Result<()> {
        self.service.update_vulnerability_status(id, status).await
    }

    // RAG Collections
    pub async fn create_rag_collection(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<String> {
        self.service.create_rag_collection(name, description).await
    }
    pub async fn get_rag_collections(&self) -> Result<Vec<crate::database_service::rag::RagCollectionRow>> {
        self.service.get_rag_collections().await
    }
    pub async fn get_rag_collection_by_id(
        &self,
        id: &str,
    ) -> Result<Option<crate::database_service::rag::RagCollectionRow>> {
        self.service.get_rag_collection_by_id(id).await
    }
    pub async fn get_rag_collection_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::database_service::rag::RagCollectionRow>> {
        self.service.get_rag_collection_by_name(name).await
    }
    pub async fn delete_rag_collection(&self, id: &str) -> Result<()> {
        self.service.delete_rag_collection(id).await
    }
    pub async fn update_rag_collection(&self, id: &str, name: &str, description: Option<&str>) -> Result<()> {
        self.service.update_rag_collection(id, name, description).await
    }
    pub async fn set_rag_collection_active(&self, id: &str, active: bool) -> Result<()> {
        self.service.set_rag_collection_active(id, active).await
    }
    pub async fn update_collection_stats(&self, id: &str) -> Result<()> {
        self.service.update_collection_stats(id).await
    }

    // RAG Docs/Chunks
    pub async fn get_documents_by_collection_name(
        &self,
        collection_name: &str,
    ) -> Result<Vec<crate::database_service::rag::RagDocumentSourceRow>> {
        self.service.get_documents_by_collection_name(collection_name).await
    }
    pub async fn get_documents_by_collection_id(
        &self,
        collection_id: &str,
    ) -> Result<Vec<crate::database_service::rag::RagDocumentSourceRow>> {
        self.service.get_documents_by_collection_id(collection_id).await
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
        status: &str,
        metadata: &str,
        created_at: &str,
        updated_at: &str,
    ) -> Result<()> {
        self.service.insert_document_source(
            id,
            collection_id,
            file_path,
            file_name,
            file_type,
            file_size,
            file_hash,
            content_hash,
            status,
            metadata,
            created_at,
            updated_at,
        )
        .await
    }
    pub async fn delete_document_cascade(&self, document_id: &str) -> Result<()> {
        self.service.delete_document_cascade(document_id).await
    }
    pub async fn get_collection_id_by_document_id(
        &self,
        document_id: &str,
    ) -> Result<Option<String>> {
        self.service.get_collection_id_by_document_id(document_id).await
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
        metadata_json: &str,
        created_at_ts: i64,
        updated_at_ts: i64,
    ) -> Result<()> {
        self.service.insert_chunk(
            id,
            document_id,
            collection_id,
            content,
            content_hash,
            chunk_index,
            char_count,
            embedding_bytes,
            metadata_json,
            created_at_ts,
            updated_at_ts,
        )
        .await
    }
    pub async fn get_chunks_by_document_id(
        &self,
        document_id: &str,
    ) -> Result<Vec<crate::database_service::rag::RagChunkRow>> {
        self.service.get_chunks_by_document_id(document_id).await
    }

    // Tool executions
    pub async fn create_tool_execution(
        &self,
        exec: &sentinel_core::models::database::ToolExecution,
    ) -> Result<()> {
        self.service.create_tool_execution(exec).await
    }

    pub async fn update_tool_execution_status(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        execution_time: Option<i32>,
    ) -> Result<()> {
        self.service.update_tool_execution_status(
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
        self.service.get_tool_executions_by_tool(tool_id).await
    }
}
