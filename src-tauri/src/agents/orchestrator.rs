//! Agent 编排器
//!
//! 协调 Planner、TodoManager 和 ReAct 执行引擎。
//! 职责：
//! 1. 接收任务
//! 2. 分析复杂度，决定是否创建 Todos
//! 3. 如果需要，生成计划并创建 Todos
//! 4. 调用 ReAct 引擎执行
//! 5. 根据执行进度更新 Todos 状态

use super::planner::{TaskComplexity, TaskPlan, TaskPlanner, PlannerConfig};
use super::todo_manager::{Todo, TodoManager, TodoStatus};
use super::emitter::AgentMessageEmitter;
use crate::engines::react::{ReactEngine, ReactConfig};
use crate::services::ai::AiService;
use crate::services::database::DatabaseService;
use crate::services::mcp::McpService;
use crate::agents::traits::{AgentTask, AgentSession, AgentSessionStatus, AgentExecutionResult, LogLevel, SessionLog};
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tauri::AppHandle;
use tracing::{info, error};

/// Orchestrator 配置
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// 是否自动创建 Todos
    pub auto_create_todos: bool,
    /// 强制创建 Todos（忽略复杂度判断）
    pub force_todos: bool,
    /// Planner 配置
    pub planner_config: PlannerConfig,
    /// ReAct 引擎配置
    pub react_config: ReactConfig,
    /// 最大迭代次数
    pub max_iterations: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            auto_create_todos: true,
            force_todos: false,
            planner_config: PlannerConfig::default(),
            react_config: ReactConfig::default(),
            max_iterations: 10,
        }
    }
}

/// Agent 编排器
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    planner: TaskPlanner,
    todo_manager: Arc<TodoManager>,
    app_handle: Option<AppHandle>,
}

impl AgentOrchestrator {
    /// 创建新的编排器
    pub fn new(config: OrchestratorConfig, app_handle: Option<AppHandle>) -> Self {
        let planner = TaskPlanner::new(config.planner_config.clone());
        let todo_manager = Arc::new(TodoManager::new(app_handle.clone()));
        
        Self {
            config,
            planner,
            todo_manager,
            app_handle,
        }
    }

    /// 获取 TodoManager 引用（用于外部访问）
    pub fn todo_manager(&self) -> Arc<TodoManager> {
        self.todo_manager.clone()
    }

    /// 准备执行任务（分析、规划、创建 Todos）
    pub async fn prepare_task(&self, execution_id: &str, task: &str) -> Result<TaskPreparation> {
        // 1. 分析任务复杂度
        let complexity = self.planner.analyze_complexity(task);
        
        // 2. 决定是否需要 Todos
        let should_create_todos = self.config.force_todos 
            || (self.config.auto_create_todos && complexity.should_create_todos());

        let mut preparation = TaskPreparation {
            execution_id: execution_id.to_string(),
            task: task.to_string(),
            complexity,
            plan: None,
            todos_created: false,
        };

        // 3. 如果需要 Todos，创建初始任务
        if should_create_todos {
            // 创建一个初始的"分析中"任务
            let initial_todo = Todo::new("analyze", "分析任务并制定计划...")
                .with_status(TodoStatus::InProgress);
            
            self.todo_manager.write_todos(execution_id, vec![initial_todo], false).await?;
            preparation.todos_created = true;
        }

        Ok(preparation)
    }

    /// 更新计划并刷新 Todos
    pub async fn update_plan(&self, execution_id: &str, task: &str, llm_response: &str) -> Result<Option<TaskPlan>> {
        // 尝试从 LLM 响应解析计划
        let plan = match self.planner.parse_plan_from_response(task, llm_response) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        // 如果有步骤，更新 Todos
        if !plan.steps.is_empty() {
            let todos = self.planner.plan_to_todos(&plan);
            self.todo_manager.write_todos(execution_id, todos, false).await?;
        }

        Ok(Some(plan))
    }

