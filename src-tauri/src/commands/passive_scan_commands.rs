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

use sentinel_plugins::HttpTransaction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use sentinel_passive::{
    CertificateService, EvidenceRecord, Finding, FindingDeduplicator,
    InterceptAction as PassiveInterceptAction, InterceptFilterRule as PassiveInterceptFilterRule,
    InterceptState, PassiveDatabaseService, PendingInterceptRequest, PendingInterceptResponse,
    PendingInterceptWebSocketMessage, PluginManager, PluginMetadata, PluginRecord, PluginStatus,
    ProxyConfig, ProxyService, ProxyStats, ProxyStatus, ScanPipeline, ScanTask,
    VulnerabilityFilters, VulnerabilityRecord,
};

use crate::events::{
    emit_finding, emit_intercept_request, emit_intercept_response, emit_plugin_changed,
    emit_proxy_status, emit_scan_stats,
};
use crate::events::{
    FindingEvent, InterceptRequestEvent, InterceptResponseEvent, PluginChangedEvent,
    ProxyStatusEvent, ScanStatsEvent,
};

/// 拦截请求（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    pub id: String,
    pub method: String,
    pub url: String,
    pub path: String,
    pub protocol: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

/// 本地拦截动作（用于命令参数）
pub type InterceptAction = PassiveInterceptAction;

/// 内部拦截请求（包含响应通道）
pub struct InterceptedRequestInternal {
    pub request: InterceptedRequest,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
}

/// 拦截响应（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedResponse {
    pub id: String,
    pub request_id: String,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

/// 内部拦截响应（包含响应通道）
pub struct InterceptedResponseInternal {
    pub response: InterceptedResponse,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
}

/// 被动扫描服务状态（全局单例）
pub struct PassiveScanState {
    proxy_service: Arc<RwLock<Option<ProxyService>>>,
    plugin_manager: Arc<PluginManager>,
    certificate_service: Arc<CertificateService>,
    db_service: std::sync::OnceLock<Arc<PassiveDatabaseService>>,
    database_url: String,
    is_running: Arc<RwLock<bool>>,
    scan_tx: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<sentinel_passive::ScanTask>>>>,
    /// 是否启用请求拦截
    intercept_enabled: Arc<RwLock<bool>>,
    /// 是否启用响应拦截
    response_intercept_enabled: Arc<RwLock<bool>>,
    /// 待处理的拦截请求（内部使用，包含响应通道）
    intercepted_requests:
        Arc<RwLock<std::collections::HashMap<String, InterceptedRequestInternal>>>,
    /// 待处理的拦截响应（内部使用，包含响应通道）
    intercepted_responses:
        Arc<RwLock<std::collections::HashMap<String, InterceptedResponseInternal>>>,
    /// 应用句柄（用于发送事件）
    app_handle: Arc<RwLock<Option<AppHandle>>>,
    /// 拦截请求接收端（用于处理从代理发来的拦截请求）
    intercept_pending_tx:
        Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptRequest>>>>,
    /// 待处理的拦截响应发送端（用于传递给 start_proxy）
    pub intercept_response_pending_tx:
        Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptResponse>>>>,
    /// 是否启用 WebSocket 拦截
    pub websocket_intercept_enabled: Arc<RwLock<bool>>,
    /// 待处理的拦截 WebSocket 消息（等待用户操作）
    pub intercepted_websocket_messages:
        Arc<RwLock<std::collections::HashMap<String, InterceptedWebSocketMessageInternal>>>,
    /// 待处理的 WebSocket 拦截消息发送端
    pub intercept_websocket_pending_tx:
        Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptWebSocketMessage>>>>,
    /// 历史记录缓存
    pub history_cache: Arc<sentinel_passive::ProxyHistoryCache>,
    /// 请求拦截过滤规则
    pub request_filter_rules: Arc<RwLock<Vec<PassiveInterceptFilterRule>>>,
    /// 响应拦截过滤规则
    pub response_filter_rules: Arc<RwLock<Vec<PassiveInterceptFilterRule>>>,
}

/// 内部使用的拦截 WebSocket 消息结构（包含响应通道）
struct InterceptedWebSocketMessageInternal {
    pub id: String,
    pub connection_id: String,
    pub direction: sentinel_passive::ProxyWebSocketDirection,
    pub message_type: String,
    pub content: Option<String>,
    pub timestamp: i64,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
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
            intercept_enabled: self.intercept_enabled.clone(),
            response_intercept_enabled: self.response_intercept_enabled.clone(),
            intercepted_requests: self.intercepted_requests.clone(),
            intercepted_responses: self.intercepted_responses.clone(),
            app_handle: self.app_handle.clone(),
            intercept_pending_tx: self.intercept_pending_tx.clone(),
            intercept_response_pending_tx: self.intercept_response_pending_tx.clone(),
            websocket_intercept_enabled: self.websocket_intercept_enabled.clone(),
            intercepted_websocket_messages: self.intercepted_websocket_messages.clone(),
            intercept_websocket_pending_tx: self.intercept_websocket_pending_tx.clone(),
            history_cache: self.history_cache.clone(),
            request_filter_rules: self.request_filter_rules.clone(),
            response_filter_rules: self.response_filter_rules.clone(),
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
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sentinel-ai");

        // 证书目录固定在用户数据目录下的 ca 子目录
        let ca_dir = app_data_dir.join("ca");

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
            intercept_enabled: Arc::new(RwLock::new(false)),
            response_intercept_enabled: Arc::new(RwLock::new(false)),
            intercepted_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            intercepted_responses: Arc::new(RwLock::new(std::collections::HashMap::new())),
            app_handle: Arc::new(RwLock::new(None)),
            intercept_pending_tx: Arc::new(RwLock::new(None)),
            intercept_response_pending_tx: Arc::new(RwLock::new(None)),
            websocket_intercept_enabled: Arc::new(RwLock::new(false)),
            intercepted_websocket_messages: Arc::new(RwLock::new(std::collections::HashMap::new())),
            intercept_websocket_pending_tx: Arc::new(RwLock::new(None)),
            history_cache: Arc::new(sentinel_passive::ProxyHistoryCache::with_defaults()),
            request_filter_rules: Arc::new(RwLock::new(Vec::new())),
            response_filter_rules: Arc::new(RwLock::new(Vec::new())),
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

    /// 公开方法：获取运行状态（用于工具提供者）
    pub fn get_is_running(&self) -> Arc<RwLock<bool>> {
        self.is_running.clone()
    }

    /// 公开方法：获取代理服务（用于工具提供者）
    pub fn get_proxy_service(&self) -> Arc<RwLock<Option<ProxyService>>> {
        self.proxy_service.clone()
    }

    /// 公开方法：获取代理历史记录缓存（用于工具提供者和命令）
    pub fn get_history_cache(&self) -> Arc<sentinel_passive::ProxyHistoryCache> {
        self.history_cache.clone()
    }

    /// Public method: Get scan_tx (for tool providers)
    pub fn get_scan_tx(&self) -> Arc<RwLock<Option<UnboundedSender<ScanTask>>>> {
        self.scan_tx.clone()
    }

    /// Public method: Set scan_tx (for tool providers)
    pub async fn set_scan_tx(&self, tx: UnboundedSender<ScanTask>) {
        let mut scan_tx_guard = self.scan_tx.write().await;
        *scan_tx_guard = Some(tx);
    }

    /// 获取拦截启用状态
    pub fn get_intercept_enabled(&self) -> Arc<RwLock<bool>> {
        self.intercept_enabled.clone()
    }

    /// 获取拦截请求映射
    pub fn get_intercepted_requests(
        &self,
    ) -> Arc<RwLock<std::collections::HashMap<String, InterceptedRequestInternal>>> {
        self.intercepted_requests.clone()
    }

    /// 设置应用句柄
    pub async fn set_app_handle(&self, app: AppHandle) {
        let mut handle = self.app_handle.write().await;
        *handle = Some(app);
    }

    /// 获取应用句柄
    pub fn get_app_handle(&self) -> Arc<RwLock<Option<AppHandle>>> {
        self.app_handle.clone()
    }

