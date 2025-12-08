use anyhow::{anyhow, Result};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::chunker::DocumentChunker;
use crate::config::{RagConfig, EmbeddingConfig};
use crate::database::LanceDbManager;
use crate::embeddings::{create_reranking_provider, RerankingManager};
use crate::models::{
    DocumentChunk, DocumentSource, IngestRequest, IngestResponse, QueryResult, RagQueryRequest,
    RagQueryResponse, RagStatus, CollectionInfo, IngestionStatus,
};
use crate::db::RagDatabase;

/// RAG服务主类 - 使用 Rig + LanceDB
pub struct RagService<D: RagDatabase> {
    _config: RagConfig,
    database: Arc<D>,
    vector_store: Arc<LanceDbManager>,
    chunker: DocumentChunker,
    ingestion_status: RwLock<HashMap<String, IngestionStatus>>,
    reranker: Option<RerankingManager>,
}

impl<D: RagDatabase> RagService<D> {
    /// 创建新的RAG服务实例
    pub async fn new(config: RagConfig, database: Arc<D>) -> Result<Self> {
        info!("初始化RAG服务 (使用 Rig + LanceDB)");
        
        let chunker = DocumentChunker::new(config.clone());
        
        // Initialize LanceDB vector store
        let db_path = config.database_path
            .as_ref()
            .map(|p| {
                // 如果是相对路径，转换为应用数据目录下的绝对路径
                if p.is_relative() {
                    let app_data_dir = dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("sentinel-ai");
                    app_data_dir.join(p).to_string_lossy().to_string()
                } else {
                    p.to_string_lossy().to_string()
                }
            })
            .unwrap_or_else(|| {
                // 默认路径也使用应用数据目录
                let app_data_dir = dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("sentinel-ai");
                app_data_dir.join("lancedb").to_string_lossy().to_string()
            });
        
        // Build embedding config for vector store
        let embedding_config = EmbeddingConfig {
            provider: config.embedding_provider.clone(),
            model: config.embedding_model.clone(),
            api_key: config.embedding_api_key.clone(),
            base_url: config.embedding_base_url.clone(),
            dimensions: config.embedding_dimensions,
        };

        info!("RAG服务使用LanceDB路径: {}", db_path);
        
        let vector_store = Arc::new(LanceDbManager::new(db_path, embedding_config));
        vector_store.initialize().await?;
        
        let mut reranker: Option<RerankingManager> = None;
        // Initialize reranking provider if enabled in config
        if config.reranking_enabled {
            match (config.reranking_provider.clone(), config.reranking_model.clone()) {
                (Some(provider), Some(model)) => {
                    match create_reranking_provider(
                        provider.as_str(),
                        model.as_str(),
                        // Reuse embedding base_url/api_key if dedicated reranking config is not available
                        config.embedding_base_url.clone(),
                        config.embedding_api_key.clone(),
                    ) {
                        Ok(provider_impl) => {
                            let mut mgr = RerankingManager::new();
                            mgr.register_provider(provider_impl);
                            reranker = Some(mgr);
                            info!("Reranking enabled with provider: {} and model: {}", provider, model);
                        }
                        Err(e) => {
                            warn!("Failed to initialize reranking provider: {}", e);
                        }
                    }
                }
                _ => {
                    info!("Reranking enabled in config but provider/model not configured; skipping reranker init");
                }
            }
        }

        let service = Self {
            _config: config,
            database,
            vector_store,
            chunker,
            ingestion_status: RwLock::new(HashMap::new()),
            reranker,
        };
        
        // 确保默认集合存在
        service.ensure_default_collection().await?;
        
        Ok(service)
    }

    /// 确保默认集合存在
    async fn ensure_default_collection(&self) -> Result<()> {
        const DEFAULT_COLLECTION_NAME: &str = "default";
        
        match self.database.get_rag_collection_by_name(DEFAULT_COLLECTION_NAME).await? {
            Some(_) => {
                info!("默认RAG集合已存在");
            }
            None => {
                info!("创建默认RAG集合...");
                self.database.create_rag_collection(
                    DEFAULT_COLLECTION_NAME,
                    Some("系统默认知识库集合，用于存储通用文档和知识")
                ).await?;
                info!("默认RAG集合创建完成");
            }
        }
        
        // Create vector collection
        self.vector_store.create_collection(DEFAULT_COLLECTION_NAME, 768).await?;
        Ok(())
    }

