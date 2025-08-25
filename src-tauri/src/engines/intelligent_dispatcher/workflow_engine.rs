//! 工作流引擎模块
//! 
//! 负责工作流的创建、执行、监控和管理

use std::collections::HashMap;

use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use log::info;

/// 工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// 工作流元数据
    pub metadata: WorkflowMetadata,
    /// 工作流步骤
    pub steps: Vec<WorkflowStep>,
    /// 全局变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 错误处理配置
    pub error_handling: Option<ErrorHandling>,
    /// 通知配置
    pub notifications: Option<NotificationConfig>,
}

/// 工作流元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// 工作流ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// 步骤ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 代理类型
    pub agent_type: String,
    /// 执行动作
    pub action: String,
    /// 输入参数
    pub inputs: HashMap<String, serde_json::Value>,
    /// 输出映射
    pub outputs: HashMap<String, String>,
    /// 依赖步骤
    pub depends_on: Vec<String>,
    /// 执行条件
    pub condition: Option<String>,
    /// 重试配置
    pub retry: Option<RetryConfig>,
    /// 超时设置（秒）
    pub timeout: Option<u64>,
    /// 是否并行执行
    pub parallel: bool,
    /// 步骤配置
    pub config: Option<serde_json::Value>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 重试延迟（秒）
    pub delay: u64,
    /// 退避策略
    pub backoff: BackoffStrategy,
    /// 重试条件
    pub retry_on: Vec<String>,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// 固定延迟
    Fixed,
    /// 指数退避
    Exponential { multiplier: f32 },
    /// 线性增长
    Linear { increment: u64 },
}

/// 错误处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandling {
    /// 默认策略
    pub default_strategy: ErrorStrategy,
    /// 步骤特定策略
    pub step_strategies: HashMap<String, ErrorStrategy>,
    /// 错误回调
    pub on_error: Option<String>,
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorStrategy {
    /// 停止执行
    Stop,
    /// 重试
    Retry,
    /// 继续执行
    Continue,
    /// 跳过当前步骤
    Skip,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// 成功通知
    pub on_success: Vec<NotificationTarget>,
    /// 失败通知
    pub on_failure: Vec<NotificationTarget>,
    /// 进度通知
    pub on_progress: Vec<NotificationTarget>,
}

/// 通知目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTarget {
    /// 目标类型
    pub target_type: String,
    /// 配置参数
    pub config: serde_json::Value,
}

/// 工作流执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionStatus {
    /// 执行ID
    pub execution_id: String,
    /// 工作流ID
    pub workflow_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 完成的步骤
    pub completed_steps: Option<u32>,
    /// 总步骤数
    pub total_steps: Option<u32>,
    /// 执行进度 (0-100)
    pub progress: Option<u32>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 步骤执行详情
    pub step_details: HashMap<String, StepExecutionDetail>,
}

/// 执行状态枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 暂停
    Paused,
}

/// 步骤执行详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionDetail {
    /// 步骤ID
    pub step_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 重试次数
    pub retry_count: u32,
}

/// 工作流引擎
pub struct WorkflowEngine {
    /// 活跃的工作流执行
    active_executions: RwLock<HashMap<String, WorkflowExecutionStatus>>,
    /// 工作流定义缓存
    workflow_cache: RwLock<HashMap<String, WorkflowDefinition>>,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new() -> Self {
        Self {
            active_executions: RwLock::new(HashMap::new()),
            workflow_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 执行工作流
    pub async fn execute_workflow(
        &self,
        workflow: &WorkflowDefinition,
        _context: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        let execution_id = Uuid::new_v4().to_string();
        
        info!("Starting workflow execution: {} ({})", workflow.metadata.name, execution_id);

        // 创建执行状态
        let execution_status = WorkflowExecutionStatus {
            execution_id: execution_id.clone(),
            workflow_id: workflow.metadata.id.clone(),
            status: ExecutionStatus::Running,
            current_step: None,
            completed_steps: Some(0),
            total_steps: Some(workflow.steps.len() as u32),
            progress: Some(0),
            started_at: Utc::now(),
            completed_at: None,
            result: None,
            error: None,
            step_details: HashMap::new(),
        };

        // 存储执行状态
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id.clone(), execution_status);
        }

        // 缓存工作流定义
        {
            let mut cache = self.workflow_cache.write().await;
            cache.insert(workflow.metadata.id.clone(), workflow.clone());
        }

        // 启动异步执行，但不在 tokio::spawn 中执行，而是直接返回 execution_id
        // 实际执行会在调用方管理

        Ok(execution_id)
    }

