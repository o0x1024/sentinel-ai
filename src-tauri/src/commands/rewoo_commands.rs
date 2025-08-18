//! ReWOO 引擎测试命令
//! 
//! 提供前端测试 ReWOO 引擎的 Tauri 命令接口

use crate::engines::rewoo::*;
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tauri::State;
use uuid::Uuid;
use log::{info, error};

/// ReWOO 测试状态管理
pub struct ReWOOTestState {
    pub engine: Option<ReWOOEngine>,
    pub sessions: HashMap<String, ReWOOSession>,
    pub test_results: Vec<TestResult>,
}

impl Default for ReWOOTestState {
    fn default() -> Self {
        Self {
            engine: None,
            sessions: HashMap::new(),
            test_results: Vec::new(),
        }
    }
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

/// 执行日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub component: String, // 'planner' | 'worker' | 'solver' | 'system'
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub test_name: String,
    pub task: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub metrics: ReWOOMetrics,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub success: bool,
    pub logs: Vec<LogEntry>, // 新增：执行日志
}

/// 测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub name: String,
    pub task: String,
    pub expected_tools: Vec<String>,
    pub timeout_seconds: u64,
    pub rewoo_config: ReWOOConfig,
}

/// 引擎状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatusInfo {
    pub active: bool,
    pub ready: bool,
    pub version: String,
    pub uptime_seconds: u64,
    pub total_sessions: usize,
    pub active_sessions: usize,
}

/// 初始化 ReWOO 引擎
#[tauri::command]
pub async fn init_rewoo_engine(
    _config: ReWOOConfig,
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
) -> Result<String, String> {
    // 注意：这里需要实际的 AI Provider 和 Tool Manager 实现
    // 目前返回模拟结果
    let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    // 模拟引擎初始化
    test_state.engine = None; // 实际应该创建 ReWOOEngine 实例
    
    Ok("ReWOO 引擎初始化成功".to_string())
}

/// 获取引擎状态
#[tauri::command]
pub async fn get_rewoo_engine_status(
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
) -> Result<EngineStatusInfo, String> {
    let test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    let status = EngineStatusInfo {
        active: test_state.engine.is_some(),
        ready: test_state.engine.is_some(),
        version: "1.0.0".to_string(),
        uptime_seconds: 0, // 实际应该计算运行时间
        total_sessions: test_state.sessions.len(),
        active_sessions: test_state.sessions.values()
            .filter(|s| !s.is_completed)
            .count(),
    };
    
    Ok(status)
}

