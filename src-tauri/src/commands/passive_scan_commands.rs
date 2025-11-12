//! 被动扫描 Tauri 命令
//!
//! 提供前端调用的被动扫描相关命令：
//! - start_passive_scan: 启动被动扫描代理
//! - stop_passive_scan: 停止被动扫描代理
//! - get_proxy_status: 获取代理状态
//! - list_findings: 列出漏洞发现
//! - enable_plugin: 启用插件
//! - disable_plugin: 禁用插件
//! - list_plugins: 列出所有插件

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

use sentinel_passive::{
    CertificateService, Finding, FindingDeduplicator, PassiveDatabaseService, PluginManager,
    PluginRecord, PluginStatus, PluginMetadata, ProxyConfig, ProxyService, ProxyStats, ProxyStatus, ScanPipeline,
    VulnerabilityFilters, VulnerabilityRecord, EvidenceRecord,
};

use crate::events::{emit_finding, emit_plugin_changed, emit_proxy_status, emit_scan_stats};
use crate::events::{FindingEvent, PluginChangedEvent, ProxyStatusEvent, ScanStatsEvent};

/// 被动扫描服务状态（全局单例）
pub struct PassiveScanState {
    proxy_service: Arc<RwLock<Option<ProxyService>>>,
    plugin_manager: Arc<PluginManager>,
    certificate_service: Arc<CertificateService>,
    db_service: std::sync::OnceLock<Arc<PassiveDatabaseService>>,
    database_url: String,
    is_running: Arc<RwLock<bool>>,
    scan_tx: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<sentinel_passive::ScanTask>>>>,
}

impl Clone for PassiveScanState {
    fn clone(&self) -> Self {
        Self {
            proxy_service: self.proxy_service.clone(),
            plugin_manager: self.plugin_manager.clone(),
            certificate_service: self.certificate_service.clone(),
            db_service: std::sync::OnceLock::new(), // OnceLock doesn't support clone, create new
            database_url: self.database_url.clone(),
            is_running: self.is_running.clone(),
            scan_tx: self.scan_tx.clone(),
        }
    }
}

impl std::fmt::Debug for PassiveScanState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassiveScanState")
            .field("database_url", &self.database_url)
            .field("is_running", &"RwLock<bool>")
            .finish()
    }
}

impl PassiveScanState {
    pub fn new() -> Self {
        // 使用系统应用数据目录，与主数据库保持一致
        // macOS: ~/Library/Application Support/sentinel-ai/
        // Linux: ~/.local/share/sentinel-ai/
        // Windows: %APPDATA%\sentinel-ai\

        // 使用当前工作目录下的 ./ca 作为证书目录（项目内路径）
        let ca_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("ca");

        // 保持原有逻辑：仍然使用用户数据目录（避免随代码路径变化导致数据丢失）
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sentinel-ai");
        // 数据库路径：使用主数据库 database.db
        let db_path = app_data_dir.join("database.db");

        let database_url = format!("sqlite://{}", db_path.display());

        Self {
            proxy_service: Arc::new(RwLock::new(None)),
            plugin_manager: Arc::new(PluginManager::new()),
            certificate_service: Arc::new(CertificateService::new(ca_dir)),
            db_service: std::sync::OnceLock::new(),
            database_url,
            is_running: Arc::new(RwLock::new(false)),
            scan_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取或初始化数据库服务（懒加载 - 内部方法）
    async fn get_db_service_internal(&self) -> Result<Arc<PassiveDatabaseService>, String> {
        if let Some(db) = self.db_service.get() {
            return Ok(db.clone());
        }

        // 初始化数据库
        let db = Arc::new(
            PassiveDatabaseService::new(&self.database_url)
                .await
                .map_err(|e| format!("Failed to initialize database: {}", e))?,
        );

        // 尝试设置（如果其他线程已经设置了，使用已有的）
        match self.db_service.set(db.clone()) {
            Ok(_) => Ok(db),
            Err(_) => Ok(self.db_service.get().unwrap().clone()),
        }
    }

    /// 公开方法：获取数据库服务（用于工具提供者）
    pub async fn get_db_service(&self) -> Result<Arc<PassiveDatabaseService>, String> {
        self.get_db_service_internal().await
    }

    /// 公开方法：获取插件管理器（用于工具提供者）
    pub fn get_plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
    }

    /// 内部方法：列出所有插件（数据库来源，供工具提供者使用）
    pub async fn list_plugins_internal(&self) -> Result<Vec<PluginRecord>, String> {
        let db = self.get_db_service().await.map_err(|e| e.to_string())?;
        // 查询数据库中所有插件（包含 main_category）
        let rows = sqlx::query_as::<_, (
            String, // id
            String, // name
            String, // version
            Option<String>, // author
            String, // main_category
            String, // category
            Option<String>, // description
            String, // default_severity
            Option<String>, // tags (JSON)
            bool,   // enabled
            Option<String> // plugin_code
        )>(
            r#"
            SELECT id, name, version, author, main_category, category, description,
                   default_severity, tags, enabled, plugin_code
            FROM plugin_registry
            "#
        )
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Failed to query database plugins: {}", e))?;

        let mut records = Vec::new();
        for (id, name, version, author, main_category, category, description, default_severity, tags, enabled, _plugin_code) in rows {
            tracing::info!(
                "list_plugins_internal: row id='{}', main_category='{}', category='{}', enabled={}",
                id, main_category, category, enabled
            );
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            let severity = match default_severity.to_lowercase().as_str() {
                "critical" => sentinel_passive::types::Severity::Critical,
                "high" => sentinel_passive::types::Severity::High,
                "medium" => sentinel_passive::types::Severity::Medium,
                "low" => sentinel_passive::types::Severity::Low,
                "info" => sentinel_passive::types::Severity::Info,
                _ => sentinel_passive::types::Severity::Medium,
            };

            let metadata = PluginMetadata {
                id: id.clone(),
                name,
                version,
                author,
                // 从数据库字段加载 main_category（passive/agent）
                main_category,
                category,
                description,
                default_severity: severity,
                tags: tags_array,
            };

            let status = if enabled { PluginStatus::Enabled } else { PluginStatus::Disabled };

            records.push(PluginRecord {
                metadata,
                path: None,  // 插件存储在数据库中，不再使用文件路径
                status,
                last_error: None,
            });
        }

        Ok(records)
    }
}

/// 命令响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// 启动被动扫描代理
#[tauri::command]
pub async fn start_passive_scan(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    config: Option<ProxyConfig>,
) -> Result<CommandResponse<u16>, String> {
    let mut is_running = state.is_running.write().await;
    if *is_running {
        return Ok(CommandResponse::err("Proxy already running".to_string()));
    }

    let config = config.unwrap_or_default();
    
    // 使用当前工作目录下的 ./ca 目录（证书）
    let ca_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("ca");
    let proxy = ProxyService::with_ca_dir(config, ca_dir);

    // 创建扫描与发现通道（scan_rx 在单独线程内消费）
    let (scan_tx, scan_rx) = tokio::sync::mpsc::unbounded_channel();
    let (finding_tx, finding_rx) = tokio::sync::mpsc::unbounded_channel();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<Finding>();

    // 保存 scan_tx 到 state，以便后续重载插件时使用
    {
        let mut scan_tx_guard = state.scan_tx.write().await;
        *scan_tx_guard = Some(scan_tx.clone());
    }

    // 获取数据库服务（懒加载初始化）
    let db_service = state.get_db_service().await.map_err(|e| e.to_string())?;

    // 将非 Send 的 ScanPipeline 放入独立线程 + current-thread tokio runtime 中运行
    let db_for_pipeline = db_service.clone();
    let app_for_pipeline = app.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build current_thread runtime for ScanPipeline");
        rt.block_on(async move {
            let pipeline = ScanPipeline::new(scan_rx, finding_tx)
                .with_db_service(db_for_pipeline.clone())
                .with_app_handle(app_for_pipeline);
            match pipeline.load_enabled_plugins_from_db(&db_for_pipeline).await {
                Ok(n) => tracing::info!("Loaded {} enabled plugins into ScanPipeline", n),
                Err(e) => tracing::error!("Failed to load enabled plugins: {}", e),
            }
            if let Err(e) = pipeline.start().await {
                tracing::error!("ScanPipeline exited with error: {}", e);
            } else {
                tracing::info!("ScanPipeline stopped normally");
            }
        });
    });

    // 启动 Finding 去重服务（带数据库和事件发送）
    let deduplicator = FindingDeduplicator::with_database(finding_rx, db_service.clone())
        .with_event_sender(event_tx);
    tokio::spawn(async move {
        if let Err(e) = deduplicator.start().await {
            tracing::error!("FindingDeduplicator error: {}", e);
        }
    });

    // 启动代理服务（绑定端口，并将 scan_tx 注入）
    let port = match proxy.start(Some(scan_tx)).await {
        Ok(port) => port,
        Err(e) => {
            return Ok(CommandResponse::err(format!("Failed to start proxy: {}", e)))
        }
    };

    // 事件监听（发现推送给前端）
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(finding) = event_rx.recv().await {
            emit_finding(&app_clone, FindingEvent::from(finding));
        }
    });

    // 启动周期性统计发射任务（每5秒），包括插件数量（当前无法动态刷新，需要重启以加载新插件）
    let app_clone2 = app.clone();
    let is_running_arc = state.is_running.clone();
    let proxy_service_arc = state.proxy_service.clone();
    let db_service_arc = db_service.clone();
    // NOTE: 当前未在统计事件中加入插件数；需要时可通过额外通道回传

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            let is_running = *is_running_arc.read().await;
            if !is_running {
                break;
            }

            let proxy_opt = proxy_service_arc.read().await;
            if let Some(proxy) = proxy_opt.as_ref() {
                let stats = proxy.get_stats().await;

                // 查询数据库中的总漏洞数
                let total_findings = db_service_arc
                    .count_vulnerabilities(Default::default())
                    .await
                    .unwrap_or(0);

                emit_scan_stats(
                    &app_clone2,
                    ScanStatsEvent {
                        requests: stats.http_requests + stats.https_requests,
                        responses: stats.http_requests + stats.https_requests, // Assuming 1:1 for now
                        qps: stats.qps,
                        findings: total_findings as u64,
                    },
                );
            }
        }
    });

    // 保存服务实例
    *state.proxy_service.write().await = Some(proxy);
    *is_running = true;

    // 发射代理状态事件
    emit_proxy_status(
        &app,
        ProxyStatusEvent {
            running: true,
            port,
            mitm: true,
            stats: ProxyStats::default(),
        },
    );

    tracing::info!("Passive scan started on port {} (ScanPipeline running in dedicated thread)", port);
    Ok(CommandResponse::ok(port))
}

