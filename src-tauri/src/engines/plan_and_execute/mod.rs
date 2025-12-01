//! Plan-and-Execute 架构模块
//!
//! 实现基于LangGraph的Plan-and-Execute架构，包含以下核心组件：
//! - Planner: 任务规划器
//! - Executor: 任务执行器  
//! - Replanner: 动态重新规划器
//! - Memory Manager: 上下文和状态管理
//! - Tool Interface: 工具调用接口

// 核心组件模块
pub mod executor;
pub mod memory_manager;
pub mod planner;
pub mod replanner;
pub mod engine_adapter;
pub mod repository;
pub mod resource_tracker;

pub mod types;

// 重新导出核心组件
pub use engine_adapter::PlanAndExecuteEngine;
pub use executor::{Executor, ExecutorConfig};
pub use memory_manager::{MemoryManager, MemoryManagerConfig};
pub use planner::{Planner, PlannerConfig};
pub use replanner::{Replanner, ReplannerConfig};
pub use repository::PlanExecuteRepository;

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
        // DISABLED: _ai_adapter_manager: Arc<crate::ai_adapter::core::AiAdapterManager>,
        _ai_service_manager: Arc<crate::services::AiServiceManager>,
        _db_service: Arc<DatabaseService>,
    ) -> Result<Self, types::PlanAndExecuteError> {
        // 先创建新的适配器引擎
        let adapter_engine = Arc::new(
            PlanAndExecuteEngine::new().await
                .map_err(|e| types::PlanAndExecuteError::InitializationError(e.to_string()))?,
        );

        Ok(Self { engine: adapter_engine })
    }

    /// 直接获取内部引擎实例
    pub fn get_engine(&self) -> &Arc<PlanAndExecuteEngine> {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::AiServiceManager;
    use crate::services::DatabaseService;



    #[tokio::test]
    async fn test_plan_and_execute_basic() {
        let mut db_service = DatabaseService::new();

        db_service
            .initialize()
            .await
            .expect("Failed to initialize database");
        let db_service = Arc::new(db_service);

        let ai_service_manager = Arc::new(AiServiceManager::new(db_service.clone()));

        let plan_execute =
            PlanAndExecute::new(ai_service_manager, db_service)
                .await
                .unwrap();

        // 简化测试：只测试创建引擎
        let _engine = plan_execute.get_engine();
        // 测试只验证创建成功
    }
}
