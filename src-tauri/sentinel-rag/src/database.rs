use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::EmbeddingConfig;
use crate::models::{DocumentChunk, QueryResult};

use rig::embeddings::EmbeddingModel;
use rig::client::EmbeddingsClient;
use rig::vector_store::VectorStoreIndex;
use rig_lancedb::{LanceDbVectorIndex, SearchParams};
use lancedb::{connect, Connection, Table};
use lancedb::arrow::arrow_schema::{DataType, Field, Fields, Schema};
use arrow_array::{ArrayRef, RecordBatch, StringArray, Int64Array, Float64Array, FixedSizeListArray, Array};
use futures_util::StreamExt;
use std::sync::Arc;
use lancedb::query::QueryBase;
use lancedb::query::ExecutableQuery;

pub struct LanceDbManager {
    database_path: String,
    embedding_config: EmbeddingConfig,
    conn: RwLock<Option<Connection>>,
}

impl LanceDbManager {
    pub fn new(database_path: String, embedding_config: EmbeddingConfig) -> Self {
        Self { database_path, embedding_config, conn: RwLock::new(None) }
    }

    pub async fn initialize(&self) -> Result<()> {
        let db_path = Path::new(&self.database_path);
        if let Some(parent) = db_path.parent() { tokio::fs::create_dir_all(parent).await?; }
        let conn = connect(&self.database_path).execute().await?;
        { let mut guard = self.conn.write().await; *guard = Some(conn); }
        info!("LanceDB connected at: {}", self.database_path);
        Ok(())
    }

    pub async fn create_collection(&self, _collection_name: &str, _embedding_dim: usize) -> Result<()> { info!("Collection will be created on first insert"); Ok(()) }