    /// 创建RAG集合
    pub async fn create_collection(&self, name: &str, description: Option<&str>, _embedding_model: &str) -> Result<String> {
        info!("创建RAG集合: {}", name);
        
        // Create in SQL database for metadata
        let collection_id = self.database.create_rag_collection(name, description).await?;
        
        // Create in vector store
        self.vector_store.create_collection(name, 768).await?;
        
        Ok(collection_id)
    }

    /// 获取所有RAG集合
    pub async fn get_collections(&self) -> Result<Vec<CollectionInfo>> {
        info!("获取RAG集合列表");
        self.database.get_rag_collections().await
    }

    /// 删除RAG集合
    pub async fn delete_collection(&self, collection_id: &str) -> Result<()> {
        info!("删除RAG集合: {}", collection_id);
        
        // Get collection name first
        if let Some(collection) = self.database.get_rag_collection_by_id(collection_id).await? {
            // Delete from vector store
            self.vector_store.delete_collection(&collection.name).await?;
        }
        
        // Delete from SQL database
        self.database.delete_rag_collection(collection_id).await
    }

    /// 摄取文档源
    pub async fn ingest_source(&self, request: IngestRequest) -> Result<IngestResponse> {
        let task_id = Uuid::new_v4().to_string();
        info!("开始摄取任务: {} - {}", task_id, request.file_path);
        
        // 创建摄取状态
        let mut ingestion_status = IngestionStatus {
            task_id: task_id.clone(),
            source_path: request.file_path.clone(),
            status: "processing".to_string(),
            progress: 0.0,
            total_chunks: 0,
            processed_chunks: 0,
            error_message: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
        };
        
        // 存储状态
        self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status.clone());
        
        // 获取或创建集合
        let collection_id = if let Some(cid) = request.collection_id.or(Some("".to_string())) {
            // 验证集合是否存在
            if self.database.get_rag_collection_by_id(&cid).await?.is_some() {
                cid
            } else {
                return Err(anyhow::anyhow!("Collection with id {} not found", cid));
            }
        } else {
            // 如果没有指定collection_id，使用default集合
            let default_name = "default";
            match self.database.get_rag_collection_by_name(default_name).await? {
                Some(collection) => collection.id,
                None => {
                    // 创建默认集合
                    self.database.create_rag_collection(
                        default_name,
                        Some("默认集合"),
                    ).await?
                }
            }
        };

