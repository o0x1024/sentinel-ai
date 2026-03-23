use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{
    SurfaceChangeLogRow, SurfaceEvidenceRow, SurfaceFingerprintRow,
};
use anyhow::Result;

impl DatabaseService {
    pub async fn create_surface_fingerprint(&self, fingerprint: &SurfaceFingerprintRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_fingerprints (id, program_id, asset_id, fingerprint_type, fingerprint_key, fingerprint_value, confidence_score, source, observed_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&fingerprint.id)
                .bind(&fingerprint.program_id)
                .bind(&fingerprint.asset_id)
                .bind(&fingerprint.fingerprint_type)
                .bind(&fingerprint.fingerprint_key)
                .bind(&fingerprint.fingerprint_value)
                .bind(fingerprint.confidence_score)
                .bind(&fingerprint.source)
                .bind(&fingerprint.observed_at)
                .bind(&fingerprint.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_fingerprints (id, program_id, asset_id, fingerprint_type, fingerprint_key, fingerprint_value, confidence_score, source, observed_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&fingerprint.id)
                .bind(&fingerprint.program_id)
                .bind(&fingerprint.asset_id)
                .bind(&fingerprint.fingerprint_type)
                .bind(&fingerprint.fingerprint_key)
                .bind(&fingerprint.fingerprint_value)
                .bind(fingerprint.confidence_score)
                .bind(&fingerprint.source)
                .bind(&fingerprint.observed_at)
                .bind(&fingerprint.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_fingerprints (id, program_id, asset_id, fingerprint_type, fingerprint_key, fingerprint_value, confidence_score, source, observed_at, metadata_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(&fingerprint.id)
                .bind(&fingerprint.program_id)
                .bind(&fingerprint.asset_id)
                .bind(&fingerprint.fingerprint_type)
                .bind(&fingerprint.fingerprint_key)
                .bind(&fingerprint.fingerprint_value)
                .bind(fingerprint.confidence_score)
                .bind(&fingerprint.source)
                .bind(&fingerprint.observed_at)
                .bind(&fingerprint.metadata_json)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_surface_fingerprints(
        &self,
        program_id: Option<&str>,
        asset_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceFingerprintRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let mut rows: Vec<SurfaceFingerprintRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT * FROM surface_fingerprints")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_fingerprints")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_fingerprints")
                    .fetch_all(pool)
                    .await?
            }
        };

        if let Some(program_id) = program_id {
            rows.retain(|row| row.program_id == program_id);
        }
        if let Some(asset_id) = asset_id {
            rows.retain(|row| row.asset_id == asset_id);
        }

        rows.sort_by(|a, b| b.observed_at.cmp(&a.observed_at));

        if let Some(limit) = limit {
            rows.truncate(limit.max(0) as usize);
        }

        Ok(rows)
    }

    pub async fn create_surface_evidence(&self, evidence: &SurfaceEvidenceRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_evidence (id, program_id, asset_id, evidence_type, title, content_text, content_path, content_json, collected_at, collected_by, probe_node, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&evidence.id)
                .bind(&evidence.program_id)
                .bind(&evidence.asset_id)
                .bind(&evidence.evidence_type)
                .bind(&evidence.title)
                .bind(&evidence.content_text)
                .bind(&evidence.content_path)
                .bind(&evidence.content_json)
                .bind(&evidence.collected_at)
                .bind(&evidence.collected_by)
                .bind(&evidence.probe_node)
                .bind(&evidence.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_evidence (id, program_id, asset_id, evidence_type, title, content_text, content_path, content_json, collected_at, collected_by, probe_node, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&evidence.id)
                .bind(&evidence.program_id)
                .bind(&evidence.asset_id)
                .bind(&evidence.evidence_type)
                .bind(&evidence.title)
                .bind(&evidence.content_text)
                .bind(&evidence.content_path)
                .bind(&evidence.content_json)
                .bind(&evidence.collected_at)
                .bind(&evidence.collected_by)
                .bind(&evidence.probe_node)
                .bind(&evidence.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_evidence (id, program_id, asset_id, evidence_type, title, content_text, content_path, content_json, collected_at, collected_by, probe_node, metadata_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                )
                .bind(&evidence.id)
                .bind(&evidence.program_id)
                .bind(&evidence.asset_id)
                .bind(&evidence.evidence_type)
                .bind(&evidence.title)
                .bind(&evidence.content_text)
                .bind(&evidence.content_path)
                .bind(&evidence.content_json)
                .bind(&evidence.collected_at)
                .bind(&evidence.collected_by)
                .bind(&evidence.probe_node)
                .bind(&evidence.metadata_json)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_surface_evidence(
        &self,
        program_id: Option<&str>,
        asset_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceEvidenceRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let mut rows: Vec<SurfaceEvidenceRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT * FROM surface_evidence")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_evidence")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_evidence")
                    .fetch_all(pool)
                    .await?
            }
        };

        if let Some(program_id) = program_id {
            rows.retain(|row| row.program_id == program_id);
        }
        if let Some(asset_id) = asset_id {
            rows.retain(|row| row.asset_id.as_deref() == Some(asset_id));
        }

        rows.sort_by(|a, b| b.collected_at.cmp(&a.collected_at));

        if let Some(limit) = limit {
            rows.truncate(limit.max(0) as usize);
        }

        Ok(rows)
    }

    pub async fn create_surface_change_log(&self, change_log: &SurfaceChangeLogRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO surface_change_logs (id, program_id, asset_id, relation_id, change_type, old_value_json, new_value_json, summary, detected_at, source_run_id, risk_delta, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&change_log.id)
                .bind(&change_log.program_id)
                .bind(&change_log.asset_id)
                .bind(&change_log.relation_id)
                .bind(&change_log.change_type)
                .bind(&change_log.old_value_json)
                .bind(&change_log.new_value_json)
                .bind(&change_log.summary)
                .bind(&change_log.detected_at)
                .bind(&change_log.source_run_id)
                .bind(change_log.risk_delta)
                .bind(&change_log.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_change_logs (id, program_id, asset_id, relation_id, change_type, old_value_json, new_value_json, summary, detected_at, source_run_id, risk_delta, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&change_log.id)
                .bind(&change_log.program_id)
                .bind(&change_log.asset_id)
                .bind(&change_log.relation_id)
                .bind(&change_log.change_type)
                .bind(&change_log.old_value_json)
                .bind(&change_log.new_value_json)
                .bind(&change_log.summary)
                .bind(&change_log.detected_at)
                .bind(&change_log.source_run_id)
                .bind(change_log.risk_delta)
                .bind(&change_log.metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO surface_change_logs (id, program_id, asset_id, relation_id, change_type, old_value_json, new_value_json, summary, detected_at, source_run_id, risk_delta, metadata_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                )
                .bind(&change_log.id)
                .bind(&change_log.program_id)
                .bind(&change_log.asset_id)
                .bind(&change_log.relation_id)
                .bind(&change_log.change_type)
                .bind(&change_log.old_value_json)
                .bind(&change_log.new_value_json)
                .bind(&change_log.summary)
                .bind(&change_log.detected_at)
                .bind(&change_log.source_run_id)
                .bind(change_log.risk_delta)
                .bind(&change_log.metadata_json)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_surface_change_logs(
        &self,
        program_id: Option<&str>,
        asset_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceChangeLogRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let mut rows: Vec<SurfaceChangeLogRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT * FROM surface_change_logs")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_change_logs")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_change_logs")
                    .fetch_all(pool)
                    .await?
            }
        };

        if let Some(program_id) = program_id {
            rows.retain(|row| row.program_id == program_id);
        }
        if let Some(asset_id) = asset_id {
            rows.retain(|row| row.asset_id.as_deref() == Some(asset_id));
        }

        rows.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));

        if let Some(limit) = limit {
            rows.truncate(limit.max(0) as usize);
        }

        Ok(rows)
    }
}
