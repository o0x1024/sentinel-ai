//! 流量分析流水线模块
//!
//! 负责：
//! - 接收来自代理的请求/响应上下文
//! - 扇出分发给已启用插件
//! - 收集 Finding 并去重

use crate::history_cache::{HttpRequestRecord, ProxyHistoryCache};
use crate::{
    Finding, TrafficError, RequestContext, ResponseContext,
    Result, InterceptFilterRule,
};
use sentinel_db::DatabaseService;
use sentinel_plugins::{types::HttpTransaction, PluginExecutor};
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

// TrafficPluginConfig 移除，直接使用 PluginExecutor

/// 流量分析流水线
pub struct ScanPipeline {
    /// 接收来自代理的扫描任务
    task_rx: mpsc::UnboundedReceiver<ScanTask>,
    /// 发送 Finding 到去重服务
    finding_tx: FindingSender,
    /// 已启用插件执行器（plugin_id -> PluginExecutor）
    /// 使用长期运行的 PluginExecutor 避免重复创建 V8 Runtime
    plugin_executors: Arc<RwLock<HashMap<String, Arc<PluginExecutor>>>>,
    /// 请求上下文缓存（request_id -> RequestContext）
    /// 用于匹配请求和响应
    request_cache: Arc<RwLock<HashMap<String, RequestContext>>>,
    /// 数据库服务（用于加载插件和存储漏洞）
    db_service: Option<Arc<DatabaseService>>,
    /// 历史记录内存缓存（用于存储 HTTP/WebSocket 请求历史）
    history_cache: Option<Arc<ProxyHistoryCache>>,
    /// App Handle (用于发送事件到前端)
    app_handle: Option<tauri::AppHandle>,
    /// 请求拦截过滤规则（用于过滤流量分析）
    request_filter_rules: Arc<RwLock<Vec<InterceptFilterRule>>>,
    /// 响应拦截过滤规则（用于过滤流量分析）
    response_filter_rules: Arc<RwLock<Vec<InterceptFilterRule>>>,
    /// 是否排除本应用流量的扫描
    exclude_self_traffic: Arc<RwLock<bool>>,
    /// 是否启用流量分析插件扫描
    plugin_scanning_enabled: Arc<RwLock<bool>>,
    /// 并发控制信号量（限制同时执行的插件数量）
    plugin_semaphore: Arc<tokio::sync::Semaphore>,
}

impl ScanPipeline {
    /// 创建新的扫描流水线
    pub fn new(task_rx: mpsc::UnboundedReceiver<ScanTask>, finding_tx: FindingSender) -> Self {
        Self {
            task_rx,
            finding_tx,
            plugin_executors: Arc::new(RwLock::new(HashMap::new())),
            request_cache: Arc::new(RwLock::new(HashMap::new())),
            db_service: None,
            history_cache: None,
            app_handle: None,
            request_filter_rules: Arc::new(RwLock::new(Vec::new())),
            response_filter_rules: Arc::new(RwLock::new(Vec::new())),
            exclude_self_traffic: Arc::new(RwLock::new(true)),
            plugin_scanning_enabled: Arc::new(RwLock::new(true)),
            plugin_semaphore: Arc::new(tokio::sync::Semaphore::new(20)), // 最多20个并发插件执行
        }
    }

    /// 设置请求过滤规则
    pub fn with_request_filter_rules(mut self, rules: Arc<RwLock<Vec<InterceptFilterRule>>>) -> Self {
        self.request_filter_rules = rules;
        self
    }

    /// 设置响应过滤规则
    pub fn with_response_filter_rules(mut self, rules: Arc<RwLock<Vec<InterceptFilterRule>>>) -> Self {
        self.response_filter_rules = rules;
        self
    }

    /// 设置是否排除本应用流量的扫描
    pub fn with_exclude_self_traffic(mut self, exclude: Arc<RwLock<bool>>) -> Self {
        self.exclude_self_traffic = exclude;
        self
    }

    /// 设置是否启用流量分析插件扫描
    pub fn with_plugin_scanning_enabled(mut self, enabled: Arc<RwLock<bool>>) -> Self {
        self.plugin_scanning_enabled = enabled;
        self
    }