/// 执行 ReWOO 测试
#[tauri::command]
pub async fn execute_rewoo_test(
    test_config: TestConfig,
    _app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let test_id = Uuid::new_v4().to_string();
    let started_at = SystemTime::now();
    
    info!("开始执行 ReWOO 测试: {} (ID: {})", test_config.name, test_id);
    
    // 创建测试结果记录
    let mut test_result = TestResult {
        id: test_id.clone(),
        test_name: test_config.name.clone(),
        task: test_config.task.clone(),
        result: None,
        error: None,
        metrics: ReWOOMetrics::default(),
        started_at,
        completed_at: None,
        success: false,
        logs: Vec::new(),
    };
    
    // 添加初始化日志
    test_result.logs.push(LogEntry {
        timestamp: SystemTime::now(),
        level: LogLevel::INFO,
        component: "system".to_string(),
        message: format!("开始执行测试: {}", test_config.name),
        details: Some(serde_json::json!({
            "task": test_config.task,
            "expected_tools": test_config.expected_tools,
            "timeout_seconds": test_config.timeout_seconds
        })),
    });
    
    // 真实 ReWOO 引擎执行
    test_result.logs.push(LogEntry {
        timestamp: SystemTime::now(),
        level: LogLevel::INFO,
        component: "system".to_string(),
        message: "开始创建ReWOO引擎实例".to_string(),
        details: Some(serde_json::json!({
            "config": test_config.rewoo_config
        })),
    });
    
    // 获取 AI Provider
    let ai_provider = {
        // 使用全局的 AiAdapterManager 获取 Provider
        let adapter_manager = crate::ai_adapter::core::AiAdapterManager::global();
        let providers = match adapter_manager.list_providers() {
            Ok(providers) => providers,
            Err(e) => {
                let error_msg = format!("获取AI提供商列表失败: {}", e);
                test_result.logs.push(LogEntry {
                    timestamp: SystemTime::now(),
                    level: LogLevel::ERROR,
                    component: "system".to_string(),
                    message: error_msg.clone(),
                    details: None,
                });
                test_result.error = Some(error_msg);
                test_result.completed_at = Some(SystemTime::now());
                test_result.success = false;
                test_result.metrics.total_time_ms = started_at.elapsed().unwrap_or_default().as_millis() as u64;
                
                let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
                test_state.test_results.push(test_result);
                return Ok(test_id);
            }
        };
        
        if providers.is_empty() {
            let error_msg = "没有可用的AI服务";
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::ERROR,
                component: "system".to_string(),
                message: error_msg.to_string(),
                details: Some(serde_json::json!({
                    "suggestion": "请确保已配置AI服务，可以通过环境变量或数据库配置"
                })),
            });
            test_result.error = Some(error_msg.to_string());
            test_result.completed_at = Some(SystemTime::now());
            test_result.success = false;
            test_result.metrics.total_time_ms = started_at.elapsed().unwrap_or_default().as_millis() as u64;
            
            let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
            test_state.test_results.push(test_result);
            return Ok(test_id);
        }
        
        let provider_name = &providers[0];
        tracing::info!("使用AI Provider: {:?}", providers);
        info!("使用AI Provider: {}", provider_name);
        
        test_result.logs.push(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::INFO,
            component: "system".to_string(),
            message: format!("选择AI Provider: {}", provider_name),
            details: Some(serde_json::json!({
                "available_providers": providers
            })),
        });
        
        // 直接从 AiAdapterManager 获取 Provider
        adapter_manager.get_provider(provider_name)
            .map_err(|e| format!("无法获取AI Provider '{}': {}", provider_name, e))?
    };
    tracing::info!("使用AI Provider: {:?}", ai_provider);
    test_result.logs.push(LogEntry {
        timestamp: SystemTime::now(),
        level: LogLevel::INFO,
        component: "system".to_string(),
        message: "成功获取AI Provider".to_string(),
        details: None,
    });

    
    // 创建ReWOO工具管理器
    let _tool_adapter = match crate::tools::get_global_engine_adapter() {
        Ok(adapter) => {
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::INFO,
                component: "system".to_string(),
                message: "成功创建ReWOO工具管理器".to_string(),
                details: None,
            });
            adapter
        }
        Err(e) => {
            let error_msg = format!("创建ReWOO工具管理器失败: {}", e);
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::ERROR,
                component: "system".to_string(),
                message: error_msg.clone(),
                details: None,
            });
            test_result.error = Some(error_msg);
            test_result.completed_at = Some(SystemTime::now());
            test_result.success = false;
            test_result.metrics.total_time_ms = started_at.elapsed().unwrap_or_default().as_millis() as u64;
            
            let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
            test_state.test_results.push(test_result);
            return Ok(test_id);
        }
    };
    
    // 创建ReWOO引擎
    let mut rewoo_engine = match crate::engines::rewoo::ReWOOEngine::new(
        ai_provider,
        test_config.rewoo_config.clone(),
        db.inner().clone(),
    ).await {
        Ok(engine) => {
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::INFO,
                component: "system".to_string(),
                message: "ReWOO引擎创建成功".to_string(),
                details: None,
            });
            engine
        }
        Err(e) => {
            let error_msg = format!("创建ReWOO引擎失败: {}", e);
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::ERROR,
                component: "system".to_string(),
                message: error_msg.clone(),
                details: None,
            });
            test_result.error = Some(error_msg);
            test_result.completed_at = Some(SystemTime::now());
            test_result.success = false;
            test_result.metrics.total_time_ms = started_at.elapsed().unwrap_or_default().as_millis() as u64;
            
            let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
            test_state.test_results.push(test_result);
            return Ok(test_id);
        }
    };
    
    // 执行ReWOO任务
    test_result.logs.push(LogEntry {
        timestamp: SystemTime::now(),
        level: LogLevel::INFO,
        component: "system".to_string(),
        message: "开始执行ReWOO任务".to_string(),
        details: Some(serde_json::json!({
            "task": test_config.task
        })),
    });
    
    let execution_result = rewoo_engine.execute(&test_config.task).await;
    
    match execution_result {
        Ok(result) => {
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::INFO,
                component: "system".to_string(),
                message: "ReWOO任务执行成功".to_string(),
                details: Some(serde_json::json!({
                    "result_length": result.len()
                })),
            });
            
            test_result.result = Some(result);
            test_result.success = true;
        }
        Err(e) => {
            let error_msg = format!("ReWOO任务执行失败: {}", e);
            test_result.logs.push(LogEntry {
                timestamp: SystemTime::now(),
                level: LogLevel::ERROR,
                component: "system".to_string(),
                message: error_msg.clone(),
                details: None,
            });
            
            test_result.error = Some(error_msg);
            test_result.success = false;
        }
    }
    
    // 完成测试
    test_result.completed_at = Some(SystemTime::now());
    test_result.metrics.total_time_ms = started_at.elapsed().unwrap_or_default().as_millis() as u64;
    
    // 从引擎获取实际的执行指标（如果可用）
    if let Some(session) = rewoo_engine.get_all_sessions().last() {
        test_result.metrics.tool_calls = session.metrics.tool_calls;
        test_result.metrics.successful_tool_calls = session.metrics.successful_tool_calls;
        test_result.metrics.total_tokens = session.metrics.total_tokens;
    }
    
    test_result.logs.push(LogEntry {
        timestamp: SystemTime::now(),
        level: LogLevel::INFO,
        component: "system".to_string(),
        message: "ReWOO 测试执行完成".to_string(),
        details: Some(serde_json::json!({
            "success": test_result.success,
            "total_time_ms": test_result.metrics.total_time_ms,
            "tool_calls": test_result.metrics.tool_calls,
            "total_tokens": test_result.metrics.total_tokens
        })),
    });
    
    info!("ReWOO 测试执行完成: {} (ID: {}), 耗时: {}ms", 
          test_config.name, test_id, test_result.metrics.total_time_ms);
    
    // 保存测试结果
    {
        let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
        test_state.test_results.push(test_result);
    }
    
    Ok(test_id)
}

