use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use sqlx::Row;

fn is_running_tool_call_metadata(metadata: &Value) -> bool {
    metadata
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "tool_call")
        && metadata
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "running")
}

fn settle_tool_call_metadata(mut metadata: Value, terminal_status: &str, reason: &str) -> Value {
    let completed_at_ms = Utc::now().timestamp_millis();
    if let Some(object) = metadata.as_object_mut() {
        object.insert(
            "status".to_string(),
            Value::String(terminal_status.to_string()),
        );
        object.insert("success".to_string(), Value::Bool(false));
        object.insert(
            "settled_reason".to_string(),
            Value::String(reason.to_string()),
        );
        object.insert(
            "completed_at_ms".to_string(),
            Value::Number(completed_at_ms.into()),
        );

        let duration_ms = object
            .get("started_at_ms")
            .and_then(Value::as_i64)
            .map(|started_at_ms| completed_at_ms.saturating_sub(started_at_ms));
        if let Some(duration_ms) = duration_ms {
            object.insert("duration_ms".to_string(), Value::Number(duration_ms.into()));
        }

        if object
            .get("tool_result")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            object.insert("tool_result".to_string(), Value::String(reason.to_string()));
        }
    }
    metadata
}

impl DatabaseService {
    pub async fn settle_running_ai_tool_messages(
        &self,
        conversation_id: &str,
        terminal_status: &str,
        reason: &str,
    ) -> Result<u64> {
        if !matches!(terminal_status, "cancelled" | "timed_out" | "failed") {
            return Err(anyhow::anyhow!(
                "invalid terminal tool status: {}",
                terminal_status
            ));
        }

        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                "SELECT id, metadata FROM ai_messages WHERE conversation_id = $1 AND role = 'tool'",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?,
            DatabasePool::SQLite(pool) => sqlx::query(
                "SELECT id, metadata FROM ai_messages WHERE conversation_id = ? AND role = 'tool'",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?,
            DatabasePool::MySQL(pool) => sqlx::query(
                "SELECT id, metadata FROM ai_messages WHERE conversation_id = ? AND role = 'tool'",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?,
        };

        let mut updates = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let metadata_raw: Option<String> = row.get("metadata");
            let Some(metadata_raw) = metadata_raw else {
                continue;
            };
            let Ok(metadata) = serde_json::from_str::<Value>(&metadata_raw) else {
                continue;
            };
            if !is_running_tool_call_metadata(&metadata) {
                continue;
            }
            let settled = settle_tool_call_metadata(metadata, terminal_status, reason);
            updates.push((id, settled.to_string()));
        }

        for (id, metadata) in &updates {
            match runtime {
                DatabasePool::PostgreSQL(pool) => {
                    sqlx::query("UPDATE ai_messages SET metadata = $1 WHERE id = $2")
                        .bind(metadata)
                        .bind(id)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::SQLite(pool) => {
                    sqlx::query("UPDATE ai_messages SET metadata = ? WHERE id = ?")
                        .bind(metadata)
                        .bind(id)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query("UPDATE ai_messages SET metadata = ? WHERE id = ?")
                        .bind(metadata)
                        .bind(id)
                        .execute(pool)
                        .await?;
                }
            }
        }

        Ok(updates.len() as u64)
    }
}
