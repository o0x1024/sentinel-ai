use tauri::State;

use crate::commands::command_response_support::CommandResponse;

use super::TrafficAnalysisState;

/// 列出代理请求历史（从内存缓存）
#[tauri::command]
pub async fn list_proxy_requests(
    state: State<'_, TrafficAnalysisState>,
    limit: Option<i64>,
    offset: Option<i64>,
    scheme: Option<String>,
    method: Option<String>,
    host: Option<String>,
    status_code_min: Option<i32>,
    status_code_max: Option<i32>,
) -> Result<CommandResponse<Vec<sentinel_traffic::HttpRequestSummary>>, String> {
    let cache = state.get_history_cache();

    let filters = sentinel_traffic::HttpRequestFilters {
        scheme,
        method,
        host,
        status_code_min,
        status_code_max,
        search: None,
        limit: limit.map(|l| l as usize),
        offset: offset.map(|o| o as usize),
    };

    let requests = cache.list_http_request_summaries(filters).await;

    Ok(CommandResponse::ok(requests))
}

/// 获取代理请求详情（从内存缓存）
#[tauri::command]
pub async fn get_proxy_request(
    state: State<'_, TrafficAnalysisState>,
    id: i64,
) -> Result<CommandResponse<Option<sentinel_traffic::HttpRequestRecord>>, String> {
    let cache = state.get_history_cache();

    let request = cache.get_http_request_by_id(id).await;

    Ok(CommandResponse::ok(request))
}

/// 根据数据库请求 ID 解析当前历史缓存里的请求 ID，不存在则从数据库加载到缓存
#[tauri::command]
pub async fn resolve_proxy_history_request_id_by_db_request_id(
    state: State<'_, TrafficAnalysisState>,
    db_request_id: i64,
) -> Result<CommandResponse<Option<i64>>, String> {
    if db_request_id <= 0 {
        return Ok(CommandResponse::ok(None));
    }

    let cache = state.get_history_cache();
    if let Some(record) = cache.get_http_request_by_db_id(db_request_id).await {
        return Ok(CommandResponse::ok(Some(record.id)));
    }

    let db = state.get_db_service();
    let Some(db_record) = db
        .get_proxy_request_by_id(db_request_id)
        .await
        .map_err(|e| format!("Failed to load proxy request from database: {}", e))?
    else {
        return Ok(CommandResponse::ok(None));
    };

    let history_request_id = cache
        .add_http_request(sentinel_traffic::HttpRequestRecord {
            id: 0,
            db_request_id: db_record.id,
            traffic_request_id: None,
            url: db_record.url,
            host: db_record.host,
            scheme: db_record.scheme,
            http_version_observed: db_record.http_version_observed,
            method: db_record.method,
            status_code: db_record.status_code,
            request_headers: db_record.request_headers,
            request_body: db_record.request_body,
            response_headers: db_record.response_headers,
            response_body: db_record.response_body,
            response_size: db_record.response_size,
            response_time: db_record.response_time,
            timestamp: db_record.timestamp,
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_request_headers: None,
            edited_request_body: None,
            edited_response_headers: None,
            edited_response_body: None,
            edited_status_code: None,
        })
        .await;

    Ok(CommandResponse::ok(Some(history_request_id)))
}

