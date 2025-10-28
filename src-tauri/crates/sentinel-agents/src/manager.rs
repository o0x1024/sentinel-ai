//! Agent管理器 - 替代原有的intelligent_dispatcher

use super::traits::*;
use super::session::DefaultAgentSession;
// 引擎适配器将在注册时直接使用
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use tracing::debug;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn, error};
use crate::services::database::{Database, DatabaseService};

/// Agent管理器 - 负责Agent的注册、调度和管理
pub struct AgentManager {
    /// 注册的Agent列表
    agents: Arc<RwLock<HashMap<String, Arc<dyn Agent>>>>,
    /// 执行引擎列表
    engines: Arc<RwLock<HashMap<String, Arc<dyn ExecutionEngine>>>>,
    /// 活跃的会话
    pub active_sessions: Arc<RwLock<HashMap<String, Box<dyn AgentSession>>>>,
    /// 已完成的会话（用于工作流监控）
    pub completed_sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    /// 管理器统计
    statistics: Arc<RwLock<ManagerStatistics>>,
    /// 数据库服务
    database: Arc<DatabaseService>,
}

/// 管理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ManagerStatistics {
    /// 总任务数
    pub total_tasks: u64,
    /// 成功任务数
    pub successful_tasks: u64,
    /// 失败任务数
    pub failed_tasks: u64,
    /// 活跃会话数
    pub active_sessions: u32,
    /// 平均执行时间(毫秒)
    pub average_execution_time_ms: f64,
}

/// Agent选择策略
#[derive(Debug, Clone)]
pub enum AgentSelectionStrategy {
    /// 自动选择最佳Agent
    Auto,
    /// 指定Agent名称
    Specific(String),
    /// 基于能力选择
    ByCapability(Vec<Capability>),
    /// 性能优先
    PerformanceOptimized,
    /// 资源优先
    ResourceOptimized,
}

/// 多Agent执行请求
#[derive(Debug, Clone)]
pub struct MultiAgentRequest {
    /// 用户输入
    pub user_input: String,
    /// 目标信息
    pub target: Option<String>,
    /// 执行上下文
    pub context: HashMap<String, serde_json::Value>,
    /// Agent选择策略
    pub selection_strategy: AgentSelectionStrategy,
    /// 优先级
    pub priority: TaskPriority,
    /// 用户ID
    pub user_id: String,
}

