use anyhow::{anyhow, Result};
use rig::client::{EmbeddingsClient, ProviderClient};
use rig::embeddings::embedding::{Embedding, EmbeddingModel};
use rig::vector_store::request::{SearchFilter, VectorSearchRequest};
use rig::vector_store::VectorStoreIndex;
use rig::OneOrMany;
use rig_sqlite::{
    Column, ColumnValue, SqliteSearchFilter, SqliteVectorStore, SqliteVectorStoreTable,
};
use rusqlite::ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Once;
use tokio::sync::RwLock;
use tokio_rusqlite::Connection;
use tracing::{error, info, warn};

use crate::config::EmbeddingConfig;
use crate::models::{DocumentChunk, QueryResult};

type SqliteExtensionFn =
    unsafe extern "C" fn(*mut sqlite3, *mut *mut i8, *const sqlite3_api_routines) -> i32;
static SQLITE_VEC_REGISTER: Once = Once::new();

type HttpClient = rig::http_client::ReqwestClient;
type OpenAiEmbedding = rig::providers::openai::EmbeddingModel<HttpClient>;
type OpenRouterEmbedding = rig::providers::openrouter::EmbeddingModel<HttpClient>;
type OllamaEmbedding = rig::providers::ollama::EmbeddingModel<HttpClient>;
type CohereEmbedding = rig::providers::cohere::EmbeddingModel<HttpClient>;
type GeminiEmbedding = rig::providers::gemini::EmbeddingModel<HttpClient>;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RagVectorRow {
    id: String,
    collection_name: String,
    source_id: String,
    chunk_index: String,
    definition: String,
}

impl SqliteVectorStoreTable for RagVectorRow {
    fn name() -> &'static str {
        "rag_vectors"
    }
    fn schema() -> Vec<Column> {
        vec![
            Column::new("id", "TEXT PRIMARY KEY"),
            Column::new("collection_name", "TEXT").indexed(),
            Column::new("source_id", "TEXT").indexed(),
            Column::new("chunk_index", "TEXT"),
            Column::new("definition", "TEXT"),
        ]
    }
    fn id(&self) -> String {
        self.id.clone()
    }
    fn column_values(&self) -> Vec<(&'static str, Box<dyn ColumnValue>)> {
        vec![
            ("id", Box::new(self.id.clone())),
            ("collection_name", Box::new(self.collection_name.clone())),
            ("source_id", Box::new(self.source_id.clone())),
            ("chunk_index", Box::new(self.chunk_index.clone())),
            ("definition", Box::new(self.definition.clone())),
        ]
    }
}

#[derive(Clone)]
enum ProviderStore {
    OpenAi(SqliteVectorStore<OpenAiEmbedding, RagVectorRow>),
    OpenRouter(SqliteVectorStore<OpenRouterEmbedding, RagVectorRow>),
    Ollama(SqliteVectorStore<OllamaEmbedding, RagVectorRow>),
    Cohere(SqliteVectorStore<CohereEmbedding, RagVectorRow>),
    Gemini(SqliteVectorStore<GeminiEmbedding, RagVectorRow>),
}

pub struct SqliteVectorManager {
    database_path: String,
    embedding_config: EmbeddingConfig,
    conn: RwLock<Option<Connection>>,
    store: RwLock<Option<ProviderStore>>,
}