/// 清空代理请求历史（清空内存缓存）
#[tauri::command]
pub async fn clear_proxy_requests(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<u64>, String> {
    let cache = state.get_history_cache();

    let count = cache.count_http_requests().await as u64;
    cache.clear_http_requests().await;

    tracing::info!("Cleared {} HTTP requests from cache", count);
    Ok(CommandResponse::ok(count))
}

/// 保存历史记录到数据库
#[tauri::command]
pub async fn save_history_to_database(
    state: State<'_, TrafficAnalysisState>,
    limit: Option<usize>,
) -> Result<CommandResponse<usize>, String> {
    let cache = state.get_history_cache();
    let db = state.get_db_service();

    let filters = sentinel_traffic::HttpRequestFilters {
        scheme: None,
        method: None,
        host: None,
        status_code_min: None,
        status_code_max: None,
        search: None,
        limit,
        offset: None,
    };

    let requests = cache.list_http_requests(filters).await;
    let total = requests.len();
    let mut saved = 0;
    let mut failed = 0;

    tracing::info!("Saving {} HTTP requests to database", total);

    for request in requests {
        if request.db_request_id.is_some() {
            continue;
        }

        let cache_request_id = request.id;
        let record = sentinel_db::ProxyRequestRecord {
            id: None,
            url: request.url,
            host: request.host,
            scheme: request.scheme,
            http_version_observed: request.http_version_observed,
            method: request.method,
            status_code: request.status_code,
            request_headers: request.request_headers,
            request_body: request.request_body,
            response_headers: request.response_headers,
            response_body: request.response_body,
            response_size: request.response_size,
            response_time: request.response_time,
            timestamp: request.timestamp,
            request_body_compressed: false,
            response_body_compressed: false,
        };

        match db.insert_proxy_request(&record).await {
            Ok(db_request_id) => {
                cache
                    .set_http_request_db_id(cache_request_id, db_request_id)
                    .await;
                saved += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to save request {}: {}", cache_request_id, e);
                failed += 1;
            }
        }
    }

    tracing::info!("Saved {} requests to database ({} failed)", saved, failed);
    Ok(CommandResponse::ok(saved))
}

/// 从数据库加载历史记录
#[tauri::command]
pub async fn load_history_from_database(
    state: State<'_, TrafficAnalysisState>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CommandResponse<Vec<sentinel_traffic::HttpRequestRecord>>, String> {
    let db = state.get_db_service();

    tracing::info!(
        "Loading history from database (limit: {:?}, offset: {:?})",
        limit,
        offset
    );

    let filters = sentinel_db::ProxyRequestFilters {
        scheme: None,
        method: None,
        host: None,
        status_code_min: None,
        status_code_max: None,
        limit: limit.map(|l| l as i64),
        offset: offset.map(|o| o as i64),
    };

    let db_records = db
        .list_proxy_requests(filters)
        .await
        .map_err(|e| format!("Failed to load from database: {}", e))?;

    let mut records = Vec::new();
    for db_record in db_records {
        let record = sentinel_traffic::HttpRequestRecord {
            id: db_record.id.unwrap_or(0),
            db_request_id: db_record.id,
            traffic_request_id: None,
            url: db_record.url,
            host: db_record.host,
            scheme: db_record.scheme,
            http_version_observed: db_record.http_version_observed,
            method: db_record.method,
            status_code: db_record.status_code,
            request_headers: db_record.request_headers,
            request_body: db_record.request_body,
            response_headers: db_record.response_headers,
            response_body: db_record.response_body,
            response_size: db_record.response_size,
            response_time: db_record.response_time,
            timestamp: db_record.timestamp,
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_request_headers: None,
            edited_request_body: None,
            edited_response_headers: None,
            edited_response_body: None,
            edited_status_code: None,
        };
        records.push(record);
    }

    tracing::info!("Loaded {} requests from database", records.len());
    Ok(CommandResponse::ok(records))
}

/// 统计代理请求数量（从内存缓存）
#[tauri::command]
pub async fn count_proxy_requests(
    state: State<'_, TrafficAnalysisState>,
    _protocol: Option<String>,
    _method: Option<String>,
    _host: Option<String>,
    _status_code_min: Option<i32>,
    _status_code_max: Option<i32>,
) -> Result<CommandResponse<i64>, String> {
    let cache = state.get_history_cache();

    let count = cache.count_http_requests().await as i64;

    Ok(CommandResponse::ok(count))
}

/// 列出 WebSocket 连接（从内存缓存）
#[tauri::command]
pub async fn list_websocket_connections(
    state: State<'_, TrafficAnalysisState>,
    host: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CommandResponse<Vec<sentinel_traffic::WebSocketConnectionRecord>>, String> {
    let cache = state.get_history_cache();

    let status_filter = status.as_ref().and_then(|s| match s.as_str() {
        "open" => Some(sentinel_traffic::WebSocketConnectionStatus::Open),
        "closed" => Some(sentinel_traffic::WebSocketConnectionStatus::Closed),
        "error" => Some(sentinel_traffic::WebSocketConnectionStatus::Error),
        _ => None,
    });

    let filters = sentinel_traffic::WebSocketFilters {
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
    state: State<'_, TrafficAnalysisState>,
    connection_id: String,
    direction: Option<String>,
    message_type: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CommandResponse<Vec<sentinel_traffic::WebSocketMessageRecord>>, String> {
    let cache = state.get_history_cache();

    let direction_filter = direction.as_ref().and_then(|d| match d.as_str() {
        "send" => Some(sentinel_traffic::WebSocketDirection::Send),
        "receive" => Some(sentinel_traffic::WebSocketDirection::Receive),
        _ => None,
    });

    let type_filter = message_type.as_ref().and_then(|t| match t.as_str() {
        "text" => Some(sentinel_traffic::WebSocketMessageType::Text),
        "binary" => Some(sentinel_traffic::WebSocketMessageType::Binary),
        "ping" => Some(sentinel_traffic::WebSocketMessageType::Ping),
        "pong" => Some(sentinel_traffic::WebSocketMessageType::Pong),
        "close" => Some(sentinel_traffic::WebSocketMessageType::Close),
        _ => None,
    });

    let filters = sentinel_traffic::WebSocketFilters {
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
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<u64>, String> {
    let cache = state.get_history_cache();

    let count = cache.count_ws_connections().await as u64;
    cache.clear_ws_data().await;

    Ok(CommandResponse::ok(count))
}

/// 获取历史缓存统计信息
#[tauri::command]
pub async fn get_history_stats(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<sentinel_traffic::HistoryCacheStats>, String> {
    let cache = state.get_history_cache();

    let stats = cache.stats().await;

    Ok(CommandResponse::ok(stats))
}

/// 清空所有历史记录（HTTP + WebSocket）
#[tauri::command]
pub async fn clear_all_history(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    let cache = state.get_history_cache();

    cache.clear_all().await;

    Ok(CommandResponse::ok("All history cleared".to_string()))
}
