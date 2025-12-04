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
use crate::engines::rewoo::engine_adapter::ReWooEngine;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
// use crate::engines::orchestrator::engine_adapter::OrchestratorEngineAdapter; // Orchestrator已删除
// use crate::engines::plan_and_execute::executor::ExecutionMode; // not needed directly
use crate::agents::traits::{ExecutionEngine, AgentTask, TaskPriority};
use sentinel_core::models::agent::{AgentTask as CoreAgentTask, TaskPriority as CoreTaskPriority, AgentExecutionResult as CoreAgentExecutionResult, SessionLog as CoreSessionLog};
use futures::StreamExt;
use sentinel_core::models::scenario_agent::{ScenarioAgentProfile, AgentEngine};

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

    let db_task = CoreAgentTask {
        id: agent_task.id.clone(),
        description: agent_task.description.clone(),
        target: agent_task.target.clone(),
        parameters: agent_task.parameters.clone(),
        user_id: agent_task.user_id.clone(),
        priority: match agent_task.priority {
            TaskPriority::Low => CoreTaskPriority::Low,
            TaskPriority::Normal => CoreTaskPriority::Normal,
            TaskPriority::High => CoreTaskPriority::High,
            TaskPriority::Critical => CoreTaskPriority::Critical,
        },
        timeout: agent_task.timeout,
    };

    db_service.create_agent_task(&db_task).await
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
use sentinel_core::models::workflow::WorkflowStepDetail;

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
use log::{info, warn};

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
        // 跳过插件工具（名称以 plugin:: 开头或包含 plugin 标签）
        // 插件工具应该通过 list_plugins 接口单独管理
        let is_plugin = t.name.starts_with("plugin::") || t.metadata.tags.iter().any(|x| x == "plugin");
        if is_plugin {
            continue;
        }

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
        AgentEngine::Travel => "travel",
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

    if let Some(conv_id) = conversation_id.as_ref() {
        if !conv_id.is_empty() {
            let user_msg = sentinel_core::models::database::AiMessage {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: request.query.clone(),
                metadata: None,
                token_count: Some(request.query.len() as i32),
                cost: None,
                tool_calls: None,
                attachments: None,
                timestamp: chrono::Utc::now(),
                architecture_type: None,
                architecture_meta: None,
                structured_data: None,
            };
            let _ = db_service.create_message(&user_msg).await;
        }
    }

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
    {
        let prompts = &profile.prompts;
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
    {
        let tool_policy = &profile.tools;
        log::info!("Agent tools policy - allow: {:?}, deny: {:?}", tool_policy.allow, tool_policy.deny);
        options.insert(
            "tools_allow".to_string(),
            serde_json::json!(tool_policy.allow.clone())
        );
        if let Some(deny) = &tool_policy.deny {
            options.insert("tools_deny".to_string(), serde_json::json!(deny.clone()));
        }
    }

    // 执行策略（超时/重试/严格模式/并发）
    {
        let exec = &profile.execution;
        if let Some(timeout) = exec.timeout_sec {
            options.insert("execution_timeout_sec".to_string(), serde_json::json!(timeout));
        }
        let retry = &exec.retry;
        options.insert("execution_retry_max".to_string(), serde_json::json!(retry.max_retries));
        options.insert("execution_retry_backoff".to_string(), serde_json::json!(retry.backoff.clone()));
        if let Some(iv) = retry.interval_ms { options.insert("execution_retry_interval_ms".to_string(), serde_json::json!(iv)); }
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
        "travel" => {
            dispatch_with_travel(
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

    // 如果调度成功，按架构决定是否需要异步开始"真实执行"
    if let Ok(ref dispatch_result) = result {
        // 仅对需要 register_execution 的架构触发后续执行（如 plan-execute / llm-compiler）
        // 注意：travel 和 react 架构在 dispatch 函数中直接执行，不需要异步执行
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
                        None,
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
                            None,
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
            info!("Architecture '{}' completes within dispatch; ", arch_for_exec);
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
#[tauri::command(rename_all = "snake_case")]
pub async fn stop_execution(
    execution_id: String,
    app: AppHandle,
) -> Result<(), String> {
    info!("🛑 Stopping execution: {}", execution_id);

    // 1. ✅ 取消CancellationToken（对ReAct架构有效）
    use crate::managers::cancellation_manager;
    let cancelled_by_token = cancellation_manager::cancel_execution(&execution_id).await;
    if cancelled_by_token {
        log::info!("✅ Cancelled execution via CancellationToken: {}", execution_id);
    }

    // 2. 尝试停止执行管理器中的任务（对Plan-Execute/LLMCompiler有效）
    let execution_manager = app.state::<Arc<crate::managers::ExecutionManager>>();
    let manager = execution_manager.inner().clone();
    if let Err(e) = manager.stop_execution(&execution_id).await {
        log::warn!("Failed to stop execution via ExecutionManager {}: {}", execution_id, e);
    } else {
        log::info!("✅ Stopped execution via ExecutionManager: {}", execution_id);
    }

    // 3. 如果execution_id看起来像会话ID，也尝试取消对应的流
    // 这样可以处理用会话ID调用stop的情况
    if execution_id.starts_with("conv_") || execution_id.len() == 36 {
        // 可能是会话ID或UUID格式
        use crate::commands::ai::cancel_conversation_stream;
        cancel_conversation_stream(&execution_id);
        log::info!("✅ Cancelled stream for conversation: {}", execution_id);
    }

    // 4. 发送停止事件（统一事件名称）
    if let Err(e) = app.emit("execution_stopped", serde_json::json!({
        "execution_id": execution_id,
        "message": "Execution stopped by user"
    })) {
        log::warn!("Failed to emit execution_stopped event: {}", e);
    }

    log::info!("✅ Stop execution completed: {}", execution_id);

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
pub async fn get_agent_statistics(
    manager: State<'_, crate::commands::agent_commands::GlobalAgentManager>,
) -> Result<AgentStatistics, String> {
    let manager_guard = manager.read().await;

    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => {
            // Agent管理器未初始化，返回默认值
            return Ok(AgentStatistics {
                active_count: 0,
                total_tasks: 0,
                successful_tasks: 0,
                failed_tasks: 0,
                average_execution_time: 0.0,
            });
        }
    };

    // 从Agent管理器获取真实统计数据
    let stats = agent_manager.get_statistics().await;
    let sessions = agent_manager.get_all_sessions().await;

    // 统计活跃会话数
    let active_count = sessions.iter().filter(|(_, info)| {
        matches!(
            info.status,
            crate::agents::traits::AgentSessionStatus::Planning |
            crate::agents::traits::AgentSessionStatus::Executing
        )
    }).count();

    Ok(AgentStatistics {
        active_count: active_count as u32,
        total_tasks: stats.total_tasks as u32,
        successful_tasks: stats.successful_tasks as u32,
        failed_tasks: stats.failed_tasks as u32,
        average_execution_time: stats.average_execution_time_ms / 1000.0, // 转换为秒
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
        "travel" => Err("Travel architecture dispatch not yet implemented".to_string()),
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
    use crate::managers::cancellation_manager;

    info!("Creating ReAct dispatch for: {}", request.query);

    // ✅ 注册取消令牌
    let cancellation_token = cancellation_manager::register_cancellation_token(execution_id.clone()).await;

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
                    // Provider配置获取失败，尝试使用 "default" 服务
                    log::warn!("Failed to get provider config for '{}', falling back to 'default' service", provider);
                    match ai_service_manager.get_service("default") {
                        Some(service) => Arc::new(service),
                        None => {
                            return Err(format!("Failed to get AI provider config for '{}' and no default service available", provider));
                        }
                    }
                }
            }
        }
        _ => {
            // 没有配置默认模型，尝试使用 "default" 服务
            log::warn!("No default chat model configured, trying to use 'default' service");
            match ai_service_manager.get_service("default") {
                Some(service) => Arc::new(service),
                None => {
                    return Err("No default AI model configured and no default service available".to_string());
                }
            }
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
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    _execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    log::info!("Creating ReWOO dispatch for: {}", request.query);

    // 创建ReWOO引擎配置
    let config = ReWOOConfig::default();

    // 创建ReWOO引擎
    let mut engine = ReWooEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
    ).await.map_err(|e| format!("Failed to create ReWOO engine: {}", e))?;

    // 设置app_handle用于推送执行结果到前端
    engine.set_app_handle(app.clone());

    // 创建Agent任务
    let mut task_params = request.options.unwrap_or_default();

    // 添加 conversation_id 和 message_id 到 parameters
    if let Some(conv_id) = &request.conversation_id {
        task_params.insert("conversation_id".to_string(), serde_json::json!(conv_id));
    }
    if let Some(msg_id) = &request.message_id {
        task_params.insert("message_id".to_string(), serde_json::json!(msg_id));
    }
    task_params.insert("execution_id".to_string(), serde_json::json!(execution_id));

    let task = AgentTask {
        id: execution_id.clone(),
        user_id: "system".to_string(),
        description: request.query.clone(),
        priority: TaskPriority::Normal,
        target: None,
        parameters: task_params.clone(),
        timeout: Some(300), // 5分钟超时
    };

    // 设置运行时参数
    engine.set_runtime_params(task_params);

    // 执行ReWOO流程
    let start_time = std::time::Instant::now();
    match engine.execute(&task).await {
        Ok(result) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;

            // 从 result.data 中提取响应文本
            let response = if let Some(data) = &result.data {
                if let Some(result_str) = data.get("result").and_then(|v| v.as_str()) {
                    result_str.to_string()
                } else {
                    data.to_string()
                }
            } else {
                "ReWOO execution completed".to_string()
            };

            Ok(DispatchResult {
                execution_id,
                initial_response: response,
                execution_plan: None,
                estimated_duration: duration_ms,
                selected_architecture: "ReWOO".to_string(),
            })
        }
        Err(e) => Err(format!("ReWOO execution failed: {}", e))
    }
}

async fn dispatch_with_llm_compiler(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    // info!("Creating LLMCompiler dispatch for: {}", request.query);

    // 创建LLMCompiler引擎配置
    let config = LlmCompilerConfig::default();

    // 创建LLMCompiler引擎
    let mut engine = LlmCompilerEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
    ).await.map_err(|e| format!("Failed to create LLMCompiler engine: {}", e))?;

    // ✅ 设置app_handle用于推送工具执行结果到前端
    engine.set_app_handle(app.clone());

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

    // ✅ 注入运行期参数，包括用户的任务描述
    let mut runtime_params = task.parameters.clone();
    runtime_params.insert(
        "task_description".to_string(),
        serde_json::Value::String(task.description.clone())
    );
    engine.set_runtime_params(runtime_params);

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


/// Travel架构调度
async fn dispatch_with_travel(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    _execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    use crate::engines::travel::{TravelEngine, TravelConfig};
    use std::collections::HashMap;
    use crate::agents::traits::{AgentTask, TaskPriority};
    use crate::managers::cancellation_manager;

    info!("Creating Travel dispatch for: {}", request.query);

    // 注册取消令牌
    let _cancellation_token = cancellation_manager::register_cancellation_token(execution_id.clone()).await;

    // 从 options 中提取配置
    let options = request.options.unwrap_or_default();
    let mut config = TravelConfig::default();

    // 提取Travel特定配置
    if let Some(max_cycles) = options.get("max_ooda_cycles").and_then(|v| v.as_u64()) {
        config.max_ooda_cycles = max_cycles as u32;
    }
    if let Some(strict_mode) = options.get("guardrail_strict_mode").and_then(|v| v.as_bool()) {
        config.guardrail_config.strict_mode = strict_mode;
    }
    if let Some(enable_rag) = options.get("enable_threat_intel_rag").and_then(|v| v.as_bool()) {
        config.threat_intel_config.enable_rag = enable_rag;
    }
    if let Some(enable_cve) = options.get("enable_threat_intel_cve").and_then(|v| v.as_bool()) {
        config.threat_intel_config.enable_cve_tool = enable_cve;
    }

    // 工具白名单/黑名单
    if let Some(tools_allow) = options.get("tools_allow") {
        if let Some(arr) = tools_allow.as_array() {
            let tool_names: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            log::info!("Travel dispatch: 设置 allowed_tools = {:?}", tool_names);
            // Travel的工具白名单将通过task parameters传递
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
                    log::warn!("Failed to get provider config for '{}', falling back to 'default' service", provider);
                    match ai_service_manager.get_service("default") {
                        Some(service) => Arc::new(service),
                        None => {
                            return Err(format!("Failed to get AI provider config for '{}' and no default service available", provider));
                        }
                    }
                }
            }
        }
        _ => {
            log::warn!("No default chat model configured, trying to use 'default' service");
            match ai_service_manager.get_service("default") {
                Some(service) => Arc::new(service),
                None => {
                    return Err("No default AI model configured and no default service available".to_string());
                }
            }
        }
    };

    // 创建 PromptRepository
    let prompt_repo = Arc::new(crate::services::prompt_db::PromptRepository::new(
        db_service.get_pool().map_err(|e| e.to_string())?.clone()
    ));
    
    // 获取 FrameworkToolAdapter - 从全局工具系统动态获取（确保包含最新注册的工具）
    let framework_adapter: Option<std::sync::Arc<dyn crate::tools::FrameworkToolAdapter>> = 
        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
            let tool_manager = tool_system.get_manager();
            // 创建新的 LLMCompilerAdapter，使用当前的 tool_manager（包含所有已注册的工具）
            let adapter = crate::tools::framework_adapters::LLMCompilerAdapter::new(tool_manager);
            log::info!("Travel dispatch: Created LLMCompilerAdapter for TravelEngine");
            Some(std::sync::Arc::new(adapter))
        } else {
            log::warn!("Travel dispatch: Global tool system not available, framework_adapter will be None");
            None
        };

    // 获取 MCP 服务（用于 VisionExplorer）
    let mcp_service = ai_service_manager.get_mcp_service();
    
    // 创建 TravelEngine 并设置依赖
    let mut engine = TravelEngine::new(config)
        .with_ai_service(ai_service.clone())
        .with_prompt_repo(prompt_repo)
        .with_app_handle(app.clone());
    
    // 设置 MCP 服务（用于 VisionExplorer）
    if let Some(mcp) = mcp_service {
        engine = engine.with_mcp_service(mcp);
        log::info!("Travel dispatch: MCP service configured for VisionExplorer");
    } else {
        log::warn!("Travel dispatch: MCP service not available, VisionExplorer will be disabled");
    }
    
    // 设置 PassiveScanState（用于 VisionExplorer 启动代理）
    if let Some(passive_state) = app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>() {
        let passive_state_arc = Arc::new(passive_state.inner().clone());
        engine = engine.with_passive_scan_state(passive_state_arc.clone());
        log::info!("Travel dispatch: PassiveScanState configured for VisionExplorer");
        
        // 获取 PassiveDatabaseService（用于 VisionExplorer 获取代理请求）
        match passive_state.get_db_service().await {
            Ok(db) => {
                engine = engine.with_passive_db(db);
                log::info!("Travel dispatch: PassiveDatabaseService configured for VisionExplorer");
            }
            Err(e) => {
                log::warn!("Travel dispatch: Failed to get PassiveDatabaseService: {}", e);
            }
        }
    } else {
        log::warn!("Travel dispatch: PassiveScanState not available, VisionExplorer won't auto-start proxy");
    }
    
    // 设置 framework_adapter（如果可用）
    if let Some(adapter) = framework_adapter {
        engine = engine.with_framework_adapter(adapter);
    }

    // 使用 LLM 从 query 中智能提取目标信息和任务类型
    let (target_info, task_type, target_type) = extract_target_with_llm(&request.query, &ai_service).await;
    
    log::info!("Travel dispatch: 任务类型={}, 目标={:?}, 目标类型={}", task_type, target_info, target_type);
    
    // 创建 AgentTask
    let task = AgentTask {
        id: execution_id.clone(),
        description: request.query.clone(),
        target: target_info.clone(),
        parameters: {
            let mut map = HashMap::new();
            map.insert("query".to_string(), serde_json::json!(request.query));

            // 透传工具白名单/黑名单
            if let Some(tools_allow) = options.get("tools_allow") {
                map.insert("tools_allow".to_string(), tools_allow.clone());
            }
            if let Some(tools_deny) = options.get("tools_deny") {
                map.insert("tools_deny".to_string(), tools_deny.clone());
            }

            // 添加 conversation_id 和 message_id
            if let Some(conv_id) = &request.conversation_id {
                map.insert("conversation_id".to_string(), serde_json::json!(conv_id));
            }
            if let Some(msg_id) = &request.message_id {
                map.insert("message_id".to_string(), serde_json::json!(msg_id));
            }

            // 添加目标信息和任务类型
            if let Some(target) = options.get("target") {
                map.insert("target".to_string(), target.clone());
            } else if let Some(target) = &target_info {
                // 如果 options 中没有 target，使用从 query 中提取的目标
                map.insert("target".to_string(), serde_json::json!(target));
            }
            
            // 添加任务类型和目标类型
            map.insert("task_type".to_string(), serde_json::json!(task_type));
            map.insert("target_type".to_string(), serde_json::json!(target_type));

            // 添加授权信息
            if let Some(authorized) = options.get("authorized") {
                map.insert("authorized".to_string(), authorized.clone());
            } else {
                // 默认授权（用于测试）
                map.insert("authorized".to_string(), serde_json::json!(true));
            }

            map
        },
        user_id: "default".to_string(),
        priority: TaskPriority::Normal,
        timeout: Some(300000), // 5 minutes default timeout
    };

    // 创建 dummy session
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
                source: "travel".to_string(),
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

    // 执行任务
    let task_clone = session.task.clone();
    let start_time = std::time::Instant::now();
    match engine.execute(&task_clone, &mut session).await {
        Ok(result) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            
            // 从 result.data 中提取响应
            let response = if let Some(data) = &result.data {
                if let Some(output) = data.get("output") {
                    serde_json::to_string_pretty(output).unwrap_or_else(|_| "Travel execution completed".to_string())
                } else {
                    serde_json::to_string_pretty(data).unwrap_or_else(|_| "Travel execution completed".to_string())
                }
            } else {
                "Travel OODA execution completed".to_string()
            };

            Ok(DispatchResult {
                execution_id,
                initial_response: response,
                execution_plan: None,
                estimated_duration: duration_ms,
                selected_architecture: "Travel".to_string(),
            })
        }
        Err(e) => Err(format!("Travel execution failed: {}", e))
    }
}

