use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::SurfaceAssetFilter;
use crate::database_service::surface_asset_query::push_surface_asset_filters;
use anyhow::Result;
use sqlx::QueryBuilder;
use std::collections::HashSet;

const SURFACE_DELETE_BATCH_SIZE: usize = 200;

const SURFACE_EXTENSION_TABLES: &[&str] = &[
    "surface_org_assets",
    "surface_domain_assets",
    "surface_ip_assets",
    "surface_host_assets",
    "surface_port_assets",
    "surface_service_assets",
    "surface_web_assets",
    "surface_cert_assets",
];

const SURFACE_RELATED_ASSET_TABLES: &[&str] = &[
    "surface_fingerprints",
    "surface_asset_classifications",
    "surface_evidence",
    "surface_change_logs",
];

fn dedupe_asset_ids(asset_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::with_capacity(asset_ids.len());
    let mut deduped = Vec::with_capacity(asset_ids.len());

    for asset_id in asset_ids {
        let normalized = asset_id.trim();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            deduped.push(normalized.to_string());
        }
    }

    deduped
}

fn push_bind_ids<'args, DB>(query_builder: &mut QueryBuilder<'args, DB>, asset_ids: &[String])
where
    DB: sqlx::Database,
    String: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let mut separated = query_builder.separated(", ");
    for asset_id in asset_ids {
        separated.push_bind(asset_id.clone());
    }
}

fn build_delete_by_asset_id_query<'args, DB>(
    table: &str,
    asset_ids: &[String],
) -> QueryBuilder<'args, DB>
where
    DB: sqlx::Database,
    String: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let mut query_builder =
        QueryBuilder::<DB>::new(format!("DELETE FROM {table} WHERE asset_id IN ("));
    push_bind_ids(&mut query_builder, asset_ids);
    query_builder.push(")");
    query_builder
}

fn build_delete_relations_query<'args, DB>(asset_ids: &[String]) -> QueryBuilder<'args, DB>
where
    DB: sqlx::Database,
    String: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let mut query_builder =
        QueryBuilder::<DB>::new("DELETE FROM surface_relations WHERE from_asset_id IN (");
    push_bind_ids(&mut query_builder, asset_ids);
    query_builder.push(") OR to_asset_id IN (");
    push_bind_ids(&mut query_builder, asset_ids);
    query_builder.push(")");
    query_builder
}

fn build_delete_surface_assets_query<'args, DB>(asset_ids: &[String]) -> QueryBuilder<'args, DB>
where
    DB: sqlx::Database,
    String: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let mut query_builder = QueryBuilder::<DB>::new("DELETE FROM surface_assets WHERE id IN (");
    push_bind_ids(&mut query_builder, asset_ids);
    query_builder.push(")");
    query_builder
}

fn dedupe_observation_targets(targets: &[(String, String)]) -> Vec<(String, String)> {
    let mut seen = HashSet::with_capacity(targets.len());
    let mut deduped = Vec::with_capacity(targets.len());

    for (program_id, object_key) in targets {
        let normalized_program_id = program_id.trim();
        let normalized_object_key = object_key.trim();
        if normalized_program_id.is_empty() || normalized_object_key.is_empty() {
            continue;
        }

        let target = (
            normalized_program_id.to_string(),
            normalized_object_key.to_string(),
        );
        if seen.insert(target.clone()) {
            deduped.push(target);
        }
    }

    deduped
}

fn build_delete_surface_observations_query<'args, DB>(
    source_plugin: &str,
    artifact_type: &str,
    targets: &[(String, String)],
) -> QueryBuilder<'args, DB>
where
    DB: sqlx::Database,
    String: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    let mut query_builder =
        QueryBuilder::<DB>::new("DELETE FROM surface_observations WHERE source_plugin = ");
    query_builder
        .push_bind(source_plugin.to_string())
        .push(" AND artifact_type = ")
        .push_bind(artifact_type.to_string())
        .push(" AND (");

    let mut first = true;
    for (program_id, object_key) in targets {
        if !first {
            query_builder.push(" OR ");
        }
        first = false;
        query_builder
            .push("(")
            .push("program_id = ")
            .push_bind(program_id.clone())
            .push(" AND object_key = ")
            .push_bind(object_key.clone())
            .push(")");
    }

    query_builder.push(")");
    query_builder
}

