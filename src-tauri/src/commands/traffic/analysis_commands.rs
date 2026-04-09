//! 流量分析 Tauri 命令
//!
//! 提供前端调用的流量分析相关命令：
//! - start_traffic_analysis: 启动流量分析代理
//! - stop_traffic_analysis: 停止流量分析代理
//! - get_proxy_status: 获取代理状态
//! - list_findings: 列出漏洞发现
//! - enable_plugin: 启用插件
//! - disable_plugin: 禁用插件
//! - list_plugins: 列出所有插件

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use sentinel_traffic::{
    Finding, FindingDeduplicator, InterceptFilterRule as TrafficInterceptFilterRule,
    InterceptState, PendingInterceptRequest, PendingInterceptResponse, ProxyConfig, ProxyService,
    ProxyStats, ProxyStatus, ScanPipeline, VulnerabilityFilters,
};

use crate::commands::command_response_support::CommandResponse;
use crate::events::{
    emit_finding, emit_intercept_request, emit_intercept_response, emit_proxy_status,
    emit_scan_stats,
};
use crate::events::{
    FindingEvent, InterceptRequestEvent, InterceptResponseEvent, ProxyStatusEvent, ScanStatsEvent,
};
use crate::services::SystemAgentRuntime;

use super::analysis_state_support::{
    InterceptedRequestInternal, InterceptedResponseInternal, InterceptedWebSocketMessageInternal,
};
use super::finding_support::TrafficFindingView;
use super::intercept_commands::InterceptFilterRules;
use super::replay_support::{
    replay_raw_request as replay_raw_request_impl, RawReplayConfig, RawReplayResult,
};

pub use super::analysis_state_support::{
    InterceptedRequest, InterceptedResponse, TrafficAnalysisState,
};