/// 使用 LLM 从查询文本中提取目标信息和任务类型
async fn extract_target_with_llm(
    query: &str,
    ai_service: &Arc<crate::services::ai::AiService>,
) -> (Option<String>, String, String) {
    let system_prompt = r#"你是一个安全测试任务分析专家。请分析用户输入，提取关键信息。

请按照以下JSON格式返回结果（只返回JSON，不要其他文字）：
{
  "task_type": "任务类型",
  "target": "目标对象",
  "target_type": "目标类型"
}

**任务类型(task_type)选项：**
- web_pentest: Web渗透测试（网站安全测试、漏洞扫描）
- api_pentest: API安全测试（REST API、GraphQL测试）
- code_audit: 代码审计（源码审计、SAST、代码扫描）
- ctf: CTF夺旗赛（解题、challenge）
- reverse_engineering: 逆向工程（二进制分析、反编译）
- forensics: 数字取证（日志分析、事件调查）
- mobile_security: 移动应用安全（Android/iOS测试）
- cloud_security: 云安全评估（AWS/Azure/GCP配置审计）
- iot_security: 物联网/工控安全（智能设备、SCADA）
- network_pentest: 网络渗透（内网渗透、端口扫描）
- social_engineering: 社会工程学（钓鱼测试）
- other: 其他安全测试

**目标类型(target_type)选项：**
- url: HTTP/HTTPS网址
- file_path: 文件或目录路径
- github_repo: GitHub仓库（owner/repo格式）
- ip_address: IP地址或IP段（CIDR）
- domain: 域名
- binary_file: 二进制文件
- mobile_app: 移动应用（包名或APK/IPA）
- cloud_resource: 云资源标识
- none: 无明确目标

**提取规则：**
1. 识别查询中的关键词，判断任务类型
2. 提取具体的目标对象（URL、路径、仓库等）
3. 如果没有明确目标，target设为null
4. 严格按照JSON格式返回，确保可解析

示例：
- "对 http://example.com 进行渗透测试" → {"task_type":"web_pentest","target":"http://example.com","target_type":"url"}
- "审计 /path/to/code 的代码" → {"task_type":"code_audit","target":"/path/to/code","target_type":"file_path"}
- "解这道CTF题" → {"task_type":"ctf","target":null,"target_type":"none"}"#;

    let user_prompt = format!(r#"用户输入："{}"

请提取任务类型、目标和目标类型，返回JSON格式。"#, query);

    // 使用统一的 LlmClient
    let llm_client = crate::engines::create_client(&ai_service);
    
    match llm_client.completion(Some(system_prompt), &user_prompt).await {
        Ok(response) => {
            log::debug!("LLM extraction response: {}", response);
            
            // 尝试从响应中提取JSON（可能包含markdown代码块）
            let json_str: String = if response.contains("```json") {
                // 提取 ```json ... ``` 中的内容
                if let Some(start) = response.find("```json") {
                    let json_start = start + 7; // "```json".len()
                    if let Some(end_pos) = response[json_start..].find("```") {
                        response[json_start..json_start + end_pos].trim().to_string()
                    } else {
                        response.trim().to_string()
                    }
                } else {
                    response.trim().to_string()
                }
            } else if response.contains("```") {
                // 提取 ``` ... ``` 中的内容
                if let Some(start) = response.find("```") {
                    let content_start = start + 3;
                    if let Some(end_pos) = response[content_start..].find("```") {
                        response[content_start..content_start + end_pos].trim().to_string()
                    } else {
                        response.trim().to_string()
                    }
                } else {
                    response.trim().to_string()
                }
            } else {
                response.trim().to_string()
            };
            
            // 解析 JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let task_type = json.get("task_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("other")
                    .to_string();
                
                let target = json.get("target")
                    .and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            v.as_str().map(|s| s.to_string())
                        }
                    });
                
                let target_type = json.get("target_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none")
                    .to_string();
                
                log::info!("✅ LLM extraction - task_type: {}, target: {:?}, target_type: {}", 
                    task_type, target, target_type);
                
                return (target, task_type, target_type);
            } else {
                log::warn!("Failed to parse LLM response as JSON: {}", json_str);
            }
        }
        Err(e) => {
            log::error!("Failed to call LLM for target extraction: {}", e);
        }
    }
    
    // 降级：使用简单的正则提取
    log::info!("⚠️ Falling back to regex-based extraction");
    fallback_extract_target(query)
}

