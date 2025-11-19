use crate::services::database::{Database, DatabaseService};
use crate::services::ai::AiServiceManager;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tauri::{State, AppHandle, Emitter, Manager};
use sentinel_rag::models::{AssistantRagRequest,AssistantRagResponse,IngestRequest, IngestResponse, RagQueryRequest, RagQueryResponse, RagStatus,DocumentSource,DocumentChunk, QueryResult, CollectionInfo};
use sentinel_rag::service::RagService;
use sentinel_rag::db::RagDatabase;
use sentinel_rag::config::RagConfig;

// ============================================================================
// 全局RAG服务管理器
// ============================================================================

/// 全局RAG服务实例
/// 使用Arc<RwLock<Option<Arc<RagService>>>>来允许共享和可选性
type AppRagService = RagService<DatabaseService>;
static GLOBAL_RAG_SERVICE: OnceLock<Arc<RwLock<Option<Arc<AppRagService>>>>> = OnceLock::new();

#[async_trait::async_trait]
impl RagDatabase for DatabaseService {
    async fn create_rag_collection(&self, name: &str, description: Option<&str>) -> anyhow::Result<String> {
        self.create_rag_collection(name, description).await
    }

    async fn get_rag_collections(&self) -> anyhow::Result<Vec<CollectionInfo>> {
        self.get_rag_collections().await
    }

    async fn get_rag_collection_by_id(&self, id: &str) -> anyhow::Result<Option<CollectionInfo>> {
        self.get_rag_collection_by_id(id).await
    }

    async fn get_rag_collection_by_name(&self, name: &str) -> anyhow::Result<Option<CollectionInfo>> {
        self.get_rag_collection_by_name(name).await
    }

    async fn delete_rag_collection(&self, id: &str) -> anyhow::Result<()> {
        self.delete_rag_collection(id).await
    }

    async fn create_rag_document(
        &self,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        content: &str,
        metadata: &str,
    ) -> anyhow::Result<String> {
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
    ) -> anyhow::Result<String> {
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

    async fn update_collection_stats(&self, collection_id: &str) -> anyhow::Result<()> {
        self.update_collection_stats(collection_id).await
    }

    async fn get_rag_documents(&self, collection_id: &str) -> anyhow::Result<Vec<DocumentSource>> {
        self.get_rag_documents_by_collection_id(collection_id).await
    }

    async fn get_rag_chunks(&self, document_id: &str) -> anyhow::Result<Vec<DocumentChunk>> {
        self.get_rag_chunks_by_document_id(document_id).await
    }

    async fn delete_rag_document(&self, document_id: &str) -> anyhow::Result<()> {
        self.delete_rag_document(document_id).await
    }

    async fn save_rag_query(
        &self,
        collection_id: Option<&str>,
        query: &str,
        response: &str,
        processing_time_ms: u64,
    ) -> anyhow::Result<()> {
        // Stored against provided id/name; current implementation treats param as name
        self.save_rag_query(collection_id, query, response, processing_time_ms).await
    }

    async fn get_rag_query_history(
        &self,
        collection_id: Option<&str>,
        limit: Option<i32>,
    ) -> anyhow::Result<Vec<QueryResult>> {
        self.get_rag_query_history(collection_id, limit).await
    }
}

/// 初始化全局RAG服务
pub async fn initialize_global_rag_service(database: Arc<DatabaseService>) -> Result<(), String> {
    // 尝试从数据库加载配置，失败则使用默认配置
    let config = match database.get_rag_config().await {
        Ok(Some(config)) => {
            info!("使用数据库中的RAG配置");
            config
        }
        Ok(None) => {
            info!("数据库中未找到RAG配置，使用默认配置");
            RagConfig::default()
        }
        Err(e) => {
            log::warn!("加载数据库RAG配置失败: {}，使用默认配置", e);
            RagConfig::default()
        }
    };

    let rag_service = RagService::new(config, database).await
        .map_err(|e| format!("Failed to create RAG service: {}", e))?;

    // 如果全局实例已存在，则直接替换内部的服务引用；否则初始化一次
    if let Some(existing_wrapper) = GLOBAL_RAG_SERVICE.get() {
        let mut guard = existing_wrapper.write().await;
        *guard = Some(Arc::new(rag_service));
        info!("Global RAG service reloaded successfully");
        Ok(())
    } else {
        let service_wrapper = Arc::new(RwLock::new(Some(Arc::new(rag_service))));
        GLOBAL_RAG_SERVICE
            .set(service_wrapper)
            .map_err(|_| "Failed to set global RAG service")?;
        info!("Global RAG service initialized successfully");
        Ok(())
    }
}

/// 获取全局RAG服务实例
pub async fn get_global_rag_service() -> Result<Arc<AppRagService>, String> {
    let service_wrapper = GLOBAL_RAG_SERVICE.get()
        .ok_or("Global RAG service not initialized")?;
    
    let service_guard = service_wrapper.read().await;
    let service = service_guard.as_ref()
        .ok_or("RAG service not available")?;
    
    // 返回Arc的克隆
    Ok(Arc::clone(service))
}

/// 关闭全局RAG服务
pub async fn shutdown_global_rag_service() -> Result<(), String> {
    if let Some(service_wrapper) = GLOBAL_RAG_SERVICE.get() {
        let mut service_guard = service_wrapper.write().await;
        *service_guard = None;
        info!("Global RAG service shutdown successfully");
        Ok(())
    } else {
        Err("Global RAG service not initialized".to_string())
    }
}

// ============================================================================
// RAG命令实现
// ============================================================================

/// RAG 系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSystemStatus {
    pub initialized: bool,
    pub collections_count: usize,
    pub total_documents: usize,
    pub total_chunks: usize,
}

