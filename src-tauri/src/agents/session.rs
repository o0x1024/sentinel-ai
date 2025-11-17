//! Agent会话的默认实现

use super::traits::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Agent会话数据（用于数据库存储）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionData {
    pub session_id: String,
    pub task_id: String,
    pub status: String,
    pub agent_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 默认的Agent会话实现
#[derive(Debug)]
pub struct DefaultAgentSession {
    /// 会话ID
    session_id: String,
    /// 关联任务
    task: AgentTask,
    /// 当前状态
    status: AgentSessionStatus,
    /// 执行日志
    logs: Vec<SessionLog>,
    /// 执行结果
    result: Option<AgentExecutionResult>,
    /// 创建时间
    created_at: chrono::DateTime<Utc>,
    /// 更新时间
    updated_at: chrono::DateTime<Utc>,
    /// 取消标记
    is_cancellation_requested: bool,
}

impl DefaultAgentSession {
    /// 创建新的会话
    pub fn new(task: AgentTask) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            task,
            status: AgentSessionStatus::Created,
            logs: Vec::new(),
            result: None,
            created_at: now,
            updated_at: now,
            is_cancellation_requested: false,
        }
    }

    /// 获取创建时间
    pub fn get_created_at(&self) -> chrono::DateTime<Utc> {
        self.created_at
    }

    /// 获取更新时间
    pub fn get_updated_at(&self) -> chrono::DateTime<Utc> {
        self.updated_at
    }

    /// 检查是否有取消请求
    pub fn is_cancellation_requested(&self) -> bool {
        self.is_cancellation_requested
    }

    /// 请求取消
    pub fn request_cancellation(&mut self) {
        self.is_cancellation_requested = true;
        self.updated_at = Utc::now();
    }
}

#[async_trait]
impl AgentSession for DefaultAgentSession {
    fn get_session_id(&self) -> &str {
        &self.session_id
    }

    fn get_task(&self) -> &AgentTask {
        &self.task
    }

    fn get_status(&self) -> AgentSessionStatus {
        self.status.clone()
    }

    async fn update_status(&mut self, status: AgentSessionStatus) -> Result<()> {
        // 如果状态变更为已取消，设置取消标记
        if matches!(status, AgentSessionStatus::Cancelled) {
            self.is_cancellation_requested = true;
        }

        self.status = status;
        self.updated_at = Utc::now();

        // 记录状态变更日志
        let message = format!("Status changed to {:?}", self.status);
        self.add_log(LogLevel::Info, message).await?;

        Ok(())
    }

    async fn add_log(&mut self, level: LogLevel, message: String) -> Result<()> {
        let log = SessionLog {
            level,
            message,
            timestamp: Utc::now(),
            source: "agent_session".to_string(),
        };

        self.logs.push(log);
        self.updated_at = Utc::now();

        Ok(())
    }

    fn get_logs(&self) -> &[SessionLog] {
        &self.logs
    }

    async fn set_result(&mut self, result: AgentExecutionResult) -> Result<()> {
        self.result = Some(result);
        self.updated_at = Utc::now();

        // 根据结果更新状态
        if let Some(ref result) = self.result {
            let new_status = if result.success {
                AgentSessionStatus::Completed
            } else {
                AgentSessionStatus::Failed
            };
            self.update_status(new_status).await?;
        }

        Ok(())
    }

    fn get_result(&self) -> Option<&AgentExecutionResult> {
        self.result.as_ref()
    }
}
