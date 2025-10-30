use anyhow::Result;

use crate::models::{CollectionInfo, DocumentChunk, DocumentSource, QueryResult};

#[async_trait::async_trait]
pub trait RagDatabase: Send + Sync {
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> Result<String>;
    async fn get_rag_collections(&self) -> Result<Vec<CollectionInfo>>;
    async fn get_rag_collection_by_id(&self, id: &str) -> Result<Option<CollectionInfo>>;
    async fn get_rag_collection_by_name(&self, name: &str) -> Result<Option<CollectionInfo>>;
    async fn delete_rag_collection(&self, id: &str) -> Result<()>;

    async fn create_rag_document(
        &self,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        content: &str,
        metadata: &str,
    ) -> Result<String>;

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
    ) -> Result<String>;

    async fn update_collection_stats(&self, collection_id: &str) -> Result<()>;

    async fn get_rag_documents(&self, collection_id: &str) -> Result<Vec<DocumentSource>>;
    async fn get_rag_chunks(&self, document_id: &str) -> Result<Vec<DocumentChunk>>;
    async fn delete_rag_document(&self, document_id: &str) -> Result<()>;

    async fn save_rag_query(
        &self,
        collection_id: Option<&str>,
        query: &str,
        response: &str,
        processing_time_ms: u64,
    ) -> Result<()>;
    async fn get_rag_query_history(
        &self,
        collection_id: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<QueryResult>>;
}