    /// 更新当前步骤状态
    pub async fn update_step_status(
        &self,
        execution_id: &str,
        step_id: &str,
        status: TodoStatus,
    ) -> Result<()> {
        self.todo_manager.update_status(execution_id, step_id, status).await
    }

    /// 标记步骤开始执行
    pub async fn mark_step_started(&self, execution_id: &str, step_id: &str) -> Result<()> {
        self.update_step_status(execution_id, step_id, TodoStatus::InProgress).await
    }

    /// 标记步骤完成
    pub async fn mark_step_completed(&self, execution_id: &str, step_id: &str) -> Result<()> {
        self.update_step_status(execution_id, step_id, TodoStatus::Completed).await
    }

    /// 标记步骤失败（保持 in_progress 状态，等待处理）
    pub async fn mark_step_failed(&self, execution_id: &str, step_id: &str) -> Result<()> {
        // 失败的步骤保持 in_progress 状态，需要用户/系统处理
        // 这与 Claude Code 的行为一致：失败任务不会自动标记为完成
        Ok(())
    }

    /// 开始下一个待办步骤
    pub async fn start_next_pending_step(&self, execution_id: &str) -> Result<Option<String>> {
        let todos = self.todo_manager.get_todos(execution_id).await;
        
        // 找到第一个 pending 的步骤
        for todo in &todos {
            if todo.status == TodoStatus::Pending {
                self.todo_manager.update_status(execution_id, &todo.id, TodoStatus::InProgress).await?;
                return Ok(Some(todo.id.clone()));
            }
        }
        
        Ok(None)
    }

    /// 获取当前进行中的步骤
    pub async fn get_current_step(&self, execution_id: &str) -> Option<Todo> {
        self.todo_manager.get_in_progress(execution_id).await
    }

    /// 获取所有 Todos
    pub async fn get_todos(&self, execution_id: &str) -> Vec<Todo> {
        self.todo_manager.get_todos(execution_id).await
    }

    /// 清理执行数据
    pub async fn cleanup(&self, execution_id: &str) {
        self.todo_manager.clear(execution_id).await;
    }

    /// 根据工具名称更新相关 Todo 状态
    pub async fn update_todo_by_tool(
        &self,
        execution_id: &str,
        tool_name: &str,
        status: TodoStatus,
    ) -> Result<()> {
        let todos = self.todo_manager.get_todos(execution_id).await;
        
        // 找到与工具关联的 Todo
        for todo in &todos {
            if let Some(ref metadata) = todo.metadata {
                if metadata.tool_name.as_deref() == Some(tool_name) {
                    self.todo_manager.update_status(execution_id, &todo.id, status.clone()).await?;
                    return Ok(());
                }
            }
        }
        
        Ok(())
    }

    /// 自动推进 Todos（完成当前步骤，开始下一步）
    pub async fn advance_todos(&self, execution_id: &str) -> Result<Option<String>> {
        // 找到当前进行中的步骤
        if let Some(current) = self.todo_manager.get_in_progress(execution_id).await {
            // 标记为完成
            self.todo_manager.update_status(execution_id, &current.id, TodoStatus::Completed).await?;
        }
        
        // 开始下一个步骤
        self.start_next_pending_step(execution_id).await
    }

