use std::collections::{HashMap, HashSet};

use anyhow::Result;
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::{
    SurfaceAssetRow, SurfaceChangeLogRow, SurfaceDiscoveryRunRow, SurfaceObservationRow,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn surface_observation_item_count(payload_json: &str) -> i32 {
    match serde_json::from_str::<Value>(payload_json) {
        Ok(Value::Array(items)) => items.len().try_into().unwrap_or(i32::MAX),
        Ok(Value::Null) | Err(_) => 0,
        Ok(_) => 1,
    }
}

fn normalize_surface_observed_asset_key(asset_type: &str, asset_key: &str) -> String {
    let trimmed = asset_key.trim();
    if asset_type != "web" || trimmed.contains('?') || trimmed.contains('#') {
        return trimmed.to_string();
    }
    trimmed.trim_end_matches('/').to_string()
}

fn extract_observed_asset_identities(
    observations: &[SurfaceObservationRow],
) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut identities = Vec::new();

    for observation in observations {
        let Ok(payload) = serde_json::from_str::<Value>(&observation.payload_json) else {
            continue;
        };
        let items: Vec<&Value> = match &payload {
            Value::Array(values) => values.iter().collect(),
            Value::Object(_) => vec![&payload],
            _ => Vec::new(),
        };

        for item in items {
            let Some(asset_type) = item.get("asset_type").and_then(Value::as_str) else {
                continue;
            };
            let Some(asset_key) = item.get("asset_key").and_then(Value::as_str) else {
                continue;
            };
            let asset_type = asset_type.trim();
            let asset_key = normalize_surface_observed_asset_key(asset_type, asset_key);
            if asset_type.is_empty() || asset_key.is_empty() {
                continue;
            }
            let identity = (asset_type.to_string(), asset_key);
            if seen.insert(identity.clone()) {
                identities.push(identity);
            }
        }
    }

    identities
}

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

    pub async fn list_surface_observations_filtered(
        &self,
        program_id: Option<&str>,
        source_plugin: Option<&str>,
        artifact_type: Option<&str>,
        object_key: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceObservationRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let safe_limit = limit.map(|value| value.max(0));
        let sql = "SELECT * FROM surface_observations WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                if let Some(object_key) = object_key {
                    query_builder
                        .push(" AND object_key = ")
                        .push_bind(object_key.to_string());
                }
                query_builder.push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                if let Some(object_key) = object_key {
                    query_builder
                        .push(" AND object_key = ")
                        .push_bind(object_key.to_string());
                }
                query_builder.push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                if let Some(object_key) = object_key {
                    query_builder
                        .push(" AND object_key = ")
                        .push_bind(object_key.to_string());
                }
                query_builder.push(" ORDER BY observed_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    query_builder.push(" LIMIT ").push_bind(limit);
                }
                Ok(query_builder
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

    pub async fn list_latest_surface_observations_by_target(
        &self,
        program_id: Option<&str>,
        source_plugin: Option<&str>,
        artifact_type: Option<&str>,
    ) -> Result<Vec<SurfaceObservationRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT id, run_id, program_id, artifact_type, object_key, payload_json, source_plugin, confidence_score, observed_at, normalized, metadata_json FROM (SELECT id, run_id, program_id, artifact_type, object_key, payload_json, source_plugin, confidence_score, observed_at, normalized, metadata_json, ROW_NUMBER() OVER (PARTITION BY program_id, source_plugin, artifact_type, object_key ORDER BY observed_at DESC, id DESC) AS rn FROM surface_observations WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                query_builder.push(") latest WHERE rn = 1 ORDER BY observed_at DESC, id DESC");
                Ok(query_builder
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                query_builder.push(") latest WHERE rn = 1 ORDER BY observed_at DESC, id DESC");
                Ok(query_builder
                    .build_query_as::<SurfaceObservationRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                if let Some(program_id) = program_id {
                    query_builder
                        .push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                if let Some(source_plugin) = source_plugin {
                    query_builder
                        .push(" AND source_plugin = ")
                        .push_bind(source_plugin.to_string());
                }
                if let Some(artifact_type) = artifact_type {
                    query_builder
                        .push(" AND artifact_type = ")
                        .push_bind(artifact_type.to_string());
                }
                query_builder.push(") latest WHERE rn = 1 ORDER BY observed_at DESC, id DESC");
                Ok(query_builder
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

    fn push_surface_asset_identity_filters<'args, DB>(
        query: &mut QueryBuilder<'args, DB>,
        identities: &[(String, String)],
    ) where
        DB: Database,
        String: for<'q> Encode<'q, DB> + Type<DB>,
    {
        query.push(" AND (");
        for (index, (asset_type, asset_name)) in identities.iter().enumerate() {
            if index > 0 {
                query.push(" OR ");
            }
            query
                .push("(")
                .push("asset_type = ")
                .push_bind(asset_type.clone())
                .push(" AND asset_name = ")
                .push_bind(asset_name.clone())
                .push(")");
        }
        query.push(")");
    }

    async fn list_surface_assets_by_identities(
        &self,
        program_id: &str,
        identities: &[(String, String)],
        limit: Option<i64>,
    ) -> Result<Vec<SurfaceAssetRow>> {
        if identities.is_empty() {
            return Ok(Vec::new());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let safe_limit = limit.map(|value| value.max(0));
        let base_sql = "SELECT * FROM surface_assets WHERE program_id = ";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query = QueryBuilder::<sqlx::Sqlite>::new(base_sql);
                query.push_bind(program_id.to_string());
                Self::push_surface_asset_identity_filters(&mut query, identities);
                query.push(" ORDER BY last_seen_at DESC, id DESC");
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
                query.push_bind(program_id.to_string());
                Self::push_surface_asset_identity_filters(&mut query, identities);
                query.push(" ORDER BY last_seen_at DESC, id DESC");
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
                query.push_bind(program_id.to_string());
                Self::push_surface_asset_identity_filters(&mut query, identities);
                query.push(" ORDER BY last_seen_at DESC, id DESC");
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

    fn merge_surface_run_assets(
        created_assets: Vec<SurfaceAssetRow>,
        observed_assets: Vec<SurfaceAssetRow>,
    ) -> Vec<SurfaceAssetRow> {
        let mut seen = HashSet::new();
        let mut assets = Vec::new();

        for asset in created_assets.into_iter().chain(observed_assets) {
            if seen.insert(asset.id.clone()) {
                assets.push(asset);
            }
        }

        assets.sort_by(|left, right| {
            right
                .last_seen_at
                .cmp(&left.last_seen_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        assets
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
                .or_insert(0) += surface_observation_item_count(&observation.payload_json);
        }

        let created_assets = self
            .list_surface_assets_for_run(&run.program_id, run_id, Some(500))
            .await?;
        let observed_identities = extract_observed_asset_identities(&observations);
        let observed_assets = self
            .list_surface_assets_by_identities(&run.program_id, &observed_identities, Some(500))
            .await?;
        let assets = Self::merge_surface_run_assets(created_assets, observed_assets);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(artifact_type: &str, payload_json: &str) -> SurfaceObservationRow {
        SurfaceObservationRow {
            id: "obs-1".to_string(),
            run_id: "run-1".to_string(),
            program_id: "program-1".to_string(),
            artifact_type: artifact_type.to_string(),
            object_key: None,
            payload_json: payload_json.to_string(),
            source_plugin: Some("favicon_fingerprinter".to_string()),
            confidence_score: None,
            observed_at: "2026-05-09T00:00:00Z".to_string(),
            normalized: true,
            metadata_json: None,
        }
    }

    #[test]
    fn observation_item_count_counts_array_items() {
        assert_eq!(
            surface_observation_item_count(
                r#"[{"asset_key":"https://a"},{"asset_key":"https://b"}]"#
            ),
            2
        );
        assert_eq!(
            surface_observation_item_count(r#"{"asset_key":"https://a"}"#),
            1
        );
        assert_eq!(surface_observation_item_count("not-json"), 0);
    }

    #[test]
    fn extract_observed_asset_identities_normalizes_web_keys() {
        let observations = vec![observation(
            "evidences",
            r#"[
                {"asset_type":"web","asset_key":"https://example.com/"},
                {"asset_type":"web","asset_key":"https://example.com"},
                {"asset_type":"domain","asset_key":"example.com/"}
            ]"#,
        )];

        let identities = extract_observed_asset_identities(&observations);

        assert_eq!(
            identities,
            vec![
                ("web".to_string(), "https://example.com".to_string()),
                ("domain".to_string(), "example.com/".to_string())
            ]
        );
    }

    #[test]
    fn surface_asset_identity_filter_generates_valid_or_groups() {
        let identities = vec![
            ("web".to_string(), "https://example.com".to_string()),
            ("web".to_string(), "https://app.example.com".to_string()),
        ];
        let mut query_builder =
            QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE program_id = ");
        query_builder.push_bind("program-1".to_string());

        DatabaseService::push_surface_asset_identity_filters(&mut query_builder, &identities);

        let sql = query_builder.sql();
        assert!(sql.contains(
            "AND ((asset_type = ? AND asset_name = ?) OR (asset_type = ? AND asset_name = ?))"
        ));
        assert!(!sql.contains("( OR"));
        assert!(!sql.contains("OR )"));
    }
}