    /// 获取当前运行的代理地址 (host:port)
    /// 返回格式: "http://127.0.0.1:8081"
    pub async fn get_running_proxy_address(&self) -> Option<String> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return None;
        }

        let proxy_opt = self.proxy_service.read().await;
        if let Some(proxy) = proxy_opt.as_ref() {
            if let Some(port) = proxy.get_port().await {
                // 代理默认监听在 127.0.0.1
                return Some(format!("http://127.0.0.1:{}", port));
            }
        }
        None
    }

    /// 内部方法：列出所有插件（数据库来源，供工具提供者使用）
    /// 只返回已批准（Approved）的插件，待审核和已拒绝的插件不显示
    pub async fn list_plugins_internal(&self) -> Result<Vec<PluginRecord>, String> {
        let db = self.get_db_service().await.map_err(|e| e.to_string())?;
        // 查询数据库中所有插件（包含 main_category 和收藏状态）
        // 只查询已批准的插件：validation_status = 'Approved'
        let rows = sqlx::query_as::<
            _,
            (
                String,         // id
                String,         // name
                String,         // version
                Option<String>, // author
                String,         // main_category
                String,         // category
                Option<String>, // description
                String,         // default_severity
                Option<String>, // tags (JSON)
                bool,           // enabled
                Option<String>, // plugin_code
                i64,            // is_favorited (0 or 1)
            ),
        >(
            r#"
            SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                   p.default_severity, p.tags, p.enabled, p.plugin_code,
                   CASE WHEN f.plugin_id IS NOT NULL THEN 1 ELSE 0 END as is_favorited
            FROM plugin_registry p
            LEFT JOIN plugin_favorites f ON p.id = f.plugin_id AND f.user_id = 'default'
            WHERE p.validation_status = 'Approved'
            "#,
        )
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Failed to query database plugins: {}", e))?;

        let mut records = Vec::new();
        for (
            id,
            name,
            version,
            author,
            main_category,
            category,
            description,
            default_severity,
            tags,
            enabled,
            _plugin_code,
            is_favorited,
        ) in rows
        {
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

            let status = if enabled {
                PluginStatus::Enabled
            } else {
                PluginStatus::Disabled
            };

            records.push(PluginRecord {
                metadata,
                path: None, // 插件存储在数据库中，不再使用文件路径
                status,
                last_error: None,
                is_favorited: is_favorited == 1,
            });
        }

        Ok(records)
    }

    /// 执行Agent插件（用于工作流）
    pub async fn execute_agent_plugin(
        &self,
        plugin_id: &str,
        inputs: &serde_json::Value,
    ) -> Result<
        (
            Vec<sentinel_passive::types::Finding>,
            Option<serde_json::Value>,
        ),
        String,
    > {
        self.plugin_manager
            .execute_agent(plugin_id, inputs)
            .await
            .map_err(|e| format!("Failed to execute agent plugin '{}': {}", plugin_id, e))
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

/// 内部启动函数（可在内部和外部复用）
pub async fn start_passive_scan_internal(
    app: &AppHandle,
    state: &PassiveScanState,
    config: Option<ProxyConfig>,
) -> Result<u16, String> {
    let mut is_running = state.is_running.write().await;
    if *is_running {
        return Err("Proxy already running".to_string());
    }

    let config = config.unwrap_or_default();

    // 证书目录固定在用户数据目录下
    let ca_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sentinel-ai")
        .join("ca");

    // 创建请求拦截通道
    let (intercept_pending_tx, mut intercept_pending_rx) =
        tokio::sync::mpsc::unbounded_channel::<PendingInterceptRequest>();

    // 创建响应拦截通道
    let (intercept_response_pending_tx, mut intercept_response_pending_rx) =
        tokio::sync::mpsc::unbounded_channel::<PendingInterceptResponse>();

    // 创建 WebSocket 拦截通道
    let (intercept_websocket_pending_tx, mut intercept_websocket_pending_rx) =
        tokio::sync::mpsc::unbounded_channel::<sentinel_passive::PendingInterceptWebSocketMessage>(
        );

    // 保存拦截发送端到 state
    {
        let mut tx_guard = state.intercept_pending_tx.write().await;
        *tx_guard = Some(intercept_pending_tx.clone());
    }
    {
        let mut tx_guard = state.intercept_response_pending_tx.write().await;
        *tx_guard = Some(intercept_response_pending_tx.clone());
    }
    {
        let mut tx_guard = state.intercept_websocket_pending_tx.write().await;
        *tx_guard = Some(intercept_websocket_pending_tx.clone());
    }

    // 创建拦截状态
    let intercept_state = InterceptState {
        enabled: state.intercept_enabled.clone(),
        response_enabled: state.response_intercept_enabled.clone(),
        websocket_enabled: state.websocket_intercept_enabled.clone(),
        pending_tx: Some(intercept_pending_tx),
        pending_response_tx: Some(intercept_response_pending_tx),
        pending_websocket_tx: Some(intercept_websocket_pending_tx),
        request_filter_rules: state.request_filter_rules.clone(),
        response_filter_rules: state.response_filter_rules.clone(),
    };

    // 创建代理服务（支持拦截）
    let proxy = ProxyService::with_intercept(config, ca_dir, intercept_state);

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

    // 获取历史记录缓存
    let history_cache = state.get_history_cache();

    // 将非 Send 的 ScanPipeline 放入独立线程 + current-thread tokio runtime 中运行
    let db_for_pipeline = db_service.clone();
    let cache_for_pipeline = history_cache.clone();
    let app_for_pipeline = app.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build current_thread runtime for ScanPipeline");
        rt.block_on(async move {
            // 使用 LocalSet 确保所有 !Send 任务固定在该线程执行
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async move {
                    let pipeline = ScanPipeline::new(scan_rx, finding_tx)
                        .with_db_service(db_for_pipeline.clone())
                        .with_history_cache(cache_for_pipeline)
                        .with_app_handle(app_for_pipeline);
                    match pipeline
                        .load_enabled_plugins_from_db(&db_for_pipeline)
                        .await
                    {
                        Ok(n) => tracing::info!("Loaded {} enabled plugins into ScanPipeline", n),
                        Err(e) => tracing::error!("Failed to load enabled plugins: {}", e),
                    }
                    if let Err(e) = pipeline.start().await {
                        tracing::error!("ScanPipeline exited with error: {}", e);
                    } else {
                        tracing::info!("ScanPipeline stopped normally");
                    }
                })
                .await;
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
        Err(e) => return Err(format!("Failed to start proxy: {}", e)),
    };

    // 事件监听（发现推送给前端）
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(finding) = event_rx.recv().await {
            emit_finding(&app_clone, FindingEvent::from(finding));
        }
    });

    // 处理拦截请求（从代理发来的待处理请求）
    let app_for_intercept = app.clone();
    let intercepted_requests_arc = state.intercepted_requests.clone();
    tokio::spawn(async move {
        while let Some(pending_req) = intercept_pending_rx.recv().await {
            tracing::info!(
                "Received intercept request: {} {}",
                pending_req.method,
                pending_req.url
            );

            // 发送事件到前端
            emit_intercept_request(
                &app_for_intercept,
                InterceptRequestEvent {
                    id: pending_req.id.clone(),
                    method: pending_req.method.clone(),
                    url: pending_req.url.clone(),
                    path: pending_req.path.clone(),
                    protocol: pending_req.protocol.clone(),
                    headers: pending_req.headers.clone(),
                    body: pending_req.body.clone(),
                    timestamp: pending_req.timestamp,
                },
            );

            // 保存到待处理列表（带响应通道）
            let mut requests = intercepted_requests_arc.write().await;
            requests.insert(
                pending_req.id.clone(),
                InterceptedRequestInternal {
                    request: InterceptedRequest {
                        id: pending_req.id.clone(),
                        method: pending_req.method,
                        url: pending_req.url,
                        path: pending_req.path,
                        protocol: pending_req.protocol,
                        headers: pending_req.headers,
                        body: pending_req.body,
                        timestamp: pending_req.timestamp,
                    },
                    response_tx: pending_req.response_tx,
                },
            );
        }
    });

    // 处理 WebSocket 拦截消息
    let app_for_ws_intercept = app.clone();
    let intercepted_ws_arc = state.intercepted_websocket_messages.clone();
    tokio::spawn(async move {
        while let Some(pending_msg) = intercept_websocket_pending_rx.recv().await {
            tracing::info!(
                "Received intercept websocket: {} {}",
                pending_msg.connection_id,
                pending_msg.message_type
            );

            // 发送事件到前端
            // 注意：因为 PendingInterceptWebSocketMessage 包含 oneshot sender，不能直接发送
            // 这里我们构造一个前端友好的结构体
            let event_payload = serde_json::json!({
                "id": pending_msg.id,
                "connection_id": pending_msg.connection_id,
                "direction": pending_msg.direction,
                "message_type": pending_msg.message_type,
                "content": pending_msg.content,
                "timestamp": pending_msg.timestamp,
            });

            if let Err(e) = app_for_ws_intercept.emit("proxy:intercept_websocket", event_payload) {
                tracing::error!("Failed to emit proxy:intercept_websocket event: {}", e);
            }

            // 保存到待处理列表（带响应通道）
            let mut messages = intercepted_ws_arc.write().await;
            let msg_id = pending_msg.id.clone();
            messages.insert(
                msg_id,
                InterceptedWebSocketMessageInternal {
                    id: pending_msg.id,
                    connection_id: pending_msg.connection_id,
                    direction: pending_msg.direction,
                    message_type: pending_msg.message_type,
                    content: pending_msg.content,
                    timestamp: pending_msg.timestamp,
                    response_tx: pending_msg.response_tx,
                },
            );
        }
    });
    let app_for_response_intercept = app.clone();
    let intercepted_responses_arc = state.intercepted_responses.clone();
    tokio::spawn(async move {
        while let Some(pending_resp) = intercept_response_pending_rx.recv().await {
            tracing::info!(
                "Received intercept response: {} (status: {})",
                pending_resp.id,
                pending_resp.status
            );

            // 发送事件到前端
            emit_intercept_response(
                &app_for_response_intercept,
                InterceptResponseEvent {
                    id: pending_resp.id.clone(),
                    request_id: pending_resp.request_id.clone(),
                    status: pending_resp.status,
                    headers: pending_resp.headers.clone(),
                    body: pending_resp.body.clone(),
                    timestamp: pending_resp.timestamp,
                },
            );

            // 保存到待处理列表（带响应通道）
            let mut responses = intercepted_responses_arc.write().await;
            responses.insert(
                pending_resp.id.clone(),
                InterceptedResponseInternal {
                    response: InterceptedResponse {
                        id: pending_resp.id.clone(),
                        request_id: pending_resp.request_id,
                        status: pending_resp.status,
                        headers: pending_resp.headers,
                        body: pending_resp.body,
                        timestamp: pending_resp.timestamp,
                    },
                    response_tx: pending_resp.response_tx,
                },
            );
        }
    });

    // 启动周期性统计发射任务（每5秒）
    let app_clone2 = app.clone();
    let is_running_arc = state.is_running.clone();
    let proxy_service_arc = state.proxy_service.clone();
    let db_service_arc = db_service.clone();

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
                        responses: stats.http_requests + stats.https_requests,
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
        app,
        ProxyStatusEvent {
            running: true,
            port,
            mitm: true,
            stats: ProxyStats::default(),
        },
    );

    tracing::info!(
        "Passive scan started on port {} (ScanPipeline running in dedicated thread)",
        port
    );
    Ok(port)
}

