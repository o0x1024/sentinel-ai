use crate::services::database::DatabaseService;
use sentinel_db::Database;
use log::{info, warn};
use sentinel_rag::config::RagConfig as RagConfigRag;
use sentinel_core::models::rag_config::RagConfig as RagConfigCore;
use sentinel_rag::models::{
    IngestRequest, IngestResponse, RagQueryRequest, RagQueryResponse, RagStatus,
    DocumentSource, DocumentChunk,
};
use sentinel_rag::service::RagService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

// ============================================================================
// 全局RAG服务管理器
// ============================================================================

/// 全局RAG服务实例
/// 使用Arc<RwLock<Option<Arc<RagService>>>>来允许共享和可选性
type AppRagService = RagService<DatabaseService>;
static GLOBAL_RAG_SERVICE: OnceLock<Arc<RwLock<Option<Arc<AppRagService>>>>> = OnceLock::new();


fn convert_core_to_rag(core: RagConfigCore) -> RagConfigRag {
    RagConfigRag {
        database_path: core.database_path,
        chunk_size_chars: core.chunk_size_chars,
        chunk_overlap_chars: core.chunk_overlap_chars,
        top_k: core.top_k,
        mmr_lambda: core.mmr_lambda,
        batch_size: core.batch_size,
        max_concurrent: core.max_concurrent,
        embedding_provider: core.embedding_provider,
        embedding_model: core.embedding_model,
        embedding_dimensions: core.embedding_dimensions,
        embedding_api_key: core.embedding_api_key,
        embedding_base_url: core.embedding_base_url,
        reranking_provider: core.reranking_provider,
        reranking_model: core.reranking_model,
        reranking_enabled: core.reranking_enabled,
        similarity_threshold: core.similarity_threshold,
        augmentation_enabled: core.augmentation_enabled,
        context_window_size: core.context_window_size,
        chunking_strategy: match core.chunking_strategy {
            sentinel_core::models::rag_config::ChunkingStrategy::FixedSize => sentinel_rag::config::ChunkingStrategy::FixedSize,
            sentinel_core::models::rag_config::ChunkingStrategy::RecursiveCharacter => sentinel_rag::config::ChunkingStrategy::RecursiveCharacter,
            sentinel_core::models::rag_config::ChunkingStrategy::Semantic => sentinel_rag::config::ChunkingStrategy::Semantic,
            sentinel_core::models::rag_config::ChunkingStrategy::StructureAware => sentinel_rag::config::ChunkingStrategy::StructureAware,
        },
        min_chunk_size_chars: core.min_chunk_size_chars,
        max_chunk_size_chars: core.max_chunk_size_chars,
    }
}

fn convert_rag_to_core(rag: RagConfigRag) -> RagConfigCore {
    RagConfigCore {
        database_path: rag.database_path,
        chunk_size_chars: rag.chunk_size_chars,
        chunk_overlap_chars: rag.chunk_overlap_chars,
        top_k: rag.top_k,
        mmr_lambda: rag.mmr_lambda,
        batch_size: rag.batch_size,
        max_concurrent: rag.max_concurrent,
        embedding_provider: rag.embedding_provider,
        embedding_model: rag.embedding_model,
        embedding_dimensions: rag.embedding_dimensions,
        embedding_api_key: rag.embedding_api_key,
        embedding_base_url: rag.embedding_base_url,
        reranking_provider: rag.reranking_provider,
        reranking_model: rag.reranking_model,
        reranking_enabled: rag.reranking_enabled,
        similarity_threshold: rag.similarity_threshold,
        augmentation_enabled: rag.augmentation_enabled,
        context_window_size: rag.context_window_size,
        chunking_strategy: match rag.chunking_strategy {
            sentinel_rag::config::ChunkingStrategy::FixedSize => sentinel_core::models::rag_config::ChunkingStrategy::FixedSize,
            sentinel_rag::config::ChunkingStrategy::RecursiveCharacter => sentinel_core::models::rag_config::ChunkingStrategy::RecursiveCharacter,
            sentinel_rag::config::ChunkingStrategy::Semantic => sentinel_core::models::rag_config::ChunkingStrategy::Semantic,
            sentinel_rag::config::ChunkingStrategy::StructureAware => sentinel_core::models::rag_config::ChunkingStrategy::StructureAware,
        },
        min_chunk_size_chars: rag.min_chunk_size_chars,
        max_chunk_size_chars: rag.max_chunk_size_chars,
    }
}

/// 初始化全局RAG服务
pub async fn initialize_global_rag_service(database: Arc<DatabaseService>) -> Result<(), String> {
    // 尝试从数据库加载配置，失败则使用默认配置
    let config = match database.get_rag_config().await {
        Ok(Some(config_core)) => {
            info!("使用数据库中的RAG配置");
            convert_core_to_rag(config_core)
        }
        Ok(None) => {
            info!("数据库中未找到RAG配置，使用默认配置");
            RagConfigRag::default()
        }
        Err(e) => {
            log::warn!("加载数据库RAG配置失败: {}，使用默认配置", e);
            RagConfigRag::default()
        }
    };

    let rag_service = RagService::new(config, database)
        .await
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
    let service_wrapper = GLOBAL_RAG_SERVICE
        .get()
        .ok_or("Global RAG service not initialized")?;

    let service_guard = service_wrapper.read().await;
    let service = service_guard.as_ref().ok_or("RAG service not available")?;

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
    rag_service
        .ingest_source(request)
        .await
        .map_err(|e| e.to_string())
}