/// 降级方案：使用正则表达式提取目标
fn fallback_extract_target(query: &str) -> (Option<String>, String, String) {
    let query_lower = query.to_lowercase();
    
    // 1. 尝试提取 URL
    if let Ok(url_regex) = regex::Regex::new(r"https?://[^\s]+") {
        if let Some(m) = url_regex.find(query) {
            let url = m.as_str().to_string();
            let task_type = if query_lower.contains("api") {
                "api_pentest"
            } else {
                "web_pentest"
            };
            return (Some(url), task_type.to_string(), "url".to_string());
        }
    }
    
    // 2. 尝试提取文件路径
    let path_patterns = vec![
        r"/[^\s]+",                 // Unix 路径
        r"[A-Z]:\\[^\s]+",          // Windows 路径
        r"\./[^\s]+",               // 相对路径
        r"~/[^\s]+",                // Home 路径
    ];
    
    for pattern in path_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if let Some(m) = regex.find(query) {
                let path = m.as_str().to_string();
                let task_type = if query_lower.contains("代码") || query_lower.contains("code") || query_lower.contains("审计") {
                    "code_audit"
                } else if query_lower.contains("ctf") || query_lower.contains("题") {
                    "ctf"
                } else if query_lower.contains("逆向") || query_lower.contains("reverse") {
                    "reverse_engineering"
                } else if query_lower.contains("取证") || query_lower.contains("forensics") || query_lower.contains("日志") {
                    "forensics"
                } else {
                    "other"
                };
                return (Some(path), task_type.to_string(), "file_path".to_string());
            }
        }
    }
    
    // 3. 尝试提取 GitHub 仓库
    if let Ok(regex) = regex::Regex::new(r"github\.com/([a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+)") {
        if let Some(captures) = regex.captures(query) {
            if let Some(repo) = captures.get(1) {
                return (
                    Some(repo.as_str().to_string()),
                    "code_audit".to_string(),
                    "github_repo".to_string()
                );
            }
        }
    }
    
    // 4. 尝试提取 IP 地址
    if let Ok(regex) = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}(?:/\d{1,2})?\b") {
        if let Some(m) = regex.find(query) {
            return (
                Some(m.as_str().to_string()),
                "network_pentest".to_string(),
                "ip_address".to_string()
            );
        }
    }
    
    // 5. 根据关键词推断任务类型
    let task_type = if query_lower.contains("代码") || query_lower.contains("code") || query_lower.contains("审计") {
        "code_audit"
    } else if query_lower.contains("ctf") || query_lower.contains("夺旗") {
        "ctf"
    } else if query_lower.contains("逆向") || query_lower.contains("reverse") {
        "reverse_engineering"
    } else if query_lower.contains("取证") || query_lower.contains("forensics") {
        "forensics"
    } else if query_lower.contains("api") {
        "api_pentest"
    } else if query_lower.contains("移动") || query_lower.contains("mobile") || query_lower.contains("android") || query_lower.contains("ios") {
        "mobile_security"
    } else if query_lower.contains("云") || query_lower.contains("cloud") || query_lower.contains("aws") || query_lower.contains("azure") {
        "cloud_security"
    } else if query_lower.contains("网络") || query_lower.contains("network") || query_lower.contains("内网") {
        "network_pentest"
    } else {
        "other"
    };
    
    // 没有找到明确目标
    (None, task_type.to_string(), "none".to_string())
}