        // 分块处理
        let (document_source, chunks) = match self.chunker.process_document(&request.file_path).await {
            Ok((source, chunks)) => (source, chunks),
            Err(e) => {
                error!("文档分块处理失败: {}", e);
                ingestion_status.status = "failed".to_string();
                ingestion_status.error_message = Some(format!("文档分块处理失败: {}", e));
                ingestion_status.completed_at = Some(chrono::Utc::now());
                self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status);
                return Err(anyhow!("文档分块处理失败: {}", e));
            }
        };
        ingestion_status.total_chunks = chunks.len();
        
        // 创建文档记录
        let document_id = match self.database.create_rag_document(
            &collection_id,
            &document_source.file_path,
            &document_source.file_name,
            "", // 内容在chunks中
            request.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()).as_deref().unwrap_or(""),
        ).await {
            Ok(id) => id,
            Err(e) => {
                error!("创建文档记录失败: {}", e);
                ingestion_status.status = "failed".to_string();
                ingestion_status.error_message = Some(format!("创建文档记录失败: {}", e));
                ingestion_status.completed_at = Some(chrono::Utc::now());
                self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status);
                return Err(anyhow!("创建文档记录失败: {}", e));
            }
        };
        
        // Get collection name for vector store
        let collection_name = if let Some(collection) = self.database.get_rag_collection_by_id(&collection_id).await? {
            collection.name
        } else {
            "default".to_string()
        };
        
        // Insert into vector store using Rig + LanceDB
        let chunks_created = match self.vector_store.insert_chunks(&collection_name, chunks.clone()).await {
            Ok(count) => count,
            Err(e) => {
                error!("向量存储插入失败: {}", e);
                ingestion_status.status = "failed".to_string();
                ingestion_status.error_message = Some(format!("向量存储插入失败: {}", e));
                ingestion_status.completed_at = Some(chrono::Utc::now());
                self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status);
                return Err(anyhow!("向量存储插入失败: {}", e));
            }
        };

        // Also save chunks to SQL database for metadata
        for (index, chunk) in chunks.iter().enumerate() {
            let metadata_json = serde_json::to_string(&chunk.metadata).unwrap_or("{}".to_string());
            
            if let Err(e) = self.database.create_rag_chunk(
                &document_id,
                &collection_id,
                &chunk.content,
                index as i32,
                None, // No embedding in SQL - stored in LanceDB
                &self._config.embedding_model,
                self._config.embedding_dimensions.unwrap_or(768) as i32,
                &metadata_json,
            ).await {
                warn!("创建chunk记录失败: {}", e);
            }
        }

        // 完成摄取，更新集合统计
        if let Err(e) = self.database.update_collection_stats(&collection_id).await {
            warn!("更新集合统计失败: {}", e);
        }

        ingestion_status.status = "completed".to_string();
        ingestion_status.completed_at = Some(chrono::Utc::now());
        ingestion_status.processed_chunks = chunks.len();
        ingestion_status.progress = 100.0;
        self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status.clone());

        let processing_time = ingestion_status.completed_at.unwrap()
            .signed_duration_since(ingestion_status.started_at)
            .num_milliseconds() as u64;

        Ok(IngestResponse {
            source_id: document_id,
            chunks_created,
            processing_time_ms: processing_time,
            status: ingestion_status,
        })
    }

    /// 摄取手动输入的文本
    pub async fn ingest_text(
        &self,
        title: &str,
        content: &str,
        collection_id: Option<&str>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<IngestResponse> {
        let task_id = Uuid::new_v4().to_string();
        info!("开始摄取手动输入文本: {} - {}", task_id, title);

        // 创建摄取状态
        let mut ingestion_status = IngestionStatus {
            task_id: task_id.clone(),
            source_path: format!("manual://{}", title),
            status: "processing".to_string(),
            progress: 0.0,
            total_chunks: 0,
            processed_chunks: 0,
            error_message: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        // 存储状态
        self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status.clone());

        // 获取或验证集合
        let target_collection_id = if let Some(cid) = collection_id {
            if self.database.get_rag_collection_by_id(cid).await?.is_some() {
                cid.to_string()
            } else {
                return Err(anyhow::anyhow!("Collection with id {} not found", cid));
            }
        } else {
            // 使用默认集合
            let default_name = "default";
            match self.database.get_rag_collection_by_name(default_name).await? {
                Some(collection) => collection.id,
                None => {
                    self.database.create_rag_collection(default_name, Some("默认集合")).await?
                }
            }
        };

        // 直接对文本进行分块
        let chunks = self.chunk_raw_text(content, title)?;
        ingestion_status.total_chunks = chunks.len();

        // 创建文档记录
        let document_id = match self.database.create_rag_document(
            &target_collection_id,
            &format!("manual://{}", title),
            title,
            content, // 传入实际内容用于计算大小
            metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()).as_deref().unwrap_or(""),
        ).await {
            Ok(id) => id,
            Err(e) => {
                error!("创建文档记录失败: {}", e);
                ingestion_status.status = "failed".to_string();
                ingestion_status.error_message = Some(format!("创建文档记录失败: {}", e));
                ingestion_status.completed_at = Some(chrono::Utc::now());
                self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status);
                return Err(anyhow!("创建文档记录失败: {}", e));
            }
        };

        // 获取集合名称用于向量存储
        let collection_name = if let Some(collection) = self.database.get_rag_collection_by_id(&target_collection_id).await? {
            collection.name
        } else {
            "default".to_string()
        };

        // 插入向量存储
        let chunks_created = match self.vector_store.insert_chunks(&collection_name, chunks.clone()).await {
            Ok(count) => count,
            Err(e) => {
                error!("向量存储插入失败: {}", e);
                ingestion_status.status = "failed".to_string();
                ingestion_status.error_message = Some(format!("向量存储插入失败: {}", e));
                ingestion_status.completed_at = Some(chrono::Utc::now());
                self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status);
                return Err(anyhow!("向量存储插入失败: {}", e));
            }
        };

        // 保存chunks到SQL数据库
        for (index, chunk) in chunks.iter().enumerate() {
            let metadata_json = serde_json::to_string(&chunk.metadata).unwrap_or("{}".to_string());
            
            if let Err(e) = self.database.create_rag_chunk(
                &document_id,
                &target_collection_id,
                &chunk.content,
                index as i32,
                None,
                &self._config.embedding_model,
                self._config.embedding_dimensions.unwrap_or(768) as i32,
                &metadata_json,
            ).await {
                warn!("创建chunk记录失败: {}", e);
            }
        }

        // 更新集合统计
        if let Err(e) = self.database.update_collection_stats(&target_collection_id).await {
            warn!("更新集合统计失败: {}", e);
        }

        ingestion_status.status = "completed".to_string();
        ingestion_status.completed_at = Some(chrono::Utc::now());
        ingestion_status.processed_chunks = chunks.len();
        ingestion_status.progress = 100.0;
        self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status.clone());

        let processing_time = ingestion_status.completed_at.unwrap()
            .signed_duration_since(ingestion_status.started_at)
            .num_milliseconds() as u64;

        Ok(IngestResponse {
            source_id: document_id,
            chunks_created,
            processing_time_ms: processing_time,
            status: ingestion_status,
        })
    }

    /// 对原始文本进行分块
    fn chunk_raw_text(&self, content: &str, title: &str) -> Result<Vec<DocumentChunk>> {
        use crate::models::ChunkMetadata;

        let mut chunks = Vec::new();
        let chunk_size = self._config.chunk_size_chars;
        let overlap = self._config.chunk_overlap_chars;

        let chars: Vec<char> = content.chars().collect();
        let total_len = chars.len();

        if total_len == 0 {
            return Ok(chunks);
        }

        // 如果内容短于chunk_size，作为单个chunk
        if total_len <= chunk_size {
            let chunk = DocumentChunk {
                id: Uuid::new_v4().to_string(),
                source_id: String::new(), // 会在外部设置
                content: content.to_string(),
                content_hash: format!("{:x}", md5::compute(content.as_bytes())),
                chunk_index: 0,
                metadata: ChunkMetadata {
                    file_path: format!("manual://{}", title),
                    file_name: title.to_string(),
                    file_type: "text".to_string(),
                    file_size: content.len() as u64,
                    chunk_start_char: 0,
                    chunk_end_char: total_len,
                    page_number: None,
                    section_title: Some(title.to_string()),
                    custom_fields: HashMap::new(),
                },
                embedding: None,
                created_at: chrono::Utc::now(),
            };
            chunks.push(chunk);
            return Ok(chunks);
        }

        // 分块处理
        let mut chunk_index = 0;
        let mut start = 0;

        while start < total_len {
            let end = std::cmp::min(start + chunk_size, total_len);
            let chunk_content: String = chars[start..end].iter().collect();

            let chunk = DocumentChunk {
                id: Uuid::new_v4().to_string(),
                source_id: String::new(),
                content: chunk_content.clone(),
                content_hash: format!("{:x}", md5::compute(chunk_content.as_bytes())),
                chunk_index,
                metadata: ChunkMetadata {
                    file_path: format!("manual://{}", title),
                    file_name: title.to_string(),
                    file_type: "text".to_string(),
                    file_size: content.len() as u64,
                    chunk_start_char: start,
                    chunk_end_char: end,
                    page_number: None,
                    section_title: Some(title.to_string()),
                    custom_fields: HashMap::new(),
                },
                embedding: None,
                created_at: chrono::Utc::now(),
            };
            chunks.push(chunk);
            chunk_index += 1;

            if end >= total_len {
                break;
            }
            start = end.saturating_sub(overlap);
            if start >= end {
                break;
            }
        }

        Ok(chunks)
    }

    /// 查询相似文档 - 使用 Rig + LanceDB
    pub async fn query(&self, request: RagQueryRequest) -> Result<RagQueryResponse> {
        info!("执行RAG查询 (使用 Rig + LanceDB): {}", request.query);
        
        let start_time = std::time::Instant::now();
        
        // 获取集合名称
        let collection_name = if let Some(cid) = request.collection_id {
            if cid.is_empty() {
                "default".to_string()
            } else {
                // Get collection name from ID
                if let Some(collection) = self.database.get_rag_collection_by_id(&cid).await? {
                    collection.name
                } else {
                    return Err(anyhow!("Collection not found"));
                }
            }
        } else {
            "default".to_string()
        };
        
        let top_k = request.top_k.unwrap_or(5);
        
        // 使用 Rig + LanceDB 进行语义搜索
        let query_results = self.vector_store.search_similar(&collection_name, &request.query, top_k).await?;
        
        // 构建上下文
        let context_parts: Vec<String> = query_results.iter().map(|r| r.chunk.content.clone()).collect();
        let context = context_parts.join("\n\n");
        let processing_time = start_time.elapsed().as_millis() as u64;

        // 保存查询历史
        if let Some(collection) = self.database.get_rag_collection_by_name(&collection_name).await? {
            if let Err(e) = self.database.save_rag_query(
                Some(&collection.id),
                &request.query,
                &context,
                processing_time,
            ).await {
                warn!("保存查询历史失败: {}", e);
            }
        }

        Ok(RagQueryResponse {
            query: request.query,
            results: query_results,
            context: context.clone(),
            total_tokens: context.len(), // 简化的token计算
            processing_time_ms: processing_time,
        })
    }

    /// 为AI助手查询RAG，返回格式化的上下文和引用
    pub async fn query_for_assistant(&self, request: &crate::models::AssistantRagRequest) -> Result<(String, Vec<crate::models::Citation>)> {
        use crate::models::Citation;
        
        let start_time = std::time::Instant::now();
        

        // 获取集合名称
        let collection_name = if let Some(cid) = &request.collection_id {
            if cid.is_empty() {
                "default".to_string()
            } else {
                if let Some(collection) = self.database.get_rag_collection_by_id(cid).await? {
                    collection.name
                } else {
                    "default".to_string()
                }
            }
        } else {
            "default".to_string()
        };

        let top_k = request.top_k.unwrap_or(5);
        
        // 使用 Rig + LanceDB 进行语义搜索
        let mut results: Vec<crate::models::QueryResult> = match self.vector_store.search_similar(&collection_name, &request.query, top_k).await {
            Ok(results) => results,
            Err(e) => {
                warn!("向量搜索失败: {}, 返回空结果", e);
                return Ok((String::new(), vec![]));
            }
        };

        // Optional reranking step
        let rerank_enabled_req = request.reranking_enabled.unwrap_or(false);
        let rerank_enabled_cfg = self._config.reranking_enabled;
        let should_rerank = (rerank_enabled_req || rerank_enabled_cfg)
            && self.reranker.as_ref().map(|r| r.is_enabled()).unwrap_or(false);

        if should_rerank {
            let documents: Vec<String> = results.iter().map(|r| r.chunk.content.clone()).collect();
            if let Some(reranker) = &self.reranker {
                match reranker.rerank(&request.query, &documents, Some(top_k)).await {
                    Ok(reranked) => {
                        let mut reordered: Vec<crate::models::QueryResult> = Vec::new();
                        for rr in reranked.into_iter().take(top_k) {
                            if let Some(item) = results.get(rr.index) {
                                let mut item_clone = item.clone();
                                // overwrite score with rerank score to reflect ordering rationale
                                item_clone.score = rr.score;
                                reordered.push(item_clone);
                            }
                        }
                        if !reordered.is_empty() {
                            info!("Applied reranking to {} results", reordered.len());
                            results = reordered;
                        }
                    }
                    Err(e) => {
                        warn!("Reranking failed: {}. Falling back to vector search order.", e);
                    }
                }
            }
        }


        if results.is_empty() {
            info!("未找到相关文档，返回空结果");
            return Ok((String::new(), vec![]));
        }

        // 构建Evidence Blocks格式的上下文
        let mut evidence_blocks = Vec::new();
        let mut citations = Vec::new();
        
        for (idx, item) in results.iter().enumerate() {
            let chunk = &item.chunk;
            let source_num = idx + 1;
            let file_name = chunk.metadata.file_name.clone();
            let content_preview = chunk.content.clone();
            
            // 格式化Evidence Block
            let evidence_block = format!(
                "=== SOURCE {} | {} | page: {} | score: {:.2} ===\n{}",
                source_num,
                file_name,
                chunk.metadata.page_number.unwrap_or(1),
                item.score,
                chunk.content
            );
            evidence_blocks.push(evidence_block);
            
            // 构建Citation
            let citation = Citation {
                id: chunk.id.clone(),
                source_id: chunk.source_id.clone(),
                file_name: file_name.clone(),
                file_path: Some(chunk.metadata.file_path.clone()),
                page_number: chunk.metadata.page_number.map(|p| p as i32),
                section_title: chunk.metadata.section_title.clone(),
                start_char: chunk.metadata.chunk_start_char,
                end_char: chunk.metadata.chunk_end_char,
                score: item.score,
                content_preview,
            };
            citations.push(citation);
        }
        
        let context = evidence_blocks.join("\n\n");
        
        info!("为AI助手准备了 {} 个证据块，处理时间: {:?}", 
              results.len(), start_time.elapsed());
        
        Ok((context, citations))
    }

    /// 获取文档列表
    pub async fn get_documents(&self, collection_id: &str) -> Result<Vec<DocumentSource>> {
        info!("获取文档列表: {}", collection_id);
        self.database.get_rag_documents(collection_id).await
    }

    /// 删除文档
    pub async fn delete_document(&self, document_id: &str) -> Result<()> {
        info!("删除文档: {}", document_id);
        self.database.delete_rag_document(document_id).await
    }

    /// 获取文档chunks
    pub async fn get_document_chunks(&self, document_id: &str) -> Result<Vec<DocumentChunk>> {
        info!("获取文档chunks: {}", document_id);
        self.database.get_rag_chunks(document_id).await
    }

    /// 获取查询历史
    pub async fn get_query_history(&self, collection_id: &str, limit: Option<i32>) -> Result<Vec<QueryResult>> {
        info!("获取查询历史: {}", collection_id);
        self.database.get_rag_query_history(Some(collection_id), limit).await
    }

    /// 获取摄取状态
    pub async fn get_ingestion_status(&self, task_id: &str) -> Result<Option<IngestionStatus>> {
        let status_map = self.ingestion_status.read().await;
        Ok(status_map.get(task_id).cloned())
    }

    /// 清空集合
    pub async fn clear_collection(&self, collection_id: &str) -> Result<()> {
        info!("清空集合: {}", collection_id);
        
        // Get collection name
        if let Some(collection) = self.database.get_rag_collection_by_id(collection_id).await? {
            // Clear vector store
            self.vector_store.delete_collection(&collection.name).await?;
            // Recreate empty collection
            self.vector_store.create_collection(&collection.name, 768).await?;
        }
        
        // Clear SQL database
        self.database.delete_rag_collection(collection_id).await
    }

    /// 获取当前配置
    pub fn get_config(&self) -> &RagConfig {
        &self._config
    }

    /// 确保默认集合存在（公开方法供命令调用）
    pub async fn ensure_default_collection_public(&self) -> Result<String> {
        const DEFAULT_COLLECTION_NAME: &str = "default";
        
        match self.database.get_rag_collection_by_name(DEFAULT_COLLECTION_NAME).await? {
            Some(collection) => {
                info!("默认RAG集合已存在: {}", collection.id);
                Ok(collection.id)
            }
            None => {
                info!("创建默认RAG集合...");
                let collection_id = self.database.create_rag_collection(
                    DEFAULT_COLLECTION_NAME,
                    Some("系统默认知识库集合，用于存储通用文档和知识")
                ).await?;
                
                // Create vector collection
                self.vector_store.create_collection(DEFAULT_COLLECTION_NAME, 768).await?;
                
                info!("默认RAG集合创建完成: {}", collection_id);
                Ok(collection_id)
            }
        }
    }

    /// 获取RAG状态
    pub async fn get_status(&self) -> Result<RagStatus> {
        info!("获取RAG状态");
        
        let collections = self.database.get_rag_collections().await?;
        let total_documents: usize = collections.iter().map(|c| c.document_count as usize).sum();
        let total_chunks: usize = collections.iter().map(|c| c.chunk_count as usize).sum();
        
        // Already CollectionInfo from database
        let collection_infos: Vec<CollectionInfo> = collections;
        
        Ok(RagStatus {
            collections: collection_infos,
            total_documents,
            total_chunks,
            database_size_mb: 0.0, // 简化实现
        })
    }
}

// Tests omitted in crate to avoid cross-crate dependencies