impl AgentManager {
    /// 创建新的Agent管理器
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            engines: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            completed_sessions: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(ManagerStatistics::default())),
            database,
        }
    }
    
    /// 初始化管理器，注册默认的Agent和Engine
    pub async fn initialize(&self) -> Result<()> {
        debug!("Initializing Agent Manager");
        
        // 注册执行引擎
        self.register_default_engines().await?;
        
        // 注册默认Agent
        self.register_default_agents().await?;
        
        debug!("Agent Manager initialized successfully");
        Ok(())
    }
    
    /// 使用依赖服务初始化管理器
    pub async fn initialize_with_dependencies(
        &self,
        ai_service_manager: Arc<crate::services::AiServiceManager>,
        db_service: Arc<crate::services::database::DatabaseService>,
    ) -> Result<()> {
        debug!("Initializing Agent Manager with dependencies");
        
        // 注册具有完整依赖的执行引擎
        self.register_engines_with_dependencies(ai_service_manager, db_service).await?;
        
        // 注册默认Agent
        self.register_default_agents().await?;
        
        debug!("Agent Manager initialized successfully with full dependencies");
        Ok(())
    }
    
    /// 注册默认的执行引擎
    async fn register_default_engines(&self) -> Result<()> {
        let mut engines = self.engines.write().await;
        
        // 注册引擎适配器（使用统一的ExecutionEngine trait）
        // 这些适配器封装了原始引擎的复杂性，提供统一接口
        
        // 注册Plan-and-Execute引擎适配器
        match crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine::new().await {
            Ok(engine) => {
                engines.insert("plan_execute".to_string(), Arc::new(engine));
                debug!("Registered Plan-and-Execute engine adapter");
            }
            Err(e) => {
                warn!("Failed to register Plan-and-Execute engine: {}", e);
            }
        }
        
        // DISABLED: ReWOO engine registration (needs Rig refactor)
        warn!("ReWOO engine disabled - needs Rig refactor");
        
        // 注册LLMCompiler引擎适配器  
        match crate::engines::llm_compiler::engine_adapter::LlmCompilerEngine::new().await {
            Ok(engine) => {
                engines.insert("llm_compiler".to_string(), Arc::new(engine));
                info!("Registered LLMCompiler engine adapter");
            }
            Err(e) => {
                warn!("Failed to register LLMCompiler engine: {}", e);
            }
        }
        
        info!("Registered {} execution engines", engines.len());
        Ok(())
    }
    
    /// 注册具有完整依赖的执行引擎
    async fn register_engines_with_dependencies(
        &self,
        ai_service_manager: Arc<crate::services::AiServiceManager>,
        db_service: Arc<crate::services::database::DatabaseService>,
    ) -> Result<()> {
        let mut engines = self.engines.write().await;
        
        // 注册Plan-and-Execute引擎适配器（带完整依赖）
        let plan_execute_config = crate::engines::plan_and_execute::types::PlanAndExecuteConfig::default();
        match crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine::new_with_dependencies(
            ai_service_manager.clone(),
            plan_execute_config,
            db_service.clone(),
            None, // 在AgentManager上下文中，没有直接的AppHandle
        ).await {
            Ok(engine) => {
                engines.insert("plan_execute".to_string(), Arc::new(engine));
                debug!("Registered Plan-and-Execute engine adapter with full dependencies");
            }
            Err(e) => {
                warn!("Failed to register Plan-and-Execute engine with dependencies: {}", e);
            }
        }
        
        // 注册ReWOO引擎适配器（带完整依赖）
        // ReWOO需要AI Provider和配置
        let rewoo_config = crate::engines::rewoo::rewoo_types::ReWOOConfig::default();
        
        match crate::engines::rewoo::engine_adapter::ReWooEngine::new_with_dependencies(
            ai_service_manager.clone(),
            rewoo_config,
            db_service.clone(),
        ).await {
            Ok(engine) => {
                // DISABLED: ReWOO engine registration removed
                info!("Registered ReWOO engine adapter with full dependencies");
            }
            Err(e) => {
                warn!("Failed to register ReWOO engine with dependencies: {}", e);
                
            }
        }
        
        // 注册LLMCompiler引擎适配器（带完整依赖）
        let llm_config = crate::engines::llm_compiler::types::LlmCompilerConfig::default();
        
        match crate::engines::llm_compiler::engine_adapter::LlmCompilerEngine::new_with_dependencies(
            ai_service_manager.clone(),
            llm_config,
            db_service.clone(),
        ).await {
            Ok(engine) => {
                engines.insert("llm_compiler".to_string(), Arc::new(engine));
                info!("Registered LLMCompiler engine adapter with full dependencies");
            }
            Err(e) => {
                warn!("Failed to register LLMCompiler engine with dependencies: {}", e);
                
            }
        }
        
        info!("Registered {} execution engines with dependencies", engines.len());
        Ok(())
    }
    
    /// 注册默认的Agent
    async fn register_default_agents(&self) -> Result<()> {
        let engines = self.engines.read().await;
        let mut agents = self.agents.write().await;
        
        // 为每个引擎创建对应的Agent
        for (engine_name, engine) in engines.iter() {
            let agent = Arc::new(DefaultAgent::new(
                format!("{}_agent", engine_name),
                format!("{} Agent", engine_name),
                engine.clone(),
            ));
            agents.insert(agent.get_name().to_string(), agent);
        }
        
        info!("Registered {} agents", agents.len());
        Ok(())
    }
    
    /// 注册自定义Agent
    pub async fn register_agent(&self, agent: Arc<dyn Agent>) -> Result<()> {
        let mut agents = self.agents.write().await;
        let name = agent.get_name().to_string();
        agents.insert(name.clone(), agent);
        info!("Registered custom agent: {}", name);
        Ok(())
    }
    
    /// 获取所有已注册的Agent
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
    
    /// 获取所有已注册的执行引擎
    pub async fn list_engines(&self) -> Vec<String> {
        let engines = self.engines.read().await;
        engines.keys().cloned().collect()
    }
    
    /// 分发多Agent任务
    pub async fn dispatch_task(&self, request: MultiAgentRequest) -> Result<String> {
        info!("Dispatching task: {}", request.user_input);
        
        // 创建任务
        let task = AgentTask::new(request.user_input.clone(), request.user_id.clone())
            .with_priority(request.priority)
            .with_parameter("context".to_string(), serde_json::to_value(&request.context)?);
        
        let task = if let Some(target) = request.target {
            task.with_target(target)
        } else {
            task
        };
        
        // 保存任务到数据库
        if let Err(e) = self.database.create_agent_task(&task).await {
            warn!("Failed to save task to database: {}", e);
        }
        
        // 选择合适的Agent
        let agent = self.select_agent(&task, &request.selection_strategy).await?;
        
        // 创建会话
        let session = agent.create_session(task.clone()).await?;
        let session_id = session.get_session_id().to_string();
        
        // 创建Agent会话记录
        if let Err(e) = self.database.create_agent_session(&session_id, &task.id, &agent.get_name()).await {
            warn!("Failed to create agent session in database: {}", e);
        }
        
        // 添加到活跃会话
        {
            let mut active_sessions = self.active_sessions.write().await;
            active_sessions.insert(session_id.clone(), session);
        }
        
        // 执行任务
        tokio::spawn({
            let agent = agent.clone();
            let manager = self.clone();
            let session_id_clone = session_id.clone();
            let task_clone = task.clone();
            
            async move {
                // 更新任务状态为执行中
                if let Err(e) = manager.database.update_agent_task_status(&task_clone.id, "Running", Some(&agent.get_name()), None).await {
                    warn!("Failed to update task status to Running: {}", e);
                }
                if let Err(e) = manager.database.update_agent_task_timing(&task_clone.id, Some(chrono::Utc::now()), None, None).await {
                    warn!("Failed to update task start time: {}", e);
                }
                if let Err(e) = manager.database.update_agent_session_status(&session_id_clone, "Running").await {
                    warn!("Failed to update session status to Running: {}", e);
                }
                
                let start_time = std::time::Instant::now();
                
                // 从活跃会话中获取session的可变引用
                let result = {
                    let mut active_sessions = manager.active_sessions.write().await;
                    if let Some(session) = active_sessions.get_mut(&session_id_clone) {
                        agent.execute(session.as_mut()).await
                    } else {
                        Err(anyhow!("Session not found: {}", session_id_clone))
                    }
                };
                
                let execution_time_ms = start_time.elapsed().as_millis() as f64;
                
                // 更新任务完成状态
                let status = if result.is_ok() { "Completed" } else { "Failed" };
                if let Err(e) = manager.database.update_agent_task_status(&task_clone.id, status, None, None).await {
                    warn!("Failed to update task status to {}: {}", status, e);
                }
                if let Err(e) = manager.database.update_agent_task_timing(&task_clone.id, None, Some(chrono::Utc::now()), Some(execution_time_ms as u64)).await {
                    warn!("Failed to update task completion time: {}", e);
                }
                if let Err(e) = manager.database.update_agent_session_status(&session_id_clone, status).await {
                    warn!("Failed to update session status to {}: {}", status, e);
                }
                
                // 如果执行失败，保存错误信息
                if let Err(ref error) = result {
                    let error_message = format!("{:?}", error);
                    if let Err(e) = manager.database.update_agent_task_error(&task_clone.id, &error_message).await {
                        warn!("Failed to save task error: {}", e);
                    }
                }
                
                // 保存执行结果到数据库
                if result.is_ok() {
                    // 从会话中获取执行结果
                    let session_result_opt = {
                        let active_sessions = manager.active_sessions.read().await;
                        if let Some(session) = active_sessions.get(&session_id_clone) {
                            session.get_result().cloned()
                        } else {
                            None
                        }
                    };
                    
                    if let Some(session_result) = session_result_opt {
                        if let Err(e) = manager.database.save_agent_execution_result(&session_id_clone, &session_result).await {
                            warn!("Failed to save execution result: {}", e);
                        }
                    }
                }
                
                // 更新统计信息
                manager.update_statistics(&result).await;
                
                // 将完成的会话移到已完成列表中
                let session_info = {
                    let mut active_sessions = manager.active_sessions.write().await;
                    if let Some(session) = active_sessions.remove(&session_id_clone) {
                        Some(SessionInfo {
                            task: session.get_task().clone(),
                            status: session.get_status(),
                            created_at: chrono::Utc::now(), // TODO: 实际应该从session中获取
                            completed_at: Some(chrono::Utc::now()),
                            error: if result.is_err() { 
                                Some(format!("{:?}", result.as_ref().unwrap_err()))
                            } else { 
                                None 
                            },
                            result: session.get_result().and_then(|r| r.data.clone()),
                        })
                    } else {
                        None
                    }
                };
                
                // 保存到已完成会话中
                if let Some(info) = session_info {
                    let mut completed_sessions = manager.completed_sessions.write().await;
                    completed_sessions.insert(session_id_clone.clone(), info);
                    
                    // 限制已完成会话的数量，避免内存泄漏
                    if completed_sessions.len() > 100 {
                        // 移除最旧的会话（这里简化处理，实际应该按时间排序）
                        let keys: Vec<String> = completed_sessions.keys().cloned().collect();
                        if let Some(oldest_key) = keys.first() {
                            completed_sessions.remove(oldest_key);
                        }
                    }
                }
                
                if let Err(e) = result {
                    error!("Task execution failed: {}", e);
                } else {
                    info!("Task completed successfully: {}", session_id_clone);
                }
            }
        });
        
        // 更新统计
        let mut stats = self.statistics.write().await;
        stats.total_tasks += 1;
        let active_sessions = self.active_sessions.read().await;
        stats.active_sessions = active_sessions.len() as u32;
        
        Ok(session_id)
    }
    
    /// 选择合适的Agent
    async fn select_agent(
        &self, 
        task: &AgentTask, 
        strategy: &AgentSelectionStrategy
    ) -> Result<Arc<dyn Agent>> {
        let agents = self.agents.read().await;
        
        match strategy {
            AgentSelectionStrategy::Specific(name) => {
                agents.get(name)
                    .cloned()
                    .ok_or_else(|| anyhow!("Agent not found: {}", name))
            },
            
            AgentSelectionStrategy::Auto => {
                // 自动选择第一个可以处理任务的Agent
                for agent in agents.values() {
                    if agent.can_handle_task(task) {
                        return Ok(agent.clone());
                    }
                }
                Err(anyhow!("No suitable agent found for task"))
            },
            
            AgentSelectionStrategy::ByCapability(capabilities) => {
                // 根据能力选择Agent
                for agent in agents.values() {
                    let agent_caps = agent.get_capabilities();
                    if capabilities.iter().all(|cap| agent_caps.contains(cap)) {
                        return Ok(agent.clone());
                    }
                }
                Err(anyhow!("No agent found with required capabilities"))
            },
            
            _ => {
                // 默认选择第一个可用的Agent
                agents.values().next()
                    .cloned()
                    .ok_or_else(|| anyhow!("No agents available"))
            }
        }
    }
    
    /// 更新统计信息
    async fn update_statistics(&self, result: &Result<()>) {
        let mut stats = self.statistics.write().await;
        
        match result {
            Ok(_) => stats.successful_tasks += 1,
            Err(_) => stats.failed_tasks += 1,
        }
    }
    
    /// 获取统计信息
    pub async fn get_statistics(&self) -> ManagerStatistics {
        self.statistics.read().await.clone()
    }
    
    /// 获取会话状态
    pub async fn get_session_status(&self, session_id: &str) -> Option<AgentSessionStatus> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).map(|s| s.get_status())
    }
    
    /// 取消任务
    pub async fn cancel_task(&self, session_id: &str) -> Result<()> {
        info!("Attempting to cancel task: {}", session_id);
        
        // 首先检查活跃会话中是否存在该任务
        let mut session_found = false;
        let mut session_cancelled = false;
        
        {
            let mut active_sessions = self.active_sessions.write().await;
            if let Some(session) = active_sessions.get_mut(session_id) {
                session_found = true;
                
                // 更新会话状态为已取消
                if let Ok(_) = session.update_status(AgentSessionStatus::Cancelled).await {
                    session.add_log(LogLevel::Info, "Task cancellation requested".to_string()).await?;
                    session_cancelled = true;
                    info!("Session {} status updated to Cancelled", session_id);
                }
            }
        }
        
        if !session_found {
            warn!("Session not found in active sessions: {}", session_id);
            return Err(anyhow!("Session not found: {}", session_id));
        }
        
        // 通知所有Agent取消该会话相关的执行
        let agents = self.agents.read().await;
        let mut cancel_success = false;
        
        for agent in agents.values() {
            // 尝试取消Agent中的执行
            if let Ok(_) = agent.cancel(session_id).await {
                info!("Agent {} successfully cancelled session: {}", agent.get_name(), session_id);
                cancel_success = true;
            }
        }
        
        // 如果会话状态更新成功或Agent取消成功，则将会话移动到已完成列表
        if session_cancelled || cancel_success {
            let session_info = {
                let mut active_sessions = self.active_sessions.write().await;
                if let Some(session) = active_sessions.remove(session_id) {
                    Some(SessionInfo {
                        task: session.get_task().clone(),
                        status: AgentSessionStatus::Cancelled,
                        created_at: chrono::Utc::now(), // TODO: 应该从session中获取实际创建时间
                        completed_at: Some(chrono::Utc::now()),
                        error: Some("Task was cancelled by user".to_string()),
                        result: None,
                    })
                } else {
                    None
                }
            };
            
            // 将取消的会话保存到已完成会话列表
            if let Some(info) = session_info {
                let mut completed_sessions = self.completed_sessions.write().await;
                completed_sessions.insert(session_id.to_string(), info);
                
                // 限制已完成会话的数量
                if completed_sessions.len() > 100 {
                    let keys: Vec<String> = completed_sessions.keys().cloned().collect();
                    if let Some(oldest_key) = keys.first() {
                        completed_sessions.remove(oldest_key);
                    }
                }
            }
            
            // 更新统计信息
            {
                let mut stats = self.statistics.write().await;
                stats.failed_tasks += 1; // 取消的任务计为失败
                let active_sessions = self.active_sessions.read().await;
                stats.active_sessions = active_sessions.len() as u32;
            }
            
            info!("Task successfully cancelled: {}", session_id);
            Ok(())
        } else {
            error!("Failed to cancel task: {}", session_id);
            Err(anyhow!("Failed to cancel task: {}", session_id))
        }
    }
    
    /// 获取所有会话信息（用于工作流监控）
    pub async fn get_all_sessions(&self) -> HashMap<String, SessionInfo> {
        let mut session_infos = HashMap::new();
        
        // 获取活跃会话
        {
            let active_sessions = self.active_sessions.read().await;
            for (session_id, session) in active_sessions.iter() {
                let session_info = SessionInfo {
                    task: session.get_task().clone(),
                    status: session.get_status(),
                    created_at: chrono::Utc::now(), // TODO: 实际应该从session中获取
                    completed_at: None,
                    error: None,
                    result: session.get_result().and_then(|r| r.data.clone()),
                };
                
                session_infos.insert(session_id.clone(), session_info);
            }
        }
        
        // 获取已完成会话
        {
            let completed_sessions = self.completed_sessions.read().await;
            for (session_id, session_info) in completed_sessions.iter() {
                session_infos.insert(session_id.clone(), session_info.clone());
            }
        }
        
        session_infos
    }
    
    /// 获取数据库服务引用
    pub fn get_database(&self) -> &Arc<DatabaseService> {
        &self.database
    }
    
    /// 保存执行步骤到数据库
    pub async fn save_execution_step(&self, session_id: &str, step: &crate::commands::agent_commands::WorkflowStepDetail) -> Result<()> {
        self.database.save_agent_execution_step(session_id, step).await
            .map_err(|e| anyhow!("Failed to save execution step: {}", e))
    }
    
    /// 获取执行步骤
    pub async fn get_execution_steps(&self, session_id: &str) -> Result<Vec<crate::commands::agent_commands::WorkflowStepDetail>> {
        self.database.get_agent_execution_steps(session_id).await
            .map_err(|e| anyhow!("Failed to get execution steps: {}", e))
    }
    
    /// 更新执行步骤状态
    pub async fn update_execution_step_status(
        &self, 
        step_id: &str, 
        status: &str, 
        started_at: Option<chrono::DateTime<chrono::Utc>>, 
        completed_at: Option<chrono::DateTime<chrono::Utc>>, 
        duration_ms: Option<u64>, 
        error_message: Option<&str>
    ) -> Result<()> {
        self.database.update_agent_execution_step_status(step_id, status, started_at, completed_at, duration_ms, error_message).await
            .map_err(|e| anyhow!("Failed to update execution step status: {}", e))
    }
}

