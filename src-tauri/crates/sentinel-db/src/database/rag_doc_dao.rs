use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct RagDocumentSourceRow {
    pub id: String,
    pub collection_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub content_hash: String,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RagChunkRow {
    pub id: String,
    pub document_id: String,
    pub content: String,
    pub content_hash: String,
    pub chunk_index: i64,
    pub metadata: String,
    pub created_at: i64,
    pub embedding_vector: Option<Vec<u8>>, // optional
}

pub async fn insert_document_source(
    pool: &SqlitePool,
    id: &str,
    collection_id: &str,
    file_path: &str,
    file_name: &str,
    file_type: &str,
    file_size: i64,
    file_hash: &str,
    content_hash: &str,
    metadata: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO rag_document_sources (id, collection_id, file_path, file_name, file_type, file_size, file_hash, content_hash, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(collection_id)
    .bind(file_path)
    .bind(file_name)
    .bind(file_type)
    .bind(file_size)
    .bind(file_hash)
    .bind(content_hash)
    .bind(metadata)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_documents_by_collection_name(
    pool: &SqlitePool,
    collection_name: &str,
) -> Result<Vec<RagDocumentSourceRow>> {
    let rows = sqlx::query(
        r#"
        SELECT s.id, s.collection_id, s.file_path, s.file_name, s.file_type, s.file_size, s.content_hash, s.metadata, s.created_at, s.updated_at
        FROM rag_document_sources s
        JOIN rag_collections c ON s.collection_id = c.id
        WHERE c.name = ?
        ORDER BY s.created_at DESC
        "#,
    )
    .bind(collection_name)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RagDocumentSourceRow {
            id: row.get("id"),
            collection_id: row.get("collection_id"),
            file_path: row.get("file_path"),
            file_name: row.get("file_name"),
            file_type: row.get("file_type"),
            file_size: row.get("file_size"),
            content_hash: row.get("content_hash"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn get_documents_by_collection_id(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<RagDocumentSourceRow>> {
    let rows = sqlx::query(
        r#"
        SELECT id, collection_id, file_path, file_name, file_type, file_size, content_hash, metadata, created_at, updated_at
        FROM rag_document_sources
        WHERE collection_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RagDocumentSourceRow {
            id: row.get("id"),
            collection_id: row.get("collection_id"),
            file_path: row.get("file_path"),
            file_name: row.get("file_name"),
            file_type: row.get("file_type"),
            file_size: row.get("file_size"),
            content_hash: row.get("content_hash"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn delete_document_cascade(pool: &SqlitePool, document_id: &str) -> Result<()> {
    // delete new table chunks
    sqlx::query("DELETE FROM rag_chunks WHERE document_id = ?")
        .bind(document_id)
        .execute(pool)
        .await?;
    // delete legacy chunks if any
    let _ = sqlx::query("DELETE FROM rag_document_chunks WHERE source_id = ?")
        .bind(document_id)
        .execute(pool)
        .await;
    // delete document source
    sqlx::query("DELETE FROM rag_document_sources WHERE id = ?")
        .bind(document_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_collection_id_by_document_id(pool: &SqlitePool, document_id: &str) -> Result<Option<String>> {
    let cid = sqlx::query_scalar(
        "SELECT collection_id FROM rag_document_sources WHERE id = ?",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(cid)
}

pub async fn insert_chunk(
    pool: &SqlitePool,
    id: &str,
    document_id: &str,
    collection_id: &str,
    content: &str,
    content_hash: &str,
    chunk_index: i32,
    char_count: i32,
    embedding_bytes: Option<Vec<u8>>,
    embedding_model: &str,
    embedding_dimension: i32,
    metadata_json: &str,
    created_at_ts: i64,
    updated_at_ts: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO rag_chunks (id, document_id, collection_id, content, content_hash, chunk_index, char_count, embedding_vector, embedding_model, embedding_dimension, metadata, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(document_id)
    .bind(collection_id)
    .bind(content)
    .bind(content_hash)
    .bind(chunk_index)
    .bind(char_count)
    .bind(embedding_bytes)
    .bind(embedding_model)
    .bind(embedding_dimension)
    .bind(metadata_json)
    .bind(created_at_ts)
    .bind(updated_at_ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_chunks_by_document_id(pool: &SqlitePool, document_id: &str) -> Result<Vec<RagChunkRow>> {
    let rows = sqlx::query(
        r#"
        SELECT id, document_id, content, content_hash, chunk_index, metadata, created_at, embedding_vector
        FROM rag_chunks
        WHERE document_id = ?
        ORDER BY chunk_index ASC
        "#,
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| RagChunkRow {
            id: row.get("id"),
            document_id: row.get("document_id"),
            content: row.get("content"),
            content_hash: row.get("content_hash"),
            chunk_index: row.get("chunk_index"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            embedding_vector: row.get("embedding_vector"),
        })
        .collect())
}


