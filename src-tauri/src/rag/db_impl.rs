use anyhow::Result;

use crate::services::database::DatabaseService;
use sentinel_rag::db::RagDatabase;
use sentinel_rag::models::{CollectionInfo, DocumentChunk, DocumentSource, QueryResult};

#[async_trait::async_trait]
impl RagDatabase for DatabaseService {
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String> {
        self.create_rag_collection(name, description).await
    }

    async fn get_rag_collections(&self) -> Result<Vec<CollectionInfo>> {
        self.get_rag_collections().await
    }

    async fn get_rag_collection_by_id(&self, id: &str) -> Result<Option<CollectionInfo>> {
        self.get_rag_collection_by_id(id).await
    }

    async fn get_rag_collection_by_name(&self, name: &str) -> Result<Option<CollectionInfo>> {
        self.get_rag_collection_by_name(name).await
    }

    async fn delete_rag_collection(&self, id: &str) -> Result<()> {
        self.delete_rag_collection(id).await
    }

    async fn create_rag_document(
        &self,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        content: &str,
        metadata: &str,
    ) -> Result<String> {
        self.create_rag_document(collection_id, file_path, file_name, content, metadata).await
    }

    async fn create_rag_chunk(
        &self,
        document_id: &str,
        collection_id: &str,
        content: &str,
        chunk_index: i32,
        embedding: Option<&[f32]>,
        embedding_model: &str,
        embedding_dimension: i32,
        metadata_json: &str,
    ) -> Result<String> {
        self.create_rag_chunk(
            document_id,
            collection_id,
            content,
            chunk_index,
            embedding,
            embedding_model,
            embedding_dimension,
            metadata_json,
        )
        .await
    }

    async fn update_collection_stats(&self, collection_id: &str) -> Result<()> {
        self.update_collection_stats(collection_id).await
    }

    async fn get_rag_documents(&self, collection_id: &str) -> Result<Vec<DocumentSource>> {
        self.get_rag_documents_by_collection_id(collection_id).await
    }

    async fn get_rag_chunks(&self, document_id: &str) -> Result<Vec<DocumentChunk>> {
        self.get_rag_chunks_by_document_id(document_id).await
    }

    async fn delete_rag_document(&self, document_id: &str) -> Result<()> {
        self.delete_rag_document(document_id).await
    }

    async fn save_rag_query(
        &self,
        collection_id: Option<&str>,
        query: &str,
        response: &str,
        processing_time_ms: u64,
    ) -> Result<()> {
        // DatabaseService currently expects collection_name; we accept an ID or None
        // For now pass None to keep behavior and avoid wrong mapping
        // TODO: If needed, resolve ID -> name here.
        let collection_name_opt = None;
        self.save_rag_query(collection_name_opt, query, response, processing_time_ms).await
    }

    async fn get_rag_query_history(
        &self,
        collection_id: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<QueryResult>> {
        // Placeholder passthrough; underlying currently returns empty
        let _ = collection_id; // unused currently
        self.get_rag_query_history(None, limit).await
    }
}