/// 导入数据源到RAG系统
#[tauri::command]
pub async fn rag_ingest_source(
    file_path: String,
    collection_id: Option<String>,
    metadata: Option<HashMap<String, String>>,
) -> Result<IngestResponse, String> {
    info!("开始导入数据源: {}", file_path);
    
    let request = IngestRequest {
        file_path: file_path.clone(),
        collection_id,
        metadata,
    };
    
    let rag_service = get_global_rag_service().await?;
    rag_service.ingest_source(request).await.map_err(|e| e.to_string())
}

/// 查询RAG系统
#[tauri::command]
pub async fn rag_query(
    query: String,
    collection_id: Option<String>,
    top_k: Option<usize>,
    use_mmr: Option<bool>,
    mmr_lambda: Option<f32>,
    filters: Option<HashMap<String, String>>,
) -> Result<RagQueryResponse, String> {
    info!("RAG查询: {}", query);
    
    let request = RagQueryRequest {query:query.clone(),collection_id,top_k,use_mmr,mmr_lambda,filters, use_embedding: Some(true), reranking_enabled: Some(true) };
    
    let rag_service = get_global_rag_service().await?;  
    rag_service.query(request).await.map_err(|e| e.to_string())
}

/// 清空RAG集合
#[tauri::command]
pub async fn rag_clear_collection(
    collection_id: String,
) -> Result<bool, String> {
    info!("清空RAG集合: {}", collection_id);
    
    let rag_service = get_global_rag_service().await?;
    rag_service.clear_collection(&collection_id).await.map_err(|e| e.to_string())?;
    Ok(true)
}


