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
    pub async fn create_collection(&self, name: &str, description: Option<&str>) -> Result<String> {
        info!("创建RAG集合: {}", name);
        
        // Create in SQL database for metadata
        let collection_id = self.database.create_rag_collection(name, description).await?;
        
        // Create in vector store
        let dimensions = self._config.embedding_dimensions.unwrap_or(768);
        self.vector_store.create_collection(name, dimensions).await?;
        
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

    /// 检测文档是否为POC类型（需要完整性保证）
    fn is_poc_document(&self, file_path: &str, content: &str) -> bool {
        // 检查文件名
        let file_name_lower = file_path.to_lowercase();
        if file_name_lower.contains("poc") 
            || file_name_lower.contains("exploit") 
            || file_name_lower.contains("payload") {
            return true;
        }
        
        // 检查内容关键词
        let content_lower = content.to_lowercase();
        let poc_keywords = [
            "poc", "exploit", "payload", "vulnerability", "cve-",
            "proof of concept", "security test", "penetration test",
            "漏洞验证", "漏洞利用", "安全测试", "渗透测试"
        ];
        
        let keyword_count = poc_keywords.iter()
            .filter(|&keyword| content_lower.contains(keyword))
            .count();
        
        // 如果包含3个或以上POC相关关键词，认为是POC文档
        keyword_count >= 3
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
                        Some("默认集合")
                    ).await?
                }
            }
        };

        // 分块处理
        let (document_source, mut chunks) = match self.chunker.process_document(&request.file_path).await {
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
        
        // 检测是否为POC文档
        let full_content: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n");
        let is_poc = self.is_poc_document(&request.file_path, &full_content);
        
        if is_poc {
            info!("检测到POC文档，将在metadata中标记");
            // 为所有chunks添加POC标记
            for chunk in &mut chunks {
                chunk.metadata.custom_fields.insert("document_type".to_string(), "poc".to_string());
                chunk.metadata.custom_fields.insert("requires_completeness".to_string(), "true".to_string());
            }
        }
        
        // 准备metadata JSON
        let mut metadata_map = request.metadata.unwrap_or_default();
        if is_poc {
            metadata_map.insert("document_type".to_string(), "poc".to_string());
            metadata_map.insert("requires_completeness".to_string(), "true".to_string());
        }
        let metadata_json = serde_json::to_string(&metadata_map).unwrap_or_default();
        
        // 创建文档记录
        let document_id = match self.database.create_rag_document(
            &collection_id,
            &document_source.file_path,
            &document_source.file_name,
            "", // 内容在chunks中
            &metadata_json,
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

        // 关键修复：确保所有chunks使用正确的document_id (source_id)
        // 这样在检索时才能正确关联到SQL数据库中的记录
        for chunk in &mut chunks {
            chunk.source_id = document_id.clone();
        }
        
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
                
                // Rollback: delete the document from SQL database
                if let Err(del_err) = self.database.delete_rag_document(&document_id).await {
                    error!("Failed to rollback (delete) document {} after vector insertion failure: {}", document_id, del_err);
                } else {
                    info!("Rolled back document {} due to vector insertion failure", document_id);
                }

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
        let mut chunks = self.chunk_raw_text(content, title)?;
        ingestion_status.total_chunks = chunks.len();

        // 检测是否为POC文档
        let is_poc = self.is_poc_document(title, content);
        
        if is_poc {
            info!("检测到POC文本，将在metadata中标记");
            // 为所有chunks添加POC标记
            for chunk in &mut chunks {
                chunk.metadata.custom_fields.insert("document_type".to_string(), "poc".to_string());
                chunk.metadata.custom_fields.insert("requires_completeness".to_string(), "true".to_string());
            }
        }
        
        // 准备metadata JSON
        let mut metadata_map = metadata.unwrap_or_default();
        if is_poc {
            metadata_map.insert("document_type".to_string(), "poc".to_string());
            metadata_map.insert("requires_completeness".to_string(), "true".to_string());
        }
        let metadata_json = serde_json::to_string(&metadata_map).unwrap_or_default();

        // 创建文档记录
        let document_id = match self.database.create_rag_document(
            &target_collection_id,
            &format!("manual://{}", title),
            title,
            content, // 传入实际内容用于计算大小
            &metadata_json,
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

        // 关键修复：确保所有chunks使用正确的document_id (source_id)
        // 这样在检索时才能正确关联到SQL数据库中的记录
        for chunk in &mut chunks {
            chunk.source_id = document_id.clone();
        }

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

                // Rollback: delete the document from SQL database
                if let Err(del_err) = self.database.delete_rag_document(&document_id).await {
                    error!("Failed to rollback (delete) document {} after vector insertion failure: {}", document_id, del_err);
                } else {
                    info!("Rolled back document {} due to vector insertion failure", document_id);
                }

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

    /// 扩展chunk上下文：获取相邻的chunks并合并
    async fn expand_chunk_context(&self, results: Vec<QueryResult>) -> Result<Vec<QueryResult>> {
        if !self._config.chunk_expansion_enabled || results.is_empty() {
            return Ok(results);
        }

        let mut before = self._config.chunk_expansion_before;
        let mut after = self._config.chunk_expansion_after;
        
        if before == 0 && after == 0 {
            return Ok(results);
        }

        info!("Expanding chunk context: before={}, after={}", before, after);
        
        let mut expanded_results = Vec::new();
        
        for result in results {
            let chunk = &result.chunk;
            let document_id = &chunk.source_id;
            let current_index = chunk.chunk_index;
            
            // 获取该文档的所有chunks
            let all_chunks = match self.database.get_rag_chunks(document_id).await {
                Ok(chunks) => chunks,
                Err(e) => {
                    warn!("Failed to get chunks for document {}: {}", document_id, e);
                    expanded_results.push(result);
                    continue;
                }
            };
            
            if all_chunks.is_empty() {
                expanded_results.push(result);
                continue;
            }
            
            // 检查是否为POC文档，如果是则扩大扩展范围
            let is_poc = chunk.metadata.custom_fields.get("document_type")
                .map(|v| v == "poc")
                .unwrap_or(false);
            
            if is_poc {
                // POC文档需要更大的上下文，扩展到前后各2个chunk或整个文档
                before = std::cmp::max(before, 2);
                after = std::cmp::max(after, 2);
                info!("POC document detected, expanding context to before={}, after={}", before, after);
            }
            
            // 计算需要包含的chunk索引范围
            let start_index = current_index.saturating_sub(before);
            let end_index = std::cmp::min(current_index + after + 1, all_chunks.len());
            
            // 收集相关chunks并按索引排序
            let mut relevant_chunks: Vec<&DocumentChunk> = all_chunks
                .iter()
                .filter(|c| c.chunk_index >= start_index && c.chunk_index < end_index)
                .collect();
            relevant_chunks.sort_by_key(|c| c.chunk_index);
            
            if relevant_chunks.is_empty() {
                expanded_results.push(result);
                continue;
            }
            
            // 合并chunks内容
            let expanded_content = relevant_chunks
                .iter()
                .map(|c| c.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            
            // 创建扩展后的chunk
            let mut expanded_chunk = chunk.clone();
            expanded_chunk.content = expanded_content;
            expanded_chunk.content_hash = format!("{:x}", md5::compute(expanded_chunk.content.as_bytes()));
            
            // 更新metadata以反映扩展范围
            if let Some(first) = relevant_chunks.first() {
                expanded_chunk.metadata.chunk_start_char = first.metadata.chunk_start_char;
            }
            if let Some(last) = relevant_chunks.last() {
                expanded_chunk.metadata.chunk_end_char = last.metadata.chunk_end_char;
            }
            
            // 在metadata中标记扩展信息
            expanded_chunk.metadata.custom_fields.insert(
                "expanded".to_string(),
                "true".to_string()
            );
            expanded_chunk.metadata.custom_fields.insert(
                "original_chunk_index".to_string(),
                current_index.to_string()
            );
            expanded_chunk.metadata.custom_fields.insert(
                "expanded_range".to_string(),
                format!("{}-{}", start_index, end_index - 1)
            );
            expanded_chunk.metadata.custom_fields.insert(
                "chunks_merged".to_string(),
                relevant_chunks.len().to_string()
            );
            expanded_chunk.metadata.custom_fields.insert(
                "skip_similarity_filter".to_string(),
                "true".to_string()
            );
            
            info!(
                "Expanded chunk {} from index {} to range {}-{} ({} chunks merged)",
                chunk.id, current_index, start_index, end_index - 1, relevant_chunks.len()
            );
            
            // 扩展后的chunk使用原始分数，但标记为跳过相似度过滤
            expanded_results.push(QueryResult {
                chunk: expanded_chunk,
                score: result.score,
                rank: result.rank,
            });
        }
        
        Ok(expanded_results)
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
        let mut query_results = self.vector_store.search_similar(&collection_name, &request.query, top_k).await?;
        
        info!("Vector search returned {} results", query_results.len());
        for (i, r) in query_results.iter().enumerate() {
            info!("  Result {}: score={:.4}, content_preview={}", 
                  i+1, r.score, r.chunk.content.chars().take(50).collect::<String>());
        }

        // Deduplicate by content_hash to prevent duplicate chunks from multiple ingestions
        use std::collections::HashSet;
        let mut seen_hashes = HashSet::new();
        let before_dedup = query_results.len();
        query_results.retain(|r| {
            let hash = &r.chunk.content_hash;
            if seen_hashes.contains(hash) {
                false
            } else {
                seen_hashes.insert(hash.clone());
                true
            }
        });
        let after_dedup = query_results.len();
        
        if before_dedup > after_dedup {
            info!("Removed {} duplicate chunks (content_hash deduplication)", before_dedup - after_dedup);
        }
        
        // 先进行chunk上下文扩展（在相似度过滤之前）
        // 这样可以确保POC等重要文档的完整性，即使某些部分相似度较低
        query_results = self.expand_chunk_context(query_results).await?;
        info!("After chunk expansion: {} results", query_results.len());

        // 第二轮去重：去除扩展后产生的重复内容
        // 场景：同一个文档的不同chunk被检索到，扩展后变成了完全相同的内容
        // 我们只保留分数最高的那一个
        let mut unique_results = Vec::new();
        let mut seen_expanded_hashes = HashSet::new();
        
        // 按分数降序排序，确保保留分数最高的
        query_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        for result in query_results {
            let hash = &result.chunk.content_hash;
            if !seen_expanded_hashes.contains(hash) {
                seen_expanded_hashes.insert(hash.clone());
                unique_results.push(result);
            } else {
                info!("Removed duplicate expanded content (score: {:.4})", result.score);
            }
        }
        query_results = unique_results;
        
        // Apply similarity threshold filter (but skip expanded chunks)
        let similarity_threshold = request.similarity_threshold.unwrap_or(self._config.similarity_threshold);
        info!("Applying similarity threshold: {:.2}", similarity_threshold);
        let before_filter = query_results.len();
        query_results.retain(|r| {
            // 扩展后的chunk跳过相似度过滤
            let skip_filter = r.chunk.metadata.custom_fields
                .get("skip_similarity_filter")
                .map(|v| v == "true")
                .unwrap_or(false);
            
            if skip_filter {
                info!("Skipping similarity filter for expanded chunk: {}", r.chunk.id);
                true
            } else {
                r.score >= similarity_threshold
            }
        });
        let after_filter = query_results.len();
        
        if before_filter > after_filter {
            info!("Filtered out {} results below similarity threshold {:.2}", 
                  before_filter - after_filter, similarity_threshold);
        }
        
        if query_results.is_empty() {
            info!("No results above similarity threshold {:.2}, returning empty", similarity_threshold);
        } else {
            info!("Returning {} results after threshold filter", query_results.len());
        }
        
        // 构建上下文
        let context_parts: Vec<String> = query_results.iter().map(|r| r.chunk.content.clone()).collect();
        let context = context_parts.join("\n\n");
        let processing_time = start_time.elapsed().as_millis() as u64;

        // 保存查询历史
        if let Some(collection) = self.database.get_rag_collection_by_name(&collection_name).await? {
            if let Err(e) = self.database.save_rag_query(
                Some(&collection.id),
                None, // conversation_id is None for manual queries
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
        
        // 获取所有要查询的集合名称
        let mut collection_names = Vec::new();
        
        if let Some(cids) = &request.collection_ids {
            for cid in cids {
                if !cid.is_empty() {
                    if let Some(collection) = self.database.get_rag_collection_by_id(cid).await? {
                        collection_names.push(collection.name);
                    }
                }
            }
        } else if let Some(cid) = &request.collection_id {
            if cid.is_empty() {
                collection_names.push("default".to_string());
            } else if let Some(collection) = self.database.get_rag_collection_by_id(cid).await? {
                collection_names.push(collection.name);
            }
        } else {
            collection_names.push("default".to_string());
        }

        let top_k = request.top_k.unwrap_or(5);
        let mut all_results: Vec<crate::models::QueryResult> = Vec::new();

        // 从各个集合中搜索并汇总
        for collection_name in collection_names {
            match self.vector_store.search_similar(&collection_name, &request.query, top_k).await {
                Ok(results) => {
                    info!("Collection '{}' returned {} results", collection_name, results.len());
                    for (i, r) in results.iter().enumerate() {
                        info!("  Result {}: score={:.4}, content_hash={}, preview={}", 
                              i+1, r.score, &r.chunk.content_hash[..8], 
                              r.chunk.content.chars().take(50).collect::<String>());
                    }
                    all_results.extend(results);
                }
                Err(e) => {
                    warn!("集合 {} 向量搜索失败: {}", collection_name, e);
                }
            };
        }

        if all_results.is_empty() {
            info!("未找到相关文档，返回空结果");
            return Ok((String::new(), vec![]));
        }

        // 排序并取 Top K（在扩展之前）
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let mut results: Vec<crate::models::QueryResult> = all_results.into_iter().take(top_k).collect();

        // 先进行chunk上下文扩展（在相似度过滤之前）
        // 这样可以确保POC等重要文档的完整性，即使某些部分相似度较低
        results = self.expand_chunk_context(results).await?;
        info!("After chunk expansion: {} results", results.len());

        // Apply similarity threshold filter (but skip expanded chunks)
        let similarity_threshold = request.similarity_threshold.unwrap_or(self._config.similarity_threshold);
        let before_filter = results.len();
        results.retain(|r| {
            // 扩展后的chunk跳过相似度过滤
            let skip_filter = r.chunk.metadata.custom_fields
                .get("skip_similarity_filter")
                .map(|v| v == "true")
                .unwrap_or(false);
            
            if skip_filter {
                info!("Skipping similarity filter for expanded chunk in assistant query: {}", r.chunk.id);
                true
            } else {
                r.score >= similarity_threshold
            }
        });
        let after_filter = results.len();
        
        if before_filter > after_filter {
            info!("Filtered out {} results below similarity threshold {:.2}", 
                  before_filter - after_filter, similarity_threshold);
        }
        
        if results.is_empty() {
            info!("No results above similarity threshold {:.2}, returning empty", similarity_threshold);
            return Ok((String::new(), vec![]));
        }

        // Deduplicate by content_hash to prevent duplicate chunks (after expansion)
        use std::collections::HashSet;
        let mut seen_hashes = HashSet::new();
        let before_dedup = results.len();
        results.retain(|r| {
            let hash = &r.chunk.content_hash;
            if seen_hashes.contains(hash) {
                false
            } else {
                seen_hashes.insert(hash.clone());
                true
            }
        });
        let after_dedup = results.len();
        
        if before_dedup > after_dedup {
            info!("Removed {} duplicate chunks (content_hash deduplication)", before_dedup - after_dedup);
        }

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
        let processing_time = start_time.elapsed().as_millis() as u64;

        // 保存查询历史
        if let Err(e) = self.database.save_rag_query(
            None, // Assistant queries might span multiple collections
            request.conversation_id.as_deref(),
            &request.query,
            &context,
            processing_time,
        ).await {
            warn!("保存助手查询历史失败: {}", e);
        }
        
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
        let total_documents: usize = collections.iter().map(|c| c.document_count).sum();
        let total_chunks: usize = collections.iter().map(|c| c.chunk_count).sum();
        
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