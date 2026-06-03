use crate::core::models::database::SubagentMessage;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;

impl DatabaseService {
    pub async fn upsert_subagent_message_internal(&self, message: &SubagentMessage) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO ai_subagent_messages (
                        id, subagent_run_id, role, content, metadata, tool_calls, attachments,
                        reasoning_content, timestamp, structured_data
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT (id) DO UPDATE SET
                        subagent_run_id = EXCLUDED.subagent_run_id,
                        role = EXCLUDED.role,
                        content = EXCLUDED.content,
                        metadata = EXCLUDED.metadata,
                        tool_calls = EXCLUDED.tool_calls,
                        attachments = EXCLUDED.attachments,
                        reasoning_content = EXCLUDED.reasoning_content,
                        timestamp = EXCLUDED.timestamp,
                        structured_data = EXCLUDED.structured_data
                    "#,
                )
                .bind(&message.id)
                .bind(&message.subagent_run_id)
                .bind(&message.role)
                .bind(&message.content)
                .bind(&message.metadata)
                .bind(&message.tool_calls)
                .bind(&message.attachments)
                .bind(&message.reasoning_content)
                .bind(message.timestamp)
                .bind(&message.structured_data)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO ai_subagent_messages (
                        id, subagent_run_id, role, content, metadata, tool_calls, attachments,
                        reasoning_content, timestamp, structured_data
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        subagent_run_id = excluded.subagent_run_id,
                        role = excluded.role,
                        content = excluded.content,
                        metadata = excluded.metadata,
                        tool_calls = excluded.tool_calls,
                        attachments = excluded.attachments,
                        reasoning_content = excluded.reasoning_content,
                        timestamp = excluded.timestamp,
                        structured_data = excluded.structured_data
                    "#,
                )
                .bind(&message.id)
                .bind(&message.subagent_run_id)
                .bind(&message.role)
                .bind(&message.content)
                .bind(&message.metadata)
                .bind(&message.tool_calls)
                .bind(&message.attachments)
                .bind(&message.reasoning_content)
                .bind(message.timestamp)
                .bind(&message.structured_data)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO ai_subagent_messages (
                        id, subagent_run_id, role, content, metadata, tool_calls, attachments,
                        reasoning_content, timestamp, structured_data
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        subagent_run_id = VALUES(subagent_run_id),
                        role = VALUES(role),
                        content = VALUES(content),
                        metadata = VALUES(metadata),
                        tool_calls = VALUES(tool_calls),
                        attachments = VALUES(attachments),
                        reasoning_content = VALUES(reasoning_content),
                        timestamp = VALUES(timestamp),
                        structured_data = VALUES(structured_data)
                    "#,
                )
                .bind(&message.id)
                .bind(&message.subagent_run_id)
                .bind(&message.role)
                .bind(&message.content)
                .bind(&message.metadata)
                .bind(&message.tool_calls)
                .bind(&message.attachments)
                .bind(&message.reasoning_content)
                .bind(message.timestamp)
                .bind(&message.structured_data)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}
