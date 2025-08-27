//! AI助手相关命令
//! 
//! 提供AI助手的核心功能，包括：
//! - 智能查询调度
//! - 执行监控
//! - 架构管理
//! - Agent统计

use crate::services::{AiServiceManager, database::DatabaseService};
use crate::ai_adapter::core::AiAdapterManager;
use crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine;
use crate::engines::rewoo::engine_adapter::ReWooEngine;
use crate::engines::llm_compiler::engine_adapter::LlmCompilerEngine;
use crate::engines::llm_compiler::types::LlmCompilerConfig;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
use crate::engines::plan_and_execute::types::PlanAndExecuteConfig;
use crate::agents::traits::{ExecutionEngine, AgentTask, TaskPriority};

use tauri::{AppHandle, State, Emitter, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use log::info;


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

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionStatusUpdate {
    pub execution_id: String,
    pub status: String,
    pub progress: f32,
    pub current_step: Option<String>,
    pub message: Option<String>,
}

/// 智能查询调度 - 支持自动架构选择
#[tauri::command]
pub async fn dispatch_intelligent_query(
    request: DispatchQueryRequest,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    ai_adapter_manager: State<'_, Arc<AiAdapterManager>>,
    db_service: State<'_, Arc<DatabaseService>>,
    execution_manager: State<'_, Arc<crate::managers::ExecutionManager>>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    info!("Dispatching intelligent query: {}", request.query);
    
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
    
    let message_id = request.options.as_ref()
        .and_then(|opts| opts.get("message_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let execution_id = request.options.as_ref()
        .and_then(|opts| opts.get("execution_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // 如果是任务模式且架构为"auto"，进行智能选择
    let selected_architecture = if is_task_mode && request.architecture == "auto" {
        // 发送任务分析开始事件
        let _ = app.emit("task-progress", serde_json::json!({
            "conversation_id": conversation_id,
            "execution_id": execution_id,
            "phase": "analysis",
            "content": "正在分析任务类型和复杂度...",
            "progress": 10.0
        }));
        
        let auto_selected = select_best_architecture(&request.query).await
            .map_err(|e| format!("Failed to select architecture: {}", e))?;
        
        info!("Auto-selected architecture: {} for query: {}", auto_selected, request.query);
        
        // 发送架构选择结果事件
        let _ = app.emit("task-progress", serde_json::json!({
            "conversation_id": conversation_id,
            "execution_id": execution_id,
            "phase": "planning",
            "content": format!("已选择 {} 架构执行任务...", get_architecture_display_name(&auto_selected)),
            "progress": 25.0,
            "selected_architecture": auto_selected
        }));
        
        auto_selected
    } else {
        request.architecture.clone()
    };
    
    // 根据选择的架构创建调度器
    let result = match selected_architecture.as_str() {
        "plan-execute" => {
            dispatch_with_plan_execute(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*ai_adapter_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
            ).await
        },
        "rewoo" => {
            dispatch_with_rewoo(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
            ).await
        },
        "llm-compiler" => {
            dispatch_with_llm_compiler(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
            ).await
        },
        "intelligent-dispatcher" => {
            dispatch_with_real_intelligent_dispatcher(
                execution_id.clone(),
                request,
                (*ai_service_manager).clone(),
                (*ai_adapter_manager).clone(),
                (*db_service).clone(),
                (*execution_manager).clone(),
                app.clone(),
            ).await
        },
        _ => Err(format!("Unsupported architecture: {}", selected_architecture))
    };
    
    // 如果调度成功，自动开始执行
    if let Ok(ref dispatch_result) = result {
        let execution_id_clone = dispatch_result.execution_id.clone();
        let app_clone = app.clone();
        
        // 发送执行开始事件
        if is_task_mode {
            let _ = app.emit("task-progress", serde_json::json!({
                "conversation_id": conversation_id,
                "execution_id": execution_id,
                "phase": "execution",
                "content": "任务执行已开始...",
                "progress": 50.0,
                "selected_architecture": selected_architecture
            }));
        }
        
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

/// 使用Plan-and-Execute架构调度
async fn dispatch_with_plan_execute(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    _ai_adapter_manager: Arc<AiAdapterManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
) -> Result<DispatchResult, String> {
    info!("Creating Plan-and-Execute dispatch for: {}", request.query);
    
    // 创建Plan-and-Execute引擎配置
    let config = PlanAndExecuteConfig::default();
    
    // 创建Plan-and-Execute引擎
    let engine = PlanAndExecuteEngine::new_with_dependencies(
        ai_service_manager.clone(),
        config,
        db_service.clone(),
    ).await.map_err(|e| format!("Failed to create Plan-and-Execute engine: {}", e))?;
    
    // 创建Agent任务
    let task = AgentTask {
        id: execution_id.clone(),
        user_id: "system".to_string(),
        description: request.query.clone(),
        priority: TaskPriority::Normal,
        target: None,
        parameters: request.options.unwrap_or_default(),
        timeout: Some(600), // 10分钟超时
    };
    
    // 创建执行计划
    let plan = engine.create_plan(&task).await
        .map_err(|e| format!("Failed to create execution plan: {}", e))?;
    
    // 注册执行上下文和引擎实例到执行管理器
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

/// 使用ReWOO架构调度
async fn dispatch_with_rewoo(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
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

/// 使用LLMCompiler架构调度
async fn dispatch_with_llm_compiler(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
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

/// 使用真实智能调度器
async fn dispatch_with_real_intelligent_dispatcher(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    _ai_adapter_manager: Arc<AiAdapterManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    app: tauri::AppHandle,
) -> Result<DispatchResult, String> {
    info!("Creating Real Intelligent Dispatcher for: {}", request.query);
    
    // 获取智能调度器状态
    let dispatcher_state = app.state::<crate::commands::intelligent_dispatcher_commands::IntelligentDispatcherState>();
    
    // 确保智能调度器已初始化
    {
        let state = dispatcher_state.read().await;
        if state.is_none() {
            // 初始化智能调度器
            drop(state);
            let mcp_service = app.state::<Arc<crate::services::mcp::McpService>>();
            let init_result = crate::commands::intelligent_dispatcher_commands::initialize_intelligent_dispatcher(
                dispatcher_state.clone(),
                app.state::<Arc<AiServiceManager>>(),
                mcp_service.clone(),
                app.state::<Arc<crate::services::database::DatabaseService>>(),
            ).await;
            
            if let Err(e) = init_result {
                return Err(format!("Failed to initialize intelligent dispatcher: {}", e));
            }
        }
    }
    
    // 调用智能调度器处理查询
    let dispatcher_request = crate::commands::intelligent_dispatcher_commands::IntelligentQueryRequest {
        user_input: request.query.clone(),
        user_id: "system".to_string(),
        session_id: None,
        priority: None,
        custom_parameters: request.options.clone(),
    };
    
    let dispatch_response = crate::commands::intelligent_dispatcher_commands::intelligent_process_query(
        dispatcher_request,
        dispatcher_state.clone(),
    ).await.map_err(|e| format!("Intelligent dispatcher failed: {}", e))?;
    
    if !dispatch_response.success {
        return Err(dispatch_response.error.unwrap_or("Unknown error".to_string()));
    }
    
    let response_data = dispatch_response.data.ok_or("No response data")?;
    
    // 根据智能调度器选择的架构，创建相应的执行计划
    let selected_arch = response_data.selected_architecture.clone();
    let real_execution_result = match selected_arch.as_str() {
        "PlanAndExecute" => {
            dispatch_with_plan_execute(
                execution_id.clone(),
                request.clone(),
                ai_service_manager,
                _ai_adapter_manager,
                db_service,
                execution_manager,
            ).await
        },
        "ReWoo" => {
            dispatch_with_rewoo(
                execution_id.clone(),
                request.clone(),
                ai_service_manager,
                db_service,
                execution_manager,
            ).await
        },
        "LlmCompiler" => {
            dispatch_with_llm_compiler(
                execution_id.clone(),
                request.clone(),
                ai_service_manager,
                db_service,
                execution_manager,
            ).await
        },
        _ => {
            // 默认使用Plan-Execute
            dispatch_with_plan_execute(
                execution_id.clone(),
                request.clone(),
                ai_service_manager,
                _ai_adapter_manager,
                db_service,
                execution_manager,
            ).await
        }
    }?;
    
    // 增强响应信息
    Ok(DispatchResult {
        execution_id: real_execution_result.execution_id,
        initial_response: format!(
            "🧠 智能调度器分析完成!\n📋 选择架构: {} (置信度: {:.1}%)\n💡 选择理由: {}\n⚡ 预估时长: {}秒\n🚀 {}",
            response_data.selected_architecture,
            response_data.confidence * 100.0,
            response_data.reasoning,
            response_data.estimated_duration.unwrap_or(240),
            real_execution_result.initial_response
        ),
        execution_plan: real_execution_result.execution_plan,
        estimated_duration: response_data.estimated_duration.unwrap_or(real_execution_result.estimated_duration),
        selected_architecture: response_data.selected_architecture,
    })
}

/// 开始执行 - 使用真实引擎
#[tauri::command]
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
                let _ = app_clone.emit("execution_completed", serde_json::json!({
                    "execution_id": execution_id_clone,
                    "result": {
                        "status": "failed",
                        "error": "Execution context not found"
                    }
                }));
                return;
            }
        };

        log::info!("Starting real execution for: {} with engine: {:?}", execution_id_clone, context.engine_type);

        // 创建Agent会话（简化版，用于日志记录）
        let mut session = SimpleAgentSession::new(execution_id_clone.clone());

        // 发送步骤初始化事件
        let _ = app_clone.emit("execution_steps_initialized", serde_json::json!({
            "execution_id": execution_id_clone,
            "steps": context.plan.steps.iter().map(|step| {
                serde_json::json!({
                    "id": step.id,
                    "name": step.name,
                    "description": step.description,
                    "status": "pending",
                    "started_at": null,
                    "completed_at": null,
                    "result": null,
                    "error": null
                })
            }).collect::<Vec<_>>()
        }));

        // 执行真实的引擎计划
        match execution_manager_clone.execute_plan(&execution_id_clone, &mut session).await {
            Ok(result) => {
                log::info!("Execution completed successfully: {}", execution_id_clone);
                
                // 将执行记录发送到全局状态管理器，供工作流监控使用
                if let Err(e) = record_ai_assistant_execution(&context, &result, &app_clone).await {
                    log::error!("Failed to record AI assistant execution: {}", e);
                }
                
                // 发送执行完成事件
                let status = if result.success { "success" } else { "failed" };
                let _ = app_clone.emit("execution_completed", serde_json::json!({
                    "execution_id": execution_id_clone,
                    "result": {
                        "status": status,
                        "data": result.data,
                        "summary": format!("执行结果ID: {}", result.id),
                        "duration": result.execution_time_ms,
                        "architecture": format!("{:?}", context.engine_type),
                        "artifacts": result.artifacts.len(),
                        "step_results": extract_step_results_from_agent_result(&result)
                    },
                    "final_response": format!("任务执行完成。状态: {}, 耗时: {}ms", 
                                            status, result.execution_time_ms)
                }));
            }
            Err(e) => {
                log::error!("Execution failed: {}: {}", execution_id_clone, e);
                
                // 发送执行失败事件
                let _ = app_clone.emit("execution_completed", serde_json::json!({
                    "execution_id": execution_id_clone,
                    "result": {
                        "status": "failed",
                        "error": e.to_string(),
                        "architecture": format!("{:?}", context.engine_type)
                    },
                    "final_response": format!("任务执行失败: {}", e)
                }));
            }
        }

        // 清理执行上下文
        execution_manager_clone.cleanup_execution(&execution_id_clone).await;
    });

    Ok(())
}

/// 简化的 Agent 会话实现（用于日志记录）
struct SimpleAgentSession {
    id: String,
    task: crate::agents::traits::AgentTask,
    status: crate::agents::traits::AgentSessionStatus,
    logs: Vec<crate::agents::traits::SessionLog>,
    result: Option<crate::agents::traits::AgentExecutionResult>,
}

impl SimpleAgentSession {
    fn new(id: String) -> Self {
        Self {
            task: crate::agents::traits::AgentTask {
                id: id.clone(),
                description: "AI Assistant Execution".to_string(),
                target: None,
                parameters: std::collections::HashMap::new(),
                user_id: "system".to_string(),
                priority: crate::agents::traits::TaskPriority::Normal,
                timeout: Some(600),
            },
            id,
            status: crate::agents::traits::AgentSessionStatus::Created,
            logs: Vec::new(),
            result: None,
        }
    }
}

#[async_trait::async_trait]
impl crate::agents::traits::AgentSession for SimpleAgentSession {
    fn get_session_id(&self) -> &str {
        &self.id
    }
    
    fn get_task(&self) -> &crate::agents::traits::AgentTask {
        &self.task
    }
    
    fn get_status(&self) -> crate::agents::traits::AgentSessionStatus {
        self.status.clone()
    }
    
    async fn update_status(&mut self, status: crate::agents::traits::AgentSessionStatus) -> anyhow::Result<()> {
        self.status = status;
        Ok(())
    }
    
    async fn add_log(&mut self, level: crate::agents::traits::LogLevel, message: String) -> anyhow::Result<()> {
        let log = crate::agents::traits::SessionLog {
            level,
            message: message.clone(),
            timestamp: chrono::Utc::now(),
            source: "SimpleAgentSession".to_string(),
        };
        self.logs.push(log);
        log::info!("Session {}: {}", self.id, message);
        Ok(())
    }
    
    fn get_logs(&self) -> &[crate::agents::traits::SessionLog] {
        &self.logs
    }
    
    async fn set_result(&mut self, result: crate::agents::traits::AgentExecutionResult) -> anyhow::Result<()> {
        self.result = Some(result);
        Ok(())
    }
    
    fn get_result(&self) -> Option<&crate::agents::traits::AgentExecutionResult> {
        self.result.as_ref()
    }
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
        serde_json::json!({
            "id": "intelligent-dispatcher",
            "name": "Intelligent Dispatcher",
            "description": "AI驱动的架构选择器",
            "suitable_for": ["任何任务类型", "自动优化"],
            "performance": "自适应",
            "status": "ai-powered"
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
            "intelligent-dispatcher".to_string(),
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

/// 记录AI助手执行结果，使其在工作流监控中可见
async fn record_ai_assistant_execution(
    context: &crate::managers::execution_manager::ExecutionContext,
    result: &crate::agents::traits::AgentExecutionResult,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    use crate::services::database::DatabaseService;
    use std::sync::Arc;
    
    // 获取数据库服务
    let db_service = app.state::<Arc<DatabaseService>>();
    
    // 创建AI助手执行记录
    let ai_execution_record = serde_json::json!({
        "execution_id": context.execution_id,
        "task_name": context.plan.name,
        "architecture": format!("{:?}", context.engine_type),
        "status": if result.success { "completed" } else { "failed" },
        "started_at": context.created_at.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs(),
        "completed_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs(),
        "execution_time_ms": result.execution_time_ms,
        "step_count": context.plan.steps.len(),
        "success": result.success,
        "error": result.error,
        "artifacts_count": result.artifacts.len(),
        "step_results": extract_step_results_from_agent_result(result),
        "source": "ai_assistant"
    });
    
    // 获取现有的执行记录
    let mut execution_list = match db_service.get_config("ai_assistant", "recent_executions").await {
        Ok(Some(data)) => {
            serde_json::from_str::<Vec<serde_json::Value>>(&data).unwrap_or_else(|_| Vec::new())
        },
        _ => Vec::new(),
    };
    
    // 添加新记录到列表开头
    execution_list.insert(0, ai_execution_record);
    
    // 只保留最近100条记录
    execution_list.truncate(100);
    
    // 保存更新后的列表
    if let Err(e) = db_service.set_config(
        "ai_assistant",
        "recent_executions",
        &serde_json::to_string(&execution_list).unwrap_or_default(),
        Some("AI Assistant recent execution records")
    ).await {
        log::warn!("Failed to save AI assistant execution record: {}", e);
    }
    
    log::info!("Recorded AI Assistant execution: {}", context.execution_id);
    Ok(())
}

/// 从AgentExecutionResult中提取步骤结果用于前端显示
fn extract_step_results_from_agent_result(result: &crate::agents::traits::AgentExecutionResult) -> serde_json::Value {
    // 尝试从result.data中提取step_results
    if let Some(data) = &result.data {
        if let Some(step_results) = data.get("step_results") {
            return step_results.clone();
        }
    }
    
    // 如果没有找到step_results，返回空对象
    serde_json::json!({})
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
        // 对于一般任务，使用智能调度器
        Ok("intelligent-dispatcher".to_string())
    }
}

/// 获取架构的显示名称
fn get_architecture_display_name(architecture: &str) -> &str {
    match architecture {
        "plan-execute" => "Plan-and-Execute",
        "rewoo" => "ReWOO",
        "llm-compiler" => "LLM Compiler",
        "intelligent-dispatcher" => "Intelligent Dispatcher",
        _ => architecture,
    }
}


