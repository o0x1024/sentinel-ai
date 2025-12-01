//! 被动扫描工具提供者
//!
//! 为 MCP 系统提供被动扫描相关工具：
//! - passive.list_findings: 列出漏洞发现（支持筛选）
//! - passive.<plugin_id>: 每个启用的插件对应一个离线分析工具

use super::*;
use crate::commands::passive_scan_commands::PassiveScanState;
use sentinel_tools::unified_types::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// 被动扫描工具提供者
#[derive(Clone)]
pub struct PassiveToolProvider {
    state: Arc<PassiveScanState>,
    app_handle: Option<tauri::AppHandle>,
}

impl std::fmt::Debug for PassiveToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassiveToolProvider")
            .field("state", &self.state)
            .field("app_handle", &"<AppHandle>")
            .finish()
    }
}

impl PassiveToolProvider {
    pub fn new(state: Arc<PassiveScanState>) -> Self {
        Self { 
            state,
            app_handle: None,
        }
    }
    
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }
}

#[async_trait::async_trait]
impl ToolProvider for PassiveToolProvider {
    fn name(&self) -> &str {
        "passive"
    }

    fn description(&self) -> &str {
        "Passive security scanning tools for analyzing captured HTTP/HTTPS traffic"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();

        // 1. 被动扫描控制工具
        tools.push(Arc::new(StartPassiveScanTool::new(self.state.clone(), self.app_handle.clone())));
        tools.push(Arc::new(StopPassiveScanTool::new(self.state.clone(), self.app_handle.clone())));
        tools.push(Arc::new(GetPassiveScanStatusTool::new(self.state.clone())));

        // 2. 漏洞查询工具
        tools.push(Arc::new(ListFindingsTool::new(self.state.clone())));
        tools.push(Arc::new(GetFindingDetailTool::new(self.state.clone())));
        tools.push(Arc::new(ReportVulnerabilityTool::new(self.state.clone())));

        // 3. 插件管理工具
        tools.push(Arc::new(ListPluginsTool::new(self.state.clone())));
        tools.push(Arc::new(EnablePluginTool::new(self.state.clone())));
        tools.push(Arc::new(DisablePluginTool::new(self.state.clone())));
        tools.push(Arc::new(LoadPluginTool::new(self.state.clone())));
        // Note: generate_plugin (方案A) 已删除，请使用 generate_advanced_plugin (方案B)

        // 4. 动态添加每个启用插件的工具
        let plugins = self.state.list_plugins_internal().await
            .map_err(|e| anyhow::anyhow!("Failed to list plugins: {}", e))?;
        
        for plugin in plugins {
            if plugin.status == sentinel_passive::PluginStatus::Enabled {
                tools.push(Arc::new(PluginAnalysisTool::new(
                    self.state.clone(),
                    plugin.metadata.id.clone(),
                    plugin.metadata.name.clone(),
                    plugin.metadata.description.clone().unwrap_or_default(),
                )));
            }
        }

        Ok(tools)
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        // 被动扫描控制工具
        match name {
            "start_passive_scan" => return Ok(Some(Arc::new(StartPassiveScanTool::new(self.state.clone(), self.app_handle.clone())))),
            "stop_passive_scan" => return Ok(Some(Arc::new(StopPassiveScanTool::new(self.state.clone(), self.app_handle.clone())))),
            "get_passive_scan_status" => return Ok(Some(Arc::new(GetPassiveScanStatusTool::new(self.state.clone())))),
            "list_findings" => return Ok(Some(Arc::new(ListFindingsTool::new(self.state.clone())))),
            "get_finding_detail" => return Ok(Some(Arc::new(GetFindingDetailTool::new(self.state.clone())))),
            "report_vulnerability" => return Ok(Some(Arc::new(ReportVulnerabilityTool::new(self.state.clone())))),
            "list_plugins" => return Ok(Some(Arc::new(ListPluginsTool::new(self.state.clone())))),
            "enable_plugin" => return Ok(Some(Arc::new(EnablePluginTool::new(self.state.clone())))),
            "disable_plugin" => return Ok(Some(Arc::new(DisablePluginTool::new(self.state.clone())))),
            "load_plugin" => return Ok(Some(Arc::new(LoadPluginTool::new(self.state.clone())))),
            _ => {}
        }

        // 检查是否是插件工具（格式：plugin_id）
        let plugins = self.state.list_plugins_internal().await
            .map_err(|e| anyhow::anyhow!("Failed to list plugins: {}", e))?;
        
        for plugin in plugins {
            if plugin.status == sentinel_passive::PluginStatus::Enabled && plugin.metadata.id == name {
                return Ok(Some(Arc::new(PluginAnalysisTool::new(
                    self.state.clone(),
                    plugin.metadata.id.clone(),
                    plugin.metadata.name.clone(),
                    plugin.metadata.description.clone().unwrap_or_default(),
                ))));
            }
        }

        Ok(None)
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        tracing::info!("Refreshing passive scan tools");
        // 工具列表动态生成，无需显式刷新
        Ok(())
    }
}