impl DatabaseService {
    async fn list_surface_asset_ids(&self, filter: &SurfaceAssetFilter) -> Result<Vec<String>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder =
                    QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                Ok(query_builder
                    .build_query_scalar::<String>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("SELECT id FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                Ok(query_builder
                    .build_query_scalar::<String>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("SELECT id FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                Ok(query_builder
                    .build_query_scalar::<String>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

    pub async fn delete_surface_assets_by_ids(&self, asset_ids: &[String]) -> Result<usize> {
        let asset_ids = dedupe_asset_ids(asset_ids);
        if asset_ids.is_empty() {
            return Ok(0);
        }

        let runtime = self.get_runtime_pool()?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in asset_ids.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    for table in SURFACE_EXTENSION_TABLES {
                        build_delete_by_asset_id_query::<sqlx::Sqlite>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    build_delete_relations_query::<sqlx::Sqlite>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?;

                    for table in SURFACE_RELATED_ASSET_TABLES {
                        build_delete_by_asset_id_query::<sqlx::Sqlite>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    deleted += build_delete_surface_assets_query::<sqlx::Sqlite>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in asset_ids.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    for table in SURFACE_EXTENSION_TABLES {
                        build_delete_by_asset_id_query::<MySql>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    build_delete_relations_query::<MySql>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?;

                    for table in SURFACE_RELATED_ASSET_TABLES {
                        build_delete_by_asset_id_query::<MySql>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    deleted += build_delete_surface_assets_query::<MySql>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in asset_ids.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    for table in SURFACE_EXTENSION_TABLES {
                        build_delete_by_asset_id_query::<Postgres>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    build_delete_relations_query::<Postgres>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?;

                    for table in SURFACE_RELATED_ASSET_TABLES {
                        build_delete_by_asset_id_query::<Postgres>(table, batch)
                            .build()
                            .execute(&mut *tx)
                            .await?;
                    }

                    deleted += build_delete_surface_assets_query::<Postgres>(batch)
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
        }
    }

    pub async fn delete_surface_observations_by_targets(
        &self,
        source_plugin: &str,
        artifact_type: &str,
        targets: &[(String, String)],
    ) -> Result<usize> {
        let targets = dedupe_observation_targets(targets);
        if targets.is_empty() {
            return Ok(0);
        }

        let runtime = self.get_runtime_pool()?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in targets.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    deleted += build_delete_surface_observations_query::<sqlx::Sqlite>(
                        source_plugin,
                        artifact_type,
                        batch,
                    )
                    .build()
                    .execute(&mut *tx)
                    .await?
                    .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in targets.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    deleted += build_delete_surface_observations_query::<MySql>(
                        source_plugin,
                        artifact_type,
                        batch,
                    )
                    .build()
                    .execute(&mut *tx)
                    .await?
                    .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0usize;

                for batch in targets.chunks(SURFACE_DELETE_BATCH_SIZE) {
                    deleted += build_delete_surface_observations_query::<Postgres>(
                        source_plugin,
                        artifact_type,
                        batch,
                    )
                    .build()
                    .execute(&mut *tx)
                    .await?
                    .rows_affected() as usize;
                }

                tx.commit().await?;
                Ok(deleted)
            }
        }
    }

    pub async fn delete_surface_inventory(&self, filter: &SurfaceAssetFilter) -> Result<usize> {
        let delete_filter = SurfaceAssetFilter {
            program_id: filter.program_id.clone(),
            asset_type: filter.asset_type.clone(),
            status: filter.status.clone(),
            search: filter.search.clone(),
            favicon_hash: filter.favicon_hash.clone(),
            has_favicon_hash: filter.has_favicon_hash,
            http_status_code: filter.http_status_code,
            service_name: filter.service_name.clone(),
            transport_protocol: filter.transport_protocol.clone(),
            view_state: filter.view_state.clone(),
            limit: None,
            offset: None,
        };
        let asset_ids = self.list_surface_asset_ids(&delete_filter).await?;
        self.delete_surface_assets_by_ids(&asset_ids).await
    }
}
