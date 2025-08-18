//! Plan-and-Execute 架构模块
//!
//! 实现基于LangGraph的Plan-and-Execute架构，包含以下核心组件：
//! - Planner: 任务规划器
//! - Executor: 任务执行器  
//! - Replanner: 动态重新规划器
//! - Memory Manager: 上下文和状态管理
//! - Tool Interface: 工具调用接口

// 核心组件模块
pub mod engine;
pub mod executor;
pub mod memory_manager;
pub mod planner;
pub mod replanner;

pub mod types;

// 重新导出核心组件
pub use engine::{
    EngineMetrics, EngineStatus, PlanAndExecuteConfig, PlanAndExecuteEngine, TaskSession,
};
pub use executor::{Executor, ExecutorConfig};
pub use memory_manager::{MemoryManager, MemoryManagerConfig};
pub use planner::{Planner, PlannerConfig};
pub use replanner::{Replanner, ReplannerConfig};

pub use types::*;

use crate::services::database::DatabaseService;
use std::sync::Arc;

/// Plan-and-Execute 架构主入口
#[derive(Debug, Clone)]
pub struct PlanAndExecute {
    engine: Arc<PlanAndExecuteEngine>,
}

impl PlanAndExecute {
    /// 创建新的 Plan-and-Execute 实例
    pub async fn new(
        config: PlanAndExecuteConfig,
        ai_adapter_manager: Arc<crate::ai_adapter::core::AiAdapterManager>,
        ai_service_manager: Arc<crate::services::AiServiceManager>,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self, PlanAndExecuteError> {
        let engine = Arc::new(
            PlanAndExecuteEngine::new(config, ai_adapter_manager, ai_service_manager, db_service)
                .await?,
        );

        Ok(Self { engine })
    }

    /// 启动架构
    pub async fn start(&self) -> Result<(), PlanAndExecuteError> {
        self.engine.start().await
    }

    /// 停止架构
    pub async fn stop(&self) -> Result<(), PlanAndExecuteError> {
        self.engine.stop().await
    }

    /// 执行任务
    pub async fn execute_task(&self, request: TaskRequest) -> Result<String, PlanAndExecuteError> {
        self.engine.execute_task(request).await
    }

    /// 获取任务状态
    pub async fn get_task_status(&self, session_id: &str) -> Option<TaskStatus> {
        self.engine.get_task_status(session_id).await
    }

    /// 获取任务结果
    pub async fn get_task_result(&self, session_id: &str) -> Option<TaskResult> {
        self.engine.get_task_result(session_id).await
    }

    /// 取消任务
    pub async fn cancel_task(&self, session_id: &str) -> Result<bool, PlanAndExecuteError> {
        self.engine.cancel_task(session_id).await
    }

    /// 获取引擎状态
    pub async fn get_status(&self) -> EngineStatus {
        self.engine.get_status().await
    }

    /// 获取引擎指标
    pub async fn get_metrics(&self) -> EngineMetrics {
        self.engine.get_metrics().await
    }

    /// 暂停执行
    pub async fn pause(&self) -> Result<(), PlanAndExecuteError> {
        self.engine.pause().await
    }

    /// 恢复执行
    pub async fn resume(&self) -> Result<(), PlanAndExecuteError> {
        self.engine.resume().await
    }

    /// 获取活跃任务列表
    pub async fn get_active_tasks(&self) -> Vec<String> {
        self.engine.get_active_tasks().await
    }

    /// 获取任务历史
    pub async fn get_task_history(&self, limit: Option<usize>) -> Vec<TaskSession> {
        self.engine.get_task_history(limit).await
    }

    /// 导出状态
    pub async fn export_state(&self) -> Result<serde_json::Value, PlanAndExecuteError> {
        self.engine.export_state().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::AiServiceManager;
    use crate::{ai_adapter::core::AiAdapterManager, services::DatabaseService};

    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_plan_and_execute_basic() {
        let mut db_service = DatabaseService::new();

        db_service
            .initialize()
            .await
            .expect("Failed to initialize database");
        let db_service = Arc::new(db_service);

        let config = PlanAndExecuteConfig::default();

        let ai_adapter_manager = Arc::new(AiAdapterManager::new());
        let ai_service_manager = Arc::new(AiServiceManager::new(db_service.clone()));

        let plan_execute =
            PlanAndExecute::new(config, ai_adapter_manager, ai_service_manager, db_service)
                .await
                .unwrap();

        // 启动
        assert!(plan_execute.start().await.is_ok());

        // 创建测试任务
        let request = TaskRequest {
            id: Uuid::new_v4().to_string(),
            name: "测试安全扫描".to_string(),
            description: "对目标网站进行安全扫描".to_string(),
            task_type: TaskType::InformationRetrieval,
            target: Some(TargetInfo {
                target_type: TargetType::Url,
                identifier: "https://example.com".to_string(),
                parameters: HashMap::new(),
                credentials: None,
                metadata: HashMap::new(),
            }),
            priority: Priority::High,
            parameters: HashMap::new(),
            constraints: HashMap::new(),
            metadata: HashMap::new(),
            created_at: SystemTime::now(),
        };

        // 执行任务
        let session_id = plan_execute.execute_task(request).await.unwrap();
        assert!(!session_id.is_empty());

        // 检查状态
        tokio::time::sleep(Duration::from_millis(100)).await;
        let status = plan_execute.get_task_status(&session_id).await;
        assert!(status.is_some());

        // 停止
        assert!(plan_execute.stop().await.is_ok());
    }
}
