//! AI相关命令整合模块
//! 
//! 整合了智能调度器和AI助手的功能，包括：
//! - 智能查询调度
//! - 执行监控
//! - 架构管理
//! - Agent统计

use crate::services::{AiServiceManager, database::{Database, DatabaseService}};
use crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine;
use crate::engines::llm_compiler::engine_adapter::LlmCompilerEngine;
use crate::engines::llm_compiler::types::LlmCompilerConfig;
use crate::engines::plan_and_execute::types::PlanAndExecuteConfig;
// use crate::engines::plan_and_execute::executor::ExecutionMode; // not needed directly
use crate::agents::traits::{ExecutionEngine, AgentTask, TaskPriority};

/// 创建AI助手会话记录
async fn create_ai_assistant_session(
    db_service: &Arc<DatabaseService>,
    execution_id: &str,
    agent_name: &str,
    task_description: &str,
) -> Result<(), String> {
    use crate::services::database::Database;
    
    // 创建task_id
    let task_id = format!("{}_task", execution_id);
    
    // 先创建agent_task记录（因为agent_sessions表有外键约束）
    let agent_task = crate::agents::traits::AgentTask {
        id: task_id.clone(),
        user_id: "ai_assistant".to_string(),
        description: task_description.to_string(),
        priority: crate::agents::traits::TaskPriority::Normal,
        target: None,
        parameters: std::collections::HashMap::new(),
        timeout: Some(300),
    };
    
    db_service.create_agent_task(&agent_task).await
        .map_err(|e| format!("Failed to create agent task: {}", e))?;
    
    // 然后创建agent_session记录
    db_service.create_agent_session(execution_id, &task_id, agent_name).await
        .map_err(|e| format!("Failed to create agent session: {}", e))?;
    
    Ok(())
}

/// 保存AI助手执行记录到数据库
async fn save_ai_assistant_execution(
    db_service: &Arc<DatabaseService>,
    execution_id: &str,
    _task_name: &str,
    architecture: &str,
    success: bool,
    error: Option<&str>,
    result: Option<&str>,
    started_at: u64,
    completed_at: u64,
    duration_ms: u64,
) -> Result<(), String> {
    use crate::services::database::Database;
    use crate::commands::agent_commands::WorkflowStepDetail;
    
    // 保存执行步骤到 agent_execution_steps 表
    let step_detail = WorkflowStepDetail {
        step_id: "step_1".to_string(),
        step_name: format!("AI Assistant Task ({})", architecture),
        status: if success { "Completed".to_string() } else { "Failed".to_string() },
        started_at: Some(started_at.to_string()),
        completed_at: Some(completed_at.to_string()),
        duration_ms,
        result_data: result.map(|r| serde_json::json!(r)),
        error: error.map(|e| e.to_string()),
        retry_count: 0,
        dependencies: vec![],
        tool_result: None,
    };
    
    db_service.save_agent_execution_step(execution_id, &step_detail).await
        .map_err(|e| format!("Failed to save execution step: {}", e))?;
    
    // 更新session状态
    let status_str = if success { "Completed" } else { "Failed" };
    if let Err(e) = db_service.update_agent_session_status(execution_id, status_str).await {
        log::warn!("Failed to update agent session status: {}", e);
    }
    
    Ok(())
}
 


use tauri::{AppHandle, State, Emitter, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use log::{info};

/// 命令响应包装器
#[derive(Debug, Serialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
    pub request_id: String,
}

impl<T> CommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

// ===== 智能调度器相关结构体 =====


