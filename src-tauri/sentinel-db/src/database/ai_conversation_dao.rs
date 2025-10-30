use anyhow::Result;
use sentinel_core::models::database::{AiConversation, AiMessage};
use sqlx::sqlite::SqlitePool;

pub async fn create_ai_conversation(pool: &SqlitePool, conversation: &AiConversation) -> Result<()> {
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
            total_tokens, cost, tags, is_archived, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(&conversation.tags)
    .bind(conversation.is_archived)
    .bind(conversation.created_at)
    .bind(conversation.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_ai_conversations(pool: &SqlitePool) -> Result<Vec<AiConversation>> {
    let conversations = sqlx::query_as::<_, AiConversation>(
        "SELECT * FROM ai_conversations ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(conversations)
}

pub async fn get_ai_conversation(pool: &SqlitePool, id: &str) -> Result<Option<AiConversation>> {
    let conversation = sqlx::query_as::<_, AiConversation>(
        "SELECT * FROM ai_conversations WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(conversation)
}

pub async fn update_ai_conversation(pool: &SqlitePool, conversation: &AiConversation) -> Result<()> {
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
            is_archived = ?, updated_at = ?
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
    .bind(&conversation.tags)
    .bind(conversation.is_archived)
    .bind(chrono::Utc::now())
    .bind(&conversation.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_ai_conversation(pool: &SqlitePool, id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_conversation_title(pool: &SqlitePool, id: &str, title: &str) -> Result<()> {
    sqlx::query("UPDATE ai_conversations SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn archive_ai_conversation(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("UPDATE ai_conversations SET is_archived = 1, updated_at = ? WHERE id = ?")
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_ai_message(pool: &SqlitePool, message: &AiMessage) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO ai_messages (
            id, conversation_id, role, content, metadata, token_count, cost, tool_calls,
            attachments, timestamp
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(message.timestamp)
    .execute(pool)
    .await?;

    // 更新会话 updated_at & total_messages
    sqlx::query("UPDATE ai_conversations SET updated_at = ?, total_messages = total_messages + 1 WHERE id = ?")
        .bind(chrono::Utc::now())
        .bind(&message.conversation_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_ai_messages_by_conversation(pool: &SqlitePool, conversation_id: &str) -> Result<Vec<AiMessage>> {
    let messages = sqlx::query_as::<_, AiMessage>(
        "SELECT * FROM ai_messages WHERE conversation_id = ? ORDER BY timestamp ASC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;
    Ok(messages)
}