    /// 执行任务 - 核心执行循环
    /// 
    /// 这个方法将：
    /// 1. 准备任务（分析复杂度、创建初始 Todos）
    /// 2. 创建 ReAct 引擎实例
    /// 3. 执行任务
    /// 4. 根据执行结果更新 Todos
    pub async fn execute(
        &self,
        execution_id: &str,
        task: &str,
        ai_service: Arc<AiService>,
        mcp_service: Option<Arc<McpService>>,
        db_service: Option<Arc<DatabaseService>>,
    ) -> Result<ExecutionResult> {
        info!("Orchestrator: Starting execution for task: {}", task);
        
        let start_time = std::time::Instant::now();
        let emitter = AgentMessageEmitter::new(self.app_handle.clone(), execution_id);
        
        // 1. 准备任务
        let preparation = self.prepare_task(execution_id, task).await?;
        
        // 2. 发送开始事件
        emitter.emit_start(task);
        
        // 3. 创建 ReAct 引擎
        let engine = ReactEngine::new(self.config.react_config.clone())
            .with_services(
                ai_service,
                mcp_service,
                db_service,
                self.app_handle.clone(),
            );
        
        // 4. 创建执行会话
        let mut session = OrchestratorSession::new(execution_id.to_string(), task.to_string());
        
        // 5. 创建 AgentTask
        let mut parameters = HashMap::new();
        parameters.insert("query".to_string(), serde_json::json!(task));
        parameters.insert("execution_id".to_string(), serde_json::json!(execution_id));
        
        let agent_task = AgentTask {
            id: execution_id.to_string(),
            description: task.to_string(),
            target: None,
            parameters,
            user_id: "default".to_string(),
            priority: crate::agents::TaskPriority::Normal,
            timeout: Some(300),
        };
        
        // 6. 执行任务
        let result = match engine.execute(&agent_task, &mut session).await {
            Ok(exec_result) => {
                // 更新所有 Todos 为完成状态
                let todos = self.todo_manager.get_todos(execution_id).await;
                for todo in todos {
                    if todo.status != TodoStatus::Completed {
                        let _ = self.todo_manager.update_status(
                            execution_id, 
                            &todo.id, 
                            TodoStatus::Completed
                        ).await;
                    }
                }
                
                // 发送完成事件
                emitter.emit_complete(exec_result.success, exec_result.data.clone());
                
                ExecutionResult {
                    execution_id: execution_id.to_string(),
                    success: exec_result.success,
                    output: exec_result.data.as_ref()
                        .and_then(|d| d.get("output"))
                        .and_then(|o| o.as_str())
                        .map(|s| s.to_string()),
                    error: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    complexity: preparation.complexity,
                    iterations: exec_result.data.as_ref()
                        .and_then(|d| d.get("iterations"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    tool_calls: exec_result.data.as_ref()
                        .and_then(|d| d.get("tool_calls"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                }
            }
            Err(e) => {
                error!("Orchestrator: Execution failed: {}", e);
                
                // 失败时保持当前 in_progress 状态（与 Claude Code 行为一致）
                // 任务失败不等于任务完成，需要人工处理
                
                // 发送错误事件
                emitter.emit_error(&e.to_string());
                
                ExecutionResult {
                    execution_id: execution_id.to_string(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    complexity: preparation.complexity,
                    iterations: 0,
                    tool_calls: 0,
                }
            }
        };
        
        Ok(result)
    }
}

/// 任务准备结果
#[derive(Debug, Clone)]
pub struct TaskPreparation {
    pub execution_id: String,
    pub task: String,
    pub complexity: TaskComplexity,
    pub plan: Option<TaskPlan>,
    pub todos_created: bool,
}

/// 执行结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub complexity: TaskComplexity,
    pub iterations: u32,
    pub tool_calls: u32,
}

/// Orchestrator 内部使用的会话
struct OrchestratorSession {
    id: String,
    task: AgentTask,
    status: AgentSessionStatus,
    logs: Vec<SessionLog>,
    result: Option<AgentExecutionResult>,
}

impl OrchestratorSession {
    fn new(id: String, task_description: String) -> Self {
        let task = AgentTask {
            id: id.clone(),
            description: task_description,
            target: None,
            parameters: HashMap::new(),
            user_id: "default".to_string(),
            priority: crate::agents::TaskPriority::Normal,
            timeout: Some(300),
        };
        
        Self {
            id,
            task,
            status: AgentSessionStatus::Executing,
            logs: Vec::new(),
            result: None,
        }
    }
}

#[async_trait::async_trait]
impl AgentSession for OrchestratorSession {
    fn get_session_id(&self) -> &str { &self.id }
    fn get_task(&self) -> &AgentTask { &self.task }
    fn get_status(&self) -> AgentSessionStatus { self.status.clone() }
    
    async fn update_status(&mut self, status: AgentSessionStatus) -> Result<()> {
        self.status = status;
        Ok(())
    }
    
    async fn add_log(&mut self, level: LogLevel, message: String) -> Result<()> {
        self.logs.push(SessionLog {
            level,
            message,
            timestamp: chrono::Utc::now(),
            source: "orchestrator".to_string(),
        });
        Ok(())
    }
    
    fn get_logs(&self) -> &[SessionLog] { &self.logs }
    
    async fn set_result(&mut self, result: AgentExecutionResult) -> Result<()> {
        self.result = Some(result);
        Ok(())
    }
    
    fn get_result(&self) -> Option<&AgentExecutionResult> { self.result.as_ref() }
}

/// 快捷函数：创建默认配置的 Orchestrator
pub fn create_orchestrator(app_handle: Option<AppHandle>) -> AgentOrchestrator {
    AgentOrchestrator::new(OrchestratorConfig::default(), app_handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = create_orchestrator(None);
        assert!(orchestrator.config.auto_create_todos);
    }

    #[tokio::test]
    async fn test_prepare_simple_task() {
        let orchestrator = create_orchestrator(None);
        let prep = orchestrator.prepare_task("exec-1", "查询天气").await.unwrap();
        
        assert_eq!(prep.complexity, TaskComplexity::Simple);
        assert!(!prep.todos_created); // 简单任务不创建 Todos
    }

    #[tokio::test]
    async fn test_prepare_complex_task() {
        let orchestrator = create_orchestrator(None);
        let prep = orchestrator.prepare_task("exec-2", "对 example.com 进行渗透测试").await.unwrap();
        
        assert_eq!(prep.complexity, TaskComplexity::Complex);
        assert!(prep.todos_created); // 复杂任务创建 Todos
    }

    #[tokio::test]
    async fn test_force_todos() {
        let config = OrchestratorConfig {
            force_todos: true,
            ..Default::default()
        };
        let orchestrator = AgentOrchestrator::new(config, None);
        let prep = orchestrator.prepare_task("exec-3", "简单查询").await.unwrap();
        
        assert!(prep.todos_created); // 强制创建 Todos
    }

    #[tokio::test]
    async fn test_update_step_status() {
        let orchestrator = create_orchestrator(None);
        let execution_id = "exec-4";
        
        // 准备任务
        let _ = orchestrator.prepare_task(execution_id, "渗透测试").await.unwrap();
        
        // 手动添加一些 Todos
        let todos = vec![
            Todo::new("1", "步骤1").with_status(TodoStatus::InProgress),
            Todo::new("2", "步骤2"),
        ];
        orchestrator.todo_manager.write_todos(execution_id, todos, false).await.unwrap();
        
        // 完成第一步
        orchestrator.mark_step_completed(execution_id, "1").await.unwrap();
        
        // 验证状态
        let updated = orchestrator.get_todos(execution_id).await;
        assert_eq!(updated[0].status, TodoStatus::Completed);
    }

    #[tokio::test]
    async fn test_advance_todos() {
        let orchestrator = create_orchestrator(None);
        let execution_id = "exec-5";
        
        // 添加 Todos
        let todos = vec![
            Todo::new("1", "步骤1").with_status(TodoStatus::InProgress),
            Todo::new("2", "步骤2"),
            Todo::new("3", "步骤3"),
        ];
        orchestrator.todo_manager.write_todos(execution_id, todos, false).await.unwrap();
        
        // 推进到下一步
        let next = orchestrator.advance_todos(execution_id).await.unwrap();
        assert_eq!(next, Some("2".to_string()));
        
        // 验证状态
        let updated = orchestrator.get_todos(execution_id).await;
        assert_eq!(updated[0].status, TodoStatus::Completed);
        assert_eq!(updated[1].status, TodoStatus::InProgress);
        assert_eq!(updated[2].status, TodoStatus::Pending);
    }
}

