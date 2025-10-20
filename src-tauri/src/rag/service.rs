use anyhow::{anyhow, Result};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::rag::chunker::DocumentChunker;
use crate::rag::config::RagConfig;
use crate::rag::embeddings::{EmbeddingManager, RerankingManager};
use crate::rag::models::{
    DocumentChunk, DocumentSource, IngestRequest, IngestResponse, QueryResult, RagQueryRequest,
    RagQueryResponse, RagStatus, CollectionInfo, IngestionStatus,
};
use crate::services::database::DatabaseService;

/// RAG服务主类
pub struct RagService {
    _config: RagConfig,
    database: Arc<DatabaseService>,
    embedding_manager: RwLock<EmbeddingManager>,
    reranking_manager: RwLock<RerankingManager>,
    chunker: DocumentChunker,
    ingestion_status: RwLock<HashMap<String, IngestionStatus>>,
}

impl RagService {
    /// 创建新的RAG服务实例
    pub async fn new(config: RagConfig, database: Arc<DatabaseService>) -> Result<Self> {
        info!("初始化RAG服务");
        
        let mut embedding_manager = EmbeddingManager::new();
        
        // 根据配置创建嵌入提供商
        let embedding_config = crate::rag::config::EmbeddingConfig {
            provider: config.embedding_provider.clone(),
            model: config.embedding_model.clone(),
            api_key: config.embedding_api_key.clone(),
            base_url: config.embedding_base_url.clone(),
            dimensions: config.embedding_dimensions,
        };
        
        if let Ok(provider) = crate::rag::embeddings::create_embedding_provider(&embedding_config) {
            embedding_manager.register_provider(provider);
            info!("已注册嵌入提供商: {}:{}", config.embedding_provider, config.embedding_model);
        } else {
            warn!("无法创建嵌入提供商: {}:{}", config.embedding_provider, config.embedding_model);
        }
        
        let chunker = DocumentChunker::new(config.clone());
        
        // 初始化重排序管理器
        let mut reranking_manager = RerankingManager::new();
        
        // 根据配置创建重排序提供商
        if config.reranking_enabled {
            if let (Some(provider), Some(model)) = (&config.reranking_provider, &config.reranking_model) {
                if let Ok(reranking_provider) = crate::rag::embeddings::create_reranking_provider(
                    provider,
                    model,
                    config.embedding_base_url.clone(), // 重排序使用相同的base_url
                    config.embedding_api_key.clone()   // 重排序使用相同的api_key
                ) {
                    reranking_manager.register_provider(reranking_provider);
                    info!("已注册重排序提供商: {}:{}", provider, model);
                } else {
                    warn!("无法创建重排序提供商: {}:{}", provider, model);
                }
            }
        }
        
        let service = Self {
            _config: config,
            database,
            embedding_manager: RwLock::new(embedding_manager),
            reranking_manager: RwLock::new(reranking_manager),
            chunker,
            ingestion_status: RwLock::new(HashMap::new()),
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
        Ok(())
    }

    /// 创建RAG集合
    pub async fn create_collection(&self, name: &str, description: Option<&str>, _embedding_model: &str) -> Result<String> {
        info!("创建RAG集合: {}", name);
        self.database.create_rag_collection(name, description).await
    }

    /// 获取所有RAG集合
    pub async fn get_collections(&self) -> Result<Vec<CollectionInfo>> {
        info!("获取RAG集合列表");
        self.database.get_rag_collections().await
    }

    /// 删除RAG集合
    pub async fn delete_collection(&self, collection_id: &str) -> Result<()> {
        info!("删除RAG集合: {}", collection_id);
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
        
        let mut processed_chunks = 0;
        let mut chunks_created = 0;

        for (index, chunk) in chunks.iter().enumerate() {
            // 生成嵌入向量
            let embedding = match self.embedding_manager.read().await.embed_texts(&[chunk.content.clone()]).await {
                Ok(embeddings) => embeddings.into_iter().next(),
                Err(e) => {
                    warn!("生成嵌入向量失败: {}", e);
                    None
                }
            };

            // 元数据JSON（包含真实字符偏移等）
            let metadata_json = serde_json::to_string(&chunk.metadata).unwrap_or("{}".to_string());

            // 创建chunk记录
            match self.database.create_rag_chunk(
                &document_id,
                &collection_id,
                &chunk.content,
                index as i32,
                embedding.as_deref(),
                &self._config.embedding_model,
                self._config.embedding_dimensions.unwrap_or(768) as i32,
                &metadata_json,
            ).await {
                Ok(_) => {
                    chunks_created += 1;
                    processed_chunks += 1;
                }
                Err(e) => {
                    warn!("创建chunk记录失败: {}", e);
                    processed_chunks += 1;
                }
            }

            // 更新进度
            ingestion_status.processed_chunks = processed_chunks;
            ingestion_status.progress = (processed_chunks as f64 / ingestion_status.total_chunks as f64) * 100.0;
            self.ingestion_status.write().await.insert(task_id.clone(), ingestion_status.clone());
        }

        // 完成摄取，更新集合统计
        if let Err(e) = self.database.update_collection_stats(&collection_id).await {
            warn!("更新集合统计失败: {}", e);
        }

        ingestion_status.status = "completed".to_string();
        ingestion_status.completed_at = Some(chrono::Utc::now());
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

    /// 查询相似文档
    pub async fn query(&self, request: RagQueryRequest) -> Result<RagQueryResponse> {
        info!("执行RAG查询: {}", request.query);
        
        let start_time = std::time::Instant::now();
        let use_embedding = request.use_embedding.unwrap_or(false);
        let reranking_enabled = request.reranking_enabled.unwrap_or(false);
        
        // 可选：生成查询嵌入向量
        let query_embedding: Vec<f32> = if use_embedding {
            match self.embedding_manager.read().await.embed_texts(&[request.query.clone()]).await {
                Ok(embeddings) => embeddings.into_iter().next().unwrap_or_default(),
                Err(e) => {
                    warn!("生成查询嵌入向量失败，降级到文本检索: {}", e);
                    Vec::new()
                }
            }
        } else { Vec::new() };

        // 搜索相似chunks  
        let collection_id = if let Some(cid) = request.collection_id {
            if cid.is_empty() {
                // 如果collection_id是空字符串，获取默认集合
                match self.database.get_rag_collection_by_name("default").await? {
                    Some(collection) => collection.id,
                    None => {
                        return Err(anyhow!("Default collection not found"));
                    }
                }
            } else {
                cid
            }
        } else {
            // 获取默认集合的ID
            match self.database.get_rag_collection_by_name("default").await? {
                Some(collection) => collection.id,
                None => {
                    return Err(anyhow!("Default collection not found"));
                }
            }
        };
        let top_k = request.top_k.unwrap_or(5);
        
        // 获取候选块：向量 or 文本（携带分数）
        let scored_chunks: Vec<(f32, crate::rag::models::DocumentChunk)> = if use_embedding && !query_embedding.is_empty() {
            // 取较大的候选集，再做内存向量相似度排序
            let candidate_limit = std::cmp::max(top_k * 20, 200);
            let emb_dim = query_embedding.len() as i32;
            let candidates = self.database.fetch_chunks_with_embeddings(&collection_id, &self._config.embedding_model, emb_dim, candidate_limit).await?;
            if candidates.is_empty() {
                info!("无嵌入候选，降级文本检索");
                self.database
                    .search_rag_chunks_by_id(&collection_id, &request.query, top_k)
                    .await?
                    .into_iter()
                    .map(|c| (0.0_f32, c))
                    .collect()
            } else {
                // 计算余弦相似度并排序，仅使用维度匹配的向量
                let mut scored: Vec<(f32, crate::rag::models::DocumentChunk)> = Vec::new();
                for c in candidates.into_iter() {
                    if let Some(ref emb) = c.embedding {
                        if emb.len() == query_embedding.len() {
                            let sim = Self::cosine_similarity_local(&query_embedding, emb);
                            scored.push((sim as f32, c));
                        }
                    }
                }
                if scored.is_empty() {
                    info!("嵌入维度不匹配或无有效向量，降级文本检索");
                    self.database
                        .search_rag_chunks_by_id(&collection_id, &request.query, top_k)
                        .await?
                        .into_iter()
                        .map(|c| (1.0_f32, c))
                        .collect()
                } else {
                    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                    let mut out: Vec<(f32, crate::rag::models::DocumentChunk)> =
                        scored.into_iter().take(top_k).collect();
                    // 若不足top_k，用文本检索补齐
                    if out.len() < top_k {
                        let fill = self
                            .database
                            .search_rag_chunks_by_id(&collection_id, &request.query, top_k - out.len())
                            .await?
                            .into_iter()
                            .map(|c| (1.0_f32, c));
                        out.extend(fill);
                    }
                    out
                }
            }
        } else {
            // 文本检索（分词 AND LIKE）
            self.database
                .search_rag_chunks_by_id(&collection_id, &request.query, top_k)
                .await?
                .into_iter()
                .map(|c| (0.0_f32, c))
                .collect()
        };

        // 构建结果
        let mut results = Vec::new();
        let mut context_parts = Vec::new();
        
        // 可选重排：对候选内容再次排序
        let final_chunks = if reranking_enabled {
            let documents: Vec<String> = scored_chunks.iter().map(|(_, c)| c.content.clone()).collect();
            match self.reranking_manager.read().await.rerank(&request.query, &documents, Some(top_k)).await {
                Ok(reranked) => {
                    // 将重排分数与原相似度进行融合，避免重排把弱相关文本顶到前面
                    // 融合: fused = 0.7 * original_sim + 0.3 * rerank_score
                    let mut fused: Vec<(f32, usize)> = Vec::new();
                    for r in reranked {
                        if r.index < scored_chunks.len() {
                            let orig = scored_chunks[r.index].0;
                            let score = 0.7_f32 * orig + 0.3_f32 * (r.score as f32);
                            fused.push((score, r.index));
                        }
                    }
                    fused.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                    fused.into_iter().take(top_k).map(|(_, idx)| (scored_chunks[idx].0, scored_chunks[idx].1.clone())).collect()
                }
                Err(e) => {
                    warn!("重排失败，使用原相似度排序: {}", e);
                    scored_chunks
                }
            }
        } else {
            scored_chunks
        };

        for (rank, (s, chunk)) in final_chunks.into_iter().enumerate() {
            let score = s;
            
            results.push(QueryResult {
                chunk: chunk.clone(),
                score,
                rank,
            });
            
            context_parts.push(chunk.content);
        }

        let context = context_parts.join("\n\n");
        let processing_time = start_time.elapsed().as_millis() as u64;

        // 保存查询历史
        if let Err(e) = self.database.save_rag_query(
            Some(&collection_id),
            &request.query,
            &context,
            processing_time,
        ).await {
            warn!("保存查询历史失败: {}", e);
        }

        Ok(RagQueryResponse {
            query: request.query,
            results,
            context: context.clone(),
            total_tokens: context.len(), // 简化的token计算
            processing_time_ms: processing_time,
        })
    }

    /// 本地余弦相似度实现，避免外部模块依赖
    fn cosine_similarity_local(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 { return 0.0; }
        dot / (na * nb)
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

    /// 为AI助手查询RAG，返回格式化的上下文和引用
    pub async fn query_for_assistant(&self, request: &crate::rag::models::AssistantRagRequest) -> Result<(String, Vec<crate::rag::models::Citation>)> {
        use crate::rag::models::Citation;
        
        let start_time = std::time::Instant::now();
        
        // 构建查询上下文（仅用于提示展示或后续重排，不参与向量生成）
        let _query_context = if let Some(history) = &request.conversation_history {
            if !history.is_empty() {
                let context_summary = history.iter()
                    .rev()
                    .take(3)
                    .map(|msg| msg.chars().take(100).collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\\n");
                format!("Context: {}\\n\\nQuery: {}", context_summary, request.query)
            } else {
                request.query.clone()
            }
        } else {
            request.query.clone()
        };
        
        // 生成查询嵌入（仅使用原始query，提高召回稳定性）
        let query_embedding = match self.embedding_manager.read().await.embed_texts(&[request.query.clone()]).await {
            Ok(embeddings) => embeddings.into_iter().next().unwrap_or_default(),
            Err(e) => {
                warn!("生成查询嵌入失败: {}, 降级到文本搜索", e);
                vec![]
            }
        };

        // 获取集合ID，自动创建默认集合
        let collection_id = if let Some(cid) = &request.collection_id {
            if cid.is_empty() {
                match self.database.get_rag_collection_by_name("default").await? {
                    Some(collection) => collection.id,
                    None => {
                        info!("默认RAG集合不存在，正在创建...");
                        self.database.create_rag_collection("default", Some("系统默认知识库集合")).await?
                    }
                }
            } else {
                cid.clone()
            }
        } else {
            match self.database.get_rag_collection_by_name("default").await? {
                Some(collection) => collection.id,
                None => {
                    info!("默认RAG集合不存在，正在创建...");
                    self.database.create_rag_collection("default", Some("系统默认知识库集合")).await?
                }
            }
        };

        let top_k = request.top_k.unwrap_or(5);
        let sim_threshold = request.similarity_threshold.unwrap_or(self._config.similarity_threshold);
        
        // 搜索相关文档块
        let chunks = if !query_embedding.is_empty() {
            info!("使用向量相似度搜索，嵌入维度: {}", query_embedding.len());
            
            // 使用向量相似度搜索
            let scored_results = self.database.search_rag_chunks_by_vector(
                &collection_id,
                &query_embedding,
                &self._config.embedding_model,
                top_k,
                sim_threshold,
            ).await?;
            
            // 提取文档块（已按相似度排序）
            let vector_chunks: Vec<_> = scored_results.into_iter().map(|(score, chunk)| {
                info!("向量搜索结果: 相似度={:.4}, 内容预览={}", score, 
                      chunk.content.chars().take(100).collect::<String>());
                chunk
            }).collect();
            
            // 如果向量搜索结果不足，补充文本搜索结果
            if vector_chunks.len() < top_k {
                let remaining = top_k - vector_chunks.len();
                info!("向量搜索结果不足({}/{}), 补充文本搜索结果", vector_chunks.len(), top_k);
                
                let mut text_chunks = self.database.search_rag_chunks_by_id(&collection_id, &request.query, remaining).await?;
                
                // 去重：移除已在向量搜索结果中的文档块
                let vector_ids: std::collections::HashSet<_> = vector_chunks.iter().map(|c| &c.id).collect();
                text_chunks.retain(|c| !vector_ids.contains(&c.id));
                
                // 合并结果
                let mut combined_chunks = vector_chunks;
                combined_chunks.extend(text_chunks.into_iter().take(remaining));
                combined_chunks
            } else {
                vector_chunks
            }
        } else {
            info!("查询嵌入为空，使用文本搜索");
            self.database.search_rag_chunks_by_id(&collection_id, &request.query, top_k).await?
        };

        if chunks.is_empty() {
            info!("未找到相关文档，返回空结果");
            return Ok((String::new(), vec![]));
        }

        // 可选：重排序
        let final_chunks = if request.reranking_enabled.unwrap_or(false) {
            info!("启用重排序处理");
            // 使用RerankingManager进行重排序
            let documents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
            match self.reranking_manager.read().await.rerank(&request.query, &documents, Some(top_k)).await {
                Ok(rerank_results) => {
                    info!("重排序完成，返回 {} 个结果", rerank_results.len());
                    // 根据重排序结果重新排列chunks
                    let mut reranked_chunks = Vec::new();
                    for result in rerank_results {
                        if result.index < chunks.len() {
                            reranked_chunks.push(chunks[result.index].clone());
                        }
                    }
                    reranked_chunks
                }
                Err(e) => {
                    warn!("重排序失败: {}, 使用原始顺序", e);
                    chunks
                }
            }
        } else {
            chunks
        };

        // 扩展上下文：为每个检索到的块获取相邻块
        let expanded_chunks = self.expand_context_window(&final_chunks, &collection_id).await?;
        
        // 构建Evidence Blocks格式的上下文
        let mut evidence_blocks = Vec::new();
        let mut citations = Vec::new();
        
        for (idx, chunk) in expanded_chunks.iter().enumerate() {
            let source_num = idx + 1;
            let file_name = chunk.metadata.file_name.clone();
            let content_preview = chunk.content.chars().take(200).collect::<String>();
            
            // 格式化Evidence Block
            let evidence_block = format!(
                "=== SOURCE {} | {} | page: {} | score: {:.2} ===\\n{}",
                source_num,
                file_name,
                chunk.metadata.page_number.unwrap_or(1),
                0.85, // 临时分数，实际应该从向量搜索得到
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
                score: 0.85, // 临时分数
                content_preview,
            };
            citations.push(citation);
        }
        
        let context = evidence_blocks.join("\\n\\n");
        
        info!("为AI助手准备了 {} 个证据块，处理时间: {:?}", 
              final_chunks.len(), start_time.elapsed());
        
        Ok((context, citations))
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
                info!("默认RAG集合创建完成: {}", collection_id);
                Ok(collection_id)
            }
        }
    }

    /// 重新配置嵌入提供商
    pub async fn reconfigure_embedding(&self, config: &RagConfig) -> Result<()> {
        let mut embedding_manager = self.embedding_manager.write().await;
        
        // 清空现有提供商
        *embedding_manager = EmbeddingManager::new();
        
        // 根据新配置创建嵌入提供商
        let embedding_config = crate::rag::config::EmbeddingConfig {
            provider: config.embedding_provider.clone(),
            model: config.embedding_model.clone(),
            api_key: config.embedding_api_key.clone(),
            base_url: config.embedding_base_url.clone(),
            dimensions: config.embedding_dimensions,
        };
        
        if let Ok(provider) = crate::rag::embeddings::create_embedding_provider(&embedding_config) {
            embedding_manager.register_provider(provider);
            info!("已重新配置嵌入提供商: {}:{}", config.embedding_provider, config.embedding_model);
            Ok(())
        } else {
            let error_msg = format!("无法创建嵌入提供商: {}:{}", config.embedding_provider, config.embedding_model);
            warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    /// 扩展上下文窗口：为检索到的块获取相邻块
    async fn expand_context_window(&self, chunks: &[DocumentChunk], _collection_id: &str) -> Result<Vec<DocumentChunk>> {
        let mut expanded_chunks = Vec::new();
        let mut processed_chunks = std::collections::HashSet::new();
        
        for chunk in chunks {
            // 按文档分组，获取同一文档的相邻块
            let document_chunks = self.database.get_rag_chunks(&chunk.source_id).await?;
            
            // 找到当前块在文档中的位置
            if let Some(current_pos) = document_chunks.iter().position(|c| c.id == chunk.id) {
                // 获取前后各N个块（可配置）
                let context_window = self._config.context_window_size;
                let start_idx = current_pos.saturating_sub(context_window);
                let end_idx = std::cmp::min(current_pos + context_window + 1, document_chunks.len());
                
                // 添加上下文窗口内的所有块
                for i in start_idx..end_idx {
                    if let Some(context_chunk) = document_chunks.get(i) {
                        if !processed_chunks.contains(&context_chunk.id) {
                            expanded_chunks.push(context_chunk.clone());
                            processed_chunks.insert(context_chunk.id.clone());
                        }
                    }
                }
            } else {
                // 如果找不到位置，至少添加原始块
                if !processed_chunks.contains(&chunk.id) {
                    expanded_chunks.push(chunk.clone());
                    processed_chunks.insert(chunk.id.clone());
                }
            }
        }
        
        // 按文档和块索引排序，保持逻辑顺序
        expanded_chunks.sort_by(|a, b| {
            a.source_id.cmp(&b.source_id)
                .then_with(|| a.chunk_index.cmp(&b.chunk_index))
        });
        
        info!("上下文扩展: {} -> {} 块", chunks.len(), expanded_chunks.len());
        Ok(expanded_chunks)
    }

    /// 获取RAG状态
    pub async fn get_status(&self) -> Result<RagStatus> {
        info!("获取RAG状态");
        
        let collections = self.database.get_rag_collections().await?;
        let total_documents: usize = collections.iter().map(|c| c.document_count).sum();
        let total_chunks: usize = collections.iter().map(|c| c.chunk_count).sum();
        
        Ok(RagStatus {
            collections,
            total_documents,
            total_chunks,
            database_size_mb: 0.0, // 简化实现
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rag_service_creation() {
        let _temp_dir = tempdir().unwrap();
        let config = RagConfig {
            augmentation_enabled:false,
            database_path: Some(_temp_dir.path().to_path_buf()),
            chunk_size_chars: 512,
            chunk_overlap_chars: 50,
            top_k: 5,
            mmr_lambda: 0.7,
            batch_size: 10,
            max_concurrent: 4,
            embedding_provider: "lm-studio".to_string(),
            embedding_model: "text-embedding-qwen3-embedding-0.6b".to_string(),
            embedding_dimensions: None,
            embedding_api_key: None,
            embedding_base_url: Some("http://localhost:11434".to_string()),
            reranking_provider: None,
            reranking_model: None,
            reranking_enabled: false,
            similarity_threshold: 0.7,
            context_window_size: 1,
        };

        // 创建临时数据库
        let _temp_dir = tempdir().unwrap();
        let mut database_service = DatabaseService::new();
        database_service.initialize().await.unwrap();
        let database = Arc::new(database_service);
        
        let service = RagService::new(config, database).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_get_status() {
        let _temp_dir = tempdir().unwrap();
        let config = RagConfig {
            augmentation_enabled:false,
            database_path: Some(_temp_dir.path().to_path_buf()),
            chunk_size_chars: 512,
            chunk_overlap_chars: 50,
            top_k: 5,
            mmr_lambda: 0.7,
            batch_size: 10,
            max_concurrent: 4,
            embedding_provider: "ollama".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimensions: None,
            embedding_api_key: None,
            embedding_base_url: Some("http://localhost:11434".to_string()),
            reranking_provider: None,
            reranking_model: None,
            reranking_enabled: false,
            similarity_threshold: 0.7,
            context_window_size: 1,
        };

        // 创建临时数据库
        let mut database_service = DatabaseService::new();
        database_service.initialize().await.unwrap();
        let database = Arc::new(database_service);
        
        let service = RagService::new(config, database).await.unwrap();
        let status = service.get_status().await;
        assert!(status.is_ok());
    }
}