    pub async fn insert_chunks(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        if chunks.is_empty() { return Ok(0); }
        // For now, compute embeddings via rig-ollama provider (through LanceDbVectorIndex API requires embedding model)
        let ollama = rig::providers::ollama::Client::new();
        let embedding_model = ollama.embedding_model(self.embedding_config.model.as_str());

        let ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();
        let source_ids: Vec<String> = chunks.iter().map(|c| c.source_id.clone()).collect();
        let chunk_indices: Vec<i64> = chunks.iter().map(|c| c.chunk_index as i64).collect();
        let definitions: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let file_names: Vec<String> = chunks.iter().map(|c| c.metadata.file_name.clone()).collect();
        let file_paths: Vec<String> = chunks.iter().map(|c| c.metadata.file_path.clone()).collect();
        let start_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_start_char as i64).collect();
        let end_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_end_char as i64).collect();

        let embeddings = embedding_model.embed_texts(definitions.clone()).await?;
        if embeddings.len() != definitions.len() { return Err(anyhow!("Embedding count mismatch: expected {}, got {}", definitions.len(), embeddings.len())); }
        let embedding_dim = embeddings.first().map(|e| e.vec.len()).unwrap_or(0);
        if embedding_dim == 0 { return Err(anyhow!("Embedding dims is 0")); }
        let mut flat: Vec<f64> = Vec::with_capacity(definitions.len() * embedding_dim);
        for e in embeddings { flat.extend_from_slice(&e.vec); }

        let id_arr = Arc::new(StringArray::from(ids)) as ArrayRef;
        let source_id_arr = Arc::new(StringArray::from(source_ids)) as ArrayRef;
        let chunk_index_arr = Arc::new(Int64Array::from(chunk_indices)) as ArrayRef;
        let definition_arr = Arc::new(StringArray::from(definitions)) as ArrayRef;
        let file_name_arr = Arc::new(StringArray::from(file_names)) as ArrayRef;
        let file_path_arr = Arc::new(StringArray::from(file_paths)) as ArrayRef;
        let start_char_arr = Arc::new(Int64Array::from(start_chars)) as ArrayRef;
        let end_char_arr = Arc::new(Int64Array::from(end_chars)) as ArrayRef;
        let flat_arr = Arc::new(Float64Array::from(flat)) as ArrayRef;
        let list_size_i32: i32 = embedding_dim.try_into().unwrap_or(0);
        let child_field = Arc::new(Field::new("item", DataType::Float64, true));
        let embedding_arr = Arc::new(
            FixedSizeListArray::try_new(child_field, list_size_i32, flat_arr.clone(), None)
                .map_err(|e| anyhow!(e.to_string()))?,
        ) as ArrayRef;
        let schema = Arc::new(Schema::new(Fields::from(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("source_id", DataType::Utf8, true),
            Field::new("chunk_index", DataType::Int64, true),
            Field::new("definition", DataType::Utf8, false),
            Field::new("embedding", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float64, true)), list_size_i32), false),
            Field::new("file_name", DataType::Utf8, true),
            Field::new("file_path", DataType::Utf8, true),
            Field::new("start_char", DataType::Int64, true),
            Field::new("end_char", DataType::Int64, true),
        ])));
        let batch = RecordBatch::try_new(schema.clone(), vec![id_arr, source_id_arr, chunk_index_arr, definition_arr, embedding_arr, file_name_arr, file_path_arr, start_char_arr, end_char_arr]).map_err(|e| anyhow!(e.to_string()))?;
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let table: Table = match conn.open_table(collection_name).execute().await { Ok(t) => t, Err(_) => { let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch.clone())].into_iter(), schema.clone()); conn.create_table(collection_name, Box::new(reader)).execute().await? } };
        let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
        table.add(Box::new(reader)).execute().await?;
        let count = table.count_rows(None).await.unwrap_or(0) as usize;
        info!("Inserted chunks into collection '{}' (total rows: {})", collection_name, count);
        Ok(count)
    }

    pub async fn search_similar(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let table = match conn.open_table(collection_name).execute().await { Ok(t) => t, Err(_) => return Ok(Vec::new()) };
        let ollama = rig::providers::ollama::Client::new();
        let embedding_model = ollama.embedding_model(self.embedding_config.model.as_str());
        let index = LanceDbVectorIndex::new(table, embedding_model, "id", SearchParams::default()).await.map_err(|e| anyhow!(e.to_string()))?;
        let req = rig::vector_store::request::VectorSearchRequest::builder().query(query).samples(top_k as u64).build()?;
        let top_docs = index.top_n::<serde_json::Value>(req).await?;
        let mut results = Vec::new();
        for (rank, (score, _id, value)) in top_docs.into_iter().enumerate() {
            let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let source_id = value.get("source_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let content = value.get("definition").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let file_name = value.get("file_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let file_path = value.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let chunk_index = value.get("chunk_index").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let start_char = value.get("start_char").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let end_char = value.get("end_char").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let chunk = DocumentChunk { id, source_id, content: content.clone(), content_hash: format!("{:x}", md5::compute(content.as_bytes())), chunk_index, metadata: crate::models::ChunkMetadata { file_path, file_name, file_type: "unknown".to_string(), file_size: 0, chunk_start_char: start_char, chunk_end_char: end_char, page_number: None, section_title: None, custom_fields: HashMap::new(), }, embedding: None, created_at: chrono::Utc::now(), };
            results.push(QueryResult { chunk, score: score as f32, rank });
        }
        info!("Found {} similar chunks in collection '{}'", results.len(), collection_name);
        Ok(results)
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let _ = conn.drop_table(collection_name).await; info!("Deleted collection: {}", collection_name); Ok(())
    }

    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let names = conn.table_names().execute().await.unwrap_or_default(); Ok(names)
    }

    pub async fn get_collection_stats(&self, collection_name: &str) -> Result<(usize, usize)> {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let table = match conn.open_table(collection_name).execute().await { Ok(t) => t, Err(_) => return Ok((0, 0)) };
        let chunk_count = table.count_rows(None).await.unwrap_or(0) as usize;
        let mut unique_sources: HashSet<String> = HashSet::new();
        let mut stream = table.query().select(lancedb::query::Select::Columns(vec!["source_id".to_string()])).execute().await.map_err(|e| anyhow!(e.to_string()))?;
        while let Some(batch_res) = stream.next().await { let batch = batch_res.map_err(|e| anyhow!(e.to_string()))?; if let Ok(idx) = batch.schema().index_of("source_id") { let col = batch.column(idx); if let Some(arr) = col.as_any().downcast_ref::<StringArray>() { for i in 0..arr.len() { if arr.is_null(i) { continue; } unique_sources.insert(arr.value(i).to_string()); } } } }
        Ok((unique_sources.len(), chunk_count))
    }
}