/// 停止被动扫描代理
#[tauri::command]
pub async fn stop_passive_scan(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    let mut is_running = state.is_running.write().await;
    if !*is_running {
        return Ok(CommandResponse::err("Proxy not running".to_string()));
    }

    let mut proxy = state.proxy_service.write().await;
    if let Some(p) = proxy.take() {
        if let Err(e) = p.stop().await {
            tracing::error!("Failed to stop proxy: {}", e);
            return Ok(CommandResponse::err(format!("Failed to stop proxy: {}", e)));
        }
    }

    *is_running = false;

    // 发射代理停止事件
    emit_proxy_status(
        &app,
        ProxyStatusEvent {
            running: false,
            port: 0,
            mitm: false,
            stats: ProxyStats::default(),
        },
    );

    tracing::info!("Passive scan stopped");
    Ok(CommandResponse::ok("Proxy stopped".to_string()))
}

/// 重载插件（在运行时热更新插件代码）
#[tauri::command]
pub async fn reload_plugin_in_pipeline(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<String>, String> {
    tracing::info!("Reload plugin request: {}", plugin_id);
    
    // 检查是否正在运行
    if !*state.is_running.read().await {
        return Ok(CommandResponse::err("被动扫描未运行".to_string()));
    }
    
    // 发送重载任务到 ScanPipeline
    if let Some(scan_tx) = state.scan_tx.read().await.as_ref() {
        if let Err(e) = scan_tx.send(sentinel_passive::ScanTask::ReloadPlugin(plugin_id.clone())) {
            tracing::error!("Failed to send reload task for plugin {}: {}", plugin_id, e);
            return Ok(CommandResponse::err(format!("发送重载任务失败: {}", e)));
        }
        
        tracing::info!("Sent reload task for plugin: {}", plugin_id);
        Ok(CommandResponse::ok(format!("插件 {} 重载任务已发送", plugin_id)))
    } else {
        Ok(CommandResponse::err("扫描通道不可用".to_string()))
    }
}

/// 获取代理状态
#[tauri::command]
pub async fn get_proxy_status(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<ProxyStatus>, String> {
    let is_running = *state.is_running.read().await;
    let proxy_opt = state.proxy_service.read().await;

    let status = if is_running {
        if let Some(proxy) = proxy_opt.as_ref() {
            let port = proxy.get_port().await.unwrap_or(0);
            let stats = proxy.get_stats().await;
            ProxyStatus {
                running: true,
                port,
                mitm_enabled: true,
                stats,
            }
        } else {
            ProxyStatus {
                running: false,
                port: 0,
                mitm_enabled: false,
                stats: ProxyStats::default(),
            }
        }
    } else {
        ProxyStatus {
            running: false,
            port: 0,
            mitm_enabled: false,
            stats: ProxyStats::default(),
        }
    };

    Ok(CommandResponse::ok(status))
}

/// 列出漏洞发现
#[tauri::command]
pub async fn list_findings(
    state: State<'_, PassiveScanState>,
    limit: Option<i64>,
    offset: Option<i64>,
    severity_filter: Option<String>,
) -> Result<CommandResponse<Vec<sentinel_passive::VulnerabilityWithEvidence>>, String> {
    let filters = VulnerabilityFilters {
        severity: severity_filter,
        limit: Some(limit.unwrap_or(10)), // 默认每页10条
        offset,
        ..Default::default()
    };

    let db_service = state.get_db_service().await?;
    match db_service.list_vulnerabilities_with_evidence(filters).await {
        Ok(records) => {
            tracing::info!("Loaded {} findings with evidence from database", records.len());
            Ok(CommandResponse::ok(records))
        }
        Err(e) => {
            tracing::error!("Failed to load findings: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 统计漏洞总数（用于分页）
#[tauri::command]
pub async fn count_findings(
    state: State<'_, PassiveScanState>,
    severity_filter: Option<String>,
) -> Result<CommandResponse<i64>, String> {
    let filters = VulnerabilityFilters {
        severity: severity_filter,
        ..Default::default()
    };

    let db_service = state.get_db_service().await?;
    match db_service.count_vulnerabilities(filters).await {
        Ok(count) => {
            tracing::info!("Total findings count: {}", count);
            Ok(CommandResponse::ok(count))
        }
        Err(e) => {
            tracing::error!("Failed to count findings: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

// （已移除）文件路径加载插件命令。插件仅从数据库读取。

/// 启用插件
#[tauri::command]
pub async fn enable_plugin(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    // DB-only：直接更新数据库中的插件状态
    let db = state.get_db_service().await?;
    
    // 获取插件的main_category
    let main_category: Option<String> = sqlx::query_scalar(
        "SELECT main_category FROM plugin_registry WHERE id = ?"
    )
    .bind(&plugin_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;
    
    let result = sqlx::query(
        "UPDATE plugin_registry SET enabled = ? WHERE id = ?"
    )
    .bind(true)
    .bind(&plugin_id)
    .execute(db.pool())
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                tracing::info!("Plugin enabled in database: {}", plugin_id);

                let plugin_name = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM plugin_registry WHERE id = ?"
                )
                .bind(&plugin_id)
                .fetch_optional(db.pool())
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| plugin_id.clone());

                emit_plugin_changed(
                    &app,
                    PluginChangedEvent {
                        plugin_id: plugin_id.clone(),
                        enabled: true,
                        name: plugin_name,
                    },
                );
                
                // 如果是Agent插件，刷新全局工具系统
                if let Some(cat) = main_category {
                    if cat == "agent" {
                        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                            if let Err(e) = tool_system.refresh_all().await {
                                tracing::warn!("Failed to refresh tool system after enabling agent plugin: {}", e);
                            } else {
                                tracing::info!("Tool system refreshed successfully after enabling agent plugin");
                            }
                        }
                    }
                }

                Ok(CommandResponse::ok(()))
            } else {
                Ok(CommandResponse::err(format!(
                    "Failed to enable plugin: Plugin not found: {}",
                    plugin_id
                )))
            }
        }
        Err(db_err) => Ok(CommandResponse::err(format!(
            "Failed to enable plugin (Database update error): {}",
            db_err
        ))),
    }
}

/// 禁用插件
#[tauri::command]
pub async fn disable_plugin(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    // DB-only：直接更新数据库中的插件状态
    let db = state.get_db_service().await?;
    
    // 获取插件的main_category
    let main_category: Option<String> = sqlx::query_scalar(
        "SELECT main_category FROM plugin_registry WHERE id = ?"
    )
    .bind(&plugin_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;
    
    let result = sqlx::query(
        "UPDATE plugin_registry SET enabled = ? WHERE id = ?"
    )
    .bind(false)
    .bind(&plugin_id)
    .execute(db.pool())
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                tracing::info!("Plugin disabled in database: {}", plugin_id);

                let plugin_name = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM plugin_registry WHERE id = ?"
                )
                .bind(&plugin_id)
                .fetch_optional(db.pool())
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| plugin_id.clone());

                emit_plugin_changed(
                    &app,
                    PluginChangedEvent {
                        plugin_id: plugin_id.clone(),
                        enabled: false,
                        name: plugin_name,
                    },
                );
                
                // 如果是Agent插件，刷新全局工具系统
                if let Some(cat) = main_category {
                    if cat == "agent" {
                        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                            if let Err(e) = tool_system.refresh_all().await {
                                tracing::warn!("Failed to refresh tool system after disabling agent plugin: {}", e);
                            } else {
                                tracing::info!("Tool system refreshed successfully after disabling agent plugin");
                            }
                        }
                    }
                }

                Ok(CommandResponse::ok(()))
            } else {
                Ok(CommandResponse::err(format!(
                    "Failed to disable plugin: Plugin not found: {}",
                    plugin_id
                )))
            }
        }
        Err(db_err) => Ok(CommandResponse::err(format!(
            "Failed to disable plugin (Database update error): {}",
            db_err
        ))),
    }
}

