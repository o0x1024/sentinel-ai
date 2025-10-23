//! ReWOO引擎适配器 - DISABLED (ai_adapter removed)
//! 
//! 简化版本，所有功能都被禁用，等待 Rig 重构

use crate::agents::traits::*;
use crate::engines::rewoo::{
    rewoo_planner::ReWOOPlanner,
    rewoo_worker::ReWOOWorker,
    rewoo_solver::ReWOOSolver,
    rewoo_types::*,
};
use crate::services::ai::AiServiceManager;
use crate::services::prompt_db::PromptRepository;
use crate::services::database::DatabaseService;
use crate::services::database::Database; // trait for DB methods
use crate::commands::agent_commands::WorkflowStepDetail;
use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// ReWOO引擎适配器 - DISABLED
#[derive(Debug)]
pub struct ReWooEngine {
    engine_info: EngineInfo,
    planner: Option<ReWOOPlanner>,
    worker: Option<ReWOOWorker>,
    solver: Option<ReWOOSolver>,
    config: ReWOOConfig,
    sessions: HashMap<String, ReWOOSession>,
    ai_service_manager: Option<Arc<AiServiceManager>>,
    runtime_params: Option<HashMap<String, serde_json::Value>>,
    app_handle: Option<tauri::AppHandle>,
    db_service: Option<Arc<DatabaseService>>,
}

impl ReWooEngine {
    /// 创建基础引擎适配器 - DISABLED
    pub fn new(config: ReWOOConfig) -> Result<Self> {
        let engine_info = EngineInfo {
            name: "ReWOO".to_string(),
            version: "1.0.0".to_string(),
            description: "DISABLED - ReWOO engine needs Rig refactor".to_string(),
            supported_scenarios: vec![],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 0,
                execution_speed: 0,
                resource_usage: 0,
                concurrency_capability: 0,
                complexity_handling: 0,
            },
        };

        Ok(Self {
            engine_info,
            planner: None,
            worker: None,
            solver: None,
            config,
            sessions: HashMap::new(),
            ai_service_manager: None,
            runtime_params: None,
            app_handle: None,
            db_service: None,
        })
    }
    
    /// 使用完整依赖创建引擎适配器 - DISABLED
    pub async fn new_with_dependencies(
        _ai_service_manager: Arc<AiServiceManager>,
        _config: ReWOOConfig,
        _db_service: Arc<DatabaseService>,
    ) -> Result<Self> {
        // DISABLED: ReWOO engine needs complete Rig refactor
        Err(anyhow::anyhow!("ReWOO engine disabled - needs complete Rig refactor"))
    }

    /// 从AI服务管理器获取AI provider - DISABLED
    async fn get_ai_provider_from_service_manager(
        _ai_service_manager: &Arc<AiServiceManager>,
    ) -> Result<String> { // DISABLED: was Arc<dyn AiProvider>
        // DISABLED: ReWOO engine needs Rig refactor
        Err(anyhow::anyhow!("ReWOO engine disabled - needs Rig refactor"))
    }
}

#[async_trait]
impl ExecutionEngine for ReWooEngine {
    async fn execute(&mut self, _task: TaskRequest) -> Result<TaskResult, ExecutionError> {
        Err(ExecutionError::EngineError("ReWOO engine disabled - needs Rig refactor".to_string()))
    }

    async fn cancel(&mut self, _task_id: &str) -> Result<(), ExecutionError> {
        Err(ExecutionError::EngineError("ReWOO engine disabled - needs Rig refactor".to_string()))
    }

    async fn get_status(&self, _task_id: &str) -> Result<TaskStatus, ExecutionError> {
        Err(ExecutionError::EngineError("ReWOO engine disabled - needs Rig refactor".to_string()))
    }

    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }

    async fn health_check(&self) -> Result<HealthStatus, ExecutionError> {
        Ok(HealthStatus {
            status: "disabled".to_string(),
            message: Some("ReWOO engine disabled - needs Rig refactor".to_string()),
            details: HashMap::new(),
        })
    }

    async fn get_metrics(&self) -> Result<EngineMetrics, ExecutionError> {
        Ok(EngineMetrics {
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            average_execution_time: 0.0,
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0,
                disk_io_mb: 0.0,
            },
        })
    }

    async fn configure(&mut self, _config: serde_json::Value) -> Result<(), ExecutionError> {
        Err(ExecutionError::EngineError("ReWOO engine disabled - needs Rig refactor".to_string()))
    }
}