/// 启动被动扫描代理
#[tauri::command]
pub async fn start_passive_scan(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    config: Option<ProxyConfig>,
) -> Result<CommandResponse<u16>, String> {
    // Multi-point license verification
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Ok(CommandResponse::err(
            "License required for this feature".to_string(),
        ));
    }

    match start_passive_scan_internal(&app, &state, config).await {
        Ok(port) => Ok(CommandResponse::ok(port)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 内部停止函数（可在内部和外部复用）
pub async fn stop_passive_scan_internal(
    app: &AppHandle,
    state: &PassiveScanState,
) -> Result<(), String> {
    let mut is_running = state.is_running.write().await;
    if !*is_running {
        return Err("Proxy not running".to_string());
    }

    let mut proxy = state.proxy_service.write().await;
    if let Some(p) = proxy.take() {
        if let Err(e) = p.stop().await {
            tracing::error!("Failed to stop proxy: {}", e);
            return Err(format!("Failed to stop proxy: {}", e));
        }
    }

    *is_running = false;

    // 发射代理停止事件
    emit_proxy_status(
        app,
        ProxyStatusEvent {
            running: false,
            port: 0,
            mitm: false,
            stats: ProxyStats::default(),
        },
    );

    tracing::info!("Passive scan stopped");
    Ok(())
}

/// 停止被动扫描代理
#[tauri::command]
pub async fn stop_passive_scan(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    match stop_passive_scan_internal(&app, &state).await {
        Ok(_) => Ok(CommandResponse::ok("Proxy stopped".to_string())),
        Err(e) => Ok(CommandResponse::err(e)),
    }
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
        Ok(CommandResponse::ok(format!(
            "插件 {} 重载任务已发送",
            plugin_id
        )))
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
            tracing::info!(
                "Loaded {} findings with evidence from database",
                records.len()
            );
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
    let main_category: Option<String> =
        sqlx::query_scalar("SELECT main_category FROM plugin_registry WHERE id = ?")
            .bind(&plugin_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;

    let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
        .bind(true)
        .bind(&plugin_id)
        .execute(db.pool())
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                tracing::info!("Plugin enabled in database: {}", plugin_id);

                let plugin_name = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM plugin_registry WHERE id = ?",
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
    let main_category: Option<String> =
        sqlx::query_scalar("SELECT main_category FROM plugin_registry WHERE id = ?")
            .bind(&plugin_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;

    let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
        .bind(false)
        .bind(&plugin_id)
        .execute(db.pool())
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                tracing::info!("Plugin disabled in database: {}", plugin_id);

                let plugin_name = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM plugin_registry WHERE id = ?",
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

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchToggleResult {
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub failed_ids: Vec<String>,
}

#[tauri::command]
pub async fn batch_enable_plugins(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    plugin_ids: Vec<String>,
) -> Result<CommandResponse<BatchToggleResult>, String> {
    let db = state.get_db_service().await?;

    let mut enabled_count: usize = 0;
    let mut failed_ids: Vec<String> = Vec::new();

    for plugin_id in plugin_ids.iter() {
        let main_category: Option<String> =
            sqlx::query_scalar("SELECT main_category FROM plugin_registry WHERE id = ?")
                .bind(plugin_id)
                .fetch_optional(db.pool())
                .await
                .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;

        let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
            .bind(true)
            .bind(plugin_id)
            .execute(db.pool())
            .await;

        match result {
            Ok(exec) => {
                if exec.rows_affected() > 0 {
                    enabled_count += 1;
                    let plugin_name = sqlx::query_scalar::<_, String>(
                        "SELECT name FROM plugin_registry WHERE id = ?",
                    )
                    .bind(plugin_id)
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
                } else {
                    failed_ids.push(plugin_id.clone());
                }
            }
            Err(_) => {
                failed_ids.push(plugin_id.clone());
            }
        }
    }

    Ok(CommandResponse::ok(BatchToggleResult {
        enabled_count,
        disabled_count: 0,
        failed_ids,
    }))
}

#[tauri::command]
pub async fn batch_disable_plugins(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    plugin_ids: Vec<String>,
) -> Result<CommandResponse<BatchToggleResult>, String> {
    let db = state.get_db_service().await?;

    let mut disabled_count: usize = 0;
    let mut failed_ids: Vec<String> = Vec::new();

    for plugin_id in plugin_ids.iter() {
        let main_category: Option<String> =
            sqlx::query_scalar("SELECT main_category FROM plugin_registry WHERE id = ?")
                .bind(plugin_id)
                .fetch_optional(db.pool())
                .await
                .map_err(|e| format!("Failed to query plugin main_category: {}", e))?;

        let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
            .bind(false)
            .bind(plugin_id)
            .execute(db.pool())
            .await;

        match result {
            Ok(exec) => {
                if exec.rows_affected() > 0 {
                    disabled_count += 1;
                    let plugin_name = sqlx::query_scalar::<_, String>(
                        "SELECT name FROM plugin_registry WHERE id = ?",
                    )
                    .bind(plugin_id)
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
                } else {
                    failed_ids.push(plugin_id.clone());
                }
            }
            Err(_) => {
                failed_ids.push(plugin_id.clone());
            }
        }
    }

    Ok(CommandResponse::ok(BatchToggleResult {
        enabled_count: 0,
        disabled_count,
        failed_ids,
    }))
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
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
            e
        ))),
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
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
            e
        ))),
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

