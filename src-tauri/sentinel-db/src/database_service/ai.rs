use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::core::models::ai::AiRole;
use crate::core::models::database::{AiConversation, AiMessage};
use crate::database_service::service::DatabaseService;
use sqlx::Row;

impl DatabaseService {
    pub async fn create_ai_conversation_internal(&self, conversation: &AiConversation) -> Result<()> {
        let pool = self.get_pool()?;

        // 验证关联ID存在性
        let vulnerability_id = if let Some(ref vuln_id) = conversation.vulnerability_id {
            if !vuln_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM vulnerabilities WHERE id = ?")
                    .bind(vuln_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(vuln_id.clone()) } else { None }
            } else { None }
        } else { None };

        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM scan_tasks WHERE id = ?")
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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

        let rows = sqlx::query_as::<_, AiConversation>(
            "SELECT * FROM ai_conversations ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_ai_conversations_paginated_internal(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiConversation>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, AiConversation>(
            "SELECT * FROM ai_conversations ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_ai_conversations_count_internal(&self) -> Result<i64> {
        let pool = self.get_pool()?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ai_conversations")
            .fetch_one(pool)
            .await?;

        Ok(count.0)
    }

    pub async fn get_ai_conversation_internal(&self, id: &str) -> Result<Option<AiConversation>> {
        let pool = self.get_pool()?;

        let row =
            sqlx::query_as::<_, AiConversation>("SELECT * FROM ai_conversations WHERE id = ?")
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
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM vulnerabilities WHERE id = ?")
                    .bind(vuln_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(vuln_id.clone()) } else { None }
            } else { None }
        } else { None };

        let scan_task_id = if let Some(ref task_id) = conversation.scan_task_id {
            if !task_id.is_empty() {
                let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM scan_tasks WHERE id = ?")
                    .bind(task_id)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() { Some(task_id.clone()) } else { None }
            } else { None }
        } else { None };

        sqlx::query(
            r#"
            UPDATE ai_conversations
            SET title = ?, service_name = ?, model_name = ?, model_provider = ?, context_type = ?,
                project_id = ?, vulnerability_id = ?, scan_task_id = ?, conversation_data = ?,
                summary = ?, total_messages = ?, total_tokens = ?, cost = ?, tags = ?,
                tool_config = ?, is_archived = ?, updated_at = ?
            WHERE id = ?
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
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        // 再删除对话
        sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_ai_conversation_title_internal(&self, id: &str, title: &str) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query("UPDATE ai_conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn archive_ai_conversation_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE ai_conversations SET is_archived = 1, updated_at = ? WHERE id = ?")
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        sqlx::query("UPDATE ai_conversations SET updated_at = ?, total_messages = total_messages + 1 WHERE id = ?")
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
        
        self.retry_on_locked(|| async {
            let mut tx = pool.begin().await?;

            let exists = sqlx::query("SELECT id FROM ai_messages WHERE id = ?")
                .bind(&msg.id)
                .fetch_optional(&mut *tx)
                .await?;

            if exists.is_some() {
                sqlx::query("UPDATE ai_messages SET content = content || ?, metadata = ?, token_count = ?, cost = ?, timestamp = ? WHERE id = ?")
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
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
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
        }).await
    }

    pub async fn get_ai_messages_by_conversation_internal(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query_as::<_, AiMessage>(
            "SELECT * FROM ai_messages WHERE conversation_id = ? ORDER BY timestamp ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
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
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(roles)
    }

    pub async fn create_ai_role_internal(&self, role: &AiRole) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("INSERT INTO ai_roles (id, title, description, prompt, is_system, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
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
        sqlx::query("UPDATE ai_roles SET title = ?, description = ?, prompt = ?, updated_at = ? WHERE id = ?")
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
        sqlx::query("DELETE FROM ai_roles WHERE id = ?")
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
            let row = sqlx::query("SELECT id, title, description, prompt, is_system, created_at, updated_at FROM ai_roles WHERE id = ?")
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
        sqlx::query("DELETE FROM ai_messages WHERE id = ?")
            .bind(message_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_ai_messages_by_conversation_internal(&self, conversation_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ?")
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
            "SELECT timestamp FROM ai_messages WHERE id = ? AND conversation_id = ?"
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
            "DELETE FROM ai_messages WHERE conversation_id = ? AND timestamp > ?"
        )
        .bind(conversation_id)
        .bind(timestamp)
        .execute(pool)
        .await?;

        let deleted_count = result.rows_affected();

        // Update conversation's message count
        if deleted_count > 0 {
            sqlx::query(
                "UPDATE ai_conversations SET total_messages = total_messages - ?, updated_at = ? WHERE id = ?"
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
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider, model) DO UPDATE SET
                input_tokens = input_tokens + excluded.input_tokens,
                output_tokens = output_tokens + excluded.output_tokens,
                total_tokens = total_tokens + excluded.total_tokens,
                cost = cost + excluded.cost,
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
        let rows = sqlx::query_as::<_, crate::core::models::database::AiUsageStats>(
            "SELECT * FROM ai_usage_stats ORDER BY total_tokens DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_aggregated_ai_usage_internal(&self) -> Result<std::collections::HashMap<String, crate::core::models::database::AiUsageStats>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            r#"
            SELECT 
                provider,
                'aggregated' as model,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                SUM(total_tokens) as total_tokens,
                SUM(cost) as cost,
                MAX(last_used) as last_used
            FROM ai_usage_stats
            GROUP BY provider
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut stats = std::collections::HashMap::new();
        for row in rows {
            let provider: String = row.get("provider");
            stats.insert(provider.clone(), crate::core::models::database::AiUsageStats {
                provider,
                model: "aggregated".to_string(),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                total_tokens: row.get("total_tokens"),
                cost: row.get("cost"),
                last_used: row.get::<Option<DateTime<Utc>>, _>("last_used"),
            });
        }
        Ok(stats)
    }
}
