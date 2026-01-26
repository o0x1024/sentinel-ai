use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use sentinel_core::models::rag::{
    DocumentChunk, ChunkMetadata, DocumentSource, IngestionStatusEnum, IngestionStatus, QueryResult, CollectionInfo
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQueryRequest {
    pub query: String,
    #[serde(alias = "collectionId")]
    pub collection_id: Option<String>,
    pub top_k: Option<usize>,
    pub use_mmr: Option<bool>,
    pub mmr_lambda: Option<f64>,
    pub filters: Option<HashMap<String, String>>,
    #[serde(default)]
    pub use_embedding: Option<bool>,
    #[serde(default)]
    pub reranking_enabled: Option<bool>,
    pub similarity_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQueryResponse {
    pub query: String,
    pub results: Vec<QueryResult>,
    pub context: String,
    pub total_tokens: usize,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub source_id: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub page_number: Option<i32>,
    pub section_title: Option<String>,
    pub start_char: usize,
    pub end_char: usize,
    pub score: f64,
    pub content_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantRagRequest {
    pub query: String,
    pub conversation_id: Option<String>,
    pub collection_id: Option<String>,
    pub collection_ids: Option<Vec<String>>,
    pub conversation_history: Option<Vec<String>>,
    pub top_k: Option<usize>,
    pub use_mmr: Option<bool>,
    pub mmr_lambda: Option<f64>,
    pub similarity_threshold: Option<f64>,
    pub reranking_enabled: Option<bool>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantRagResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub context_used: String,
    pub total_tokens_used: usize,
    pub rag_tokens: usize,
    pub llm_tokens: usize,
    pub processing_time_ms: u64,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub file_path: String,
    pub collection_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub source_id: String,
    pub chunks_created: usize,
    pub processing_time_ms: u64,
    pub status: IngestionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagStatus {
    pub collections: Vec<CollectionInfo>,
    pub total_documents: usize,
    pub total_chunks: usize,
    pub database_size_mb: f64,
}
