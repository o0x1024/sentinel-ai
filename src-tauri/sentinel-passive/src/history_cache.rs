//! 代理历史记录内存缓存模块
//!
//! 用于存储 HTTP 和 WebSocket 请求历史记录的内存缓存，
//! 替代原有的数据库存储方案，提供更高的性能。
//!
//! 特性：
//! - 固定容量的 LRU 缓存（默认 2000 条 HTTP + 200 个 WebSocket 连接）
//! - 线程安全（使用 RwLock）
//! - 自动过期清理
//! - 支持过滤和分页查询

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// HTTP 请求记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestRecord {
    pub id: i64,
    pub url: String,
    pub host: String,
    pub protocol: String,
    pub method: String,
    pub status_code: i32,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub response_size: i64,
    pub response_time: i64,
    pub timestamp: DateTime<Utc>,
    /// 是否经过拦截修改
    #[serde(default)]
    pub was_edited: bool,
    /// 修改后的请求头（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_request_headers: Option<String>,
    /// 修改后的请求体（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_request_body: Option<String>,
    /// 修改后的请求方法（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_method: Option<String>,
    /// 修改后的请求 URL（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_url: Option<String>,
    /// 修改后的响应头（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_response_headers: Option<String>,
    /// 修改后的响应体（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_response_body: Option<String>,
    /// 修改后的状态码（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_status_code: Option<i32>,
}

/// WebSocket 连接记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnectionRecord {
    pub id: String,
    pub url: String,
    pub host: String,
    pub protocol: String, // "ws" or "wss"
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub status: WebSocketConnectionStatus,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub close_code: Option<u16>,
    pub close_reason: Option<String>,
    /// 关联的消息 ID 列表（用于快速查找）
    #[serde(skip)]
    pub message_ids: Vec<i64>,
}

/// WebSocket 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketConnectionStatus {
    Open,
    Closed,
    Error,
}

/// WebSocket 消息记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageRecord {
    pub id: i64,
    pub connection_id: String,
    pub direction: WebSocketDirection,
    pub message_type: WebSocketMessageType,
    pub content: Option<String>,
    pub content_length: usize,
    pub timestamp: DateTime<Utc>,
}

/// WebSocket 消息方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketDirection {
    Send,
    Receive,
}

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// 统一的代理历史项（用于时间线展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProxyHistoryItem {
    Http(HttpRequestRecord),
    WsConnection(WebSocketConnectionRecord),
    WsMessage(WebSocketMessageRecord),
}

impl ProxyHistoryItem {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ProxyHistoryItem::Http(r) => r.timestamp,
            ProxyHistoryItem::WsConnection(r) => r.opened_at,
            ProxyHistoryItem::WsMessage(r) => r.timestamp,
        }
    }

    pub fn id_str(&self) -> String {
        match self {
            ProxyHistoryItem::Http(r) => format!("http-{}", r.id),
            ProxyHistoryItem::WsConnection(r) => format!("ws-conn-{}", r.id),
            ProxyHistoryItem::WsMessage(r) => format!("ws-msg-{}", r.id),
        }
    }
}