    /// 设置数据库服务（用于加载插件和存储漏洞，不再用于请求历史）
    pub fn with_db_service(mut self, db_service: Arc<DatabaseService>) -> Self {
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

        // 启动请求缓存清理任务（每60秒清理一次超过5分钟的请求）
        let request_cache_clone = self.request_cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut cache = request_cache_clone.write().await;
                let now = chrono::Utc::now();
                let original_size = cache.len();
                
                cache.retain(|_, ctx| {
                    (now - ctx.timestamp).num_seconds() < 300 // 保留5分钟内的请求
                });
                
                let cleaned = original_size - cache.len();
                if cleaned > 0 {
                    info!("Request cache cleanup: removed {} expired entries, {} remaining", cleaned, cache.len());
                }
            }
        });

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
                ScanTask::RemovePlugin(plugin_id) => {
                    match self.remove_plugin(&plugin_id).await {
                        Ok(_) => {
                            info!("Successfully removed plugin: {}", plugin_id);
                        }
                        Err(e) => {
                            error!("Failed to remove plugin {}: {}", plugin_id, e);
                        }
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

        // 检查流量分析插件扫描是否启用
        let plugin_scanning_enabled = *self.plugin_scanning_enabled.read().await;
        if !plugin_scanning_enabled {
            debug!(
                "Traffic analysis plugin scanning is disabled, skipping request scan: {}",
                req_ctx.url
            );
            return;
        }

        // 检查请求是否应该被过滤（不进行流量分析）
        if !self.should_scan_request(&req_ctx).await {
            debug!(
                "Request {} skipped by filter rules, not scanning with plugins",
                req_ctx.url
            );
            return;
        }

        let plugins = self.plugin_executors.read().await;
        if plugins.is_empty() {
            // 暂无插件，仅记录历史，不进行流量分析
            debug!(
                "No traffic plugins enabled, skipping request scan but keeping context for history: {}",
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
        // 获取插件执行器列表
        let executors: Vec<(String, Arc<PluginExecutor>)> = plugins
            .iter()
            .map(|(id, exec)| (id.clone(), exec.clone()))
            .collect();
        drop(plugins);

        // 构造 http 事务（仅请求）
        let transaction = HttpTransaction {
            request: req_ctx.clone(),
            response: None,
        };

        // 重新设计：
        // 无论如何，我们需要 spawn 一个 task 来等待结果，不能阻塞 process_request。

        // 克隆 finding_tx 和 semaphore 用于 task
        let finding_tx = self.finding_tx.clone();
        let semaphore = self.plugin_semaphore.clone();

        tokio::spawn(async move {
            for (plugin_id, executor) in executors {
                let tx_clone = transaction.clone();
                let finding_tx = finding_tx.clone();
                let plugin_id_clone = plugin_id.clone();
                let executor_clone = executor.clone();
                let semaphore_clone = semaphore.clone();

                tokio::spawn(async move {
                    // 获取信号量许可，限制并发执行
                    let _permit = semaphore_clone.acquire().await.ok();
                    
                    // Check if restart is needed before execution
                    if let Ok(stats) = executor.get_stats().await {
                        if stats.current_instance_executions >= executor.max_executions_before_restart() {
                            info!(
                                "Auto-restarting plugin {} (executions: {}/{})",
                                plugin_id_clone,
                                stats.current_instance_executions,
                                executor.max_executions_before_restart()
                            );
                            
                            // 重试重启最多3次
                            let mut restart_success = false;
                            for retry in 0..3 {
                                match executor_clone.restart().await {
                                    Ok(_) => {
                                        info!("Plugin {} auto-restarted successfully", plugin_id_clone);
                                        restart_success = true;
                                        break;
                                    }
                                    Err(e) => {
                                        error!("Failed to auto-restart plugin {} (attempt {}/3): {}", 
                                            plugin_id_clone, retry + 1, e);
                                        if retry < 2 {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                        }
                                    }
                                }
                            }
                            
                            if !restart_success {
                                error!("Plugin {} failed to restart after 3 attempts, skipping execution", plugin_id_clone);
                                return;
                            }
                        }
                    }

                    match executor.scan_transaction(tx_clone).await {
                        Ok(findings) => {
                            if !findings.is_empty() {
                                debug!(
                                    "Plugin {} found {} issues in request",
                                    plugin_id,
                                    findings.len()
                                );
                                for finding in findings {
                                    if let Err(e) = finding_tx.send(finding) {
                                        error!("Failed to send finding: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Plugin {} failed to scan request: {:?}", plugin_id, e);
                        }
                    }
                    // _permit 在此处自动释放
                });
            }
        });
    }

    /// 处理响应上下文
    async fn process_response(&self, resp_ctx: ResponseContext) {
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

        // 检查流量分析插件扫描是否启用
        let plugin_scanning_enabled = *self.plugin_scanning_enabled.read().await;
        if !plugin_scanning_enabled {
            debug!(
                "Traffic analysis plugin scanning is disabled, skipping response scan: {}",
                req_ctx.url
            );
            // 仍然记录到历史缓存，但不进行插件扫描
            self.record_to_history_cache(&req_ctx, &resp_ctx).await;
            return;
        }

        // 检查响应是否应该被过滤（不进行流量分析）
        if !self.should_scan_response(&req_ctx, &resp_ctx).await {
            debug!(
                "Response for request {} skipped by filter rules, not scanning with plugins",
                req_ctx.url
            );
            // 仍然记录到历史缓存，但不进行插件扫描
            self.record_to_history_cache(&req_ctx, &resp_ctx).await;
            return;
        }

        let plugins = self.plugin_executors.read().await;
        let has_plugins = !plugins.is_empty();

        let executors: Vec<(String, Arc<PluginExecutor>)> = plugins
            .iter()
            .map(|(id, exec)| (id.clone(), exec.clone()))
            .collect();
        drop(plugins);

        // 记录请求到历史缓存
        self.record_to_history_cache(&req_ctx, &resp_ctx).await;

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

        // 构造完整事务
        let transaction = HttpTransaction {
            request: req_ctx.clone(),
            response: Some(resp_ctx.clone()),
        };

        // 克隆 finding_tx 和 semaphore 用于 task
        let finding_tx = self.finding_tx.clone();
        let semaphore = self.plugin_semaphore.clone();

        // 异步调用插件
        tokio::spawn(async move {
            for (plugin_id, executor) in executors {
                let tx_clone = transaction.clone();
                let finding_tx = finding_tx.clone();
                let plugin_id_clone = plugin_id.clone();
                let executor_clone = executor.clone();
                let semaphore_clone = semaphore.clone();

                tokio::spawn(async move {
                    // 获取信号量许可，限制并发执行
                    let _permit = semaphore_clone.acquire().await.ok();
                    
                    // Check if restart is needed before execution
                    if let Ok(stats) = executor.get_stats().await {
                        if stats.current_instance_executions >= executor.max_executions_before_restart() {
                            info!(
                                "Auto-restarting plugin {} (executions: {}/{})",
                                plugin_id_clone,
                                stats.current_instance_executions,
                                executor.max_executions_before_restart()
                            );
                            
                            // 重试重启最多3次
                            let mut restart_success = false;
                            for retry in 0..3 {
                                match executor_clone.restart().await {
                                    Ok(_) => {
                                        info!("Plugin {} auto-restarted successfully", plugin_id_clone);
                                        restart_success = true;
                                        break;
                                    }
                                    Err(e) => {
                                        error!("Failed to auto-restart plugin {} (attempt {}/3): {}", 
                                            plugin_id_clone, retry + 1, e);
                                        if retry < 2 {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                        }
                                    }
                                }
                            }
                            
                            if !restart_success {
                                error!("Plugin {} failed to restart after 3 attempts, skipping execution", plugin_id_clone);
                                return;
                            }
                        }
                    }

                    match executor.scan_transaction(tx_clone).await {
                        Ok(findings) => {
                            if !findings.is_empty() {
                                debug!(
                                    "Plugin {} found {} issues in response",
                                    plugin_id,
                                    findings.len()
                                );
                                for finding in findings {
                                    if let Err(e) = finding_tx.send(finding) {
                                        error!("Failed to send finding: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Plugin {} failed to scan response: {:?}", plugin_id, e);
                        }
                    }
                    // _permit 在此处自动释放
                });
            }
        });

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
    pub async fn add_plugin(
        &self,
        plugin_id: String,
        metadata: crate::types::PluginMetadata,
        code: String,
    ) -> Result<()> {
        let mut executors = self.plugin_executors.write().await;
        if executors.contains_key(&plugin_id) {
            return Err(TrafficError::Plugin(format!(
                "Plugin already loaded: {}",
                plugin_id
            )));
        }

        let executor = PluginExecutor::new(metadata, code, 1000).map_err(|e| {
            TrafficError::Plugin(format!("Failed to create plugin executor: {}", e))
        })?;

        executors.insert(plugin_id.clone(), Arc::new(executor));
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
        db_service: &Arc<DatabaseService>,
    ) -> Result<usize> {
        use crate::types::PluginMetadata;

        info!("Loading enabled plugins from database...");

        // 查询所有启用的流量分析插件（过滤掉 agent 工具插件）
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
            WHERE enabled = true AND main_category = 'traffic'
            "#,
        )
        .fetch_all(db_service.pool())
        .await
        .map_err(|e| TrafficError::Database(format!("Failed to query enabled plugins: {}", e)))?;

        let mut loaded_count = 0;
        let mut executors = self.plugin_executors.write().await;

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
                main_category: "traffic".to_string(), // 从数据库加载的默认为traffic
                category,
                description,
                default_severity: severity,
                tags: tags_array,
            };

            // 创建 PluginExecutor
            match PluginExecutor::new(metadata, plugin_code, 1000) {
                Ok(executor) => {
                    executors.insert(id.clone(), Arc::new(executor));
                    loaded_count += 1;
                    info!("Plugin registered in pipeline: {} ({})", name, id);
                }
                Err(e) => {
                    error!("Failed to create executor for plugin {}: {}", id, e);
                }
            }
        }

        info!("Loaded {} enabled plugins from database", loaded_count);
        Ok(loaded_count)
    }

    /// 移除插件
    pub async fn remove_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut executors = self.plugin_executors.write().await;
        if executors.remove(plugin_id).is_some() {
            info!("Plugin removed from pipeline: {}", plugin_id);
            Ok(())
        } else {
            Err(TrafficError::Plugin(format!(
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
        db_service: &Arc<DatabaseService>,
    ) -> Result<()> {
        use crate::types::PluginMetadata;

        info!("Reloading plugin from database: {}", plugin_id);

        // 查询插件信息（仅加载 traffic 类型的插件）
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
            TrafficError::Database(format!("Failed to query plugin {}: {}", plugin_id, e))
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
            TrafficError::Plugin(format!("Plugin not found in database: {}", plugin_id))
        })?;

        // 检查插件类型，只允许加载 traffic 类型的插件
        if main_category != "traffic" {
            return Err(TrafficError::Plugin(format!(
                "Plugin {} is not a traffic scan plugin (main_category: {})",
                plugin_id, main_category
            )));
        }

        if !enabled {
            return Err(TrafficError::Plugin(format!(
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
            main_category: "traffic".to_string(), // 从数据库加载的默认为traffic
            category,
            description,
            default_severity: severity,
            tags: tags_array,
        };

        // 替换旧实例
        let mut executors = self.plugin_executors.write().await;

        let executor = PluginExecutor::new(metadata, plugin_code, 1000).map_err(|e| {
            TrafficError::Plugin(format!(
                "Failed to create executor for {}: {}",
                plugin_id, e
            ))
        })?;

        executors.insert(id.clone(), Arc::new(executor));

        info!("Plugin reloaded: {}", name);
        Ok(())
    }

    /// 获取已加载插件数量
    pub async fn plugin_count(&self) -> usize {
        self.plugin_executors.read().await.len()
    }

    /// 获取请求缓存大小
    pub async fn request_cache_size(&self) -> usize {
        self.request_cache.read().await.len()
    }

    /// 检查请求是否来自本应用
    /// 通过检测特殊的 Header 标识（X-Sentinel-Internal）
    fn is_self_traffic(headers: &std::collections::HashMap<String, String>) -> bool {
        // 检查所有可能的 header 名称变体（HTTP headers 是大小写不敏感的）
        let header_value = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-sentinel-internal"))
            .map(|(_, v)| v.as_str());
        
        if let Some(value) = header_value {
            info!("Found x-sentinel-internal header with value: {}", value);
            value == "true" || value == "1"
        } else {
            // info!("x-sentinel-internal header not found. Available headers: {:?}", 
            //     headers.keys().collect::<Vec<_>>());
            false
        }
    }

    /// 检查请求是否为插件主动扫描测试请求
    /// 通过检测 X-Sentinel-Test header 标识
    fn is_plugin_test_request(headers: &std::collections::HashMap<String, String>) -> bool {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-sentinel-test"))
            .map(|(_, v)| v.as_str() == "true" || v.as_str() == "1")
            .unwrap_or(false)
    }

    /// 检查请求是否应该被扫描（应用过滤规则）
    /// 返回 true 表示应该扫描，false 表示应该跳过
    async fn should_scan_request(&self, req_ctx: &RequestContext) -> bool {
        // 首先检查是否为插件测试请求，避免递归扫描
        if Self::is_plugin_test_request(&req_ctx.headers) {
            debug!("✓ Skipping plugin scan for test request: url={}", req_ctx.url);
            return false;
        }

        // 检查是否为本应用流量且配置了排除
        let exclude_self = *self.exclude_self_traffic.read().await;
        let is_self = Self::is_self_traffic(&req_ctx.headers);
        
        // info!(
        //     "Checking request scan: url={}, exclude_self_config={}, is_self_traffic={}", 
        //     req_ctx.url, exclude_self, is_self
        // );
        
        if exclude_self && is_self {
            debug!("✓ Skipping plugin scan for self traffic: url={}", req_ctx.url);
            return false;
        }

        let rules = self.request_filter_rules.read().await;
        if rules.is_empty() {
            return true; // No rules, scan all
        }

        // Extract domain from URL
        let domain = req_ctx
            .url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.split(':').next())
            .unwrap_or("");

        // Extract file extension from URL
        let path = req_ctx.url.split('?').next().unwrap_or(&req_ctx.url);
        let file_ext = path.rsplit('.').next().unwrap_or("");

        // Evaluate rules (AND logic by default, rules with "does_not_match" exclude requests)
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            let value_to_match = match rule.match_type.as_str() {
                "domain_name" | "domain" => domain,
                "url" => &req_ctx.url,
                "http_method" | "method" => &req_ctx.method,
                "file_extension" | "fileExt" => file_ext,
                _ => continue,
            };

            let matches = if rule.condition.is_empty() {
                false
            } else {
                match regex::Regex::new(&rule.condition) {
                    Ok(re) => re.is_match(value_to_match),
                    Err(_) => {
                        // Fallback to simple contains check
                        value_to_match
                            .to_lowercase()
                            .contains(&rule.condition.to_lowercase())
                    }
                }
            };

            // "does_not_match" or "notMatches" means: if condition matches, skip scanning
            if (rule.relationship == "does_not_match" || rule.relationship == "notMatches") && matches {
                debug!(
                    "Request {} skipped by filter rule: {} does_not_match {}",
                    req_ctx.url, rule.match_type, rule.condition
                );
                return false;
            }

            // "matches" means: if condition doesn't match, skip scanning
            if rule.relationship == "matches" && !matches {
                debug!(
                    "Request {} skipped by filter rule: {} matches {}",
                    req_ctx.url, rule.match_type, rule.condition
                );
                return false;
            }
        }

        true
    }

    /// 检查响应是否应该被扫描（应用过滤规则）
    /// 返回 true 表示应该扫描，false 表示应该跳过
    async fn should_scan_response(&self, req_ctx: &RequestContext, resp_ctx: &ResponseContext) -> bool {
        // 首先检查是否为插件测试请求，避免递归扫描
        if Self::is_plugin_test_request(&req_ctx.headers) {
            debug!("✓ Skipping plugin scan for test response: url={}", req_ctx.url);
            return false;
        }

        // 检查是否为本应用流量且配置了排除
        let exclude_self = *self.exclude_self_traffic.read().await;
        let is_self = Self::is_self_traffic(&req_ctx.headers);
        
        debug!(
            "Checking response scan: url={}, exclude_self_config={}, is_self_traffic={}", 
            req_ctx.url, exclude_self, is_self
        );
        
        if exclude_self && is_self {
            debug!("✓ Skipping plugin scan for self traffic response: url={}", req_ctx.url);
            return false;
        }
        
        let rules = self.response_filter_rules.read().await;
        if rules.is_empty() {
            return true; // No rules, scan all
        }

        // Extract domain from URL
        let domain = req_ctx
            .url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.split(':').next())
            .unwrap_or("");

        // Extract file extension from URL
        let path = req_ctx.url.split('?').next().unwrap_or(&req_ctx.url);
        let file_ext = path.rsplit('.').next().unwrap_or("");

        // Extract content type
        let content_type = resp_ctx
            .headers
            .get("content-type")
            .or_else(|| resp_ctx.headers.get("Content-Type"))
            .map(|s| s.as_str())
            .unwrap_or("");

        // Evaluate rules
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            let value_to_match = match rule.match_type.as_str() {
                "domain_name" | "domain" => domain,
                "url" => &req_ctx.url,
                "http_method" | "method" => &req_ctx.method,
                "file_extension" | "fileExt" => file_ext,
                "status" | "status_code" => &resp_ctx.status.to_string(),
                "contentType" | "content_type" => content_type,
                _ => continue,
            };

            let matches = if rule.condition.is_empty() {
                false
            } else {
                match regex::Regex::new(&rule.condition) {
                    Ok(re) => re.is_match(value_to_match),
                    Err(_) => {
                        // Fallback to simple contains check
                        value_to_match
                            .to_lowercase()
                            .contains(&rule.condition.to_lowercase())
                    }
                }
            };

            // "does_not_match" or "notMatches" means: if condition matches, skip scanning
            if (rule.relationship == "does_not_match" || rule.relationship == "notMatches") && matches {
                debug!(
                    "Response for {} skipped by filter rule: {} does_not_match {}",
                    req_ctx.url, rule.match_type, rule.condition
                );
                return false;
            }

            // "matches" means: if condition doesn't match, skip scanning
            if rule.relationship == "matches" && !matches {
                debug!(
                    "Response for {} skipped by filter rule: {} matches {}",
                    req_ctx.url, rule.match_type, rule.condition
                );
                return false;
            }
        }

        true
    }

    /// 记录请求/响应到历史缓存
    async fn record_to_history_cache(&self, req_ctx: &RequestContext, resp_ctx: &ResponseContext) {
        if let Some(cache) = &self.history_cache {
            use url::Url;

            let start_time = req_ctx.timestamp;
            let end_time = resp_ctx.timestamp;
            let response_time = (end_time - start_time).num_milliseconds().max(0);

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
            let request_body = if req_ctx.body.is_empty() {
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

            let response_body = if resp_ctx.body.is_empty() {
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

            let response_size = resp_ctx.body.len() as i64;

            // 处理 edited 字段
            let (was_edited, edited_method, edited_url, edited_request_headers, edited_request_body) =
                if req_ctx.was_edited {
                    let edited_headers_str = req_ctx.edited_headers.as_ref().map(|h| {
                        h.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    });
                    let edited_body_str = req_ctx
                        .edited_body
                        .as_ref()
                        .and_then(|b| String::from_utf8(b.clone()).ok());
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

            let (edited_response_headers, edited_response_body, edited_status_code) =
                if resp_ctx.was_edited {
                    let edited_headers_str = resp_ctx.edited_headers.as_ref().map(|h| {
                        h.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    });
                    let edited_body_str = resp_ctx
                        .edited_body
                        .as_ref()
                        .and_then(|b| String::from_utf8(b.clone()).ok());
                    (
                        edited_headers_str,
                        edited_body_str,
                        resp_ctx.edited_status.map(|s| s as i32),
                    )
                } else {
                    (None, None, None)
                };

            let record = HttpRequestRecord {
                id: 0,
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
                "Successfully recorded request to cache: id={}, url={}",
                inserted_id, req_ctx.url
            );

            // 发送事件通知前端
            if let Some(ref app_handle) = self.app_handle {
                let mut record_with_id = record;
                record_with_id.id = inserted_id;
                if let Err(e) = app_handle.emit("proxy:request", &record_with_id) {
                    warn!("Failed to emit proxy:request event: {}", e);
                }
            }
        }
    }
}

/// Finding 去重服务
pub struct FindingDeduplicator {
    /// 接收来自插件的 Finding
    finding_rx: FindingReceiver,
    /// 去重缓存（使用 Finding 签名）
    cache: Arc<RwLock<std::collections::HashSet<String>>>,
    /// 数据库服务（可选）
    db_service: Option<Arc<DatabaseService>>,
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
        db_service: Arc<DatabaseService>,
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

    /// 使用共享缓存（用于在删除漏洞时清理缓存）
    pub fn with_shared_cache(mut self, cache: Arc<RwLock<std::collections::HashSet<String>>>) -> Self {
        self.cache = cache;
        self
    }

    /// 将 Finding 转换为 TrafficFinding 并插入数据库
    async fn insert_finding_to_db(&self, finding: &Finding, db: &Arc<DatabaseService>) -> Result<()> {
        use sentinel_db::TrafficFinding;
        
        let traffic_finding = TrafficFinding {
            id: finding.id.clone(),
            plugin_id: finding.plugin_id.clone(),
            vuln_type: finding.vuln_type.clone(),
            severity: format!("{}", finding.severity),
            confidence: format!("{:?}", finding.confidence),
            title: finding.title.clone(),
            description: finding.description.clone(),
            cwe: finding.cwe.clone(),
            owasp: finding.owasp.clone(),
            remediation: finding.remediation.clone(),
            url: finding.url.clone(),
            method: finding.method.clone(),
            location: finding.location.clone(),
            evidence: finding.evidence.clone(),
            request_headers: finding.request_headers.clone(),
            request_body: finding.request_body.clone(),
            response_status: finding.response_status,
            response_headers: finding.response_headers.clone(),
            response_body: finding.response_body.clone(),
            created_at: finding.created_at,
        };
        
        db.insert_traffic_vulnerability(&traffic_finding).await?;
        Ok(())
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
                    info!(
                        "Finding duplicate (memory cache hit): {} - signature: {}",
                        finding.title,
                        &signature[..8.min(signature.len())]
                    );
                    if let Some(ref db) = self.db_service {
                        if let Err(e) = db.update_traffic_vulnerability_hit(&signature).await {
                            error!("Failed to update hit count: {}", e);
                        }
                    }
                    continue;
                }
            }

            // 检查数据库（如果配置了）
            if let Some(ref db) = self.db_service {
                match db.check_traffic_signature_exists(&signature).await {
                    Ok(true) => {
                        // 数据库已存在，更新命中次数并加入内存缓存
                        if let Err(e) = db.update_traffic_vulnerability_hit(&signature).await {
                            error!("Failed to update hit count: {}", e);
                        }
                        self.cache.write().await.insert(signature.clone());
                        info!(
                            "Finding exists in DB, updated hit count: {} (signature: {})",
                            finding.title,
                            &signature[..8.min(signature.len())]
                        );
                        continue;
                    }
                    Ok(false) => {
                        // 数据库不存在，插入新记录
                        match self.insert_finding_to_db(&finding, db).await {
                            Ok(_) => {
                                self.cache.write().await.insert(signature.clone());
                                info!(
                                    "New finding inserted to DB: {} - {} (signature: {})",
                                    finding.title,
                                    finding.severity,
                                    &signature[..8.min(signature.len())]
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