/// 会话信息（用于工作流监控）
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub task: AgentTask,
    pub status: AgentSessionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
}

impl Clone for AgentManager {
    fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            engines: self.engines.clone(),
            active_sessions: self.active_sessions.clone(),
            completed_sessions: self.completed_sessions.clone(),
            statistics: self.statistics.clone(),
            database: self.database.clone(),
        }
    }
}

/// 默认Agent实现 - 包装执行引擎
pub struct DefaultAgent {
    name: String,
    description: String,
    engine: Arc<dyn ExecutionEngine>,
    capabilities: Vec<Capability>,
    statistics: Arc<RwLock<AgentStatistics>>,
    /// 活跃的执行会话（用于取消操作）
    active_executions: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

impl DefaultAgent {
    pub fn new(name: String, description: String, engine: Arc<dyn ExecutionEngine>) -> Self {
        // 根据引擎信息确定 能力
        let capabilities = Self::derive_capabilities_from_engine(&engine);
        
        Self {
            name,                                                                                                              
            engine,
            capabilities,
            statistics: Arc::new(RwLock::new(AgentStatistics {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                average_execution_time_ms: 0.0,
                last_execution: None,
            })),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            description,
        }
    }
    
    fn derive_capabilities_from_engine(engine: &Arc<dyn ExecutionEngine>) -> Vec<Capability> {
        let info = engine.get_engine_info();
        let mut capabilities = vec![Capability::ToolIntegration];
        
        // 根据引擎名称和特征推断能力
        if info.name.contains("plan") {
            capabilities.push(Capability::DataAnalysis);
        }
        
        if info.performance_characteristics.concurrency_capability > 70 {
            capabilities.push(Capability::ParallelProcessing);
        }
        
        if info.supported_scenarios.iter().any(|s| s.contains("security") || s.contains("vulnerability")) {
            capabilities.extend(vec![
                Capability::NetworkScanning,
                Capability::VulnerabilityDetection,
            ]);
        }
        
        capabilities.push(Capability::NaturalLanguageProcessing);
        capabilities
    }
}

#[async_trait]
impl Agent for DefaultAgent {
    fn get_name(&self) -> &str {
        &self.name
    }
    
