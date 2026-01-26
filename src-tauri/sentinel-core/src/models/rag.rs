use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub source_id: String,
    pub content: String,
    pub content_hash: String,
    pub chunk_index: usize,
    pub metadata: ChunkMetadata,
    pub embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: u64,
    pub chunk_start_char: usize,
    pub chunk_end_char: usize,
    pub page_number: Option<usize>,
    pub section_title: Option<String>,
    pub custom_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSource {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: u64,
    pub file_hash: String,
    pub chunk_count: usize,
    pub ingestion_status: IngestionStatusEnum,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionStatusEnum { Pending, Processing, Completed, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStatus {
    pub task_id: String,
    pub source_path: String,
    pub status: String,
    pub progress: f64,
    pub total_chunks: usize,
    pub processed_chunks: usize,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult { pub chunk: DocumentChunk, pub score: f64, pub rank: usize }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub document_count: usize,
    pub chunk_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DocumentChunk {
    pub fn new(source_id: String, content: String, chunk_index: usize, metadata: ChunkMetadata) -> Self {
        let content_hash = format!("{:x}", md5::compute(&content));
        Self {
            id: Uuid::new_v4().to_string(),
            source_id,
            content,
            content_hash,
            chunk_index,
            metadata,
            embedding: None,
            created_at: Utc::now(),
        }
    }
}

impl DocumentSource {
    pub fn new(file_path: String, file_name: String, file_type: String, file_size: u64, file_hash: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            file_path,
            file_name,
            file_type,
            file_size,
            file_hash,
            chunk_count: 0,
            ingestion_status: IngestionStatusEnum::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

