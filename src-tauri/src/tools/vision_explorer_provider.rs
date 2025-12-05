//! VisionExplorer 工具提供者
//!
//! 将 VisionExplorer 作为通用工具提供给所有架构使用
//!
//! ## 功能
//! - 提供 `vision_explore` 工具用于 VLM 驱动的网站探索
//! - 支持 API 端点发现、表单枚举、攻击面分析
//! - 可被任何架构（ReAct、LlmCompiler、PlanAndExecute、Travel 等）调用

use super::unified_types::*;
use crate::engines::vision_explorer::{VisionExplorer, VisionExplorerConfig, ExplorationSummary};
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// VisionExplorer 工具提供者
// ============================================================================

/// VisionExplorer 工具提供者
/// 
/// 将 VisionExplorer 能力作为通用工具暴露给所有架构
#[derive(Debug)]
pub struct VisionExplorerProvider {
    /// MCP 服务（用于 Playwright 浏览器控制）
    mcp_service: Arc<McpService>,
    /// 工具实例
    tool: Arc<VisionExploreTool>,
}

impl VisionExplorerProvider {
    /// 创建新的 VisionExplorer 工具提供者
    pub fn new(
        mcp_service: Arc<McpService>,
        llm_provider: String,
        llm_model: String,
    ) -> Self {
        let tool = Arc::new(VisionExploreTool::new(
            mcp_service.clone(),
            llm_provider,
            llm_model,
        ));
        
        Self {
            mcp_service,
            tool,
        }
    }
    
    /// 检查 Playwright MCP 服务器是否可用
    pub async fn is_playwright_available(&self) -> bool {
        match self.mcp_service.get_connection_info().await {
            Ok(connections) => {
                connections.iter().any(|c| 
                    c.name.to_lowercase().contains("playwright") && c.status == "connected"
                )
            }
            Err(_) => false,
        }
    }
}

#[async_trait]
impl ToolProvider for VisionExplorerProvider {
    fn name(&self) -> &str {
        "vision_explorer"
    }

    fn description(&self) -> &str {
        "VLM-driven website exploration tool for API discovery and attack surface analysis"
    }

    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>> {
        Ok(vec![self.tool.clone()])
    }

    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>> {
        if name == "vision_explore" {
            Ok(Some(self.tool.clone()))
        } else {
            Ok(None)
        }
    }

    async fn refresh(&self) -> Result<()> {
        debug!("VisionExplorer provider refresh (no-op)");
        Ok(())
    }

    async fn is_available(&self) -> bool {
        self.is_playwright_available().await
    }
}

// ============================================================================
// VisionExplore 工具实现
// ============================================================================

/// VisionExplore 工具
/// 
/// 使用 VLM 驱动的网站探索，发现 API 端点和攻击面
#[derive(Debug)]
pub struct VisionExploreTool {
    mcp_service: Arc<McpService>,
    llm_provider: String,
    llm_model: String,
    parameters: ToolParameters,
    metadata: ToolMetadata,
    /// 运行时配置（可通过 set_runtime_config 动态设置）
    runtime_config: Arc<RwLock<VisionExploreRuntimeConfig>>,
}

/// 运行时配置（用于传递 AppHandle、被动扫描状态等）
#[derive(Clone)]
pub struct VisionExploreRuntimeConfig {
    /// Tauri AppHandle（用于前端消息推送）
    pub app_handle: Option<tauri::AppHandle>,
    /// 被动扫描状态
    pub passive_scan_state: Option<Arc<crate::commands::passive_scan_commands::PassiveScanState>>,
    /// 被动扫描数据库服务
    pub passive_db: Option<Arc<sentinel_passive::PassiveDatabaseService>>,
    /// 取消令牌
    pub cancellation_token: Option<tokio_util::sync::CancellationToken>,
    /// 执行 ID
    pub execution_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 是否启用多模态模式（截图）
    /// true: 多模态模式，发送截图给 VLM
    /// false: 文本模式，发送元素列表给 LLM
    pub enable_multimodal: bool,
}

impl Default for VisionExploreRuntimeConfig {
    fn default() -> Self {
        Self {
            app_handle: None,
            passive_scan_state: None,
            passive_db: None,
            cancellation_token: None,
            execution_id: None,
            message_id: None,
            conversation_id: None,
            enable_multimodal: true, // 默认启用多模态
        }
    }
}

impl std::fmt::Debug for VisionExploreRuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionExploreRuntimeConfig")
            .field("has_app_handle", &self.app_handle.is_some())
            .field("has_passive_scan_state", &self.passive_scan_state.is_some())
            .field("has_passive_db", &self.passive_db.is_some())
            .field("has_cancellation_token", &self.cancellation_token.is_some())
            .field("execution_id", &self.execution_id)
            .field("message_id", &self.message_id)
            .field("conversation_id", &self.conversation_id)
            .field("enable_multimodal", &self.enable_multimodal)
            .finish()
    }
}

