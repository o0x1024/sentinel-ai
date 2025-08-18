//! ReWOO Engine 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的状态图引擎
//! 实现 Planner -> Worker -> Solver 的执行流程

use super::*;
use crate::ai_adapter::AiProvider;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;
use crate::services::prompt_db::PromptRepository;
use crate::services::database::DatabaseService;



/// ReWOO 引擎 - 实现完整的 ReWOO 执行流程
pub struct ReWOOEngine {
    /// Planner 组件
    planner: ReWOOPlanner,
    /// Worker 组件
    worker: ReWOOWorker,
    /// Solver 组件
    solver: ReWOOSolver,
    /// 配置
    config: ReWOOConfig,
    /// 执行会话
    sessions: HashMap<String, ReWOOSession>,
}

impl ReWOOEngine {
    /// 创建新的 ReWOO 引擎  
    pub async fn new(
        ai_provider: Arc<dyn AiProvider>,
        config: ReWOOConfig,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self, ReWOOError> {
        // Prompt 仓库
        let prompt_repo = {
            let pool = db_service.get_pool().map_err(|e| ReWOOError::ConfigurationError(format!("DB pool error: {}", e)))?;
            Some(PromptRepository::new(pool.clone()))
        };
        
        // 获取ReWOO框架适配器
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await
            .map_err(|e| ReWOOError::ToolSystemError(format!("获取ReWOO框架适配器失败: {}", e)))?;
        
        let planner = ReWOOPlanner::new(
            Arc::clone(&ai_provider),
            framework_adapter.clone(),
            config.planner.clone(),
            prompt_repo.clone(),
        )?;
        
        let worker = ReWOOWorker::new(
            framework_adapter,
            config.worker.clone(),
        );
        
        let solver = ReWOOSolver::new(
            ai_provider,
            config.solver.clone(),
            prompt_repo.clone(),
        );
        
        Ok(Self {
            planner,
            worker,
            solver,
            config,
            sessions: HashMap::new(),
        })
    }
    
    /// 使用全局统一工具系统创建新的 ReWOO 引擎
    /// 这个构造函数会自动使用全局工具系统，包含所有已注册的工具（内置工具 + MCP工具）
    pub async fn new_with_global_tools(
        ai_provider: Arc<dyn AiProvider>,
        config: ReWOOConfig,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self, ReWOOError> {
        // 使用现有的构造函数（已经使用全局适配器）
        Self::new(ai_provider, config, db_service).await
    }
    
    /// 执行 ReWOO 流程
    pub async fn execute(&mut self, task: &str) -> Result<String, ReWOOError> {
        let session_id = Uuid::new_v4().to_string();
        let _start_time = SystemTime::now();
        
        // 创建执行会话
        let mut session = ReWOOSession::new(session_id.clone(), task.to_string());
        
        // 执行状态图流程
        let result = self.execute_state_graph(&mut session).await;
        
        // 更新会话结束时间
        if result.is_ok() {
            session.complete();
        } else {
            session.fail(result.as_ref().err().unwrap().to_string());
        }
        
        // 保存会话
        self.sessions.insert(session_id, session);
        
        result
    }
    
    /// 执行状态图流程
    async fn execute_state_graph(&mut self, session: &mut ReWOOSession) -> Result<String, ReWOOError> {
        // 1. Planning 阶段
        let planning_start = SystemTime::now();
        self.planner.plan(&mut session.state).await?;
        session.metrics.planning_time_ms = planning_start.elapsed().unwrap_or_default().as_millis() as u64;
        session.metrics.tool_calls = session.state.steps.len() as u32;
        
        // 2. Working 阶段 - 执行所有步骤
        let working_start = SystemTime::now();
        self.execute_working_phase(session).await?;
        session.metrics.working_time_ms = working_start.elapsed().unwrap_or_default().as_millis() as u64;
        
        // 3. Solving 阶段
        let solving_start = SystemTime::now();
        let answer = self.solver.solve(&session.state).await?;
        session.metrics.solving_time_ms = solving_start.elapsed().unwrap_or_default().as_millis() as u64;
        
        Ok(answer)
    }
    
    /// 执行工作阶段
    async fn execute_working_phase(&mut self, session: &mut ReWOOSession) -> Result<(), ReWOOError> {
        while let Some(current_step) = self.planner.get_current_step(&session.state) {
            // 解析当前步骤
            let parsed_step = self.planner.parse_step(&current_step)?;
            
            // 验证步骤
            self.worker.validate_step(&parsed_step).await?;
            
            // 替换变量
            let substituted_args = self.planner.substitute_variables(
                &parsed_step.args,
                &session.state.results,
            );
            
            // 执行工具调用
            let tool_result = if self.config.worker.max_retries > 0 {
                self.worker.execute_tool_with_retry(
                    &parsed_step,
                    &substituted_args,
                    self.config.worker.max_retries,
                ).await?
            } else {
                self.worker.execute_tool(&parsed_step, &substituted_args).await?
            };
            
            // 更新状态
            if tool_result.success {
                session.state.results.insert(
                    parsed_step.variable.clone(),
                    tool_result.content.clone(),
                );
                session.metrics.successful_tool_calls += 1;
            } else {
                // 失败的工具调用不增加 successful_tool_calls 计数
                return Err(ReWOOError::ToolExecutionError(
                    format!("Step {} failed: {}", 
                        parsed_step.variable,
                        tool_result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                ));
            }
        }
        
        Ok(())
    }
    
    /// 路由决策 - 决定下一个执行节点
    pub fn route(&self, state: &ReWOOState) -> NodeRoute {
        // 如果没有计划，需要规划
        if state.steps.is_empty() {
            return NodeRoute::Tool;
        }
        
        // 如果还有未完成的步骤，继续工作
        if !self.planner.all_steps_completed(state) {
            return NodeRoute::Tool;
        }
        
        // 所有步骤完成，进行求解
        NodeRoute::Solve
    }
    
    /// 获取执行会话
    pub fn get_session(&self, session_id: &str) -> Option<&ReWOOSession> {
        self.sessions.get(session_id)
    }
    
    /// 获取所有会话
    pub fn get_all_sessions(&self) -> Vec<&ReWOOSession> {
        self.sessions.values().collect()
    }
    
    /// 清理过期会话
    pub fn cleanup_expired_sessions(&mut self, max_age_hours: u64) {
        let cutoff_time = SystemTime::now() - std::time::Duration::from_secs(max_age_hours * 3600);
        
        self.sessions.retain(|_, session| {
            session.started_at > cutoff_time
        });
    }
    
    /// 获取引擎统计信息
    pub fn get_engine_stats(&self) -> EngineStats {
        let total_sessions = self.sessions.len();
        let completed_sessions = self.sessions.values()
            .filter(|s| s.completed_at.is_some())
            .count();
        
        let total_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.tool_calls)
            .sum();
        
        let completed_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.successful_tool_calls)
            .sum();
        
        let failed_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.tool_calls - s.metrics.successful_tool_calls)
            .sum();
        
        let avg_execution_time = if completed_sessions > 0 {
            let total_time_ms: u64 = self.sessions.values()
                .filter(|s| s.completed_at.is_some())
                .map(|s| s.metrics.total_time_ms)
                .sum();
            let total_time = std::time::Duration::from_millis(total_time_ms);
            total_time / completed_sessions as u32
        } else {
            std::time::Duration::from_secs(0)
        };
        
        EngineStats {
            total_sessions,
            completed_sessions,
            total_steps,
            completed_steps,
            failed_steps,
            avg_execution_time,
            success_rate: if total_steps > 0 {
                (completed_steps as f32 / total_steps as f32) * 100.0
            } else {
                0.0
            },
        }
    }
    