// ============================================================================
// 自定义 AI 提供商相关命令
// ============================================================================

/// 测试自定义提供商请求参数
#[derive(Debug, Serialize, Deserialize)]
pub struct TestCustomProviderRequest {
    pub name: String,
    pub api_key: Option<String>,
    pub api_base: String,
    pub model_id: String,
    pub compat_mode: String, // openai, anthropic, rig_openai, rig_anthropic
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
    pub timeout: Option<u64>,
}

/// 测试自定义提供商响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TestCustomProviderResponse {
    pub success: bool,
    pub message: String,
}

/// 添加自定义提供商请求参数
#[derive(Debug, Serialize, Deserialize)]
pub struct AddCustomProviderRequest {
    pub name: String,
    pub display_name: String,
    pub api_key: Option<String>,
    pub api_base: String,
    pub model_id: String,
    pub compat_mode: String,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
    pub timeout: Option<u64>,
    pub max_retries: Option<u32>,
}

/// 测试自定义 AI 提供商连接
#[tauri::command]
pub async fn test_custom_provider(
    request: TestCustomProviderRequest,
) -> Result<TestCustomProviderResponse, String> {
    info!("Testing custom provider: {} (mode: {})", request.name, request.compat_mode);
    
    // 使用 rig-core 测试所有提供商
    let result = test_with_rig(&request).await;
    
    match result {
        Ok(msg) => Ok(TestCustomProviderResponse {
            success: true,
            message: msg,
        }),
        Err(e) => Ok(TestCustomProviderResponse {
            success: false,
            message: format!("Connection test failed: {}", e),
        }),
    }
}

