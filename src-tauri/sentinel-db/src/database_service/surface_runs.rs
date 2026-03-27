use std::collections::HashMap;

use anyhow::Result;
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::{
    SurfaceAssetRow, SurfaceChangeLogRow, SurfaceDiscoveryRunRow, SurfaceObservationRow,
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
    fn push_surface_run_filters<'args, DB>(
        query_builder: &mut QueryBuilder<'args, DB>,
        program_id: Option<&str>,
    ) where
        DB: Database,
        String: for<'q> Encode<'q, DB> + Type<DB>,
    {
        if let Some(program_id) = program_id {
            query_builder
                .push(" AND program_id = ")
                .push_bind(program_id.to_string());
        }
    }

    pub async fn list_surface_discovery_runs(
        &self,
        program_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceDiscoveryRunRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let safe_limit = limit.map(|value| value.max(0));
        let sql = "SELECT * FROM surface_discovery_runs WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                Self::push_surface_run_filters(&mut query_builder, program_id);
                query_builder.push(" ORDER BY started_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceDiscoveryRunRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                Self::push_surface_run_filters(&mut query_builder, program_id);
                query_builder.push(" ORDER BY started_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceDiscoveryRunRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                Self::push_surface_run_filters(&mut query_builder, program_id);
                query_builder.push(" ORDER BY started_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceDiscoveryRunRow>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

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

        let safe_limit = limit.map(|value| value.max(0));

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = QueryBuilder::<sqlx::Sqlite>::new(
                    "SELECT * FROM surface_observations WHERE run_id = ",
                );
                query
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query = QueryBuilder::<MySql>::new(
                    "SELECT * FROM surface_observations WHERE run_id = ",
                );
                query
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query = QueryBuilder::<Postgres>::new(
                    "SELECT * FROM surface_observations WHERE run_id = ",
                );
                query
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

    async fn list_surface_assets_for_run(
        &self,
        program_id: &str,
        run_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let safe_limit = limit.map(|value| value.max(0));
        let base_sql = "SELECT * FROM surface_assets WHERE program_id = ";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = QueryBuilder::<sqlx::Sqlite>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND discovery_task_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query = QueryBuilder::<MySql>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND discovery_task_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query = QueryBuilder::<Postgres>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND discovery_task_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

    async fn list_surface_change_logs_for_run(
        &self,
        program_id: &str,
        run_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceChangeLogRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let safe_limit = limit.map(|value| value.max(0));
        let base_sql = "SELECT * FROM surface_change_logs WHERE program_id = ";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = QueryBuilder::<sqlx::Sqlite>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND source_run_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY detected_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceChangeLogRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query = QueryBuilder::<MySql>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND source_run_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY detected_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceChangeLogRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query = QueryBuilder::<Postgres>::new(base_sql);
                query
                    .push_bind(program_id.to_string())
                    .push(" AND source_run_id = ")
                    .push_bind(run_id.to_string())
                    .push(" ORDER BY detected_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query.push(" LIMIT ").push_bind(limit);
                }
                Ok(query
                    .build_query_as::<SurfaceChangeLogRow>()
                    .fetch_all(pool)
                    .await?)
            }
        }
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

        let assets = self
            .list_surface_assets_for_run(&run.program_id, run_id, Some(500))
            .await?;
        let changes = self
            .list_surface_change_logs_for_run(&run.program_id, run_id, Some(500))
            .await?;

        Ok(Some(SurfaceDiscoveryRunDetailResponse {
            run,
            assets,
            observations,
            changes,
            by_artifact,
        }))
    }
}