    /// 执行工作流步骤
    async fn execute_workflow_steps(
        &self,
        workflow: &WorkflowDefinition,
        execution_id: &str,
        _context: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        info!("Executing workflow steps for: {}", execution_id);

        // 简化的步骤执行逻辑
        for (index, step) in workflow.steps.iter().enumerate() {
            info!("Executing step {}: {}", index + 1, step.name);

            // 更新当前步骤
            self.update_current_step(execution_id, &step.id).await;

            // 模拟步骤执行
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 标记步骤完成
            self.mark_step_completed(execution_id, &step.id).await;

            // 更新进度
            let progress = ((index + 1) as f32 / workflow.steps.len() as f32 * 100.0) as u32;
            self.update_progress(execution_id, progress).await;
        }

        // 标记工作流完成
        self.mark_execution_completed(execution_id).await;

        info!("Workflow execution completed: {}", execution_id);
        Ok(())
    }

    /// 获取执行状态
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<WorkflowExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", execution_id))
    }

    /// 取消执行
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Cancelled;
            execution.completed_at = Some(Utc::now());
            info!("Workflow execution cancelled: {}", execution_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// 更新当前步骤
    async fn update_current_step(&self, execution_id: &str, step_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.current_step = Some(step_id.to_string());
        }
    }

    /// 标记步骤完成
    async fn mark_step_completed(&self, execution_id: &str, step_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            let step_detail = StepExecutionDetail {
                step_id: step_id.to_string(),
                status: ExecutionStatus::Completed,
                started_at: Some(Utc::now()),
                completed_at: Some(Utc::now()),
                result: Some(serde_json::json!({"status": "success"})),
                error: None,
                retry_count: 0,
            };
            execution.step_details.insert(step_id.to_string(), step_detail);
            
            if let Some(completed) = execution.completed_steps.as_mut() {
                *completed += 1;
            }
        }
    }

    /// 更新进度
    async fn update_progress(&self, execution_id: &str, progress: u32) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.progress = Some(progress);
        }
    }

    /// 标记执行完成
    async fn mark_execution_completed(&self, execution_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Completed;
            execution.completed_at = Some(Utc::now());
            execution.progress = Some(100);
            execution.result = Some(serde_json::json!({
                "status": "completed",
                "message": "Workflow executed successfully"
            }));
        }
    }

    /// 标记执行失败
    async fn mark_execution_failed(&self, execution_id: &str, error: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Failed;
            execution.completed_at = Some(Utc::now());
            execution.error = Some(error.to_string());
        }
    }

    /// 清理已完成的执行
    pub async fn cleanup_completed_executions(&self) {
        let mut executions = self.active_executions.write().await;
        let mut to_remove = Vec::new();
        
        for (execution_id, execution) in executions.iter() {
            if matches!(execution.status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled) {
                if let Some(completed_at) = execution.completed_at {
                    let elapsed = Utc::now().signed_duration_since(completed_at);
                    if elapsed.num_hours() > 24 { // 清理24小时前的执行记录
                        to_remove.push(execution_id.clone());
                    }
                }
            }
        }
        
        for execution_id in to_remove {
            executions.remove(&execution_id);
            info!("Cleaned up completed execution: {}", execution_id);
        }
    }

    /// 获取所有活跃执行
    pub async fn get_active_executions(&self) -> Vec<WorkflowExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.values().cloned().collect()
    }
}