/// 初始化RAG服务
#[tauri::command]
pub async fn rag_initialize_service(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<bool, String> {
    info!("初始化RAG服务");
    
    initialize_global_rag_service(database.inner().clone()).await?;
    Ok(true)
}

/// 关闭RAG服务
#[tauri::command]
pub async fn rag_shutdown_service() -> Result<bool, String> {
    info!("关闭RAG服务");
    
    shutdown_global_rag_service().await?;
    Ok(true)
}

// ============================================================================
// 文档级别操作命令（列出文档、查看文档内容、删除文档）
// ============================================================================

/// 列出集合中的文档（通过集合ID）
#[tauri::command]
pub async fn list_rag_documents(
    collection_id: String,
    database: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<DocumentSource>, String> {
    database
        .get_rag_documents_by_collection_id(&collection_id)
        .await
        .map_err(|e| format!("获取文档列表失败: {}", e))
}

/// 获取指定文档的所有文本块
#[tauri::command]
pub async fn get_rag_document_chunks(
    document_id: String,
    database: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<DocumentChunk>, String> {
    database
        .get_rag_chunks_by_document_id(&document_id)
        .await
        .map_err(|e| format!("获取文档内容失败: {}", e))
}

/// 删除指定文档（并更新集合统计）
#[tauri::command]
pub async fn delete_rag_document(
    document_id: String,
    database: State<'_, Arc<DatabaseService>>,
) -> Result<bool, String> {
    // 获取集合ID用于删除后更新统计
    let collection_id = database
        .get_collection_id_by_document_id(&document_id)
        .await
        .map_err(|e| format!("查询集合ID失败: {}", e))?;

    database
        .delete_rag_document(&document_id)
        .await
        .map_err(|e| format!("删除文档失败: {}", e))?;

    if let Some(cid) = collection_id.as_deref() {
        let _ = database.update_collection_stats(cid).await;
    }

    Ok(true)
}

/// 获取支持的文件类型
#[tauri::command]
pub async fn rag_get_supported_file_types() -> Result<Vec<String>, String> {
    info!("获取支持的文件类型");
    
    let supported_types = vec![
        "txt".to_string(),
        "md".to_string(),
        "docx".to_string(),
        "pdf".to_string(),
    ];
    
    Ok(supported_types)
}

/// 获取RAG系统状态 (前端兼容命名)
#[tauri::command]
pub async fn get_rag_status() -> Result<RagStatus, String> {

    let rag_service = get_global_rag_service().await?;
    rag_service.get_status().await.map_err(|e| e.to_string())
}

/// 创建RAG集合
#[tauri::command]
pub async fn create_rag_collection(
    name: String,
    description: Option<String>,
) -> Result<bool, String> {
    info!("创建RAG集合: {}", name);
    
    let rag_service = get_global_rag_service().await?;
    let _collection_id = rag_service.create_collection(&name, description.as_deref(), "default").await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// 前端兼容的查询命令
#[tauri::command]
pub async fn query_rag(
    request: RagQueryRequest,
) -> Result<RagQueryResponse, String> {
    let service = get_global_rag_service().await?;
    
    service.query(request).await
        .map_err(|e| format!("Query failed: {}", e))
}

/// 前端兼容的删除集合命令
#[tauri::command]
pub async fn delete_rag_collection(
    collection_id: String,
) -> Result<bool, String> {
    let service = get_global_rag_service().await?;
    
    service.clear_collection(&collection_id).await
        .map_err(|e| format!("Failed to delete collection: {}", e))?;
    
    Ok(true)
}

/// 获取RAG配置
#[tauri::command]
pub async fn get_rag_config(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<RagConfig, String> {
    info!("获取RAG配置");
    
    match database.get_rag_config().await {
        Ok(Some(config)) => {
            info!("成功从数据库加载RAG配置");
            Ok(config)
        }
        Ok(None) => {
            info!("数据库中未找到RAG配置，返回默认配置");
            Ok(RagConfig::default())
        }
        Err(e) => {
            let error_msg = format!("获取RAG配置失败: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 保存RAG配置
#[tauri::command]
pub async fn save_rag_config(
    config: RagConfig,
    database: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<bool, String> {
    info!("保存RAG配置: {:?}", config);
    
    match database.save_rag_config(&config).await {
        Ok(_) => {
            info!("RAG配置已成功保存到数据库");
            // 立即重载RAG服务以应用最新配置（嵌入模型与分块策略生效）
            if let Err(e) = initialize_global_rag_service(database.inner().clone()).await {
                warn!("重载RAG服务失败: {}", e);
            }
            // 向前端广播配置变更事件
            if let Err(e) = app.emit("rag_config_updated", &config) {
                log::warn!("Failed to emit rag_config_updated: {}", e);
            }
            Ok(true)
        }
        Err(e) => {
            let error_msg = format!("保存RAG配置失败: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 重置RAG配置为默认值
#[tauri::command]
pub async fn reset_rag_config(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<RagConfig, String> {
    info!("重置RAG配置为默认值");
    
    let default_config = RagConfig::default();
    
    match database.save_rag_config(&default_config).await {
        Ok(_) => {
            info!("RAG配置已重置并保存到数据库");
            Ok(default_config)
        }
        Err(e) => {
            let error_msg = format!("重置RAG配置失败: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 获取文件夹中的所有文档文件
#[tauri::command]
pub async fn get_folder_files(
    folder_path: String,
    extensions: Vec<String>,
) -> Result<Vec<String>, String> {
    use std::path::Path;
    use walkdir::WalkDir;

    let mut files = Vec::new();
    let folder = Path::new(&folder_path);
    
    if !folder.exists() || !folder.is_dir() {
        return Err("指定的路径不存在或不是文件夹".to_string());
    }
    
    // 遍历文件夹中的所有文件
    for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // 只处理文件，跳过目录
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if let Some(ext_str) = extension.to_str() {
                    // 检查文件扩展名是否在支持的扩展名列表中
                    if extensions.iter().any(|e| e.eq_ignore_ascii_case(ext_str)) {
                        if let Some(path_str) = path.to_str() {
                            files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    info!("在文件夹 {} 中找到 {} 个文档文件", folder_path, files.len());
    Ok(files)
}

/// AI助手RAG答案生成（非流式）
#[tauri::command]
pub async fn assistant_rag_answer(
    request: AssistantRagRequest,
    database: State<'_, Arc<DatabaseService>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    app: AppHandle,
) -> Result<AssistantRagResponse, String> {
    use AssistantRagResponse;
    
    let start_time = std::time::Instant::now();
    info!("AI助手RAG查询: {}", request.query);

    // 全局开关：若未启用增强，则直接返回提示并不进行检索
    match database.get_rag_config().await {
        Ok(Some(cfg)) if !cfg.augmentation_enabled => {
            return Ok(AssistantRagResponse {
                answer: "知识检索增强已关闭。本次未使用知识库。".to_string(),
                citations: vec![],
                context_used: String::new(),
                total_tokens_used: 0,
                rag_tokens: 0,
                llm_tokens: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                fallback_reason: Some("RAG disabled".to_string()),
            });
        }
        Ok(None) => {
            // 无配置等同未启用
            return Ok(AssistantRagResponse {
                answer: "知识检索增强未启用。本次未使用知识库。".to_string(),
                citations: vec![],
                context_used: String::new(),
                total_tokens_used: 0,
                rag_tokens: 0,
                llm_tokens: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                fallback_reason: Some("RAG disabled (no config)".to_string()),
            });
        }
        Err(e) => {
            log::warn!("读取RAG配置失败({}), 视为未启用", e);
            return Ok(AssistantRagResponse {
                answer: "知识检索增强状态未知，已按未启用处理。".to_string(),
                citations: vec![],
                context_used: String::new(),
                total_tokens_used: 0,
                rag_tokens: 0,
                llm_tokens: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                fallback_reason: Some("RAG disabled (config error)".to_string()),
            });
        }
        _ => {}
    }
    
    // 获取RAG服务实例
    let rag_service = get_global_rag_service().await
        .map_err(|e| format!("Failed to get RAG service: {}", e))?;
    
    // 执行RAG检索
    let (context, citations) = match rag_service.query_for_assistant(&request).await {
        Ok(result) => result,
        Err(e) => {
            warn!("RAG检索失败: {}, 将返回无上下文回答", e);
            return Ok(AssistantRagResponse {
                answer: "抱歉，无法检索到相关知识来回答您的问题。".to_string(),
                citations: vec![],
                context_used: String::new(),
                total_tokens_used: 0,
                rag_tokens: 0,
                llm_tokens: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                fallback_reason: Some(format!("RAG检索失败: {}", e)),
            });
        }
    };
    
    // 记录是否找到了相关上下文，但不提前返回，继续调用LLM
    let has_context = !context.is_empty();
    if !has_context {
        info!("未找到相关上下文，但将继续调用LLM处理用户查询");
    }
    
    // 获取当前角色提示词
    let mut role_prompt = String::new();
    if let Ok(Some(current_role)) = database.get_current_ai_role().await {
        if !current_role.prompt.trim().is_empty() {
            role_prompt = current_role.prompt;
            tracing::info!("Using role prompt from: {}", current_role.title);
        }
    }

    // 构建系统提示词（根据是否有上下文调整策略）
    let system_prompt = {
        let history = request
            .conversation_history
            .as_ref()
            .map(|h| if h.is_empty() { String::new() } else { format!("\n[CONVERSATION HISTORY]\n{}\n", h.join("\n")) })
            .unwrap_or_default();
        
        // 基础系统提示词（角色提示词优先）
        let base_system = if !role_prompt.is_empty() {
            role_prompt
        } else if let Some(custom_prompt) = &request.system_prompt {
            if !custom_prompt.trim().is_empty() {
                custom_prompt.clone()
            } else {
                "你是一个有用的AI助手。".to_string()
            }
        } else {
            "你是一个有用的AI助手。".to_string()
        };
        
        if has_context {
            let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。如果证据不足，请直接回答并避免编造。";
            format!(
                "{}\n\n[知识溯源规范]\n{}\n\n[证据块]\n{}{}",
                base_system, policy, context, history
            )
        } else {
            format!(
                "{}。由于没有找到特定知识库上下文，请根据您的训练数据提供一般有用的回答。{}",
                base_system, history
            )
        }
    };

    // 选择模型：优先请求体中的 provider/model，其次默认聊天模型
    let (provider, model) = if let (Some(p), Some(m)) = (request.model_provider.clone(), request.model_name.clone()) {
        (p, m)
    } else {
        match ai_manager.get_default_chat_model().await.map_err(|e| e.to_string())? {
            Some((p, m)) => (p, m),
            None => {
                let msg = "No default chat model configured".to_string();
                log::error!("{}", msg);
                return Err(msg);
            }
        }
    };

    // 直接基于配置构建AI服务
    let mut service = if let Ok(Some(provider_config)) = ai_manager.get_provider_config(&provider).await {
        let mut dynamic_config = provider_config;
        dynamic_config.model = model.clone();
        let db_service = app.state::<Arc<crate::services::database::DatabaseService>>();
        let mcp_service = ai_manager.get_mcp_service();
        crate::services::ai::AiService::new(
            dynamic_config,
            db_service.inner().clone(),
            Some(app.clone()),
            mcp_service,
        )
    } else {
        let msg = format!("Provider config not found for: {}", provider);
        log::error!("{}", msg);
        return Err(msg);
    };
    // 绑定 AppHandle（尽管本次非流式，不强制要求）
    service.set_app_handle(app);

    // 实际调用模型（非流式，不发事件）
    let answer = match service
        .send_message_stream(
            Some(&request.query),
            Some(&system_prompt),
            None,
            None,
            false, // stream
            false, // is_final
            None,
        )
        .await
    {
        Ok(content) => content,
        Err(e) => {
            warn!("LLM请求失败: {}", e);
            return Ok(AssistantRagResponse {
                answer: "抱歉，生成回答失败。请检查AI服务配置或稍后再试。".to_string(),
                citations,
                context_used: context,
                total_tokens_used: 0,
                rag_tokens: 0,
                llm_tokens: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                fallback_reason: Some(format!("LLM request failed: {}", e)),
            });
        }
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    // 简单 token 估算（长度近似）
    let rag_tokens = context.len();
    let llm_tokens = answer.len();
    let total_tokens = rag_tokens + llm_tokens;
    
    info!("AI助手RAG回答生成完成，耗时: {}ms, tokens: {}", processing_time, total_tokens);
    
    Ok(AssistantRagResponse {
        answer,
        citations,
        context_used: context,
        total_tokens_used: total_tokens,
        rag_tokens,
        llm_tokens,
        processing_time_ms: processing_time,
        fallback_reason: if !has_context { Some("No relevant context found, using general AI knowledge".to_string()) } else { None },
    })
}

/// 确保默认RAG集合存在
#[tauri::command]
pub async fn ensure_default_rag_collection() -> Result<String, String> {
    info!("确保默认RAG集合存在");
    
    let rag_service = get_global_rag_service().await?;
    
    const DEFAULT_COLLECTION_NAME: &str = "default";
    
    match rag_service.ensure_default_collection_public().await {
        Ok(collection_id) => {
            info!("默认RAG集合准备就绪: {}", collection_id);
            Ok(collection_id)
        }
        Err(e) => {
            let error_msg = format!("确保默认RAG集合失败: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 重载RAG服务配置
#[tauri::command]
pub async fn reload_rag_service(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<bool, String> {
    info!("重载RAG服务配置");
    
    // 获取最新配置
    let _config = match database.get_rag_config().await {
        Ok(Some(config)) => config,
        Ok(None) => RagConfig::default(),
        Err(e) => return Err(format!("加载配置失败: {}", e)),
    };
    
    // 重新初始化全局RAG服务
    match initialize_global_rag_service(database.inner().clone()).await {
        Ok(_) => {
            info!("RAG服务配置已重载");
            Ok(true)
        }
        Err(e) => {
            let error_msg = format!("重载RAG服务失败: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 设置集合激活状态
#[tauri::command]
pub async fn set_rag_collection_active(
    collection_id: String,
    active: bool,
    database: State<'_, Arc<DatabaseService>>,
) -> Result<bool, String> {
    database
        .set_rag_collection_active(&collection_id, active)
        .await
        .map_err(|e| format!("设置集合激活状态失败: {}", e))?;
    Ok(true)
}

/// 获取所有已激活集合ID列表
#[tauri::command]
pub async fn get_active_rag_collections(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let cols = database
        .get_rag_collections()
        .await
        .map_err(|e| format!("获取集合失败: {}", e))?;
    Ok(cols.into_iter().filter(|c| c.is_active).map(|c| c.id).collect())
}

/// 测试嵌入连接
#[tauri::command]
pub async fn test_embedding_connection(
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use sentinel_rag::config::EmbeddingConfig;
    use sentinel_rag::embeddings::create_embedding_provider;
    
    info!("测试嵌入连接");
    
    // 解析配置
    let embedding_config: EmbeddingConfig = serde_json::from_value(config)
        .map_err(|e| format!("解析嵌入配置失败: {}", e))?;
    
    // 创建嵌入提供商
    let provider = create_embedding_provider(&embedding_config)
        .map_err(|e| format!("创建嵌入提供商失败: {}", e))?;
    
    // 测试嵌入生成
    let test_texts = vec!["Hello world".to_string(), "Test embedding".to_string()];
    
    match provider.embed_texts(&test_texts).await {
        Ok(embeddings) => {
            let dimension = provider.get_embedding_dimension().await.unwrap_or(0);
            info!(
                "嵌入连接测试成功: 提供商={}, 模型={}, 维度={}, 测试向量数={}",
                provider.provider_name(),
                provider.model_name(),
                dimension,
                embeddings.len()
            );
            
            Ok(serde_json::json!({
                "success": true,
                "message": format!(
                    "Successfully connected to {} ({}), dimension: {}, generated {} test embeddings",
                    provider.provider_name(),
                    provider.model_name(),
                    dimension,
                    embeddings.len()
                ),
                "provider": provider.provider_name(),
                "model": provider.model_name(),
                "dimension": dimension,
                "test_embeddings_count": embeddings.len()
            }))
        }
        Err(e) => {
            let error_msg = format!("嵌入连接测试失败: {}", e);
            warn!("{}", error_msg);
            
            Ok(serde_json::json!({
                "success": false,
                "message": error_msg,
                "provider": provider.provider_name(),
                "model": provider.model_name()
            }))
        }
    }
}