// ============================================================================
// 工具实现
// ============================================================================

// ===== 1. 被动扫描控制工具 =====

/// 启动被动扫描工具
#[derive(Debug, Clone)]
struct StartPassiveScanTool {
    state: Arc<PassiveScanState>,
    app_handle: Option<tauri::AppHandle>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl StartPassiveScanTool {
    fn new(state: Arc<PassiveScanState>, app_handle: Option<tauri::AppHandle>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "port".to_string(),
                    description: "Proxy port to listen on".to_string(),
                    param_type: ParameterType::Number,
                    required: false,
                    default_value: Some(json!(8080)),
                },
                ParameterDefinition {
                    name: "max_request_size".to_string(),
                    description: "Maximum request body size in bytes".to_string(),
                    param_type: ParameterType::Number,
                    required: false,
                    default_value: Some(json!(2097152)),
                },
                ParameterDefinition {
                    name: "max_response_size".to_string(),
                    description: "Maximum response body size in bytes".to_string(),
                    param_type: ParameterType::Number,
                    required: false,
                    default_value: Some(json!(2097152)),
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "port": { "type": "number", "default": 8080 },
                    "max_request_size": { "type": "number", "default": 2097152 },
                    "max_response_size": { "type": "number", "default": 2097152 }
                }
            }),
            required: vec![],
            optional: vec!["port".to_string(), "max_request_size".to_string(), "max_response_size".to_string()],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "control".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            app_handle,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for StartPassiveScanTool {
    fn name(&self) -> &str {
        "start_passive_scan"
    }

    fn description(&self) -> &str {
        "Start the passive scanning proxy server to intercept and analyze HTTP/HTTPS traffic"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        use sentinel_passive::{ProxyService, ProxyConfig, ScanPipeline, FindingDeduplicator};
        
        let start_time = chrono::Utc::now();

        // Parse parameters
        let port = params.inputs.get("port").and_then(|v| v.as_i64()).unwrap_or(8080) as u16;
        let max_request_size = params.inputs.get("max_request_size").and_then(|v| v.as_i64()).unwrap_or(2097152) as usize;
        let max_response_size = params.inputs.get("max_response_size").and_then(|v| v.as_i64()).unwrap_or(2097152) as usize;

        // Check if already running
        let is_running_lock = self.state.get_is_running();
        let mut is_running_guard = is_running_lock.write().await;
        if *is_running_guard {
            return Ok(ToolExecutionResult {
                execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                tool_name: "start_passive_scan".to_string(),
                tool_id: "passive.start_passive_scan".to_string(),
                success: false,
                output: json!({
                    "error": "Proxy already running",
                    "port": 0
                }),
                error: Some("Proxy already running".to_string()),
                execution_time_ms: 0,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(chrono::Utc::now()),
                status: ExecutionStatus::Failed,
            });
        }

        // Check if AppHandle is available
        let app_handle = match &self.app_handle {
            Some(app) => app.clone(),
            None => {
                return Ok(ToolExecutionResult {
                    execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                    tool_name: "start_passive_scan".to_string(),
                    tool_id: "passive.start_passive_scan".to_string(),
                    success: false,
                    output: json!({
                        "error": "AppHandle not available in tool context",
                        "note": "This tool requires Tauri application context"
                    }),
                    error: Some("AppHandle not available".to_string()),
                    execution_time_ms: 0,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(chrono::Utc::now()),
                    status: ExecutionStatus::Failed,
                });
            }
        };

        // Configure proxy
        let config = ProxyConfig {
            start_port: port,
            max_request_body_size: max_request_size,
            max_response_body_size: max_response_size,
            ..Default::default()
        };

        // Use current directory's ./ca directory for certificates
        let ca_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("ca");
        let proxy = ProxyService::with_ca_dir(config, ca_dir);

        // Create scan and finding channels
        let (scan_tx, scan_rx) = tokio::sync::mpsc::unbounded_channel();
        let (finding_tx, finding_rx) = tokio::sync::mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

        // Save scan_tx to state
        self.state.set_scan_tx(scan_tx.clone()).await;

        // Get database service
        let db_service = match self.state.get_db_service().await {
            Ok(db) => db,
            Err(e) => {
                return Ok(ToolExecutionResult {
                    execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                    tool_name: "start_passive_scan".to_string(),
                    tool_id: "passive.start_passive_scan".to_string(),
                    success: false,
                    output: json!({
                        "error": format!("Failed to initialize database: {}", e)
                    }),
                    error: Some(format!("Database error: {}", e)),
                    execution_time_ms: 0,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(chrono::Utc::now()),
                    status: ExecutionStatus::Failed,
                });
            }
        };

        // Start ScanPipeline in a separate thread
        let db_for_pipeline = db_service.clone();
        let app_for_pipeline = app_handle.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build runtime for ScanPipeline");
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
                }
            });
        });

        // Start FindingDeduplicator
        let deduplicator = FindingDeduplicator::with_database(finding_rx, db_service.clone())
            .with_event_sender(event_tx);
        tokio::spawn(async move {
            if let Err(e) = deduplicator.start().await {
                tracing::error!("FindingDeduplicator error: {}", e);
            }
        });

        // Start event forwarding to frontend
        // Note: Event broadcasting is handled by FindingDeduplicator internally
        tokio::spawn(async move {
            while let Some(_finding) = event_rx.recv().await {
                // Event handling can be added here if needed
            }
        });

        // Start proxy service
        let actual_port = match proxy.start(Some(scan_tx)).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolExecutionResult {
                    execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                    tool_name: "start_passive_scan".to_string(),
                    tool_id: "passive.start_passive_scan".to_string(),
                    success: false,
                    output: json!({
                        "error": format!("Failed to start proxy: {}", e)
                    }),
                    error: Some(format!("Proxy start failed: {}", e)),
                    execution_time_ms: 0,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(chrono::Utc::now()),
                    status: ExecutionStatus::Failed,
                });
            }
        };

        // Save proxy service to state
        {
            let proxy_service_lock = self.state.get_proxy_service();
            let mut proxy_guard = proxy_service_lock.write().await;
            *proxy_guard = Some(proxy);
        }

        // Update running status
        *is_running_guard = true;
        drop(is_running_guard);

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        tracing::info!("Passive scan started successfully on port {}", actual_port);

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "start_passive_scan".to_string(),
            tool_id: "passive.start_passive_scan".to_string(),
            success: true,
            output: json!({
                "message": "Passive scan started successfully",
                "port": actual_port,
                "config": {
                    "max_request_size": max_request_size,
                    "max_response_size": max_response_size
                }
            }),
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

