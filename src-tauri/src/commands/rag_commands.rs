use crate::rag::config::RagConfig;
use crate::rag::models::{IngestRequest, IngestResponse, RagQueryRequest, RagQueryResponse, RagStatus};
use crate::rag::service::RagService;
use crate::services::database::DatabaseService;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tauri::{State, AppHandle, Emitter};

// ============================================================================
// 全局RAG服务管理器
// ============================================================================

/// 全局RAG服务实例
/// 使用Arc<RwLock<Option<Arc<RagService>>>>来允许共享和可选性
static GLOBAL_RAG_SERVICE: OnceLock<Arc<RwLock<Option<Arc<RagService>>>>> = OnceLock::new();

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
    
    let service_wrapper = Arc::new(RwLock::new(Some(Arc::new(rag_service))));
    GLOBAL_RAG_SERVICE.set(service_wrapper)
        .map_err(|_| "Failed to set global RAG service")?;
    
    info!("Global RAG service initialized successfully");
    Ok(())
}

/// 获取全局RAG服务实例
pub async fn get_global_rag_service() -> Result<Arc<RagService>, String> {
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

/// 获取RAG系统状态
#[tauri::command]
pub async fn rag_get_status() -> Result<RagStatus, String> {
    info!("获取RAG系统状态");
    
    let rag_service = get_global_rag_service().await?;
    rag_service.get_status().await.map_err(|e| e.to_string())
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
) -> Result<Vec<crate::rag::models::DocumentSource>, String> {
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
) -> Result<Vec<crate::rag::models::DocumentChunk>, String> {
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
    info!("获取RAG系统状态 (前端兼容)");
    
    // 调用原有的rag_get_status函数
    rag_get_status().await
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
    request: crate::rag::models::AssistantRagRequest,
    database: State<'_, Arc<DatabaseService>>,
) -> Result<crate::rag::models::AssistantRagResponse, String> {
    use crate::rag::models::AssistantRagResponse;
    
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
    
    // 如果没有找到相关上下文，返回提示
    if context.is_empty() {
        return Ok(AssistantRagResponse {
            answer: "没有找到相关的知识来回答您的问题。请尝试重新表述问题或检查知识库内容。".to_string(),
            citations: vec![],
            context_used: String::new(),
            total_tokens_used: 0,
            rag_tokens: context.len(),
            llm_tokens: 0,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            fallback_reason: Some("未找到相关上下文".to_string()),
        });
    }
    
    // 构建AI提示词
    let _system_prompt = format!(
        "你是一个智能助手，必须基于提供的上下文回答问题。\n\
        规则：\n\
        1. 只能基于提供的上下文回答，不能编造信息\n\
        2. 如果上下文不足以回答问题，请明确说明\n\
        3. 在回答中使用 [SOURCE n] 格式引用来源\n\
        4. 保持回答准确、简洁、有用\n\n\
        上下文：\n{}", 
        context
    );
    
    let _user_prompt = request.query.clone();
    
    // 调用AI模型生成回答
    // 这里需要集成现有的AI服务
    // 暂时返回一个模拟回答
    let answer = format!(
        "基于提供的上下文，我找到了 {} 个相关来源来回答您的问题。\n\n\
        [模拟回答] 根据检索到的文档内容...\n\n\
        参考来源: {}", 
        citations.len(),
        citations.iter()
            .enumerate()
            .map(|(i, c)| format!("[SOURCE {}] {}", i + 1, c.file_name))
            .collect::<Vec<_>>()
            .join(", ")
    );
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    // TODO: 实际的token计算
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
        fallback_reason: None,
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