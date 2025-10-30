use anyhow::Result;
use sentinel_core::models::database::Configuration;
use sqlx::sqlite::SqlitePool;

pub async fn get_config(pool: &SqlitePool, category: &str, key: &str) -> Result<Option<String>> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM configurations WHERE category = ? AND key = ?",
    )
    .bind(category)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|(v,)| v))
}

pub async fn set_config(
    pool: &SqlitePool,
    category: &str,
    key: &str,
    value: &str,
    description: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO configurations (category, key, value, description) VALUES (?, ?, ?, ?)\n             ON CONFLICT(category, key) DO UPDATE SET value = excluded.value, description = excluded.description",
    )
    .bind(category)
    .bind(key)
    .bind(value)
    .bind(description)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_configs_by_category(pool: &SqlitePool, category: &str) -> Result<Vec<Configuration>> {
    let rows = sqlx::query_as::<_, Configuration>(
        "SELECT * FROM configurations WHERE category = ? ORDER BY key",
    )
    .bind(category)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}