/// HTTP 请求过滤条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HttpRequestFilters {
    pub protocol: Option<String>,
    pub method: Option<String>,
    pub host: Option<String>,
    pub status_code_min: Option<i32>,
    pub status_code_max: Option<i32>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// WebSocket 过滤条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSocketFilters {
    pub host: Option<String>,
    pub status: Option<WebSocketConnectionStatus>,
    pub direction: Option<WebSocketDirection>,
    pub message_type: Option<WebSocketMessageType>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 统一过滤条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyHistoryFilters {
    /// 协议类型过滤: "all", "http", "websocket"
    pub protocol_type: Option<String>,
    pub http: HttpRequestFilters,
    pub websocket: WebSocketFilters,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 代理历史缓存配置
#[derive(Debug, Clone)]
pub struct HistoryCacheConfig {
    /// HTTP 请求最大缓存数量
    pub max_http_requests: usize,
    /// WebSocket 连接最大缓存数量
    pub max_ws_connections: usize,
    /// 每个 WebSocket 连接最大消息数量
    pub max_messages_per_connection: usize,
}

impl Default for HistoryCacheConfig {
    fn default() -> Self {
        Self {
            max_http_requests: 2000,
            max_ws_connections: 200,
            max_messages_per_connection: 1000,
        }
    }
}

/// 代理历史记录缓存
pub struct ProxyHistoryCache {
    config: HistoryCacheConfig,

    /// HTTP 请求缓存（按时间顺序，新的在前）
    http_requests: Arc<RwLock<VecDeque<HttpRequestRecord>>>,
    /// HTTP ID 计数器
    http_id_counter: AtomicI64,

    /// WebSocket 连接缓存 (connection_id -> record)
    ws_connections: Arc<RwLock<HashMap<String, WebSocketConnectionRecord>>>,
    /// WebSocket 连接顺序（用于 LRU）
    ws_connection_order: Arc<RwLock<VecDeque<String>>>,

    /// WebSocket 消息缓存 (connection_id -> messages)
    ws_messages: Arc<RwLock<HashMap<String, VecDeque<WebSocketMessageRecord>>>>,
    /// WebSocket 消息 ID 计数器
    ws_message_id_counter: AtomicI64,
}

impl ProxyHistoryCache {
    /// 创建新的缓存实例
    pub fn new(config: HistoryCacheConfig) -> Self {
        info!(
            "Creating ProxyHistoryCache with max_http={}, max_ws_conn={}, max_msg_per_conn={}",
            config.max_http_requests, config.max_ws_connections, config.max_messages_per_connection
        );

        Self {
            config,
            http_requests: Arc::new(RwLock::new(VecDeque::new())),
            http_id_counter: AtomicI64::new(1),
            ws_connections: Arc::new(RwLock::new(HashMap::new())),
            ws_connection_order: Arc::new(RwLock::new(VecDeque::new())),
            ws_messages: Arc::new(RwLock::new(HashMap::new())),
            ws_message_id_counter: AtomicI64::new(1),
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(HistoryCacheConfig::default())
    }

    // ============================================================
    // HTTP 请求操作
    // ============================================================

    /// 添加 HTTP 请求记录
    pub async fn add_http_request(&self, mut record: HttpRequestRecord) -> i64 {
        let id = self.http_id_counter.fetch_add(1, Ordering::SeqCst);
        record.id = id;

        let mut requests = self.http_requests.write().await;

        // 添加到队列前端（最新的）
        requests.push_front(record);

        // 超出容量时移除最旧的
        while requests.len() > self.config.max_http_requests {
            requests.pop_back();
        }

        debug!("Added HTTP request #{}, total: {}", id, requests.len());
        id
    }

    /// 获取 HTTP 请求列表
    pub async fn list_http_requests(&self, filters: HttpRequestFilters) -> Vec<HttpRequestRecord> {
        let requests = self.http_requests.read().await;

        let mut results: Vec<_> = requests
            .iter()
            .filter(|r| {
                // 协议过滤
                if let Some(ref protocol) = filters.protocol {
                    if &r.protocol != protocol {
                        return false;
                    }
                }
                // 方法过滤
                if let Some(ref method) = filters.method {
                    if &r.method != method {
                        return false;
                    }
                }
                // 主机过滤
                if let Some(ref host) = filters.host {
                    if !r.host.contains(host) {
                        return false;
                    }
                }
                // 状态码范围过滤
                if let Some(min) = filters.status_code_min {
                    if r.status_code < min {
                        return false;
                    }
                }
                if let Some(max) = filters.status_code_max {
                    if r.status_code > max {
                        return false;
                    }
                }
                // 搜索过滤
                if let Some(ref search) = filters.search {
                    let search_lower = search.to_lowercase();
                    if !r.url.to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // 分页
        let offset = filters.offset.unwrap_or(0);
        let limit = filters.limit.unwrap_or(100);

        if offset > 0 {
            results = results.into_iter().skip(offset).collect();
        }
        results.truncate(limit);

        results
    }

    /// 根据 ID 获取 HTTP 请求
    pub async fn get_http_request_by_id(&self, id: i64) -> Option<HttpRequestRecord> {
        let requests = self.http_requests.read().await;
        requests.iter().find(|r| r.id == id).cloned()
    }

    /// 统计 HTTP 请求数量
    pub async fn count_http_requests(&self) -> usize {
        self.http_requests.read().await.len()
    }

    /// 清空 HTTP 请求
    pub async fn clear_http_requests(&self) {
        let mut requests = self.http_requests.write().await;
        let count = requests.len();
        requests.clear();
        info!("Cleared {} HTTP requests", count);
    }

    // ============================================================
    // WebSocket 连接操作
    // ============================================================

    /// 添加 WebSocket 连接
    pub async fn add_ws_connection(&self, record: WebSocketConnectionRecord) {
        let conn_id = record.id.clone();

        let mut connections = self.ws_connections.write().await;
        let mut order = self.ws_connection_order.write().await;

        // 如果连接已存在，更新它
        if connections.contains_key(&conn_id) {
            connections.insert(conn_id.clone(), record);
            return;
        }

        // 超出容量时移除最旧的连接及其消息
        while order.len() >= self.config.max_ws_connections {
            if let Some(old_id) = order.pop_back() {
                connections.remove(&old_id);
                // 移除该连接的消息
                let mut messages = self.ws_messages.write().await;
                messages.remove(&old_id);
            }
        }

        connections.insert(conn_id.clone(), record);
        order.push_front(conn_id.clone());

        debug!(
            "Added WebSocket connection: {}, total: {}",
            conn_id,
            connections.len()
        );
    }

    /// 更新 WebSocket 连接状态（关闭）
    pub async fn close_ws_connection(
        &self,
        conn_id: &str,
        code: Option<u16>,
        reason: Option<String>,
    ) {
        let mut connections = self.ws_connections.write().await;

        if let Some(conn) = connections.get_mut(conn_id) {
            conn.status = WebSocketConnectionStatus::Closed;
            conn.closed_at = Some(Utc::now());
            conn.close_code = code;
            conn.close_reason = reason;
            debug!("Closed WebSocket connection: {}", conn_id);
        }
    }

    /// 获取 WebSocket 连接列表
    pub async fn list_ws_connections(
        &self,
        filters: WebSocketFilters,
    ) -> Vec<WebSocketConnectionRecord> {
        let connections = self.ws_connections.read().await;
        let order = self.ws_connection_order.read().await;

        let mut results: Vec<_> = order
            .iter()
            .filter_map(|id| connections.get(id))
            .filter(|c| {
                // 主机过滤
                if let Some(ref host) = filters.host {
                    if !c.host.contains(host) {
                        return false;
                    }
                }
                // 状态过滤
                if let Some(ref status) = filters.status {
                    if &c.status != status {
                        return false;
                    }
                }
                // 搜索过滤
                if let Some(ref search) = filters.search {
                    let search_lower = search.to_lowercase();
                    if !c.url.to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // 分页
        let offset = filters.offset.unwrap_or(0);
        let limit = filters.limit.unwrap_or(100);

        if offset > 0 {
            results = results.into_iter().skip(offset).collect();
        }
        results.truncate(limit);

        results
    }

    /// 获取单个 WebSocket 连接
    pub async fn get_ws_connection(&self, conn_id: &str) -> Option<WebSocketConnectionRecord> {
        let connections = self.ws_connections.read().await;
        connections.get(conn_id).cloned()
    }

    /// 统计 WebSocket 连接数量
    pub async fn count_ws_connections(&self) -> usize {
        self.ws_connections.read().await.len()
    }

    /// 清空 WebSocket 连接和消息
    pub async fn clear_ws_data(&self) {
        let mut connections = self.ws_connections.write().await;
        let mut order = self.ws_connection_order.write().await;
        let mut messages = self.ws_messages.write().await;

        let conn_count = connections.len();
        let msg_count: usize = messages.values().map(|v| v.len()).sum();

        connections.clear();
        order.clear();
        messages.clear();

        info!(
            "Cleared {} WebSocket connections and {} messages",
            conn_count, msg_count
        );
    }

    // ============================================================
    // WebSocket 消息操作
    // ============================================================

    /// 添加 WebSocket 消息
    pub async fn add_ws_message(&self, mut record: WebSocketMessageRecord) -> i64 {
        let id = self.ws_message_id_counter.fetch_add(1, Ordering::SeqCst);
        record.id = id;
        let conn_id = record.connection_id.clone();

        let mut messages = self.ws_messages.write().await;

        let msg_queue = messages
            .entry(conn_id.clone())
            .or_insert_with(VecDeque::new);

        // 添加到队列前端
        msg_queue.push_front(record);

        // 超出每连接容量时移除最旧的
        while msg_queue.len() > self.config.max_messages_per_connection {
            msg_queue.pop_back();
        }

        // 同时更新连接的消息 ID 列表
        drop(messages);

        let mut connections = self.ws_connections.write().await;
        if let Some(conn) = connections.get_mut(&conn_id) {
            conn.message_ids.push(id);
        }

        debug!("Added WebSocket message #{} to connection {}", id, conn_id);
        id
    }

    /// 获取 WebSocket 消息列表
    pub async fn list_ws_messages(
        &self,
        conn_id: &str,
        filters: WebSocketFilters,
    ) -> Vec<WebSocketMessageRecord> {
        let messages = self.ws_messages.read().await;

        let Some(msg_queue) = messages.get(conn_id) else {
            return Vec::new();
        };

        let mut results: Vec<_> = msg_queue
            .iter()
            .filter(|m| {
                // 方向过滤
                if let Some(ref direction) = filters.direction {
                    if &m.direction != direction {
                        return false;
                    }
                }
                // 消息类型过滤
                if let Some(ref msg_type) = filters.message_type {
                    if &m.message_type != msg_type {
                        return false;
                    }
                }
                // 搜索过滤
                if let Some(ref search) = filters.search {
                    let search_lower = search.to_lowercase();
                    if let Some(ref content) = m.content {
                        if !content.to_lowercase().contains(&search_lower) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // 分页
        let offset = filters.offset.unwrap_or(0);
        let limit = filters.limit.unwrap_or(100);

        if offset > 0 {
            results = results.into_iter().skip(offset).collect();
        }
        results.truncate(limit);

        results
    }

    /// 获取连接的消息数量
    pub async fn count_ws_messages(&self, conn_id: &str) -> usize {
        let messages = self.ws_messages.read().await;
        messages.get(conn_id).map(|q| q.len()).unwrap_or(0)
    }

    // ============================================================
    // 统一历史记录操作
    // ============================================================

    /// 获取统一的历史记录列表（按时间排序）
    pub async fn list_history(&self, filters: ProxyHistoryFilters) -> Vec<ProxyHistoryItem> {
        let mut items = Vec::new();

        let include_http = filters
            .protocol_type
            .as_ref()
            .map_or(true, |t| t == "all" || t == "http");
        let include_ws = filters
            .protocol_type
            .as_ref()
            .map_or(true, |t| t == "all" || t == "websocket");

        // 获取 HTTP 请求
        if include_http {
            let http_records = self.list_http_requests(filters.http.clone()).await;
            for r in http_records {
                items.push(ProxyHistoryItem::Http(r));
            }
        }

        // 获取 WebSocket 连接和消息
        if include_ws {
            let ws_connections = self.list_ws_connections(filters.websocket.clone()).await;
            for conn in ws_connections {
                items.push(ProxyHistoryItem::WsConnection(conn.clone()));

                // 获取该连接的消息
                let messages = self
                    .list_ws_messages(&conn.id, filters.websocket.clone())
                    .await;
                for msg in messages {
                    items.push(ProxyHistoryItem::WsMessage(msg));
                }
            }
        }

        // 按时间排序（最新的在前）
        items.sort_by(|a, b| b.timestamp().cmp(&a.timestamp()));

        // 应用全局分页
        let offset = filters.offset.unwrap_or(0);
        let limit = filters.limit.unwrap_or(200);

        if offset > 0 {
            items = items.into_iter().skip(offset).collect();
        }
        items.truncate(limit);

        items
    }

    /// 清空所有历史记录
    pub async fn clear_all(&self) {
        self.clear_http_requests().await;
        self.clear_ws_data().await;
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> HistoryCacheStats {
        HistoryCacheStats {
            http_count: self.count_http_requests().await,
            ws_connection_count: self.count_ws_connections().await,
            ws_message_count: {
                let messages = self.ws_messages.read().await;
                messages.values().map(|v| v.len()).sum()
            },
            max_http_requests: self.config.max_http_requests,
            max_ws_connections: self.config.max_ws_connections,
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryCacheStats {
    pub http_count: usize,
    pub ws_connection_count: usize,
    pub ws_message_count: usize,
    pub max_http_requests: usize,
    pub max_ws_connections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // async fn test_http_request_cache() {
    //     let cache = ProxyHistoryCache::new(HistoryCacheConfig {
    //         max_http_requests: 5,
    //         ..Default::default()
    //     });

    //     // 添加请求
    //     for i in 0..7 {
    //         let record = HttpRequestRecord {
    //             id: 0, // 将被覆盖
    //             url: format!("http://example.com/api/{}", i),
    //             host: "example.com".to_string(),
    //             protocol: "http".to_string(),
    //             method: "GET".to_string(),
    //             status_code: 200,
    //             request_headers: None,
    //             request_body: None,
    //             response_headers: None,
    //             response_body: None,
    //             response_size: 100,
    //             response_time: 50,
    //             timestamp: Utc::now(),
    //         };
    //         cache.add_http_request(record).await;
    //     }

    //     // 验证只保留最新的 5 条
    //     assert_eq!(cache.count_http_requests().await, 5);

    //     // 验证最新的在前
    //     let requests = cache
    //         .list_http_requests(HttpRequestFilters::default())
    //         .await;
    //     assert!(requests[0].url.contains("/api/6"));
    // }
    #[tokio::test]
    async fn test_ws_message_cache() {
        let cache = ProxyHistoryCache::with_defaults();

        // 添加连接
        let conn = WebSocketConnectionRecord {
            id: "conn-1".to_string(),
            url: "wss://example.com/ws".to_string(),
            host: "example.com".to_string(),
            protocol: "wss".to_string(),
            request_headers: None,
            response_headers: None,
            status: WebSocketConnectionStatus::Open,
            opened_at: Utc::now(),
            closed_at: None,
            close_code: None,
            close_reason: None,
            message_ids: Vec::new(),
        };
        cache.add_ws_connection(conn).await;

        // 添加消息
        for i in 0..3 {
            let msg = WebSocketMessageRecord {
                id: 0,
                connection_id: "conn-1".to_string(),
                direction: if i % 2 == 0 {
                    WebSocketDirection::Send
                } else {
                    WebSocketDirection::Receive
                },
                message_type: WebSocketMessageType::Text,
                content: Some(format!("message {}", i)),
                content_length: 10,
                timestamp: Utc::now(),
            };
            cache.add_ws_message(msg).await;
        }

        // 验证消息数量
        assert_eq!(cache.count_ws_messages("conn-1").await, 3);

        // 验证连接关闭
        cache
            .close_ws_connection("conn-1", Some(1000), Some("Normal".to_string()))
            .await;
        let conn = cache.get_ws_connection("conn-1").await.unwrap();
        assert_eq!(conn.status, WebSocketConnectionStatus::Closed);
    }
}
