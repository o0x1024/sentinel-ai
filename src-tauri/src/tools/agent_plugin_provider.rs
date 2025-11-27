//! Agent插件工具提供者
//!
//! 为Agent提供agent类别的插件作为工具：
//! - plugin::<plugin_id>: 每个启用的agent插件对应一个工具

use crate::commands::passive_scan_commands::PassiveScanState;
use crate::tools::plugin_parser;
use sentinel_tools::unified_types::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Agent插件工具提供者
#[derive(Debug, Clone)]
pub struct AgentPluginProvider {
    state: Arc<PassiveScanState>,
}

impl AgentPluginProvider {
    pub fn new(state: Arc<PassiveScanState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl ToolProvider for AgentPluginProvider {
    fn name(&self) -> &str {
        "agent_plugin"
    }

    fn description(&self) -> &str {
        "Agent plugin tools for AI-assisted security analysis"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();

        // 动态添加每个启用的agent类别插件
        let plugins = self
            .state
            .list_plugins_internal()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list plugins: {}", e))?;

        tracing::info!(
            "AgentPluginProvider: scanning {} plugins for agent tools",
            plugins.len()
        );

        for plugin in plugins {
            let id = plugin.metadata.id.clone();
            let main_category = plugin.metadata.main_category.clone();
            let status = format!("{:?}", plugin.status);

            // 只注册main_category='agent'且已启用的插件
            if plugin.status == sentinel_passive::PluginStatus::Enabled
                && plugin.metadata.main_category == "agent"
            {
                tracing::info!(
                    "AgentPluginProvider: register plugin tool => id={}, name='{}', main_category='{}', status={}",
                    id,
                    plugin.metadata.name,
                    main_category,
                    status
                );
                tools.push(Arc::new(AgentPluginTool::new(
                    self.state.clone(),
                    plugin.metadata.id.clone(),
                    plugin.metadata.name.clone(),
                    plugin.metadata.description.clone().unwrap_or_default(),
                )));
            } else {
                tracing::debug!(
                    "AgentPluginProvider: skip plugin id='{}' (main_category='{}', status={})",
                    id,
                    main_category,
                    status
                );
            }
        }

        tracing::info!("AgentPluginProvider: registered {} agent plugin tools", tools.len());
        Ok(tools)
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        // 去除 plugin:: 前缀（如果有）
        let plugin_id = name.strip_prefix("plugin::").unwrap_or(name);

        let plugins = self
            .state
            .list_plugins_internal()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list plugins: {}", e))?;

        tracing::debug!(
            "AgentPluginProvider::get_tool query='{}' (normalized id='{}'), scanning {} plugins",
            name,
            plugin_id,
            plugins.len()
        );

        for plugin in plugins {
            if plugin.status == sentinel_passive::PluginStatus::Enabled 
                && plugin.metadata.id == plugin_id
                && plugin.metadata.main_category == "agent" {
                tracing::info!(
                    "AgentPluginProvider::get_tool hit id='{}' (name='{}')",
                    plugin_id,
                    plugin.metadata.name
                );
                return Ok(Some(Arc::new(AgentPluginTool::new(
                    self.state.clone(),
                    plugin.metadata.id.clone(),
                    plugin.metadata.name.clone(),
                    plugin.metadata.description.clone().unwrap_or_default(),
                ))));
            } else {
                tracing::debug!(
                    "AgentPluginProvider::get_tool skip id='{}' (main_category='{}', status={:?})",
                    plugin.metadata.id,
                    plugin.metadata.main_category,
                    plugin.status
                );
            }
        }

        tracing::warn!(
            "AgentPluginProvider::get_tool not found for id='{}' (name query='{}')",
            plugin_id,
            name
        );
        Ok(None)
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        tracing::info!("Refreshing agent plugin tools");
        // 工具列表动态生成，无需显式刷新
        Ok(())
    }
}

// ============================================================================
// Agent插件工具实现
// ============================================================================

/// Agent插件工具
#[derive(Debug, Clone)]
struct AgentPluginTool {
    state: Arc<PassiveScanState>,
    plugin_id: String,
    plugin_name: String,
    plugin_description: String,
    full_tool_name: String, // 存储完整工具名称 "plugin::xxx"
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl AgentPluginTool {
    fn new(
        state: Arc<PassiveScanState>,
        plugin_id: String,
        plugin_name: String,
        plugin_description: String,
    ) -> Self {
        let full_tool_name = format!("plugin::{}", plugin_id);
        
        // 尝试从数据库获取插件代码并提取参数
        let (parameters, schema) = Self::extract_plugin_parameters(&state, &plugin_id);

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["agent".to_string(), "plugin".to_string(), plugin_id.clone()],
            install_command: None,
            requirements: vec![],
        };

        Self {
            state,
            plugin_id,
            plugin_name,
            plugin_description,
            full_tool_name,
            parameters,
            metadata,
        }
    }

    /// 从插件代码中提取参数定义
    fn extract_plugin_parameters(state: &Arc<PassiveScanState>, plugin_id: &str) -> (ToolParameters, serde_json::Value) {
        // 尝试同步获取数据库服务和插件代码
        let plugin_code = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let db = state.get_db_service().await.ok()?;
                db.get_plugin_code(plugin_id).await.ok()?
            })
        });

        if let Some(code) = plugin_code {
            // 解析插件代码，提取参数
            let parsed_params = plugin_parser::extract_parameters_from_code(&code);
            
            if !parsed_params.is_empty() {
                tracing::info!(
                    "AgentPluginTool: extracted {} parameters from plugin '{}'",
                    parsed_params.len(),
                    plugin_id
                );
                
                // 转换为 ToolParameters
                let mut parameters_vec = Vec::new();
                let mut required = Vec::new();
                let mut optional = Vec::new();
                
                for param in &parsed_params {
                    let param_type = match param.param_type.as_str() {
                        "string" => ParameterType::String,
                        "number" => ParameterType::Number,
                        "boolean" => ParameterType::Boolean,
                        "array" => ParameterType::Array,
                        _ => ParameterType::Object,
                    };
                    
                    parameters_vec.push(ParameterDefinition {
                        name: param.name.clone(),
                        description: param.description.clone()
                            .unwrap_or_else(|| format!("{} parameter", param.name)),
                        param_type,
                        required: param.required,
                        default_value: None,
                    });
                    
                    if param.required {
                        required.push(param.name.clone());
                    } else {
                        optional.push(param.name.clone());
                    }
                }
                
                let schema = plugin_parser::parameters_to_json_schema(&parsed_params);
                
                tracing::info!(
                    "AgentPluginTool: extracted {} parameters from plugin '{}': {:?}",
                    parsed_params.len(),
                    plugin_id,
                    parsed_params.iter().map(|p| format!("{}:{}", p.name, p.param_type)).collect::<Vec<_>>()
                );
                
                return (
                    ToolParameters {
                        parameters: parameters_vec,
                        schema: schema.clone(),
                        required,
                        optional,
                    },
                    schema
                );
            }
        }
        
        // 如果无法提取参数，使用默认的通用参数
        tracing::warn!(
            "AgentPluginTool: failed to extract parameters from plugin '{}', using default",
            plugin_id
        );
        
        let default_schema = json!({
            "type": "object",
            "properties": {
                "input": { 
                    "type": "object",
                    "description": "Flexible input parameters as key-value pairs",
                    "additionalProperties": true
                }
            }
        });
        
        (
            ToolParameters {
                parameters: vec![
                    ParameterDefinition {
                        name: "input".to_string(),
                        description: "Plugin input parameters (flexible key-value pairs)".to_string(),
                        param_type: ParameterType::Object,
                        required: false,
                        default_value: Some(json!({})),
                    },
                ],
                schema: default_schema.clone(),
                required: vec![],
                optional: vec!["input".to_string()],
            },
            default_schema
        )
    }
}