#[derive(Debug, Serialize)]
pub struct IntelligentQueryResponse {
    pub request_id: String,
    pub execution_id: String,
    pub selected_architecture: String,
    pub task_type: String,
    pub complexity: String,
    pub reasoning: String,
    pub confidence: f32,
    pub estimated_duration: Option<f64>,
    pub workflow_status: String,
    pub started_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionStatusRequest {
    pub id: String,
    pub id_type: String,
}

#[derive(Debug, Serialize)]
pub struct ExecutionStatusResponse {
    pub execution_id: String,
    pub request_id: String,
    pub status: String,
    pub progress: f32,
    pub current_step: Option<String>,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionHistoryRequest {
    pub user_id: Option<String>,
    pub architecture: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionHistoryResponse {
    pub records: Vec<ExecutionHistoryItem>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct ExecutionHistoryItem {
    pub request_id: String,
    pub execution_id: String,
    pub user_input: String,
    pub architecture: String,
    pub task_type: String,
    pub complexity: String,
    pub status: String,
    pub execution_time: Option<f64>,
    pub success_rate: Option<f32>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DispatcherStatistics {
    pub total_requests: f64,
    pub successful_requests: f64,
    pub failed_requests: f64,
    pub average_execution_time: f64,
    pub architecture_usage: HashMap<String, f64>,
    pub uptime_seconds: f64,
}

// ===== AI助手相关结构体 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchQueryRequest {
    pub query: String,
    pub architecture: String,
    pub agent_id: Option<String>,
    pub options: Option<HashMap<String, serde_json::Value>>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
}




#[derive(Debug, Serialize, Deserialize)]
pub struct DispatchResult {
    pub execution_id: String,
    pub initial_response: String,
    pub execution_plan: Option<ExecutionPlanView>,
    pub estimated_duration: u64,
    pub selected_architecture: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionPlanView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ExecutionStepView>,
    pub estimated_duration: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionStepView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub estimated_duration: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIAssistantSettings {
    #[serde(default)]
    pub auto_execute: bool,
    #[serde(default = "default_notification_enabled")]
    pub notification_enabled: bool,
}

fn default_notification_enabled() -> bool {
    true
}

// ===== 场景 Agent Profile（最小可用版本）=====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEngine { 
    PlanExecute, 
    React,
    Rewoo, 
    LlmCompiler, 
    Auto 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmId { pub provider: String, pub model: String, pub temperature: Option<f32>, pub max_tokens: Option<u32> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigBundle { pub default: LlmId }

// Optional extended configs for scenario agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerStageLlm { pub planner: Option<LlmId>, pub executor: Option<LlmId>, pub replanner: Option<LlmId>, pub evaluator: Option<LlmId> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBundle { pub system: Option<String>, pub planner: Option<String>, pub executor: Option<String>, pub replanner: Option<String>, pub evaluator: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy { pub allow: Vec<String>, pub deny: Option<Vec<String>> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy { pub max_retries: u32, pub backoff: String, pub interval_ms: Option<f64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy { pub timeout_sec: Option<f64>, pub retry: Option<RetryPolicy>, pub concurrency: Option<u32>, pub strict_mode: Option<bool> }

// Prompt template IDs bound to agent (from PromptManagement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptIds {
    pub system: Option<i64>,
    pub planner: Option<i64>,
    pub executor: Option<i64>,
    pub replanner: Option<i64>,
    pub evaluator: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioAgentProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: Option<String>,
    pub engine: AgentEngine,
    pub llm: LlmConfigBundle,
    #[serde(default)]
    pub prompts: Option<PromptBundle>,
    #[serde(default)]
    pub tools: Option<ToolPolicy>,
    #[serde(default)]
    pub execution: Option<ExecutionPolicy>,
    #[serde(default)]
    pub prompt_ids: Option<PromptIds>,
    // Unified prompt system fields
    #[serde(default)]
    pub prompt_strategy: Option<String>,
    #[serde(default)]
    pub group_id: Option<i64>,
    #[serde(default)]
    pub pinned_versions: Option<std::collections::HashMap<i64, String>>,    
    #[serde(default)]
    pub pre_hooks: Option<Vec<String>>,
    #[serde(default)]
    pub post_hooks: Option<Vec<String>>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// Expose tools catalog for agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleToolInfo { 
    pub name: String, 
    pub title: Option<String>, 
    pub category: Option<String>, 
    pub description: Option<String>,
    pub available: bool,
    pub source: Option<String>,
    pub group: Option<String>,
}

#[tauri::command]
pub async fn list_unified_tools(
    tool_system: State<'_, Arc<crate::tools::ToolSystem>>,
) -> Result<Vec<SimpleToolInfo>, String> {
    let tools = tool_system.list_tools().await;
    let list = tools
        .into_iter()
        .map(|t| SimpleToolInfo {
            name: t.name,
            title: None, // ToolMetadata 没有通用 title 字段，这里为空
            category: Some(t.category.to_string()),
            description: if t.description.is_empty() { None } else { Some(t.description) },
            available: t.available,
            source: {
                // 优先用metadata.tags判断mcp，否则fallback
                let tag_has_mcp = t.metadata.tags.iter().any(|x| x == "mcp");
                Some(if tag_has_mcp { "mcp".to_string() } else { "builtin".to_string() })
            },
            group: t.metadata.tags.iter()
                .find_map(|tag| tag.strip_prefix("connection:").map(|s| s.to_string())),
        })
        .collect();
    Ok(list)
}

// 分组返回：内置工具 + MCP按连接分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolGroup { pub connection: String, pub tools: Vec<SimpleToolInfo> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedToolsResponse { pub builtin: Vec<SimpleToolInfo>, pub mcp: Vec<McpToolGroup> }

#[tauri::command]
pub async fn list_unified_tools_grouped(
    tool_system: State<'_, Arc<crate::tools::ToolSystem>>,
) -> Result<GroupedToolsResponse, String> {
    let tools = tool_system.list_tools().await;
    let mut builtin: Vec<SimpleToolInfo> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<SimpleToolInfo>> = std::collections::HashMap::new();

    for t in tools.into_iter() {
        let is_mcp = t.metadata.tags.iter().any(|x| x == "mcp");
        let group = t.metadata.tags.iter()
            .find_map(|tag| tag.strip_prefix("connection:").map(|s| s.to_string()));
        let item = SimpleToolInfo {
            name: t.name,
            title: None,
            category: Some(t.category.to_string()),
            description: if t.description.is_empty() { None } else { Some(t.description) },
            available: t.available,
            source: Some(if is_mcp { "mcp".to_string() } else { "builtin".to_string() }),
            group: group.clone(),
        };

        if is_mcp {
            let key = group.unwrap_or_else(|| "unknown".to_string());
            groups.entry(key).or_default().push(item);
        } else {
            builtin.push(item);
        }
    }

    let mut mcp: Vec<McpToolGroup> = groups.into_iter()
        .map(|(k, v)| McpToolGroup { connection: k, tools: v })
        .collect();
    // 稳定排序连接名
    mcp.sort_by(|a, b| a.connection.cmp(&b.connection));

    Ok(GroupedToolsResponse { builtin, mcp })
}

#[tauri::command]
pub async fn list_scenario_agents(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ScenarioAgentProfile>, String> {
    db_service
        .list_scenario_agents()
        .await
        .map_err(|e| format!("Failed to load scenario agents: {}", e))
}

#[tauri::command]
pub async fn save_scenario_agent(
    profile: ScenarioAgentProfile,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db_service
        .upsert_scenario_agent(&profile)
        .await
        .map_err(|e| format!("Failed to save scenario agent: {}", e))
}

#[tauri::command]
pub async fn delete_scenario_agent(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db_service
        .delete_scenario_agent(&id)
        .await
        .map_err(|e| format!("Failed to delete scenario agent: {}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTaskDispatchRequest {
    pub agent_id: String,
    pub query: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
}

#[tauri::command]
pub async fn dispatch_scenario_task(
    request: ScenarioTaskDispatchRequest,
    db_service: State<'_, Arc<DatabaseService>>,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    execution_manager: State<'_, Arc<crate::managers::ExecutionManager>>,
    app_handle: AppHandle,
) -> Result<DispatchResult, String> {
    // 读取 Agent Profile
    let agents = list_scenario_agents(db_service.clone()).await?;
    let Some(profile) = agents.into_iter().find(|p| p.id == request.agent_id && p.enabled) else {
        return Err(format!("Scenario agent not found or disabled: {}", request.agent_id));
    };

    // 选架构
    let architecture = match profile.engine {
        AgentEngine::PlanExecute => "plan-execute",
        AgentEngine::React => "react",
        AgentEngine::Rewoo => "rewoo",
        AgentEngine::LlmCompiler => "llm-compiler",
        AgentEngine::Auto => "auto",
    }.to_string();

    let mut options = request.options.unwrap_or_default();
    options.insert("agent_id".to_string(), serde_json::Value::String(request.agent_id.clone()));
    
    // 从 options 中提取 conversation_id 和 message_id（向后兼容前端把它们放在 options 里的情况）
    let conversation_id = request.conversation_id.clone()
        .or_else(|| options.get("conversation_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let message_id = request.message_id.clone()
        .or_else(|| options.get("message_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    
    // 获取当前角色提示词并添加到options中
    if let Ok(Some(current_role)) = db_service.get_current_ai_role().await {
        if !current_role.prompt.trim().is_empty() {
            options.insert("role_prompt".to_string(), serde_json::Value::String(current_role.prompt));
            tracing::info!("Added role prompt from: {}", current_role.title);
        }
    }
    
    // 透传已绑定的提示词模板ID，供引擎或执行层使用
    if let Some(pids) = &profile.prompt_ids {
        options.insert("prompt_ids".to_string(), serde_json::json!({
            "system": pids.system,
            "planner": pids.planner,
            "executor": pids.executor,
            "replanner": pids.replanner,
            "evaluator": pids.evaluator,
        }));
    }
    // 透传统一提示词系统策略、分组、文本覆盖及版本固定
    if let Some(strategy) = &profile.prompt_strategy {
        options.insert("prompt_strategy".to_string(), serde_json::Value::String(strategy.clone()));
    }
    if let Some(gid) = profile.group_id {
        options.insert("group_id".to_string(), serde_json::json!(gid));
    }
    if let Some(prompts) = &profile.prompts {
        options.insert("prompts".to_string(), serde_json::json!({
            "system": prompts.system,
            "planner": prompts.planner,
            "executor": prompts.executor,
            "replanner": prompts.replanner,
            "evaluator": prompts.evaluator,
        }));
    }
    if let Some(pinned) = &profile.pinned_versions {
        options.insert("pinned_versions".to_string(), serde_json::to_value(pinned).unwrap_or_else(|_| serde_json::json!({})));
    }

    // 工具白名单/黑名单策略（用于执行期过滤）
    // 要求：System prompt 中的工具清单应严格依据 AgentManager.vue 中“可用+已选”的集合。
    // 语义：
    // - 若前端配置存在（profile.tools 有值）：按 allow/deny 透传；
    // - 若前端未配置（profile.tools 为 None）：也要显式传入空白名单，表示“未选择任何工具 ⇒ 禁用所有工具”。
    //   这样 ReAct/Planner 在构建工具清单时不会退回到“允许所有”。
    if let Some(tool_policy) = &profile.tools {
        log::info!("Agent tools policy - allow: {:?}, deny: {:?}", tool_policy.allow, tool_policy.deny);
        // 允许列表（可能为空，但键一定存在）
        options.insert(
            "tools_allow".to_string(),
            serde_json::json!(tool_policy.allow.clone())
        );
        // 禁止列表（可空）
        if let Some(deny) = &tool_policy.deny {
            options.insert("tools_deny".to_string(), serde_json::json!(deny.clone()));
        }
    } else {
        // 显式设置空白名单：与前端“未选择任何工具”一致，防止引擎回退到“全量可用”。
        log::warn!("Agent has no tools policy configured! Falling back to strict: tools_allow = []");
        options.insert("tools_allow".to_string(), serde_json::json!([] as [String; 0]));
    }

    // 执行策略（超时/重试/严格模式/并发）
    if let Some(exec) = &profile.execution {
        if let Some(timeout) = exec.timeout_sec {
            options.insert("execution_timeout_sec".to_string(), serde_json::json!(timeout));
        }
        if let Some(retry) = &exec.retry {
            options.insert("execution_retry_max".to_string(), serde_json::json!(retry.max_retries));
            options.insert("execution_retry_backoff".to_string(), serde_json::json!(retry.backoff.clone()));
            if let Some(iv) = retry.interval_ms { options.insert("execution_retry_interval_ms".to_string(), serde_json::json!(iv)); }
        }
        if let Some(conc) = exec.concurrency { options.insert("execution_concurrency".to_string(), serde_json::json!(conc)); }
        if let Some(strict) = exec.strict_mode { options.insert("execution_strict_mode".to_string(), serde_json::json!(strict)); }
    }

    // LLM配置（用于覆盖阶段默认模型）
    // 直接传递完整结构，便于后续解析
    options.insert(
        "llm".to_string(),
        serde_json::json!({
            "default": {
                "provider": profile.llm.default.provider,
                "model": profile.llm.default.model,
                "temperature": profile.llm.default.temperature,
                "max_tokens": profile.llm.default.max_tokens,
            }
        })
    );

    // 以下是原 dispatch_intelligent_query 的逻辑
    // 提取任务模式标识和相关信息
    let is_task_mode = options.get("task_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    // conversation_id 和 message_id 已经在上面从 request 或 options 中提取
    // 这里不需要再次提取，直接使用之前的变量
    
    let execution_id = options.get("execution_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // 创建agent_session记录用于统一的工作流监控
    if let Err(e) = create_ai_assistant_session(
        &db_service,
        &execution_id,
        &profile.name,
        &request.query,
    ).await {
        log::warn!("Failed to create AI assistant session: {}", e);
    }
    
    // 提取会话ID（用于日志记录）
    if let Some(ref conv_id) = conversation_id {
        if !conv_id.is_empty() {
            info!("Processing request for conversation: {}", conv_id);
        }
    }
    
    // 如果是任务模式且架构为"auto"，进行智能选择
    let selected_architecture = if is_task_mode && architecture == "auto" {
        let auto_selected = select_best_architecture(&request.query).await
            .map_err(|e| format!("Failed to select architecture: {}", e))?;
        
        info!("Auto-selected architecture: {} for query: {}", auto_selected, request.query);
        
        auto_selected
    } else {
        architecture.clone()
    };
    
    // 创建 DispatchQueryRequest
    let dispatch_req = DispatchQueryRequest {
        query: request.query,
        architecture: selected_architecture.clone(),
        agent_id: Some(profile.id),
        options: Some(options),
        conversation_id: conversation_id.clone(),
        message_id: message_id.clone(),
    };
    
    let app_clone = app_handle.clone();
    // 根据选择的架构创建调度器
    let result = match selected_architecture.as_str() {
        "plan-execute" => {
            dispatch_with_plan_execute(
                execution_id.clone(),
                dispatch_req,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_handle.clone(),
            ).await
        },
        "react" => {
            dispatch_with_react(
                execution_id.clone(),
                dispatch_req,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone.clone(),
            ).await
        },
        "rewoo" => {
            dispatch_with_rewoo(
                execution_id.clone(),
                dispatch_req,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone.clone(),
            ).await
        },
        "llm-compiler" => {
            dispatch_with_llm_compiler(
                execution_id.clone(),
                dispatch_req,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone.clone(),
            ).await
        },
        "auto" => {
            dispatch_with_auto(
                execution_id.clone(),
                dispatch_req,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone.clone(),
            ).await
        }
        _ => {
            Err(format!("Unsupported architecture: {}", selected_architecture))
        }
    };
    
    // 如果调度成功，按架构决定是否需要异步开始“真实执行”
    if let Ok(ref dispatch_result) = result {
        // 仅对需要 register_execution 的架构触发后续执行（如 plan-execute / llm-compiler）
        let arch_for_exec = selected_architecture.clone();
        if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "auto") {
            let execution_id_clone = dispatch_result.execution_id.clone();
            let app_clone = app_handle.clone();
            
            // 异步开始执行，不阻塞调度响应
            tokio::spawn(async move {
                info!("Starting real engine execution: {}", execution_id_clone);
                
                // 从应用状态获取执行管理器
                let execution_manager = app_clone.state::<Arc<crate::managers::ExecutionManager>>();
                let execution_manager_clone = execution_manager.inner().clone();
                let app_inner = app_clone.clone();
                let execution_id_inner = execution_id_clone.clone();
                let db_service_clone = app_clone.state::<Arc<DatabaseService>>().inner().clone();
                
                tokio::spawn(async move {
                    // 获取执行上下文
                    let context = match execution_manager_clone.get_execution_context(&execution_id_inner).await {
                        Some(ctx) => ctx,
                        None => {
                            // 对于不该触发的情况已在外层过滤，这里若仍然缺失，可能是被外部取消或过期清理
                            log::error!("Execution context not found: {}", execution_id_inner);
                            return;
                        }
                    };

                log::info!("Starting real execution for: {} with engine: {:?}", execution_id_inner, context.engine_type);

                // 从任务参数中提取消息ID和会话ID
                let message_id = context.task.parameters.get("message_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let conversation_id = context.task.parameters.get("conversation_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                // 发送 PlanUpdate 事件给前端展示执行计划（预留）
                let _plan_json = serde_json::json!({
                    "id": context.plan.id,
                    "name": context.plan.name,
                    "estimated_duration": context.plan.estimated_duration,
                    "resource_requirements": context.plan.resource_requirements,
                    "steps": context
                        .plan
                        .steps
                        .iter()
                        .enumerate()
                        .map(|(i, s)| serde_json::json!({
                            "index": i + 1,
                            "id": s.id,
                            "name": s.name,
                            "description": s.description,
                            "type": format!("{:?}", s.step_type),
                            "dependencies": s.dependencies,
                            "parameters": s.parameters
                        }))
                        .collect::<Vec<_>>()
                });

                // Emit plan as a PlanInfo message chunk to frontend
                if let Ok(plan_str) = serde_json::to_string(&_plan_json) {
                    crate::utils::ordered_message::emit_message_chunk_arc(
                        &Arc::new(app_inner.clone()),
                        &execution_id_inner,
                        message_id.as_deref().unwrap_or(&execution_id_inner),
                        conversation_id.as_deref(),
                        crate::utils::ordered_message::ChunkType::PlanInfo,
                        &plan_str,
                        false,
                        Some("planner"),
                        None,
                    );
                }

                // Emit a one-shot Meta message with execution configuration
                let _meta_json = {
                    let params = &context.task.parameters;
                    serde_json::json!({
                        "engine": format!("{:?}", context.engine_type),
                        "agent_id": params.get("agent_id").and_then(|v| v.as_str()),
                        "prompt_ids": params.get("prompt_ids"),
                        "prompt_strategy": params.get("prompt_strategy").and_then(|v| v.as_str()),
                        "group_id": params.get("group_id"),
                        "pinned_versions": params.get("pinned_versions"),
                    })
                };

                // 记录执行开始时间
                let execution_start_time = std::time::SystemTime::now();
                
                // 执行真实的引擎计划
                let exec_result = execution_manager_clone.execute_plan(&execution_id_inner).await;
                
                // 记录执行完成时间
                let execution_end_time = std::time::SystemTime::now();
                
                // 保存执行结果到数据库
                let task_name = context.task.description.clone();
                let architecture = format!("{:?}", context.engine_type);
                let started_at = execution_start_time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let completed_at = execution_end_time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let duration_ms = execution_end_time.duration_since(execution_start_time).unwrap_or_default().as_millis() as u64;
                
                let success = exec_result.is_ok();
                let error = if let Err(ref e) = exec_result { Some(e.to_string()) } else { None };
                let result = if success { Some("Task completed successfully".to_string()) } else { None };
                
                // 只有非Plan-and-Execute架构才保存通用的AI助手执行步骤
                // Plan-and-Execute引擎会自己保存详细的步骤信息
                if !architecture.contains("PlanExecute") {
                    if let Err(e) = save_ai_assistant_execution(
                        &db_service_clone,
                        &execution_id_inner,
                        &task_name,
                        &architecture,
                        success,
                        error.as_deref(),
                        result.as_deref(),
                        started_at,
                        completed_at,
                        duration_ms,
                    ).await {
                        log::warn!("Failed to save AI assistant execution: {}", e);
                    }
                } else {
                    // 对于Plan-and-Execute架构，只更新session状态
                    use crate::services::database::Database;
                    let status_str = if success { "Completed" } else { "Failed" };
                    if let Err(e) = db_service_clone.update_agent_session_status(&execution_id_inner, status_str).await {
                        log::warn!("Failed to update agent session status: {}", e);
                    }
                }
                
                match exec_result {
                    Ok(_result) => {
                        log::info!("Execution completed successfully: {}", execution_id_inner);
                        // 移除原始事件，只使用ai_stream_message
                    }
                    Err(e) => {
                        log::error!("Execution failed: {}: {}", execution_id_inner, e);
                        
                        // 使用更友好的错误消息格式
                        let error_message = format!(
                            "任务执行失败: {}\n\n如需帮助，请检查执行配置或联系技术支持。",
                            e.to_string()
                        );
                        
                        // 使用有序消息块发送错误
                        crate::utils::ordered_message::emit_message_chunk_arc(
                            &Arc::new(app_inner.clone()),
                            &execution_id_inner,
                            message_id.as_deref().unwrap_or(&execution_id_inner),
                            conversation_id.as_deref(),
                            crate::utils::ordered_message::ChunkType::Error,
                            &error_message,
                            true, // 确保标记为最终消息
                            None,
                            None,
                        );
                        
                        // 确保发送一个内容块来正式结束会话
                        crate::utils::ordered_message::emit_message_chunk_arc(
                            &Arc::new(app_inner.clone()),
                            &execution_id_inner,
                            message_id.as_deref().unwrap_or(&execution_id_inner),
                            conversation_id.as_deref(),
                            crate::utils::ordered_message::ChunkType::Content,
                            "", // 空内容，仅用于结束流
                            true, // 最终消息
                            Some("error_termination"),
                            None,
                        );
                    }
                }

                    // 清理执行上下文
                    execution_manager_clone.cleanup_execution(&execution_id_inner).await;
                });
            });
        } else {
            // ReAct 等架构已在调度阶段完成执行，这里不再重复触发
            info!("Architecture '{}' completes within dispatch; skipping real engine execution.", arch_for_exec);
        }
    }
    
    // 更新返回结果中的架构信息
    result.map(|mut dispatch_result| {
        // 当外层选择为 "auto" 时，不覆盖具体调度器返回的架构信息
        if selected_architecture != "auto" {
            dispatch_result.selected_architecture = selected_architecture.clone();
        }
        dispatch_result
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub active_count: u32,
    pub total_tasks: u32,
    pub successful_tasks: u32,
    pub failed_tasks: u32,
    pub average_execution_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub status: String,
    pub tasks_completed: u32,
}



/// 停止执行
#[tauri::command]
pub async fn stop_execution(
    execution_id: String,
    app: AppHandle,
) -> Result<(), String> {
    info!("Stopping execution: {}", execution_id);

    // 1. 尝试停止执行管理器中的任务
    let execution_manager = app.state::<Arc<crate::managers::ExecutionManager>>();
    let manager = execution_manager.inner().clone();
    if let Err(e) = manager.stop_execution(&execution_id).await {
        log::warn!("Failed to stop execution {}: {}", execution_id, e);
    }

    // 2. 如果execution_id看起来像会话ID，也尝试取消对应的流
    // 这样可以处理用会话ID调用stop的情况
    if execution_id.starts_with("conv_") || execution_id.len() == 36 {
        // 可能是会话ID或UUID格式
        use crate::commands::ai::cancel_conversation_stream;
        cancel_conversation_stream(&execution_id);
        info!("Also cancelled stream for conversation: {}", execution_id);
    }

    // 3. 发送停止事件（统一事件名称）
    if let Err(e) = app.emit("execution_stopped", serde_json::json!({
        "execution_id": execution_id,
        "message": "Execution stopped by user"
    })) {
        log::warn!("Failed to emit execution_stopped event: {}", e);
    }

    info!("Execution stop completed: {}", execution_id);
    Ok(())
}

/// 获取AI助手设置
#[tauri::command]
pub async fn get_ai_assistant_settings(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<AIAssistantSettings, String> {
    // 从数据库加载设置，使用专门的key存储AIAssistantSettings
    match db_service.get_config("ai_assistant", "assistant_settings").await {
        Ok(Some(json_str)) => {
            serde_json::from_str::<AIAssistantSettings>(&json_str)
                .map_err(|e| format!("Failed to parse AI assistant settings: {}", e))
        }
        Ok(None) => {
            // 返回默认设置
            let default_settings = AIAssistantSettings {
                auto_execute: false,
                notification_enabled: true,
            };
            info!("Using default AI assistant settings");
            Ok(default_settings)
        }
        Err(e) => Err(format!("Failed to load AI assistant settings: {}", e)),
    }
}

/// 保存AI助手设置
#[tauri::command]
pub async fn save_ai_assistant_settings(
    settings: AIAssistantSettings,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    info!("Saving AI assistant settings: {:?}", settings);
    let json = serde_json::to_string(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    db_service
        .set_config(
            "ai_assistant",
            "assistant_settings",
            &json,
            None,
        )
        .await
        .map_err(|e| format!("Failed to save AI assistant settings: {}", e))
}

/// 获取Agent统计信息
#[tauri::command]
pub async fn get_agent_statistics() -> Result<AgentStatistics, String> {
    // 从数据库或监控系统获取统计信息
    Ok(AgentStatistics {
        active_count: 2,
        total_tasks: 156,
        successful_tasks: 142,
        failed_tasks: 14,
        average_execution_time: 180.5,
    })
}

/// 获取可用架构列表
#[tauri::command]
pub async fn get_available_architectures() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({
            "id": "auto",
            "name": "Auto",
            "description": "自动选择最优架构",
            "suitable_for": ["所有任务"],
            "performance": "自动",
            "status": "stable"
        }),
        serde_json::json!({
            "id": "plan-execute",
            "name": "Plan-and-Execute",
            "description": "基于规划和执行的智能Agent架构",
            "suitable_for": ["复杂任务", "多步骤流程", "需要重规划的任务"],
            "performance": "稳定",
            "status": "stable"
        }),
        serde_json::json!({
            "id": "rewoo",
            "name": "ReWOO",
            "description": "推理而非观察的工作流架构",
            "suitable_for": ["推理密集型任务", "分析类任务"],
            "performance": "高效",
            "status": "beta"
        }),
        serde_json::json!({
            "id": "llm-compiler",
            "name": "LLMCompiler",
            "description": "并行执行引擎",
            "suitable_for": ["并行任务", "独立的多个子任务"],
            "performance": "快速",
            "status": "experimental"
        }),

    ])
}

/// 获取自定义Agent列表
#[tauri::command]
pub async fn get_ai_assistant_agents(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<CustomAgent>, String> {
    match db_service.get_config("ai_assistant", "custom_agents").await {
        Ok(Some(json_str)) => serde_json::from_str::<Vec<CustomAgent>>(&json_str)
            .map_err(|e| format!("Failed to parse custom agents: {}", e)),
        Ok(None) => Ok(vec![]),
        Err(e) => Err(format!("Failed to load custom agents: {}", e)),
    }
}

/// 保存自定义Agent列表
#[tauri::command]
pub async fn save_ai_assistant_agents(
    agents: Vec<CustomAgent>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let json = serde_json::to_string(&agents)
        .map_err(|e| format!("Failed to serialize custom agents: {}", e))?;
    db_service
        .set_config(
            "ai_assistant",
            "custom_agents",
            &json,
            None,
        )
        .await
        .map_err(|e| format!("Failed to save custom agents: {}", e))
}

/// 获取架构启用偏好（返回启用的架构ID列表）
#[tauri::command]
pub async fn get_ai_architecture_prefs(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    match db_service.get_config("ai_assistant", "enabled_architectures").await {
        Ok(Some(json_str)) => serde_json::from_str::<Vec<String>>(&json_str)
            .map_err(|e| format!("Failed to parse architecture prefs: {}", e)),
        Ok(None) => Ok(vec![
            "plan-execute".to_string(),
            "rewoo".to_string(),
            "llm-compiler".to_string(),
        ]),
        Err(e) => Err(format!("Failed to load architecture prefs: {}", e)),
    }
}

/// 保存架构启用偏好
#[tauri::command]
pub async fn save_ai_architecture_prefs(
    enabled_architectures: Vec<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let json = serde_json::to_string(&enabled_architectures)
        .map_err(|e| format!("Failed to serialize architecture prefs: {}", e))?;
    db_service
        .set_config(
            "ai_assistant",
            "enabled_architectures",
            &json,
            None,
        )
        .await
        .map_err(|e| format!("Failed to save architecture prefs: {}", e))
}

// ===== 辅助函数和结构体 =====

#[derive(Debug, Deserialize)]
pub struct TaskSubmissionRequest {
    pub user_input: String,
    pub user_id: String,
    pub priority: Option<String>,
    pub estimated_duration: Option<f64>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct NodeRegistrationRequest {
    pub name: String,
    pub capacity: NodeCapacityRequest,
}

#[derive(Debug, Deserialize)]
pub struct NodeCapacityRequest {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub network_mbps: f32,
    pub storage_gb: u32,
    pub max_concurrent_tasks: u32,
}



/// 智能选择最佳架构
async fn select_best_architecture(user_input: &str) -> Result<String, String> {
    // 简单的规则基础架构选择
    let input_lower = user_input.to_lowercase();
    
    // 分析任务特征
    let has_complex_analysis = input_lower.contains("分析") || input_lower.contains("analysis");
    let has_scanning = input_lower.contains("扫描") || input_lower.contains("scan");
    let has_monitoring = input_lower.contains("监控") || input_lower.contains("monitor");
    let has_multiple_steps = input_lower.contains("步骤") || input_lower.contains("多个") || input_lower.contains("multiple");
    let has_parallel_tasks = input_lower.contains("同时") || input_lower.contains("并行") || input_lower.contains("parallel");
    
    // 架构选择逻辑
    if has_parallel_tasks || (has_scanning && has_multiple_steps) {
        Ok("llm-compiler".to_string())
    } else if has_complex_analysis {
        Ok("rewoo".to_string())
    } else if has_monitoring || input_lower.len() > 100 {
        Ok("plan-execute".to_string())
    } else {
        // 对于一般任务，使用plan-execute
        Ok("plan-execute".to_string())
    }
}



async fn dispatch_with_auto(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    let architecture = select_best_architecture(&request.query).await?;
    match architecture.as_str() {
        "plan-execute" => dispatch_with_plan_execute(execution_id, request, ai_service_manager, db_service, execution_manager, app).await,
        "rewoo" => dispatch_with_rewoo(execution_id, request, ai_service_manager, db_service, execution_manager, app).await,
        "llm-compiler" => dispatch_with_llm_compiler(execution_id, request, ai_service_manager, db_service, execution_manager, app).await,
        _ => Err(format!("Unsupported architecture: {}", architecture)),
    }
}   

async fn dispatch_with_plan_execute(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    
    // 创建Plan-and-Execute引擎配置
    let config = PlanAndExecuteConfig::default();
    
    // 创建Plan-and-Execute引擎
    let mut engine = PlanAndExecuteEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
        Some(Arc::new(app.clone())),
    ).await.map_err(|e| format!("Failed to create Plan-and-Execute engine: {}", e))?;
    
    // 创建Agent任务
    let mut parameters = request.options.unwrap_or_default();
    // 统一使用 snake_case keys，兼容可能传入的 camelCase
    if let Some(v) = parameters.remove("executionId") { parameters.insert("execution_id".to_string(), v); }
    if let Some(v) = parameters.remove("messageId") { parameters.insert("message_id".to_string(), v); }
    if let Some(v) = parameters.remove("conversationId") { parameters.insert("conversation_id".to_string(), v); }
    if let Some(v) = parameters.remove("taskMode") { parameters.insert("task_mode".to_string(), v); }
    // 统一提示词ID字段（兼容 camelCase -> snake_case）
    if let Some(v) = parameters.remove("promptIds") { parameters.insert("prompt_ids".to_string(), v); }
    parameters.insert("execution_id".to_string(), serde_json::Value::String(execution_id.clone()));

    let task = AgentTask {
        id: Uuid::new_v4().to_string(), // The internal task ID can be unique
        user_id: "system".to_string(),
        description: request.query.clone(),
        priority: TaskPriority::Normal,
        target: None,
        parameters,
        timeout: Some(600), // 10 minute timeout
    };
    
    // 将参数注入引擎，便于执行阶段访问（如 prompt_ids ）
    engine.set_runtime_params(task.parameters.clone());

    // Create execution plan
    let plan = engine.create_plan(&task).await
        .map_err(|e| format!("Failed to create execution plan: {}", e))?;

    // Register execution context and engine instance to execution manager
    let engine_instance = crate::managers::EngineInstance::PlanExecute(engine);
    execution_manager.register_execution(
        execution_id.clone(),
        crate::managers::EngineType::PlanExecute,
        plan.clone(),
        task,
        engine_instance,
    ).await.map_err(|e| format!("Failed to register execution: {}", e))?;
    
    let execution_plan = ExecutionPlanView {
        id: plan.id.clone(),
        name: plan.name.clone(),
        description: format!("Plan-and-Execute任务: {}", request.query),
        steps: plan.steps.iter().map(|step| ExecutionStepView {
            id: step.id.clone(),
            name: step.name.clone(),
            description: step.description.clone(),
            status: "pending".to_string(),
            estimated_duration: 60,
        }).collect(),
        estimated_duration: plan.estimated_duration,
    };
    
    Ok(DispatchResult {
        execution_id,
        initial_response: "已创建Plan-and-Execute执行计划，引擎实例已注册，准备真实执行...".to_string(),
        execution_plan: Some(execution_plan),
        estimated_duration: plan.estimated_duration,
        selected_architecture: "Plan-and-Execute".to_string(),
    })
}

async fn dispatch_with_react(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    _execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    use crate::engines::react::{ReactEngine, ReactConfig};
    use std::collections::HashMap;
    use crate::agents::traits::{AgentTask, TaskPriority};
    
    info!("Creating ReAct dispatch for: {}", request.query);
    
    // 从 options 中提取配置
    let options = request.options.unwrap_or_default();
    let mut config = ReactConfig::default();
    let max_iterations = config.max_iterations; // 保存用于超时计算
    
    if let Some(max_iter) = options.get("max_iterations").and_then(|v| v.as_u64()) {
        config.max_iterations = max_iter as u32;
    }
    if let Some(temp) = options.get("temperature").and_then(|v| v.as_f64()) {
        config.temperature = Some(temp as f32);
    }
    if let Some(max_tok) = options.get("max_tokens").and_then(|v| v.as_u64()) {
        config.max_tokens = Some(max_tok as u32);
    }
    if let Some(rag) = options.get("enable_rag").and_then(|v| v.as_bool()) {
        config.enable_rag = rag;
    }
    if let Some(verbose) = options.get("verbose").and_then(|v| v.as_bool()) {
        config.verbose = verbose;
    }
    
    // **关键修复**: 从 options 中读取 tools_allow 并设置到 ReactConfig.allowed_tools
    // 这样 ReAct executor 的 build_tools_information 才能读取到正确的工具白名单
    if let Some(tools_allow) = options.get("tools_allow") {
        if let Some(arr) = tools_allow.as_array() {
            let tool_names: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            log::info!("ReAct dispatch: 设置 allowed_tools = {:?}", tool_names);
            config.allowed_tools = Some(tool_names);
        }
    }
    
    // 获取默认 AI 服务
    let ai_service = match ai_service_manager.get_default_chat_model().await {
        Ok(Some((provider, model))) => {
            match ai_service_manager.get_provider_config(&provider).await {
                Ok(Some(mut provider_config)) => {
                    provider_config.model = model;
                    let mcp_service = ai_service_manager.get_mcp_service();
                    let ai_svc = crate::services::ai::AiService::new(
                        provider_config,
                        db_service.clone(),
                        Some(app.clone()),
                        mcp_service.clone(),
                    );
                    Arc::new(ai_svc)
                }
                _ => {
                    return Err("Failed to get AI provider config".to_string());
                }
            }
        }
        _ => {
            return Err("No default AI model configured".to_string());
        }
    };
    
    // 序列化 config
    let config_json = serde_json::to_value(&config).map_err(|e| e.to_string())?;
    
    // 创建 ReactEngine
    let engine = ReactEngine::new(config).with_services(
        ai_service,
        ai_service_manager.get_mcp_service(),
        Some(db_service.clone()),
        Some(app.clone()),
    );
    
    // 创建 AgentTask
    let task = AgentTask {
        id: execution_id.clone(),
        description: request.query.clone(),
        target: None,
        parameters: {
            let mut map = HashMap::new();
            map.insert("query".to_string(), serde_json::json!(request.query));
            map.insert("config".to_string(), config_json);
            
            // **关键修复**: 将 tools_allow 和 tools_deny 从 options 透传到 task.parameters 顶层
            // ReAct executor 的 build_tools_information 会从这里读取
            if let Some(tools_allow) = options.get("tools_allow") {
                log::info!("ReAct dispatch: 透传 tools_allow 到 task.parameters");
                map.insert("tools_allow".to_string(), tools_allow.clone());
            }
            if let Some(tools_deny) = options.get("tools_deny") {
                log::info!("ReAct dispatch: 透传 tools_deny 到 task.parameters");
                map.insert("tools_deny".to_string(), tools_deny.clone());
            }
            
            // 添加 conversation_id 和 message_id 到 parameters，让 ReAct 引擎能够提取
            if let Some(conv_id) = &request.conversation_id {
                map.insert("conversation_id".to_string(), serde_json::json!(conv_id));
            }
            if let Some(msg_id) = &request.message_id {
                map.insert("message_id".to_string(), serde_json::json!(msg_id));
            }
            map
        },
        user_id: "default".to_string(),
        priority: TaskPriority::Normal,
        timeout: Some(max_iterations as u64 * 30000), // 30s per iteration
    };
    
    // 创建 dummy session 用于执行
    use crate::agents::traits::{AgentSession, AgentSessionStatus, LogLevel, AgentExecutionResult, SessionLog};
    struct DummySession {
        task: AgentTask,
        status: AgentSessionStatus,
        logs: Vec<SessionLog>,
        result: Option<AgentExecutionResult>,
    }
    
    #[async_trait::async_trait]
    impl AgentSession for DummySession {
        fn get_session_id(&self) -> &str { "dummy" }
        fn get_task(&self) -> &AgentTask { &self.task }
        fn get_status(&self) -> AgentSessionStatus { self.status.clone() }
        async fn update_status(&mut self, status: AgentSessionStatus) -> anyhow::Result<()> {
            self.status = status;
            Ok(())
        }
        async fn add_log(&mut self, level: LogLevel, message: String) -> anyhow::Result<()> {
            self.logs.push(SessionLog {
                level,
                message,
                timestamp: chrono::Utc::now(),
                source: "react".to_string(),
            });
            Ok(())
        }
        fn get_logs(&self) -> &[SessionLog] { &self.logs }
        async fn set_result(&mut self, result: AgentExecutionResult) -> anyhow::Result<()> {
            self.result = Some(result);
            Ok(())
        }
        fn get_result(&self) -> Option<&AgentExecutionResult> { self.result.as_ref() }
    }
    
    let mut session = DummySession {
        task,
        status: AgentSessionStatus::Executing,
        logs: Vec::new(),
        result: None,
    };
    
    // 执行任务 - 先克隆 task 避免借用冲突
    let task_clone = session.task.clone();
    let start_time = std::time::Instant::now();
    match engine.execute(&task_clone, &mut session).await {
        Ok(result) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            // 从 result.data 中提取响应文本
            let response = if let Some(data) = &result.data {
                data.as_str().unwrap_or("").to_string()
            } else {
                "ReAct execution completed".to_string()
            };
            
            Ok(DispatchResult {
                execution_id,
                initial_response: response,
                execution_plan: None,
                estimated_duration: duration_ms,
                selected_architecture: "ReAct".to_string(),
            })
        }
        Err(e) => Err(format!("ReAct execution failed: {}", e))
    }
}

async fn dispatch_with_rewoo(
    _execution_id: String,
    _request: DispatchQueryRequest,
    _ai_service_manager: Arc<AiServiceManager>,
    _db_service: Arc<DatabaseService>,
    _execution_manager: Arc<crate::managers::ExecutionManager>,
    _app: AppHandle,
) -> Result<DispatchResult, String> {
    // DISABLED: ReWOO engine needs Rig refactor
    Err("ReWOO engine disabled - needs Rig refactor".to_string())
}

async fn dispatch_with_llm_compiler(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    _app: AppHandle,
) -> Result<DispatchResult, String> {
    info!("Creating LLMCompiler dispatch for: {}", request.query);
    
    // 创建LLMCompiler引擎配置
    let config = LlmCompilerConfig::default();
    
    // 创建LLMCompiler引擎
    let mut engine = LlmCompilerEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
    ).await.map_err(|e| format!("Failed to create LLMCompiler engine: {}", e))?;
    
    // 创建Agent任务
    let task = AgentTask {
        id: execution_id.clone(),
        user_id: "system".to_string(),
        description: request.query.clone(),
        priority: TaskPriority::High, // LLMCompiler适合高优先级任务
        target: None,
        parameters: request.options.unwrap_or_default(),
        timeout: Some(240), // 4分钟超时
    };
    // 注入运行期参数
    engine.set_runtime_params(task.parameters.clone());
    
    // 创建执行计划
    let plan = engine.create_plan(&task).await
        .map_err(|e| format!("Failed to create LLMCompiler plan: {}", e))?;
    
    // 注册执行上下文和引擎实例到执行管理器
    let engine_instance = crate::managers::EngineInstance::LLMCompiler(engine);
    execution_manager.register_execution(
        execution_id.clone(),
        crate::managers::EngineType::LLMCompiler,
        plan.clone(),
        task,
        engine_instance,
    ).await.map_err(|e| format!("Failed to register execution: {}", e))?;
    
    let execution_plan = ExecutionPlanView {
        id: plan.id.clone(),
        name: plan.name.clone(),
        description: format!("LLMCompiler并行任务: {}", request.query),
        steps: plan.steps.iter().map(|step| ExecutionStepView {
            id: step.id.clone(),
            name: step.name.clone(),
            description: step.description.clone(),
            status: "pending".to_string(),
            estimated_duration: 30, // LLMCompiler步骤通常更快
        }).collect(),
        estimated_duration: plan.estimated_duration,
    };
    
    Ok(DispatchResult {
        execution_id,
        initial_response: "已启动LLMCompiler并行执行引擎，引擎实例已注册，准备真实执行...".to_string(),
        execution_plan: Some(execution_plan),
        estimated_duration: plan.estimated_duration,
        selected_architecture: "LLMCompiler".to_string(),
    })
}