impl VisionExploreTool {
    pub fn new(
        mcp_service: Arc<McpService>,
        llm_provider: String,
        llm_model: String,
    ) -> Self {
        // 构建参数定义
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "target_url".to_string(),
                    param_type: ParameterType::String,
                    description: "The target URL to explore".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "max_iterations".to_string(),
                    param_type: ParameterType::Number,
                    description: "Maximum exploration iterations (default: 50)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(50)),
                },
                ParameterDefinition {
                    name: "focus_areas".to_string(),
                    param_type: ParameterType::Array,
                    description: "Specific areas to focus on (e.g., 'authentication', 'api', 'admin')".to_string(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "viewport_width".to_string(),
                    param_type: ParameterType::Number,
                    description: "Browser viewport width (default: 1920)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(1920)),
                },
                ParameterDefinition {
                    name: "viewport_height".to_string(),
                    param_type: ParameterType::Number,
                    description: "Browser viewport height (default: 1080)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(1080)),
                },
            ],
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target_url": {
                        "type": "string",
                        "description": "The target URL to explore"
                    },
                    "max_iterations": {
                        "type": "integer",
                        "description": "Maximum exploration iterations (default: 50)"
                    },
                    "focus_areas": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Specific areas to focus on"
                    },
                    "viewport_width": {
                        "type": "integer",
                        "description": "Browser viewport width"
                    },
                    "viewport_height": {
                        "type": "integer",
                        "description": "Browser viewport height"
                    }
                },
                "required": ["target_url"]
            }),
            required: vec!["target_url".to_string()],
            optional: vec![
                "max_iterations".to_string(),
                "focus_areas".to_string(),
                "viewport_width".to_string(),
                "viewport_height".to_string(),
            ],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec![
                "vlm".to_string(),
                "browser".to_string(),
                "reconnaissance".to_string(),
                "api-discovery".to_string(),
                "attack-surface".to_string(),
            ],
            install_command: None,
            requirements: vec!["playwright".to_string(), "vlm".to_string()],
        };

        Self {
            mcp_service,
            llm_provider,
            llm_model,
            parameters,
            metadata,
            runtime_config: Arc::new(RwLock::new(VisionExploreRuntimeConfig::default())),
        }
    }

    /// 设置运行时配置
    pub async fn set_runtime_config(&self, config: VisionExploreRuntimeConfig) {
        let mut runtime_config = self.runtime_config.write().await;
        *runtime_config = config;
    }

    /// 获取运行时配置的克隆
    pub async fn get_runtime_config(&self) -> VisionExploreRuntimeConfig {
        self.runtime_config.read().await.clone()
    }

    /// 检查 Playwright 是否可用
    async fn is_playwright_available(&self) -> bool {
        match self.mcp_service.get_connection_info().await {
            Ok(connections) => {
                connections.iter().any(|c| 
                    c.name.to_lowercase().contains("playwright") && c.status == "connected"
                )
            }
            Err(_) => false,
        }
    }
}

#[async_trait]
impl UnifiedTool for VisionExploreTool {
    fn name(&self) -> &str {
        "vision_explore"
    }

    fn description(&self) -> &str {
        "Use VLM-driven exploration to discover API endpoints and attack surface of a target website. \
         Simulates human interaction (clicking, typing, navigating) to discover all accessible functionality."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Reconnaissance
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        self.is_playwright_available().await
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let start_time = Utc::now();
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);

        info!("VisionExploreTool: Starting execution {}", execution_id);

        // 检查 Playwright 是否可用
        if !self.is_playwright_available().await {
            return Ok(ToolExecutionResult {
                execution_id,
                tool_name: "vision_explore".to_string(),
                tool_id: "vision_explore".to_string(),
                success: false,
                output: serde_json::json!({
                    "error": "Playwright MCP server not available",
                    "suggestion": "Please ensure Playwright MCP server is connected"
                }),
                error: Some("Playwright MCP server not available".to_string()),
                execution_time_ms: 0,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(Utc::now()),
                status: ExecutionStatus::Failed,
            });
        }

