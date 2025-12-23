use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub metadata: WorkflowMetadata,
    pub steps: Vec<WorkflowStep>,
    pub variables: HashMap<String, serde_json::Value>,
    pub error_handling: Option<ErrorHandling>,
    pub notifications: Option<NotificationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub action: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, String>,
    pub depends_on: Vec<String>,
    pub condition: Option<String>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<f64>,
    pub parallel: bool,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay: f64,
    pub backoff: BackoffStrategy,
    pub retry_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Exponential { multiplier: f32 },
    Linear { increment: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandling {
    pub default_strategy: ErrorStrategy,
    pub step_strategies: HashMap<String, ErrorStrategy>,
    pub on_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorStrategy {
    Stop,
    Retry,
    Continue,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub on_success: Vec<NotificationTarget>,
    pub on_failure: Vec<NotificationTarget>,
    pub on_progress: Vec<NotificationTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTarget {
    pub target_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionStatus {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub completed_steps: Option<u32>,
    pub total_steps: Option<u32>,
    pub progress: Option<u32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>, 
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub step_details: HashMap<String, StepExecutionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionDetail {
    pub step_id: String,
    pub status: ExecutionStatus,
    pub started_at: Option<DateTime<Utc>>, 
    pub completed_at: Option<DateTime<Utc>>, 
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
}

pub struct WorkflowEngine {
    active_executions: RwLock<HashMap<String, WorkflowExecutionStatus>>, 
    workflow_cache: RwLock<HashMap<String, WorkflowDefinition>>, 
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self { 
            active_executions: RwLock::new(HashMap::new()),
            workflow_cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn execute_workflow(&self, workflow: &WorkflowDefinition, _context: Option<HashMap<String, serde_json::Value>>) -> Result<String> {
        let execution_id = Uuid::new_v4().to_string();
        info!("Starting workflow execution: {} ({})", workflow.metadata.name, execution_id);
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
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id.clone(), execution_status);
        }
        {
            let mut cache = self.workflow_cache.write().await;
            cache.insert(workflow.metadata.id.clone(), workflow.clone());
        }
        Ok(execution_id)
    }

    #[allow(dead_code)]
    async fn execute_workflow_steps(&self, workflow: &WorkflowDefinition, execution_id: &str, _context: Option<HashMap<String, serde_json::Value>>) -> Result<()> {
        info!("Executing workflow steps for: {}", execution_id);
        for (index, step) in workflow.steps.iter().enumerate() {
            info!("Executing step {}: {}", index + 1, step.name);
            self.update_current_step(execution_id, &step.id).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            self.mark_step_completed(execution_id, &step.id).await;
            let progress = ((index + 1) as f32 / workflow.steps.len() as f32 * 100.0) as u32;
            self.update_progress(execution_id, progress).await;
        }
        self.mark_execution_completed(execution_id).await;
        info!("Workflow execution completed: {}", execution_id);
        Ok(())
    }

    pub async fn get_execution_status(&self, execution_id: &str) -> Result<WorkflowExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id).cloned().ok_or_else(|| anyhow::anyhow!("Execution not found: {}", execution_id))
    }

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

    pub async fn update_current_step(&self, execution_id: &str, step_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.current_step = Some(step_id.to_string());
        }
    }

    pub async fn mark_step_completed(&self, execution_id: &str, step_id: &str) {
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
            if let Some(completed) = execution.completed_steps.as_mut() { *completed += 1; }
        }
    }

    pub async fn mark_step_completed_with_result(&self, execution_id: &str, step_id: &str, result: serde_json::Value) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            let step_detail = StepExecutionDetail {
                step_id: step_id.to_string(),
                status: ExecutionStatus::Completed,
                started_at: Some(Utc::now()),
                completed_at: Some(Utc::now()),
                result: Some(result),
                error: None,
                retry_count: 0,
            };
            execution.step_details.insert(step_id.to_string(), step_detail);
            if let Some(completed) = execution.completed_steps.as_mut() { *completed += 1; }
        }
    }

    pub async fn update_progress(&self, execution_id: &str, progress: u32) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) { execution.progress = Some(progress); }
    }

    pub async fn get_step_result(&self, execution_id: &str, step_id: &str) -> Option<serde_json::Value> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id).and_then(|e| e.step_details.get(step_id)).and_then(|d| d.result.clone())
    }

    pub async fn mark_execution_completed(&self, execution_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Completed;
            execution.completed_at = Some(Utc::now());
            execution.progress = Some(100);
            execution.result = Some(serde_json::json!({"status": "completed", "message": "Workflow executed successfully"}));
        }
    }

    pub async fn mark_execution_failed(&self, execution_id: &str, error: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Failed;
            execution.completed_at = Some(Utc::now());
            execution.error = Some(error.to_string());
        }
    }

    pub async fn cleanup_completed_executions(&self) {
        let mut executions = self.active_executions.write().await;
        let mut to_remove = Vec::new();
        for (execution_id, execution) in executions.iter() {
            if matches!(execution.status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled) {
                if let Some(completed_at) = execution.completed_at {
                    let elapsed = Utc::now().signed_duration_since(completed_at);
                    if elapsed.num_hours() > 24 { to_remove.push(execution_id.clone()); }
                }
            }
        }
        for execution_id in to_remove { executions.remove(&execution_id); info!("Cleaned up completed execution: {}", execution_id); }
    }

    pub async fn get_active_executions(&self) -> Vec<WorkflowExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.values().cloned().collect()
    }
}

