use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::config::EmbeddingConfig;
use crate::models::{DocumentChunk, QueryResult};

use rig::embeddings::embedding::EmbeddingModel;
use rig::client::ProviderClient;
use rig::client::EmbeddingsClient;
use rig::vector_store::VectorStoreIndex;
use rig::vector_store::VectorSearchRequest;
use rig_lancedb::{LanceDbVectorIndex, SearchParams};
use lancedb::{connect, Connection};
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
        
        let provider = self.embedding_config.provider.to_lowercase();
        info!("Using embedding provider: {}, model: {}", provider, self.embedding_config.model);
        
        // Dispatch to provider-specific implementation
        match provider.as_str() {
            "ollama" => self.insert_chunks_ollama(collection_name, chunks).await,
            "openai" => self.insert_chunks_openai(collection_name, chunks).await,
            "cohere" => self.insert_chunks_cohere(collection_name, chunks).await,
            "anthropic" => self.insert_chunks_anthropic(collection_name, chunks).await,
            "gemini" | "google" => self.insert_chunks_gemini(collection_name, chunks).await,
            "deepseek" => self.insert_chunks_deepseek(collection_name, chunks).await,
            "moonshot" => self.insert_chunks_moonshot(collection_name, chunks).await,
            "openrouter" => self.insert_chunks_openrouter(collection_name, chunks).await,
            "modelscope" => self.insert_chunks_modelscope(collection_name, chunks).await,
            "lm studio" | "lmstudio" | "lm_studio" => self.insert_chunks_lmstudio(collection_name, chunks).await,
            _ => Err(anyhow!("Unsupported embedding provider: {}. Please check your RAG configuration.", provider))
        }
    }
    
    async fn insert_chunks_ollama(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        let base_url = self.embedding_config.base_url.as_deref().unwrap_or("http://localhost:11434");
        info!("Using Ollama embedding service at: {}", base_url);
        
        let client = rig::providers::ollama::Client::builder()
            .api_key(rig::client::Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;
            
        let dimensions = self.embedding_config.dimensions.unwrap_or(768);
        let embedding_model: rig::providers::ollama::EmbeddingModel<reqwest::Client> = rig::providers::ollama::EmbeddingModel::new(client, &self.embedding_config.model, dimensions);
        
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_openai(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_cohere(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .ok_or_else(|| anyhow!("Cohere API key not configured"))?;
        std::env::set_var("COHERE_API_KEY", &api_key_str);
        let client = rig::providers::cohere::Client::from_env();
        // Cohere requires input_type parameter: "search_document" for indexing
        let embedding_model = client.embedding_model(&self.embedding_config.model, "search_document");
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_anthropic(&self, _collection_name: &str, _chunks: Vec<DocumentChunk>) -> Result<usize> {
        // Anthropic doesn't have native embedding models
        warn!("Anthropic doesn't provide embedding models. Consider using OpenAI or other providers for embeddings.");
        Err(anyhow!("Anthropic doesn't support embedding models. Please use OpenAI, Cohere, or other embedding providers."))
    }
    
    async fn insert_chunks_gemini(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| anyhow!("Google/Gemini API key not configured"))?;
        std::env::set_var("GOOGLE_API_KEY", &api_key_str);
        let client = rig::providers::gemini::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_deepseek(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        // DeepSeek uses OpenAI-compatible API
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .ok_or_else(|| anyhow!("DeepSeek API key not configured"))?;
        
        // For OpenAI-compatible APIs, we use OpenAI client
        // Note: Custom base URLs need to be set via environment or client configuration
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_moonshot(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        // Moonshot uses OpenAI-compatible API
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| anyhow!("Moonshot API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_openrouter(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        // OpenRouter uses OpenAI-compatible API
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| anyhow!("OpenRouter API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_modelscope(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        // ModelScope may use OpenAI-compatible API or custom implementation
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("MODELSCOPE_API_KEY").ok())
            .ok_or_else(|| anyhow!("ModelScope API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.insert_chunks_impl(collection_name, chunks, embedding_model).await
    }
    
    async fn insert_chunks_lmstudio(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        let base_url = self.embedding_config.base_url.as_deref().unwrap_or("http://localhost:1234");
        info!("Using LM Studio embedding service at: {}", base_url);
        
        let ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();
        let source_ids: Vec<String> = chunks.iter().map(|c| c.source_id.clone()).collect();
        let chunk_indices: Vec<i64> = chunks.iter().map(|c| c.chunk_index as i64).collect();
        let definitions: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let file_names: Vec<String> = chunks.iter().map(|c| c.metadata.file_name.clone()).collect();
        let file_paths: Vec<String> = chunks.iter().map(|c| c.metadata.file_path.clone()).collect();
        let start_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_start_char as i64).collect();
        let end_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_end_char as i64).collect();

        // Call LM Studio embedding API
        let embeddings = self.call_lmstudio_embedding(&definitions).await?;
        
        if embeddings.len() != definitions.len() { 
            return Err(anyhow!("Embedding count mismatch: expected {}, got {}", definitions.len(), embeddings.len())); 
        }
        let embedding_dim = embeddings.first().map(|e| e.len()).unwrap_or(0);
        if embedding_dim == 0 { return Err(anyhow!("Embedding dims is 0")); }
        let mut flat: Vec<f64> = Vec::with_capacity(definitions.len() * embedding_dim);
        for e in &embeddings { flat.extend(e.iter().map(|&f| f as f64)); }

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
        
        // Check if table exists and validate schema
        if let Ok(existing_table) = conn.open_table(collection_name).execute().await {
            // Validate embedding dimensions
            let existing_schema = existing_table.schema().await?;
            if let Some(embedding_field) = existing_schema.field_with_name("embedding").ok() {
                if let DataType::FixedSizeList(_, existing_dim) = embedding_field.data_type() {
                    if *existing_dim != list_size_i32 {
                        return Err(anyhow!(
                            "Embedding dimension mismatch: Collection '{}' was created with dimension {} but current model generates dimension {}. \
                            Please either: 1) Use the same embedding model (check RAG settings), or 2) Delete this collection and recreate it with the new model.",
                            collection_name, existing_dim, list_size_i32
                        ));
                    }
                }
            }
            // Table exists, append data
            let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
            existing_table.add(Box::new(reader)).execute().await
                .map_err(|e| anyhow!("Failed to insert into vector store: {}", e))?;
        } else {
            // Create new table with data
            let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
            conn.create_table(collection_name, Box::new(reader)).execute().await?;
        }
        
        // Get final count
        let table = conn.open_table(collection_name).execute().await?;
        let count = table.count_rows(None).await.unwrap_or(0) as usize;
        info!("Inserted {} chunks into collection '{}' (total rows: {})", chunks.len(), collection_name, count);
        Ok(chunks.len())
    }
    
    async fn call_lmstudio_embedding(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base_url = self.embedding_config.base_url.as_deref().unwrap_or("http://localhost:1234");
        // Apply global proxy configuration
        let builder = reqwest::Client::builder();
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        if let Some(api_key) = &self.embedding_config.api_key {
            headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
        }
        
        let input = if texts.len() == 1 {
            serde_json::Value::String(texts[0].clone())
        } else {
            serde_json::Value::Array(texts.iter().map(|t| serde_json::Value::String(t.clone())).collect())
        };
        let payload = serde_json::json!({ "model": self.embedding_config.model, "input": input });
        
        let mut retry_count = 0;
        let max_retries = 3;
        loop {
            let response = client.post(&format!("{}/v1/embeddings", base_url))
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await;
                
            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let response_text = resp.text().await?;
                    if status.is_success() {
                        let result: serde_json::Value = serde_json::from_str(&response_text)?;
                        if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
                            let mut out = Vec::new();
                            for item in data {
                                if let Some(ev) = item.get("embedding").and_then(|v| v.as_array()) {
                                    let embedding: Vec<f32> = ev.iter()
                                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                                        .collect();
                                    out.push(embedding);
                                } else {
                                    return Err(anyhow!("Invalid embedding format in LM Studio response"));
                                }
                            }
                            return Ok(out);
                        } else {
                            return Err(anyhow!("LM Studio response data invalid"));
                        }
                    } else {
                        error!("LM Studio request failed (status: {}): {}", status, response_text);
                        return Err(anyhow!("LM Studio embedding request failed ({}): {}", status, response_text));
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!("Failed to call LM Studio after {} retries: {}", max_retries, e);
                        return Err(anyhow!("Failed to call LM Studio embedding API: {}. Please check if LM Studio is running at {}", e, base_url));
                    }
                    let delay = std::time::Duration::from_secs(2u64.pow(retry_count));
                    warn!("LM Studio request failed (attempt {}/{}): {}. Retrying in {:?}...", retry_count, max_retries, e, delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    async fn insert_chunks_impl<M: EmbeddingModel>(&self, collection_name: &str, chunks: Vec<DocumentChunk>, embedding_model: M) -> Result<usize> {

        let ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();
        let source_ids: Vec<String> = chunks.iter().map(|c| c.source_id.clone()).collect();
        let chunk_indices: Vec<i64> = chunks.iter().map(|c| c.chunk_index as i64).collect();
        let definitions: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let file_names: Vec<String> = chunks.iter().map(|c| c.metadata.file_name.clone()).collect();
        let file_paths: Vec<String> = chunks.iter().map(|c| c.metadata.file_path.clone()).collect();
        let start_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_start_char as i64).collect();
        let end_chars: Vec<i64> = chunks.iter().map(|c| c.metadata.chunk_end_char as i64).collect();

        // Retry embedding with exponential backoff
        let mut retry_count = 0;
        let max_retries = 3;
        let embeddings = loop {
            match embedding_model.embed_texts(definitions.clone()).await {
                Ok(emb) => break emb,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!("Failed to generate embeddings after {} retries: {}", max_retries, e);
                        return Err(anyhow!("Failed to generate embeddings: {}. Please check if embedding service ({}) is running", e, self.embedding_config.provider));
                    }
                    let delay = std::time::Duration::from_secs(2u64.pow(retry_count));
                    warn!("Embedding request failed (attempt {}/{}): {}. Retrying in {:?}...", retry_count, max_retries, e, delay);
                    tokio::time::sleep(delay).await;
                }
            }
        };
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
        
        // Check if table exists and validate schema
        if let Ok(existing_table) = conn.open_table(collection_name).execute().await {
            // Table exists, validate embedding dimensions
            let existing_schema = existing_table.schema().await?;
            
            // Extract embedding dimension from existing schema
            if let Some(embedding_field) = existing_schema.field_with_name("embedding").ok() {
                if let DataType::FixedSizeList(_, existing_dim) = embedding_field.data_type() {
                    if *existing_dim != list_size_i32 {
                        return Err(anyhow!(
                            "Embedding dimension mismatch: Collection '{}' was created with dimension {} but current model generates dimension {}. \
                            Please either: 1) Use the same embedding model (check RAG settings), or 2) Delete this collection and recreate it with the new model.",
                            collection_name, existing_dim, list_size_i32
                        ));
                    }
                }
            }
            
            // Dimensions match, add the data
            let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
            existing_table.add(Box::new(reader)).execute().await
                .map_err(|e| anyhow!("Failed to insert into vector store: {}", e))?;
        } else {
            // Table doesn't exist, create it with the data
            let reader = arrow_array::RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
            conn.create_table(collection_name, Box::new(reader)).execute().await?;
        }
        
        // Get final count
        let table = conn.open_table(collection_name).execute().await?;
        let count = table.count_rows(None).await.unwrap_or(0) as usize;
        info!("Inserted {} chunks into collection '{}' (total rows: {})", chunks.len(), collection_name, count);
        Ok(chunks.len())
    }

    pub async fn search_similar(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let provider = self.embedding_config.provider.to_lowercase();
        
        // Dispatch to provider-specific implementation
        match provider.as_str() {
            "ollama" => self.search_similar_ollama(collection_name, query, top_k).await,
            "openai" => self.search_similar_openai(collection_name, query, top_k).await,
            "cohere" => self.search_similar_cohere(collection_name, query, top_k).await,
            "anthropic" => self.search_similar_anthropic(collection_name, query, top_k).await,
            "gemini" | "google" => self.search_similar_gemini(collection_name, query, top_k).await,
            "deepseek" => self.search_similar_deepseek(collection_name, query, top_k).await,
            "moonshot" => self.search_similar_moonshot(collection_name, query, top_k).await,
            "openrouter" => self.search_similar_openrouter(collection_name, query, top_k).await,
            "modelscope" => self.search_similar_modelscope(collection_name, query, top_k).await,
            "lm studio" | "lmstudio" | "lm_studio" => self.search_similar_lmstudio(collection_name, query, top_k).await,
            _ => Err(anyhow!("Unsupported embedding provider: {}. Please check your RAG configuration.", provider))
        }
    }
    
    async fn search_similar_ollama(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let base_url = self.embedding_config.base_url.as_deref().unwrap_or("http://localhost:11434");
        info!("Using Ollama embedding service for search at: {}", base_url);
        
        let client = rig::providers::ollama::Client::builder()
            .api_key(rig::client::Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;
            
        let dimensions = self.embedding_config.dimensions.unwrap_or(768);
        let embedding_model: rig::providers::ollama::EmbeddingModel<reqwest::Client> = rig::providers::ollama::EmbeddingModel::new(client, &self.embedding_config.model, dimensions);
        
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_openai(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_cohere(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .ok_or_else(|| anyhow!("Cohere API key not configured"))?;
        std::env::set_var("COHERE_API_KEY", &api_key_str);
        let client = rig::providers::cohere::Client::from_env();
        // Cohere requires input_type parameter: "search_query" for querying
        let embedding_model = client.embedding_model(&self.embedding_config.model, "search_query");
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_anthropic(&self, _collection_name: &str, _query: &str, _top_k: usize) -> Result<Vec<QueryResult>> {
        warn!("Anthropic doesn't provide embedding models.");
        Err(anyhow!("Anthropic doesn't support embedding models. Please use OpenAI, Cohere, or other embedding providers."))
    }
    
    async fn search_similar_gemini(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| anyhow!("Google/Gemini API key not configured"))?;
        std::env::set_var("GOOGLE_API_KEY", &api_key_str);
        let client = rig::providers::gemini::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_deepseek(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .ok_or_else(|| anyhow!("DeepSeek API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_moonshot(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| anyhow!("Moonshot API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_openrouter(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| anyhow!("OpenRouter API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_modelscope(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let api_key_str = self.embedding_config.api_key.clone()
            .or_else(|| std::env::var("MODELSCOPE_API_KEY").ok())
            .ok_or_else(|| anyhow!("ModelScope API key not configured"))?;
        
        std::env::set_var("OPENAI_API_KEY", &api_key_str);
        let client = rig::providers::openai::Client::from_env();
        let embedding_model = client.embedding_model(&self.embedding_config.model);
        self.search_similar_impl(collection_name, query, top_k, embedding_model).await
    }
    
    async fn search_similar_lmstudio(&self, collection_name: &str, query: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let base_url = self.embedding_config.base_url.as_deref().unwrap_or("http://localhost:1234");
        info!("Using LM Studio embedding service for search at: {}", base_url);
        
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let table = match conn.open_table(collection_name).execute().await { Ok(t) => t, Err(_) => return Ok(Vec::new()) };
        
        // Generate query embedding using LM Studio
        let query_embeddings = self.call_lmstudio_embedding(&[query.to_string()]).await?;
        let query_embedding = query_embeddings.into_iter().next().ok_or_else(|| anyhow!("Failed to get query embedding"))?;
        let query_vec: Vec<f64> = query_embedding.iter().map(|&f| f as f64).collect();
        
        // Perform vector search
        let mut stream = table.vector_search(query_vec)?
            .limit(top_k)
            .execute()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        
        let mut results = Vec::new();
        let mut rank = 0;
        while let Some(batch_res) = stream.next().await {
            let batch = batch_res.map_err(|e| anyhow!(e.to_string()))?;
            let schema = batch.schema();
            
            // Debug: log all column names to find the distance field
            if rank == 0 {
                let column_names: Vec<String> = schema.fields().iter().map(|f| f.name().to_string()).collect();
                info!("LanceDB schema columns: {:?}", column_names);
            }
            
            let id_idx = schema.index_of("id").ok();
            let source_id_idx = schema.index_of("source_id").ok();
            let definition_idx = schema.index_of("definition").ok();
            let file_name_idx = schema.index_of("file_name").ok();
            let file_path_idx = schema.index_of("file_path").ok();
            let chunk_index_idx = schema.index_of("chunk_index").ok();
            let start_char_idx = schema.index_of("start_char").ok();
            let end_char_idx = schema.index_of("end_char").ok();
            // Try different possible distance field names
            let distance_idx = schema.index_of("_distance").ok()
                .or_else(|| schema.index_of("distance").ok())
                .or_else(|| schema.index_of("score").ok());
            
            for row in 0..batch.num_rows() {
                let id = id_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string()).unwrap_or_default();
                let source_id = source_id_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string()).unwrap_or_default();
                let content = definition_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string()).unwrap_or_default();
                let file_name = file_name_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string()).unwrap_or_default();
                let file_path = file_path_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(row).to_string()).unwrap_or_default();
                let chunk_index = chunk_index_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<Int64Array>())
                    .map(|a| a.value(row) as usize).unwrap_or(0);
                let start_char = start_char_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<Int64Array>())
                    .map(|a| a.value(row) as usize).unwrap_or(0);
                let end_char = end_char_idx.and_then(|i| batch.column(i).as_any().downcast_ref::<Int64Array>())
                    .map(|a| a.value(row) as usize).unwrap_or(0);
                
                // Try to read distance as Float64 or Float32
                let distance = if let Some(idx) = distance_idx {
                    batch.column(idx).as_any().downcast_ref::<Float64Array>()
                        .map(|a| a.value(row) as f32)
                        .or_else(|| {
                            batch.column(idx).as_any().downcast_ref::<arrow_array::Float32Array>()
                                .map(|a| a.value(row))
                        })
                        .unwrap_or_else(|| {
                            warn!("Failed to read distance value at row {}, using default", row);
                            f32::MAX // Use large distance as fallback
                        })
                } else {
                    warn!("Distance column not found in schema, using default");
                    f32::MAX // Use large distance as fallback
                };
                
                // Convert distance to similarity score
                // For L2 distance: smaller is better, convert to similarity score [0, 1]
                let score = 1.0 / (1.0 + distance);
                
                if rank == 0 && row == 0 {
                    info!("First result: distance={}, score={}", distance, score);
                }
                
                let chunk = DocumentChunk {
                    id,
                    source_id,
                    content: content.clone(),
                    content_hash: format!("{:x}", md5::compute(content.as_bytes())),
                    chunk_index,
                    metadata: crate::models::ChunkMetadata {
                        file_path,
                        file_name,
                        file_type: "unknown".to_string(),
                        file_size: 0,
                        chunk_start_char: start_char,
                        chunk_end_char: end_char,
                        page_number: None,
                        section_title: None,
                        custom_fields: HashMap::new(),
                    },
                    embedding: None,
                    created_at: chrono::Utc::now(),
                };
                results.push(QueryResult { chunk, score, rank });
                rank += 1;
            }
        }
        info!("Found {} similar chunks in collection '{}' using LM Studio", results.len(), collection_name);
        Ok(results)
    }
    
    async fn search_similar_impl<M>(&self, collection_name: &str, query: &str, top_k: usize, embedding_model: M) -> Result<Vec<QueryResult>>
    where
        M: EmbeddingModel + Sync + Send,
    {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let table = match conn.open_table(collection_name).execute().await { Ok(t) => t, Err(_) => return Ok(Vec::new()) };
        
        // Retry index creation with better error handling
        let index = match LanceDbVectorIndex::new(table, embedding_model, "id", SearchParams::default()).await {
            Ok(idx) => idx,
            Err(e) => {
                error!("Failed to create LanceDB vector index: {}. Please check if embedding service ({}) is running", e, self.embedding_config.provider);
                return Err(anyhow!("Failed to create vector index: {}", e));
            }
        };
        
        // 构建VectorSearchRequest
        let search_request = VectorSearchRequest::builder()
            .query(query)
            .samples(top_k as u64)
            .build()?;
        let top_docs: Vec<(f64, String, serde_json::Value)> = <LanceDbVectorIndex<M> as VectorStoreIndex>::top_n::<serde_json::Value>(&index, search_request).await?;
        
        let mut results = Vec::new();
        for (rank, (score, _id, value)) in top_docs.into_iter().enumerate() {
            let id = value.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string();
            let source_id = value.get("source_id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string();
            let content = value.get("definition").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string();
            let file_name = value.get("file_name").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string();
            let file_path = value.get("file_path").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string();
            let chunk_index = value.get("chunk_index").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as usize;
            let start_char = value.get("start_char").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as usize;
            let end_char = value.get("end_char").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as usize;
            let chunk = DocumentChunk { id, source_id, content: content.clone(), content_hash: format!("{:x}", md5::compute(content.as_bytes())), chunk_index, metadata: crate::models::ChunkMetadata { file_path, file_name, file_type: "unknown".to_string(), file_size: 0, chunk_start_char: start_char, chunk_end_char: end_char, page_number: None, section_title: None, custom_fields: HashMap::new(), }, embedding: None, created_at: chrono::Utc::now(), };
            results.push(QueryResult { chunk, score: score as f32, rank });
        }
        info!("Found {} similar chunks in collection '{}'", results.len(), collection_name);
        Ok(results)
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let conn = { let guard = self.conn.read().await; guard.as_ref().cloned().ok_or_else(|| anyhow!("LanceDB not initialized"))? };
        let _ = conn.drop_table(collection_name, &[]).await; 
        info!("Deleted collection: {}", collection_name); 
        Ok(())
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

