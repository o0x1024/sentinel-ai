//! 测试Agent执行流程的端到端验证
//! 
//! 这个模块提供了完整的Agent系统测试，验证从初始化到执行的整个流程

use crate::agents::{AgentManager, AgentSelectionStrategy, MultiAgentRequest, TaskPriority};
use crate::tools::ToolSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::RwLock;

/// 全局Agent管理器类型
pub type GlobalAgentManager = Arc<RwLock<Option<AgentManager>>>;

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFlowTestResult {
    /// 测试是否成功
    pub success: bool,
    /// 测试阶段
    pub stage: String,
    /// 测试消息
    pub message: String,
    /// 执行详情
    pub details: Option<serde_json::Value>,
    /// 执行时间（毫秒）
    pub execution_time_ms: f64,
}

/// 端到端Agent流程测试
#[command]
pub async fn test_complete_agent_flow(
    agent_manager: State<'_, GlobalAgentManager>,
    tool_system: State<'_, Arc<ToolSystem>>,
) -> Result<AgentFlowTestResult, String> {
    let start_time = std::time::Instant::now();
    
    log::info!("Starting complete agent flow test");
    
    // 阶段1: 检查Agent管理器是否已初始化
    let manager = {
        let manager_guard = agent_manager.read().await;
        if manager_guard.is_none() {
            return Ok(AgentFlowTestResult {
                success: false,
                stage: "initialization".to_string(),
                message: "Agent manager not initialized".to_string(),
                details: None,
                execution_time_ms: start_time.elapsed().as_millis() as f64,
            });
        }
        manager_guard.as_ref().unwrap().clone()
    };
    
    // 阶段2: 检查引擎是否注册成功
    let engines = manager.list_engines().await;
    if engines.is_empty() {
        return Ok(AgentFlowTestResult {
            success: false,
            stage: "engine_registration".to_string(),
            message: "No engines registered".to_string(),
            details: Some(serde_json::json!({"engines": engines})),
            execution_time_ms: start_time.elapsed().as_millis() as f64,
        });
    }
    
    log::info!("Found {} registered engines: {:?}", engines.len(), engines);
    
    // 阶段3: 检查Agent是否注册成功
    let agents = manager.list_agents().await;
    if agents.is_empty() {
        return Ok(AgentFlowTestResult {
            success: false,
            stage: "agent_registration".to_string(),
            message: "No agents registered".to_string(),
            details: Some(serde_json::json!({"agents": agents, "engines": engines})),
            execution_time_ms: start_time.elapsed().as_millis() as f64,
        });
    }
    
    log::info!("Found {} registered agents: {:?}", agents.len(), agents);
    
    // 阶段4: 检查工具系统是否可用
    let available_tools = tool_system.list_tools().await;
    log::info!("Found {} available tools", available_tools.len());
    
    // 阶段5: 执行一个简单的测试任务
    let test_request = MultiAgentRequest {
        user_input: "Test port scan on localhost".to_string(),
        target: Some("127.0.0.1".to_string()),
        context: HashMap::new(),
        selection_strategy: AgentSelectionStrategy::Specific("plan_execute_agent".to_string()),
        priority: TaskPriority::Normal,
        user_id: "test_user".to_string(),
    };
    
    match manager.dispatch_task(test_request).await {
        Ok(session_id) => {
            log::info!("Successfully dispatched test task: {}", session_id);
            
            // 等待一小段时间让任务开始执行
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // 检查任务状态
            let active_sessions = manager.active_sessions.read().await;
            let session_exists = active_sessions.contains_key(&session_id);
            
            let execution_time = start_time.elapsed().as_millis() as f64;
            
            Ok(AgentFlowTestResult {
                success: true,
                stage: "task_execution".to_string(),
                message: "Agent flow test completed successfully".to_string(),
                details: Some(serde_json::json!({
                    "session_id": session_id,
                    "engines": engines,
                    "agents": agents,
                    "available_tools": available_tools.len(),
                    "session_exists": session_exists,
                    "test_target": "127.0.0.1"
                })),
                execution_time_ms: execution_time,
            })
        }
        Err(e) => {
            Ok(AgentFlowTestResult {
                success: false,
                stage: "task_dispatch".to_string(),
                message: format!("Failed to dispatch test task: {}", e),
                details: Some(serde_json::json!({
                    "error": e.to_string(),
                    "engines": engines,
                    "agents": agents,
                    "available_tools": available_tools.len()
                })),
                execution_time_ms: start_time.elapsed().as_millis() as f64,
            })
        }
    }
}

/// 测试工具系统可用性
#[command]
pub async fn test_tool_system_availability(
    tool_system: State<'_, Arc<ToolSystem>>,
) -> Result<serde_json::Value, String> {
    let start_time = std::time::Instant::now();
    
    // 获取所有可用工具
    let tools = tool_system.list_tools().await;
    
    // 检查内置工具
    let builtin_tools: Vec<_> = tools.iter()
        .filter(|tool| tool.available)
        .collect();
    
    let execution_time = start_time.elapsed().as_millis() as f64;
    
    Ok(serde_json::json!({
        "total_tools": tools.len(),
        "available_tools": builtin_tools.len(),
        "tools": builtin_tools.iter().map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "category": tool.category,
                "description": tool.description,
                "available": tool.available
            })
        }).collect::<Vec<_>>(),
        "execution_time_ms": execution_time
    }))
}

/// 测试特定工具的执行
#[command]
pub async fn test_tool_execution(
    tool_system: State<'_, Arc<ToolSystem>>,
    tool_name: String,
    target: Option<String>,
) -> Result<serde_json::Value, String> {
    let start_time = std::time::Instant::now();
    
    let target = target.unwrap_or_else(|| "127.0.0.1".to_string());
    
    // 准备工具执行参数
    let mut inputs = HashMap::new();
    inputs.insert("target".to_string(), serde_json::json!(target));
    inputs.insert("ports".to_string(), serde_json::json!("22,80,443"));
    inputs.insert("threads".to_string(), serde_json::json!(10));
    
    let execution_params = crate::tools::ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: Some(std::time::Duration::from_secs(30)),
        execution_id: Some(uuid::Uuid::new_v4()),
    };
    
    match tool_system.execute_tool(&tool_name, execution_params).await {
        Ok(result) => {
            let execution_time = start_time.elapsed().as_millis() as f64;
            
            Ok(serde_json::json!({
                "success": true,
                "tool_name": tool_name,
                "target": target,
                "execution_time_ms": execution_time,
                "result": {
                    "execution_id": result.execution_id,
                    "tool_name": result.tool_name,
                    "success": result.success,
                    "output": result.output,
                    "duration_ms": result.execution_time_ms,
                    "error": result.error
                }
            }))
        }
        Err(e) => {
            let execution_time = start_time.elapsed().as_millis() as f64;
            
            Ok(serde_json::json!({
                "success": false,
                "tool_name": tool_name,
                "target": target,
                "execution_time_ms": execution_time,
                "error": e.to_string()
            }))
        }
    }
}
