//! 工作流引擎核心实现
//! 支持YAML定义的工作流解析、验证和执行

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use anyhow::{Result, anyhow};
use async_trait::async_trait;

/// 工作流定义结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub metadata: WorkflowMetadata,
    pub variables: HashMap<String, serde_json::Value>,
    pub steps: Vec<WorkflowStep>,
    pub error_handling: Option<ErrorHandling>,
}

/// 工作流元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub timeout: Option<u64>, // 秒
}

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub action: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: Vec<String>,
    pub depends_on: Vec<String>,
    pub condition: Option<String>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<u64>,
    pub parallel: Option<bool>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay_seconds: u64,
    pub backoff_multiplier: Option<f64>,
}

/// 错误处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandling {
    pub on_failure: String, // "stop", "continue", "retry"
    pub cleanup_steps: Vec<String>,
    pub notification: Option<NotificationConfig>,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub channels: Vec<String>,
    pub template: String,
}

/// 工作流执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// 步骤执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Retrying,
}

/// 工作流执行实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub context: ExecutionContext,
    pub step_results: HashMap<String, StepResult>,
    pub error: Option<String>,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: StepStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub logs: Vec<String>,
}

/// Agent执行接口
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    async fn execute(
        &self,
        action: &str,
        inputs: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>>;
    
    fn get_capabilities(&self) -> Vec<String>;
    fn get_agent_type(&self) -> String;
}

/// 工作流引擎
pub struct WorkflowEngine {
    agents: Arc<RwLock<HashMap<String, Arc<dyn AgentExecutor>>>>,
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    event_sender: mpsc::UnboundedSender<WorkflowEvent>,
}

