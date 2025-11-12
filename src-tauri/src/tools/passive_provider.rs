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
#[derive(Debug, Clone)]
pub struct PassiveToolProvider {
    state: Arc<PassiveScanState>,
}

impl PassiveToolProvider {
    pub fn new(state: Arc<PassiveScanState>) -> Self {
        Self { state }
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

        // 1. 添加 list_findings 聚合工具
        tools.push(Arc::new(ListFindingsTool::new(self.state.clone())));

        // 2. 动态添加每个启用插件的工具
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
        if name == "list_findings" {
            return Ok(Some(Arc::new(ListFindingsTool::new(self.state.clone()))));
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