impl SqliteVectorManager {
    pub fn new(database_path: String, embedding_config: EmbeddingConfig) -> Self {
        Self {
            database_path,
            embedding_config,
            conn: RwLock::new(None),
            store: RwLock::new(None),
        }
    }
    pub async fn initialize(&self) -> Result<()> {
        let db_path = Path::new(&self.database_path);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        register_sqlite_vec_extension();

        let conn = Connection::open(&self.database_path)
            .await
            .map_err(|e| anyhow!("Failed to open sqlite vector DB: {}", e))?;

        let mut guard = self.conn.write().await;
        *guard = Some(conn);

        info!("SQLite vector store connected at: {}", self.database_path);
        Ok(())
    }
    pub async fn create_collection(
        &self,
        _collection_name: &str,
        _embedding_dim: usize,
    ) -> Result<()> {
        // Collection rows are created on first insert.
        Ok(())
    }
    pub async fn insert_chunks(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let provider = self.embedding_config.provider.to_lowercase();
        info!(
            "Using embedding provider: {}, model: {}",
            provider, self.embedding_config.model
        );

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
            "lm studio" | "lmstudio" | "lm_studio" => {
                self.insert_chunks_lmstudio(collection_name, chunks).await
            }
            _ => Err(anyhow!(
                "Unsupported embedding provider: {}. Please check your RAG configuration.",
                provider
            )),
        }
    }
    async fn insert_chunks_ollama(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let base_url = self
            .embedding_config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");

        let client = rig::providers::ollama::Client::<HttpClient>::builder()
            .api_key(rig::client::Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;

        let dimensions = self.embedding_config.dimensions.unwrap_or(768);
        let embedding_model: OllamaEmbedding = rig::providers::ollama::EmbeddingModel::new(
            client,
            &self.embedding_config.model,
            dimensions,
        );

        let store = self.ensure_ollama_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_openai(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client =
            self.openai_compatible_client("OPENAI_API_KEY", "https://api.openai.com/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_cohere(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let api_key_str = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .ok_or_else(|| anyhow!("Cohere API key not configured"))?;

        std::env::set_var("COHERE_API_KEY", &api_key_str);
        let client = rig::providers::cohere::Client::from_env();
        let embedding_model: CohereEmbedding = match self.embedding_config.dimensions {
            Some(dim) if dim > 0 => client.embedding_model_with_ndims(
                &self.embedding_config.model,
                "search_document",
                dim,
            ),
            _ => client.embedding_model(&self.embedding_config.model, "search_document"),
        };

        let store = self.ensure_cohere_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_anthropic(
        &self,
        _collection_name: &str,
        _chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        warn!("Anthropic doesn't provide embedding models.");
        Err(anyhow!("Anthropic doesn't support embedding models. Please use OpenAI, Cohere, or other embedding providers."))
    }
    async fn insert_chunks_gemini(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let api_key_str = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| anyhow!("Google/Gemini API key not configured"))?;

        std::env::set_var("GEMINI_API_KEY", &api_key_str);
        let client = rig::providers::gemini::Client::from_env();
        let embedding_model: GeminiEmbedding = client.embedding_model(&self.embedding_config.model);

        let store = self.ensure_gemini_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_deepseek(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client =
            self.openai_compatible_client("DEEPSEEK_API_KEY", "https://api.deepseek.com/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_moonshot(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client =
            self.openai_compatible_client("MOONSHOT_API_KEY", "https://api.moonshot.cn/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_openrouter(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client = self.openrouter_client()?;
        let embedding_model = self.openrouter_embedding_model(&client)?;
        let store = self.ensure_openrouter_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_modelscope(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client = self.openai_compatible_client(
            "MODELSCOPE_API_KEY",
            "https://api-inference.modelscope.cn/v1",
        )?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_lmstudio(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
    ) -> Result<usize> {
        let client = self.openai_compatible_client("OPENAI_API_KEY", "http://localhost:1234/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.insert_chunks_impl(collection_name, chunks, embedding_model, store)
            .await
    }
    async fn insert_chunks_impl<M>(
        &self,
        collection_name: &str,
        chunks: Vec<DocumentChunk>,
        embedding_model: M,
        store: SqliteVectorStore<M, RagVectorRow>,
    ) -> Result<usize>
    where
        M: EmbeddingModel + Sync + Send + Clone + 'static,
    {
        let definitions: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        let mut retry_count = 0;
        let max_retries = 3;
        let embeddings = loop {
            match embedding_model.embed_texts(definitions.clone()).await {
                Ok(emb) => break emb,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!(
                            "Failed to generate embeddings after {} retries: {}",
                            max_retries, e
                        );
                        return Err(anyhow!(
                            "Failed to generate embeddings: {}. Please check if embedding service ({}) is running",
                            e,
                            self.embedding_config.provider
                        ));
                    }
                    let delay = std::time::Duration::from_secs(2u64.pow(retry_count));
                    warn!(
                        "Embedding request failed (attempt {}/{}): {}. Retrying in {:?}...",
                        retry_count, max_retries, e, delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        };

        if embeddings.len() != definitions.len() {
            return Err(anyhow!(
                "Embedding count mismatch: expected {}, got {}",
                definitions.len(),
                embeddings.len()
            ));
        }

        let docs = chunks_to_rows(collection_name, chunks)
            .into_iter()
            .zip(embeddings.into_iter())
            .map(|(row, embedding)| (row, OneOrMany::one(embedding)))
            .collect::<Vec<(RagVectorRow, OneOrMany<Embedding>)>>();

        store
            .add_rows(docs.clone())
            .await
            .map_err(|e| anyhow!("Failed to insert into sqlite vector store: {}", e))?;

        Ok(docs.len())
    }
    pub async fn search_similar(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        if top_k == 0 {
            return Ok(Vec::new());
        }

        let provider = self.embedding_config.provider.to_lowercase();

        match provider.as_str() {
            "ollama" => {
                self.search_similar_ollama(collection_name, query, top_k)
                    .await
            }
            "openai" => {
                self.search_similar_openai(collection_name, query, top_k)
                    .await
            }
            "cohere" => {
                self.search_similar_cohere(collection_name, query, top_k)
                    .await
            }
            "anthropic" => {
                self.search_similar_anthropic(collection_name, query, top_k)
                    .await
            }
            "gemini" | "google" => {
                self.search_similar_gemini(collection_name, query, top_k)
                    .await
            }
            "deepseek" => {
                self.search_similar_deepseek(collection_name, query, top_k)
                    .await
            }
            "moonshot" => {
                self.search_similar_moonshot(collection_name, query, top_k)
                    .await
            }
            "openrouter" => {
                self.search_similar_openrouter(collection_name, query, top_k)
                    .await
            }
            "modelscope" => {
                self.search_similar_modelscope(collection_name, query, top_k)
                    .await
            }
            "lm studio" | "lmstudio" | "lm_studio" => {
                self.search_similar_lmstudio(collection_name, query, top_k)
                    .await
            }
            _ => Err(anyhow!(
                "Unsupported embedding provider: {}. Please check your RAG configuration.",
                provider
            )),
        }
    }
    async fn search_similar_ollama(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let base_url = self
            .embedding_config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");

        let client = rig::providers::ollama::Client::<HttpClient>::builder()
            .api_key(rig::client::Nothing)
            .base_url(base_url)
            .build()
            .map_err(|e| anyhow!("Failed to create Ollama client: {}", e))?;

        let dimensions = self.embedding_config.dimensions.unwrap_or(768);
        let embedding_model: OllamaEmbedding = rig::providers::ollama::EmbeddingModel::new(
            client,
            &self.embedding_config.model,
            dimensions,
        );

        let store = self.ensure_ollama_store(&embedding_model).await?;
        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_openai(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client =
            self.openai_compatible_client("OPENAI_API_KEY", "https://api.openai.com/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;
        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_cohere(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let api_key_str = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .ok_or_else(|| anyhow!("Cohere API key not configured"))?;

        std::env::set_var("COHERE_API_KEY", &api_key_str);
        let client = rig::providers::cohere::Client::from_env();
        let embedding_model: CohereEmbedding = match self.embedding_config.dimensions {
            Some(dim) if dim > 0 => {
                client.embedding_model_with_ndims(&self.embedding_config.model, "search_query", dim)
            }
            _ => client.embedding_model(&self.embedding_config.model, "search_query"),
        };
        let store = self.ensure_cohere_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_anthropic(
        &self,
        _collection_name: &str,
        _query: &str,
        _top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        warn!("Anthropic doesn't provide embedding models.");
        Err(anyhow!("Anthropic doesn't support embedding models. Please use OpenAI, Cohere, or other embedding providers."))
    }
    async fn search_similar_gemini(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let api_key_str = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| anyhow!("Google/Gemini API key not configured"))?;

        std::env::set_var("GEMINI_API_KEY", &api_key_str);
        let client = rig::providers::gemini::Client::from_env();
        let embedding_model: GeminiEmbedding = client.embedding_model(&self.embedding_config.model);
        let store = self.ensure_gemini_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_deepseek(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client =
            self.openai_compatible_client("DEEPSEEK_API_KEY", "https://api.deepseek.com/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_moonshot(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client =
            self.openai_compatible_client("MOONSHOT_API_KEY", "https://api.moonshot.cn/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_openrouter(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client = self.openrouter_client()?;
        let embedding_model = self.openrouter_embedding_model(&client)?;
        let store = self.ensure_openrouter_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_modelscope(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client = self.openai_compatible_client(
            "MODELSCOPE_API_KEY",
            "https://api-inference.modelscope.cn/v1",
        )?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_lmstudio(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let client = self.openai_compatible_client("OPENAI_API_KEY", "http://localhost:1234/v1")?;
        let embedding_model = self.openai_embedding_model(&client)?;
        let store = self.ensure_openai_store(&embedding_model).await?;

        self.search_similar_impl(collection_name, query, top_k, embedding_model, store)
            .await
    }
    async fn search_similar_impl<M>(
        &self,
        collection_name: &str,
        query: &str,
        top_k: usize,
        embedding_model: M,
        store: SqliteVectorStore<M, RagVectorRow>,
    ) -> Result<Vec<QueryResult>>
    where
        M: EmbeddingModel + Sync + Send + Clone + 'static,
    {
        let index = store.index(embedding_model);

        let req = VectorSearchRequest::builder()
            .query(query)
            .samples(top_k as u64)
            .filter(SqliteSearchFilter::eq(
                "collection_name",
                collection_name.to_string().into(),
            ))
            .build()
            .map_err(|e| anyhow!("Failed to build vector search request: {}", e))?;

        let hits = index
            .top_n::<RagVectorRow>(req)
            .await
            .map_err(|e| anyhow!("Vector search failed: {}", e))?;

        let mut results = Vec::with_capacity(hits.len());
        for (rank, (score, _id, row)) in hits.into_iter().enumerate() {
            results.push(map_row_to_query_result(row, score, rank));
        }
        Ok(results)
    }
    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let conn = self.connection().await?;
        let collection_name = collection_name.to_string();
        let collection_name_for_log = collection_name.clone();

        conn.call(move |conn| {
            let table_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rag_vectors'",
                [],
                |row| row.get(0),
            )?;

            if table_exists == 0 {
                return Ok(());
            }

            let tx = conn.transaction()?;

            let embeddings_exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rag_vectors_embeddings'",
                [],
                |row| row.get(0),
            )?;

            if embeddings_exists > 0 {
                tx.execute(
                    "DELETE FROM rag_vectors_embeddings WHERE rowid IN (SELECT rowid FROM rag_vectors WHERE collection_name = ?1)",
                    rusqlite::params![collection_name],
                )?;
            }

            tx.execute(
                "DELETE FROM rag_vectors WHERE collection_name = ?1",
                rusqlite::params![collection_name],
            )?;

            tx.commit()?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Failed to delete collection: {}", e))?;

        info!("Deleted collection: {}", collection_name_for_log);
        Ok(())
    }
    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let conn = self.connection().await?;

        let names = conn
            .call(|conn| {
                let table_exists: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rag_vectors'",
                    [],
                    |row| row.get(0),
                )?;

                if table_exists == 0 {
                    return Ok(Vec::new());
                }

                let mut stmt = conn.prepare(
                    "SELECT DISTINCT collection_name FROM rag_vectors ORDER BY collection_name",
                )?;
                let rows = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
            .map_err(|e| anyhow!("Failed to list collections: {}", e))?;

        Ok(names)
    }
    pub async fn get_collection_stats(&self, collection_name: &str) -> Result<(usize, usize)> {
        let conn = self.connection().await?;
        let collection_name = collection_name.to_string();

        let (source_count, chunk_count) = conn
            .call(move |conn| {
                let table_exists: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rag_vectors'",
                    [],
                    |row| row.get(0),
                )?;

                if table_exists == 0 {
                    return Ok((0_i64, 0_i64));
                }

                let row = conn.query_row(
                    "SELECT COUNT(DISTINCT source_id), COUNT(*) FROM rag_vectors WHERE collection_name = ?1",
                    rusqlite::params![collection_name],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )?;

                Ok(row)
            })
            .await
            .map_err(|e| anyhow!("Failed to get collection stats: {}", e))?;

        Ok((source_count.max(0) as usize, chunk_count.max(0) as usize))
    }
    async fn connection(&self) -> Result<Connection> {
        let guard = self.conn.read().await;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("SQLite vector store not initialized"))
    }
    async fn rollback_stale_transaction(&self, conn: &Connection) -> Result<()> {
        let result = conn
            .call(|conn| match conn.execute_batch("ROLLBACK") {
                Ok(()) => Ok(true),
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("no transaction is active") {
                        Ok(false)
                    } else {
                        Err(tokio_rusqlite::Error::Rusqlite(e))
                    }
                }
            })
            .await
            .map_err(|e| anyhow!("Failed to reset sqlite transaction state: {}", e))?;

        if result {
            warn!("Rolled back stale sqlite transaction before vector store initialization");
        }
        Ok(())
    }
    async fn reopen_connection(&self) -> Result<Connection> {
        register_sqlite_vec_extension();
        let conn = Connection::open(&self.database_path)
            .await
            .map_err(|e| anyhow!("Failed to reopen sqlite vector DB: {}", e))?;

        let mut guard = self.conn.write().await;
        *guard = Some(conn.clone());
        Ok(conn)
    }
    fn is_nested_transaction_error(message: &str) -> bool {
        message
            .to_lowercase()
            .contains("cannot start a transaction within a transaction")
    }
    fn validate_nonzero_dimensions<M: EmbeddingModel>(&self, embedding_model: &M) -> Result<()> {
        if embedding_model.ndims() == 0 {
            return Err(anyhow!(
                "Embedding dimension resolved to 0 for provider '{}' model '{}'. Please set RAG embedding_dimensions to a positive value (LM Studio nomic-embed-text-v1.5 usually uses 768).",
                self.embedding_config.provider,
                self.embedding_config.model
            ));
        }
        Ok(())
    }
    async fn init_store_with_recovery<M>(
        &self,
        conn: &Connection,
        embedding_model: &M,
    ) -> Result<SqliteVectorStore<M, RagVectorRow>>
    where
        M: EmbeddingModel + Sync + Send + Clone + 'static,
    {
        self.validate_nonzero_dimensions(embedding_model)?;
        self.rollback_stale_transaction(conn).await?;

        match SqliteVectorStore::new(conn.clone(), embedding_model).await {
            Ok(store) => Ok(store),
            Err(e) => {
                let message = e.to_string();
                if Self::is_nested_transaction_error(&message) {
                    warn!("Detected nested sqlite transaction during vector store init, reopening connection and retrying once");
                    let reopened = self.reopen_connection().await?;
                    self.rollback_stale_transaction(&reopened).await?;
                    SqliteVectorStore::new(reopened, embedding_model)
                        .await
                        .map_err(|retry_err| {
                            anyhow!(
                                "Failed to initialize sqlite vector store after retry: {}",
                                retry_err
                            )
                        })
                } else {
                    Err(anyhow!(
                        "Failed to initialize sqlite vector store: {}",
                        message
                    ))
                }
            }
        }
    }
    async fn ensure_openai_store(
        &self,
        embedding_model: &OpenAiEmbedding,
    ) -> Result<SqliteVectorStore<OpenAiEmbedding, RagVectorRow>> {
        let conn = self.connection().await?;
        let mut guard = self.store.write().await;

        match guard.as_ref() {
            Some(ProviderStore::OpenAi(store)) => Ok(store.clone()),
            Some(_) => Err(anyhow!(
                "Vector store provider mismatch. Restart RAG service after provider change."
            )),
            None => {
                let store = self
                    .init_store_with_recovery(&conn, embedding_model)
                    .await?;
                *guard = Some(ProviderStore::OpenAi(store.clone()));
                Ok(store)
            }
        }
    }
    async fn ensure_ollama_store(
        &self,
        embedding_model: &OllamaEmbedding,
    ) -> Result<SqliteVectorStore<OllamaEmbedding, RagVectorRow>> {
        let conn = self.connection().await?;
        let mut guard = self.store.write().await;

        match guard.as_ref() {
            Some(ProviderStore::Ollama(store)) => Ok(store.clone()),
            Some(_) => Err(anyhow!(
                "Vector store provider mismatch. Restart RAG service after provider change."
            )),
            None => {
                let store = self
                    .init_store_with_recovery(&conn, embedding_model)
                    .await?;
                *guard = Some(ProviderStore::Ollama(store.clone()));
                Ok(store)
            }
        }
    }
    async fn ensure_openrouter_store(
        &self,
        embedding_model: &OpenRouterEmbedding,
    ) -> Result<SqliteVectorStore<OpenRouterEmbedding, RagVectorRow>> {
        let conn = self.connection().await?;
        let mut guard = self.store.write().await;

        match guard.as_ref() {
            Some(ProviderStore::OpenRouter(store)) => Ok(store.clone()),
            Some(_) => Err(anyhow!(
                "Vector store provider mismatch. Restart RAG service after provider change."
            )),
            None => {
                let store = self
                    .init_store_with_recovery(&conn, embedding_model)
                    .await?;
                *guard = Some(ProviderStore::OpenRouter(store.clone()));
                Ok(store)
            }
        }
    }
    async fn ensure_cohere_store(
        &self,
        embedding_model: &CohereEmbedding,
    ) -> Result<SqliteVectorStore<CohereEmbedding, RagVectorRow>> {
        let conn = self.connection().await?;
        let mut guard = self.store.write().await;

        match guard.as_ref() {
            Some(ProviderStore::Cohere(store)) => Ok(store.clone()),
            Some(_) => Err(anyhow!(
                "Vector store provider mismatch. Restart RAG service after provider change."
            )),
            None => {
                let store = self
                    .init_store_with_recovery(&conn, embedding_model)
                    .await?;
                *guard = Some(ProviderStore::Cohere(store.clone()));
                Ok(store)
            }
        }
    }
    async fn ensure_gemini_store(
        &self,
        embedding_model: &GeminiEmbedding,
    ) -> Result<SqliteVectorStore<GeminiEmbedding, RagVectorRow>> {
        let conn = self.connection().await?;
        let mut guard = self.store.write().await;

        match guard.as_ref() {
            Some(ProviderStore::Gemini(store)) => Ok(store.clone()),
            Some(_) => Err(anyhow!(
                "Vector store provider mismatch. Restart RAG service after provider change."
            )),
            None => {
                let store = self
                    .init_store_with_recovery(&conn, embedding_model)
                    .await?;
                *guard = Some(ProviderStore::Gemini(store.clone()));
                Ok(store)
            }
        }
    }
    fn openai_embedding_model(
        &self,
        client: &rig::providers::openai::Client,
    ) -> Result<OpenAiEmbedding> {
        let model = &self.embedding_config.model;
        let embedding_model = match self.embedding_config.dimensions {
            Some(dim) if dim > 0 => client.embedding_model_with_ndims(model, dim),
            _ => client.embedding_model(model),
        };
        self.validate_nonzero_dimensions(&embedding_model)?;
        Ok(embedding_model)
    }
    fn openai_compatible_client(
        &self,
        api_key_env: &str,
        default_base: &str,
    ) -> Result<rig::providers::openai::Client> {
        let api_key = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var(api_key_env).ok())
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("{} not configured", api_key_env))?;

        std::env::set_var("OPENAI_API_KEY", &api_key);

        let base = self
            .embedding_config
            .base_url
            .clone()
            .unwrap_or_else(|| default_base.to_string());
        std::env::set_var("OPENAI_BASE_URL", normalize_openai_base_url(&base));

        Ok(rig::providers::openai::Client::from_env())
    }
    fn openrouter_client(&self) -> Result<rig::providers::openrouter::Client<HttpClient>> {
        let api_key = self
            .embedding_config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| anyhow!("OPENROUTER_API_KEY not configured"))?;

        let mut builder = rig::providers::openrouter::Client::builder().api_key(api_key);
        if let Some(base_url) = self.embedding_config.base_url.clone() {
            builder = builder.base_url(normalize_openrouter_base_url(&base_url));
        }

        builder
            .build()
            .map_err(|e| anyhow!("Failed to create OpenRouter client: {}", e))
    }
    fn openrouter_embedding_model(
        &self,
        client: &rig::providers::openrouter::Client<HttpClient>,
    ) -> Result<OpenRouterEmbedding> {
        let model = &self.embedding_config.model;
        let embedding_model = match self.embedding_config.dimensions {
            Some(dim) if dim > 0 => client.embedding_model_with_ndims(model, dim),
            _ => client.embedding_model(model),
        };
        self.validate_nonzero_dimensions(&embedding_model)?;
        Ok(embedding_model)
    }
}
fn register_sqlite_vec_extension() {
    SQLITE_VEC_REGISTER.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<*const (), SqliteExtensionFn>(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}
fn normalize_openai_base_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}
fn normalize_openrouter_base_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/api/v1") || trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/v1")
    }
}
fn chunks_to_rows(collection_name: &str, chunks: Vec<DocumentChunk>) -> Vec<RagVectorRow> {
    chunks
        .into_iter()
        .map(|chunk| RagVectorRow {
            id: chunk.id,
            collection_name: collection_name.to_string(),
            source_id: chunk.source_id,
            chunk_index: chunk.chunk_index.to_string(),
            definition: chunk.content,
        })
        .collect()
}
fn map_row_to_query_result(row: RagVectorRow, score: f64, rank: usize) -> QueryResult {
    let content = row.definition.clone();
    let chunk_index = row.chunk_index.parse::<usize>().unwrap_or(0);

    QueryResult {
        chunk: DocumentChunk {
            id: row.id,
            source_id: row.source_id,
            content: content.clone(),
            content_hash: format!("{:x}", md5::compute(content.as_bytes())),
            chunk_index,
            metadata: crate::models::ChunkMetadata {
                file_path: String::new(),
                file_name: String::new(),
                file_type: "unknown".to_string(),
                file_size: 0,
                chunk_start_char: 0,
                chunk_end_char: 0,
                page_number: None,
                section_title: None,
                custom_fields: HashMap::new(),
            },
            embedding: None,
            created_at: chrono::Utc::now(),
        },
        score,
        rank,
    }
}