/// 使用 rig-core 测试连接
async fn test_with_rig(request: &TestCustomProviderRequest) -> Result<String, String> {
    use rig::client::{CompletionClient, ProviderClient};
    use rig::completion::Prompt;
    
    let provider = if request.compat_mode == "rig_anthropic" {
        "anthropic"
    } else {
        "openai"
    };
    
    // 设置环境变量
    if let Some(api_key) = &request.api_key {
        match provider {
            "anthropic" => {
                std::env::set_var("ANTHROPIC_API_KEY", api_key);
                std::env::set_var("ANTHROPIC_API_BASE", &request.api_base);
            }
            _ => {
                std::env::set_var("OPENAI_API_KEY", api_key);
                std::env::set_var("OPENAI_API_BASE", &request.api_base);
                std::env::set_var("OPENAI_BASE_URL", &request.api_base);
            }
        }
    }
    
    let timeout = std::time::Duration::from_secs(request.timeout.unwrap_or(30));
    
    // 根据 provider 创建 agent
    let response = if provider == "anthropic" {
        use rig::providers::anthropic;
        let client = anthropic::Client::from_env();
        let agent = client.agent(&request.model_id).max_tokens(1024).build();
        tokio::time::timeout(timeout, agent.prompt("Hello, respond with 'OK' if you receive this."))
            .await
            .map_err(|_| "Request timeout".to_string())?
            .map_err(|e| format!("Request failed: {}", e))?
    } else {
        use rig::providers::openai;
        let client = openai::Client::from_env();
        let agent = client.agent(&request.model_id).build();
        tokio::time::timeout(timeout, agent.prompt("Hello, respond with 'OK' if you receive this."))
            .await
            .map_err(|_| "Request timeout".to_string())?
            .map_err(|e| format!("Request failed: {}", e))?
    };
    
    Ok(format!("Connection successful! Response: {}", response.chars().take(100).collect::<String>()))
}

