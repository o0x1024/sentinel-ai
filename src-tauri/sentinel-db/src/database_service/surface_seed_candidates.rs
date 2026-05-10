use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::SurfaceSeedCandidateRow;
use anyhow::Result;

impl DatabaseService {
    pub async fn list_surface_seed_candidates(
        &self,
        program_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<SurfaceSeedCandidateRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR status = ?)
                     ORDER BY observed_at DESC, created_at DESC, seed_type ASC, seed_value ASC",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(status)
                .bind(status)
                .fetch_all(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<MySql, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE (? IS NULL OR program_id = ?)
                       AND (? IS NULL OR status = ?)
                     ORDER BY observed_at DESC, created_at DESC, seed_type ASC, seed_value ASC",
                )
                .bind(program_id)
                .bind(program_id)
                .bind(status)
                .bind(status)
                .fetch_all(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<Postgres, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE ($1::text IS NULL OR program_id = $1)
                       AND ($2::text IS NULL OR status = $2)
                     ORDER BY observed_at DESC, created_at DESC, seed_type ASC, seed_value ASC",
                )
                .bind(program_id)
                .bind(status)
                .fetch_all(pool)
                .await
                .map_err(Into::into)
            }
        }
    }

    pub async fn get_surface_seed_candidate_by_id(
        &self,
        candidate_id: &str,
    ) -> Result<Option<SurfaceSeedCandidateRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates WHERE id = ?",
                )
                .bind(candidate_id)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<MySql, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates WHERE id = ?",
                )
                .bind(candidate_id)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<Postgres, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates WHERE id = $1",
                )
                .bind(candidate_id)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
        }
    }

    pub async fn get_surface_seed_candidate_by_identity(
        &self,
        program_id: &str,
        seed_type: &str,
        seed_value: &str,
        source_asset_id: &str,
        source_detail_key: &str,
    ) -> Result<Option<SurfaceSeedCandidateRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE program_id = ? AND seed_type = ? AND seed_value = ? AND source_asset_id = ? AND source_detail_key = ?
                     LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .bind(source_asset_id)
                .bind(source_detail_key)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<MySql, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE program_id = ? AND seed_type = ? AND seed_value = ? AND source_asset_id = ? AND source_detail_key = ?
                     LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .bind(source_asset_id)
                .bind(source_detail_key)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<Postgres, SurfaceSeedCandidateRow>(
                    "SELECT id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at
                     FROM surface_seed_candidates
                     WHERE program_id = $1 AND seed_type = $2 AND seed_value = $3 AND source_asset_id = $4 AND source_detail_key = $5
                     LIMIT 1",
                )
                .bind(program_id)
                .bind(seed_type)
                .bind(seed_value)
                .bind(source_asset_id)
                .bind(source_detail_key)
                .fetch_optional(pool)
                .await
                .map_err(Into::into)
            }
        }
    }

    pub async fn create_surface_seed_candidate(
        &self,
        candidate: &SurfaceSeedCandidateRow,
    ) -> Result<SurfaceSeedCandidateRow> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seed_candidates (id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&candidate.id)
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.created_at)
                .bind(&candidate.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seed_candidates (id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&candidate.id)
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.created_at)
                .bind(&candidate.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_seed_candidates (id, program_id, seed_type, seed_value, status, source_asset_id, source_asset_type, source_detail_key, source_display_value, source_canonical_url, confidence_score, observed_at, reviewed_at, metadata_json, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
                )
                .bind(&candidate.id)
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.created_at)
                .bind(&candidate.updated_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(candidate.clone())
    }

    pub async fn update_surface_seed_candidate(
        &self,
        candidate: &SurfaceSeedCandidateRow,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE surface_seed_candidates
                     SET program_id = ?, seed_type = ?, seed_value = ?, status = ?, source_asset_id = ?, source_asset_type = ?, source_detail_key = ?, source_display_value = ?, source_canonical_url = ?, confidence_score = ?, observed_at = ?, reviewed_at = ?, metadata_json = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.updated_at)
                .bind(&candidate.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE surface_seed_candidates
                     SET program_id = ?, seed_type = ?, seed_value = ?, status = ?, source_asset_id = ?, source_asset_type = ?, source_detail_key = ?, source_display_value = ?, source_canonical_url = ?, confidence_score = ?, observed_at = ?, reviewed_at = ?, metadata_json = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.updated_at)
                .bind(&candidate.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE surface_seed_candidates
                     SET program_id = $1, seed_type = $2, seed_value = $3, status = $4, source_asset_id = $5, source_asset_type = $6, source_detail_key = $7, source_display_value = $8, source_canonical_url = $9, confidence_score = $10, observed_at = $11, reviewed_at = $12, metadata_json = $13, updated_at = $14
                     WHERE id = $15",
                )
                .bind(&candidate.program_id)
                .bind(&candidate.seed_type)
                .bind(&candidate.seed_value)
                .bind(&candidate.status)
                .bind(&candidate.source_asset_id)
                .bind(&candidate.source_asset_type)
                .bind(&candidate.source_detail_key)
                .bind(&candidate.source_display_value)
                .bind(&candidate.source_canonical_url)
                .bind(candidate.confidence_score)
                .bind(&candidate.observed_at)
                .bind(&candidate.reviewed_at)
                .bind(&candidate.metadata_json)
                .bind(&candidate.updated_at)
                .bind(&candidate.id)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        Ok(rows > 0)
    }
}
