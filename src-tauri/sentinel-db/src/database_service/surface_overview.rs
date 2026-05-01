use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::{SurfaceAssetFilter, SurfaceOverview};
use crate::database_service::surface_asset_query::push_surface_asset_filters;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct SurfaceOverviewCountsRow {
    total_assets: i64,
    active_assets: Option<i64>,
    high_risk_assets: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct SurfaceAssetTypeCountRow {
    asset_type: String,
    count: i64,
}

impl DatabaseService {
    fn push_program_filter<'args, DB>(
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

    async fn get_surface_overview_counts(
        &self,
        program_id: Option<&str>,
    ) -> Result<SurfaceOverviewCountsRow> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"
            SELECT
                COUNT(*) AS total_assets,
                SUM(CASE WHEN LOWER(status) = 'active' THEN 1 ELSE 0 END) AS active_assets,
                SUM(CASE WHEN LOWER(COALESCE(risk_level, '')) IN ('high', 'critical') THEN 1 ELSE 0 END) AS high_risk_assets
            FROM surface_assets
            WHERE 1=1
        "#;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                Ok(query_builder
                    .build_query_as::<SurfaceOverviewCountsRow>()
                    .fetch_one(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                Ok(query_builder
                    .build_query_as::<SurfaceOverviewCountsRow>()
                    .fetch_one(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                Ok(query_builder
                    .build_query_as::<SurfaceOverviewCountsRow>()
                    .fetch_one(pool)
                    .await?)
            }
        }
    }

    async fn get_surface_asset_type_counts_for_program(
        &self,
        program_id: Option<&str>,
    ) -> Result<HashMap<String, i32>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = r#"
            SELECT asset_type, COUNT(*) AS count
            FROM surface_assets
            WHERE 1=1
        "#;

        let rows: Vec<SurfaceAssetTypeCountRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY asset_type");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY asset_type");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY asset_type");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| (row.asset_type, row.count as i32))
            .collect())
    }

    pub async fn count_surface_assets_by_type(
        &self,
        filter: &SurfaceAssetFilter,
    ) -> Result<HashMap<String, i32>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = r#"
            SELECT asset_type, COUNT(*) AS count
            FROM surface_assets
            WHERE 1=1
        "#;

        let rows: Vec<SurfaceAssetTypeCountRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" GROUP BY asset_type ORDER BY COUNT(*) DESC, asset_type ASC");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" GROUP BY asset_type ORDER BY COUNT(*) DESC, asset_type ASC");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" GROUP BY asset_type ORDER BY COUNT(*) DESC, asset_type ASC");
                query_builder
                    .build_query_as::<SurfaceAssetTypeCountRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| (row.asset_type, row.count as i32))
            .collect())
    }

    async fn count_surface_relations_for_program(&self, program_id: Option<&str>) -> Result<i32> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT COUNT(*) FROM surface_relations WHERE 1=1";

        let count = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                Self::push_program_filter(&mut query_builder, program_id);
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
        };

        Ok(count as i32)
    }

    pub async fn get_surface_overview(&self, program_id: Option<&str>) -> Result<SurfaceOverview> {
        let counts = self.get_surface_overview_counts(program_id).await?;
        let by_type = self.get_surface_asset_type_counts_for_program(program_id).await?;
        let total_relations = self.count_surface_relations_for_program(program_id).await?;
        let runs = self
            .list_surface_discovery_runs(program_id, Some(20))
            .await?;
        let recent_changes = runs
            .iter()
            .map(|run| run.changed_asset_count.unwrap_or(0))
            .sum::<i32>();

        Ok(SurfaceOverview {
            total_assets: counts.total_assets as i32,
            active_assets: counts.active_assets.unwrap_or(0) as i32,
            high_risk_assets: counts.high_risk_assets.unwrap_or(0) as i32,
            by_type,
            total_relations,
            recent_changes,
            recent_runs: runs.len() as i32,
        })
    }
}
