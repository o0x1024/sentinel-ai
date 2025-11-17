use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use sentinel_engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine;
use sentinel_engines::rewoo::engine_adapter::ReWooEngine;
use sentinel_engines::llm_compiler::engine_adapter::LlmCompilerEngine;
use crate::agents::traits::{AgentTask, AgentSession};

// 使用sentinel-engines中的ExecutionEngine trait和相关类型
use sentinel_engines::agent_traits::{
    ExecutionPlan as AgentExecutionPlan, AgentExecutionResult, ExecutionProgress, ExecutionEngine,
};

/// 执行引擎类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineType {
    PlanExecute,
    ReWOO,
    LLMCompiler,
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: String,
    pub engine_type: EngineType,
    pub plan: AgentExecutionPlan,
    pub task: AgentTask,
    pub created_at: std::time::SystemTime,
}

/// 存储的引擎实例
pub enum EngineInstance {
    PlanExecute(PlanAndExecuteEngine),
    ReWOO(ReWooEngine),
    LLMCompiler(LlmCompilerEngine),
}

impl EngineInstance {
    /// 执行计划
    pub async fn execute_plan(
        &self,
        plan: &AgentExecutionPlan,
    ) -> Result<AgentExecutionResult> {
        match self {
            EngineInstance::PlanExecute(engine) => engine.execute_plan(plan).await,
            EngineInstance::ReWOO(engine) => engine.execute_plan(plan).await,
            EngineInstance::LLMCompiler(engine) => engine.execute_plan(plan).await,
        }
    }

    /// 获取进度
    pub async fn get_progress(&self, session_id: &str) -> Result<ExecutionProgress> {
        match self {
            EngineInstance::PlanExecute(engine) => engine.get_progress(session_id).await,
            EngineInstance::ReWOO(engine) => engine.get_progress(session_id).await,
            EngineInstance::LLMCompiler(engine) => engine.get_progress(session_id).await,
        }
    }

    /// 停止执行
    pub async fn stop_execution(&self, session_id: &str) -> Result<()> {
        match self {
            EngineInstance::PlanExecute(engine) => engine.cancel_execution(session_id).await,
            EngineInstance::ReWOO(engine) => engine.cancel_execution(session_id).await,
            EngineInstance::LLMCompiler(engine) => engine.cancel_execution(session_id).await,
        }
    }
}

/// 全局执行管理器
pub struct ExecutionManager {
    /// 活跃的执行上下文
    active_executions: RwLock<HashMap<String, ExecutionContext>>,
    /// 存储的引擎实例
    engine_instances: RwLock<HashMap<String, EngineInstance>>,
}

impl ExecutionManager {
    /// 创建新的执行管理器
    pub fn new() -> Self {
        Self {
            active_executions: RwLock::new(HashMap::new()),
            engine_instances: RwLock::new(HashMap::new()),
        }
    }

    /// 注册执行上下文和引擎实例
    pub async fn register_execution(
        &self,
        execution_id: String,
        engine_type: EngineType,
        plan: AgentExecutionPlan,
        task: AgentTask,
        engine: EngineInstance,
    ) -> Result<()> {
        let context = ExecutionContext {
            execution_id: execution_id.clone(),
            engine_type,
            plan,
            task,
            created_at: std::time::SystemTime::now(),
        };

        let execution_id_for_context = execution_id.clone();
        let execution_id_for_engine = execution_id;

        // 存储执行上下文
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id_for_context.clone(), context);
        }

        // 存储引擎实例
        {
            let mut engines = self.engine_instances.write().await;
            engines.insert(execution_id_for_engine, engine);
        }

        log::info!("Registered execution context: {}", execution_id_for_context);
        Ok(())
    }

    /// 获取执行上下文
    pub async fn get_execution_context(&self, execution_id: &str) -> Option<ExecutionContext> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id).cloned()
    }

    /// 获取引擎实例（由于所有权问题，暂不实现此方法）
    #[allow(dead_code)]
    async fn _get_engine_instance(&self, _execution_id: &str) -> Option<()> {
        // 由于引擎实例的所有权问题，此方法暂不实现
        // 在 execute_plan 方法中直接访问引擎实例
        None
    }

    /// 执行计划
    pub async fn execute_plan(
        &self,
        execution_id: &str,
    ) -> Result<AgentExecutionResult> {
        // 获取执行上下文
        let context = self.get_execution_context(execution_id).await
            .ok_or_else(|| anyhow::anyhow!("Execution context not found: {}", execution_id))?;

        // 获取引擎实例并执行
        let engines = self.engine_instances.read().await;
        if let Some(engine) = engines.get(execution_id) {
            engine.execute_plan(&context.plan).await
        } else {
            Err(anyhow::anyhow!("Engine instance not found: {}", execution_id))
        }
    }

    /// 获取执行进度
    pub async fn get_execution_progress(
        &self,
        execution_id: &str,
    ) -> Result<ExecutionProgress> {
        let engines = self.engine_instances.read().await;
        if let Some(engine) = engines.get(execution_id) {
            engine.get_progress(execution_id).await
        } else {
            Err(anyhow::anyhow!("Engine instance not found: {}", execution_id))
        }
    }

    /// 停止执行
    pub async fn stop_execution(&self, execution_id: &str) -> Result<()> {
        {
            let engines = self.engine_instances.read().await;
            if let Some(engine) = engines.get(execution_id) {
                engine.stop_execution(execution_id).await?;
            }
        }

        // 清理执行上下文
        self.cleanup_execution(execution_id).await;
        Ok(())
    }

    /// 清理执行上下文
    pub async fn cleanup_execution(&self, execution_id: &str) {
        {
            let mut executions = self.active_executions.write().await;
            executions.remove(execution_id);
        }
        
        {
            let mut engines = self.engine_instances.write().await;
            engines.remove(execution_id);
        }

        log::info!("Cleaned up execution context: {}", execution_id);
    }

    /// 获取所有活跃的执行
    pub async fn get_active_executions(&self) -> Vec<ExecutionContext> {
        let executions = self.active_executions.read().await;
        executions.values().cloned().collect()
    }

    /// 清理超时的执行
    pub async fn cleanup_expired_executions(&self, timeout_seconds: u64) {
        let now = std::time::SystemTime::now();
        let timeout_duration = std::time::Duration::from_secs(timeout_seconds);

        let mut expired_ids = Vec::new();
        
        {
            let executions = self.active_executions.read().await;
            for (id, context) in executions.iter() {
                if let Ok(elapsed) = now.duration_since(context.created_at) {
                    if elapsed > timeout_duration {
                        expired_ids.push(id.clone());
                    }
                }
            }
        }

        for id in expired_ids {
            log::warn!("Cleaning up expired execution: {}", id);
            self.cleanup_execution(&id).await;
        }
    }
}

impl Default for ExecutionManager {
    fn default() -> Self {
        Self::new()
    }
}