/// 停止被动扫描工具
#[derive(Debug, Clone)]
struct StopPassiveScanTool {
    state: Arc<PassiveScanState>,
    app_handle: Option<tauri::AppHandle>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl StopPassiveScanTool {
    fn new(state: Arc<PassiveScanState>, app_handle: Option<tauri::AppHandle>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![],
            schema: json!({
                "type": "object",
                "properties": {}
            }),
            required: vec![],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "control".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            app_handle,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for StopPassiveScanTool {
    fn name(&self) -> &str {
        "stop_passive_scan"
    }

    fn description(&self) -> &str {
        "Stop the passive scanning proxy server"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        // Check if proxy is running
        let is_running_lock = self.state.get_is_running();
        let is_running = *is_running_lock.read().await;
        if !is_running {
            return Ok(ToolExecutionResult {
                execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                tool_name: "stop_passive_scan".to_string(),
                tool_id: "passive.stop_passive_scan".to_string(),
                success: false,
                output: json!({
                    "error": "Proxy not running"
                }),
                error: Some("Proxy not running".to_string()),
                execution_time_ms: 0,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(chrono::Utc::now()),
                status: ExecutionStatus::Failed,
            });
        }

        // Check if AppHandle is available
        let app_handle = match &self.app_handle {
            Some(app) => app.clone(),
            None => {
                return Ok(ToolExecutionResult {
                    execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                    tool_name: "stop_passive_scan".to_string(),
                    tool_id: "passive.stop_passive_scan".to_string(),
                    success: false,
                    output: json!({
                        "error": "AppHandle not available in tool context",
                        "note": "This tool requires Tauri application context"
                    }),
                    error: Some("AppHandle not available".to_string()),
                    execution_time_ms: 0,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(chrono::Utc::now()),
                    status: ExecutionStatus::Failed,
                });
            }
        };

        // Stop the proxy
        tracing::info!("Stopping passive scan proxy via tool");
        
        // Update the is_running flag
            let mut is_running_guard = is_running_lock.write().await;
            *is_running_guard = false;
            drop(is_running_guard);
            
        // Emit stop event using the events module
        use crate::events::{emit_proxy_status, ProxyStatusEvent};
        use sentinel_passive::ProxyStats;
        emit_proxy_status(
            &app_handle,
            ProxyStatusEvent {
                running: false,
                port: 0,
                mitm: false,
                stats: ProxyStats::default(),
            },
        );
            
            tracing::info!("Passive scan proxy stopped successfully");
            
            let end_time = chrono::Utc::now();
            let duration = (end_time - start_time).to_std().unwrap_or_default();
            
            Ok(ToolExecutionResult {
                execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                tool_name: "stop_passive_scan".to_string(),
                tool_id: "passive.stop_passive_scan".to_string(),
                success: true,
                output: json!({
                    "message": "Passive scan proxy stopped successfully"
                }),
                error: None,
                execution_time_ms: duration.as_millis() as u64,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(end_time),
                status: ExecutionStatus::Completed,
            })
    }
}

/// 获取被动扫描状态工具
#[derive(Debug, Clone)]
struct GetPassiveScanStatusTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl GetPassiveScanStatusTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![],
            schema: json!({
                "type": "object",
                "properties": {}
            }),
            required: vec![],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "status".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for GetPassiveScanStatusTool {
    fn name(&self) -> &str {
        "get_passive_scan_status"
    }

    fn description(&self) -> &str {
        "Get the current status of the passive scanning proxy"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let is_running_lock = self.state.get_is_running();
        let is_running = *is_running_lock.read().await;
        let proxy_service_lock = self.state.get_proxy_service();
        let proxy_opt = proxy_service_lock.read().await;

        let status = if is_running {
            if let Some(proxy) = proxy_opt.as_ref() {
                let port = proxy.get_port().await.unwrap_or(0);
                let stats = proxy.get_stats().await;
                json!({
                    "running": true,
                    "port": port,
                    "mitm_enabled": true,
                    "stats": {
                        "http_requests": stats.http_requests,
                        "https_requests": stats.https_requests,
                        "errors": stats.errors,
                        "qps": stats.qps
                    }
                })
            } else {
                json!({
                    "running": false,
                    "port": 0,
                    "mitm_enabled": false,
                    "stats": {
                        "http_requests": 0,
                        "https_requests": 0,
                        "errors": 0,
                        "qps": 0.0
                    }
                })
            }
        } else {
            json!({
                "running": false,
                "port": 0,
                "mitm_enabled": false,
                "stats": {
                    "http_requests": 0,
                    "https_requests": 0,
                    "errors": 0,
                    "qps": 0.0
                }
            })
        };

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "get_passive_scan_status".to_string(),
            tool_id: "passive.get_passive_scan_status".to_string(),
            success: true,
            output: status,
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

// ===== 2. 漏洞查询工具 =====

/// 获取漏洞详情工具
#[derive(Debug, Clone)]
struct GetFindingDetailTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl GetFindingDetailTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "finding_id".to_string(),
                    description: "ID of the vulnerability finding".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "finding_id": { "type": "string" }
                },
                "required": ["finding_id"]
            }),
            required: vec!["finding_id".to_string()],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "vulnerability".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for GetFindingDetailTool {
    fn name(&self) -> &str {
        "get_finding_detail"
    }

    fn description(&self) -> &str {
        "Get detailed information about a specific vulnerability finding including all evidence"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let finding_id = params.inputs.get("finding_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: finding_id"))?;

        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        let vulnerability = db_service.get_vulnerability_by_id(finding_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch vulnerability: {}", e))?;

        if vulnerability.is_none() {
            return Ok(ToolExecutionResult {
                execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                tool_name: "get_finding_detail".to_string(),
                tool_id: "passive.get_finding_detail".to_string(),
                success: false,
                output: json!({"error": "Finding not found"}),
                error: Some("Finding not found".to_string()),
                execution_time_ms: 0,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(chrono::Utc::now()),
                status: ExecutionStatus::Failed,
            });
        }

        let vulnerability = vulnerability.unwrap();
        let evidence = db_service.get_evidence_by_vuln_id(finding_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch evidence: {}", e))?;

        let output = json!({
            "vulnerability": vulnerability,
            "evidence": evidence
        });

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "get_finding_detail".to_string(),
            tool_id: "passive.get_finding_detail".to_string(),
            success: true,
            output,
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

// ===== 2.5 漏洞上报工具 =====

/// 上报漏洞工具 - 让 AI 在发现漏洞时写入数据库
#[derive(Debug, Clone)]
struct ReportVulnerabilityTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl ReportVulnerabilityTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "vuln_type".to_string(),
                    description: "Vulnerability type: sqli, xss, idor, path_traversal, command_injection, ssrf, xxe, csrf, auth_bypass, info_leak, etc.".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "severity".to_string(),
                    description: "Severity level: critical, high, medium, low, info".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "title".to_string(),
                    description: "Vulnerability title, e.g., 'SQL Injection in login.php'".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "url".to_string(),
                    description: "Affected URL".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "parameter".to_string(),
                    description: "Vulnerable parameter name".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "payload".to_string(),
                    description: "Payload used to exploit the vulnerability".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "evidence".to_string(),
                    description: "Evidence of exploitation (e.g., error message, response)".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "description".to_string(),
                    description: "Detailed description of the vulnerability".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "remediation".to_string(),
                    description: "Recommended fix for the vulnerability".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "vuln_type": { "type": "string", "enum": ["sqli", "xss", "idor", "path_traversal", "command_injection", "ssrf", "xxe", "csrf", "auth_bypass", "info_leak", "other"] },
                    "severity": { "type": "string", "enum": ["critical", "high", "medium", "low", "info"] },
                    "title": { "type": "string" },
                    "url": { "type": "string" },
                    "parameter": { "type": "string" },
                    "payload": { "type": "string" },
                    "evidence": { "type": "string" },
                    "description": { "type": "string" },
                    "remediation": { "type": "string" }
                },
                "required": ["vuln_type", "severity", "title", "url"]
            }),
            required: vec!["vuln_type".to_string(), "severity".to_string(), "title".to_string(), "url".to_string()],
            optional: vec!["parameter".to_string(), "payload".to_string(), "evidence".to_string(), "description".to_string(), "remediation".to_string()],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "vulnerability".to_string(), "report".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for ReportVulnerabilityTool {
    fn name(&self) -> &str {
        "report_vulnerability"
    }

    fn description(&self) -> &str {
        "Report a discovered vulnerability to the security center database. Use this when you confirm a vulnerability through testing."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        // 解析必需参数
        let vuln_type = params.inputs.get("vuln_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: vuln_type"))?;
        let severity = params.inputs.get("severity")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: severity"))?;
        let title = params.inputs.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;
        let url = params.inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;

        // 解析可选参数
        let parameter = params.inputs.get("parameter").and_then(|v| v.as_str());
        let payload = params.inputs.get("payload").and_then(|v| v.as_str());
        let evidence = params.inputs.get("evidence").and_then(|v| v.as_str());
        let description = params.inputs.get("description").and_then(|v| v.as_str());
        let remediation = params.inputs.get("remediation").and_then(|v| v.as_str());

        // 构建漏洞描述
        let full_description = format!(
            "{}{}{}",
            description.unwrap_or(""),
            parameter.map(|p| format!("\n\nVulnerable Parameter: {}", p)).unwrap_or_default(),
            payload.map(|p| format!("\n\nPayload: {}", p)).unwrap_or_default()
        );

        // 构建 PoC
        let poc = format!(
            "URL: {}\n{}{}",
            url,
            parameter.map(|p| format!("Parameter: {}\n", p)).unwrap_or_default(),
            payload.map(|p| format!("Payload: {}\n", p)).unwrap_or_default()
        );

        // 获取数据库服务
        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        // 创建 Finding 记录（使用 sentinel_plugins 的 Finding 类型）
        let vuln_id = uuid::Uuid::new_v4().to_string();
        let finding = sentinel_passive::Finding {
            id: vuln_id.clone(),
            plugin_id: "ai-agent".to_string(),
            vuln_type: vuln_type.to_string(),
            severity: Self::str_to_severity(severity),
            title: title.to_string(),
            description: if full_description.trim().is_empty() { 
                format!("{} vulnerability found at {}", vuln_type, url) 
            } else { 
                full_description 
            },
            evidence: evidence.unwrap_or("").to_string(),
            location: parameter.unwrap_or("").to_string(),
            confidence: sentinel_passive::Confidence::High,
            cwe: Self::vuln_type_to_cwe(vuln_type),
            owasp: Self::vuln_type_to_owasp(vuln_type),
            remediation: remediation.map(|s| s.to_string()),
            url: url.to_string(),
            method: "GET".to_string(),
            created_at: chrono::Utc::now(),
            request_headers: None,
            request_body: payload.map(|p| p.to_string()),
            response_status: None,
            response_headers: None,
            response_body: None,
        };

        // 保存到被动扫描数据库
        db_service.insert_vulnerability(&finding).await
            .map_err(|e| anyhow::anyhow!("Failed to save vulnerability: {}", e))?;

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        let output = json!({
            "success": true,
            "vulnerability_id": vuln_id,
            "message": format!("Vulnerability '{}' has been reported and saved to the security center", title),
            "severity": severity,
            "type": vuln_type,
            "url": url
        });

        tracing::info!("Reported vulnerability: {} ({})", title, vuln_id);

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "report_vulnerability".to_string(),
            tool_id: "passive.report_vulnerability".to_string(),
            success: true,
            output,
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

impl ReportVulnerabilityTool {
    /// 将字符串转换为 Severity 枚举
    fn str_to_severity(severity: &str) -> sentinel_passive::Severity {
        match severity.to_lowercase().as_str() {
            "critical" => sentinel_passive::Severity::Critical,
            "high" => sentinel_passive::Severity::High,
            "medium" => sentinel_passive::Severity::Medium,
            "low" => sentinel_passive::Severity::Low,
            _ => sentinel_passive::Severity::Info,
        }
    }

    /// 将漏洞类型转换为 CWE ID
    fn vuln_type_to_cwe(vuln_type: &str) -> Option<String> {
        match vuln_type.to_lowercase().as_str() {
            "sqli" => Some("CWE-89".to_string()),
            "xss" => Some("CWE-79".to_string()),
            "idor" => Some("CWE-639".to_string()),
            "path_traversal" => Some("CWE-22".to_string()),
            "command_injection" => Some("CWE-78".to_string()),
            "ssrf" => Some("CWE-918".to_string()),
            "xxe" => Some("CWE-611".to_string()),
            "csrf" => Some("CWE-352".to_string()),
            "auth_bypass" => Some("CWE-287".to_string()),
            "info_leak" => Some("CWE-200".to_string()),
            _ => None,
        }
    }

    /// 将漏洞类型转换为 OWASP 类别
    fn vuln_type_to_owasp(vuln_type: &str) -> Option<String> {
        match vuln_type.to_lowercase().as_str() {
            "sqli" => Some("A03:2021-Injection".to_string()),
            "xss" => Some("A03:2021-Injection".to_string()),
            "idor" => Some("A01:2021-Broken Access Control".to_string()),
            "path_traversal" => Some("A01:2021-Broken Access Control".to_string()),
            "command_injection" => Some("A03:2021-Injection".to_string()),
            "ssrf" => Some("A10:2021-SSRF".to_string()),
            "xxe" => Some("A05:2021-Security Misconfiguration".to_string()),
            "csrf" => Some("A01:2021-Broken Access Control".to_string()),
            "auth_bypass" => Some("A07:2021-Identification and Authentication Failures".to_string()),
            "info_leak" => Some("A01:2021-Broken Access Control".to_string()),
            _ => None,
        }
    }
}

// ===== 3. 插件管理工具 =====

/// 列出插件工具
#[derive(Debug, Clone)]
struct ListPluginsTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl ListPluginsTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![],
            schema: json!({
                "type": "object",
                "properties": {}
            }),
            required: vec![],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "plugin".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for ListPluginsTool {
    fn name(&self) -> &str {
        "list_plugins"
    }

    fn description(&self) -> &str {
        "List all available passive scanning plugins"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let plugins = self.state.list_plugins_internal().await
            .map_err(|e| anyhow::anyhow!("Failed to list plugins: {}", e))?;

        let output = json!({
            "plugins": plugins,
            "count": plugins.len()
        });

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "list_plugins".to_string(),
            tool_id: "passive.list_plugins".to_string(),
            success: true,
            output,
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

/// 启用插件工具
#[derive(Debug, Clone)]
struct EnablePluginTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl EnablePluginTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "plugin_id".to_string(),
                    description: "ID of the plugin to enable".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": { "type": "string" }
                },
                "required": ["plugin_id"]
            }),
            required: vec!["plugin_id".to_string()],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "plugin".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for EnablePluginTool {
    fn name(&self) -> &str {
        "enable_plugin"
    }

    fn description(&self) -> &str {
        "Enable a passive scanning plugin"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let plugin_id = params.inputs.get("plugin_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plugin_id"))?;

        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
            .bind(true)
            .bind(plugin_id)
            .execute(db_service.pool())
            .await;

        let (success, output, error) = match result {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!("Plugin enabled via MCP tool: {}", plugin_id);
                    (true, json!({"plugin_id": plugin_id, "enabled": true}), None)
                } else {
                    (false, json!({"error": "Plugin not found"}), Some("Plugin not found".to_string()))
                }
            }
            Err(e) => (false, json!({"error": e.to_string()}), Some(e.to_string())),
        };

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "enable_plugin".to_string(),
            tool_id: "passive.enable_plugin".to_string(),
            success,
            output,
            error,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: if success { ExecutionStatus::Completed } else { ExecutionStatus::Failed },
        })
    }
}

