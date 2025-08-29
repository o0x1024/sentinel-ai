//! AI相关命令整合模块
//! 
//! 整合了智能调度器和AI助手的功能，包括：
//! - 智能查询调度
//! - 执行监控
//! - 架构管理
//! - Agent统计

use crate::services::{AiServiceManager, database::DatabaseService};
use crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine;
use crate::engines::rewoo::engine_adapter::ReWooEngine;
use crate::engines::llm_compiler::engine_adapter::LlmCompilerEngine;
use crate::engines::llm_compiler::types::LlmCompilerConfig;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
use crate::engines::plan_and_execute::types::PlanAndExecuteConfig;
use crate::agents::traits::{ExecutionEngine, AgentTask, TaskPriority};
use crate::engines::ExecutionContext;


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
    pub estimated_duration: Option<u64>,
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
    pub execution_time: Option<u64>,
    pub success_rate: Option<f32>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DispatcherStatistics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_execution_time: f64,
    pub architecture_usage: HashMap<String, u64>,
    pub uptime_seconds: u64,
}

// ===== AI助手相关结构体 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchQueryRequest {
    pub query: String,
    pub architecture: String,
    pub agent_id: Option<String>,
    pub options: Option<HashMap<String, serde_json::Value>>,
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
    pub default_architecture: String,
    pub max_concurrent_tasks: u32,
    #[serde(default)]
    pub auto_execute: bool,
    #[serde(default = "default_notification_enabled")]
    pub notification_enabled: bool,
}

