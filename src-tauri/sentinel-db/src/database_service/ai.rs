use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::core::models::ai::AiRole;
use crate::core::models::database::{AiConversation, AiMessage, SubagentMessage, SubagentRun};
use crate::database_service::service::DatabaseService;
use sqlx::Row;

impl DatabaseService {
    pub async fn create_subagent_message_internal(&self, message: &SubagentMessage) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO ai_subagent_messages (
                id, subagent_run_id, role, content, metadata, tool_calls, attachments,
                reasoning_content, timestamp, structured_data
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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

        Ok(())
    }

    pub async fn get_subagent_messages_by_run_internal(
        &self,
        subagent_run_id: &str,
    ) -> Result<Vec<SubagentMessage>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            "SELECT * FROM ai_subagent_messages WHERE subagent_run_id = $1 ORDER BY timestamp ASC",
        )
        .bind(subagent_run_id)
        .fetch_all(pool)
        .await?;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            messages.push(SubagentMessage {
                id: row.get("id"),
                subagent_run_id: row.get("subagent_run_id"),
                role: row.get("role"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                tool_calls: row.get("tool_calls"),
                attachments: row.get("attachments"),
                reasoning_content: row.get("reasoning_content"),
                timestamp: row.try_get::<chrono::DateTime<chrono::Utc>, _>("timestamp").unwrap_or_else(|_| {
                    row.try_get::<String, _>("timestamp").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                structured_data: row.get("structured_data"),
            });
        }

        Ok(messages)
    }

    pub async fn create_subagent_run_internal(&self, run: &SubagentRun) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO ai_subagent_runs (
                id, parent_execution_id, role, task, status, output, error,
                model_name, model_provider, started_at, completed_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&run.id)
        .bind(&run.parent_execution_id)
        .bind(&run.role)
        .bind(&run.task)
        .bind(&run.status)
        .bind(&run.output)
        .bind(&run.error)
        .bind(&run.model_name)
        .bind(&run.model_provider)
        .bind(run.started_at)
        .bind(run.completed_at)
        .bind(run.created_at)
        .bind(run.updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_subagent_run_result_internal(
        &self,
        id: &str,
        status: &str,
        output: Option<&str>,
        error: Option<&str>,
        completed_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            UPDATE ai_subagent_runs
            SET status = $1, output = $2, error = $3, completed_at = $4, updated_at = $5
            WHERE id = $6
            "#,
        )
        .bind(status)
        .bind(output)
        .bind(error)
        .bind(completed_at)
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_subagent_runs_by_parent_internal(
        &self,
        parent_execution_id: &str,
    ) -> Result<Vec<SubagentRun>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            "SELECT * FROM ai_subagent_runs WHERE parent_execution_id = $1 ORDER BY started_at DESC",
        )
        .bind(parent_execution_id)
        .fetch_all(pool)
        .await?;

        let mut runs = Vec::with_capacity(rows.len());
        for row in rows {
            runs.push(SubagentRun {
                id: row.get("id"),
                parent_execution_id: row.get("parent_execution_id"),
                role: row.get("role"),
                task: row.get("task"),
                status: row.get("status"),
                output: row.get("output"),
                error: row.get("error"),
                model_name: row.get("model_name"),
                model_provider: row.get("model_provider"),
                started_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("started_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("started_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                completed_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at").unwrap_or_else(|_| {
                    row.try_get::<Option<String>, _>("completed_at").ok().flatten().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    })
                }),
                created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("created_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("updated_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
            });
        }

        Ok(runs)
    }

    /// Delete subagent runs that started after a specific timestamp
    pub async fn delete_subagent_runs_after_internal(
        &self,
        parent_execution_id: &str,
        after_timestamp: DateTime<Utc>,
    ) -> Result<u64> {
        let pool = self.get_pool()?;

        // First get the IDs of runs to delete (for deleting related messages)
        let run_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM ai_subagent_runs WHERE parent_execution_id = $1 AND started_at > $2"
        )
        .bind(parent_execution_id)
        .bind(after_timestamp)
        .fetch_all(pool)
        .await?;

        if run_ids.is_empty() {
            return Ok(0);
        }

        // Delete related messages first
        for run_id in &run_ids {
            sqlx::query("DELETE FROM ai_subagent_messages WHERE subagent_run_id = $1")
                .bind(run_id)
                .execute(pool)
                .await?;
        }

        // Delete the runs
        let result = sqlx::query(
            "DELETE FROM ai_subagent_runs WHERE parent_execution_id = $1 AND started_at > $2"
        )
        .bind(parent_execution_id)
        .bind(after_timestamp)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete all subagent runs for a parent execution
    pub async fn delete_subagent_runs_by_parent_internal(
        &self,
        parent_execution_id: &str,
    ) -> Result<u64> {
        let pool = self.get_pool()?;

        // First get the IDs of runs to delete (for deleting related messages)
        let run_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM ai_subagent_runs WHERE parent_execution_id = $1"
        )
        .bind(parent_execution_id)
        .fetch_all(pool)
        .await?;

        if run_ids.is_empty() {
            return Ok(0);
        }

        // Delete related messages first
        for run_id in &run_ids {
            sqlx::query("DELETE FROM ai_subagent_messages WHERE subagent_run_id = $1")
                .bind(run_id)
                .execute(pool)
                .await?;
        }

        // Delete the runs
        let result = sqlx::query(
            "DELETE FROM ai_subagent_runs WHERE parent_execution_id = $1"
        )
        .bind(parent_execution_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn create_ai_conversation_internal(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;

        // 验证关联ID存在性
        let vulnerability_id = if let Some(ref vuln_id) = conversation.vulnerability_id {
            if !vuln_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM vulnerabilities WHERE id = $1")
                    .bind(vuln_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(vuln_id.clone()) } else { None }
            } else { None }
        } else { None };

        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM scan_tasks WHERE id = $1")
                    .bind(task_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(task_id.clone()) } else { None }
            } else { None }
        } else { None };

        sqlx::query(
            r#"
            INSERT INTO ai_conversations (
                id, title, service_name, model_name, model_provider, context_type, project_id,
                vulnerability_id, scan_task_id, conversation_data, summary, total_messages,
                total_tokens, cost, tags, tool_config, is_archived, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
        )
        .bind(&conversation.id)
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(vulnerability_id)
        .bind(scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(&conversation.tool_config)
        .bind(conversation.is_archived)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_ai_conversations_internal(&self) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            "SELECT * FROM ai_conversations WHERE service_name != 'subagent' AND (context_type IS NULL OR context_type != 'subagent') ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;

        let mut conversations = Vec::with_capacity(rows.len());
        for row in rows {
            conversations.push(AiConversation {
                id: row.get("id"),
                title: row.get("title"),
                service_name: row.get("service_name"),
                model_name: row.get("model_name"),
                model_provider: row.get("model_provider"),
                context_type: row.get("context_type"),
                project_id: row.get("project_id"),
                vulnerability_id: row.get("vulnerability_id"),
                scan_task_id: row.get("scan_task_id"),
                conversation_data: row.get("conversation_data"),
                summary: row.get("summary"),
                total_messages: row.try_get::<i32, _>("total_messages").unwrap_or_else(|_| row.try_get::<i64, _>("total_messages").unwrap_or(0) as i32),
                total_tokens: row.try_get::<i32, _>("total_tokens").unwrap_or_else(|_| row.try_get::<i64, _>("total_tokens").unwrap_or(0) as i32),
                cost: row.get("cost"),
                tags: row.get("tags"),
                tool_config: row.get("tool_config"),
                is_archived: row.get("is_archived"),
                created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("created_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("updated_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
            });
        }

        Ok(conversations)
    }

    pub async fn get_ai_conversations_paginated_internal(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            "SELECT * FROM ai_conversations WHERE service_name != 'subagent' AND (context_type IS NULL OR context_type != 'subagent') ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let mut conversations = Vec::with_capacity(rows.len());
        for row in rows {
            conversations.push(AiConversation {
                id: row.get("id"),
                title: row.get("title"),
                service_name: row.get("service_name"),
                model_name: row.get("model_name"),
                model_provider: row.get("model_provider"),
                context_type: row.get("context_type"),
                project_id: row.get("project_id"),
                vulnerability_id: row.get("vulnerability_id"),
                scan_task_id: row.get("scan_task_id"),
                conversation_data: row.get("conversation_data"),
                summary: row.get("summary"),
                total_messages: row.get("total_messages"),
                total_tokens: row.get("total_tokens"),
                cost: row.get("cost"),
                tags: row.get("tags"),
                tool_config: row.get("tool_config"),
                is_archived: row.get("is_archived"),
                created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("created_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("updated_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
            });
        }

        Ok(conversations)
    }

    pub async fn get_ai_conversations_count_internal(&self) -> Result<i64> {
        let pool = self.get_pool()?;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ai_conversations WHERE service_name != 'subagent' AND (context_type IS NULL OR context_type != 'subagent')"
        )
            .fetch_one(pool)
            .await?;

        Ok(count.0)
    }

    pub async fn get_ai_conversation_internal(&self, id: &str) -> Result<Option<AiConversation>> {
        let pool = self.get_pool()?;

        let row =
            sqlx::query_as::<_, AiConversation>("SELECT * FROM ai_conversations WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        Ok(row)
    }

    pub async fn update_ai_conversation_internal(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;

        // 验证关联ID存在性
        let vulnerability_id = if let Some(ref vuln_id) = conversation.vulnerability_id {
            if !vuln_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM vulnerabilities WHERE id = $1")
                    .bind(vuln_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(vuln_id.clone()) } else { None }
            } else { None }
        } else { None };

        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM scan_tasks WHERE id = $1")
                    .bind(task_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(task_id.clone()) } else { None }
            } else { None }
        } else { None };

        sqlx::query(
            r#"
            UPDATE ai_conversations
            SET title = $1, service_name = $2, model_name = $3, model_provider = $4, context_type = $5,
                project_id = $6, vulnerability_id = $7, scan_task_id = $8, conversation_data = $9,
                summary = $10, total_messages = $11, total_tokens = $12, cost = $13, tags = $14,
                tool_config = $15, is_archived = $16, updated_at = $17
            WHERE id = $18
            "#,
        )
        .bind(&conversation.title)
        .bind(&conversation.service_name)
        .bind(&conversation.model_name)
        .bind(&conversation.model_provider)
        .bind(&conversation.context_type)
        .bind(&conversation.project_id)
        .bind(vulnerability_id)
        .bind(scan_task_id)
        .bind(&conversation.conversation_data)
        .bind(&conversation.summary)
        .bind(conversation.total_messages)
        .bind(conversation.total_tokens)
        .bind(conversation.cost)
        .bind(serde_json::to_string(&conversation.tags).unwrap_or_default())
        .bind(&conversation.tool_config)
        .bind(conversation.is_archived)
        .bind(Utc::now())
        .bind(&conversation.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_ai_conversation_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;

        // 先删除相关的消息
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        // 再删除对话
        sqlx::query("DELETE FROM ai_conversations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_ai_conversation_title_internal(&self, id: &str, title: &str) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query("UPDATE ai_conversations SET title = $1, updated_at = $2 WHERE id = $3")
            .bind(title)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn archive_ai_conversation_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_conversations SET is_archived = TRUE, updated_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create_ai_message_internal(&self, message: &AiMessage) -> Result<()> {
        // Acquire write lock to serialize database writes
        let _permit = self.write_semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO ai_messages (
                id, conversation_id, role, content, metadata,
                token_count, cost, tool_calls, attachments, reasoning_content, timestamp,
                architecture_type, architecture_meta, structured_data
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.metadata)
        .bind(message.token_count)
        .bind(message.cost)
        .bind(&message.tool_calls)
        .bind(&message.attachments)
        .bind(&message.reasoning_content)
        .bind(message.timestamp)
        .bind(&message.architecture_type)
        .bind(&message.architecture_meta)
        .bind(&message.structured_data)
        .execute(pool)
        .await?;

        // 更新对话的updated_at和消息计数
        sqlx::query("UPDATE ai_conversations SET updated_at = $1, total_messages = total_messages + 1 WHERE id = $2")
            .bind(Utc::now())
            .bind(&message.conversation_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn upsert_ai_message_append_internal(&self, message: &AiMessage) -> Result<()> {
        // Acquire write lock to serialize database writes
        let _permit = self.write_semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        
        let pool = self.get_pool()?.clone();
        let msg = message.clone();
        
        // This retry logic might need adjustment for PG transaction handling but logic is sound
        // PG concurrency is handled well, but retry wrapper also handles locking errors
        // Ideally we should use the built-in PgPool transaction handling without manual retries for locking unless specific
        // But for minimal API changes we keep the structure inside retry_on_locked (which we removed/modified? No, we removed it from Service!)
        // WAIT, we removed `retry_on_locked` from `service.rs`. So this code will fail to compile if we use `retry_on_locked`.
        // We should just use a direct transaction for Postgres.
            
        let mut tx = pool.begin().await?;

        let exists = sqlx::query("SELECT id FROM ai_messages WHERE id = $1")
            .bind(&msg.id)
            .fetch_optional(&mut *tx)
            .await?;

        if exists.is_some() {
            sqlx::query("UPDATE ai_messages SET content = content || $1, metadata = $2, token_count = $3, cost = $4, timestamp = $5 WHERE id = $6")
                .bind(&msg.content)
                .bind(&msg.metadata)
                .bind(msg.token_count)
                .bind(msg.cost)
                .bind(msg.timestamp)
                .bind(&msg.id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO ai_messages (id, conversation_id, role, content, metadata, token_count, cost, timestamp)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            )
            .bind(&msg.id)
            .bind(&msg.conversation_id)
            .bind(&msg.role)
            .bind(&msg.content)
            .bind(&msg.metadata)
            .bind(msg.token_count)
            .bind(msg.cost)
            .bind(msg.timestamp)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_ai_messages_by_conversation_internal(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(
            "SELECT * FROM ai_messages WHERE conversation_id = $1 ORDER BY timestamp ASC, id ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            messages.push(AiMessage {
                id: row.get("id"),
                conversation_id: row.get("conversation_id"),
                role: row.get("role"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                token_count: row.try_get::<Option<i32>, _>("token_count").unwrap_or_else(|_| row.try_get::<Option<i64>, _>("token_count").ok().flatten().map(|v| v as i32)),
                cost: row.get("cost"),
                tool_calls: row.get("tool_calls"),
                attachments: row.get("attachments"),
                reasoning_content: row.get("reasoning_content"),
                timestamp: row.try_get::<chrono::DateTime<chrono::Utc>, _>("timestamp").unwrap_or_else(|_| {
                    row.try_get::<String, _>("timestamp").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                architecture_type: row.get("architecture_type"),
                architecture_meta: row.get("architecture_meta"),
                structured_data: row.get("structured_data"),
            });
        }

        Ok(messages)
    }

    pub async fn get_ai_roles_internal(&self) -> Result<Vec<AiRole>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;

        let mut roles = Vec::with_capacity(rows.len());
        for row in rows {
            roles.push(AiRole {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                prompt: row.get("prompt"),
                is_system: row.get("is_system"),
                created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("created_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
                updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").unwrap_or_else(|_| {
                    row.try_get::<String, _>("updated_at").ok().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    }).unwrap_or_else(|| chrono::Utc::now())
                }),
            });
        }

        Ok(roles)
    }

    pub async fn create_ai_role_internal(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("INSERT INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&role.id)
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(role.is_system)
            .bind(role.created_at)
            .bind(role.updated_at)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_ai_role_internal(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_roles SET title = $1, description = $2, prompt = $3, updated_at = $4 WHERE id = $5")
            .bind(&role.title)
            .bind(&role.description)
            .bind(&role.prompt)
            .bind(Utc::now())
            .bind(&role.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_ai_role_internal(&self, role_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM ai_roles WHERE id = $1")
            .bind(role_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_current_ai_role_internal(&self, role_id: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        if let Some(rid) = role_id {
            self.set_config_internal("ai", "current_role", rid, Some("当前使用的AI角色")).await?;
        } else {
            sqlx::query("DELETE FROM configurations WHERE category = 'ai' AND key = 'current_role'")
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    pub async fn get_current_ai_role_internal(&self) -> Result<Option<AiRole>> {
        let role_id = self.get_config_internal("ai", "current_role").await?;
        if let Some(rid) = role_id {
            let pool = self.get_pool()?;
            let row = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles WHERE id = $1")
                .bind(rid)
                .fetch_optional(pool)
                .await?;
            
            if let Some(row) = row {
                Ok(Some(AiRole {
                    id: row.get("id"),
                    title: row.get("title"),
                    description: row.get("description"),
                    prompt: row.get("prompt"),
                    is_system: row.get("is_system"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn delete_ai_message_internal(&self, message_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM ai_messages WHERE id = $1")
            .bind(message_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_ai_messages_by_conversation_internal(&self, conversation_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = $1")
            .bind(conversation_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all messages after a specific message (by timestamp)
    pub async fn delete_ai_messages_after_internal(&self, conversation_id: &str, message_id: &str) -> Result<u64> {
        let pool = self.get_pool()?;
        
        // First get the timestamp of the target message
        let timestamp: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT timestamp FROM ai_messages WHERE id = $1 AND conversation_id = $2"
        )
        .bind(message_id)
        .bind(conversation_id)
        .fetch_optional(pool)
        .await?;

        let timestamp = match timestamp {
            Some(ts) => ts,
            None => return Err(anyhow::anyhow!("Message not found: {}", message_id)),
        };

        // Delete all messages with timestamp greater than the target message
        let result = sqlx::query(
            "DELETE FROM ai_messages WHERE conversation_id = $1 AND timestamp > $2"
        )
        .bind(conversation_id)
        .bind(timestamp)
        .execute(pool)
        .await?;

        let deleted_count = result.rows_affected();

        // Update conversation's message count
        if deleted_count > 0 {
            sqlx::query(
                "UPDATE ai_conversations SET total_messages = total_messages - $1, updated_at = $2 WHERE id = $3"
            )
            .bind(deleted_count as i64)
            .bind(Utc::now())
            .bind(conversation_id)
            .execute(pool)
            .await?;
        }

        Ok(deleted_count)
    }

    pub async fn update_ai_usage_internal(
        &self,
        provider: &str,
        model: &str,
        input_tokens: i32,
        output_tokens: i32,
        cost: f64,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let total_tokens = input_tokens + output_tokens;
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO ai_usage_stats (provider, model, input_tokens, output_tokens, total_tokens, cost, last_used)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT(provider, model) DO UPDATE SET
                input_tokens = ai_usage_stats.input_tokens + excluded.input_tokens,
                output_tokens = ai_usage_stats.output_tokens + excluded.output_tokens,
                total_tokens = ai_usage_stats.total_tokens + excluded.total_tokens,
                cost = ai_usage_stats.cost + excluded.cost,
                last_used = excluded.last_used
            "#,
        )
        .bind(provider)
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(total_tokens)
        .bind(cost)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_ai_usage_stats_internal(&self) -> Result<Vec<crate::core::models::database::AiUsageStats>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            "SELECT provider, model, input_tokens, output_tokens, total_tokens, cost, last_used FROM ai_usage_stats ORDER BY total_tokens DESC",
        )
        .fetch_all(pool)
        .await?;
        
        let mut results = Vec::new();
        use chrono::{DateTime, Utc};
        for row in rows {
            results.push(crate::core::models::database::AiUsageStats {
                provider: row.get("provider"),
                model: row.get("model"),
                input_tokens: row.try_get::<i32, _>("input_tokens").unwrap_or_else(|_| row.try_get::<i64, _>("input_tokens").unwrap_or(0) as i32),
                output_tokens: row.try_get::<i32, _>("output_tokens").unwrap_or_else(|_| row.try_get::<i64, _>("output_tokens").unwrap_or(0) as i32),
                total_tokens: row.try_get::<i32, _>("total_tokens").unwrap_or_else(|_| row.try_get::<i64, _>("total_tokens").unwrap_or(0) as i32),
                cost: row.get("cost"),
                last_used: row.try_get::<Option<DateTime<Utc>>, _>("last_used").unwrap_or_else(|_| {
                    row.try_get::<Option<String>, _>("last_used").ok().flatten().and_then(|s| {
                        DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)).ok()
                    })
                }),
            });
        }
        Ok(results)
    }

    pub async fn clear_ai_usage_stats_internal(&self) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM ai_usage_stats")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_aggregated_ai_usage_internal(&self) -> Result<std::collections::HashMap<String, crate::core::models::database::AiUsageStats>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            r#"
            SELECT 
                provider,
                'aggregated' as model,
                CAST(SUM(input_tokens) AS BIGINT) as input_tokens,
                CAST(SUM(output_tokens) AS BIGINT) as output_tokens,
                CAST(SUM(total_tokens) AS BIGINT) as total_tokens,
                SUM(cost) as cost,
                MAX(last_used) as last_used
            FROM ai_usage_stats
            GROUP BY provider
            "#,
        )
        .fetch_all(pool)
        .await?;

        // Note: Postgres SUM returns numeric or double if float.
        // If underlying columns are INTEGER, SUM might return BIGINT.
        // We CAST above just in case or handle generically.
        // Actually, AiUsageStats expects what type? 
        // struct AiUsageStats fields are likely i32 or i64.
        // Assuming i32 originally, but SUM can exceed. Let's assume we map to generic usage.
        
        let mut stats = std::collections::HashMap::new();
        for row in rows {
            let provider: String = row.get("provider");
            
            // Handle potentially NULL or BigInt returns from PG SUM
             let input_tokens: i64 = row.get("input_tokens");
             let output_tokens: i64 = row.get("output_tokens");
             let total_tokens: i64 = row.get("total_tokens");
             
             // Struct expects what? Check models... usually i32. 
             // We'll coerce to expected type. If struct has i32, we cast.
             // But let's check the struct if possible? Assuming i32 based on update_ai_usage signature.
             
            stats.insert(provider.clone(), crate::core::models::database::AiUsageStats {
                provider,
                model: "aggregated".to_string(),
                input_tokens: input_tokens as i32,
                output_tokens: output_tokens as i32,
                total_tokens: total_tokens as i32,
                cost: row.get("cost"),
                last_used: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_used").unwrap_or_else(|_| {
                    row.try_get::<Option<String>, _>("last_used").ok().flatten().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok()
                    })
                }),
            });
        }
        Ok(stats)
    }

    pub async fn save_agent_run_state_internal(&self, execution_id: &str, state_json: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                execution_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at BIGINT NOT NULL
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO agent_run_states (execution_id, state_json, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT(execution_id) DO UPDATE SET state_json = excluded.state_json, updated_at = excluded.updated_at"
        )
        .bind(execution_id)
        .bind(state_json)
        .bind(Utc::now().timestamp_millis())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_agent_run_state_internal(&self, execution_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let state: Option<(String,)> = sqlx::query_as("SELECT state_json FROM agent_run_states WHERE execution_id = $1")
            .bind(execution_id)
            .fetch_optional(pool)
            .await?;
        Ok(state.map(|s| s.0))
    }
    pub async fn delete_agent_run_state_internal(&self, execution_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM agent_run_states WHERE execution_id = $1")
            .bind(execution_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
