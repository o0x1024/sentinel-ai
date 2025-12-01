//! Plan-and-Execute架构数据库访问层
//! 
//! 这个模块提供了Plan-and-Execute架构的数据持久化功能，
//! 包括执行计划、步骤、会话、结果、指标等数据的CRUD操作。

use crate::engines::types::*;
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::time::UNIX_EPOCH;

/// 仓库错误类型
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    DatabaseError(String),
    #[error("序列化错误: {0}")]
    SerializationError(String),
    #[error("时间错误: {0}")]
    TimeError(String),
    #[error("未找到记录: {0}")]
    NotFound(String),
}

/// 执行统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionStatistics {
    /// 总会话数
    pub total_sessions: u64,
    /// 完成会话数
    pub completed_sessions: u64,
    /// 失败会话数
    pub failed_sessions: u64,
    /// 运行中会话数
    pub running_sessions: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均执行时间（秒）
    pub average_execution_time: u64,
}

/// Plan-and-Execute数据库仓库
#[derive(Debug)]
pub struct PlanExecuteRepository {
    /// 数据库连接池
    pool: SqlitePool,
}

impl PlanExecuteRepository {
    /// 创建新的仓库实例
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    /// 保存执行计划
    pub async fn save_execution_plan(&self, plan: &ExecutionPlan) -> Result<(), RepositoryError> {
        let created_at = plan.created_at.duration_since(UNIX_EPOCH)
            .map_err(|e| RepositoryError::TimeError(format!("时间转换失败: {}", e)))?
            .as_secs() as i64;
        
        let metadata_json = serde_json::to_string(&plan.metadata)
            .map_err(|e| RepositoryError::SerializationError(format!("元数据序列化失败: {}", e)))?;
        
        // 保存执行计划
        sqlx::query(
            "INSERT OR REPLACE INTO execution_plans (id, name, description, estimated_duration, created_at, metadata) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )
        .bind(&plan.id)
        .bind(&plan.name)
        .bind(&plan.description)
        .bind(plan.estimated_duration as i64)
        .bind(created_at)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("保存执行计划失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取执行计划
    pub async fn get_execution_plan(&self, plan_id: &str) -> Result<Option<ExecutionPlan>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, name, description, estimated_duration, created_at, metadata 
             FROM execution_plans WHERE id = ?1"
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询执行计划失败: {}", e)))?;
        
        if let Some(row) = row {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let description: String = row.get("description");
            let estimated_duration: i64 = row.get("estimated_duration");
            let created_at: i64 = row.get("created_at");
            let metadata_json: String = row.get("metadata");
            
            let metadata: PlanMetadata = serde_json::from_str(&metadata_json)
                .map_err(|e| RepositoryError::SerializationError(format!("元数据反序列化失败: {}", e)))?;
            
            let plan = ExecutionPlan {
                id,
                name,
                description,
                steps: Vec::new(), // 简化版本，不加载步骤
                estimated_duration: estimated_duration as u64,
                created_at: UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64),
                metadata,
                dependencies: HashMap::new(),
            };
            
            Ok(Some(plan))
        } else {
            Ok(None)
        }
    }
    
    /// 保存执行会话
    pub async fn save_execution_session(&self, session: &ExecutionSession) -> Result<(), RepositoryError> {
        let started_at = session.started_at.duration_since(UNIX_EPOCH)
            .map_err(|e| RepositoryError::TimeError(format!("时间转换失败: {}", e)))?
            .as_secs() as i64;
        
        let metadata_json = serde_json::to_string(&session.metadata)
            .map_err(|e| RepositoryError::SerializationError(format!("元数据序列化失败: {}", e)))?;
        
        let context_json = serde_json::to_string(&session.context)
            .map_err(|e| RepositoryError::SerializationError(format!("上下文序列化失败: {}", e)))?;
        
        sqlx::query(
            "INSERT OR REPLACE INTO execution_sessions 
             (id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(&session.id)
        .bind(&session.plan_id)
        .bind(format!("{:?}", session.status))
        .bind(started_at)
        .bind(session.completed_at.map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64))
        .bind(session.current_step)
        .bind(session.progress)
        .bind(context_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("保存执行会话失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取执行会话
    pub async fn get_execution_session(&self, session_id: &str) -> Result<Option<ExecutionSession>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, plan_id, status, started_at, completed_at, CAST(current_step AS INTEGER) AS current_step, progress, context, metadata 
             FROM execution_sessions WHERE id = ?1"
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询执行会话失败: {}", e)))?;
        
        if let Some(row) = row {
            let id: String = row.get("id");
            let plan_id: String = row.get("plan_id");
            let status_str: String = row.get("status");
            let started_at: i64 = row.get("started_at");
            let completed_at: Option<i64> = row.get("completed_at");
            let current_step: Option<i32> = row.get("current_step");
            let progress: u32 = row.get("progress");
            let context_json: String = row.get("context");
            let metadata_json: String = row.get("metadata");
            
            let status = match status_str.as_str() {
                "Pending" => ExecutionStatus::Pending,
                "Running" => ExecutionStatus::Running,
                "Completed" => ExecutionStatus::Completed,
                "Failed" => ExecutionStatus::Failed,
                "Cancelled" => ExecutionStatus::Cancelled,
                "Paused" => ExecutionStatus::Paused,
                _ => return Err(RepositoryError::SerializationError(format!("未知状态: {}", status_str))),
            };
            
            let context: ExecutionContext = serde_json::from_str(&context_json)
                .map_err(|e| RepositoryError::SerializationError(format!("上下文反序列化失败: {}", e)))?;
            
            let metadata: SessionMetadata = serde_json::from_str(&metadata_json)
                .map_err(|e| RepositoryError::SerializationError(format!("元数据反序列化失败: {}", e)))?;
            
            let session = ExecutionSession {
                id,
                plan_id,
                status,
                started_at: UNIX_EPOCH + std::time::Duration::from_secs(started_at as u64),
                completed_at: completed_at.map(|t| UNIX_EPOCH + std::time::Duration::from_secs(t as u64)),
                current_step,
                progress,
                context,
                step_results: HashMap::new(),
                metadata,
            };
            
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }
    
    /// 列出所有执行会话
    pub async fn list_execution_sessions(&self) -> Result<Vec<ExecutionSession>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, plan_id, status, started_at, completed_at, CAST(current_step AS INTEGER) AS current_step, progress, context, metadata 
             FROM execution_sessions ORDER BY started_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询执行会话列表失败: {}", e)))?;
        
        let mut sessions = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let plan_id: String = row.get("plan_id");
            let status_str: String = row.get("status");
            let started_at: i64 = row.get("started_at");
            let completed_at: Option<i64> = row.get("completed_at");
            let current_step: Option<i32> = row.get("current_step");
            let progress: u32 = row.get("progress")     ;
            let context_json: String = row.get("context");
            let metadata_json: String = row.get("metadata");
            
            let status = match status_str.as_str() {
                "Pending" => ExecutionStatus::Pending,
                "Running" => ExecutionStatus::Running,
                "Completed" => ExecutionStatus::Completed,
                "Failed" => ExecutionStatus::Failed,
                "Cancelled" => ExecutionStatus::Cancelled,
                "Paused" => ExecutionStatus::Paused,
                _ => continue, // 跳过未知状态
            };
            
            let context: ExecutionContext = serde_json::from_str(&context_json).unwrap_or_default();
            let metadata: SessionMetadata = serde_json::from_str(&metadata_json).unwrap_or_default();
            
            let session = ExecutionSession {
                id,
                plan_id,
                status,
                started_at: UNIX_EPOCH + std::time::Duration::from_secs(started_at as u64),
                completed_at: completed_at.map(|t| UNIX_EPOCH + std::time::Duration::from_secs(t as u64)),
                current_step,
                progress,
                context,
                step_results: HashMap::new(),
                metadata,
            };
            
            sessions.push(session);
        }
        
        Ok(sessions)
    }
    
    /// 删除执行会话
    pub async fn delete_execution_session(&self, session_id: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM execution_sessions WHERE id = ?1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("删除执行会话失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取执行统计信息
    pub async fn get_execution_statistics(&self) -> Result<ExecutionStatistics, RepositoryError> {
        let total_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询总会话数失败: {}", e)))? as u64;
            
        let completed_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Completed'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询完成会话数失败: {}", e)))? as u64;
            
        let failed_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Failed'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询失败会话数失败: {}", e)))? as u64;
            
        let running_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Running'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询运行中会话数失败: {}", e)))? as u64;
            
        // 计算平均执行时间（仅针对已完成的会话）
        let avg_time = if completed_sessions > 0 {
            let avg_time_f64 = sqlx::query_scalar::<_, Option<f64>>(
                "SELECT AVG(CAST((julianday(datetime(completed_at, 'unixepoch')) - julianday(datetime(started_at, 'unixepoch'))) * 86400 AS REAL)) 
                 FROM execution_sessions 
                 WHERE status = 'Completed' AND completed_at IS NOT NULL"
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询平均执行时间失败: {}", e)))?;
            avg_time_f64.unwrap_or(0.0).round() as u64
        } else {
            0
        };
        
        Ok(ExecutionStatistics {
            total_sessions,
            completed_sessions,
            failed_sessions,
            running_sessions,
            success_rate: if total_sessions > 0 { (completed_sessions as f64 / total_sessions as f64) * 100.0 } else { 0.0 },
            average_execution_time: avg_time,
        })
    }

    // ===== 步骤持久化功能 =====

    /// 保存执行步骤
    pub async fn save_execution_step(&self, plan_id: &str, step: &crate::engines::plan_and_execute::types::ExecutionStep) -> Result<(), RepositoryError> {
        let tool_config_json = serde_json::to_string(&step.tool_config)
            .map_err(|e| RepositoryError::SerializationError(format!("工具配置序列化失败: {}", e)))?;
        
        let parameters_json = serde_json::to_string(&step.parameters)
            .map_err(|e| RepositoryError::SerializationError(format!("参数序列化失败: {}", e)))?;
        
        let retry_config_json = serde_json::to_string(&step.retry_config)
            .map_err(|e| RepositoryError::SerializationError(format!("重试配置序列化失败: {}", e)))?;
        
        let preconditions_json = serde_json::to_string(&step.preconditions)
            .map_err(|e| RepositoryError::SerializationError(format!("前置条件序列化失败: {}", e)))?;
        
        let postconditions_json = serde_json::to_string(&step.postconditions)
            .map_err(|e| RepositoryError::SerializationError(format!("后置条件序列化失败: {}", e)))?;
        
        sqlx::query(
            "INSERT OR REPLACE INTO execution_steps 
             (id, plan_id, name, description, step_type, tool_config, parameters, estimated_duration, retry_config, preconditions, postconditions) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )
        .bind(&step.id)
        .bind(plan_id)
        .bind(&step.name)
        .bind(&step.description)
        .bind(format!("{:?}", step.step_type))
        .bind(tool_config_json)
        .bind(parameters_json)
        .bind(step.estimated_duration as i64)
        .bind(retry_config_json)
        .bind(preconditions_json)
        .bind(postconditions_json)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("保存执行步骤失败: {}", e)))?;
        
        Ok(())
    }

    /// 获取计划的所有步骤
    pub async fn get_execution_steps(&self, plan_id: &str) -> Result<Vec<crate::engines::plan_and_execute::types::ExecutionStep>, RepositoryError> {
        use crate::engines::plan_and_execute::types::{ExecutionStep, StepType, ToolConfig, RetryConfig};
        
        let rows = sqlx::query(
            "SELECT id, name, description, step_type, tool_config, parameters, estimated_duration, retry_config, preconditions, postconditions 
             FROM execution_steps WHERE plan_id = ?1 ORDER BY id"
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询执行步骤失败: {}", e)))?;
        
        let mut steps = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let description: String = row.get("description");
            let step_type_str: String = row.get("step_type");
            let tool_config_json: String = row.get("tool_config");
            let parameters_json: String = row.get("parameters");
            let estimated_duration: i64 = row.get("estimated_duration");
            let retry_config_json: String = row.get("retry_config");
            let preconditions_json: String = row.get("preconditions");
            let postconditions_json: String = row.get("postconditions");
            
            let step_type = match step_type_str.as_str() {
                "ToolCall" => StepType::ToolCall,
                "AiReasoning" => StepType::AiReasoning,
                "DataProcessing" => StepType::DataProcessing,
                "Conditional" => StepType::Conditional,
                "Parallel" => StepType::Parallel,
                "Wait" => StepType::Wait,
                "ManualConfirmation" => StepType::ManualConfirmation,
                _ => StepType::AiReasoning,
            };
            
            let tool_config: Option<ToolConfig> = serde_json::from_str(&tool_config_json).ok();
            let parameters: HashMap<String, serde_json::Value> = serde_json::from_str(&parameters_json).unwrap_or_default();
            let retry_config: RetryConfig = serde_json::from_str(&retry_config_json).unwrap_or_default();
            let preconditions: Vec<String> = serde_json::from_str(&preconditions_json).unwrap_or_default();
            let postconditions: Vec<String> = serde_json::from_str(&postconditions_json).unwrap_or_default();
            
            steps.push(ExecutionStep {
                id,
                name,
                description,
                step_type,
                tool_config,
                parameters,
                estimated_duration: estimated_duration as u64,
                retry_config,
                preconditions,
                postconditions,
            });
        }
        
        Ok(steps)
    }

    /// 保存完整的执行计划（包含步骤）
    pub async fn save_execution_plan_with_steps(&self, plan: &crate::engines::plan_and_execute::types::ExecutionPlan) -> Result<(), RepositoryError> {
        // 先保存计划
        self.save_execution_plan_internal(plan).await?;
        
        // 然后保存所有步骤
        for step in &plan.steps {
            self.save_execution_step(&plan.id, step).await?;
        }
        
        Ok(())
    }

    /// 内部方法：保存执行计划（使用新的types）
    async fn save_execution_plan_internal(&self, plan: &crate::engines::plan_and_execute::types::ExecutionPlan) -> Result<(), RepositoryError> {
        let created_at = plan.created_at.duration_since(UNIX_EPOCH)
            .map_err(|e| RepositoryError::TimeError(format!("时间转换失败: {}", e)))?
            .as_secs() as i64;
        
        let metadata_json = serde_json::to_string(&plan.metadata)
            .map_err(|e| RepositoryError::SerializationError(format!("元数据序列化失败: {}", e)))?;
        
        let dependencies_json = serde_json::to_string(&plan.dependencies)
            .map_err(|e| RepositoryError::SerializationError(format!("依赖关系序列化失败: {}", e)))?;
        
        sqlx::query(
            "INSERT OR REPLACE INTO execution_plans (id, task_id, name, description, estimated_duration, created_at, metadata, dependencies) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )
        .bind(&plan.id)
        .bind(&plan.task_id)
        .bind(&plan.name)
        .bind(&plan.description)
        .bind(plan.estimated_duration as i64)
        .bind(created_at)
        .bind(metadata_json)
        .bind(dependencies_json)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("保存执行计划失败: {}", e)))?;
        
        Ok(())
    }

    /// 获取完整的执行计划（包含步骤）
    pub async fn get_execution_plan_with_steps(&self, plan_id: &str) -> Result<Option<crate::engines::plan_and_execute::types::ExecutionPlan>, RepositoryError> {
        use crate::engines::plan_and_execute::types::ExecutionPlan;
        
        let row = sqlx::query(
            "SELECT id, task_id, name, description, estimated_duration, created_at, metadata, dependencies 
             FROM execution_plans WHERE id = ?1"
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询执行计划失败: {}", e)))?;
        
        if let Some(row) = row {
            let id: String = row.get("id");
            let task_id: String = row.try_get("task_id").unwrap_or_default();
            let name: String = row.get("name");
            let description: String = row.get("description");
            let estimated_duration: i64 = row.get("estimated_duration");
            let created_at: i64 = row.get("created_at");
            let metadata_json: String = row.get("metadata");
            let dependencies_json: Option<String> = row.try_get("dependencies").ok();
            
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap_or_default();
            let dependencies: HashMap<String, Vec<String>> = dependencies_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();
            
            // 加载步骤
            let steps = self.get_execution_steps(&id).await?;
            
            let plan = ExecutionPlan {
                id,
                task_id,
                name,
                description,
                steps,
                estimated_duration: estimated_duration as u64,
                created_at: UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64),
                metadata,
                dependencies,
            };
            
            Ok(Some(plan))
        } else {
            Ok(None)
        }
    }

    // ===== 步骤结果持久化 =====

    /// 保存步骤执行结果
    pub async fn save_step_result(&self, session_id: &str, step_id: &str, result: &crate::engines::plan_and_execute::executor::StepResult) -> Result<(), RepositoryError> {
        let started_at = result.started_at.duration_since(UNIX_EPOCH)
            .map_err(|e| RepositoryError::TimeError(format!("时间转换失败: {}", e)))?
            .as_secs() as i64;
        
        let completed_at = result.completed_at.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
        
        let result_data_json = serde_json::to_string(&result.result_data)
            .map_err(|e| RepositoryError::SerializationError(format!("结果数据序列化失败: {}", e)))?;
        
        let status_str = format!("{:?}", result.status);
        
        sqlx::query(
            "INSERT OR REPLACE INTO step_results 
             (id, session_id, step_id, status, started_at, completed_at, duration_ms, result_data, error, retry_count) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )
        .bind(format!("{}_{}", session_id, step_id))
        .bind(session_id)
        .bind(step_id)
        .bind(status_str)
        .bind(started_at)
        .bind(completed_at)
        .bind(result.duration_ms as i64)
        .bind(result_data_json)
        .bind(&result.error)
        .bind(result.retry_count as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("保存步骤结果失败: {}", e)))?;
        
        Ok(())
    }

    /// 获取会话的所有步骤结果
    pub async fn get_step_results(&self, session_id: &str) -> Result<Vec<crate::engines::plan_and_execute::executor::StepResult>, RepositoryError> {
        use crate::engines::plan_and_execute::executor::StepResult;
        use crate::engines::StepExecutionStatus;
        
        let rows = sqlx::query(
            "SELECT step_id, status, started_at, completed_at, duration_ms, result_data, error, retry_count 
             FROM step_results WHERE session_id = ?1 ORDER BY started_at"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询步骤结果失败: {}", e)))?;
        
        let mut results = Vec::new();
        for row in rows {
            let step_id: String = row.get("step_id");
            let status_str: String = row.get("status");
            let started_at: i64 = row.get("started_at");
            let completed_at: Option<i64> = row.get("completed_at");
            let duration_ms: i64 = row.get("duration_ms");
            let result_data_json: String = row.get("result_data");
            let error: Option<String> = row.get("error");
            let retry_count: i32 = row.get("retry_count");
            
            let status = match status_str.as_str() {
                "Completed" => StepExecutionStatus::Completed,
                "Failed" => StepExecutionStatus::Failed,
                "Skipped" => StepExecutionStatus::Skipped,
                "Running" => StepExecutionStatus::Running,
                "Pending" => StepExecutionStatus::Pending,
                "Retrying" => StepExecutionStatus::Retrying,
                "Cancelled" => StepExecutionStatus::Cancelled,
                _ => StepExecutionStatus::Pending,
            };
            
            let result_data: Option<serde_json::Value> = serde_json::from_str(&result_data_json).ok();
            
            results.push(StepResult {
                step_id,
                status,
                started_at: UNIX_EPOCH + std::time::Duration::from_secs(started_at as u64),
                completed_at: completed_at.map(|t| UNIX_EPOCH + std::time::Duration::from_secs(t as u64)),
                duration_ms: duration_ms as u64,
                result_data,
                error,
                retry_count: retry_count as u32,
                tool_result: None,
            });
        }
        
        Ok(results)
    }

