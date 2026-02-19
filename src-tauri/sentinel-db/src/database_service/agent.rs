use anyhow::Result;
use crate::core::models::agent::{AgentTask, AgentSessionData, AgentExecutionResult, SessionLog};
use crate::core::models::workflow::WorkflowStepDetail;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    // ============================================================================
    // Agent Task Operations
    // ============================================================================

    pub async fn create_agent_task_internal(&self, task: &AgentTask) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_tasks (id, user_id, description, target, parameters, priority, timeout)
                       VALUES ($1, $2, $3, $4, $5, $6, $7)"#
                )
                .bind(&task.id)
                .bind(&task.user_id)
                .bind(&task.description)
                .bind(&task.target)
                .bind(&task.parameters)
                .bind(&task.priority)
                .bind(task.timeout)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_tasks (id, user_id, description, target, parameters, priority, timeout)
                       VALUES (?, ?, ?, ?, ?, ?, ?)"#
                )
                .bind(&task.id)
                .bind(&task.user_id)
                .bind(&task.description)
                .bind(&task.target)
                .bind(&task.parameters)
                .bind(&task.priority)
                .bind(task.timeout)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_tasks (id, user_id, description, target, parameters, priority, timeout)
                       VALUES (?, ?, ?, ?, ?, ?, ?)"#
                )
                .bind(&task.id)
                .bind(&task.user_id)
                .bind(&task.description)
                .bind(&task.target)
                .bind(&task.parameters)
                .bind(&task.priority)
                .bind(task.timeout)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_task_internal(&self, id: &str) -> Result<Option<AgentTask>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row)
    }

    pub async fn get_agent_tasks_internal(&self, user_id: Option<&str>) -> Result<Vec<AgentTask>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(uid) = user_id {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE user_id = $1 ORDER BY id DESC")
                        .bind(uid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks ORDER BY id DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(uid) = user_id {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE user_id = ? ORDER BY id DESC")
                        .bind(uid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks ORDER BY id DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(uid) = user_id {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks WHERE user_id = ? ORDER BY id DESC")
                        .bind(uid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, AgentTask>("SELECT * FROM agent_tasks ORDER BY id DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
        };
        Ok(rows)
    }

    pub async fn update_agent_task_status_internal(&self, id: &str, status: &str, _agent_name: Option<&str>, _architecture: Option<&str>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE agent_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_agent_task_timing_internal(&self, id: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, execution_time_ms: Option<u64>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET started_at = $1, completed_at = $2, execution_time_ms = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4")
                    .bind(started_at)
                    .bind(completed_at)
                    .bind(execution_time_ms.map(|ms| ms as i64))
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE agent_tasks SET started_at = ?, completed_at = ?, execution_time_ms = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(started_at)
                    .bind(completed_at)
                    .bind(execution_time_ms.map(|ms| ms as i64))
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET started_at = ?, completed_at = ?, execution_time_ms = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(started_at)
                    .bind(completed_at)
                    .bind(execution_time_ms.map(|ms| ms as i64))
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_agent_task_error_internal(&self, id: &str, error_message: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET error_message = $1, status = 'error', updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(error_message)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE agent_tasks SET error_message = ?, status = 'error', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(error_message)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE agent_tasks SET error_message = ?, status = 'error', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(error_message)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    // ============================================================================
    // Agent Session Operations
    // ============================================================================

    pub async fn create_agent_session_internal(&self, session_id: &str, task_id: &str, agent_name: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_sessions (id, task_id, agent_name, status) VALUES ($1, $2, $3, 'active')"
                )
                .bind(session_id)
                .bind(task_id)
                .bind(agent_name)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_sessions (id, task_id, agent_name, status) VALUES (?, ?, ?, 'active')"
                )
                .bind(session_id)
                .bind(task_id)
                .bind(agent_name)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_sessions (id, task_id, agent_name, status) VALUES (?, ?, ?, 'active')"
                )
                .bind(session_id)
                .bind(task_id)
                .bind(agent_name)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_agent_session_status_internal(&self, session_id: &str, status: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE agent_sessions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(status)
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE agent_sessions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE agent_sessions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_session_internal(&self, session_id: &str) -> Result<Option<AgentSessionData>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions WHERE id = $1")
                    .bind(session_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions WHERE id = ?")
                    .bind(session_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions WHERE id = ?")
                    .bind(session_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row)
    }

    pub async fn list_agent_sessions_internal(&self) -> Result<Vec<AgentSessionData>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions ORDER BY created_at DESC")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions ORDER BY created_at DESC")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentSessionData>("SELECT * FROM agent_sessions ORDER BY created_at DESC")
                    .fetch_all(pool)
                    .await?
            }
        };
        Ok(rows)
    }

    pub async fn delete_agent_session_internal(&self, session_id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM agent_sessions WHERE id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM agent_sessions WHERE id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    // ============================================================================
    // Agent Log & Result Operations
    // ============================================================================

    pub async fn add_agent_session_log_internal(&self, session_id: &str, level: &str, message: &str, source: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_session_logs (id, session_id, level, message, source) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(level)
                .bind(message)
                .bind(source)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_session_logs (id, session_id, level, message, source) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(level)
                .bind(message)
                .bind(source)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_session_logs (id, session_id, level, message, source) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(level)
                .bind(message)
                .bind(source)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_session_logs_internal(&self, session_id: &str) -> Result<Vec<SessionLog>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SessionLog>(
                    "SELECT * FROM agent_session_logs WHERE session_id = $1 ORDER BY created_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SessionLog>(
                    "SELECT * FROM agent_session_logs WHERE session_id = ? ORDER BY created_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SessionLog>(
                    "SELECT * FROM agent_session_logs WHERE session_id = ? ORDER BY created_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows)
    }

    pub async fn save_agent_execution_result_internal(&self, session_id: &str, result: &AgentExecutionResult) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_results (id, session_id, success, data, error) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(&result.id)
                .bind(session_id)
                .bind(result.success)
                .bind(&result.data)
                .bind(&result.error)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_results (id, session_id, success, data, error) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&result.id)
                .bind(session_id)
                .bind(result.success)
                .bind(&result.data)
                .bind(&result.error)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_results (id, session_id, success, data, error) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&result.id)
                .bind(session_id)
                .bind(result.success)
                .bind(&result.data)
                .bind(&result.error)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_execution_result_internal(&self, session_id: &str) -> Result<Option<AgentExecutionResult>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentExecutionResult>(
                    "SELECT * FROM agent_execution_results WHERE session_id = $1"
                )
                .bind(session_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentExecutionResult>(
                    "SELECT * FROM agent_execution_results WHERE session_id = ?"
                )
                .bind(session_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentExecutionResult>(
                    "SELECT * FROM agent_execution_results WHERE session_id = ?"
                )
                .bind(session_id)
                .fetch_optional(pool)
                .await?
            }
        };
        Ok(row)
    }

    // ============================================================================
    // Agent Step Operations
    // ============================================================================

    pub async fn save_agent_execution_step_internal(&self, session_id: &str, step: &WorkflowStepDetail) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_steps (step_id, session_id, step_name, status, result_data) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(&step.step_id)
                .bind(session_id)
                .bind(&step.step_name)
                .bind(&step.status)
                .bind(&step.result_data)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_steps (step_id, session_id, step_name, status, result_data) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&step.step_id)
                .bind(session_id)
                .bind(&step.step_name)
                .bind(&step.status)
                .bind(&step.result_data)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_execution_steps (step_id, session_id, step_name, status, result_data) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&step.step_id)
                .bind(session_id)
                .bind(&step.step_name)
                .bind(&step.status)
                .bind(&step.result_data)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_execution_steps_internal(&self, session_id: &str) -> Result<Vec<WorkflowStepDetail>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, WorkflowStepDetail>(
                    "SELECT * FROM agent_execution_steps WHERE session_id = $1 ORDER BY started_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, WorkflowStepDetail>(
                    "SELECT * FROM agent_execution_steps WHERE session_id = ? ORDER BY started_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, WorkflowStepDetail>(
                    "SELECT * FROM agent_execution_steps WHERE session_id = ? ORDER BY started_at ASC"
                )
                .bind(session_id)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows)
    }

    pub async fn update_agent_execution_step_status_internal(&self, step_id: &str, status: &str, started_at: Option<chrono::DateTime<chrono::Utc>>, completed_at: Option<chrono::DateTime<chrono::Utc>>, duration_ms: Option<u64>, error_message: Option<&str>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_steps SET status = $1, started_at = $2, completed_at = $3, duration_ms = $4, error_message = $5 WHERE step_id = $6"
                )
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(duration_ms.map(|ms| ms as i64))
                .bind(error_message)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_steps SET status = ?, started_at = ?, completed_at = ?, duration_ms = ?, error_message = ? WHERE step_id = ?"
                )
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(duration_ms.map(|ms| ms as i64))
                .bind(error_message)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_steps SET status = ?, started_at = ?, completed_at = ?, duration_ms = ?, error_message = ? WHERE step_id = ?"
                )
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(duration_ms.map(|ms| ms as i64))
                .bind(error_message)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn delete_agent_execution_steps_internal(&self, session_id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM agent_execution_steps WHERE session_id = $1")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM agent_execution_steps WHERE session_id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM agent_execution_steps WHERE session_id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }
}