    /// 验证引擎配置
    pub fn validate_config(&self) -> Result<(), ReWOOError> {
        // 验证 Planner 配置
        if self.config.planner.max_steps == 0 {
            return Err(ReWOOError::ConfigurationError(
                "Planner max_steps must be greater than 0".to_string()
            ));
        }
        
        if self.config.planner.max_tokens == 0 {
            return Err(ReWOOError::ConfigurationError(
                "Planner max_tokens must be greater than 0".to_string()
            ));
        }
        
        // 验证 Worker 配置
        if self.config.worker.timeout_seconds == 0 {
            return Err(ReWOOError::ConfigurationError(
                "Worker timeout_seconds must be greater than 0".to_string()
            ));
        }
        
        // 验证 Solver 配置
        if self.config.solver.max_tokens == 0 {
            return Err(ReWOOError::ConfigurationError(
                "Solver max_tokens must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// 重置引擎状态
    pub fn reset(&mut self) {
        self.sessions.clear();
    }
    
    /// 获取可用工具列表
        pub async fn get_available_tools(&self) -> Vec<String> {
        self.worker.get_available_tools().await
    }

    /// 检查工具是否可用
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.worker.is_tool_available(tool_name).await
    }
}

/// 引擎统计信息
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// 总会话数
    pub total_sessions: usize,
    /// 已完成会话数
    pub completed_sessions: usize,
    /// 总步骤数
    pub total_steps: u32,
    /// 已完成步骤数
    pub completed_steps: u32,
    /// 失败步骤数
    pub failed_steps: u32,
    /// 平均执行时间
    pub avg_execution_time: std::time::Duration,
    /// 成功率 (0-100)
    pub success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_route_decision() {
        // 创建测试配置
        // let config = ReWOOConfig {
        //     planner: PlannerConfig {
        //         model_name: "test".to_string(),
        //         temperature: 0.0,
        //         max_tokens: 1000,
        //         max_steps: 10,
        //     },
        //     worker: WorkerConfig {
        //         timeout_seconds: 30,
        //         max_retries: 3,
        //         enable_parallel: false,
        //     },
        //     solver: SolverConfig {
        //         model_name: "test".to_string(),
        //         temperature: 0.0,
        //         max_tokens: 2000,
        //     },
        // };
        
        // 创建模拟的依赖项
        // let ai_provider = Arc::new(MockAiProvider::new());
        // let tool_manager = Arc::new(MockToolManager::new());
        // let engine = ReWOOEngine::new(ai_provider, tool_manager, config).unwrap();
        
        // 测试空状态 - 应该路由到 Plan
        // let empty_state = ReWOOState {
        //     task: "test".to_string(),
        //     plan_string: String::new(),
        //     steps: Vec::new(),
        //     results: HashMap::new(),
        //     result: String::new(),
        // };
        // assert_eq!(engine.route(&empty_state), NodeRoute::Tool);
        
        // 测试有步骤但未完成 - 应该路由到 Tool
        let working_state = ReWOOState {
            task: "test".to_string(),
            plan_string: String::new(),
            steps: vec!["#E1 = Search[test]".to_string()],
            results: HashMap::new(),
            result: String::new(),
        };
        // assert_eq!(engine.route(&working_state), NodeRoute::Tool);
        
        // 测试所有步骤完成 - 应该路由到 Solve
        let mut completed_state = working_state.clone();
        completed_state.results.insert("#E1".to_string(), "result".to_string());
        // assert_eq!(engine.route(&completed_state), NodeRoute::Solve);
    }
    
    #[test]
    fn test_validate_config() {
        let valid_config = ReWOOConfig {
            planner: PlannerConfig {
                model_name: "test".to_string(),
                temperature: 0.0,
                max_tokens: 1000,
                max_steps: 10,
            },
            worker: WorkerConfig {
                timeout_seconds: 30,
                max_retries: 3,
                enable_parallel: false,
            },
            solver: SolverConfig {
                model_name: "test".to_string(),
                temperature: 0.0,
                max_tokens: 2000,
            },
        };
        
        // let ai_provider = Arc::new(MockAiProvider::new());
        // let tool_manager = Arc::new(MockToolManager::new());
        // let engine = ReWOOEngine::new(ai_provider, tool_manager, valid_config).unwrap();
        // assert!(engine.validate_config().is_ok());
        
        // 测试无效配置
        let mut invalid_config = valid_config.clone();
        invalid_config.planner.max_steps = 0;
        // let engine = ReWOOEngine::new(ai_provider, tool_manager, invalid_config).unwrap();
        // assert!(engine.validate_config().is_err());
    }
}