/// 添加自定义 AI 提供商
#[tauri::command]
pub async fn add_custom_provider(
    request: AddCustomProviderRequest,
    db_service: State<'_, Arc<DatabaseService>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<(), String> {
    info!("Adding custom provider: {} ({})", request.name, request.display_name);
    
    // 获取现有的 providers_config
    let mut providers: serde_json::Map<String, serde_json::Value> = 
        match db_service.get_config("ai", "providers_config").await {
            Ok(Some(json_str)) => {
                serde_json::from_str(&json_str).unwrap_or_default()
            }
            _ => serde_json::Map::new(),
        };
    
    // 构建新提供商配置
    let provider_id = request.name.to_lowercase().replace(" ", "_");
    let new_provider = serde_json::json!({
        "id": provider_id,
        "provider": provider_id,
        "name": request.display_name,
        "enabled": true,
        "api_key": request.api_key,
        "api_base": request.api_base,
        "organization": null,
        "default_model": request.model_id,
        "compat_mode": request.compat_mode,
        "extra_headers": request.extra_headers,
        "timeout": request.timeout.unwrap_or(120),
        "max_retries": request.max_retries.unwrap_or(3),
        "is_custom": true,
        "models": [{
            "id": request.model_id,
            "name": request.model_id,
            "description": format!("Custom model from {}", request.display_name),
            "context_length": 4096,
            "supports_streaming": true,
            "supports_tools": false,
            "supports_vision": false,
            "is_available": true
        }]
    });
    
    // 添加到配置
    providers.insert(request.name.clone(), new_provider);
    
    // 保存到数据库
    let config_str = serde_json::to_string(&providers)
        .map_err(|e| format!("Failed to serialize providers config: {}", e))?;
    
    db_service
        .set_config(
            "ai",
            "providers_config",
            &config_str,
            Some("AI providers configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save providers config: {}", e))?;
    
    // 如果有 API Key，单独保存（加密存储）
    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() {
            let key_name = format!("api_key_{}", provider_id);
            db_service
                .set_config("ai", &key_name, api_key, Some(&format!("{} API key", request.display_name)))
                .await
                .map_err(|e| format!("Failed to save API key: {}", e))?;
        }
    }
    
    // 重新加载 AI 服务
    if let Err(e) = ai_manager.reload_services().await {
        warn!("Failed to reload AI services after adding custom provider: {}", e);
    }
    
    info!("Custom provider '{}' added successfully", request.name);
    Ok(())
}