/// 工作流事件
#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    WorkflowStarted { execution_id: String },
    WorkflowCompleted { execution_id: String },
    WorkflowFailed { execution_id: String, error: String },
    StepStarted { execution_id: String, step_id: String },
    StepCompleted { execution_id: String, step_id: String },
    StepFailed { execution_id: String, step_id: String, error: String },
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new() -> (Self, mpsc::UnboundedReceiver<WorkflowEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let engine = Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        };
        
        (engine, event_receiver)
    }
    
    /// 注册Agent
    pub async fn register_agent(&self, agent_type: String, agent: Arc<dyn AgentExecutor>) {
        let mut agents = self.agents.write().await;
        agents.insert(agent_type, agent);
    }
    
    /// 验证工作流定义
    pub async fn validate_workflow(&self, workflow: &WorkflowDefinition) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // 检查步骤依赖
        for step in &workflow.steps {
            for dep in &step.depends_on {
                if !workflow.steps.iter().any(|s| s.id == *dep) {
                    issues.push(format!("步骤 '{}' 依赖的步骤 '{}' 不存在", step.id, dep));
                }
            }
            
            // 检查Agent是否存在
            let agents = self.agents.read().await;
            if !agents.contains_key(&step.agent_type) {
                issues.push(format!("步骤 '{}' 使用的Agent类型 '{}' 未注册", step.id, step.agent_type));
            }
        }
        
        // 检查循环依赖
        if let Err(e) = self.check_circular_dependencies(&workflow.steps) {
            issues.push(e.to_string());
        }
        
        Ok(issues)
    }
    
    /// 执行工作流
    pub async fn execute_workflow(
        &self,
        workflow: WorkflowDefinition,
        initial_context: Option<ExecutionContext>,
    ) -> Result<String> {
        // 验证工作流
        let issues = self.validate_workflow(&workflow).await?;
        if !issues.is_empty() {
            return Err(anyhow!("工作流验证失败: {:?}", issues));
        }
        
        let execution_id = Uuid::new_v4().to_string();
        let context = initial_context.unwrap_or_else(|| ExecutionContext {
            variables: workflow.variables.clone(),
            outputs: HashMap::new(),
            metadata: HashMap::new(),
        });
        
        let execution = WorkflowExecution {
            id: execution_id.clone(),
            workflow_id: workflow.metadata.name.clone(),
            status: WorkflowStatus::Running,
            started_at: chrono::Utc::now(),
            completed_at: None,
            context,
            step_results: HashMap::new(),
            error: None,
        };
        
        // 保存执行实例
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), execution);
        }
        
        // 发送开始事件
        let _ = self.event_sender.send(WorkflowEvent::WorkflowStarted {
            execution_id: execution_id.clone(),
        });
        
        // 异步执行工作流
        let engine = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_workflow_steps(execution_id.clone(), workflow).await {
                engine.handle_workflow_error(execution_id, e).await;
            }
        });
        
        Ok(execution_id)
    }
    
    /// 执行工作流步骤
    async fn execute_workflow_steps(
        &self,
        execution_id: String,
        workflow: WorkflowDefinition,
    ) -> Result<()> {
        let execution_order = self.calculate_execution_order(&workflow.steps)?;
        
        for step_batch in execution_order {
            // 并行执行同一批次的步骤
            let mut handles = Vec::new();
            
            for step in step_batch {
                if self.should_execute_step(&execution_id, &step).await? {
                    let engine = self.clone();
                    let execution_id = execution_id.clone();
                    let step = step.clone();
                    
                    let handle = tokio::spawn(async move {
                        engine.execute_single_step(execution_id, step).await
                    });
                    
                    handles.push(handle);
                }
            }
            
            // 等待所有步骤完成
            for handle in handles {
                handle.await??;
            }
        }
        
        // 标记工作流完成
        self.complete_workflow(execution_id).await?;
        
        Ok(())
    }
    
    /// 执行单个步骤
    async fn execute_single_step(
        &self,
        execution_id: String,
        step: WorkflowStep,
    ) -> Result<()> {
        let step_id = step.id.clone();
        
        // 发送步骤开始事件
        let _ = self.event_sender.send(WorkflowEvent::StepStarted {
            execution_id: execution_id.clone(),
            step_id: step_id.clone(),
        });
        
        let mut retry_count = 0;
        let max_retries = step.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        
        loop {
            match self.try_execute_step(&execution_id, &step).await {
                Ok(outputs) => {
                    // 步骤执行成功
                    self.record_step_success(&execution_id, &step_id, outputs).await?;
                    
                    let _ = self.event_sender.send(WorkflowEvent::StepCompleted {
                        execution_id: execution_id.clone(),
                        step_id: step_id.clone(),
                    });
                    
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    
                    if retry_count >= max_retries {
                        // 重试次数用尽，步骤失败
                        self.record_step_failure(&execution_id, &step_id, e.to_string()).await?;
                        
                        let _ = self.event_sender.send(WorkflowEvent::StepFailed {
                            execution_id: execution_id.clone(),
                            step_id: step_id.clone(),
                            error: e.to_string(),
                        });
                        
                        return Err(e);
                    } else {
                        // 等待重试
                        if let Some(retry_config) = &step.retry {
                            let delay = retry_config.delay_seconds as f64 *
                                retry_config.backoff_multiplier.unwrap_or(1.0).powi(retry_count as i32 - 1);
                            tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay)).await;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 尝试执行步骤
    async fn try_execute_step(
        &self,
        execution_id: &str,
        step: &WorkflowStep,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // 获取执行上下文
        let context = {
            let executions = self.executions.read().await;
            let execution = executions.get(execution_id)
                .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
            execution.context.clone()
        };
        
        // 解析输入参数
        let resolved_inputs = self.resolve_inputs(&step.inputs, &context)?;
        
        // 获取Agent并执行
        let agents = self.agents.read().await;
        let agent = agents.get(&step.agent_type)
            .ok_or_else(|| anyhow!("Agent类型不存在: {}", step.agent_type))?;
        
        let outputs = agent.execute(&step.action, &resolved_inputs, &context).await?;
        
        Ok(outputs)
    }
    
    /// 解析输入参数
    fn resolve_inputs(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut resolved = HashMap::new();
        
        for (key, value) in inputs {
            let resolved_value = self.resolve_value(value, context)?;
            resolved.insert(key.clone(), resolved_value);
        }
        
        Ok(resolved)
    }
    
    /// 解析单个值（支持变量引用）
    fn resolve_value(
        &self,
        value: &serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        match value {
            serde_json::Value::String(s) => {
                if s.starts_with("${{") && s.ends_with("}}") {
                    let var_name = &s[3..s.len()-2];
                    
                    // 查找变量
                    if let Some(var_value) = context.variables.get(var_name) {
                        Ok(var_value.clone())
                    } else if let Some(output_value) = context.outputs.get(var_name) {
                        Ok(output_value.clone())
                    } else {
                        Err(anyhow!("变量不存在: {}", var_name))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            _ => Ok(value.clone()),
        }
    }
    
    /// 检查是否应该执行步骤
    async fn should_execute_step(
        &self,
        execution_id: &str,
        step: &WorkflowStep,
    ) -> Result<bool> {
        // 检查条件
        if let Some(condition) = &step.condition {
            let executions = self.executions.read().await;
            let execution = executions.get(execution_id)
                .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
            
            // 简单的条件评估（实际实现可能需要更复杂的表达式解析器）
            return Ok(self.evaluate_condition(condition, &execution.context)?);
        }
        
        Ok(true)
    }
    
    /// 评估条件表达式
    fn evaluate_condition(
        &self,
        condition: &str,
        context: &ExecutionContext,
    ) -> Result<bool> {
        // 简化的条件评估实现
        // 实际项目中应该使用专门的表达式解析器
        
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = self.resolve_condition_value(parts[0].trim(), context)?;
                let right = self.resolve_condition_value(parts[1].trim(), context)?;
                return Ok(left == right);
            }
        }
        
        // 默认返回true
        Ok(true)
    }
    
    /// 解析条件值
    fn resolve_condition_value(
        &self,
        value: &str,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        if value.starts_with("${{") && value.ends_with("}}") {
            let var_name = &value[3..value.len()-2];
            if let Some(var_value) = context.variables.get(var_name) {
                Ok(var_value.clone())
            } else if let Some(output_value) = context.outputs.get(var_name) {
                Ok(output_value.clone())
            } else {
                Err(anyhow!("变量不存在: {}", var_name))
            }
        } else {
            // 尝试解析为JSON值
            Ok(serde_json::from_str(value).unwrap_or_else(|_| {
                serde_json::Value::String(value.to_string())
            }))
        }
    }
    
    /// 计算执行顺序
    fn calculate_execution_order(
        &self,
        steps: &[WorkflowStep],
    ) -> Result<Vec<Vec<WorkflowStep>>> {
        let mut order = Vec::new();
        let mut remaining_steps: Vec<_> = steps.iter().cloned().collect();
        let mut completed_steps = std::collections::HashSet::new();
        
        while !remaining_steps.is_empty() {
            let mut current_batch = Vec::new();
            let mut batch_indices = Vec::new();
            
            // 找到所有依赖已满足的步骤
            for (i, step) in remaining_steps.iter().enumerate() {
                let dependencies_satisfied = step.depends_on.iter()
                    .all(|dep| completed_steps.contains(dep));
                
                if dependencies_satisfied {
                    current_batch.push(step.clone());
                    batch_indices.push(i);
                }
            }
            
            if current_batch.is_empty() {
                return Err(anyhow!("检测到循环依赖或无法满足的依赖"));
            }
            
            // 从剩余步骤中移除当前批次
            for &i in batch_indices.iter().rev() {
                let step = remaining_steps.remove(i);
                completed_steps.insert(step.id);
            }
            
            order.push(current_batch);
        }
        
        Ok(order)
    }
    
    /// 检查循环依赖
    fn check_circular_dependencies(&self, steps: &[WorkflowStep]) -> Result<()> {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        
        for step in steps {
            if !visited.contains(&step.id) {
                if self.has_cycle(step, steps, &mut visited, &mut rec_stack)? {
                    return Err(anyhow!("检测到循环依赖"));
                }
            }
        }
        
        Ok(())
    }
    
    /// 检查是否存在循环
    fn has_cycle(
        &self,
        step: &WorkflowStep,
        all_steps: &[WorkflowStep],
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> Result<bool> {
        visited.insert(step.id.clone());
        rec_stack.insert(step.id.clone());
        
        for dep_id in &step.depends_on {
            if let Some(dep_step) = all_steps.iter().find(|s| s.id == *dep_id) {
                if !visited.contains(dep_id) {
                    if self.has_cycle(dep_step, all_steps, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(dep_id) {
                    return Ok(true);
                }
            }
        }
        
        rec_stack.remove(&step.id);
        Ok(false)
    }
    
    /// 记录步骤成功
    async fn record_step_success(
        &self,
        execution_id: &str,
        step_id: &str,
        outputs: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
        
        let step_result = StepResult {
            step_id: step_id.to_string(),
            status: StepStatus::Completed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            outputs: outputs.clone(),
            error: None,
            retry_count: 0,
            logs: Vec::new(),
        };
        
        execution.step_results.insert(step_id.to_string(), step_result);
        
        // 更新执行上下文
        for (key, value) in outputs {
            execution.context.outputs.insert(format!("{}.{}", step_id, key), value);
        }
        
        Ok(())
    }
    
    /// 记录步骤失败
    async fn record_step_failure(
        &self,
        execution_id: &str,
        step_id: &str,
        error: String,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
        
        let step_result = StepResult {
            step_id: step_id.to_string(),
            status: StepStatus::Failed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            outputs: HashMap::new(),
            error: Some(error),
            retry_count: 0,
            logs: Vec::new(),
        };
        
        execution.step_results.insert(step_id.to_string(), step_result);
        
        Ok(())
    }
    
    /// 完成工作流
    async fn complete_workflow(&self, execution_id: String) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(&execution_id)
            .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
        
        execution.status = WorkflowStatus::Completed;
        execution.completed_at = Some(chrono::Utc::now());
        
        let _ = self.event_sender.send(WorkflowEvent::WorkflowCompleted {
            execution_id: execution_id.clone(),
        });
        
        Ok(())
    }
    
    /// 处理工作流错误
    async fn handle_workflow_error(&self, execution_id: String, error: anyhow::Error) {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(&execution_id) {
            execution.status = WorkflowStatus::Failed;
            execution.completed_at = Some(chrono::Utc::now());
            execution.error = Some(error.to_string());
            
            let _ = self.event_sender.send(WorkflowEvent::WorkflowFailed {
                execution_id: execution_id.clone(),
                error: error.to_string(),
            });
        }
    }
    
    /// 获取执行状态
    pub async fn get_execution(&self, execution_id: &str) -> Option<WorkflowExecution> {
        let executions = self.executions.read().await;
        executions.get(execution_id).cloned()
    }
    
    /// 取消工作流执行
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| anyhow!("执行实例不存在: {}", execution_id))?;
        
        execution.status = WorkflowStatus::Cancelled;
        execution.completed_at = Some(chrono::Utc::now());
        
        Ok(())
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            executions: self.executions.clone(),
            event_sender: self.event_sender.clone(),
        }
    }
}

/// 工作流引擎构建器
pub struct WorkflowEngineBuilder {
    agents: HashMap<String, Arc<dyn AgentExecutor>>,
}

impl WorkflowEngineBuilder {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }
    
    pub fn with_agent(mut self, agent_type: String, agent: Arc<dyn AgentExecutor>) -> Self {
        self.agents.insert(agent_type, agent);
        self
    }
    
    pub async fn build(self) -> (WorkflowEngine, mpsc::UnboundedReceiver<WorkflowEvent>) {
        let (engine, receiver) = WorkflowEngine::new();
        
        for (agent_type, agent) in self.agents {
            engine.register_agent(agent_type, agent).await;
        }
        
        (engine, receiver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    struct MockAgent {
        agent_type: String,
        call_count: AtomicU32,
    }
    
    impl MockAgent {
        fn new(agent_type: String) -> Self {
            Self {
                agent_type,
                call_count: AtomicU32::new(0),
            }
        }
    }
    
    #[async_trait]
    impl AgentExecutor for MockAgent {
        async fn execute(
            &self,
            action: &str,
            inputs: &HashMap<String, serde_json::Value>,
            _context: &ExecutionContext,
        ) -> Result<HashMap<String, serde_json::Value>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            
            let mut outputs = HashMap::new();
            outputs.insert("result".to_string(), serde_json::json!({
                "action": action,
                "inputs": inputs,
                "call_count": self.call_count.load(Ordering::SeqCst)
            }));
            
            Ok(outputs)
        }
        
        fn get_capabilities(&self) -> Vec<String> {
            vec!["test_action".to_string()]
        }
        
        fn get_agent_type(&self) -> String {
            self.agent_type.clone()
        }
    }
    
    #[tokio::test]
    async fn test_workflow_execution() {
        let (engine, mut receiver) = WorkflowEngineBuilder::new()
            .with_agent("test_agent".to_string(), Arc::new(MockAgent::new("test_agent".to_string())))
            .build()
            .await;
        
        let workflow = WorkflowDefinition {
            metadata: WorkflowMetadata {
                name: "test_workflow".to_string(),
                version: "1.0.0".to_string(),
                description: Some("测试工作流".to_string()),
                author: Some("test".to_string()),
                tags: vec!["test".to_string()],
                timeout: None,
            },
            variables: HashMap::new(),
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "第一步".to_string(),
                    agent_type: "test_agent".to_string(),
                    action: "test_action".to_string(),
                    inputs: HashMap::new(),
                    outputs: vec!["result".to_string()],
                    depends_on: vec![],
                    condition: None,
                    retry: None,
                    timeout: None,
                    parallel: None,
                },
            ],
            error_handling: None,
        };
        
        let execution_id = engine.execute_workflow(workflow, None).await.unwrap();
        
        // 等待工作流完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let execution = engine.get_execution(&execution_id).await.unwrap();
        assert_eq!(execution.status, WorkflowStatus::Completed);
        assert!(execution.step_results.contains_key("step1"));
    }
}