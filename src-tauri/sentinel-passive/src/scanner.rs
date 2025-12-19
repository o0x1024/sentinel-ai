//! 被动扫描流水线模块
//!
//! 负责：
//! - 接收来自代理的请求/响应上下文
//! - 扇出分发给已启用插件
//! - 收集 Finding 并去重

use crate::history_cache::{HttpRequestRecord, ProxyHistoryCache};
use crate::{
    Finding, PassiveDatabaseService, PassiveError, PluginEngine, RequestContext, ResponseContext,
    Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// 扫描任务（从 proxy.rs 导入）
pub use crate::proxy::ScanTask;

/// Finding 接收器（插件发送 Finding 到此通道）
pub type FindingSender = mpsc::UnboundedSender<Finding>;
pub type FindingReceiver = mpsc::UnboundedReceiver<Finding>;

/// 被动插件配置（仅保存元数据和代码）
#[derive(Clone)]
pub struct PassivePluginConfig {
    pub metadata: crate::types::PluginMetadata,
    pub code: String,
}

/// 被动扫描流水线
pub struct ScanPipeline {
    /// 接收来自代理的扫描任务
    task_rx: mpsc::UnboundedReceiver<ScanTask>,
    /// 发送 Finding 到去重服务
    finding_tx: FindingSender,
    /// 已启用插件配置（plugin_id -> PassivePluginConfig）
    /// 仅保存元数据和代码，每次扫描时创建临时 PluginEngine 避免长期持有 V8 Runtime
    plugin_engines: Arc<RwLock<HashMap<String, PassivePluginConfig>>>,
    /// 请求上下文缓存（request_id -> RequestContext）
    /// 用于匹配请求和响应
    request_cache: Arc<RwLock<HashMap<String, RequestContext>>>,
    /// 数据库服务（用于加载插件和存储漏洞）
    db_service: Option<Arc<PassiveDatabaseService>>,
    /// 历史记录内存缓存（用于存储 HTTP/WebSocket 请求历史）
    history_cache: Option<Arc<ProxyHistoryCache>>,
    /// App Handle (用于发送事件到前端)
    app_handle: Option<tauri::AppHandle>,
}

impl ScanPipeline {
    /// 创建新的扫描流水线
    pub fn new(task_rx: mpsc::UnboundedReceiver<ScanTask>, finding_tx: FindingSender) -> Self {
        Self {
            task_rx,
            finding_tx,
            plugin_engines: Arc::new(RwLock::new(HashMap::new())),
            request_cache: Arc::new(RwLock::new(HashMap::new())),
            db_service: None,
            history_cache: None,
            app_handle: None,
        }
    }

    /// 执行单个插件的请求扫描（为每次调用创建独立 PluginEngine）
    async fn run_plugin_scan_request(
        plugin: &PassivePluginConfig,
        req_ctx: &RequestContext,
    ) -> Result<Vec<Finding>> {
        let mut engine = PluginEngine::new()
            .map_err(|e| PassiveError::Plugin(format!("Failed to create PluginEngine: {}", e)))?;

        engine
            .load_plugin_with_metadata(&plugin.code, plugin.metadata.clone())
            .await
            .map_err(|e| {
                PassiveError::Plugin(format!(
                    "Failed to load plugin {}: {}",
                    plugin.metadata.id, e
                ))
            })?;

        engine
            .scan_request(req_ctx)
            .await
            .map_err(|e| PassiveError::Plugin(format!("Plugin execution error: {}", e)))
    }

    /// 执行单个插件的响应扫描（为每次调用创建独立 PluginEngine）
    async fn run_plugin_scan_response(
        plugin: &PassivePluginConfig,
        req_ctx: &RequestContext,
        resp_ctx: &ResponseContext,
    ) -> Result<Vec<Finding>> {
        let mut engine = PluginEngine::new()
            .map_err(|e| PassiveError::Plugin(format!("Failed to create PluginEngine: {}", e)))?;

        engine
            .load_plugin_with_metadata(&plugin.code, plugin.metadata.clone())
            .await
            .map_err(|e| {
                PassiveError::Plugin(format!(
                    "Failed to load plugin {}: {}",
                    plugin.metadata.id, e
                ))
            })?;

        engine
            .scan_response(req_ctx, resp_ctx)
            .await
            .map_err(|e| PassiveError::Plugin(format!("Plugin execution error: {}", e)))
    }

    /// 设置数据库服务（用于加载插件和存储漏洞，不再用于请求历史）
    pub fn with_db_service(mut self, db_service: Arc<PassiveDatabaseService>) -> Self {
        self.db_service = Some(db_service);
        self
    }

    /// 设置历史记录内存缓存
    pub fn with_history_cache(mut self, cache: Arc<ProxyHistoryCache>) -> Self {
        self.history_cache = Some(cache);
        self
    }

    /// 设置 App Handle
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// 启动扫描流水线（异步循环）
    pub async fn start(mut self) -> Result<()> {
        info!("ScanPipeline started");

        while let Some(task) = self.task_rx.recv().await {
            match task {
                ScanTask::Request(req_ctx) => {
                    self.process_request(req_ctx).await;
                }
                ScanTask::Response(resp_ctx) => {
                    self.process_response(resp_ctx).await;
                }
                ScanTask::ReloadPlugin(plugin_id) => {
                    if let Some(ref db) = self.db_service {
                        match self.reload_plugin(&plugin_id, db).await {
                            Ok(_) => {
                                info!("Successfully reloaded plugin: {}", plugin_id);
                            }
                            Err(e) => {
                                error!("Failed to reload plugin {}: {}", plugin_id, e);
                            }
                        }
                    } else {
                        warn!("Cannot reload plugin {} - no database service", plugin_id);
                    }
                }
                ScanTask::FailedConnection(failed_conn) => {
                    self.process_failed_connection(failed_conn).await;
                }
                ScanTask::WebSocketConnection(ws_conn) => {
                    self.process_websocket_connection(ws_conn).await;
                }
                ScanTask::WebSocketMessage(ws_msg) => {
                    self.process_websocket_message(ws_msg).await;
                }
            }
        }

        info!("ScanPipeline stopped (channel closed)");
        Ok(())
    }

    /// 处理请求上下文
    async fn process_request(&self, req_ctx: RequestContext) {
        // 无论是否有插件，都先缓存请求上下文（用于后续响应匹配和历史记录）
        {
            let mut cache = self.request_cache.write().await;
            cache.insert(req_ctx.id.clone(), req_ctx.clone());
        }

        // 跳过 CONNECT 请求（HTTPS 隧道），不记录也不扫描
        if req_ctx.method == "CONNECT" {
            debug!("Skipping CONNECT request: {}", req_ctx.url);
            return;
        }

        let plugins = self.plugin_engines.read().await;
        if plugins.is_empty() {
            // 暂无插件，仅记录历史，不进行被动扫描
            debug!(
                "No passive plugins enabled, skipping request scan but keeping context for history: {}",
                req_ctx.url
            );
            return;
        }

        debug!(
            "Processing request: {} - {} plugins enabled",
            req_ctx.url,
            plugins.len()
        );

        // 扇出分发到每个插件
        // 注意：PluginEngine 需要 mut 访问，所以需要临时释放锁
        drop(plugins);

        // 获取插件 ID 列表
        let plugin_ids: Vec<String> = {
            let plugins = self.plugin_engines.read().await;
            plugins.keys().cloned().collect()
        };

        // 依次调用每个插件（注意：这里是串行，可以后续优化为并行）
        for plugin_id in plugin_ids {
            // 读取插件配置（不在持有写锁期间执行异步等待）
            let plugin_opt = {
                let plugins = self.plugin_engines.read().await;
                plugins.get(&plugin_id).cloned()
            };

            if let Some(plugin) = plugin_opt {
                match Self::run_plugin_scan_request(&plugin, &req_ctx).await {
                    Ok(findings) => {
                        debug!(
                            "Plugin {} found {} issues in request {}",
                            plugin_id,
                            findings.len(),
                            req_ctx.url
                        );

                        // 发送 Finding 到去重服务，并附加请求数据
                        for mut finding in findings {
                            // 添加请求头和请求体
                            finding.request_headers = serde_json::to_string(&req_ctx.headers).ok();
                            finding.request_body = if req_ctx.body.is_empty() {
                                None
                            } else {
                                match String::from_utf8(req_ctx.body.clone()) {
                                    Ok(s) => Some(s),
                                    Err(_) => {
                                        use base64::{engine::general_purpose, Engine as _};
                                        Some(format!(
                                            "[BASE64]{}",
                                            general_purpose::STANDARD.encode(&req_ctx.body)
                                        ))
                                    }
                                }
                            };

                            if let Err(e) = self.finding_tx.send(finding) {
                                error!("Failed to send finding: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Plugin {} failed to scan request: {:?}", plugin_id, e);
                    }
                }
            }
        }
    }

    /// 处理响应上下文
    async fn process_response(&self, resp_ctx: ResponseContext) {
        let plugins = self.plugin_engines.read().await;
        let has_plugins = !plugins.is_empty();
        drop(plugins);

        // 从缓存中获取请求上下文
        let req_ctx = {
            let cache = self.request_cache.read().await;
            cache.get(&resp_ctx.request_id).cloned()
        };

        let req_ctx = match req_ctx {
            Some(ctx) => ctx,
            None => {
                debug!(
                    "Request context not found for response: {}",
                    resp_ctx.request_id
                );
                return;
            }
        };

        // 记录请求到内存缓存
        if let Some(cache) = &self.history_cache {
            use url::Url;

            let start_time = req_ctx.timestamp;
            let end_time = resp_ctx.timestamp;
            let response_time = (end_time - start_time).num_milliseconds().max(0) as i64;

            debug!(
                "Recording request to cache: url={}, req_body_len={}, resp_body_len={}",
                req_ctx.url,
                req_ctx.body.len(),
                resp_ctx.body.len()
            );

            // 解析 URL
            let parsed_url = Url::parse(&req_ctx.url).ok();
            let host = parsed_url
                .as_ref()
                .and_then(|u| u.host_str())
                .unwrap_or("unknown")
                .to_string();
            let protocol = parsed_url
                .as_ref()
                .map(|u| u.scheme())
                .unwrap_or("http")
                .to_string();

            // 序列化请求头和响应头
            let request_headers = serde_json::to_string(&req_ctx.headers).ok();
            let response_headers = serde_json::to_string(&resp_ctx.headers).ok();

            // 转换请求体和响应体为 String
            // 对于二进制内容，使用 base64 编码
            let request_body = if req_ctx.body.is_empty() {
                None
            } else {
                // 尝试作为 UTF-8 字符串
                match String::from_utf8(req_ctx.body.clone()) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // 如果不是有效的 UTF-8，使用 base64 编码
                        use base64::{engine::general_purpose, Engine as _};
                        Some(format!(
                            "[BASE64]{}",
                            general_purpose::STANDARD.encode(&req_ctx.body)
                        ))
                    }
                }
            };

            let response_body = if resp_ctx.body.is_empty() {
                None
            } else {
                // 尝试作为 UTF-8 字符串
                match String::from_utf8(resp_ctx.body.clone()) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // 如果不是有效的 UTF-8，使用 base64 编码
                        use base64::{engine::general_purpose, Engine as _};
                        Some(format!(
                            "[BASE64]{}",
                            general_purpose::STANDARD.encode(&resp_ctx.body)
                        ))
                    }
                }
            };

            let response_size = resp_ctx.body.len() as i64;

            // 处理 edited 请求字段
            let (was_edited, edited_method, edited_url, edited_request_headers, edited_request_body) = 
                if req_ctx.was_edited {
                    let edited_headers_str = req_ctx.edited_headers.as_ref().map(|h| {
                        h.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    });
                    let edited_body_str = req_ctx.edited_body.as_ref().and_then(|b| {
                        String::from_utf8(b.clone()).ok()
                    });
                    (
                        true,
                        req_ctx.edited_method.clone(),
                        req_ctx.edited_url.clone(),
                        edited_headers_str,
                        edited_body_str,
                    )
                } else {
                    (false, None, None, None, None)
                };

            // 处理 edited 响应字段
            let (edited_response_headers, edited_response_body, edited_status_code) = 
                if resp_ctx.was_edited {
                    let edited_headers_str = resp_ctx.edited_headers.as_ref().map(|h| {
                        h.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    });
                    let edited_body_str = resp_ctx.edited_body.as_ref().and_then(|b| {
                        String::from_utf8(b.clone()).ok()
                    });
                    (
                        edited_headers_str,
                        edited_body_str,
                        resp_ctx.edited_status.map(|s| s as i32),
                    )
                } else {
                    (None, None, None)
                };

            let record = HttpRequestRecord {
                id: 0, // 将由缓存分配
                url: req_ctx.url.clone(),
                host,
                protocol,
                method: req_ctx.method.clone(),
                status_code: resp_ctx.status as i32,
                request_headers,
                request_body,
                response_headers,
                response_body: response_body.clone(),
                response_size,
                response_time,
                timestamp: req_ctx.timestamp,
                was_edited,
                edited_request_headers,
                edited_request_body,
                edited_method,
                edited_url,
                edited_response_headers,
                edited_response_body,
                edited_status_code,
            };

            let inserted_id = cache.add_http_request(record.clone()).await;
            debug!(
                "Successfully recorded request to cache: id={}, url={}, response_body_saved={}",
                inserted_id,
                req_ctx.url,
                response_body.is_some()
            );

            // 发送事件通知前端更新流量历史记录，附带 ID
            if let Some(ref app_handle) = self.app_handle {
                let mut record_with_id = record;
                record_with_id.id = inserted_id;
                if let Err(e) = app_handle.emit("proxy:request", &record_with_id) {
                    warn!("Failed to emit proxy:request event: {}", e);
                }
            }
        }

        if !has_plugins {
            // 清理请求缓存
            let mut cache = self.request_cache.write().await;
            cache.remove(&resp_ctx.request_id);
            return;
        }

        debug!(
            "Processing response for request_id: {}",
            resp_ctx.request_id
        );

        // 获取插件 ID 列表
        let plugin_ids: Vec<String> = {
            let plugins = self.plugin_engines.read().await;
            plugins.keys().cloned().collect()
        };

        // 依次调用每个插件
        for plugin_id in plugin_ids {
            // 读取插件配置（不在持有写锁期间执行异步等待）
            let plugin_opt = {
                let plugins = self.plugin_engines.read().await;
                plugins.get(&plugin_id).cloned()
            };

            if let Some(plugin) = plugin_opt {
                match Self::run_plugin_scan_response(&plugin, &req_ctx, &resp_ctx).await {
                    Ok(findings) => {
                        debug!(
                            "Plugin {} found {} issues in response for {}",
                            plugin_id,
                            findings.len(),
                            req_ctx.url
                        );

                        // 发送 Finding 到去重服务，并附加完整请求/响应数据
                        for mut finding in findings {
                            // 添加请求头和请求体
                            finding.request_headers = serde_json::to_string(&req_ctx.headers).ok();
                            finding.request_body = if req_ctx.body.is_empty() {
                                None
                            } else {
                                match String::from_utf8(req_ctx.body.clone()) {
                                    Ok(s) => Some(s),
                                    Err(_) => {
                                        use base64::{engine::general_purpose, Engine as _};
                                        Some(format!(
                                            "[BASE64]{}",
                                            general_purpose::STANDARD.encode(&req_ctx.body)
                                        ))
                                    }
                                }
                            };

                            // 添加响应状态、响应头和响应体
                            finding.response_status = Some(resp_ctx.status as i32);
                            finding.response_headers =
                                serde_json::to_string(&resp_ctx.headers).ok();
                            finding.response_body = if resp_ctx.body.is_empty() {
                                None
                            } else {
                                match String::from_utf8(resp_ctx.body.clone()) {
                                    Ok(s) => Some(s),
                                    Err(_) => {
                                        use base64::{engine::general_purpose, Engine as _};
                                        Some(format!(
                                            "[BASE64]{}",
                                            general_purpose::STANDARD.encode(&resp_ctx.body)
                                        ))
                                    }
                                }
                            };

                            if let Err(e) = self.finding_tx.send(finding) {
                                error!("Failed to send finding: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Plugin {} failed to scan response: {}", plugin_id, e);
                    }
                }
            }
        }

        // 清理请求缓存（避免内存泄漏）
        {
            let mut cache = self.request_cache.write().await;
            cache.remove(&resp_ctx.request_id);
        }
    }

    /// 处理失败的连接（TLS 握手失败等）
    /// 仅记录日志，不保存到数据库（CONNECT 方法不记录）
    async fn process_failed_connection(&self, failed_conn: crate::proxy::FailedConnection) {
        debug!(
            "Failed connection (not saved): host={}, port={}, error={}",
            failed_conn.host, failed_conn.port, failed_conn.error
        );
    }

    /// 处理 WebSocket 连接建立
    async fn process_websocket_connection(
        &self,
        ws_conn: crate::proxy::WebSocketConnectionContext,
    ) {
        debug!(
            "WebSocket connection: id={}, url={}, host={}",
            ws_conn.id, ws_conn.url, ws_conn.host
        );

        // 将连接记录到历史缓存
        if let Some(cache) = &self.history_cache {
            use crate::history_cache::{WebSocketConnectionRecord, WebSocketConnectionStatus};

            let conn_record = WebSocketConnectionRecord {
                id: ws_conn.id.clone(),
                url: ws_conn.url.clone(),
                host: ws_conn.host.clone(),
                protocol: ws_conn.protocol.clone(),
                request_headers: serde_json::to_string(&ws_conn.request_headers).ok(),
                response_headers: ws_conn
                    .response_headers
                    .as_ref()
                    .and_then(|h| serde_json::to_string(h).ok()),
                status: WebSocketConnectionStatus::Open,
                opened_at: ws_conn.opened_at,
                closed_at: None,
                close_code: None,
                close_reason: None,
                message_ids: Vec::new(),
            };

            cache.add_ws_connection(conn_record).await;
            debug!("WebSocket connection recorded: {}", ws_conn.id);

            // 发送事件通知前端
            if let Some(ref app_handle) = self.app_handle {
                use tauri::Emitter;
                if let Err(e) = app_handle.emit("proxy:websocket_connection", &ws_conn) {
                    warn!("Failed to emit WebSocket connection event: {}", e);
                }
            }
        }
    }

    /// 处理 WebSocket 消息
    async fn process_websocket_message(&self, ws_msg: crate::proxy::WebSocketMessageContext) {
        debug!(
            "WebSocket message: id={}, conn_id={}, type={}, length={}",
            ws_msg.id, ws_msg.connection_id, ws_msg.message_type, ws_msg.content_length
        );

        // 将消息记录到历史缓存
        if let Some(cache) = &self.history_cache {
            use crate::history_cache::{
                WebSocketDirection, WebSocketMessageRecord, WebSocketMessageType,
            };

            // 转换消息方向
            let direction = match ws_msg.direction {
                crate::proxy::WebSocketDirection::ClientToServer => WebSocketDirection::Send,
                crate::proxy::WebSocketDirection::ServerToClient => WebSocketDirection::Receive,
            };

            // 转换消息类型
            let message_type = match ws_msg.message_type.as_str() {
                "text" => WebSocketMessageType::Text,
                "binary" => WebSocketMessageType::Binary,
                "ping" => WebSocketMessageType::Ping,
                "pong" => WebSocketMessageType::Pong,
                "close" => WebSocketMessageType::Close,
                _ => WebSocketMessageType::Text,
            };

            let msg_record = WebSocketMessageRecord {
                id: 0, // 将由缓存分配
                connection_id: ws_msg.connection_id.clone(),
                direction,
                message_type,
                content: ws_msg.content.clone(),
                content_length: ws_msg.content_length,
                timestamp: ws_msg.timestamp,
            };

            let inserted_id = cache.add_ws_message(msg_record).await;
            debug!(
                "WebSocket message recorded: id={}, orig_id={}",
                inserted_id, ws_msg.id
            );

            // 发送事件通知前端
            if let Some(ref app_handle) = self.app_handle {
                use tauri::Emitter;
                if let Err(e) = app_handle.emit("proxy:websocket_message", &ws_msg) {
                    warn!("Failed to emit WebSocket message event: {}", e);
                }
            }
        }
    }

    /// 添加插件到启用列表（供 PluginManager 或测试调用）
    pub async fn add_plugin(&self, plugin_id: String, plugin: PassivePluginConfig) -> Result<()> {
        let mut engines = self.plugin_engines.write().await;
        if engines.contains_key(&plugin_id) {
            return Err(PassiveError::Plugin(format!(
                "Plugin already loaded: {}",
                plugin_id
            )));
        }
        engines.insert(plugin_id.clone(), plugin);
        info!("Plugin added to pipeline: {}", plugin_id);
        Ok(())
    }

    /// 从数据库加载并注册已启用的插件
    ///
    /// # 参数
    /// - `db_service`: 数据库服务，用于查询插件代码和元数据
    ///
    /// # 说明
    /// 此方法查询数据库中所有 `enabled=true` 的插件，
    /// 为每个插件创建 PluginEngine 并加载代码，
    /// 然后注册到 ScanPipeline 中。
    ///
    /// # TODO
    /// - 当前 PluginEngine 不支持 !Send，需要在 LocalSet 中执行
    /// - v8::Global 到 String 的转换尚未完全实现
    pub async fn load_enabled_plugins_from_db(
        &self,
        db_service: &Arc<PassiveDatabaseService>,
    ) -> Result<usize> {
        use crate::types::PluginMetadata;

        info!("Loading enabled plugins from database...");

        // 查询所有启用的被动扫描插件（过滤掉 agent 工具插件）
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, name, version, author, category, description,
                   default_severity, tags, plugin_code
            FROM plugin_registry
            WHERE enabled = true AND main_category = 'passive'
            "#,
        )
        .fetch_all(db_service.pool())
        .await
        .map_err(|e| PassiveError::Database(format!("Failed to query enabled plugins: {}", e)))?;

        let mut loaded_count = 0;
        let mut engines = self.plugin_engines.write().await;

        for (
            id,
            name,
            version,
            author,
            category,
            description,
            default_severity,
            tags,
            plugin_code,
        ) in rows
        {
            // 解析标签
            let tags_array: Vec<String> = tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            // 解析严重等级
            let severity = match default_severity.to_lowercase().as_str() {
                "critical" => crate::types::Severity::Critical,
                "high" => crate::types::Severity::High,
                "medium" => crate::types::Severity::Medium,
                "low" => crate::types::Severity::Low,
                "info" => crate::types::Severity::Info,
                _ => crate::types::Severity::Medium,
            };

            let metadata = PluginMetadata {
                id: id.clone(),
                name: name.clone(),
                version,
                author,
                main_category: "passive".to_string(), // 从数据库加载的默认为passive
                category,
                description,
                default_severity: severity,
                tags: tags_array,
            };

            // 仅保存插件配置（元数据 + 代码），实际执行时再创建 PluginEngine
            let config = PassivePluginConfig {
                metadata,
                code: plugin_code,
            };

            engines.insert(id.clone(), config);
            loaded_count += 1;
            info!("Plugin registered in pipeline: {} ({})", name, id);
        }

        info!("Loaded {} enabled plugins from database", loaded_count);
        Ok(loaded_count)
    }

    /// 移除插件
    pub async fn remove_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut engines = self.plugin_engines.write().await;
        if engines.remove(plugin_id).is_some() {
            info!("Plugin removed from pipeline: {}", plugin_id);
            Ok(())
        } else {
            Err(PassiveError::Plugin(format!(
                "Plugin not found: {}",
                plugin_id
            )))
        }
    }

    /// 重新加载单个插件（从数据库）
    ///
    /// # 参数
    /// - `plugin_id`: 插件ID
    /// - `db_service`: 数据库服务
    ///
    /// # 说明
    /// 从数据库重新读取插件代码和元数据，移除旧实例并加载新实例。
    /// 适用于插件代码更新后的热重载场景。
    pub async fn reload_plugin(
        &self,
        plugin_id: &str,
        db_service: &Arc<PassiveDatabaseService>,
    ) -> Result<()> {
        use crate::types::PluginMetadata;

        info!("Reloading plugin from database: {}", plugin_id);

        // 查询插件信息（仅加载 passive 类型的插件）
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
                bool,
                String,
            ),
        >(
            r#"
            SELECT id, name, version, author, category, description,
                   default_severity, tags, plugin_code, enabled, main_category
            FROM plugin_registry
            WHERE id = ?
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(db_service.pool())
        .await
        .map_err(|e| {
            PassiveError::Database(format!("Failed to query plugin {}: {}", plugin_id, e))
        })?;

        let (
            id,
            name,
            version,
            author,
            category,
            description,
            default_severity,
            tags,
            plugin_code,
            enabled,
            main_category,
        ) = row.ok_or_else(|| {
            PassiveError::Plugin(format!("Plugin not found in database: {}", plugin_id))
        })?;

        // 检查插件类型，只允许加载 passive 类型的插件
        if main_category != "passive" {
            return Err(PassiveError::Plugin(format!(
                "Plugin {} is not a passive scan plugin (main_category: {})",
                plugin_id, main_category
            )));
        }

        if !enabled {
            return Err(PassiveError::Plugin(format!(
                "Plugin {} is not enabled in database",
                plugin_id
            )));
        }

        // 解析标签和严重等级
        let tags_array: Vec<String> = tags
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();

        let severity = match default_severity.to_lowercase().as_str() {
            "critical" => crate::types::Severity::Critical,
            "high" => crate::types::Severity::High,
            "medium" => crate::types::Severity::Medium,
            "low" => crate::types::Severity::Low,
            "info" => crate::types::Severity::Info,
            _ => crate::types::Severity::Medium,
        };

        let metadata = PluginMetadata {
            id: id.clone(),
            name: name.clone(),
            version,
            author,
            main_category: "passive".to_string(), // 从数据库加载的默认为passive
            category,
            description,
            default_severity: severity,
            tags: tags_array,
        };

        // 替换旧配置
        let mut engines = self.plugin_engines.write().await;
        engines.insert(
            id.clone(),
            PassivePluginConfig {
                metadata,
                code: plugin_code,
            },
        );

        info!("Plugin reloaded: {}", name);
        Ok(())
    }

    /// 获取已加载插件数量
    pub async fn plugin_count(&self) -> usize {
        self.plugin_engines.read().await.len()
    }

    /// 获取请求缓存大小
    pub async fn request_cache_size(&self) -> usize {
        self.request_cache.read().await.len()
    }
}