        // 提取参数
        let target_url = params.inputs.get("target_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("target_url is required"))?;

        let max_iterations = params.inputs.get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as u32;

        let viewport_width = params.inputs.get("viewport_width")
            .and_then(|v| v.as_u64())
            .unwrap_or(1920) as u32;

        let viewport_height = params.inputs.get("viewport_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(1080) as u32;

        // 从上下文中获取消息相关信息（如果有）
        let ctx_execution_id = params.context.get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let ctx_message_id = params.context.get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let ctx_conversation_id = params.context.get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 获取运行时配置
        let runtime_config = self.runtime_config.read().await.clone();

        // 创建 VisionExplorer 配置
        let explorer_config = VisionExplorerConfig {
            target_url: target_url.to_string(),
            max_iterations,
            viewport_width,
            viewport_height,
            execution_id: ctx_execution_id.or(runtime_config.execution_id),
            message_id: ctx_message_id.or(runtime_config.message_id),
            conversation_id: ctx_conversation_id.or(runtime_config.conversation_id),
            finalize_on_complete: false, // 作为工具调用，不终结消息流
            enable_multimodal: runtime_config.enable_multimodal, // 从运行时配置读取
            ..Default::default()
        };

        // 创建 VisionExplorer 实例
        let mut explorer = VisionExplorer::with_ai_config(
            explorer_config,
            self.mcp_service.clone(),
            self.llm_provider.clone(),
            self.llm_model.clone(),
        );

        // 传入运行时依赖
        if let Some(app) = &runtime_config.app_handle {
            explorer = explorer.with_app_handle(app.clone());
        }
        if let Some(state) = &runtime_config.passive_scan_state {
            explorer = explorer.with_passive_scan_state(state.clone());
        }
        if let Some(db) = &runtime_config.passive_db {
            explorer = explorer.with_passive_db(db.clone());
        }
        if let Some(token) = &runtime_config.cancellation_token {
            explorer = explorer.with_cancellation_token(token.clone());
        }

        // 执行探索
        match explorer.start().await {
            Ok(summary) => {
                let state = explorer.get_state().await;
                let duration = (Utc::now() - start_time).to_std().unwrap_or(Duration::from_secs(0));

                info!(
                    "VisionExploreTool: Exploration completed - {} APIs, {} pages, {} iterations",
                    summary.apis_discovered, summary.pages_visited, summary.total_iterations
                );

                // 构建输出
                let output = serde_json::json!({
                    "status": "completed",
                    "target_url": target_url,
                    "summary": {
                        "pages_visited": summary.pages_visited,
                        "apis_discovered": summary.apis_discovered,
                        "total_iterations": summary.total_iterations,
                        "exploration_progress": summary.exploration_progress,
                    },
                    "discovered_apis": state.discovered_apis.iter().map(|api| {
                        serde_json::json!({
                            "url": api.full_url,
                            "method": api.method,
                            "path": api.path,
                            "parameters": api.parameters,
                        })
                    }).collect::<Vec<_>>(),
                    "discovered_forms": state.discovered_forms.iter().map(|form| {
                        serde_json::json!({
                            "action": form.action,
                            "method": form.method,
                            "fields": form.fields.iter().map(|f| {
                                serde_json::json!({
                                    "name": f.name,
                                    "type": f.field_type,
                                    "required": f.required,
                                })
                            }).collect::<Vec<_>>(),
                        })
                    }).collect::<Vec<_>>(),
                });

                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: "vision_explore".to_string(),
                    tool_id: "vision_explore".to_string(),
                    success: true,
                    output,
                    error: None,
                    execution_time_ms: duration.as_millis() as u64,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    status: ExecutionStatus::Completed,
                })
            }
            Err(e) => {
                let duration = (Utc::now() - start_time).to_std().unwrap_or(Duration::from_secs(0));
                error!("VisionExploreTool: Exploration failed - {}", e);

                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: "vision_explore".to_string(),
                    tool_id: "vision_explore".to_string(),
                    success: false,
                    output: serde_json::json!({
                        "error": e.to_string(),
                        "target_url": target_url,
                    }),
                    error: Some(e.to_string()),
                    execution_time_ms: duration.as_millis() as u64,
                    metadata: HashMap::new(),
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    status: ExecutionStatus::Failed,
                })
            }
        }
    }
}

// ============================================================================
// 工厂函数
// ============================================================================

/// 创建 VisionExplorer 工具提供者
/// 
/// 如果 Playwright MCP 可用，返回 Some；否则返回 None
pub async fn create_vision_explorer_provider(
    mcp_service: Arc<McpService>,
    llm_provider: String,
    llm_model: String,
) -> Result<Option<Box<dyn ToolProvider>>> {
    let provider = VisionExplorerProvider::new(mcp_service, llm_provider, llm_model);
    
    // 检查是否可用
    if provider.is_playwright_available().await {
        info!("VisionExplorer provider created successfully");
        Ok(Some(Box::new(provider)))
    } else {
        debug!("VisionExplorer provider not available (Playwright not connected)");
        Ok(None)
    }
}

/// 全局 VisionExplore 工具实例（用于运行时配置）
static GLOBAL_VISION_EXPLORE_TOOL: std::sync::OnceLock<Arc<VisionExploreTool>> = std::sync::OnceLock::new();

/// 初始化全局 VisionExplore 工具
pub fn initialize_global_vision_explore_tool(
    mcp_service: Arc<McpService>,
    llm_provider: String,
    llm_model: String,
) -> Arc<VisionExploreTool> {
    GLOBAL_VISION_EXPLORE_TOOL.get_or_init(|| {
        Arc::new(VisionExploreTool::new(mcp_service, llm_provider, llm_model))
    }).clone()
}

/// 获取全局 VisionExplore 工具
pub fn get_global_vision_explore_tool() -> Option<Arc<VisionExploreTool>> {
    GLOBAL_VISION_EXPLORE_TOOL.get().cloned()
}

/// 设置全局 VisionExplore 工具的运行时配置
pub async fn set_global_vision_explore_runtime_config(config: VisionExploreRuntimeConfig) {
    if let Some(tool) = get_global_vision_explore_tool() {
        tool.set_runtime_config(config).await;
    }
}