#[async_trait::async_trait]
impl UnifiedTool for AgentPluginTool {
    fn name(&self) -> &str {
        // 返回带 plugin:: 前缀的完整工具名称，与前端存储格式一致
        &self.full_tool_name
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
        let start_time = chrono::Utc::now();

        // 获取插件管理器
        let plugin_manager = self.state.get_plugin_manager();

        // 确保插件已在内存注册并且代码已缓存
        let db_service = self
            .state
            .get_db_service()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;

        if plugin_manager.get_plugin(&self.plugin_id).await.is_none() {
            let plugin = db_service
                .get_plugin_by_id(&self.plugin_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get plugin: {}", e))?;

            if let Some(p) = plugin {
                let name = p
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&self.plugin_id)
                    .to_string();
                let version = p
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1.0.0")
                    .to_string();
                let author = p.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                let main_category = p
                    .get("main_category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent")
                    .to_string();
                let category = p
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("custom")
                    .to_string();
                let description = p.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let default_severity = p
                    .get("default_severity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium")
                    .to_string();
                let tags: Vec<String> = p
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let plugin_code = p
                    .get("plugin_code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Plugin code not found: {}", self.plugin_id))?
                    .to_string();
                let enabled = p
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let severity = match default_severity.to_lowercase().as_str() {
                    "critical" => sentinel_passive::types::Severity::Critical,
                    "high" => sentinel_passive::types::Severity::High,
                    "medium" => sentinel_passive::types::Severity::Medium,
                    "low" => sentinel_passive::types::Severity::Low,
                    "info" => sentinel_passive::types::Severity::Info,
                    _ => sentinel_passive::types::Severity::Medium,
                };

                let metadata = sentinel_passive::PluginMetadata {
                    id: self.plugin_id.clone(),
                    name,
                    version,
                    author,
                    main_category,
                    category,
                    description,
                    default_severity: severity,
                    tags,
                };

                let _ = plugin_manager
                    .register_plugin(self.plugin_id.clone(), metadata, enabled)
                    .await;
                let _ = plugin_manager
                    .set_plugin_code(self.plugin_id.clone(), plugin_code)
                    .await;
            } else {
                let end_time = chrono::Utc::now();
                return Ok(ToolExecutionResult {
                    execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                    tool_name: format!("plugin::{}", self.plugin_id),
                    tool_id: format!("agent_plugin.{}", self.plugin_id),
                    success: false,
                    output: serde_json::json!({"error": "Plugin not found"}),
                    error: Some(format!("Plugin not found: {}", self.plugin_id)),
                    execution_time_ms: (end_time - start_time).num_milliseconds() as u64,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(end_time),
                    status: ExecutionStatus::Failed,
                });
            }
        }

        // 获取输入参数
        // 如果有 input 字段，使用它；否则使用整个 inputs 对象
        let plugin_input = if let Some(input_obj) = params.inputs.get("input") {
            input_obj.clone()
        } else {
            // 如果没有 input 字段，将所有参数作为输入
            json!(params.inputs)
        };

        // 执行 Agent 插件通用入口（优先 analyze / run / execute）
        let (findings, last_result) = plugin_manager
            .execute_agent(&self.plugin_id, &plugin_input)
            .await
            .map_err(|e| anyhow::anyhow!("Plugin execution failed: {}", e))?;

        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();
        let duration_ms = duration.as_millis() as u64;

        // 构建结果
        let output = json!({
            "plugin_id": self.plugin_id,
            "plugin_name": self.plugin_name,
            "findings": findings,
            "count": findings.len(),
            "success": true,
            "result": last_result,
        });

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: format!("plugin::{}", self.plugin_id),
            tool_id: format!("agent_plugin.{}", self.plugin_id),
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