/// Finding 去重服务
pub struct FindingDeduplicator {
    /// 接收来自插件的 Finding
    finding_rx: FindingReceiver,
    /// 去重缓存（使用 Finding 签名）
    cache: Arc<RwLock<std::collections::HashSet<String>>>,
    /// 数据库服务（可选）
    db_service: Option<Arc<PassiveDatabaseService>>,
    /// 新 Finding 事件发送器（用于通知前端）
    event_tx: Option<mpsc::UnboundedSender<Finding>>,
}

impl FindingDeduplicator {
    /// 创建去重服务（不带数据库）
    pub fn new(finding_rx: FindingReceiver) -> Self {
        Self {
            finding_rx,
            cache: Arc::new(RwLock::new(std::collections::HashSet::new())),
            db_service: None,
            event_tx: None,
        }
    }

    /// 创建去重服务（带数据库）
    pub fn with_database(
        finding_rx: FindingReceiver,
        db_service: Arc<PassiveDatabaseService>,
    ) -> Self {
        Self {
            finding_rx,
            cache: Arc::new(RwLock::new(std::collections::HashSet::new())),
            db_service: Some(db_service),
            event_tx: None,
        }
    }

    /// 设置事件发送器（用于通知前端新 Finding）
    pub fn with_event_sender(mut self, event_tx: mpsc::UnboundedSender<Finding>) -> Self {
        self.event_tx = Some(event_tx);
        self
    }