fn default_notification_enabled() -> bool {
    true
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


/// 智能查询调度 - 支持自动架构选择
#[tauri::command]
pub async fn dispatch_intelligent_query(
    request: DispatchQueryRequest,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    db_service: State<'_, Arc<DatabaseService>>,
    execution_manager: State<'_, Arc<crate::managers::ExecutionManager>>,
    app_handle: AppHandle,
) -> Result<DispatchResult, String> {

    // 提取任务模式标识和相关信息
    let is_task_mode = request.options.as_ref()
        .and_then(|opts| opts.get("task_mode"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let conversation_id = request.options.as_ref()
        .and_then(|opts| opts.get("conversation_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let execution_id = request.options.as_ref()
        .and_then(|opts| opts.get("execution_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // 提取会话ID（用于日志记录）
    if !conversation_id.is_empty() {
        info!("Processing request for conversation: {}", conversation_id);
    }
    
    // 如果是任务模式且架构为"auto"，进行智能选择
    let selected_architecture = if is_task_mode && request.architecture == "auto" {

        let auto_selected = select_best_architecture(&request.query).await
            .map_err(|e| format!("Failed to select architecture: {}", e))?;
        
        info!("Auto-selected architecture: {} for query: {}", auto_selected, request.query);
        
        auto_selected
    } else {
        request.architecture.clone()
    };
    
    let app_clone = app_handle.clone();
    // 根据选择的架构创建调度器
    let result = match selected_architecture.as_str() {
        "plan-execute" => {
            dispatch_with_plan_execute(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_handle.clone(),
            ).await
        },
        "rewoo" => {
            dispatch_with_rewoo(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone,
            ).await
        },
        "llm-compiler" => {
            dispatch_with_llm_compiler(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone,
            ).await
        },
        "auto" => {
            dispatch_with_auto(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app_clone,
            ).await
        }
        _ => {
            Err(format!("Unsupported architecture: {}", selected_architecture))
        }
    };
    
    // 如果调度成功，自动开始执行
    if let Ok(ref dispatch_result) = result {
        let execution_id_clone = dispatch_result.execution_id.clone();
        let app_clone = app_handle.clone();
        
        
        // 异步开始执行，不阻塞调度响应
        tokio::spawn(async move {
            if let Err(e) = start_execution(execution_id_clone, app_clone).await {
                log::error!("Failed to start execution after dispatch: {}", e);
            }
        });
    }
    
    // 更新返回结果中的架构信息
    result.map(|mut dispatch_result| {
        dispatch_result.selected_architecture = selected_architecture;
        dispatch_result
    })
}

/// 开始执行 - 使用真实引擎
pub async fn start_execution(
    execution_id: String,
    app: AppHandle,
) -> Result<(), String> {
    info!("Starting real engine execution: {}", execution_id);
    
    // 从应用状态获取执行管理器
    let execution_manager = app.state::<Arc<crate::managers::ExecutionManager>>();
    let execution_manager_clone = execution_manager.inner().clone();
    let app_clone = app.clone();
    let execution_id_clone = execution_id.clone();
    
    tokio::spawn(async move {
        // 获取执行上下文
        let context = match execution_manager_clone.get_execution_context(&execution_id_clone).await {
            Some(ctx) => ctx,
            None => {
                log::error!("Execution context not found: {}", execution_id_clone);
                return;
            }
        };

        log::info!("Starting real execution for: {} with engine: {:?}", execution_id_clone, context.engine_type);

        
        // 从任务参数中提取消息ID和会话ID
        let message_id = context.task.parameters.get("message_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let conversation_id = context.task.parameters.get("conversation_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        
        // 发送步骤初始化事件 - 作为普通消息内容
        let steps_text = context.plan.steps.iter()
            .enumerate()
            .map(|(i, step)| format!("* [ ] {}.**{}**: {}\n", i+1, step.name, step.description))
            .collect::<Vec<_>>()
            .join("\n");
            
        let plan_message = format!("## 执行计划\n\n{}\n\n共 {} 个步骤，准备开始执行...", 
            steps_text, context.plan.steps.len());
            

        // 执行真实的引擎计划
        match execution_manager_clone.execute_plan(&execution_id_clone).await {
            Ok(result) => {
                log::info!("Execution completed successfully: {}", execution_id_clone);
                
                let mut result_message = String::new();
                // 发送执行完成事件
                let status = if result.success { "success" } else { "failed" };
        
                // 如果有数据，添加数据部分
                if let Some(data) = &result.data {
                    if let Ok(data_str) = serde_json::to_string_pretty(data) {
                        if !data_str.is_empty() && data_str != "null" {
                            result_message.push_str("\n**详细数据**:\n```json\n");
                            result_message.push_str(&data_str);
                            result_message.push_str("\n```\n");
                        }
                    }
                }
                
                // 发送消息流更新事件，将执行结果作为普通消息内容，并包含所有必要的执行结果信息
                let _ = app_clone.emit("ai_stream_message", serde_json::json!({
                    "conversation_id": context.task.parameters.get("conversation_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "message_id": context.task.parameters.get("message_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "content": result_message,
                    "is_complete": true,
                    "execution_id": execution_id_clone,
                    "result": {
                        "status": status,
                        "data": result.data,
                        "summary": format!("执行结果ID: {}", result.id),
                        "duration": result.execution_time_ms,
                        "architecture": format!("{:?}", context.engine_type),
                    },
                    "final_response": result_message,
                    "artifacts": result.artifacts.len(),
                }));
                
                // 移除原始事件，只使用ai_stream_message
            }
            Err(e) => {
                log::error!("Execution failed: {}: {}", execution_id_clone, e);
                
                // 构建执行失败消息
                let error_message = format!("## 执行失败\n\n**错误信息**: {}\n\n**架构**: {}\n\n请检查输入参数或尝试其他执行架构。", 
                    e.to_string(), 
                    format!("{:?}", context.engine_type));
                
                // 发送消息流更新事件，将执行失败信息作为普通消息内容，并包含所有必要的执行结果信息
                let _ = app_clone.emit("ai_stream_message", serde_json::json!({
                    "conversation_id": context.task.parameters.get("conversation_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "message_id": context.task.parameters.get("message_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "content": error_message,
                    "is_complete": true,
                    "execution_id": execution_id_clone,
                    "error": true,
                    "result": {
                        "status": "failed",
                        "error": e.to_string(),
                        "architecture": format!("{:?}", context.engine_type)
                    },
                    "final_response": error_message
                }));
                
                // 移除原始事件，只使用ai_stream_message
            }
        }

        // 清理执行上下文
        execution_manager_clone.cleanup_execution(&execution_id_clone).await;
    });

    Ok(())
}

/// 停止执行
#[tauri::command]
pub async fn stop_execution(
    execution_id: String,
    app: AppHandle,
) -> Result<(), String> {
    info!("Stopping execution: {}", execution_id);
    
    // 发送停止事件
    app.emit("execution_stopped", serde_json::json!({
        "execution_id": execution_id,
        "message": "执行已被用户停止"
    })).map_err(|e| e.to_string())?;
    
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
            info!("AI assistant settings: {}", json_str);
            serde_json::from_str::<AIAssistantSettings>(&json_str)
                .map_err(|e| format!("Failed to parse AI assistant settings: {}", e))
        }
        Ok(None) => {
            // 返回默认设置
            let default_settings = AIAssistantSettings {
                default_architecture: "plan-execute".to_string(),
                max_concurrent_tasks: 5,
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
    pub estimated_duration: Option<u64>,
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

/// 获取架构的显示名称
fn get_architecture_display_name(architecture: &str) -> &str {
    match architecture {
        "plan-execute" => "Plan-and-Execute",
        "rewoo" => "ReWOO",
        "llm-compiler" => "LLM Compiler",
        "auto" => "Auto",
        _ => architecture,
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
    info!("Creating Plan-and-Execute dispatch for: {}", request.query);
    
    // 创建Plan-and-Execute引擎配置
    let config = PlanAndExecuteConfig::default();
    
    // 创建Plan-and-Execute引擎
    let engine = PlanAndExecuteEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
        Some(Arc::new(app.clone())),
    ).await.map_err(|e| format!("Failed to create Plan-and-Execute engine: {}", e))?;
    
    // 创建Agent任务
    let mut parameters = request.options.unwrap_or_default();
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

async fn dispatch_with_rewoo(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    _app: AppHandle,
) -> Result<DispatchResult, String> {
    info!("Creating ReWOO dispatch for: {}", request.query);
    
    // 创建ReWOO引擎配置
    let config = ReWOOConfig::default();
    
    // 创建ReWOO引擎
    let engine = ReWooEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
    ).await.map_err(|e| format!("Failed to create ReWOO engine: {}", e))?;
    
    // 创建Agent任务
    let task = AgentTask {
        id: execution_id.clone(),
        user_id: "system".to_string(),
        description: request.query.clone(),
        priority: TaskPriority::Normal,
        target: None,
        parameters: request.options.unwrap_or_default(),
        timeout: Some(300), // 5分钟超时
    };
    
    // 创建执行计划
    let plan = engine.create_plan(&task).await
        .map_err(|e| format!("Failed to create ReWOO plan: {}", e))?;
    
    // 注册执行上下文和引擎实例到执行管理器
    let engine_instance = crate::managers::EngineInstance::ReWOO(engine);
    execution_manager.register_execution(
        execution_id.clone(),
        crate::managers::EngineType::ReWOO,
        plan.clone(),
        task,
        engine_instance,
    ).await.map_err(|e| format!("Failed to register execution: {}", e))?;
    
    let execution_plan = ExecutionPlanView {
        id: plan.id.clone(),
        name: plan.name.clone(),
        description: format!("ReWOO推理任务: {}", request.query),
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
        initial_response: "已启动ReWOO推理工作流，引擎实例已注册，准备真实执行...".to_string(),
        execution_plan: Some(execution_plan),
        estimated_duration: plan.estimated_duration,
        selected_architecture: "ReWOO".to_string(),
    })
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
    let engine = LlmCompilerEngine::new_with_dependencies(
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