use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::surface::SurfaceSeedRow;
use anyhow::Result;

impl DatabaseService {
    pub async fn rename_surface_seed_type(&self, from_type: &str, to_type: &str) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows_affected = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE surface_seeds SET seed_type = ? WHERE seed_type = ?")
                    .bind(to_type)
                    .bind(from_type)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE surface_seeds SET seed_type = ? WHERE seed_type = ?")
                    .bind(to_type)
                    .bind(from_type)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE surface_seeds SET seed_type = $1 WHERE seed_type = $2")
                    .bind(to_type)
                    .bind(from_type)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        Ok(rows_affected)
    }

    pub async fn count_surface_seeds_filtered(
        &self,
        program_id: Option<&str>,
        search: Option<&str>,
        seed_type: Option<&str>,
        status: Option<&str>,
        source: Option<&str>,
    ) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let normalized_search = search.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("%{}%", trimmed.to_ascii_lowercase()))
            }
        });
        let normalized_seed_type = seed_type.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_status = status.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_source = source.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let total = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*)
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR seed_type = ?)
                       AND (? IS NULL OR status = ?)
                       AND (? IS NULL OR COALESCE(source, 'unknown') = ?)
                       AND (? IS NULL OR (
                            LOWER(seed_value) LIKE ?
                            OR LOWER(seed_type) LIKE ?
                            OR LOWER(status) LIKE ?
                            OR LOWER(COALESCE(source, 'unknown')) LIKE ?
                       ))",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .fetch_one(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*)
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR seed_type = ?)
                       AND (? IS NULL OR status = ?)
                       AND (? IS NULL OR COALESCE(source, 'unknown') = ?)
                       AND (? IS NULL OR (
                            LOWER(seed_value) LIKE ?
                            OR LOWER(seed_type) LIKE ?
                            OR LOWER(status) LIKE ?
                            OR LOWER(COALESCE(source, 'unknown')) LIKE ?
                       ))",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .fetch_one(pool)
                .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*)
                     FROM surface_seeds
                     WHERE ($1::text IS NULL OR program_id = $1)
                       AND ($2::text IS NULL OR seed_type = $2)
                       AND ($3::text IS NULL OR status = $3)
                       AND ($4::text IS NULL OR COALESCE(source, 'unknown') = $4)
                       AND ($5::text IS NULL OR (
                            LOWER(seed_value) LIKE $5
                            OR LOWER(seed_type) LIKE $5
                            OR LOWER(status) LIKE $5
                            OR LOWER(COALESCE(source, 'unknown')) LIKE $5
                       ))",
                )
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .fetch_one(pool)
                .await?
            }
        };

        Ok(total)
    }

    pub async fn list_surface_seeds_filtered(
        &self,
        program_id: Option<&str>,
        search: Option<&str>,
        seed_type: Option<&str>,
        status: Option<&str>,
        source: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<SurfaceSeedRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let normalized_search = search.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("%{}%", trimmed.to_ascii_lowercase()))
            }
        });
        let normalized_seed_type = seed_type.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_status = status.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_source = source.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "all" {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let normalized_limit = limit.map(|value| value as i64).unwrap_or(50);
        let normalized_offset = offset.map(|value| value as i64).unwrap_or(0);

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR seed_type = ?)
                       AND (? IS NULL OR status = ?)
                       AND (? IS NULL OR COALESCE(source, 'unknown') = ?)
                       AND (? IS NULL OR (
                            LOWER(seed_value) LIKE ?
                            OR LOWER(seed_type) LIKE ?
                            OR LOWER(status) LIKE ?
                            OR LOWER(COALESCE(source, 'unknown')) LIKE ?
                       ))
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC
                     LIMIT ? OFFSET ?",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_limit)
                .bind(normalized_offset)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR seed_type = ?)
                       AND (? IS NULL OR status = ?)
                       AND (? IS NULL OR COALESCE(source, 'unknown') = ?)
                       AND (? IS NULL OR (
                            LOWER(seed_value) LIKE ?
                            OR LOWER(seed_type) LIKE ?
                            OR LOWER(status) LIKE ?
                            OR LOWER(COALESCE(source, 'unknown')) LIKE ?
                       ))
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC
                     LIMIT ? OFFSET ?",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_limit)
                .bind(normalized_offset)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE ($1::text IS NULL OR program_id = $1)
                       AND ($2::text IS NULL OR seed_type = $2)
                       AND ($3::text IS NULL OR status = $3)
                       AND ($4::text IS NULL OR COALESCE(source, 'unknown') = $4)
                       AND ($5::text IS NULL OR (
                            LOWER(seed_value) LIKE $5
                            OR LOWER(seed_type) LIKE $5
                            OR LOWER(status) LIKE $5
                            OR LOWER(COALESCE(source, 'unknown')) LIKE $5
                       ))
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC
                     LIMIT $6 OFFSET $7",
                )
                .bind(program_id)
                .bind(normalized_seed_type.as_deref())
                .bind(normalized_status.as_deref())
                .bind(normalized_source.as_deref())
                .bind(normalized_search.as_deref())
                .bind(normalized_limit)
                .bind(normalized_offset)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows)
    }

    pub async fn get_surface_seed_by_id(&self, seed_id: &str) -> Result<Option<SurfaceSeedRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE id = ?",
                )
                .bind(seed_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE id = ?",
                )
                .bind(seed_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE id = $1",
                )
                .bind(seed_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn get_surface_seed_by_identity(
        &self,
        program_id: &str,
        seed_type: &str,
        seed_value: &str,
    ) -> Result<Option<SurfaceSeedRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE program_id = ? AND seed_type = ? AND seed_value = ? LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE program_id = ? AND seed_type = ? AND seed_value = ? LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at FROM surface_seeds WHERE program_id = $1 AND seed_type = $2 AND seed_value = $3 LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn list_surface_seeds(
        &self,
        program_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<SurfaceSeedRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR status = ?)
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(status)
                .bind(status)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR status = ?)
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(status)
                .bind(status)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SurfaceSeedRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at
                     FROM surface_seeds
                     WHERE ($1::text IS NULL OR program_id = $1)
                       AND ($2::text IS NULL OR status = $2)
                     ORDER BY
                       CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       seed_type ASC,
                       seed_value ASC,
                       updated_at DESC",
                )
                .bind(program_id)
                .bind(status)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows)
    }

    pub async fn create_surface_seed(&self, seed: &SurfaceSeedRow) -> Result<SurfaceSeedRow> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seeds (id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&seed.id)
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.created_at)
                .bind(&seed.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seeds (id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&seed.id)
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.created_at)
                .bind(&seed.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seeds (id, program_id, seed_type, seed_value, status, source, confidence_score, last_run_at, metadata_json, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                )
                .bind(&seed.id)
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.created_at)
                .bind(&seed.updated_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(seed.clone())
    }

    pub async fn update_surface_seed(&self, seed: &SurfaceSeedRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows_affected = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE surface_seeds SET program_id = ?, seed_type = ?, seed_value = ?, status = ?, source = ?, confidence_score = ?, last_run_at = ?, metadata_json = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.updated_at)
                .bind(&seed.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE surface_seeds SET program_id = ?, seed_type = ?, seed_value = ?, status = ?, source = ?, confidence_score = ?, last_run_at = ?, metadata_json = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.updated_at)
                .bind(&seed.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE surface_seeds SET program_id = $1, seed_type = $2, seed_value = $3, status = $4, source = $5, confidence_score = $6, last_run_at = $7, metadata_json = $8, updated_at = $9 WHERE id = $10",
                )
                .bind(&seed.program_id)
                .bind(&seed.seed_type)
                .bind(&seed.seed_value)
                .bind(&seed.status)
                .bind(&seed.source)
                .bind(seed.confidence_score)
                .bind(&seed.last_run_at)
                .bind(&seed.metadata_json)
                .bind(&seed.updated_at)
                .bind(&seed.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }

    pub async fn delete_surface_seed(&self, seed_id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows_affected = match runtime {
            DatabasePool::SQLite(pool) => sqlx::query("DELETE FROM surface_seeds WHERE id = ?")
                .bind(seed_id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query("DELETE FROM surface_seeds WHERE id = ?")
                .bind(seed_id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM surface_seeds WHERE id = $1")
                    .bind(seed_id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }
}