    fn get_description(&self) -> &str {
        &self.description
    }
    
    fn get_capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
    
    fn can_handle_task(&self, task: &AgentTask) -> bool {
        self.engine.supports_task(task)
    }
    
    async fn create_session(&'_ self, task: AgentTask) -> Result<Box<dyn AgentSession>> {
        let session = DefaultAgentSession::new(task);
        Ok(Box::new(session))
    }
    
    async fn execute(&'_ self, session: &mut dyn AgentSession) -> Result<()> {
        let start_time = std::time::Instant::now();
        let session_id = session.get_session_id().to_string();
        
        // 创建取消令牌
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        
        // 将取消令牌存储到活跃执行中
        {
            let mut active_executions = self.active_executions.write().await;
            active_executions.insert(session_id.clone(), cancel_tx);
        }
        
        // 使用 select! 来监听取消信号和正常执行
        let execution_result = tokio::select! {
            // 正常执行路径
            result = self.execute_with_cancellation(session) => {
                result
            }
            
            // 取消路径
            _ = cancel_rx => {
                session.add_log(LogLevel::Warn, "Execution was cancelled".to_string()).await?;
                session.update_status(AgentSessionStatus::Cancelled).await?;
                Err(anyhow!("Execution was cancelled"))
            }
        };
        
        // 清理活跃执行记录
        {
            let mut active_executions = self.active_executions.write().await;
            active_executions.remove(&session_id);
        }
        
        // 更新统计信息
        let execution_time = start_time.elapsed().as_millis() as f64;
        let success = execution_result.is_ok();
        self.update_statistics(success, execution_time).await;
        
        if success {
            session.add_log(LogLevel::Info, format!("Task completed in {}ms", execution_time)).await?;
        } else {
            session.add_log(LogLevel::Error, format!("Task failed after {}ms", execution_time)).await?;
        }
        
        execution_result
    }
    