/// 手动输入文本导入到RAG系统
#[tauri::command]
pub async fn rag_ingest_text(
    title: String,
    content: String,
    collection_id: Option<String>,
    metadata: Option<HashMap<String, String>>,
) -> Result<IngestResponse, String> {
    info!("开始导入手动输入文本: {}", title);

    if content.trim().is_empty() {
        return Err("文本内容不能为空".to_string());
    }

    let rag_service = get_global_rag_service().await?;
    rag_service
        .ingest_text(&title, &content, collection_id.as_deref(), metadata)
        .await
        .map_err(|e| e.to_string())
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
    use_embedding: Option<bool>,
    reranking_enabled: Option<bool>,
) -> Result<RagQueryResponse, String> {
    info!("RAG查询: {}", query);

    let request = RagQueryRequest {
        query: query.clone(),
        collection_id,
        top_k,
        use_mmr,
        mmr_lambda,
        filters,
        use_embedding: Some(use_embedding.unwrap_or(true)),
        reranking_enabled: Some(reranking_enabled.unwrap_or(true)),
    };

    let rag_service = get_global_rag_service().await?;
    rag_service.query(request).await.map_err(|e| e.to_string())
}

/// 清空RAG集合
#[tauri::command]
pub async fn rag_clear_collection(collection_id: String) -> Result<bool, String> {
    info!("清空RAG集合: {}", collection_id);

    let rag_service = get_global_rag_service().await?;
    rag_service
        .clear_collection(&collection_id)
        .await
        .map_err(|e| e.to_string())?;
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
        .get_rag_documents(&collection_id)
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
        .get_rag_chunks(&document_id)
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
    let embedding_model = rag_service.get_config().embedding_model.clone();
    
    let _collection_id = rag_service
        .create_collection(&name, description.as_deref(), &embedding_model)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 前端兼容的查询命令
#[tauri::command]
pub async fn query_rag(request: RagQueryRequest) -> Result<RagQueryResponse, String> {
    // License check
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Err("License required for RAG feature".to_string());
    }

    let service = get_global_rag_service().await?;

    service
        .query(request)
        .await
        .map_err(|e| format!("Query failed: {}", e))
}

/// 前端兼容的删除集合命令
#[tauri::command]
pub async fn delete_rag_collection(collection_id: String) -> Result<bool, String> {
    let service = get_global_rag_service().await?;

    service
        .clear_collection(&collection_id)
        .await
        .map_err(|e| format!("Failed to delete collection: {}", e))?;

    Ok(true)
}

/// 更新RAG集合
#[tauri::command]
pub async fn update_rag_collection(
    database: State<'_, Arc<DatabaseService>>,
    collection_id: String,
    name: String,
    description: Option<String>,
) -> Result<bool, String> {
    info!("更新RAG集合: {} -> {}", collection_id, name);
    
    database.update_rag_collection(&collection_id, &name, description.as_deref())
        .await
        .map_err(|e| format!("Failed to update collection: {}", e))?;
    
    Ok(true)
}

/// 获取RAG配置
#[tauri::command]
pub async fn get_rag_config(
    database: State<'_, Arc<DatabaseService>>,
) -> Result<RagConfigRag, String> {
    info!("获取RAG配置");

    match database.inner().get_rag_config().await {
        Ok(Some(config_core)) => {
            info!("成功从数据库加载RAG配置");
            Ok(convert_core_to_rag(config_core))
        }
        Ok(None) => {
            info!("数据库中未找到RAG配置，返回默认配置");
            Ok(RagConfigRag::default())
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
    config: RagConfigRag,
    database: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<bool, String> {
    info!("保存RAG配置: {:?}", config);

    let core_config = convert_rag_to_core(config.clone());
    match database.inner().save_rag_config(&core_config).await {
        Ok(_) => {
            info!("RAG配置已成功保存到数据库");
            // 立即重载RAG服务以应用最新配置（嵌入模型与分块策略生效）
            if let Err(e) = initialize_global_rag_service(database.inner().clone()).await {
                warn!("重载RAG服务失败: {}", e);
            }
            // 向前端广播配置变更事件
            if let Err(e) = app.emit("rag_config_updated", &core_config) {
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
) -> Result<RagConfigRag, String> {
    info!("重置RAG配置为默认值");

    let default_config = RagConfigRag::default();

    let core_config = convert_rag_to_core(default_config.clone());
    match database.inner().save_rag_config(&core_config).await {
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


/// 确保默认RAG集合存在
#[tauri::command]
pub async fn ensure_default_rag_collection() -> Result<String, String> {
    info!("确保默认RAG集合存在");

    let rag_service = get_global_rag_service().await?;

    #[allow(dead_code)]
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
pub async fn reload_rag_service(database: State<'_, Arc<DatabaseService>>) -> Result<bool, String> {
    info!("重载RAG服务配置");

    // 获取最新配置
    let _config = match database.inner().get_rag_config().await {
        Ok(Some(config_core)) => convert_core_to_rag(config_core),
        Ok(None) => RagConfigRag::default(),
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
    Ok(cols
        .into_iter()
        .filter(|c| c.is_active)
        .map(|c| c.id)
        .collect())
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
    let embedding_config: EmbeddingConfig =
        serde_json::from_value(config).map_err(|e| format!("解析嵌入配置失败: {}", e))?;

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