/// 禁用插件工具
#[derive(Debug, Clone)]
struct DisablePluginTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl DisablePluginTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "plugin_id".to_string(),
                    description: "ID of the plugin to disable".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": { "type": "string" }
                },
                "required": ["plugin_id"]
            }),
            required: vec!["plugin_id".to_string()],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "plugin".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for DisablePluginTool {
    fn name(&self) -> &str {
        "disable_plugin"
    }

    fn description(&self) -> &str {
        "Disable a passive scanning plugin"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let plugin_id = params.inputs.get("plugin_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plugin_id"))?;

        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        let result = sqlx::query("UPDATE plugin_registry SET enabled = ? WHERE id = ?")
            .bind(false)
            .bind(plugin_id)
            .execute(db_service.pool())
            .await;

        let (success, output, error) = match result {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!("Plugin disabled via MCP tool: {}", plugin_id);
                    (true, json!({"plugin_id": plugin_id, "enabled": false}), None)
                } else {
                    (false, json!({"error": "Plugin not found"}), Some("Plugin not found".to_string()))
                }
            }
            Err(e) => (false, json!({"error": e.to_string()}), Some(e.to_string())),
        };

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "disable_plugin".to_string(),
            tool_id: "passive.disable_plugin".to_string(),
            success,
            output,
            error,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: if success { ExecutionStatus::Completed } else { ExecutionStatus::Failed },
        })
    }
}

