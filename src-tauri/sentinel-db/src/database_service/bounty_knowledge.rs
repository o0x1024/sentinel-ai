use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::PgRow;

fn timestamp_to_string(row: &PgRow, column: &str) -> String {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| dt.to_rfc3339())
        .or_else(|_| row.try_get::<String, _>(column))
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

fn optional_string(row: &PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).unwrap_or(None)
}

fn row_to_bounty_knowledge_note(row: PgRow) -> BountyKnowledgeNoteRow {
    BountyKnowledgeNoteRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        title: row.get("title"),
        content: row.get("content"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
    }
}

fn row_to_bounty_knowledge_note_search(row: PgRow) -> BountyKnowledgeNoteSearchRow {
    BountyKnowledgeNoteSearchRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        program_name: row.get("program_name"),
        title: row.get("title"),
        content: row.get("content"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        snippet: optional_string(&row, "snippet"),
        score: row.try_get::<Option<f64>, _>("score").unwrap_or(None),
    }
}

fn normalize_tags_for_fts(tags_json: Option<&str>) -> String {
    let Some(tags_json) = tags_json else {
        return String::new();
    };

    serde_json::from_str::<Vec<String>>(tags_json)
        .map(|tags| {
            tags.into_iter()
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|_| tags_json.to_string())
}

fn build_sqlite_fts5_query(query: &str) -> String {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect();

    if tokens.is_empty() {
        query.trim().replace('"', "\"\"")
    } else {
        tokens.join(" AND ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyKnowledgeNoteRow {
    pub id: String,
    pub program_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyKnowledgeNoteSearchRow {
    pub id: String,
    pub program_id: Option<String>,
    pub program_name: Option<String>,
    pub title: String,
    pub content: String,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub snippet: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BountyKnowledgeNoteStats {
    pub total_notes: i64,
    pub bound_notes: i64,
    pub unbound_notes: i64,
}

#[derive(Debug, Clone, Default)]
pub struct BountyKnowledgeNoteFilter<'a> {
    pub program_id: Option<&'a str>,
    pub only_unbound: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct BountyKnowledgeNoteSearchFilter<'a> {
    pub query: &'a str,
    pub program_id: Option<&'a str>,
    pub only_unbound: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

async fn fetch_program_name_sqlite(
    pool: &sqlx::SqlitePool,
    program_id: Option<&str>,
) -> Result<Option<String>> {
    let Some(program_id) = program_id else {
        return Ok(None);
    };

    let row = sqlx::query("SELECT name FROM bounty_programs WHERE id = ?")
        .bind(program_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|row| row.try_get::<String, _>("name").ok()))
}

async fn sync_bounty_knowledge_note_fts_sqlite(
    pool: &sqlx::SqlitePool,
    note: &BountyKnowledgeNoteRow,
) -> Result<()> {
    let program_name = fetch_program_name_sqlite(pool, note.program_id.as_deref()).await?;
    let tags = normalize_tags_for_fts(note.tags_json.as_deref());

    sqlx::query("DELETE FROM bounty_knowledge_notes_fts WHERE note_id = ?")
        .bind(&note.id)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"INSERT INTO bounty_knowledge_notes_fts (
                note_id,
                title,
                content,
                tags,
                program_name
            ) VALUES (?, ?, ?, ?, ?)"#,
    )
    .bind(&note.id)
    .bind(&note.title)
    .bind(&note.content)
    .bind(tags)
    .bind(program_name)
    .execute(pool)
    .await?;

    Ok(())
}

async fn delete_bounty_knowledge_note_fts_sqlite(
    pool: &sqlx::SqlitePool,
    note_id: &str,
) -> Result<()> {
    sqlx::query("DELETE FROM bounty_knowledge_notes_fts WHERE note_id = ?")
        .bind(note_id)
        .execute(pool)
        .await?;
    Ok(())
}

impl DatabaseService {
    pub async fn create_bounty_knowledge_note(&self, note: &BountyKnowledgeNoteRow) -> Result<()> {
        let runtime = self.get_runtime_pool()?;

        match &runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO bounty_knowledge_notes (
                            id,
                            program_id,
                            title,
                            content,
                            tags_json,
                            metadata_json,
                            created_at,
                            updated_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&note.id)
                .bind(&note.program_id)
                .bind(&note.title)
                .bind(&note.content)
                .bind(&note.tags_json)
                .bind(&note.metadata_json)
                .bind(&note.created_at)
                .bind(&note.updated_at)
                .execute(pool)
                .await?;

                sync_bounty_knowledge_note_fts_sqlite(pool, note).await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO bounty_knowledge_notes (
                            id,
                            program_id,
                            title,
                            content,
                            tags_json,
                            metadata_json,
                            created_at,
                            updated_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&note.id)
                .bind(&note.program_id)
                .bind(&note.title)
                .bind(&note.content)
                .bind(&note.tags_json)
                .bind(&note.metadata_json)
                .bind(&note.created_at)
                .bind(&note.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO bounty_knowledge_notes (
                            id,
                            program_id,
                            title,
                            content,
                            tags_json,
                            metadata_json,
                            created_at,
                            updated_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
                )
                .bind(&note.id)
                .bind(&note.program_id)
                .bind(&note.title)
                .bind(&note.content)
                .bind(&note.tags_json)
                .bind(&note.metadata_json)
                .bind(&note.created_at)
                .bind(&note.updated_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_bounty_knowledge_note(
        &self,
        id: &str,
    ) -> Result<Option<BountyKnowledgeNoteRow>> {
        let runtime = self.get_runtime_pool()?;

        match &runtime {
            DatabasePool::SQLite(pool) => Ok(sqlx::query(
                "SELECT * FROM bounty_knowledge_notes WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
            .map(row_to_bounty_knowledge_note)),
            DatabasePool::MySQL(pool) => Ok(sqlx::query(
                "SELECT * FROM bounty_knowledge_notes WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
            .map(row_to_bounty_knowledge_note)),
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query(
                "SELECT * FROM bounty_knowledge_notes WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
            .map(row_to_bounty_knowledge_note)),
        }
    }

    pub async fn update_bounty_knowledge_note(
        &self,
        note: &BountyKnowledgeNoteRow,
    ) -> Result<bool> {
        let runtime = self.get_runtime_pool()?;

        let rows_affected = match &runtime {
            DatabasePool::SQLite(pool) => {
                let result = sqlx::query(
                    r#"UPDATE bounty_knowledge_notes
                        SET program_id = ?,
                            title = ?,
                            content = ?,
                            tags_json = ?,
                            metadata_json = ?,
                            updated_at = ?
                      WHERE id = ?"#,
                )
                .bind(&note.program_id)
                .bind(&note.title)
                .bind(&note.content)
                .bind(&note.tags_json)
                .bind(&note.metadata_json)
                .bind(&note.updated_at)
                .bind(&note.id)
                .execute(pool)
                .await?;

                if result.rows_affected() > 0 {
                    sync_bounty_knowledge_note_fts_sqlite(pool, note).await?;
                }

                result.rows_affected()
            }
            DatabasePool::MySQL(pool) => sqlx::query(
                r#"UPDATE bounty_knowledge_notes
                        SET program_id = ?,
                            title = ?,
                            content = ?,
                            tags_json = ?,
                            metadata_json = ?,
                            updated_at = ?
                      WHERE id = ?"#,
            )
            .bind(&note.program_id)
            .bind(&note.title)
            .bind(&note.content)
            .bind(&note.tags_json)
            .bind(&note.metadata_json)
            .bind(&note.updated_at)
            .bind(&note.id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                r#"UPDATE bounty_knowledge_notes
                        SET program_id = $1,
                            title = $2,
                            content = $3,
                            tags_json = $4,
                            metadata_json = $5,
                            updated_at = $6
                      WHERE id = $7"#,
            )
            .bind(&note.program_id)
            .bind(&note.title)
            .bind(&note.content)
            .bind(&note.tags_json)
            .bind(&note.metadata_json)
            .bind(&note.updated_at)
            .bind(&note.id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    pub async fn delete_bounty_knowledge_note(&self, id: &str) -> Result<bool> {
        let runtime = self.get_runtime_pool()?;

        let rows_affected = match &runtime {
            DatabasePool::SQLite(pool) => {
                let result = sqlx::query("DELETE FROM bounty_knowledge_notes WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;

                if result.rows_affected() > 0 {
                    delete_bounty_knowledge_note_fts_sqlite(pool, id).await?;
                }

                result.rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM bounty_knowledge_notes WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM bounty_knowledge_notes WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }

    pub async fn list_bounty_knowledge_notes(
        &self,
        filter: BountyKnowledgeNoteFilter<'_>,
    ) -> Result<Vec<BountyKnowledgeNoteSearchRow>> {
        let runtime = self.get_runtime_pool()?;
        let limit = filter.limit.unwrap_or(50).clamp(1, 200);
        let offset = filter.offset.unwrap_or(0);

        match &runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            substr(n.content, 1, 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE 1 = 1"#,
                );

                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(" AND n.program_id = ?");
                }

                query.push_str(" ORDER BY n.updated_at DESC LIMIT ? OFFSET ?");

                let mut built = sqlx::query(&query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
            DatabasePool::MySQL(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            SUBSTRING(n.content, 1, 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE 1 = 1"#,
                );

                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(" AND n.program_id = ?");
                }

                query.push_str(" ORDER BY n.updated_at DESC LIMIT ? OFFSET ?");

                let mut built = sqlx::query(&query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            substring(n.content from 1 for 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE 1 = 1"#,
                );

                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(" AND n.program_id = $1");
                }

                let (limit_placeholder, offset_placeholder) =
                    if filter.only_unbound || filter.program_id.is_none() {
                        ("$1", "$2")
                    } else {
                        ("$2", "$3")
                    };
                query.push_str(&format!(
                    " ORDER BY n.updated_at DESC LIMIT {} OFFSET {}",
                    limit_placeholder, offset_placeholder
                ));

                let mut built = sqlx::query(&query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
        }
    }

    async fn search_bounty_knowledge_notes_like(
        &self,
        filter: BountyKnowledgeNoteSearchFilter<'_>,
    ) -> Result<Vec<BountyKnowledgeNoteSearchRow>> {
        let runtime = self.get_runtime_pool()?;
        let limit = filter.limit.unwrap_or(20).clamp(1, 100);
        let offset = filter.offset.unwrap_or(0);
        let like_query = format!("%{}%", filter.query.trim().to_lowercase());

        match &runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            substr(n.content, 1, 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE (
                            lower(n.title) LIKE ?
                            OR lower(n.content) LIKE ?
                            OR lower(COALESCE(n.tags_json, '')) LIKE ?
                        )"#,
                );

                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(" AND n.program_id = ?");
                }

                query.push_str(" ORDER BY n.updated_at DESC LIMIT ? OFFSET ?");

                let mut built = sqlx::query(&query)
                    .bind(&like_query)
                    .bind(&like_query)
                    .bind(&like_query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
            DatabasePool::MySQL(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            SUBSTRING(n.content, 1, 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE (
                            lower(n.title) LIKE ?
                            OR lower(n.content) LIKE ?
                            OR lower(COALESCE(n.tags_json, '')) LIKE ?
                        )"#,
                );

                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(" AND n.program_id = ?");
                }

                query.push_str(" ORDER BY n.updated_at DESC LIMIT ? OFFSET ?");

                let mut built = sqlx::query(&query)
                    .bind(&like_query)
                    .bind(&like_query)
                    .bind(&like_query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            substring(n.content from 1 for 180) AS snippet,
                            NULL AS score
                        FROM bounty_knowledge_notes n
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE (
                            lower(n.title) LIKE $1
                            OR lower(n.content) LIKE $2
                            OR lower(COALESCE(n.tags_json, '')) LIKE $3
                        )"#,
                );

                let mut next_placeholder = 4;
                if filter.only_unbound {
                    query.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    query.push_str(&format!(" AND n.program_id = ${}", next_placeholder));
                    next_placeholder += 1;
                }

                query.push_str(&format!(
                    " ORDER BY n.updated_at DESC LIMIT ${} OFFSET ${}",
                    next_placeholder,
                    next_placeholder + 1
                ));

                let mut built = sqlx::query(&query)
                    .bind(&like_query)
                    .bind(&like_query)
                    .bind(&like_query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }
                let rows = built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await?;

                Ok(rows
                    .into_iter()
                    .map(row_to_bounty_knowledge_note_search)
                    .collect())
            }
        }
    }

    pub async fn search_bounty_knowledge_notes(
        &self,
        filter: BountyKnowledgeNoteSearchFilter<'_>,
    ) -> Result<Vec<BountyKnowledgeNoteSearchRow>> {
        let query = filter.query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }

        let runtime = self.get_runtime_pool()?;
        let limit = filter.limit.unwrap_or(20).clamp(1, 100);
        let offset = filter.offset.unwrap_or(0);

        match &runtime {
            DatabasePool::SQLite(pool) => {
                let match_query = build_sqlite_fts5_query(query);
                let mut sql = String::from(
                    r#"SELECT
                            n.id,
                            n.program_id,
                            p.name AS program_name,
                            n.title,
                            n.content,
                            n.tags_json,
                            n.metadata_json,
                            n.created_at,
                            n.updated_at,
                            snippet(bounty_knowledge_notes_fts, 2, '<mark>', '</mark>', ' … ', 18) AS snippet,
                            bm25(bounty_knowledge_notes_fts) AS score
                        FROM bounty_knowledge_notes_fts
                        JOIN bounty_knowledge_notes n ON n.id = bounty_knowledge_notes_fts.note_id
                        LEFT JOIN bounty_programs p ON p.id = n.program_id
                        WHERE bounty_knowledge_notes_fts MATCH ?"#,
                );

                if filter.only_unbound {
                    sql.push_str(" AND n.program_id IS NULL");
                } else if filter.program_id.is_some() {
                    sql.push_str(" AND n.program_id = ?");
                }

                sql.push_str(" ORDER BY score ASC, n.updated_at DESC LIMIT ? OFFSET ?");

                let mut built = sqlx::query(&sql).bind(&match_query);
                if !filter.only_unbound {
                    if let Some(program_id) = filter.program_id {
                        built = built.bind(program_id);
                    }
                }

                match built
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(pool)
                    .await
                {
                    Ok(rows) => Ok(rows
                        .into_iter()
                        .map(row_to_bounty_knowledge_note_search)
                        .collect()),
                    Err(_) => self.search_bounty_knowledge_notes_like(filter).await,
                }
            }
            DatabasePool::MySQL(_) | DatabasePool::PostgreSQL(_) => {
                self.search_bounty_knowledge_notes_like(filter).await
            }
        }
    }

    pub async fn get_bounty_knowledge_note_stats(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyKnowledgeNoteStats> {
        let runtime = self.get_runtime_pool()?;

        match &runtime {
            DatabasePool::SQLite(pool) => {
                if let Some(program_id) = program_id {
                    let total_notes = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id = ?",
                    )
                    .bind(program_id)
                    .fetch_one(pool)
                    .await?;

                    return Ok(BountyKnowledgeNoteStats {
                        total_notes,
                        bound_notes: total_notes,
                        unbound_notes: 0,
                    });
                }

                let total_notes =
                    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bounty_knowledge_notes")
                        .fetch_one(pool)
                        .await?;
                let bound_notes = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id IS NOT NULL",
                )
                .fetch_one(pool)
                .await?;
                let unbound_notes = total_notes - bound_notes;

                Ok(BountyKnowledgeNoteStats {
                    total_notes,
                    bound_notes,
                    unbound_notes,
                })
            }
            DatabasePool::MySQL(pool) => {
                if let Some(program_id) = program_id {
                    let total_notes = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id = ?",
                    )
                    .bind(program_id)
                    .fetch_one(pool)
                    .await?;

                    return Ok(BountyKnowledgeNoteStats {
                        total_notes,
                        bound_notes: total_notes,
                        unbound_notes: 0,
                    });
                }

                let total_notes =
                    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bounty_knowledge_notes")
                        .fetch_one(pool)
                        .await?;
                let bound_notes = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id IS NOT NULL",
                )
                .fetch_one(pool)
                .await?;
                let unbound_notes = total_notes - bound_notes;

                Ok(BountyKnowledgeNoteStats {
                    total_notes,
                    bound_notes,
                    unbound_notes,
                })
            }
            DatabasePool::PostgreSQL(pool) => {
                if let Some(program_id) = program_id {
                    let total_notes = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id = $1",
                    )
                    .bind(program_id)
                    .fetch_one(pool)
                    .await?;

                    return Ok(BountyKnowledgeNoteStats {
                        total_notes,
                        bound_notes: total_notes,
                        unbound_notes: 0,
                    });
                }

                let total_notes =
                    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bounty_knowledge_notes")
                        .fetch_one(pool)
                        .await?;
                let bound_notes = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM bounty_knowledge_notes WHERE program_id IS NOT NULL",
                )
                .fetch_one(pool)
                .await?;
                let unbound_notes = total_notes - bound_notes;

                Ok(BountyKnowledgeNoteStats {
                    total_notes,
                    bound_notes,
                    unbound_notes,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_service::service::DatabaseService;
    use chrono::Utc;

    async fn setup_service() -> DatabaseService {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            r#"CREATE TABLE bounty_programs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"CREATE TABLE bounty_knowledge_notes (
                id TEXT PRIMARY KEY,
                program_id TEXT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags_json TEXT,
                metadata_json TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(program_id) REFERENCES bounty_programs(id) ON DELETE SET NULL
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"CREATE VIRTUAL TABLE bounty_knowledge_notes_fts USING fts5(
                note_id UNINDEXED,
                title,
                content,
                tags,
                program_name,
                tokenize = 'unicode61 remove_diacritics 2'
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO bounty_programs (id, name) VALUES ('program-1', 'Acme')")
            .execute(&pool)
            .await
            .unwrap();

        let mut service = DatabaseService::new();
        service.runtime_pool = Some(DatabasePool::SQLite(pool));
        service
    }

    fn build_note(title: &str, content: &str) -> BountyKnowledgeNoteRow {
        let now = Utc::now().to_rfc3339();
        BountyKnowledgeNoteRow {
            id: "note-1".to_string(),
            program_id: Some("program-1".to_string()),
            title: title.to_string(),
            content: content.to_string(),
            tags_json: Some(r#"["xss","csrf"]"#.to_string()),
            metadata_json: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_search_update_delete_note_keeps_fts_in_sync() {
        let service = setup_service().await;
        let mut note = build_note("Login CSRF", "csrf token missing on profile update");

        service.create_bounty_knowledge_note(&note).await.unwrap();

        let created = service.get_bounty_knowledge_note(&note.id).await.unwrap();
        assert!(created.is_some());

        let search_rows = service
            .search_bounty_knowledge_notes(BountyKnowledgeNoteSearchFilter {
                query: "csrf",
                program_id: None,
                only_unbound: false,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(search_rows.len(), 1);
        assert_eq!(search_rows[0].program_name.as_deref(), Some("Acme"));

        note.title = "IDOR on profile".to_string();
        note.content = "user can read another profile".to_string();
        note.tags_json = Some(r#"["idor","profile"]"#.to_string());
        note.updated_at = Utc::now().to_rfc3339();
        assert!(service.update_bounty_knowledge_note(&note).await.unwrap());

        let old_search = service
            .search_bounty_knowledge_notes(BountyKnowledgeNoteSearchFilter {
                query: "csrf",
                program_id: None,
                only_unbound: false,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(old_search.is_empty());

        let new_search = service
            .search_bounty_knowledge_notes(BountyKnowledgeNoteSearchFilter {
                query: "profile",
                program_id: None,
                only_unbound: false,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(new_search.len(), 1);

        assert!(service
            .delete_bounty_knowledge_note(&note.id)
            .await
            .unwrap());
        let deleted_search = service
            .search_bounty_knowledge_notes(BountyKnowledgeNoteSearchFilter {
                query: "profile",
                program_id: None,
                only_unbound: false,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(deleted_search.is_empty());
    }
}