/// 内部启动函数（可在内部和外部复用）
pub async fn start_traffic_analysis_internal(
    app: &AppHandle,
    state: &TrafficAnalysisState,
    config: Option<ProxyConfig>,
) -> Result<u16, String> {
    let mut is_running = state.is_running.write().await;
    if *is_running {
        return Err("Proxy already running".to_string());
    }

    let config = config.unwrap_or_default();
    {
        let mut scope_include_rules = state.scope_include_rules.write().await;
        *scope_include_rules = config.scope_include_rules.clone();
        tracing::info!(
            "Loaded traffic scope include rules into runtime: {:?}",
            config.scope_include_rules
        );
    }
    {
        let mut scope_exclude_rules = state.scope_exclude_rules.write().await;
        *scope_exclude_rules = config.scope_exclude_rules.clone();
        tracing::info!(
            "Loaded traffic scope exclude rules into runtime: {:?}",
            config.scope_exclude_rules
        );
    }

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
        tokio::sync::mpsc::unbounded_channel::<sentinel_traffic::PendingInterceptWebSocketMessage>(
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

    // 从数据库加载拦截过滤规则
    let db_service = state.get_db_service();
    let loaded_rules = match db_service.load_proxy_config("intercept_filter_rules").await {
        Ok(Some(json)) => match serde_json::from_str::<InterceptFilterRules>(&json) {
            Ok(rules) => {
                tracing::info!(
                    "Loaded {} intercept filter rules from database",
                    rules.rules.len()
                );
                rules.rules
            }
            Err(e) => {
                tracing::warn!("Failed to parse intercept filter rules: {}", e);
                Vec::new()
            }
        },
        _ => {
            tracing::info!("No saved intercept filter rules found");
            Vec::new()
        }
    };

    // 将加载的规则转换为运行时规则并分类
    let mut request_rules = Vec::new();
    let mut response_rules = Vec::new();

    for rule in loaded_rules {
        let runtime_rule = TrafficInterceptFilterRule {
            enabled: rule.enabled,
            operator: "And".to_string(), // Default operator
            match_type: rule.match_type.clone(),
            relationship: rule.relationship.clone(),
            condition: rule.condition.clone(),
        };

        if rule.rule_type == "request" {
            request_rules.push(runtime_rule);
        } else if rule.rule_type == "response" {
            response_rules.push(runtime_rule);
        }
    }

    // 更新 state 中的过滤规则
    {
        let mut req_rules = state.request_filter_rules.write().await;
        *req_rules = request_rules;
        tracing::info!("Loaded {} request filter rules", req_rules.len());
    }
    {
        let mut resp_rules = state.response_filter_rules.write().await;
        *resp_rules = response_rules;
        tracing::info!("Loaded {} response filter rules", resp_rules.len());
    }

    // 从数据库加载流量分析插件扫描开关
    let plugin_scanning_enabled = match db_service
        .load_proxy_config("traffic_analysis_plugin_enabled")
        .await
    {
        Ok(Some(value)) => value.parse::<bool>().unwrap_or(true),
        _ => true, // 默认启用
    };
    {
        let mut plugin_scanning = state.plugin_scanning_enabled.write().await;
        *plugin_scanning = plugin_scanning_enabled;
        tracing::info!(
            "Loaded traffic analysis plugin scanning enabled: {}",
            plugin_scanning_enabled
        );
    }

    // 创建拦截状态
    let intercept_state = InterceptState {
        enabled: state.intercept_enabled.clone(),
        request_enabled: state.request_intercept_enabled.clone(),
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

    // 获取数据库服务
    let db_service = state.get_db_service();

    // 获取历史记录缓存
    let history_cache = state.get_history_cache();
    let system_agent_runtime: Option<Arc<SystemAgentRuntime>> = app
        .try_state::<Arc<SystemAgentRuntime>>()
        .map(|state| state.inner().clone());

    // 将非 Send 的 ScanPipeline 放入独立线程 + current-thread tokio runtime 中运行
    let db_for_pipeline = db_service.clone();
    let cache_for_pipeline = history_cache.clone();
    let app_for_pipeline = app.clone();
    let system_agent_runtime_for_pipeline = system_agent_runtime.clone();
    let request_filter_rules = state.request_filter_rules.clone();
    let response_filter_rules = state.response_filter_rules.clone();
    let exclude_self_traffic = state.exclude_self_traffic.clone();
    let scope_include_rules = state.scope_include_rules.clone();
    let scope_exclude_rules = state.scope_exclude_rules.clone();
    let plugin_scanning_enabled = state.plugin_scanning_enabled.clone();
    let behavior_signal_settings = state.behavior_signal_settings.clone();
    let behavior_extension_events = state.behavior_extension_events.clone();
    let context_extraction_settings = state.context_extraction_settings.clone();
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
                        .with_app_handle(app_for_pipeline)
                        .with_request_record_hook(Arc::new(move |record| {
                            if let Some(runtime) = &system_agent_runtime_for_pipeline {
                                let runtime = runtime.clone();
                                let behavior_signal_settings = behavior_signal_settings.clone();
                                let behavior_extension_events = behavior_extension_events.clone();
                                let context_extraction_settings =
                                    context_extraction_settings.clone();
                                tauri::async_runtime::spawn(async move {
                                    let behavior_signal =
                                        behavior_signal_settings.read().await.clone();
                                    let context_extraction =
                                        context_extraction_settings.read().await.clone();
                                    let behavior_extension_context = {
                                        let events = behavior_extension_events.read().await;
                                        events.build_context_for_request(&record)
                                    };
                                    if let Err(error) = runtime
                                        .handle_history_record(
                                            record,
                                            behavior_signal,
                                            behavior_extension_context,
                                            context_extraction,
                                        )
                                        .await
                                    {
                                        tracing::warn!(
                                            "System agent history hook failed: {}",
                                            error
                                        );
                                    }
                                });
                            }
                        }))
                        .with_request_filter_rules(request_filter_rules)
                        .with_response_filter_rules(response_filter_rules)
                        .with_exclude_self_traffic(exclude_self_traffic)
                        .with_scope_rules(scope_include_rules, scope_exclude_rules)
                        .with_plugin_scanning_enabled(plugin_scanning_enabled);
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

    // 启动 Finding 去重服务（带数据库和事件发送，使用共享缓存）
    let dedupe_cache_for_dedup = state.dedupe_cache.clone();
    let deduplicator = FindingDeduplicator::with_database(finding_rx, db_service.clone())
        .with_event_sender(event_tx)
        .with_shared_cache(dedupe_cache_for_dedup);
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
                    .count_traffic_vulnerabilities(Default::default())
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

    // 启动自动持久化任务（每30秒检查一次）
    let cache_for_persist = history_cache.clone();
    let db_for_persist = db_service.clone();
    let is_running_for_persist = state.is_running.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            let is_running = *is_running_for_persist.read().await;
            if !is_running {
                break;
            }

            // 检查是否需要持久化
            if cache_for_persist.should_auto_persist().await {
                let unpersisted = cache_for_persist.get_unpersisted_records().await;
                let count = unpersisted.len();

                if count > 0 {
                    tracing::info!("Auto-persisting {} records to database", count);

                    let mut saved = 0;
                    for record in unpersisted {
                        let cache_request_id = record.id;
                        let db_record = sentinel_db::ProxyRequestRecord {
                            id: None,
                            url: record.url,
                            host: record.host,
                            protocol: record.protocol,
                            method: record.method,
                            status_code: record.status_code as i32,
                            request_headers: record.request_headers,
                            request_body: record.request_body,
                            response_headers: record.response_headers,
                            response_body: record.response_body,
                            response_size: record.response_size,
                            response_time: record.response_time,
                            timestamp: record.timestamp,
                            request_body_compressed: false,
                            response_body_compressed: false,
                        };

                        if let Ok(db_request_id) =
                            db_for_persist.insert_proxy_request(&db_record).await
                        {
                            cache_for_persist
                                .set_http_request_db_id(cache_request_id, db_request_id)
                                .await;
                            saved += 1;
                        }
                    }

                    cache_for_persist.mark_persisted(saved).await;
                    tracing::info!("Auto-persisted {}/{} records successfully", saved, count);
                }
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
    crate::update_proxy_menu_text(app, true);

    tracing::info!(
        "Traffic scan started on port {} (ScanPipeline running in dedicated thread)",
        port
    );
    Ok(port)
}

/// 启动流量分析代理
#[tauri::command]
pub async fn start_traffic_analysis(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    config: Option<ProxyConfig>,
) -> Result<CommandResponse<u16>, String> {
    // Multi-point license verification
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Ok(CommandResponse::err(
            "License required for this feature".to_string(),
        ));
    }

    match start_traffic_analysis_internal(&app, &state, config).await {
        Ok(port) => Ok(CommandResponse::ok(port)),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 内部停止函数（可在内部和外部复用）
pub async fn stop_traffic_analysis_internal(
    app: &AppHandle,
    state: &TrafficAnalysisState,
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
    crate::update_proxy_menu_text(app, false);

    tracing::info!("Traffic scan stopped");
    Ok(())
}

/// 停止流量分析代理
#[tauri::command]
pub async fn stop_traffic_analysis(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    match stop_traffic_analysis_internal(&app, &state).await {
        Ok(_) => Ok(CommandResponse::ok("Proxy stopped".to_string())),
        Err(e) => Ok(CommandResponse::err(e)),
    }
}

/// 获取代理状态
#[tauri::command]
pub async fn get_proxy_status(
    state: State<'_, TrafficAnalysisState>,
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
    state: State<'_, TrafficAnalysisState>,
    limit: Option<i64>,
    offset: Option<i64>,
    severity_filter: Option<String>,
    plugin_id: Option<String>,
    status_filter: Option<String>,
    status_filters: Option<Vec<String>>,
    analysis_stage_filters: Option<Vec<String>>,
    search: Option<String>,
    semantic_source_filter: Option<String>,
    hypothesis_risk_type_filter: Option<String>,
    hypothesis_risk_type_filters: Option<Vec<String>>,
) -> Result<CommandResponse<Vec<TrafficFindingView>>, String> {
    let use_in_memory_filtering = super::finding_query_support::requires_in_memory_filtering(
        analysis_stage_filters.as_deref(),
        search.as_deref(),
        semantic_source_filter.as_deref(),
        hypothesis_risk_type_filter.as_deref(),
        hypothesis_risk_type_filters.as_deref(),
    );
    let db_filters = VulnerabilityFilters {
        severity: severity_filter,
        plugin_id,
        status: status_filter,
        status_in: status_filters,
        limit: if use_in_memory_filtering {
            None
        } else {
            Some(limit.unwrap_or(10))
        },
        offset: if use_in_memory_filtering {
            None
        } else {
            offset
        },
        ..Default::default()
    };

    let db_service = state.get_db_service();
    match db_service
        .list_traffic_vulnerabilities_with_evidence(db_filters)
        .await
    {
        Ok(records) => {
            let mut findings = records
                .into_iter()
                .filter(|record| {
                    super::finding_query_support::matches_finding_filters(
                        record,
                        analysis_stage_filters.as_deref(),
                        search.as_deref(),
                        semantic_source_filter.as_deref(),
                        hypothesis_risk_type_filter.as_deref(),
                        hypothesis_risk_type_filters.as_deref(),
                    )
                })
                .map(TrafficFindingView::from_record)
                .collect::<Vec<_>>();
            if use_in_memory_filtering {
                let start = offset.unwrap_or(0).max(0) as usize;
                let end = start.saturating_add(limit.unwrap_or(10).max(0) as usize);
                findings = findings
                    .into_iter()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .collect();
            }
            Ok(CommandResponse::ok(findings))
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
    state: State<'_, TrafficAnalysisState>,
    severity_filter: Option<String>,
    plugin_id: Option<String>,
    status_filter: Option<String>,
    status_filters: Option<Vec<String>>,
    analysis_stage_filters: Option<Vec<String>>,
    search: Option<String>,
    semantic_source_filter: Option<String>,
    hypothesis_risk_type_filter: Option<String>,
    hypothesis_risk_type_filters: Option<Vec<String>>,
) -> Result<CommandResponse<i64>, String> {
    let filters = VulnerabilityFilters {
        severity: severity_filter,
        plugin_id,
        status: status_filter,
        status_in: status_filters,
        ..Default::default()
    };

    let db_service = state.get_db_service();
    let use_in_memory_filtering = super::finding_query_support::requires_in_memory_filtering(
        analysis_stage_filters.as_deref(),
        search.as_deref(),
        semantic_source_filter.as_deref(),
        hypothesis_risk_type_filter.as_deref(),
        hypothesis_risk_type_filters.as_deref(),
    );
    if use_in_memory_filtering {
        match db_service
            .list_traffic_vulnerabilities_with_evidence(filters)
            .await
        {
            Ok(records) => {
                let count = records
                    .into_iter()
                    .filter(|record| {
                        super::finding_query_support::matches_finding_filters(
                            record,
                            analysis_stage_filters.as_deref(),
                            search.as_deref(),
                            semantic_source_filter.as_deref(),
                            hypothesis_risk_type_filter.as_deref(),
                            hypothesis_risk_type_filters.as_deref(),
                        )
                    })
                    .count() as i64;
                Ok(CommandResponse::ok(count))
            }
            Err(e) => {
                tracing::error!("Failed to count findings by lifecycle: {}", e);
                Ok(CommandResponse::err(format!("Database error: {}", e)))
            }
        }
    } else {
        match db_service.count_traffic_vulnerabilities(filters).await {
            Ok(count) => Ok(CommandResponse::ok(count)),
            Err(e) => {
                tracing::error!("Failed to count findings: {}", e);
                Ok(CommandResponse::err(format!("Database error: {}", e)))
            }
        }
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
/// 重放 Raw 请求（通过 TCP socket 直接发送原始字节）
#[tauri::command]
pub async fn replay_raw_request(
    host: String,
    port: u16,
    use_tls: bool,
    raw_request: String,
    timeout_secs: Option<u64>,
    follow_redirects: Option<bool>,
    max_redirects: Option<usize>,
    process_cookies_in_redirects: Option<bool>,
) -> Result<CommandResponse<RawReplayResult>, String> {
    let result = replay_raw_request_impl(RawReplayConfig {
        host,
        port,
        use_tls,
        raw_request,
        timeout_secs,
        follow_redirects: follow_redirects.unwrap_or(false),
        max_redirects: max_redirects.unwrap_or(5),
        process_cookies_in_redirects: process_cookies_in_redirects.unwrap_or(true),
    })
    .await?;

    Ok(CommandResponse::ok(result))
}
