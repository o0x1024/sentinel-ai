use anyhow::Result;
use sentinel_core::models::database::McpServerConfig;
use sqlx::sqlite::SqlitePool;

pub async fn create_mcp_server_config(
    pool: &SqlitePool,
    name: &str,
    description: Option<&str>,
    command: &str,
    args: &[String],
) -> Result<String> {
    let args_json = serde_json::to_string(args)?;
    let id = uuid::Uuid::new_v4().to_string();
    let url = "http://localhost:8080".to_string();
    let connection_type = "stdio".to_string();

    sqlx::query(
        r#"
        INSERT INTO mcp_server_configs (id, name, description, url, connection_type, command, args)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(&url)
    .bind(&connection_type)
    .bind(command)
    .bind(args_json)
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn get_all_mcp_server_configs(pool: &SqlitePool) -> Result<Vec<McpServerConfig>> {
    let configs = sqlx::query_as::<_, McpServerConfig>(
        "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, created_at, updated_at FROM mcp_server_configs",
    )
    .fetch_all(pool)
    .await?;
    Ok(configs)
}

pub async fn update_mcp_server_config_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> Result<()> {
    sqlx::query("UPDATE mcp_server_configs SET is_enabled = ? WHERE id = ?")
        .bind(enabled)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_mcp_server_config(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM mcp_server_configs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_mcp_server_config_by_name(pool: &SqlitePool, name: &str) -> Result<Option<McpServerConfig>> {
    let config = sqlx::query_as::<_, McpServerConfig>(
        "SELECT id, name, description, url, connection_type, command, args, is_enabled as enabled, created_at, updated_at FROM mcp_server_configs WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(config)
}

pub async fn update_mcp_server_config(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
    command: &str,
    args: &[String],
    enabled: bool,
) -> Result<()> {
    let args_json = serde_json::to_string(args)?;

    // 保留 url 与 connection_type
    let existing = get_mcp_server_config_by_name(pool, name).await?;
    let url = existing
        .as_ref()
        .map(|c| c.url.clone())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    let connection_type = existing
        .as_ref()
        .map(|c| c.connection_type.clone())
        .unwrap_or_else(|| "stdio".to_string());

    sqlx::query(
        "UPDATE mcp_server_configs SET name = ?, description = ?, url = ?, connection_type = ?, command = ?, args = ?, is_enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(&url)
    .bind(&connection_type)
    .bind(command)
    .bind(&args_json)
    .bind(enabled)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}


