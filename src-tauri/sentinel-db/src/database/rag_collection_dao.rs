use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagCollectionRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: i64,
    pub document_count: i64,
    pub chunk_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_rag_collection(pool: &SqlitePool, name: &str, description: Option<&str>) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO rag_collections (id, name, description, is_active, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn get_rag_collections(pool: &SqlitePool) -> Result<Vec<RagCollectionRow>> {
    let rows = sqlx::query(
        "SELECT id, name, description, is_active, document_count, chunk_count, created_at, updated_at FROM rag_collections ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(RagCollectionRow {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            is_active: row.get("is_active"),
            document_count: row.get("document_count"),
            chunk_count: row.get("chunk_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    Ok(out)
}

pub async fn get_rag_collection_by_id(pool: &SqlitePool, collection_id: &str) -> Result<Option<RagCollectionRow>> {
    let row = sqlx::query(
        "SELECT id, name, description, is_active, document_count, chunk_count, created_at, updated_at FROM rag_collections WHERE id = ?",
    )
    .bind(collection_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| RagCollectionRow {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        is_active: row.get("is_active"),
        document_count: row.get("document_count"),
        chunk_count: row.get("chunk_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn get_rag_collection_by_name(pool: &SqlitePool, name: &str) -> Result<Option<RagCollectionRow>> {
    let row = sqlx::query(
        "SELECT id, name, description, is_active, document_count, chunk_count, created_at, updated_at FROM rag_collections WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| RagCollectionRow {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        is_active: row.get("is_active"),
        document_count: row.get("document_count"),
        chunk_count: row.get("chunk_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn delete_rag_collection(pool: &SqlitePool, collection_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM rag_collections WHERE id = ?")
        .bind(collection_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_rag_collection(pool: &SqlitePool, collection_id: &str, name: &str, description: Option<&str>) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE rag_collections SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(&now)
        .bind(collection_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_rag_collection_active(pool: &SqlitePool, collection_id: &str, active: bool) -> Result<()> {
    sqlx::query("UPDATE rag_collections SET is_active = ?, updated_at = ? WHERE id = ?")
        .bind(if active { 1 } else { 0 })
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(collection_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_collection_stats(pool: &SqlitePool, collection_id: &str) -> Result<()> {
    let document_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rag_document_sources WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_one(pool)
    .await?;

    let chunk_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rag_chunks WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_one(pool)
    .await?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE rag_collections SET document_count = ?, chunk_count = ?, updated_at = ? WHERE id = ?",
    )
    .bind(document_count)
    .bind(chunk_count)
    .bind(&now)
    .bind(collection_id)
    .execute(pool)
    .await?;
    Ok(())
}