/// 重新生成 CA 证书（删除旧的并生成新的，如果代理正在运行则自动重启）
#[tauri::command]
pub async fn regenerate_ca_cert(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    // 检查代理是否正在运行
    let was_running = *state.is_running.read().await;
    let mut saved_port = None;

    // 如果代理正在运行，先停止它
    if was_running {
        tracing::info!("Proxy is running, stopping before regenerating CA...");

        // 获取当前端口（用于日志）
        if let Some(proxy) = state.proxy_service.read().await.as_ref() {
            saved_port = proxy.get_port().await;
        }

        // 停止代理
        let mut proxy = state.proxy_service.write().await;
        if let Some(p) = proxy.take() {
            if let Err(e) = p.stop().await {
                tracing::error!("Failed to stop proxy before regenerating CA: {}", e);
            }
        }

        // 更新运行状态
        *state.is_running.write().await = false;

        // 发送停止事件
        emit_proxy_status(
            &app,
            ProxyStatusEvent {
                running: false,
                port: 0,
                mitm: false,
                stats: ProxyStats::default(),
            },
        );
    }

    // 重新生成证书
    match state.certificate_service.regenerate_root_ca().await {
        Ok(_) => {
            let path = state
                .certificate_service
                .export_root_ca()
                .map_err(|e| format!("Failed to get CA path: {}", e))?;
            let path_str = path.to_string_lossy().to_string();

            tracing::info!("CA certificate regenerated at: {}", path_str);

            // 如果之前代理正在运行，重新启动它
            if was_running {
                tracing::info!("Restarting proxy with new CA certificate...");

                // 使用默认配置重新启动代理
                match start_passive_scan_internal(&app, &state, None).await {
                    Ok(new_port) => {
                        tracing::info!(
                            "Proxy restarted on port {} with new CA certificate",
                            new_port
                        );
                        Ok(CommandResponse::ok(format!(
                            "CA certificate regenerated and proxy restarted on port {}. Please re-import: {}",
                            new_port, path_str
                        )))
                    }
                    Err(e) => {
                        tracing::error!("Failed to restart proxy after CA regeneration: {}", e);
                        Ok(CommandResponse::ok(format!(
                            "CA certificate regenerated but failed to restart proxy: {}. Please re-import: {}",
                            e, path_str
                        )))
                    }
                }
            } else {
                Ok(CommandResponse::ok(format!(
                    "CA certificate regenerated successfully. Please re-import: {}",
                    path_str
                )))
            }
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

/// 导出 CA 证书为 DER 格式
#[tauri::command]
pub async fn export_ca_cert(
    state: State<'_, PassiveScanState>,
    format: String,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match format.as_str() {
        "der" => match state.certificate_service.export_cert_der() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                tracing::info!("Exported CA certificate in DER format: {}", path_str);
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
        _ => match state.certificate_service.export_root_ca() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
    }
}

/// 导出 CA 私钥为 DER 格式
#[tauri::command]
pub async fn export_ca_key(
    state: State<'_, PassiveScanState>,
    format: String,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match format.as_str() {
        "der" => match state.certificate_service.export_key_der() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                tracing::info!("Exported CA private key in DER format: {}", path_str);
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
        _ => Ok(CommandResponse::err("Unsupported format".to_string())),
    }
}

/// 导出 CA 证书和私钥为 PKCS#12 格式
#[tauri::command]
pub async fn export_ca_pkcs12(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    // Use empty password for easier import
    match state.certificate_service.export_pkcs12(Some("")) {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("Exported CA in PKCS#12 format: {}", path_str);
            Ok(CommandResponse::ok(CaCertPath { path: path_str }))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
    }
}

/// 从 PKCS#12 文件导入证书和私钥
#[tauri::command]
pub async fn import_ca_pkcs12(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    // Open file dialog to select PKCS#12 file
    let file_path = app
        .dialog()
        .file()
        .add_filter("PKCS#12 Files", &["p12", "pfx"])
        .blocking_pick_file();

    let file_path = match file_path {
        Some(path) => path
            .into_path()
            .map_err(|e| format!("Invalid path: {}", e))?,
        None => return Ok(CommandResponse::err("No file selected".to_string())),
    };

    // Read file
    let data = std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Import with empty password (user can be prompted for password if needed)
    match state.certificate_service.import_pkcs12(&data, "") {
        Ok(_) => {
            tracing::info!("Imported CA from PKCS#12: {}", file_path.display());
            Ok(CommandResponse::ok(
                "Certificate imported successfully".to_string(),
            ))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to import: {}", e))),
    }
}

/// 从 DER 格式导入证书和私钥
#[tauri::command]
pub async fn import_ca_der(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    // Select certificate file
    let cert_path = app
        .dialog()
        .file()
        .set_title("Select Certificate (DER format)")
        .add_filter("DER Certificate", &["der", "cer", "crt"])
        .blocking_pick_file();

    let cert_path = match cert_path {
        Some(path) => path
            .into_path()
            .map_err(|e| format!("Invalid path: {}", e))?,
        None => {
            return Ok(CommandResponse::err(
                "No certificate file selected".to_string(),
            ))
        }
    };

    // Select key file
    let key_path = app
        .dialog()
        .file()
        .set_title("Select Private Key (DER format)")
        .add_filter("DER Key", &["der", "key"])
        .blocking_pick_file();

    let key_path = match key_path {
        Some(path) => path
            .into_path()
            .map_err(|e| format!("Invalid path: {}", e))?,
        None => return Ok(CommandResponse::err("No key file selected".to_string())),
    };

    // Read files
    let cert_data =
        std::fs::read(&cert_path).map_err(|e| format!("Failed to read certificate: {}", e))?;
    let key_data = std::fs::read(&key_path).map_err(|e| format!("Failed to read key: {}", e))?;

    // Import
    match state.certificate_service.import_der(&cert_data, &key_data) {
        Ok(_) => {
            tracing::info!("Imported CA from DER format");
            Ok(CommandResponse::ok(
                "Certificate imported successfully".to_string(),
            ))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to import: {}", e))),
    }
}

/// 打开 CA 证书目录
#[tauri::command]
pub async fn open_ca_cert_dir(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    match state.certificate_service.export_root_ca() {
        Ok(cert_path) => {
            // 获取父目录
            let dir_path = cert_path
                .parent()
                .ok_or_else(|| "Failed to get parent directory".to_string())?;
            let dir_str = dir_path.to_string_lossy().to_string();

            // 使用系统命令打开目录
            #[cfg(target_os = "macos")]
            {
                match std::process::Command::new("open").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }

            #[cfg(target_os = "windows")]
            {
                match std::process::Command::new("explorer").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }

            #[cfg(target_os = "linux")]
            {
                match std::process::Command::new("xdg-open").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
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

    tracing::debug!(
        "Fetched finding detail: {} with {} evidence",
        finding_id,
        detail.evidence.len()
    );

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
    use std::fs;
    use tera::{Context, Tera};

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
    let critical = vulnerabilities
        .iter()
        .filter(|v| v.severity == "critical")
        .count();
    let high = vulnerabilities
        .iter()
        .filter(|v| v.severity == "high")
        .count();
    let medium = vulnerabilities
        .iter()
        .filter(|v| v.severity == "medium")
        .count();
    let low = vulnerabilities
        .iter()
        .filter(|v| v.severity == "low")
        .count();
    let info = vulnerabilities
        .iter()
        .filter(|v| v.severity == "info")
        .count();

    let total_f = total as f64;
    let summary = ReportSummary {
        total,
        critical,
        high,
        medium,
        low,
        info,
        critical_percent: if total > 0 {
            (critical as f64 / total_f) * 100.0
        } else {
            0.0
        },
        high_percent: if total > 0 {
            (high as f64 / total_f) * 100.0
        } else {
            0.0
        },
        medium_percent: if total > 0 {
            (medium as f64 / total_f) * 100.0
        } else {
            0.0
        },
        low_percent: if total > 0 {
            (low as f64 / total_f) * 100.0
        } else {
            0.0
        },
        info_percent: if total > 0 {
            (info as f64 / total_f) * 100.0
        } else {
            0.0
        },
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
            location: first_evidence
                .map(|e| e.location.clone())
                .unwrap_or_default(),
            evidence: first_evidence
                .map(|e| e.evidence_snippet.clone())
                .unwrap_or_default(),
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
        scan_scope: filters
            .plugin_id
            .clone()
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

    let html = tera
        .render("report", &context)
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

    fs::write(&output_path, html).map_err(|e| format!("Failed to write report: {}", e))?;

    let path_str = output_path.to_string_lossy().to_string();
    tracing::info!("HTML report exported to: {}", path_str);

    Ok(CommandResponse::ok(path_str))
}

// ============================================================
// 代理请求历史相关命令（使用内存缓存）
// ============================================================

/// 列出代理请求历史（从内存缓存）
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
) -> Result<CommandResponse<Vec<sentinel_passive::HttpRequestRecord>>, String> {
    let cache = state.get_history_cache();

    let filters = sentinel_passive::HttpRequestFilters {
        protocol,
        method,
        host,
        status_code_min,
        status_code_max,
        search: None,
        limit: limit.map(|l| l as usize),
        offset: offset.map(|o| o as usize),
    };

    let requests = cache.list_http_requests(filters).await;

    Ok(CommandResponse::ok(requests))
}

/// 获取代理请求详情（从内存缓存）
#[tauri::command]
pub async fn get_proxy_request(
    state: State<'_, PassiveScanState>,
    id: i64,
) -> Result<CommandResponse<Option<sentinel_passive::HttpRequestRecord>>, String> {
    let cache = state.get_history_cache();

    let request = cache.get_http_request_by_id(id).await;

    Ok(CommandResponse::ok(request))
}

/// 清空代理请求历史（清空内存缓存）
#[tauri::command]
pub async fn clear_proxy_requests(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<u64>, String> {
    let cache = state.get_history_cache();

    let count = cache.count_http_requests().await as u64;
    cache.clear_http_requests().await;

    Ok(CommandResponse::ok(count))
}

/// 统计代理请求数量（从内存缓存）
#[tauri::command]
pub async fn count_proxy_requests(
    state: State<'_, PassiveScanState>,
    _protocol: Option<String>,
    _method: Option<String>,
    _host: Option<String>,
    _status_code_min: Option<i32>,
    _status_code_max: Option<i32>,
) -> Result<CommandResponse<i64>, String> {
    let cache = state.get_history_cache();

    // 注意：当前内存缓存的 count 方法不支持过滤，返回总数
    // 如需过滤统计，可以扩展 ProxyHistoryCache
    let count = cache.count_http_requests().await as i64;

    Ok(CommandResponse::ok(count))
}

// ============================================================
// WebSocket 历史相关命令（使用内存缓存）
// ============================================================

/// 列出 WebSocket 连接（从内存缓存）
#[tauri::command]
pub async fn list_websocket_connections(
    state: State<'_, PassiveScanState>,
    host: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CommandResponse<Vec<sentinel_passive::WebSocketConnectionRecord>>, String> {
    let cache = state.get_history_cache();

    // 转换状态过滤
    let status_filter = status.as_ref().and_then(|s| match s.as_str() {
        "open" => Some(sentinel_passive::WebSocketConnectionStatus::Open),
        "closed" => Some(sentinel_passive::WebSocketConnectionStatus::Closed),
        "error" => Some(sentinel_passive::WebSocketConnectionStatus::Error),
        _ => None,
    });

    let filters = sentinel_passive::WebSocketFilters {
        host,
        status: status_filter,
        direction: None,
        message_type: None,
        search: None,
        limit,
        offset,
    };

    let connections = cache.list_ws_connections(filters).await;

    Ok(CommandResponse::ok(connections))
}

/// 列出 WebSocket 消息（从内存缓存）
#[tauri::command]
pub async fn list_websocket_messages(
    state: State<'_, PassiveScanState>,
    connection_id: String,
    direction: Option<String>,
    message_type: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CommandResponse<Vec<sentinel_passive::WebSocketMessageRecord>>, String> {
    let cache = state.get_history_cache();

    // 转换方向过滤
    let direction_filter = direction.as_ref().and_then(|d| match d.as_str() {
        "send" => Some(sentinel_passive::WebSocketDirection::Send),
        "receive" => Some(sentinel_passive::WebSocketDirection::Receive),
        _ => None,
    });

    // 转换消息类型过滤
    let type_filter = message_type.as_ref().and_then(|t| match t.as_str() {
        "text" => Some(sentinel_passive::WebSocketMessageType::Text),
        "binary" => Some(sentinel_passive::WebSocketMessageType::Binary),
        "ping" => Some(sentinel_passive::WebSocketMessageType::Ping),
        "pong" => Some(sentinel_passive::WebSocketMessageType::Pong),
        "close" => Some(sentinel_passive::WebSocketMessageType::Close),
        _ => None,
    });

    let filters = sentinel_passive::WebSocketFilters {
        host: None,
        status: None,
        direction: direction_filter,
        message_type: type_filter,
        search: None,
        limit,
        offset,
    };

    let messages = cache.list_ws_messages(&connection_id, filters).await;

    Ok(CommandResponse::ok(messages))
}

/// 清空 WebSocket 历史（清空内存缓存）
#[tauri::command]
pub async fn clear_websocket_history(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<u64>, String> {
    let cache = state.get_history_cache();

    let count = cache.count_ws_connections().await as u64;
    cache.clear_ws_data().await;

    Ok(CommandResponse::ok(count))
}

/// 获取历史缓存统计信息
#[tauri::command]
pub async fn get_history_stats(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<sentinel_passive::HistoryCacheStats>, String> {
    let cache = state.get_history_cache();

    let stats = cache.stats().await;

    Ok(CommandResponse::ok(stats))
}

/// 清空所有历史记录（HTTP + WebSocket）
#[tauri::command]
pub async fn clear_all_history(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<String>, String> {
    let cache = state.get_history_cache();

    cache.clear_all().await;

    Ok(CommandResponse::ok("All history cleared".to_string()))
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
    let plugin: sentinel_passive::PluginMetadata =
        serde_json::from_value(metadata).map_err(|e| format!("Invalid plugin metadata: {}", e))?;

    let plugin_id = plugin.id.clone();
    let main_category = plugin.main_category.clone();

    // 注册插件到数据库
    db.register_plugin_with_code(&plugin, &plugin_code)
        .await
        .map_err(|e| format!("Failed to create plugin in database: {}", e))?;

    tracing::info!("Plugin created/updated in database: {}", plugin_id);

    // **关键修复**：更新 PluginManager 的代码缓存（如果插件已在内存中）
    let plugin_manager = state.get_plugin_manager();
    if plugin_manager.get_plugin(&plugin_id).await.is_some() {
        // 插件已在内存中，更新代码缓存
        if let Err(e) = plugin_manager
            .set_plugin_code(plugin_id.clone(), plugin_code.clone())
            .await
        {
            tracing::warn!("Failed to update plugin code cache after create: {}", e);
        } else {
            tracing::info!("Plugin code cache updated after create: {}", plugin_id);
        }
    }

    Ok(CommandResponse::ok(plugin_id))
}

/// 全量更新插件（元数据 + 代码）
#[tauri::command]
pub async fn update_plugin(
    state: State<'_, PassiveScanState>,
    metadata: serde_json::Value,
    plugin_code: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service().await?;

    let plugin_id = metadata
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing plugin id")?
        .to_string();

    let plugin_name = metadata
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&plugin_id)
        .to_string();

    let plugin_description = metadata
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let main_category = metadata
        .get("main_category")
        .and_then(|v| v.as_str())
        .unwrap_or("passive")
        .to_string();

    // 解析为 PluginMetadata 用于数据库更新
    let plugin: sentinel_passive::PluginMetadata =
        serde_json::from_value(metadata).map_err(|e| format!("Invalid plugin metadata: {}", e))?;

    db.update_plugin(&plugin, &plugin_code)
        .await
        .map_err(|e| format!("Failed to update plugin: {}", e))?;

    tracing::info!("Plugin updated in database: {}", plugin_id);

    // 更新 PluginManager 的代码缓存
    let plugin_manager = state.get_plugin_manager();
    if let Err(e) = plugin_manager
        .set_plugin_code(plugin_id.clone(), plugin_code.clone())
        .await
    {
        tracing::warn!("Failed to update plugin code cache: {}", e);
    } else {
        tracing::info!("Plugin code cache updated: {}", plugin_id);
    }

    // **关键修复**：如果是 Agent 工具类插件，重新注册到 ToolServer
    if main_category == "agent" {
        let tool_server = sentinel_tools::tool_server::get_tool_server();

        // 构造工具名称
        let sanitized_id = plugin_id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        let tool_name = format!("plugin__{}", sanitized_id);

        // 先注销旧的工具
        tool_server.unregister_tool(&tool_name).await;

        // 解析新的 input_schema
        let input_schema =
            sentinel_tools::plugin_adapter::PluginToolAdapter::parse_tool_input_schema(
                &plugin_code,
            );

        // 创建执行器
        let executor = sentinel_tools::dynamic_tool::create_executor({
            let pid = plugin_id.clone();
            move |args: serde_json::Value| {
                let plugin_id = pid.clone();
                async move {
                    // 调用插件执行逻辑
                    if let Some(ctx) =
                        sentinel_tools::plugin_adapter::get_plugin_context(&plugin_id).await
                    {
                        tracing::info!("Executing plugin: {} (id: {})", ctx.name, ctx.plugin_id);
                        Ok(serde_json::json!({
                            "plugin_id": ctx.plugin_id,
                            "plugin_name": ctx.name,
                            "input": args,
                            "status": "executed"
                        }))
                    } else {
                        Err(format!("Plugin '{}' not registered", plugin_id))
                    }
                }
            }
        });

        // 重新注册工具
        tool_server
            .register_plugin_tool(
                &plugin_id,
                &plugin_name,
                &plugin_description,
                input_schema,
                executor,
            )
            .await;

        tracing::info!("Plugin tool re-registered to ToolServer: {}", tool_name);
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
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        bool,
    )> = sqlx::query_as(
        r#"
        SELECT id, name, version, author, main_category, category, description,
               default_severity, tags, enabled
        FROM plugin_registry
        WHERE id = ?
        "#,
    )
    .bind(&plugin_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Failed to query plugin: {}", e))?;

    if let Some((
        id,
        name,
        version,
        author,
        main_category,
        category,
        description,
        _sev,
        _tags,
        enabled,
    )) = row
    {
        // 如果是 Agent 插件，提示使用 Agent 测试入口
        if main_category == "agent" {
            return Ok(CommandResponse::ok(TestPluginResult {
                success: false,
                message: Some("该插件属于 Agent 工具类别，请使用 Agent 测试入口".to_string()),
                findings: None,
                error: Some("WrongTestEndpoint".to_string()),
            }));
        }
        if !enabled {
            return Ok(CommandResponse::ok(TestPluginResult {
                success: false,
                message: Some(format!("插件 '{}' 当前未启用。请先启用插件。", name)),
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
                let code_opt = sqlx::query_scalar::<_, Option<String>>(
                    "SELECT plugin_code FROM plugin_registry WHERE id = ?",
                )
                .bind(&plugin_id)
                .fetch_optional(db.pool())
                .await
                .map_err(|e| format!("Failed to load plugin code: {}", e))?;
                if let Some(code) = code_opt.flatten() {
                    // 构造 PluginMetadata 供注册（保持与 list_plugins_internal 构建一致）
                    let tags_json: Option<String> =
                        sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = ?")
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
                    let _ = plugin_manager
                        .register_plugin(plugin_id.clone(), metadata, enabled)
                        .await;
                    let _ = plugin_manager
                        .set_plugin_code(plugin_id.clone(), code)
                        .await;
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
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        };

        // 调用插件进行一次扫描，捕获真实 findings。
        let transaction = HttpTransaction {
            request: request_ctx.clone(),
            response: None,
        };
        let findings_result = plugin_manager
            .scan_transaction(&plugin_id, &transaction)
            .await;

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

    Ok(CommandResponse::err(format!(
        "Plugin not found: {}",
        plugin_id
    )))
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
    pub outputs: Option<Vec<serde_json::Value>>,
}

/// 高级并发测试插件：可自定义请求参数 + 并发 + 重复运行
#[tauri::command(rename_all = "camelCase")]
pub async fn test_plugin_advanced(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
    url: Option<String>,
    method: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
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
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id = ?")
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
            outputs: None,
        }));
    }

    use sentinel_passive::RequestContext;
    let plugin_manager = state.get_plugin_manager();

    // 解析 headers
    let parsed_headers = headers.unwrap_or_else(|| {
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
                    was_edited: false,
                    edited_method: None,
                    edited_url: None,
                    edited_headers: None,
                    edited_body: None,
                };
                let transaction = HttpTransaction {
                    request: ctx.clone(),
                    response: None,
                };
                match plugin_manager
                    .scan_transaction(&plugin_id, &transaction)
                    .await
                {
                    Ok(foundings) => {
                        let mapped: Vec<TestFinding> = foundings
                            .into_iter()
                            .map(|f| TestFinding {
                                title: f.title,
                                description: f.description,
                                severity: f.severity.to_string(),
                            })
                            .collect();
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
                run_stats.push(AdvancedRunStat {
                    run_index: idx,
                    duration_ms: dur,
                    findings: findings.len(),
                    error: None,
                });
                all_findings.extend(findings);
            }
            Err((dur, err)) => {
                run_stats.push(AdvancedRunStat {
                    run_index: idx,
                    duration_ms: dur,
                    findings: 0,
                    error: Some(err),
                });
            }
        }
    }

    let total_duration_ms = start_all.elapsed().as_millis();
    let avg_duration_ms = if run_stats.is_empty() {
        0.0
    } else {
        (run_stats.iter().map(|r| r.duration_ms).sum::<u128>() as f64) / (run_stats.len() as f64)
    };
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
        outputs: None,
    }))
}

/// Agent 插件测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTestResult {
    pub success: bool,
    pub message: Option<String>,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: u128,
    pub error: Option<String>,
}

/// 测试 Agent 类型插件
#[tauri::command(rename_all = "camelCase")]
pub async fn test_agent_plugin(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
    inputs: Option<serde_json::Value>,
) -> Result<CommandResponse<AgentTestResult>, String> {
    let db = state.get_db_service().await?;

    // 验证插件存在且是 Agent 类型
    let row: Option<(String, String, bool)> =
        sqlx::query_as("SELECT id, main_category, enabled FROM plugin_registry WHERE id = ?")
            .bind(&plugin_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| format!("Failed to query plugin: {}", e))?;

    let (id, main_category, enabled) = match row {
        Some(r) => r,
        None => {
            return Ok(CommandResponse::ok(AgentTestResult {
                success: false,
                message: Some(format!("插件 '{}' 不存在", plugin_id)),
                output: None,
                execution_time_ms: 0,
                error: Some("PluginNotFound".to_string()),
            }));
        }
    };

    if main_category != "agent" {
        return Ok(CommandResponse::ok(AgentTestResult {
            success: false,
            message: Some("该插件不是 Agent 工具类型，请使用被动扫描测试入口".to_string()),
            output: None,
            execution_time_ms: 0,
            error: Some("WrongPluginType".to_string()),
        }));
    }

    if !enabled {
        return Ok(CommandResponse::ok(AgentTestResult {
            success: false,
            message: Some(format!("插件 '{}' 未启用，请先启用", plugin_id)),
            output: None,
            execution_time_ms: 0,
            error: Some("PluginDisabled".to_string()),
        }));
    }

    // 获取插件代码
    let code: Option<String> =
        sqlx::query_scalar("SELECT plugin_code FROM plugin_registry WHERE id = ?")
            .bind(&plugin_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| format!("Failed to query plugin code: {}", e))?
            .flatten();

    let code = match code {
        Some(c) => c,
        None => {
            return Ok(CommandResponse::ok(AgentTestResult {
                success: false,
                message: Some("插件代码不存在".to_string()),
                output: None,
                execution_time_ms: 0,
                error: Some("NoPluginCode".to_string()),
            }));
        }
    };

    // 获取插件名称
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM plugin_registry WHERE id = ?")
        .bind(&plugin_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Failed to query plugin name: {}", e))?
        .flatten();
    let name = name.unwrap_or_else(|| plugin_id.clone());

    // 注册插件上下文
    let ctx = sentinel_tools::plugin_adapter::PluginContext {
        plugin_id: plugin_id.clone(),
        name: name.clone(),
        code: code.clone(),
    };
    sentinel_tools::plugin_adapter::register_plugin_context(ctx).await;

    // 准备输入参数
    let inputs = inputs.unwrap_or(serde_json::json!({}));

    // 执行插件
    let start = std::time::Instant::now();
    let name_for_result = name.clone();

    let result = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async {
            let mut engine = sentinel_plugins::PluginEngine::new()
                .map_err(|e| format!("Failed to create plugin engine: {}", e))?;

            let metadata = sentinel_plugins::PluginMetadata {
                id: plugin_id.clone(),
                name: name.clone(),
                version: "1.0.0".to_string(),
                author: None,
                main_category: "agent".to_string(),
                category: "tool".to_string(),
                default_severity: sentinel_plugins::Severity::Medium,
                tags: vec![],
                description: Some(format!("Agent tool plugin: {}", name)),
            };

            engine
                .load_plugin_with_metadata(&code, metadata)
                .await
                .map_err(|e| format!("Failed to load plugin: {}", e))?;

            let (findings, result) = engine
                .execute_agent(&inputs)
                .await
                .map_err(|e| format!("Plugin execution failed: {}", e))?;

            // 构建输出
            let output = if let Some(r) = result {
                r
            } else if !findings.is_empty() {
                let findings_json: Vec<serde_json::Value> = findings
                    .into_iter()
                    .map(|f| {
                        serde_json::json!({
                            "id": f.id,
                            "vuln_type": f.vuln_type,
                            "severity": format!("{:?}", f.severity).to_lowercase(),
                            "title": f.title,
                            "description": f.description,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "findings": findings_json,
                    "findings_count": findings_json.len()
                })
            } else {
                serde_json::json!({
                    "message": "Plugin executed successfully with no output"
                })
            };

            Ok::<serde_json::Value, String>(output)
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?;

    let execution_time_ms = start.elapsed().as_millis();

    match result {
        Ok(output) => Ok(CommandResponse::ok(AgentTestResult {
            success: true,
            message: Some(format!(
                "插件 '{}' 执行成功 ({}ms)",
                name_for_result, execution_time_ms
            )),
            output: Some(output),
            execution_time_ms,
            error: None,
        })),
        Err(e) => Ok(CommandResponse::ok(AgentTestResult {
            success: false,
            message: Some(format!("插件执行失败: {}", e)),
            output: None,
            execution_time_ms,
            error: Some(e),
        })),
    }
}

/// 获取插件输入参数 Schema
#[tauri::command(rename_all = "camelCase")]
pub async fn get_plugin_input_schema(
    state: State<'_, PassiveScanState>,
    plugin_id: String,
) -> Result<CommandResponse<serde_json::Value>, String> {
    let db = state.get_db_service().await?;

    // 获取插件代码
    let code: Option<String> =
        sqlx::query_scalar("SELECT plugin_code FROM plugin_registry WHERE id = ?")
            .bind(&plugin_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| format!("Failed to query plugin code: {}", e))?
            .flatten();

    let code = match code {
        Some(c) => c,
        None => {
            return Ok(CommandResponse::ok(serde_json::json!({
                "type": "object",
                "properties": {}
            })));
        }
    };

    // 解析 ToolInput interface
    let schema = sentinel_tools::plugin_adapter::PluginToolAdapter::parse_tool_input_schema(&code);

    Ok(CommandResponse::ok(schema))
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

    tracing::info!(
        "Batch deleted {} vulnerabilities out of {}",
        deleted_count,
        vuln_ids.len()
    );
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
    tracing::info!(
        "Starting proxy listener on {}:{} (index: {})",
        host,
        port,
        index
    );

    // 检查代理是否已经在运行
    let is_running = *state.is_running.read().await;
    if is_running {
        // 如果代理已经在运行，直接返回成功
        tracing::info!("Proxy already running, listener request acknowledged");
        return Ok(CommandResponse::ok(format!(
            "Listener {}:{} is already running",
            host, port
        )));
    }

    // 从数据库加载代理配置
    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;

    let mut config = match db.load_config("proxy_config").await {
        Ok(Some(config_json)) => match serde_json::from_str::<ProxyConfig>(&config_json) {
            Ok(config) => {
                tracing::info!(
                    "Loaded proxy configuration from database for listener: {:?}",
                    config
                );
                config
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize config, using default: {}", e);
                ProxyConfig {
                    start_port: port,
                    max_port_attempts: 1,
                    mitm_enabled: true,
                    max_request_body_size: 2 * 1024 * 1024,
                    max_response_body_size: 2 * 1024 * 1024,
                    mitm_bypass_fail_threshold: 3,
                    upstream_proxy: None,
                }
            }
        },
        Ok(None) => {
            tracing::info!("No saved configuration found, using default");
            ProxyConfig {
                start_port: port,
                max_port_attempts: 1,
                mitm_enabled: true,
                max_request_body_size: 2 * 1024 * 1024,
                max_response_body_size: 2 * 1024 * 1024,
                mitm_bypass_fail_threshold: 3,
                upstream_proxy: None,
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load config from database, using default: {}", e);
            ProxyConfig {
                start_port: port,
                max_port_attempts: 1,
                mitm_enabled: true,
                max_request_body_size: 2 * 1024 * 1024,
                max_response_body_size: 2 * 1024 * 1024,
                mitm_bypass_fail_threshold: 3,
                upstream_proxy: None,
            }
        }
    };

    // 使用传入的端口覆盖配置中的端口
    config.start_port = port;
    config.max_port_attempts = 1;

    // 调用启动代理的命令
    match start_passive_scan(app, state, Some(config)).await {
        Ok(response) => {
            if response.success {
                tracing::info!("Proxy listener started successfully on {}:{}", host, port);
                Ok(CommandResponse::ok(format!(
                    "Listener started on {}:{}",
                    host, port
                )))
            } else {
                let error_msg = response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
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
                let error_msg = response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
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

    // 获取数据库服务
    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;

    // 将配置序列化为 JSON
    let config_json = serde_json::to_string(&config).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        format!("Failed to serialize config: {}", e)
    })?;

    // 保存到数据库
    db.save_config("proxy_config", &config_json)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save config to database: {}", e);
            format!("Failed to save config: {}", e)
        })?;

    tracing::info!("Proxy configuration saved successfully");
    Ok(CommandResponse::ok(()))
}

/// 获取代理配置
#[tauri::command]
pub async fn get_proxy_config(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<ProxyConfig>, String> {
    tracing::info!("Getting proxy configuration");

    // 获取数据库服务
    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;

    // 从数据库加载配置
    let config = match db.load_config("proxy_config").await {
        Ok(Some(config_json)) => {
            // 反序列化配置
            match serde_json::from_str::<ProxyConfig>(&config_json) {
                Ok(config) => {
                    tracing::info!("Loaded proxy configuration from database: {:?}", config);
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize config, using default: {}", e);
                    ProxyConfig::default()
                }
            }
        }
        Ok(None) => {
            tracing::info!("No saved configuration found, using default");
            ProxyConfig::default()
        }
        Err(e) => {
            tracing::warn!("Failed to load config from database, using default: {}", e);
            ProxyConfig::default()
        }
    };

    Ok(CommandResponse::ok(config))
}

// ============================================================
// 请求拦截相关命令
// ============================================================

/// 设置拦截启用状态
#[tauri::command]
pub async fn set_intercept_enabled(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    enabled: bool,
) -> Result<CommandResponse<bool>, String> {
    tracing::info!("Setting intercept enabled: {}", enabled);

    // 保存 AppHandle
    state.set_app_handle(app).await;

    let mut intercept = state.intercept_enabled.write().await;
    *intercept = enabled;

    tracing::info!(
        "Intercept mode {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(CommandResponse::ok(enabled))
}

/// 获取拦截启用状态
#[tauri::command]
pub async fn get_intercept_enabled(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

/// 获取所有待处理的拦截请求
#[tauri::command]
pub async fn get_intercepted_requests(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<Vec<InterceptedRequest>>, String> {
    let requests = state.intercepted_requests.read().await;
    let list: Vec<InterceptedRequest> = requests.values().map(|r| r.request.clone()).collect();
    Ok(CommandResponse::ok(list))
}

/// 转发拦截的请求
#[tauri::command]
pub async fn forward_intercepted_request(
    state: State<'_, PassiveScanState>,
    request_id: String,
    modified_content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Forwarding intercepted request: {}", request_id);

    let mut requests = state.intercepted_requests.write().await;
    if let Some(req_internal) = requests.remove(&request_id) {
        let _ = req_internal
            .response_tx
            .send(InterceptAction::Forward(modified_content));
        tracing::info!("Request forwarded: {}", request_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Request not found: {}", request_id);
        Ok(CommandResponse::err(format!(
            "Request not found: {}",
            request_id
        )))
    }
}

/// 丢弃拦截的请求
#[tauri::command]
pub async fn drop_intercepted_request(
    state: State<'_, PassiveScanState>,
    request_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Dropping intercepted request: {}", request_id);

    let mut requests = state.intercepted_requests.write().await;
    if let Some(req_internal) = requests.remove(&request_id) {
        let _ = req_internal.response_tx.send(InterceptAction::Drop);
        tracing::info!("Request dropped: {}", request_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Request not found: {}", request_id);
        Ok(CommandResponse::err(format!(
            "Request not found: {}",
            request_id
        )))
    }
}

// ============================================================
// 响应拦截相关命令
// ============================================================

/// 设置响应拦截启用状态
#[tauri::command]
pub async fn set_response_intercept_enabled(
    app: AppHandle,
    state: State<'_, PassiveScanState>,
    enabled: bool,
) -> Result<CommandResponse<bool>, String> {
    tracing::info!("Setting response intercept enabled: {}", enabled);

    // 保存 AppHandle
    state.set_app_handle(app).await;

    let mut intercept = state.response_intercept_enabled.write().await;
    *intercept = enabled;

    tracing::info!(
        "Response intercept mode {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(CommandResponse::ok(enabled))
}

/// 获取响应拦截启用状态
#[tauri::command]
pub async fn get_response_intercept_enabled(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.response_intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

/// 获取所有待处理的拦截响应
#[tauri::command]
pub async fn get_intercepted_responses(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<Vec<InterceptedResponse>>, String> {
    let responses = state.intercepted_responses.read().await;
    let list: Vec<InterceptedResponse> = responses.values().map(|r| r.response.clone()).collect();
    Ok(CommandResponse::ok(list))
}

/// 转发拦截的响应
#[tauri::command]
pub async fn forward_intercepted_response(
    state: State<'_, PassiveScanState>,
    response_id: String,
    modified_content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Forwarding intercepted response: {}", response_id);

    let mut responses = state.intercepted_responses.write().await;
    if let Some(resp_internal) = responses.remove(&response_id) {
        let _ = resp_internal
            .response_tx
            .send(InterceptAction::Forward(modified_content));
        tracing::info!("Response forwarded: {}", response_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Response not found: {}", response_id);
        Ok(CommandResponse::err(format!(
            "Response not found: {}",
            response_id
        )))
    }
}

/// 丢弃拦截的响应
#[tauri::command]
pub async fn drop_intercepted_response(
    state: State<'_, PassiveScanState>,
    response_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Dropping intercepted response: {}", response_id);

    let mut responses = state.intercepted_responses.write().await;
    if let Some(resp_internal) = responses.remove(&response_id) {
        let _ = resp_internal.response_tx.send(InterceptAction::Drop);
        tracing::info!("Response dropped: {}", response_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Response not found: {}", response_id);
        Ok(CommandResponse::err(format!(
            "Response not found: {}",
            response_id
        )))
    }
}

// ============================================================
// 请求重放（Repeater）相关命令
// ============================================================

/// 重放请求结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub response_time_ms: u64,
}

/// 重放请求（发送自定义 HTTP 请求）
#[tauri::command]
pub async fn replay_request(
    method: String,
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<CommandResponse<ReplayResult>, String> {
    tracing::info!("Replaying request: {} {}", method, url);

    let start = std::time::Instant::now();

    // 创建 HTTP 客户端（禁用证书验证和代理以避免循环）
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy() // 禁用代理，避免通过自身代理造成循环
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 构建请求
    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
        _ => {
            return Ok(CommandResponse::err(format!(
                "Unsupported method: {}",
                method
            )))
        }
    };

    // 添加请求头
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(&key, &value);
        }
    }

    // 添加请求体
    if let Some(body_content) = body {
        request = request.body(body_content);
    }

    // 发送请求
    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    let elapsed = start.elapsed().as_millis() as u64;

    // 提取响应信息
    let status_code = response.status().as_u16();
    let mut resp_headers = std::collections::HashMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = value.to_str() {
            resp_headers.insert(name.to_string(), v.to_string());
        }
    }

    // 读取响应体
    let body = response
        .text()
        .await
        .unwrap_or_else(|e| format!("[Failed to read body: {}]", e));

    tracing::info!(
        "Replay completed: {} {} - {} in {}ms",
        method,
        url,
        status_code,
        elapsed
    );

    Ok(CommandResponse::ok(ReplayResult {
        status_code,
        headers: resp_headers,
        body,
        response_time_ms: elapsed,
    }))
}

/// 解码 chunked 传输编码
fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // 查找 chunk size 行结束
        let line_end = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|p| pos + p);

        let Some(line_end) = line_end else {
            break;
        };

        // 解析 chunk size（十六进制）
        let size_str = String::from_utf8_lossy(&data[pos..line_end]);
        let size_str = size_str.split(';').next().unwrap_or("").trim();
        let chunk_size = match usize::from_str_radix(size_str, 16) {
            Ok(s) => s,
            Err(_) => break,
        };

        // chunk size 为 0 表示结束
        if chunk_size == 0 {
            break;
        }

        // 读取 chunk 数据
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + chunk_size;

        if chunk_end > data.len() {
            break;
        }

        result.extend_from_slice(&data[chunk_start..chunk_end]);

        // 跳过 chunk 数据后的 \r\n
        pos = chunk_end + 2;
    }

    if result.is_empty() {
        data.to_vec()
    } else {
        result
    }
}

/// 解码 HTTP 响应（处理 chunked 传输编码和 gzip/deflate 压缩）
fn decode_http_response(response_buf: &[u8]) -> String {
    use flate2::read::{DeflateDecoder, GzDecoder};
    use std::io::Read;

    // 查找响应头和响应体的分隔点
    let header_end = response_buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4);

    let Some(header_end) = header_end else {
        return String::from_utf8_lossy(response_buf).to_string();
    };

    let header_bytes = &response_buf[..header_end];
    let body_bytes = &response_buf[header_end..];

    // 解析响应头
    let header_str = String::from_utf8_lossy(header_bytes);
    let header_lower = header_str.to_lowercase();

    // 检查 Transfer-Encoding
    let is_chunked = header_lower
        .lines()
        .any(|line| line.starts_with("transfer-encoding:") && line.contains("chunked"));

    // 检查 Content-Encoding
    let content_encoding = header_str
        .lines()
        .find(|line| line.to_lowercase().starts_with("content-encoding:"))
        .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_lowercase());

    // 1. 先处理 chunked 编码
    let body_bytes = if is_chunked {
        decode_chunked(body_bytes)
    } else {
        body_bytes.to_vec()
    };

    // 2. 再处理压缩编码
    let decoded_body = match content_encoding.as_deref() {
        Some("gzip") => {
            let mut decoder = GzDecoder::new(body_bytes.as_slice());
            let mut decoded = Vec::new();
            match decoder.read_to_end(&mut decoded) {
                Ok(_) => {
                    tracing::debug!(
                        "Successfully decoded gzip response, {} -> {} bytes",
                        body_bytes.len(),
                        decoded.len()
                    );
                    decoded
                }
                Err(e) => {
                    tracing::warn!("Failed to decode gzip: {}", e);
                    body_bytes
                }
            }
        }
        Some("deflate") => {
            let mut decoder = DeflateDecoder::new(body_bytes.as_slice());
            let mut decoded = Vec::new();
            match decoder.read_to_end(&mut decoded) {
                Ok(_) => decoded,
                Err(e) => {
                    tracing::warn!("Failed to decode deflate: {}", e);
                    body_bytes
                }
            }
        }
        Some("br") => {
            // Brotli 压缩暂不支持，返回原始数据
            tracing::warn!("Brotli encoding not supported, returning raw body");
            body_bytes
        }
        _ => body_bytes,
    };

    // 组合响应头和解码后的响应体
    let mut result = String::from_utf8_lossy(header_bytes).to_string();
    result.push_str(&String::from_utf8_lossy(&decoded_body));
    result
}

/// 智能读取 HTTP 响应（根据 Content-Length 或 chunked 编码判断响应结束）
async fn read_http_response<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> Vec<u8> {
    use tokio::io::AsyncReadExt;

    let mut response_buf = Vec::new();
    let mut buf = [0u8; 8192];
    let mut headers_parsed = false;
    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    let mut header_end_pos: Option<usize> = None;

    // 短超时用于检测响应结束（500ms 没有新数据就认为响应完成）
    let read_timeout = std::time::Duration::from_millis(500);

    loop {
        match tokio::time::timeout(read_timeout, stream.read(&mut buf)).await {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(n)) => {
                response_buf.extend_from_slice(&buf[..n]);

                // 解析响应头（只解析一次）
                if !headers_parsed {
                    if let Some(pos) = response_buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end_pos = Some(pos + 4);
                        headers_parsed = true;

                        // 解析头部
                        let header_str = String::from_utf8_lossy(&response_buf[..pos]);
                        for line in header_str.lines() {
                            let lower = line.to_lowercase();
                            if lower.starts_with("content-length:") {
                                if let Some(len_str) = line.split(':').nth(1) {
                                    content_length = len_str.trim().parse().ok();
                                }
                            } else if lower.starts_with("transfer-encoding:")
                                && lower.contains("chunked")
                            {
                                is_chunked = true;
                            }
                        }
                    }
                }

                // 检查是否已读取完整响应
                if headers_parsed {
                    if let Some(header_end) = header_end_pos {
                        let body_len = response_buf.len() - header_end;

                        // 有 Content-Length：检查是否已读取足够字节
                        if let Some(expected_len) = content_length {
                            if body_len >= expected_len {
                                break;
                            }
                        }

                        // chunked 编码：检查是否以 0\r\n\r\n 结尾
                        if is_chunked && response_buf.len() >= 5 {
                            let tail = &response_buf[response_buf.len().saturating_sub(7)..];
                            if tail.windows(5).any(|w| w == b"0\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Read error: {}", e);
                break;
            }
            Err(_) => {
                // 读取超时 - 如果已经有响应头，可能响应已完成
                if headers_parsed {
                    break;
                }
                // 如果还没收到响应头，继续等待（但使用更长的总超时）
                if response_buf.is_empty() {
                    // 没有收到任何数据，再等一会
                    continue;
                }
                break;
            }
        }
    }

    response_buf
}

/// Raw 请求重放结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReplayResult {
    pub raw_response: String,
    pub response_time_ms: u64,
}

/// 重放 Raw 请求（通过 TCP socket 直接发送原始字节）
#[tauri::command]
pub async fn replay_raw_request(
    host: String,
    port: u16,
    use_tls: bool,
    raw_request: String,
    timeout_secs: Option<u64>,
) -> Result<CommandResponse<RawReplayResult>, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    tracing::info!(
        "Replaying raw request to {}:{} (TLS: {})",
        host,
        port,
        use_tls
    );

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(30));

    // 连接到目标服务器
    let addr = format!("{}:{}", host, port);
    let stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| format!("Connection timeout to {}", addr))?
        .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    let response_buf = if use_tls {
        // TLS 连接
        let connector = tokio_native_tls::TlsConnector::from(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
                .map_err(|e| format!("Failed to create TLS connector: {}", e))?,
        );

        let mut tls_stream = tokio::time::timeout(timeout, connector.connect(&host, stream))
            .await
            .map_err(|_| "TLS handshake timeout".to_string())?
            .map_err(|e| format!("TLS handshake failed: {}", e))?;

        // 发送原始请求
        tls_stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;
        tls_stream
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;

        // 智能读取响应
        read_http_response(&mut tls_stream).await
    } else {
        // 普通 TCP 连接
        let mut stream = stream;

        // 发送原始请求
        stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;
        stream
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;

        // 智能读取响应
        read_http_response(&mut stream).await
    };

    // 处理响应：检查是否需要解压 gzip
    let raw_response = decode_http_response(&response_buf);

    let elapsed = start.elapsed().as_millis() as u64;
    tracing::info!(
        "Raw replay completed to {}:{} in {}ms, response size: {} bytes",
        host,
        port,
        elapsed,
        raw_response.len()
    );

    Ok(CommandResponse::ok(RawReplayResult {
        raw_response,
        response_time_ms: elapsed,
    }))
}

// ==========================================
// WebSocket 拦截控制命令
// ==========================================

/// 获取 WebSocket 拦截状态
#[tauri::command]
pub async fn get_websocket_intercept_enabled(
    state: tauri::State<'_, PassiveScanState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.websocket_intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

/// 设置 WebSocket 拦截是否启用
#[tauri::command]
pub async fn set_websocket_intercept_enabled(
    state: tauri::State<'_, PassiveScanState>,
    enabled: bool,
) -> Result<CommandResponse<()>, String> {
    let mut guard = state.websocket_intercept_enabled.write().await;
    *guard = enabled;
    tracing::info!("WebSocket intercept enabled set to: {}", enabled);
    Ok(CommandResponse::ok(()))
}

/// 转发拦截的 WebSocket 消息（可选修改内容）
#[tauri::command]
pub async fn forward_intercepted_websocket(
    state: tauri::State<'_, PassiveScanState>,
    id: String,
    content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    let mut messages = state.intercepted_websocket_messages.write().await;
    if let Some(msg) = messages.remove(&id) {
        if msg
            .response_tx
            .send(sentinel_passive::InterceptAction::Forward(content))
            .is_err()
        {
            return Err("Failed to send forward action: receiver dropped".to_string());
        }
        tracing::info!("Forwarded intercepted WebSocket message: {}", id);
        Ok(CommandResponse::ok(()))
    } else {
        Err(format!("Intercepted message not found: {}", id))
    }
}

/// 丢弃拦截的 WebSocket 消息
#[tauri::command]
pub async fn drop_intercepted_websocket(
    state: tauri::State<'_, PassiveScanState>,
    id: String,
) -> Result<CommandResponse<()>, String> {
    let mut messages = state.intercepted_websocket_messages.write().await;
    if let Some(msg) = messages.remove(&id) {
        if msg
            .response_tx
            .send(sentinel_passive::InterceptAction::Drop)
            .is_err()
        {
            return Err("Failed to send drop action: receiver dropped".to_string());
        }
        tracing::info!("Dropped intercepted WebSocket message: {}", id);
        Ok(CommandResponse::ok(()))
    } else {
        Err(format!("Intercepted message not found: {}", id))
    }
}

// ============================================================
// 拦截过滤规则相关命令
// ============================================================

/// 拦截过滤规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InterceptFilterRule {
    /// 规则 ID
    pub id: String,
    /// 规则类型: request, response
    pub rule_type: String,
    /// 匹配类型: domain, url, method, fileExt, header, status, contentType
    pub match_type: String,
    /// 匹配关系: matches, notMatches, contains, notContains
    pub relationship: String,
    /// 匹配条件
    pub condition: String,
    /// 动作: exclude, include
    pub action: String,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// 拦截过滤规则列表
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InterceptFilterRules {
    pub rules: Vec<InterceptFilterRule>,
}

/// 添加拦截过滤规则
#[tauri::command]
pub async fn add_intercept_filter_rule(
    state: State<'_, PassiveScanState>,
    rule: InterceptFilterRule,
) -> Result<CommandResponse<InterceptFilterRule>, String> {
    tracing::info!("Adding intercept filter rule: {:?}", rule);

    let db = state.get_db_service().await.map_err(|e| {
        tracing::error!("Failed to get database service: {}", e);
        format!("Failed to get database service: {}", e)
    })?;

    // Load existing rules
    let mut rules = match db.load_config("intercept_filter_rules").await {
        Ok(Some(json)) => serde_json::from_str::<InterceptFilterRules>(&json).unwrap_or_default(),
        _ => InterceptFilterRules::default(),
    };

    // Create new rule with ID
    let new_rule = InterceptFilterRule {
        id: uuid::Uuid::new_v4().to_string(),
        rule_type: rule.rule_type,
        match_type: rule.match_type,
        relationship: rule.relationship,
        condition: rule.condition,
        action: rule.action,
        enabled: rule.enabled,
    };

    rules.rules.push(new_rule.clone());

    // Save rules
    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_config("intercept_filter_rules", &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule added: {}", new_rule.id);
    Ok(CommandResponse::ok(new_rule))
}

/// 获取所有拦截过滤规则
#[tauri::command]
pub async fn get_intercept_filter_rules(
    state: State<'_, PassiveScanState>,
) -> Result<CommandResponse<Vec<InterceptFilterRule>>, String> {
    let db = state
        .get_db_service()
        .await
        .map_err(|e| format!("Failed to get database service: {}", e))?;

    let rules = match db.load_config("intercept_filter_rules").await {
        Ok(Some(json)) => {
            serde_json::from_str::<InterceptFilterRules>(&json)
                .unwrap_or_default()
                .rules
        }
        _ => Vec::new(),
    };

    Ok(CommandResponse::ok(rules))
}

/// 删除拦截过滤规则
#[tauri::command]
pub async fn remove_intercept_filter_rule(
    state: State<'_, PassiveScanState>,
    rule_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Removing intercept filter rule: {}", rule_id);

    let db = state
        .get_db_service()
        .await
        .map_err(|e| format!("Failed to get database service: {}", e))?;

    let mut rules = match db.load_config("intercept_filter_rules").await {
        Ok(Some(json)) => serde_json::from_str::<InterceptFilterRules>(&json).unwrap_or_default(),
        _ => InterceptFilterRules::default(),
    };

    rules.rules.retain(|r| r.id != rule_id);

    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_config("intercept_filter_rules", &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule removed: {}", rule_id);
    Ok(CommandResponse::ok(()))
}

/// 更新拦截过滤规则
#[tauri::command]
pub async fn update_intercept_filter_rule(
    state: State<'_, PassiveScanState>,
    rule: InterceptFilterRule,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Updating intercept filter rule: {:?}", rule);

    let db = state
        .get_db_service()
        .await
        .map_err(|e| format!("Failed to get database service: {}", e))?;

    let mut rules = match db.load_config("intercept_filter_rules").await {
        Ok(Some(json)) => serde_json::from_str::<InterceptFilterRules>(&json).unwrap_or_default(),
        _ => InterceptFilterRules::default(),
    };

    // Find and update the rule
    if let Some(existing) = rules.rules.iter_mut().find(|r| r.id == rule.id) {
        *existing = rule;
    } else {
        return Ok(CommandResponse::err(format!("Rule not found: {}", rule.id)));
    }

    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_config("intercept_filter_rules", &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule updated");
    Ok(CommandResponse::ok(()))
}

/// 用于前端传递的运行时过滤规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInterceptRule {
    pub enabled: bool,
    pub operator: String,
    pub match_type: String,
    pub relationship: String,
    pub condition: String,
}

/// 更新运行时拦截过滤规则（直接应用到代理）
#[tauri::command]
pub async fn update_runtime_filter_rules(
    state: State<'_, PassiveScanState>,
    rule_type: String,
    rules: Vec<RuntimeInterceptRule>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!(
        "Updating runtime {} filter rules: {} rules",
        rule_type,
        rules.len()
    );

    // Convert to PassiveInterceptFilterRule
    let passive_rules: Vec<PassiveInterceptFilterRule> = rules
        .into_iter()
        .map(|r| PassiveInterceptFilterRule {
            enabled: r.enabled,
            operator: r.operator,
            match_type: r.match_type,
            relationship: r.relationship,
            condition: r.condition,
        })
        .collect();

    if rule_type == "request" {
        let mut guard = state.request_filter_rules.write().await;
        *guard = passive_rules;
        tracing::info!("Request filter rules updated: {} rules", guard.len());
    } else if rule_type == "response" {
        let mut guard = state.response_filter_rules.write().await;
        *guard = passive_rules;
        tracing::info!("Response filter rules updated: {} rules", guard.len());
    } else {
        return Ok(CommandResponse::err(format!(
            "Unknown rule type: {}",
            rule_type
        )));
    }

    Ok(CommandResponse::ok(()))
}