// ============================================================================
// Aliyun DashScope Commands
// ============================================================================

/// 测试阿里云 DashScope 连接
#[tauri::command]
pub async fn test_aliyun_dashscope_connection(
    api_key: String,
    model: String,
) -> Result<bool, String> {
    use crate::utils::aliyun_oss::test_dashscope_connection;
    
    info!("Testing Aliyun DashScope connection with model: {}", model);
    
    test_dashscope_connection(&api_key, &model)
        .await
        .map_err(|e| format!("Connection test failed: {}", e))
}

/// 上传文件到阿里云 OSS
#[tauri::command]
pub async fn upload_file_to_aliyun(
    api_key: String,
    model: String,
    file_path: String,
) -> Result<crate::utils::aliyun_oss::UploadResult, String> {
    use crate::utils::aliyun_oss::upload_file_and_get_url;
    use std::path::Path;
    
    info!("Uploading file to Aliyun OSS: {}", file_path);
    
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    upload_file_and_get_url(&api_key, &model, path)
        .await
        .map_err(|e| format!("Upload failed: {}", e))
}

/// 使用数据库配置上传文件到阿里云 OSS
#[tauri::command]
pub async fn upload_file_to_aliyun_with_config(
    db: tauri::State<'_, Arc<DatabaseService>>,
    file_path: String,
) -> Result<crate::utils::aliyun_oss::UploadResult, String> {
    use crate::utils::aliyun_oss::upload_file_and_get_url;
    use crate::services::database::Database;
    use std::path::Path;
    
    // 从数据库读取配置
    let api_key = db.get_config("ai", "aliyun_dashscope_api_key")
        .await
        .map_err(|e| format!("Failed to get API key: {}", e))?
        .ok_or("Aliyun DashScope API key not configured")?;
    
    let model = db.get_config("ai", "aliyun_dashscope_model")
        .await
        .map_err(|e| format!("Failed to get model: {}", e))?
        .unwrap_or_else(|| "qwen-vl-plus".to_string());
    
    info!("Uploading file to Aliyun OSS with saved config: {}", file_path);
    
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    upload_file_and_get_url(&api_key, &model, path)
        .await
        .map_err(|e| format!("Upload failed: {}", e))
}
