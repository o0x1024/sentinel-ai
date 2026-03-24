use std::collections::HashMap;

use anyhow::Result;

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{
    SurfaceAssetFilter, SurfaceAssetRow, SurfaceChangeLogRow, SurfaceDiscoveryRunRow,
    SurfaceObservationRow,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceDiscoveryRunDetailResponse {
    pub run: SurfaceDiscoveryRunRow,
    pub assets: Vec<SurfaceAssetRow>,
    pub observations: Vec<SurfaceObservationRow>,
    pub changes: Vec<SurfaceChangeLogRow>,
    pub by_artifact: HashMap<String, i32>,
}

impl DatabaseService {
    pub async fn get_surface_discovery_run(
        &self,
        run_id: &str,
    ) -> Result<Option<SurfaceDiscoveryRunRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT * FROM surface_discovery_runs WHERE id = ? LIMIT 1")
                    .bind(run_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_discovery_runs WHERE id = ? LIMIT 1")
                    .bind(run_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_discovery_runs WHERE id = $1 LIMIT 1")
                    .bind(run_id)
                    .fetch_optional(pool)
                    .await?
            }
        };

        Ok(row)
    }

    pub async fn list_surface_observations(
        &self,
        run_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceObservationRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let mut rows: Vec<SurfaceObservationRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT * FROM surface_observations WHERE run_id = ?")
                    .bind(run_id)
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_observations WHERE run_id = ?")
                    .bind(run_id)
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT * FROM surface_observations WHERE run_id = $1")
                    .bind(run_id)
                    .fetch_all(pool)
                    .await?
            }
        };

        rows.sort_by(|a, b| b.observed_at.cmp(&a.observed_at));

        if let Some(limit) = limit {
            rows.truncate(limit.max(0) as usize);
        }

        Ok(rows)
    }

    pub async fn get_surface_discovery_run_detail(
        &self,
        run_id: &str,
    ) -> Result<Option<SurfaceDiscoveryRunDetailResponse>> {
        let Some(run) = self.get_surface_discovery_run(run_id).await? else {
            return Ok(None);
        };

        let observations = self.list_surface_observations(run_id, Some(200)).await?;
        let mut by_artifact = HashMap::new();
        for observation in &observations {
            *by_artifact
                .entry(observation.artifact_type.clone())
                .or_insert(0) += 1;
        }

        let mut assets = self
            .list_surface_assets(&SurfaceAssetFilter {
                program_id: Some(run.program_id.clone()),
                asset_type: None,
                status: None,
                search: None,
                limit: Some(500),
                offset: Some(0),
            })
            .await?;
        assets.retain(|asset| asset.discovery_task_id.as_deref() == Some(run_id));
        assets.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));

        let mut changes = self
            .list_surface_change_logs(Some(&run.program_id), None, Some(500))
            .await?;
        changes.retain(|change| change.source_run_id.as_deref() == Some(run_id));
        changes.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));

        Ok(Some(SurfaceDiscoveryRunDetailResponse {
            run,
            assets,
            observations,
            changes,
            by_artifact,
        }))
    }
}
