use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agents::traits::{AgentTask, AgentExecutionResult, ExecutionProgress, ExecutionPlan as AgentExecutionPlan};

/// 执行引擎类型
/// 所有任务统一使用泛化的 ReAct 引擎
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineType {
    React,   // 泛化后的 ReAct（唯一引擎，任务特性通过 Prompt 配置）
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
/// 注：由于引擎执行现在主要通过 ReAct 直接进行，此枚举简化为占位
pub enum EngineInstance {
    /// 占位符，实际执行通过 dispatch_with_react 直接进行
    Placeholder,
}

impl EngineInstance {
    /// 执行计划（占位实现）
    pub async fn execute_plan(
        &self,
        _plan: &AgentExecutionPlan,
    ) -> Result<AgentExecutionResult> {
        // 实际执行现在通过 dispatch_with_react 直接进行
        Err(anyhow::anyhow!("Direct execution via EngineInstance is deprecated. Use dispatch_with_react instead."))
    }

    /// 获取进度（占位实现）
    pub async fn get_progress(&self, _session_id: &str) -> Result<ExecutionProgress> {
        Ok(ExecutionProgress {
            total_steps: 0,
            completed_steps: 0,
            current_step: None,
            progress_percentage: 0.0,
            estimated_remaining_seconds: None,
        })
    }

    /// 停止执行（占位实现）
    pub async fn stop_execution(&self, _session_id: &str) -> Result<()> {
        Ok(())
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