/// 获取测试结果
#[tauri::command]
pub async fn get_test_result(
    test_id: String,
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
) -> Result<TestResult, String> {
    let test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    test_state.test_results
        .iter()
        .find(|r| r.id == test_id)
        .cloned()
        .ok_or_else(|| format!("未找到测试结果: {}", test_id))
}

/// 获取所有测试结果
#[tauri::command]
pub async fn get_all_test_results(
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
) -> Result<Vec<TestResult>, String> {
    let test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    Ok(test_state.test_results.clone())
}

/// 清除测试结果
#[tauri::command]
pub async fn clear_test_results(
    state: State<'_, Arc<Mutex<ReWOOTestState>>>,
) -> Result<(), String> {
    let mut test_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    test_state.test_results.clear();
    Ok(())
}

/// 获取预定义测试配置
#[tauri::command]
pub async fn get_predefined_test_configs() -> Result<Vec<TestConfig>, String> {
    let configs = vec![
        TestConfig {
            name: "子域名扫描测试".to_string(),
            task: "请扫描一下mgtv.com有哪些子域名".to_string(),
            expected_tools: vec!["rsubdomain".to_string()],
            timeout_seconds: 120,
            rewoo_config: ReWOOConfig::default(),
        },
        TestConfig {
            name: "端口扫描测试".to_string(),
            task: "请扫描一下本机开放了哪些端口".to_string(),
            expected_tools: vec!["port_scan".to_string()],
            timeout_seconds: 90,
            rewoo_config: ReWOOConfig::default(),
        },
    ];
    
    Ok(configs)
}

/// 验证 ReWOO 配置
#[tauri::command]
pub async fn validate_rewoo_config(
    config: ReWOOConfig,
) -> Result<bool, String> {
    // 基本配置验证
    if config.planner.max_steps == 0 {
        return Err("Planner max_steps 必须大于 0".to_string());
    }
    
    if config.planner.max_tokens == 0 {
        return Err("Planner max_tokens 必须大于 0".to_string());
    }
    
    if config.worker.timeout_seconds == 0 {
        return Err("Worker timeout_seconds 必须大于 0".to_string());
    }
    
    if config.solver.max_tokens == 0 {
        return Err("Solver max_tokens 必须大于 0".to_string());
    }
    
    Ok(true)
}

/// 获取可用工具列表
#[tauri::command]
pub async fn get_available_tools() -> Result<Vec<String>, String> {
    use crate::tools::get_global_tool_system;
    
    match get_global_tool_system() {
        Ok(tool_system) => {
            let tools = tool_system.list_tools().await;
            let tool_names: Vec<String> = tools
                .into_iter()
                .filter(|tool| tool.available) // 只返回可用的工具
                .map(|tool| tool.name)
                .collect();
            
            info!("获取到{}个可用工具: {:?}", tool_names.len(), tool_names);
            Ok(tool_names)
        }
        Err(e) => {
            error!("获取可用工具失败: {}", e);
            Ok(vec![])
        }
    }
}

/// 模拟工具执行
#[tauri::command]
pub async fn simulate_tool_execution(
    tool_name: String,
    args: String,
) -> Result<String, String> {
    // 模拟工具执行延迟
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    let result = match tool_name.as_str() {
        "search" => format!("搜索结果: 关于 '{}' 的信息", args),
        "calculator" => format!("计算结果: {} = 42", args),
        "weather" => "当前天气: 晴天，温度 25°C".to_string(),
        "summarize" => format!("摘要: {}", args),
        "analyze" => format!("分析结果: {} 的详细分析", args),
        "recommend" => format!("推荐: 基于 {} 的推荐内容", args),
        _ => format!("未知工具 {} 的执行结果", tool_name),
    };
    
    Ok(result)
}