//! ReAct 架构 Tauri 命令
//!
//! 提供 ReAct (Reasoning + Acting) 架构的前端接口

use crate::commands::ai_commands::CommandResponse as AiCommandResponse;
use crate::engines::react::{ReactEngine, ReactConfig};
use crate::agents::traits::{AgentTask, AgentExecutionResult};
use crate::agents::TaskPriority;
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tauri::{AppHandle, State};
use tracing::{error, info};

/// ReAct 执行请求
#[derive(Debug, Deserialize)]
pub struct ExecuteReactRequest {
    /// 任务描述
    pub task: String,
    /// 可选配置覆盖
    pub config: Option<ReactConfigOverride>,
    /// 对话 ID（用于流式消息）
    pub conversation_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
}

/// ReAct 配置覆盖
#[derive(Debug, Deserialize)]
pub struct ReactConfigOverride {
    pub max_iterations: Option<u32>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub enable_rag: Option<bool>,
    pub verbose: Option<bool>,
}

/// ReAct 执行响应
#[derive(Debug, Serialize)]
pub struct ExecuteReactResponse {
    pub trace_id: String,
    pub status: String,
    pub answer: Option<String>,
    pub iterations: u32,
    pub tool_calls: u32,
    pub duration_ms: u64,
    pub metadata: serde_json::Value,
}

/// 执行 ReAct 任务
#[tauri::command]
pub async fn execute_react_task(
    request: ExecuteReactRequest,
    app: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<AiCommandResponse<ExecuteReactResponse>, String> {
    info!("执行 ReAct 任务: {}", request.task);

    // 构建配置
    let mut config = ReactConfig::default();
    let max_iterations = config.max_iterations; // 保存副本用于超时计算
    if let Some(override_cfg) = request.config {
        if let Some(max_iter) = override_cfg.max_iterations {
            config.max_iterations = max_iter;
        }
        if let Some(temp) = override_cfg.temperature {
            config.temperature = Some(temp);
        }
        if let Some(max_tok) = override_cfg.max_tokens {
            config.max_tokens = Some(max_tok);
        }
        if let Some(enable_rag) = override_cfg.enable_rag {
            config.enable_rag = enable_rag;
        }
        if let Some(verbose) = override_cfg.verbose {
            config.verbose = verbose;
        }
    }

    // 获取默认 AI 服务
    let ai_service = match ai_manager.get_default_chat_model().await {
        Ok(Some((provider, model))) => {
            match ai_manager.get_provider_config(&provider).await {
                Ok(Some(mut provider_config)) => {
                    provider_config.model = model;
                    let mcp_service = ai_manager.get_mcp_service();
                    let ai_svc = crate::services::ai::AiService::new(
                        provider_config,
                        db.inner().clone(),
                        Some(app.clone()),
                        mcp_service,
                    );
                    Arc::new(ai_svc)
                }
                _ => {
                    return Ok(AiCommandResponse::error(
                        "Failed to get AI provider config".to_string()
                    ));
                }
            }
        }
        _ => {
            return Ok(AiCommandResponse::error(
                "No default AI model configured".to_string()
            ));
        }
    };

    // 序列化 config（在 move 之前）
    let config_json = serde_json::to_value(&config).map_err(|e| e.to_string())?;

    // 创建 ReAct 引擎
    let engine = ReactEngine::new(config).with_services(
        ai_service,
        ai_manager.get_mcp_service(),
        Some(db.inner().clone()),
        Some(app.clone()),
    );

    // 创建 AgentTask
    let task = AgentTask {
        id: format!("react-{}", chrono::Utc::now().timestamp()),
        description: request.task.clone(),
        target: None,
        parameters: {
            let mut map = HashMap::new();
            map.insert("query".to_string(), serde_json::json!(request.task));
            map.insert("config".to_string(), config_json);
            // 透传对话标识，供引擎与 AiService 进行消息关联
            if let Some(conv_id) = &request.conversation_id {
                map.insert("conversation_id".to_string(), serde_json::json!(conv_id));
            }
            if let Some(msg_id) = &request.message_id {
                map.insert("message_id".to_string(), serde_json::json!(msg_id));
            }
            map
        },
        user_id: "default".to_string(), // TODO: 从请求中获取
        priority: TaskPriority::Normal,
        timeout: Some(max_iterations as u64 * 30000), // 30s per iteration
    };

    // 执行任务（这里需要一个 session，先用空实现）
    use crate::agents::traits::{AgentSession, AgentSessionStatus, LogLevel};
    
    struct DummySession {
        task: AgentTask,
        logs: Vec<crate::agents::traits::SessionLog>,
        result: Option<AgentExecutionResult>,
    }
    
    #[async_trait::async_trait]
    impl AgentSession for DummySession {
        fn get_session_id(&self) -> &str {
            "dummy"
        }
        fn get_task(&self) -> &AgentTask {
            &self.task
        }
        fn get_status(&self) -> AgentSessionStatus {
            AgentSessionStatus::Executing
        }
        async fn update_status(&mut self, _status: AgentSessionStatus) -> Result<(), anyhow::Error> {
            Ok(())
        }
        async fn add_log(&mut self, _level: LogLevel, _message: String) -> Result<(), anyhow::Error> {
            Ok(())
        }
        fn get_logs(&self) -> &[crate::agents::traits::SessionLog] {
            &self.logs
        }
        async fn set_result(&mut self, result: AgentExecutionResult) -> Result<(), anyhow::Error> {
            self.result = Some(result);
            Ok(())
        }
        fn get_result(&self) -> Option<&AgentExecutionResult> {
            self.result.as_ref()
        }
    }
    
    let mut session = DummySession {
        task: task.clone(),
        logs: Vec::new(),
        result: None,
    };

    match engine.execute(&task, &mut session).await {
        Ok(result) => {
            // 从 result.data 中提取 metadata
            let metadata = result.data.clone().unwrap_or_else(|| serde_json::json!({}));
            
            let response = ExecuteReactResponse {
                trace_id: metadata.get("trace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                status: if result.success { "completed" } else { "failed" }.to_string(),
                answer: result.data
                    .as_ref()
                    .and_then(|d| d.get("output"))
                    .and_then(|o| o.as_str())
                    .map(|s| s.to_string()),
                iterations: metadata.get("iterations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                tool_calls: metadata.get("tool_calls")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                duration_ms: result.execution_time_ms,
                metadata,
            };

            info!("ReAct 任务完成: trace_id={}", response.trace_id);
            Ok(AiCommandResponse::success(response))
        }
        Err(e) => {
            error!("ReAct 任务执行失败: {}", e);
            Ok(AiCommandResponse::error(format!("执行失败: {}", e)))
        }
    }
}

/// 获取 ReAct 配置
#[tauri::command]
pub async fn get_react_config() -> Result<AiCommandResponse<ReactConfig>, String> {
    Ok(AiCommandResponse::success(ReactConfig::default()))
}

/// 更新 ReAct 配置
#[tauri::command]
pub async fn update_react_config(
    config: ReactConfig,
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<AiCommandResponse<bool>, String> {
    // TODO: 保存配置到数据库
    info!("更新 ReAct 配置: max_iterations={}", config.max_iterations);
    Ok(AiCommandResponse::success(true))
}