/// 加载插件工具
#[derive(Debug, Clone)]
struct LoadPluginTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl LoadPluginTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "plugin_code".to_string(),
                    description: "TypeScript plugin code".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "metadata".to_string(),
                    description: "Plugin metadata (JSON object with id, name, version, category, description, etc.)".to_string(),
                    param_type: ParameterType::Object,
                    required: true,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "plugin_code": { "type": "string" },
                    "metadata": { "type": "object" }
                },
                "required": ["plugin_code", "metadata"]
            }),
            required: vec!["plugin_code".to_string(), "metadata".to_string()],
            optional: vec![],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "plugin".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for LoadPluginTool {
    fn name(&self) -> &str {
        "load_plugin"
    }

    fn description(&self) -> &str {
        "Load a new passive scanning plugin into the database"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::VulnerabilityScanning
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = chrono::Utc::now();

        let plugin_code = params.inputs.get("plugin_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plugin_code"))?;

        let metadata_value = params.inputs.get("metadata")
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: metadata"))?;

        let plugin_metadata: sentinel_passive::PluginMetadata = serde_json::from_value(metadata_value.clone())
            .map_err(|e| anyhow::anyhow!("Invalid plugin metadata: {}", e))?;

        let plugin_id = plugin_metadata.id.clone();

        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        let result = db_service.register_plugin_with_code(&plugin_metadata, plugin_code).await;

        let (success, output, error) = match result {
            Ok(_) => {
                tracing::info!("Plugin loaded via MCP tool: {}", plugin_id);
                (true, json!({"plugin_id": plugin_id, "message": "Plugin loaded successfully"}), None)
            }
            Err(e) => (false, json!({"error": e.to_string()}), Some(e.to_string())),
        };

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "load_plugin".to_string(),
            tool_id: "passive.load_plugin".to_string(),
            success,
            output,
            error,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: if success { ExecutionStatus::Completed } else { ExecutionStatus::Failed },
        })
    }
}