/// 列出所有插件（包含数据库中的插件）
#[tauri::command]
pub async fn list_plugins(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<Vec<PluginRecord>>, String> {
    let plugins = state.list_plugins_internal().await?;
    Ok(CommandResponse::ok(plugins))
}

// （已移除）扫描插件目录命令。插件仅从数据库读取。

// ============================================================================
// 证书管理命令
// ============================================================================

/// 下载 Root CA 证书（返回证书路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaCertPath {
    pub path: String,
}

/// 下载 CA 证书（前端友好接口）
#[tauri::command]
pub async fn download_ca_cert(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<CaCertPath>, String> {
    // 确保 CA 存在
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match state.certificate_service.export_root_ca() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("CA certificate available at: {}", path_str);
            Ok(CommandResponse::ok(CaCertPath { path: path_str }))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to get CA path: {}", e))),
    }
}

/// 获取 Root CA 证书路径（用于下载）
#[tauri::command]
pub async fn get_ca_cert_path(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    // 确保 CA 存在
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match state.certificate_service.export_root_ca() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("CA certificate path: {}", path_str);
            Ok(CommandResponse::ok(path_str))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to get CA path: {}", e))),
    }
}

/// 信任 Root CA 到系统 Keychain（仅 macOS）
#[tauri::command]
pub async fn trust_ca_cert(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    #[cfg(target_os = "macos")]
    {
        // 确保 CA 存在
        if let Err(e) = state.certificate_service.ensure_root_ca().await {
            return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
        }

        match state.certificate_service.trust_root_ca_macos().await {
            Ok(_) => {
                tracing::info!("CA certificate trusted in macOS Keychain");
                Ok(CommandResponse::ok(
                    "CA certificate successfully trusted in system Keychain".to_string(),
                ))
            }
            Err(e) => Ok(CommandResponse::err(format!(
                "Failed to trust CA: {}. You may need to run with administrator privileges.",
                e
            ))),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err(
            "Certificate trust is only supported on macOS".to_string(),
        ))
    }
}