    async fn cancel(&'_ self, session_id: &str) -> Result<()> {
        info!("Attempting to cancel execution for session: {}", session_id);
        
        // 检查是否有活跃的执行
        let mut active_executions = self.active_executions.write().await;
        
        if let Some(cancel_sender) = active_executions.remove(session_id) {
            // 发送取消信号
            if cancel_sender.send(()).is_ok() {
                info!("Cancellation signal sent successfully for session: {}", session_id);
            } else {
                warn!("Failed to send cancellation signal for session: {} (receiver might be dropped)", session_id);
            }
            
            // 尝试通过执行引擎取消
            if let Err(e) = self.engine.cancel_execution(session_id).await {
                warn!("Failed to cancel execution in engine for session {}: {}", session_id, e);
            } else {
                info!("Successfully cancelled execution in engine for session: {}", session_id);
            }
            
            Ok(())
        } else {
            warn!("No active execution found for session: {}", session_id);
            // 即使没有活跃执行，也返回成功，因为任务可能已经完成或之前已被取消
            Ok(())
        }
    }
    
    async fn get_statistics(&'_ self) -> Result<AgentStatistics> {
        Ok(self.statistics.read().await.clone())
    }
}

impl DefaultAgent {
    /// 可取消的执行逻辑
    async fn execute_with_cancellation(&self, session: &mut dyn AgentSession) -> Result<()> {
        // 更新状态为规划中
        session.update_status(AgentSessionStatus::Planning).await?;
        session.add_log(LogLevel::Info, "Starting task execution".to_string()).await?;
        
        // 创建执行计划
        let plan = self.engine.create_plan(session.get_task()).await?;
        session.add_log(LogLevel::Info, format!("Created execution plan with {} steps", plan.steps.len())).await?;
        
        // 更新状态为执行中
        session.update_status(AgentSessionStatus::Executing).await?;
        
        // 执行计划
        let result = self.engine.execute_plan(&plan).await?;
        
        // 设置执行结果
        session.set_result(result).await?;
        
        Ok(())
    }
    
    async fn update_statistics(&self, success: bool, execution_time_ms: f64) {
        let mut stats = self.statistics.write().await;
        
        stats.total_executions += 1;
        if success {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }
        
        // 更新平均执行时间
        stats.average_execution_time_ms = 
            (stats.average_execution_time_ms * (stats.total_executions - 1) as f64 + execution_time_ms as f64) 
            / stats.total_executions as f64;
        
        stats.last_execution = Some(chrono::Utc::now());
    }
}