// ============================================================
// 方案A的 GeneratePluginTool 已删除
// 请使用方案B的 generate_advanced_plugin (在 generator_tools.rs 中)
// ============================================================

/// 列出漏洞发现工具
#[derive(Debug, Clone)]
struct ListFindingsTool {
    state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl ListFindingsTool {
    fn new(state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "vuln_type".to_string(),
                    description: "Filter by vulnerability type (e.g., 'sqli', 'xss', 'sensitive_info')".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "severity".to_string(),
                    description: "Filter by severity level".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "status".to_string(),
                    description: "Filter by status".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "plugin_id".to_string(),
                    description: "Filter by plugin ID (e.g., 'builtin.sqli')".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "limit".to_string(),
                    description: "Maximum number of findings to return".to_string(),
                    param_type: ParameterType::Number,
                    required: false,
                    default_value: Some(json!(100)),
                },
                ParameterDefinition {
                    name: "offset".to_string(),
                    description: "Number of findings to skip (for pagination)".to_string(),
                    param_type: ParameterType::Number,
                    required: false,
                    default_value: Some(json!(0)),
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "vuln_type": { "type": "string" },
                    "severity": { "type": "string" },
                    "status": { "type": "string" },
                    "plugin_id": { "type": "string" },
                    "limit": { "type": "number", "default": 100 },
                    "offset": { "type": "number", "default": 0 }
                }
            }),
            required: vec![],
            optional: vec!["vuln_type".to_string(), "severity".to_string(), "status".to_string(), 
                          "plugin_id".to_string(), "limit".to_string(), "offset".to_string()],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "vulnerability".to_string()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for ListFindingsTool {
    fn name(&self) -> &str {
        "list_findings"
    }

    fn description(&self) -> &str {
        "List all vulnerability findings from passive scanning with optional filters"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        use sentinel_passive::VulnerabilityFilters;

        // 解析参数
        let vuln_type = params.inputs.get("vuln_type").and_then(|v| v.as_str()).map(String::from);
        let severity = params.inputs.get("severity").and_then(|v| v.as_str()).map(String::from);
        let status = params.inputs.get("status").and_then(|v| v.as_str()).map(String::from);
        let plugin_id = params.inputs.get("plugin_id").and_then(|v| v.as_str()).map(String::from);
        let limit = params.inputs.get("limit").and_then(|v| v.as_i64()).or(Some(100));
        let offset = params.inputs.get("offset").and_then(|v| v.as_i64()).or(Some(0));

        let filters = VulnerabilityFilters {
            vuln_type,
            severity,
            status,
            plugin_id,
            limit,
            offset,
        };

        // 查询数据库
        let db_service = self.state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        let findings = db_service.list_vulnerabilities(filters.clone()).await
            .map_err(|e| anyhow::anyhow!("Failed to list vulnerabilities: {}", e))?;

        let total = db_service.count_vulnerabilities(filters).await
            .map_err(|e| anyhow::anyhow!("Failed to count vulnerabilities: {}", e))?;

        let result = json!({
            "findings": findings,
            "total": total,
            "count": findings.len(),
        });

        let end_time = chrono::Utc::now();
        let duration_ms = (end_time - chrono::Utc::now()).num_milliseconds().unsigned_abs();

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "list_findings".to_string(),
            tool_id: "passive.list_findings".to_string(),
            success: true,
            output: result.clone(),
            error: None,
            execution_time_ms: duration_ms,
            metadata: HashMap::new(),
            started_at: chrono::Utc::now(),
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

/// 插件离线分析工具
#[derive(Debug, Clone)]
struct PluginAnalysisTool {
    state: Arc<PassiveScanState>,
    plugin_id: String,
    plugin_name: String,
    plugin_description: String,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl PluginAnalysisTool {
    fn new(
        state: Arc<PassiveScanState>,
        plugin_id: String,
        plugin_name: String,
        plugin_description: String,
    ) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "url".to_string(),
                    description: "URL to analyze".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "method".to_string(),
                    description: "HTTP method".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: Some(json!("GET")),
                },
                ParameterDefinition {
                    name: "headers".to_string(),
                    description: "HTTP headers (JSON object)".to_string(),
                    param_type: ParameterType::Object,
                    required: false,
                    default_value: Some(json!({})),
                },
                ParameterDefinition {
                    name: "body".to_string(),
                    description: "Request/response body to analyze".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: Some(json!("")),
                },
                ParameterDefinition {
                    name: "params".to_string(),
                    description: "URL parameters (JSON object)".to_string(),
                    param_type: ParameterType::Object,
                    required: false,
                    default_value: Some(json!({})),
                },
                ParameterDefinition {
                    name: "analysis_type".to_string(),
                    description: "Type of analysis: 'request' or 'response'".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default_value: Some(json!("request")),
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "method": { "type": "string", "default": "GET" },
                    "headers": { "type": "object", "default": {} },
                    "body": { "type": "string", "default": "" },
                    "params": { "type": "object", "default": {} },
                    "analysis_type": { "type": "string", "default": "request" }
                },
                "required": ["url"]
            }),
            required: vec!["url".to_string()],
            optional: vec!["method".to_string(), "headers".to_string(), "body".to_string(),
                          "params".to_string(), "analysis_type".to_string()],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["passive".to_string(), "plugin".to_string(), plugin_id.clone()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            plugin_id,
            plugin_name,
            plugin_description,
            parameters,
            metadata,
        }
    }
}