/// 重新生成 CA 证书（删除旧的并生成新的）
#[tauri::command]
pub async fn regenerate_ca_cert(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    match state.certificate_service.regenerate_root_ca().await {
        Ok(_) => {
            let path = state
                .certificate_service
                .export_root_ca()
                .map_err(|e| format!("Failed to get CA path: {}", e))?;
            let path_str = path.to_string_lossy().to_string();
            
            tracing::info!("CA certificate regenerated at: {}", path_str);
            Ok(CommandResponse::ok(format!(
                "CA certificate regenerated successfully. Please re-import: {}",
                path_str
            )))
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to regenerate CA: {}",
            e
        ))),
    }
}

/// 获取 CA 证书指纹
#[tauri::command]
pub async fn get_ca_fingerprint(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    match state.certificate_service.get_certificate_fingerprint() {
        Ok(fingerprint) => {
            tracing::info!("CA certificate fingerprint: {}", fingerprint);
            Ok(CommandResponse::ok(fingerprint))
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get fingerprint: {}",
            e
        ))),
    }
}

// ============================================================
// 漏洞详情命令
// ============================================================

/// 漏洞详情（包含证据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDetail {
    /// 漏洞基本信息
    pub vulnerability: VulnerabilityRecord,
    /// 相关证据列表
    pub evidence: Vec<EvidenceRecord>,
}

/// 根据 ID 获取漏洞详情（包含所有证据）
#[tauri::command]
pub async fn get_finding(
    state: State<'_, PassiveScanState>,
    finding_id: String,
) -> Result<CommandResponse<Option<FindingDetail>>, String> {
    // 获取数据库服务
    let db_service = state.get_db_service().await?;

    // 查询漏洞
    let vulnerability = db_service
        .get_vulnerability_by_id(&finding_id)
        .await
        .map_err(|e| format!("Failed to fetch vulnerability: {}", e))?;

    if vulnerability.is_none() {
        return Ok(CommandResponse::ok(None));
    }

    let vulnerability = vulnerability.unwrap();

    // 查询相关证据
    let evidence = db_service
        .get_evidence_by_vuln_id(&finding_id)
        .await
        .map_err(|e| format!("Failed to fetch evidence: {}", e))?;

    let detail = FindingDetail {
        vulnerability,
        evidence,
    };

    tracing::debug!("Fetched finding detail: {} with {} evidence", finding_id, detail.evidence.len());

    Ok(CommandResponse::ok(Some(detail)))
}

/// 更新漏洞状态
#[tauri::command]
pub async fn update_finding_status(
    state: State<'_, PassiveScanState>,
    finding_id: String,
    status: String,
) -> Result<CommandResponse<String>, String> {
    // 验证状态值
    let valid_statuses = ["open", "reviewed", "false_positive", "fixed"];
    if !valid_statuses.contains(&status.as_str()) {
        return Ok(CommandResponse::err(format!(
            "Invalid status: {}. Must be one of: {}",
            status,
            valid_statuses.join(", ")
        )));
    }

    // 获取数据库服务
    let db_service = state.get_db_service().await?;

    // 更新状态
    db_service
        .update_vulnerability_status(&finding_id, &status)
        .await
        .map_err(|e| format!("Failed to update vulnerability status: {}", e))?;

    tracing::info!("Updated finding {} status to {}", finding_id, status);

    Ok(CommandResponse::ok(format!(
        "Finding {} status updated to {}",
        finding_id, status
    )))
}

/// HTML 报告数据结构
#[derive(Debug, Serialize)]
struct ReportSummary {
    total: usize,
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
    critical_percent: f64,
    high_percent: f64,
    medium_percent: f64,
    low_percent: f64,
    info_percent: f64,
}