    // ===== 执行历史查询 =====

    /// 获取任务的执行历史
    pub async fn get_task_execution_history(&self, task_id: &str, limit: usize) -> Result<Vec<TaskExecutionRecord>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT es.id, es.plan_id, es.status, es.started_at, es.completed_at, es.progress,
                    ep.name as plan_name
             FROM execution_sessions es
             LEFT JOIN execution_plans ep ON es.plan_id = ep.id
             WHERE ep.task_id = ?1
             ORDER BY es.started_at DESC
             LIMIT ?2"
        )
        .bind(task_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("查询任务执行历史失败: {}", e)))?;
        
        let mut records = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let plan_id: String = row.get("plan_id");
            let status_str: String = row.get("status");
            let started_at: i64 = row.get("started_at");
            let completed_at: Option<i64> = row.get("completed_at");
            let progress: u32 = row.get("progress");
            let plan_name: Option<String> = row.try_get("plan_name").ok();
            
            records.push(TaskExecutionRecord {
                session_id: id,
                plan_id,
                plan_name: plan_name.unwrap_or_default(),
                status: status_str,
                started_at: UNIX_EPOCH + std::time::Duration::from_secs(started_at as u64),
                completed_at: completed_at.map(|t| UNIX_EPOCH + std::time::Duration::from_secs(t as u64)),
                progress,
            });
        }
        
        Ok(records)
    }

    /// 清理过期的执行记录
    pub async fn cleanup_old_records(&self, retention_days: u32) -> Result<CleanupResult, RepositoryError> {
        let cutoff_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RepositoryError::TimeError(e.to_string()))?
            .as_secs() as i64 - (retention_days as i64 * 24 * 60 * 60);
        
        // 删除旧的步骤结果
        let step_results_deleted = sqlx::query(
            "DELETE FROM step_results WHERE started_at < ?1"
        )
        .bind(cutoff_time)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("清理步骤结果失败: {}", e)))?
        .rows_affected();
        
        // 删除旧的执行会话
        let sessions_deleted = sqlx::query(
            "DELETE FROM execution_sessions WHERE started_at < ?1"
        )
        .bind(cutoff_time)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("清理执行会话失败: {}", e)))?
        .rows_affected();
        
        Ok(CleanupResult {
            step_results_deleted,
            sessions_deleted,
            retention_days,
        })
    }
}

/// 任务执行记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskExecutionRecord {
    pub session_id: String,
    pub plan_id: String,
    pub plan_name: String,
    pub status: String,
    pub started_at: std::time::SystemTime,
    pub completed_at: Option<std::time::SystemTime>,
    pub progress: u32,
}

/// 清理结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CleanupResult {
    pub step_results_deleted: u64,
    pub sessions_deleted: u64,
    pub retention_days: u32,
}