#[async_trait::async_trait]
impl UnifiedTool for PluginAnalysisTool {
    fn name(&self) -> &str {
        &self.plugin_id
    }

    fn description(&self) -> &str {
        &self.plugin_description
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        use sentinel_passive::{RequestContext, ResponseContext};
        use std::collections::HashMap;

        let start_time = chrono::Utc::now();

        // 解析参数
        let url = params.inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;

        let analysis_type = params.inputs.get("analysis_type")
            .and_then(|v| v.as_str())
            .unwrap_or("request");

        // 获取插件引擎
        let plugin_manager = self.state.get_plugin_manager();

        // 执行插件分析
        let findings = if analysis_type == "request" {
            // 构建请求上下文
            let method = params.inputs.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string();
            let headers = params.inputs.get("headers")
                .and_then(|v| serde_json::from_value::<HashMap<String, String>>(v.clone()).ok())
                .unwrap_or_default();
            let params_map = params.inputs.get("params")
                .and_then(|v| serde_json::from_value::<HashMap<String, String>>(v.clone()).ok())
                .unwrap_or_default();
            let body_str = params.inputs.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let body = body_str.as_bytes().to_vec();

            let request_ctx = RequestContext {
                id: uuid::Uuid::new_v4().to_string(),
                method,
                url: url.to_string(),
                headers,
                body,
                content_type: Some("text/plain".to_string()),
                query_params: params_map,
                is_https: url.starts_with("https://"),
                timestamp: chrono::Utc::now(),
            };

            plugin_manager.scan_request(&self.plugin_id, &request_ctx).await
                .map_err(|e| anyhow::anyhow!("Plugin execution failed: {}", e))?
        } else {
            // 构建响应上下文
            let status = params.inputs.get("status").and_then(|v| v.as_i64()).unwrap_or(200) as u16;
            let headers = params.inputs.get("headers")
                .and_then(|v| serde_json::from_value::<HashMap<String, String>>(v.clone()).ok())
                .unwrap_or_default();
            let body_str = params.inputs.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let body = body_str.as_bytes().to_vec();

            let response_ctx = ResponseContext {
                request_id: uuid::Uuid::new_v4().to_string(),
                status,
                headers,
                body,
                content_type: Some("text/plain".to_string()),
                timestamp: chrono::Utc::now(),
            };

            plugin_manager.scan_response(&self.plugin_id, &response_ctx).await
                .map_err(|e| anyhow::anyhow!("Plugin execution failed: {}", e))?
        };

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();
        let duration_ms = duration.as_millis() as u64;

        let output = json!({
            "plugin_id": self.plugin_id,
            "plugin_name": self.plugin_name,
            "analysis_type": analysis_type,
            "findings": findings,
            "count": findings.len(),
        });

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: self.plugin_id.clone(),
            tool_id: format!("passive.{}", self.plugin_id),
            success: true,
            output,
            error: None,
            execution_time_ms: duration_ms,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}