#[derive(Debug, Serialize)]
struct ReportFinding {
    id: String,
    title: String,
    description: String,
    severity: String,
    vuln_type: String,
    plugin_id: String,
    url: String,
    method: String,
    location: String,
    evidence: String,
    confidence: String,
    cwe: Option<String>,
    owasp: Option<String>,
    remediation: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ReportData {
    report_title: String,
    generated_at: String,
    scan_scope: String,
    summary: ReportSummary,
    findings: Vec<ReportFinding>,
}

/// 导出 HTML 报告
#[tauri::command]
pub async fn export_findings_html(
    state: State<'_, PassiveScanState>,
    filters: Option<VulnerabilityFilters>,
) -> Result<CommandResponse<String>, String> {
    use tera::{Tera, Context};
    use std::fs;
    
    tracing::info!("Exporting HTML report with filters: {:?}", filters);
    
    // 获取数据库服务
    let db_service = state.get_db_service().await?;
    
    // 查询漏洞数据
    let filters = filters.unwrap_or_else(|| VulnerabilityFilters {
        vuln_type: None,
        severity: None,
        status: None,
        plugin_id: None,
        limit: Some(1000), // 默认最多导出1000条
        offset: Some(0),
    });
    
    let vulnerabilities = db_service
        .list_vulnerabilities(filters.clone())
        .await
        .map_err(|e| format!("Failed to list vulnerabilities: {}", e))?;
    
    // 统计数据
    let total = vulnerabilities.len();
    let critical = vulnerabilities.iter().filter(|v| v.severity == "critical").count();
    let high = vulnerabilities.iter().filter(|v| v.severity == "high").count();
    let medium = vulnerabilities.iter().filter(|v| v.severity == "medium").count();
    let low = vulnerabilities.iter().filter(|v| v.severity == "low").count();
    let info = vulnerabilities.iter().filter(|v| v.severity == "info").count();
    
    let total_f = total as f64;
    let summary = ReportSummary {
        total,
        critical,
        high,
        medium,
        low,
        info,
        critical_percent: if total > 0 { (critical as f64 / total_f) * 100.0 } else { 0.0 },
        high_percent: if total > 0 { (high as f64 / total_f) * 100.0 } else { 0.0 },
        medium_percent: if total > 0 { (medium as f64 / total_f) * 100.0 } else { 0.0 },
        low_percent: if total > 0 { (low as f64 / total_f) * 100.0 } else { 0.0 },
        info_percent: if total > 0 { (info as f64 / total_f) * 100.0 } else { 0.0 },
    };
    
    // 转换为报告格式（获取第一个 evidence）
    let mut findings: Vec<ReportFinding> = Vec::new();
    
    for v in vulnerabilities {
        // 尝试获取第一个证据
        let evidence_list = db_service
            .get_evidence_by_vuln_id(&v.id)
            .await
            .unwrap_or_default();
        
        let first_evidence = evidence_list.first();
        
        findings.push(ReportFinding {
            id: v.id.clone(),
            title: v.title.clone(),
            description: v.description.clone(),
            severity: v.severity.clone(),
            vuln_type: v.vuln_type.clone(),
            plugin_id: v.plugin_id.clone(),
            url: first_evidence.map(|e| e.url.clone()).unwrap_or_default(),
            method: first_evidence.map(|e| e.method.clone()).unwrap_or_default(),
            location: first_evidence.map(|e| e.location.clone()).unwrap_or_default(),
            evidence: first_evidence.map(|e| e.evidence_snippet.clone()).unwrap_or_default(),
            confidence: v.confidence.clone(),
            cwe: v.cwe.clone(),
            owasp: v.owasp.clone(),
            remediation: v.remediation.clone(),
            created_at: v.first_seen_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }
    
    // 准备模板数据
    let now = chrono::Utc::now();
    let report_data = ReportData {
        report_title: format!("被动扫描报告 - {}", now.format("%Y年%m月%d日")),
        generated_at: now.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        scan_scope: filters.plugin_id.clone()
            .map(|p| format!("插件: {}", p))
            .unwrap_or_else(|| "全部".to_string()),
        summary,
        findings,
    };
    
    // 加载模板
    let template_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("templates/vulnerability_report.html");
    
    if !template_path.exists() {
        return Err(format!("Template not found: {:?}", template_path));
    }
    
    let template_content = fs::read_to_string(&template_path)
        .map_err(|e| format!("Failed to read template: {}", e))?;
    
    // 渲染模板
    let mut tera = Tera::default();
    tera.add_raw_template("report", &template_content)
        .map_err(|e| format!("Failed to parse template: {}", e))?;
    
    let mut context = Context::new();
    context.insert("report_title", &report_data.report_title);
    context.insert("generated_at", &report_data.generated_at);
    context.insert("scan_scope", &report_data.scan_scope);
    context.insert("summary", &report_data.summary);
    context.insert("findings", &report_data.findings);
    
    let html = tera.render("report", &context)
        .map_err(|e| format!("Failed to render template: {}", e))?;
    
    // 保存报告
    let output_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".sentinel-ai")
        .join("reports");
    
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    let filename = format!("passive_scan_report_{}.html", now.format("%Y%m%d_%H%M%S"));
    let output_path = output_dir.join(&filename);
    
    fs::write(&output_path, html)
        .map_err(|e| format!("Failed to write report: {}", e))?;
    
    let path_str = output_path.to_string_lossy().to_string();
    tracing::info!("HTML report exported to: {}", path_str);
    
    Ok(CommandResponse::ok(path_str))
}

// ============================================================
// 代理请求历史相关命令
// ============================================================

/// 列出代理请求历史
#[tauri::command]
pub async fn list_proxy_requests(
    state: State<'_, PassiveScanState>,
    limit: Option<i64>,
    offset: Option<i64>,
    protocol: Option<String>,
    method: Option<String>,
    host: Option<String>,
    status_code_min: Option<i32>,
    status_code_max: Option<i32>,
) -> Result<CommandResponse<Vec<sentinel_passive::ProxyRequestRecord>>, String> {
    let db = state.get_db_service().await?;
    
    let filters = sentinel_passive::ProxyRequestFilters {
        protocol,
        method,
        host,
        status_code_min,
        status_code_max,
        limit,
        offset,
    };
    
    let requests = db
        .list_proxy_requests(filters)
        .await
        .map_err(|e| format!("Failed to list proxy requests: {}", e))?;
    
    Ok(CommandResponse::ok(requests))
}

/// 获取代理请求详情
#[tauri::command]
pub async fn get_proxy_request(
    state: State<'_, PassiveScanState>,
    id: i64,
) -> Result<CommandResponse<Option<sentinel_passive::ProxyRequestRecord>>, String> {
    let db = state.get_db_service().await?;
    
    let request = db
        .get_proxy_request_by_id(id)
        .await
        .map_err(|e| format!("Failed to get proxy request: {}", e))?;
    
    Ok(CommandResponse::ok(request))
}

/// 清空代理请求历史
#[tauri::command]
pub async fn clear_proxy_requests(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<u64>, String> {
    let db = state.get_db_service().await?;
    
    let count = db
        .clear_proxy_requests()
        .await
        .map_err(|e| format!("Failed to clear proxy requests: {}", e))?;
    
    Ok(CommandResponse::ok(count))
}

/// 统计代理请求
#[tauri::command]
pub async fn count_proxy_requests(
    state: State<'_, PassiveScanState>,
    protocol: Option<String>,
    method: Option<String>,
    host: Option<String>,
    status_code_min: Option<i32>,
    status_code_max: Option<i32>,
) -> Result<CommandResponse<i64>, String> {
    let db = state.get_db_service().await?;
    
    let filters = sentinel_passive::ProxyRequestFilters {
        protocol,
        method,
        host,
        status_code_min,
        status_code_max,
        limit: None,
        offset: None,
    };
    
    let count = db
        .count_proxy_requests(filters)
        .await
        .map_err(|e| format!("Failed to count proxy requests: {}", e))?;
    
    Ok(CommandResponse::ok(count))
}

// ============================================================
// 插件数据库操作命令
// ============================================================

/// 在数据库中创建插件（存储代码）
#[tauri::command]
pub async fn create_plugin_in_db(
    state: State<'_, PassiveScanState>,
    metadata: serde_json::Value,
    plugin_code: String,
) -> Result<CommandResponse<String>, String> {
    let db = state.get_db_service().await?;
    
    // 解析元数据
    let plugin: sentinel_passive::PluginMetadata = serde_json::from_value(metadata)
        .map_err(|e| format!("Invalid plugin metadata: {}", e))?;
    
    let plugin_id = plugin.id.clone();
    let main_category = plugin.main_category.clone();
    
    // 注册插件到数据库
    db.register_plugin_with_code(&plugin, &plugin_code)
        .await
        .map_err(|e| format!("Failed to create plugin in database: {}", e))?;
    
    tracing::info!("Plugin created in database: {}", plugin_id);
    
    // 如果是Agent插件，刷新全局工具系统
    if main_category == "agent" {
        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
            if let Err(e) = tool_system.refresh_all().await {
                tracing::warn!("Failed to refresh tool system after creating agent plugin: {}", e);
            } else {
                tracing::info!("Tool system refreshed successfully after creating agent plugin");
            }
        }
    }
    
    Ok(CommandResponse::ok(plugin_id))
}

/// 更新插件代码
#[tauri::command]
pub async fn update_plugin_code(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
    plugin_code: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;
    
    // 获取插件的main_category
    let main_category: Option<String> = sqlx::query_scalar(
        "SELECT main_category FROM plugin_registry WHERE id = ?"
    )
    .bind(&plugin_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;
    
    db.update_plugin_code(&plugin_id, &plugin_code)
        .await
        .map_err(|e| format!("Failed to update plugin code: {}", e))?;
    
    tracing::info!("Plugin code updated: {}", plugin_id);
    
    // 如果是Agent插件，刷新全局工具系统
    if let Some(cat) = main_category {
        if cat == "agent" {
            if let Ok(tool_system) = crate::tools::get_global_tool_system() {
                if let Err(e) = tool_system.refresh_all().await {
                    tracing::warn!("Failed to refresh tool system after updating agent plugin: {}", e);
                } else {
                    tracing::info!("Tool system refreshed successfully after updating agent plugin");
                }
            }
        }
    }
    
    Ok(CommandResponse::ok(()))
}

/// 获取插件代码
#[tauri::command]
pub async fn get_plugin_code(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<Option<String>>, String> {
    let db = state.get_db_service().await?;
    
    let code = db
        .get_plugin_code(&plugin_id)
        .await
        .map_err(|e| format!("Failed to get plugin code: {}", e))?;
    
    Ok(CommandResponse::ok(code))
}

/// 获取完整插件信息（包含代码）
#[tauri::command]
pub async fn get_plugin_by_id(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<Option<serde_json::Value>>, String> {
    let db = state.get_db_service().await?;
    
    let plugin = db
        .get_plugin_by_id(&plugin_id)
        .await
        .map_err(|e| format!("Failed to get plugin: {}", e))?;
    
    Ok(CommandResponse::ok(plugin))
}

/// 测试插件
#[tauri::command]
pub async fn test_plugin(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<TestPluginResult>, String> {
    // 改进：真正执行插件一次（构造一个最小的模拟请求上下文），返回真实的 findings（不再只返回元数据）
    // 如果插件未启用或不存在，保持原有提示逻辑。
    let db = state.get_db_service().await?;

    let row: Option<(
        String, String, String, Option<String>, String, Option<String>,
        String, Option<String>, bool
    )> = sqlx::query_as(
        r#"
        SELECT id, name, version, author, category, description,
               default_severity, tags, enabled
        FROM plugin_registry
        WHERE id = ?
        "#
    )
    .bind(&plugin_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Failed to query plugin: {}", e))?;

    if let Some((id, name, version, author, category, description, _sev, _tags, enabled)) = row {
        if !enabled {
            return Ok(CommandResponse::ok(TestPluginResult {
                success: false,
                message: Some(format!(
                    "插件 '{}' 当前未启用。请先启用插件。",
                    name
                )),
                findings: None,
                error: Some("Plugin is not enabled".to_string()),
            }));
        }

        // 获取插件管理器（确保内存中已注册插件，否则主动加载）
        let plugin_manager = state.get_plugin_manager();
        {
            // 如果内存中尚未注册插件元数据或代码，尝试从数据库加载
            if plugin_manager.get_plugin(&plugin_id).await.is_none() {
                // 加载代码
                let code_opt = sqlx::query_scalar::<_, Option<String>>("SELECT plugin_code FROM plugin_registry WHERE id = ?")
                    .bind(&plugin_id)
                    .fetch_optional(db.pool())
                    .await
                    .map_err(|e| format!("Failed to load plugin code: {}", e))?;
                if let Some(code) = code_opt.flatten() {
                    // 构造 PluginMetadata 供注册（保持与 list_plugins_internal 构建一致）
                    let tags_json: Option<String> = sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = ?")
                        .bind(&plugin_id)
                        .fetch_optional(db.pool())
                        .await
                        .map_err(|e| format!("Failed to query plugin tags: {}", e))?;
                    let tags: Vec<String> = tags_json
                        .and_then(|t| serde_json::from_str(&t).ok())
                        .unwrap_or_default();
                    let (name, version, author, main_category, category, description, default_severity) = sqlx::query_as::<_, (String,String,Option<String>,String,String,Option<String>,String)>(
                        "SELECT name, version, author, main_category, category, description, default_severity FROM plugin_registry WHERE id = ?"
                    )
                    .bind(&plugin_id)
                    .fetch_optional(db.pool())
                    .await
                    .map_err(|e| format!("Failed to query plugin metadata: {}", e))?
                    .ok_or_else(|| format!("Plugin metadata not found for id {}", plugin_id))?;
                    let severity = match default_severity.to_lowercase().as_str() {
                        "critical" => sentinel_passive::types::Severity::Critical,
                        "high" => sentinel_passive::types::Severity::High,
                        "medium" => sentinel_passive::types::Severity::Medium,
                        "low" => sentinel_passive::types::Severity::Low,
                        "info" => sentinel_passive::types::Severity::Info,
                        _ => sentinel_passive::types::Severity::Medium,
                    };
                    let metadata = PluginMetadata {
                        id: plugin_id.clone(),
                        name,
                        version,
                        author,
                        main_category,
                        category,
                        description,
                        default_severity: severity,
                        tags,
                    };
                    // 注册并缓存代码（忽略可能的并发注册错误）
                    let _ = plugin_manager.register_plugin(plugin_id.clone(), metadata, enabled).await;
                    let _ = plugin_manager.set_plugin_code(plugin_id.clone(), code).await;
                }
            }
        }

        // 构造一个最小的 RequestContext，供插件执行。保持与实际被动扫描结构一致。
        use sentinel_passive::RequestContext;
        let mut headers = std::collections::HashMap::new();
        headers.insert("User-Agent".to_string(), "Sentinel-Test/1.0".to_string());

        let request_ctx = RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers,
            body: vec![],
            content_type: Some("text/plain".to_string()),
            query_params: std::collections::HashMap::new(),
            is_https: true,
            timestamp: chrono::Utc::now(),
        };

        // 调用插件进行一次扫描，捕获真实 findings。
        let findings_result = plugin_manager.scan_request(&plugin_id, &request_ctx).await;

        match findings_result {
            Ok(foundings) => {
                let mapped: Vec<TestFinding> = foundings
                    .into_iter()
                    .map(|f| TestFinding {
                        title: f.title,
                        description: f.description,
                        severity: f.severity.to_string(),
                    })
                    .collect();

                return Ok(CommandResponse::ok(TestPluginResult {
                    success: true,
                    message: Some(format!(
                        "插件 '{}' (v{}) 执行测试完成。发现数量: {}。",
                        name,
                        version,
                        mapped.len()
                    )),
                    findings: Some(mapped),
                    error: None,
                }));
            }
            Err(e) => {
                return Ok(CommandResponse::ok(TestPluginResult {
                    success: false,
                    message: Some(format!("插件执行失败: {}", e)),
                    findings: None,
                    error: Some("ExecutionError".to_string()),
                }));
            }
        }
    }

    Ok(CommandResponse::err(format!("Plugin not found: {}", plugin_id)))
}

/// 测试插件结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPluginResult {
    pub success: bool,
    pub message: Option<String>,
    pub findings: Option<Vec<TestFinding>>,
    pub error: Option<String>,
}

/// 测试发现的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFinding {
    pub title: String,
    pub description: String,
    pub severity: String,
}

/// 高级测试单次运行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRunStat {
    pub run_index: u32,
    pub duration_ms: u128,
    pub findings: usize,
    pub error: Option<String>,
}

/// 高级测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedTestResult {
    pub plugin_id: String,
    pub success: bool,
    pub total_runs: u32,
    pub concurrency: u32,
    pub total_duration_ms: u128,
    pub avg_duration_ms: f64,
    pub total_findings: usize,
    pub unique_findings: usize,
    pub findings: Vec<TestFinding>,
    pub runs: Vec<AdvancedRunStat>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// 高级并发测试插件：可自定义请求参数 + 并发 + 重复运行
#[tauri::command]
pub async fn test_plugin_advanced(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
    url: Option<String>,
    method: Option<String>,
    headers: Option<String>, // JSON 字符串 {"Key":"Value"}
    body: Option<String>,
    runs: Option<u32>,
    concurrency: Option<u32>,
) -> Result<CommandResponse<AdvancedTestResult>, String> {
    let runs = runs.unwrap_or(1).max(1);
    let concurrency = concurrency.unwrap_or(1).max(1);
    let db = state.get_db_service().await?;
    // 复用 test_plugin 的加载逻辑：确保插件已注册
    let _ = test_plugin(state.clone(), plugin_id.clone()).await; // 忽略结果，只用于触发注册（若未启用会返回提示）

    // 再次确认是否启用
    let enabled: Option<bool> = sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id = ?")
        .bind(&plugin_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Failed to query plugin enabled: {}", e))?;
    if enabled != Some(true) {
        return Ok(CommandResponse::ok(AdvancedTestResult {
            plugin_id,
            success: false,
            total_runs: runs,
            concurrency,
            total_duration_ms: 0,
            avg_duration_ms: 0.0,
            total_findings: 0,
            unique_findings: 0,
            findings: vec![],
            runs: vec![],
            message: Some("插件未启用或不存在".to_string()),
            error: Some("PluginDisabled".to_string()),
        }));
    }

    use sentinel_passive::RequestContext;
    let plugin_manager = state.get_plugin_manager();

    // 解析 headers JSON
    let parsed_headers: std::collections::HashMap<String, String> = headers
        .and_then(|h| serde_json::from_str(&h).ok())
        .unwrap_or_else(|| {
            let mut m = std::collections::HashMap::new();
            m.insert("User-Agent".to_string(), "Sentinel-AdvTest/1.0".to_string());
            m
        });

    let req_url = url.unwrap_or_else(|| "https://example.com/test".to_string());
    let req_method = method.unwrap_or_else(|| "GET".to_string());
    let body_bytes = body.map(|b| b.into_bytes()).unwrap_or_default();

    // 准备任务列表
    let mut indices: Vec<u32> = (0..runs).collect();
    let start_all = std::time::Instant::now();
    let mut run_stats: Vec<AdvancedRunStat> = Vec::with_capacity(runs as usize);
    let mut all_findings: Vec<TestFinding> = Vec::new();

    // futures::stream 并发执行
    use futures::{stream, StreamExt};
    let results = stream::iter(indices.into_iter())
        .map(|i| {
            let plugin_manager = plugin_manager.clone();
            let plugin_id = plugin_id.clone();
            let headers_map = parsed_headers.clone();
            let req_url = req_url.clone();
            let req_method = req_method.clone();
            let body_bytes = body_bytes.clone();
            async move {
                let run_start = std::time::Instant::now();
                let ctx = RequestContext {
                    id: uuid::Uuid::new_v4().to_string(),
                    method: req_method.clone(),
                    url: req_url.clone(),
                    headers: headers_map.clone(),
                    body: body_bytes.clone(),
                    content_type: None,
                    query_params: std::collections::HashMap::new(),
                    is_https: req_url.starts_with("https://"),
                    timestamp: chrono::Utc::now(),
                };
                match plugin_manager.scan_request(&plugin_id, &ctx).await {
                    Ok(foundings) => {
                        let mapped: Vec<TestFinding> = foundings.into_iter().map(|f| TestFinding { title: f.title, description: f.description, severity: f.severity.to_string() }).collect();
                        let dur = run_start.elapsed().as_millis();
                        (i, Ok((dur, mapped)))
                    }
                    Err(e) => {
                        let dur = run_start.elapsed().as_millis();
                        (i, Err((dur, e.to_string())))
                    }
                }
            }
        })
        .buffer_unordered(concurrency as usize)
        .collect::<Vec<(u32, Result<(u128, Vec<TestFinding>), (u128, String)>)>>()
        .await;

    for (idx, res) in results.into_iter() {
        match res {
            Ok((dur, findings)) => {
                run_stats.push(AdvancedRunStat { run_index: idx, duration_ms: dur, findings: findings.len(), error: None });
                all_findings.extend(findings);
            }
            Err((dur, err)) => {
                run_stats.push(AdvancedRunStat { run_index: idx, duration_ms: dur, findings: 0, error: Some(err) });
            }
        }
    }

    let total_duration_ms = start_all.elapsed().as_millis();
    let avg_duration_ms = if run_stats.is_empty() { 0.0 } else { (run_stats.iter().map(|r| r.duration_ms).sum::<u128>() as f64) / (run_stats.len() as f64) };
    // 去重唯一发现（按 title+severity+description）
    use std::collections::HashSet;
    let mut uniq = HashSet::new();
    let mut unique_list: Vec<TestFinding> = Vec::new();
    for f in &all_findings {
        let key = format!("{}|{}|{}", f.title, f.severity, f.description);
        if uniq.insert(key) {
            unique_list.push(f.clone());
        }
    }

    Ok(CommandResponse::ok(AdvancedTestResult {
        plugin_id,
        success: run_stats.iter().all(|r| r.error.is_none()),
        total_runs: runs,
        concurrency,
        total_duration_ms,
        avg_duration_ms,
        total_findings: all_findings.len(),
        unique_findings: unique_list.len(),
        findings: unique_list,
        runs: run_stats,
        message: Some("高级测试完成".to_string()),
        error: None,
    }))
}

/// 删除插件
#[tauri::command]
pub async fn delete_plugin(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;
    
    // 先禁用插件
    db.update_plugin_enabled(&plugin_id, false)
        .await
        .map_err(|e| format!("Failed to disable plugin before deletion: {}", e))?;
    
    // 删除插件
    db.delete_plugin(&plugin_id)
        .await
        .map_err(|e| format!("Failed to delete plugin: {}", e))?;
    
    tracing::info!("Plugin deleted: {}", plugin_id);
    Ok(CommandResponse::ok(()))
}

/// 删除单个被动扫描发现的漏洞
#[tauri::command]
pub async fn delete_passive_vulnerability(
    state: State<'_, PassiveScanState>,
    vuln_id: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;
    
    db.delete_vulnerability(&vuln_id)
        .await
        .map_err(|e| format!("Failed to delete vulnerability: {}", e))?;
    
    tracing::info!("Vulnerability deleted: {}", vuln_id);
    Ok(CommandResponse::ok(()))
}

/// 批量删除被动扫描发现的漏洞
#[tauri::command]
pub async fn delete_passive_vulnerabilities_batch(
    state: State<'_, PassiveScanState>,
    vuln_ids: Vec<String>,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;
    
    let mut deleted_count = 0;
    for vuln_id in &vuln_ids {
        match db.delete_vulnerability(vuln_id).await {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                tracing::warn!("Failed to delete vulnerability {}: {}", vuln_id, e);
            }
        }
    }
    
    tracing::info!("Batch deleted {} vulnerabilities out of {}", deleted_count, vuln_ids.len());
    Ok(CommandResponse::ok(()))
}

/// 删除所有被动扫描发现的漏洞
#[tauri::command]
pub async fn delete_all_passive_vulnerabilities(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;
    
    db.delete_all_vulnerabilities()
        .await
        .map_err(|e| format!("Failed to delete all vulnerabilities: {}", e))?;
    
    tracing::info!("All vulnerabilities deleted");
    Ok(CommandResponse::ok(()))
}

/// 代理监听器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyListenerConfig {
    pub host: String,
    pub port: u16,
    pub mitm_enabled: bool,
}

/// 启动代理监听器
#[tauri::command]
pub async fn start_proxy_listener(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    host: String,
    port: u16,
    index: usize,
) -> Result<CommandResponse<String>, String> {
    tracing::info!("Starting proxy listener on {}:{} (index: {})", host, port, index);
    
    // 检查代理是否已经在运行
    let is_running = *state.is_running.read().await;
    if is_running {
        // 如果代理已经在运行，直接返回成功
        tracing::info!("Proxy already running, listener request acknowledged");
        return Ok(CommandResponse::ok(format!("Listener {}:{} is already running", host, port)));
    }
    
    // 创建代理配置
    let config = ProxyConfig {
        start_port: port,
        max_port_attempts: 1,
        mitm_enabled: true,
        max_request_body_size: 2 * 1024 * 1024,
        max_response_body_size: 2 * 1024 * 1024,
        mitm_bypass_fail_threshold: 3,
    };
    
    // 调用启动代理的命令
    match start_passive_scan(app, state, Some(config)).await {
        Ok(response) => {
            if response.success {
                tracing::info!("Proxy listener started successfully on {}:{}", host, port);
                Ok(CommandResponse::ok(format!("Listener started on {}:{}", host, port)))
            } else {
                let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                Ok(CommandResponse::err(error_msg))
            }
        }
        Err(e) => {
            tracing::error!("Failed to start proxy listener: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// 停止代理监听器
#[tauri::command]
pub async fn stop_proxy_listener(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    index: usize,
) -> Result<CommandResponse<String>, String> {
    tracing::info!("Stopping proxy listener at index: {}", index);
    
    // 调用停止代理的命令
    match stop_passive_scan(app, state).await {
        Ok(response) => {
            if response.success {
                tracing::info!("Proxy listener stopped successfully");
                Ok(CommandResponse::ok("Listener stopped".to_string()))
            } else {
                let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                Ok(CommandResponse::err(error_msg))
            }
        }
        Err(e) => {
            tracing::error!("Failed to stop proxy listener: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

/// 保存代理配置
#[tauri::command]
pub async fn save_proxy_config(
    state: State<'_, PassiveScanState>,
    config: ProxyConfig,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Saving proxy configuration: {:?}", config);
    
    // 这里可以将配置保存到数据库或文件
    // 目前简单返回成功
    // TODO: 实现配置持久化
    
    Ok(CommandResponse::ok(()))
}

/// 获取代理配置
#[tauri::command]
pub async fn get_proxy_config(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<ProxyConfig>, String> {
    tracing::info!("Getting proxy configuration");
    
    // 返回默认配置
    // TODO: 从数据库或文件加载配置
    let config = ProxyConfig {
        start_port: 8080,
        max_port_attempts: 10,
        mitm_enabled: true,
        max_request_body_size: 2 * 1024 * 1024,
        max_response_body_size: 2 * 1024 * 1024,
        mitm_bypass_fail_threshold: 3,
    };
    
    Ok(CommandResponse::ok(config))
}
