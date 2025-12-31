use anyhow::Result;
use crate::database_service::service::DatabaseService;
use sqlx::Row;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagCollectionRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub document_count: i64,
    pub chunk_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagDocumentSourceRow {
    pub id: String,
    pub collection_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_hash: String,
    pub content_hash: String,
    pub status: String,
    pub metadata: String,
    pub chunk_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagChunkRow {
    pub id: String,
    pub document_id: String,
    pub collection_id: String,
    pub content: String,
    pub content_hash: String,
    pub chunk_index: i32,
    pub char_count: i32,
    pub embedding: Option<Vec<u8>>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    pub async fn create_rag_collection_internal(
        &self, 
        name: &str, 
        description: Option<&str>,
    ) -> Result<String> {
        let pool = self.get_pool()?;
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

    pub async fn get_rag_collections_internal(&self) -> Result<Vec<RagCollectionRow>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            "SELECT * FROM rag_collections ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(RagCollectionRow {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                is_active: row.get::<i64, _>("is_active") != 0,
                document_count: row.get("document_count"),
                chunk_count: row.get("chunk_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }
        Ok(out)
    }

    pub async fn get_rag_collection_by_id_internal(&self, collection_id: &str) -> Result<Option<RagCollectionRow>> {
        let pool = self.get_pool()?;
        let row = sqlx::query(
            "SELECT * FROM rag_collections WHERE id = ?",
        )
        .bind(collection_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|row| RagCollectionRow {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            is_active: row.get::<i64, _>("is_active") != 0,
            document_count: row.get("document_count"),
            chunk_count: row.get("chunk_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn get_rag_collection_by_name_internal(&self, name: &str) -> Result<Option<RagCollectionRow>> {
        let pool = self.get_pool()?;
        let row = sqlx::query(
            "SELECT * FROM rag_collections WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|row| RagCollectionRow {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            is_active: row.get::<i64, _>("is_active") != 0,
            document_count: row.get("document_count"),
            chunk_count: row.get("chunk_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn delete_rag_collection_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let mut tx = pool.begin().await?;

        // 1. Delete chunks associated with the collection
        sqlx::query("DELETE FROM rag_chunks WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 2. Delete document sources associated with the collection
        sqlx::query("DELETE FROM rag_document_sources WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 3. Delete queries associated with the collection
        sqlx::query("DELETE FROM rag_queries WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 4. Finally delete the collection itself
        sqlx::query("DELETE FROM rag_collections WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_rag_collection_internal(&self, id: &str, name: &str, description: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE rag_collections SET name = ?, description = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(description)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_rag_collection_active_internal(&self, id: &str, active: bool) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE rag_collections SET is_active = ?, updated_at = ? WHERE id = ?")
            .bind(if active { 1 } else { 0 })
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_collection_stats_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let document_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rag_document_sources WHERE collection_id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        let chunk_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rag_chunks WHERE collection_id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE rag_collections SET document_count = ?, chunk_count = ?, updated_at = ? WHERE id = ?",
        )
        .bind(document_count)
        .bind(chunk_count)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_documents_by_collection_name_internal(&self, collection_name: &str) -> Result<Vec<RagDocumentSourceRow>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            r#"SELECT s.* FROM rag_document_sources s 
               JOIN rag_collections c ON s.collection_id = c.id 
               WHERE c.name = ? ORDER BY s.created_at DESC"#
        )
        .bind(collection_name)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|r| self.row_to_doc_source(r)).collect())
    }

    pub async fn get_documents_by_collection_id_internal(&self, collection_id: &str) -> Result<Vec<RagDocumentSourceRow>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            "SELECT * FROM rag_document_sources WHERE collection_id = ? ORDER BY created_at DESC"
        )
        .bind(collection_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|r| self.row_to_doc_source(r)).collect())
    }

    pub async fn insert_document_source_internal(
        &self,
        id: &str,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        file_type: &str,
        file_size: i64,
        file_hash: &str,
        content_hash: &str,
        status: &str,
        metadata: &str,
        created_at: &str,
        updated_at: &str,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO rag_document_sources (
                id, collection_id, file_path, file_name, file_type, file_size, 
                file_hash, content_hash, status, metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(id)
        .bind(collection_id)
        .bind(file_path)
        .bind(file_name)
        .bind(file_type)
        .bind(file_size)
        .bind(file_hash)
        .bind(content_hash)
        .bind(status)
        .bind(metadata)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_document_cascade_internal(&self, document_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM rag_chunks WHERE document_id = ?").bind(document_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM rag_document_sources WHERE id = ?").bind(document_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_collection_id_by_document_id_internal(&self, document_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let id: Option<String> = sqlx::query_scalar("SELECT collection_id FROM rag_document_sources WHERE id = ?")
            .bind(document_id)
            .fetch_optional(pool)
            .await?;
        Ok(id)
    }

    pub async fn insert_chunk_internal(
        &self,
        id: &str,
        document_id: &str,
        collection_id: &str,
        content: &str,
        content_hash: &str,
        chunk_index: i32,
        char_count: i32,
        embedding_bytes: Option<Vec<u8>>,
        metadata_json: &str,
        created_at_ts: i64,
        updated_at_ts: i64,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO rag_chunks (
                id, document_id, collection_id, content, content_hash, chunk_index, char_count,
                embedding, metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(id)
        .bind(document_id)
        .bind(collection_id)
        .bind(content)
        .bind(content_hash)
        .bind(chunk_index)
        .bind(char_count)
        .bind(embedding_bytes)
        .bind(metadata_json)
        .bind(created_at_ts)
        .bind(updated_at_ts)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_chunks_by_document_id_internal(&self, document_id: &str) -> Result<Vec<RagChunkRow>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM rag_chunks WHERE document_id = ? ORDER BY chunk_index ASC")
            .bind(document_id)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(|r| self.row_to_rag_chunk(r)).collect())
    }

    fn row_to_doc_source(&self, row: sqlx::sqlite::SqliteRow) -> RagDocumentSourceRow {
        RagDocumentSourceRow {
            id: row.get("id"),
            collection_id: row.get("collection_id"),
            file_path: row.get("file_path"),
            file_name: row.get("file_name"),
            file_type: row.get("file_type"),
            file_size: row.get("file_size"),
            file_hash: row.get("file_hash"),
            content_hash: row.get("content_hash"),
            status: row.get("status"),
            metadata: row.get("metadata"),
            chunk_count: row.get("chunk_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_rag_chunk(&self, row: sqlx::sqlite::SqliteRow) -> RagChunkRow {
        RagChunkRow {
            id: row.get("id"),
            document_id: row.get("document_id"),
            collection_id: row.get("collection_id"),
            content: row.get("content"),
            content_hash: row.get("content_hash"),
            chunk_index: row.get("chunk_index"),
            char_count: row.get("char_count"),
            embedding: row.get("embedding"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create_rag_document_internal(
        &self,
        collection_id: &str,
        file_path: &str,
        file_name: &str,
        content: &str,
        metadata: &str,
    ) -> Result<String> {
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let content_hash = format!("{:x}", md5::compute(content));
        
        sqlx::query(
            r#"INSERT INTO rag_document_sources (id, collection_id, file_path, file_name, file_type, file_size, content_hash, status, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&id)
        .bind(collection_id)
        .bind(file_path)
        .bind(file_name)
        .bind("text")
        .bind(content.len() as i64)
        .bind(content_hash)
        .bind("Completed") // Since we are creating it with content, mark as completed
        .bind(metadata)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    pub async fn create_rag_chunk_internal(
        &self,
        document_id: &str,
        collection_id: &str,
        content: &str,
        chunk_index: i32,
        embedding: Option<&[f32]>,
        metadata_json: &str,
    ) -> Result<String> {
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let content_hash = format!("{:x}", md5::compute(content));
        
        let embedding_bytes = embedding.map(|e| {
            let mut bytes = Vec::with_capacity(e.len() * 4);
            for &f in e {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            bytes
        });

        sqlx::query(
            r#"INSERT INTO rag_chunks (id, document_id, collection_id, content, content_hash, chunk_index, char_count, embedding, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&id)
        .bind(document_id)
        .bind(collection_id)
        .bind(content)
        .bind(content_hash)
        .bind(chunk_index)
        .bind(content.len() as i32)
        .bind(embedding_bytes)
        .bind(metadata_json)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    pub async fn get_rag_documents_internal(&self, collection_id: &str) -> Result<Vec<sentinel_rag::models::DocumentSource>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM rag_document_sources WHERE collection_id = ? ORDER BY created_at DESC")
            .bind(collection_id)
            .fetch_all(pool)
            .await?;
            
        let mut docs = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let ingestion_status = match status_str.to_lowercase().as_str() {
                "pending" => sentinel_rag::models::IngestionStatusEnum::Pending,
                "processing" => sentinel_rag::models::IngestionStatusEnum::Processing,
                "completed" => sentinel_rag::models::IngestionStatusEnum::Completed,
                "failed" => sentinel_rag::models::IngestionStatusEnum::Failed,
                _ => sentinel_rag::models::IngestionStatusEnum::Pending,
            };

            docs.push(sentinel_rag::models::DocumentSource {
                id: row.get("id"),
                file_path: row.get("file_path"),
                file_name: row.get("file_name"),
                file_type: row.get("file_type"),
                file_size: row.get::<i64, _>("file_size") as u64,
                file_hash: row.get("file_hash"),
                chunk_count: row.get::<i64, _>("chunk_count") as usize,
                ingestion_status,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap_or_default().with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap_or_default().with_timezone(&chrono::Utc),
                metadata: std::collections::HashMap::new(),
            });
        }
        Ok(docs)
    }

    pub async fn get_rag_documents_paginated_internal(
        &self, 
        collection_id: &str, 
        limit: i64, 
        offset: i64,
        search_query: Option<&str>
    ) -> Result<(Vec<sentinel_rag::models::DocumentSource>, i64)> {
        let pool = self.get_pool()?;
        
        // Build query with optional search
        let (query_str, count_str) = if let Some(_search) = search_query {
            (
                "SELECT * FROM rag_document_sources WHERE collection_id = ? AND (file_name LIKE ? OR file_path LIKE ?) ORDER BY created_at DESC LIMIT ? OFFSET ?".to_string(),
                "SELECT COUNT(*) as count FROM rag_document_sources WHERE collection_id = ? AND (file_name LIKE ? OR file_path LIKE ?)".to_string()
            )
        } else {
            (
                "SELECT * FROM rag_document_sources WHERE collection_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?".to_string(),
                "SELECT COUNT(*) as count FROM rag_document_sources WHERE collection_id = ?".to_string()
            )
        };
        
        // Get total count
        let total_count: i64 = if let Some(search) = search_query {
            let search_pattern = format!("%{}%", search);
            sqlx::query(&count_str)
                .bind(collection_id)
                .bind(&search_pattern)
                .bind(&search_pattern)
                .fetch_one(pool)
                .await?
                .get("count")
        } else {
            sqlx::query(&count_str)
                .bind(collection_id)
                .fetch_one(pool)
                .await?
                .get("count")
        };
        
        // Get paginated documents
        let rows = if let Some(search) = search_query {
            let search_pattern = format!("%{}%", search);
            sqlx::query(&query_str)
                .bind(collection_id)
                .bind(&search_pattern)
                .bind(&search_pattern)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(&query_str)
                .bind(collection_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        };
            
        let mut docs = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let ingestion_status = match status_str.to_lowercase().as_str() {
                "pending" => sentinel_rag::models::IngestionStatusEnum::Pending,
                "processing" => sentinel_rag::models::IngestionStatusEnum::Processing,
                "completed" => sentinel_rag::models::IngestionStatusEnum::Completed,
                "failed" => sentinel_rag::models::IngestionStatusEnum::Failed,
                _ => sentinel_rag::models::IngestionStatusEnum::Pending,
            };

            docs.push(sentinel_rag::models::DocumentSource {
                id: row.get("id"),
                file_path: row.get("file_path"),
                file_name: row.get("file_name"),
                file_type: row.get("file_type"),
                file_size: row.get::<i64, _>("file_size") as u64,
                file_hash: row.get("file_hash"),
                chunk_count: row.get::<i64, _>("chunk_count") as usize,
                ingestion_status,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap_or_default().with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap_or_default().with_timezone(&chrono::Utc),
                metadata: std::collections::HashMap::new(),
            });
        }
        
        Ok((docs, total_count))
    }

    pub async fn get_rag_chunks_internal(&self, document_id: &str) -> Result<Vec<sentinel_rag::models::DocumentChunk>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query(
            r#"SELECT c.*, s.file_path, s.file_name, s.file_type, s.file_size 
               FROM rag_chunks c 
               JOIN rag_document_sources s ON c.document_id = s.id 
               WHERE c.document_id = ? ORDER BY c.chunk_index ASC"#
        )
        .bind(document_id)
        .fetch_all(pool)
        .await?;
            
        let mut chunks = Vec::new();
        for row in rows {
            chunks.push(sentinel_rag::models::DocumentChunk {
                id: row.get("id"),
                source_id: row.get("document_id"),
                content: row.get("content"),
                content_hash: row.get("content_hash"),
                chunk_index: row.get::<i32, _>("chunk_index") as usize,
                metadata: sentinel_rag::models::ChunkMetadata {
                    file_path: row.get("file_path"),
                    file_name: row.get("file_name"),
                    file_type: row.get("file_type"),
                    file_size: row.get::<i64, _>("file_size") as u64,
                    chunk_start_char: 0,
                    chunk_end_char: 0,
                    page_number: None,
                    section_title: None,
                    custom_fields: std::collections::HashMap::new(),
                },
                embedding: None,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap_or_default().with_timezone(&chrono::Utc),
            });
        }
        Ok(chunks)
    }

    pub async fn delete_rag_document_internal(&self, document_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM rag_chunks WHERE document_id = ?").bind(document_id).execute(pool).await?;
        sqlx::query("DELETE FROM rag_document_sources WHERE id = ?").bind(document_id).execute(pool).await?;
        Ok(())
    }

    pub async fn save_rag_query_internal(
        &self,
        collection_id: Option<&str>,
        conversation_id: Option<&str>,
        query: &str,
        response: &str,
        processing_time_ms: u64,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO rag_queries (id, collection_id, conversation_id, query, response, processing_time_ms, created_at)
               VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"#
        )
        .bind(&id)
        .bind(collection_id)
        .bind(conversation_id)
        .bind(query)
        .bind(response)
        .bind(processing_time_ms as i64)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_rag_query_history_internal(
        &self,
        _collection_id: Option<&str>,
        _limit: Option<i32>,
    ) -> Result<Vec<sentinel_rag::models::QueryResult>> {
        Ok(Vec::new())
    }
}