    /// 启动去重服务
    pub async fn start(mut self) -> Result<()> {
        info!("FindingDeduplicator started");

        while let Some(finding) = self.finding_rx.recv().await {
            let signature = finding.calculate_signature();

            // 检查内存缓存
            {
                let cache = self.cache.read().await;
                if cache.contains(&signature) {
                    // 内存缓存命中，更新数据库命中次数
                    if let Some(ref db) = self.db_service {
                        if let Err(e) = db.update_vulnerability_hit(&signature).await {
                            error!("Failed to update hit count: {}", e);
                        }
                    }
                    continue;
                }
            }

            // 检查数据库（如果配置了）
            if let Some(ref db) = self.db_service {
                match db.check_signature_exists(&signature).await {
                    Ok(true) => {
                        // 数据库已存在，更新命中次数并加入内存缓存
                        if let Err(e) = db.update_vulnerability_hit(&signature).await {
                            error!("Failed to update hit count: {}", e);
                        }
                        self.cache.write().await.insert(signature.clone());
                        debug!(
                            "Finding exists in DB, updated hit count: {} (signature: {})",
                            finding.title,
                            &signature[..8]
                        );
                        continue;
                    }
                    Ok(false) => {
                        // 数据库不存在，插入新记录
                        match db.insert_vulnerability(&finding).await {
                            Ok(_) => {
                                self.cache.write().await.insert(signature.clone());
                                debug!(
                                    "New finding inserted to DB: {} - {} (signature: {})",
                                    finding.title,
                                    finding.severity,
                                    &signature[..8]
                                );
                                // 发送事件通知前端
                                if let Some(ref tx) = self.event_tx {
                                    if let Err(e) = tx.send(finding.clone()) {
                                        error!("Failed to send finding event: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to insert vulnerability: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to check signature in DB: {}", e);
                    }
                }
            } else {
                // 无数据库，仅内存去重
                self.cache.write().await.insert(signature.clone());
                info!(
                    "New finding (memory only): {} - {} (signature: {})",
                    finding.title,
                    finding.severity,
                    &signature[..8]
                );
            }
        }

        info!("FindingDeduplicator stopped (channel closed)");
        Ok(())
    }

    /// 获取缓存大小（用于统计）
    pub async fn cache_size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// 清空缓存
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
        info!("FindingDeduplicator cache cleared");
    }
}
