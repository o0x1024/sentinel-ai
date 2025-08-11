//! Plan-and-Execute架构数据库访问层
//! 
//! 这个模块提供了Plan-and-Execute架构的数据持久化功能，
//! 包括执行计划、步骤、会话、结果、指标等数据的CRUD操作。

use crate::engines::types::*;
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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
    pub average_execution_time: f64,
}

/// Plan-and-Execute数据库仓库
pub struct PlanExecuteRepository {
    /// 数据库连接池
    pool: SqlitePool,
}

impl PlanExecuteRepository {
    /// 创建新的仓库实例
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    /// 运行数据库迁移
    pub async fn run_migrations(&self) -> Result<(), RepositoryError> {
        // 读取并执行迁移SQL
        let migration_sql = include_str!("../../migrations/20241220_plan_execute_architecture.sql");
        
        // 将SQL按语句分割并逐个执行
        let statements: Vec<&str> = migration_sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .collect();
        
        for statement in statements {
            if !statement.is_empty() {
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| RepositoryError::DatabaseError(format!("迁移失败: {} - SQL: {}", e, statement)))?;
            }
        }
        
        Ok(())
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
            "SELECT id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata 
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
            let progress: f32 = row.get("progress");
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
            "SELECT id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata 
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
            let progress: f32 = row.get("progress");
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
            sqlx::query_scalar::<_, Option<f64>>(
                "SELECT AVG(CAST((julianday(datetime(completed_at, 'unixepoch')) - julianday(datetime(started_at, 'unixepoch'))) * 86400 AS REAL)) 
                 FROM execution_sessions 
                 WHERE status = 'Completed' AND completed_at IS NOT NULL"
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("查询平均执行时间失败: {}", e)))?
            .unwrap_or(0.0)
        } else {
            0.0
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
}