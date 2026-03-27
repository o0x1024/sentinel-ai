use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, MySqlPool, MySqlRow, PgPool, PgRow, Postgres};
use crate::database_service::surface::{
    SurfaceAssetRow, SurfaceCertAssetRow, SurfaceDomainAssetRow, SurfaceHostAssetRow,
    SurfaceIpAssetRow, SurfaceOrgAssetRow, SurfacePortAssetRow, SurfaceRelationFilter,
    SurfaceRelationRow, SurfaceServiceAssetRow, SurfaceWebAssetRow,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Encode, QueryBuilder, Type};
use std::collections::HashMap;

async fn fetch_surface_extension_map_sqlite<T, F>(
    pool: &sqlx::SqlitePool,
    table: &str,
    asset_ids: &[String],
    asset_id_of: F,
) -> Result<HashMap<String, Value>>
where
    String: for<'q> Encode<'q, sqlx::Sqlite> + Type<sqlx::Sqlite>,
    for<'row> T: Serialize + Send + Unpin + sqlx::FromRow<'row, sqlx::sqlite::SqliteRow>,
    F: Fn(&T) -> &str,
{
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut query_builder =
        QueryBuilder::<sqlx::Sqlite>::new(format!("SELECT * FROM {table} WHERE asset_id IN ("));
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");

    let rows: Vec<T> = query_builder.build_query_as().fetch_all(pool).await?;
    let mut details_by_id = HashMap::with_capacity(rows.len());

    for row in rows {
        let asset_id = asset_id_of(&row).to_string();
        details_by_id.insert(asset_id, serde_json::to_value(row)?);
    }

    Ok(details_by_id)
}

async fn fetch_surface_extension_map_mysql<T, F>(
    pool: &MySqlPool,
    table: &str,
    asset_ids: &[String],
    asset_id_of: F,
) -> Result<HashMap<String, Value>>
where
    String: for<'q> Encode<'q, MySql> + Type<MySql>,
    for<'row> T: Serialize + Send + Unpin + sqlx::FromRow<'row, MySqlRow>,
    F: Fn(&T) -> &str,
{
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut query_builder =
        QueryBuilder::<MySql>::new(format!("SELECT * FROM {table} WHERE asset_id IN ("));
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");

    let rows: Vec<T> = query_builder.build_query_as().fetch_all(pool).await?;
    let mut details_by_id = HashMap::with_capacity(rows.len());

    for row in rows {
        let asset_id = asset_id_of(&row).to_string();
        details_by_id.insert(asset_id, serde_json::to_value(row)?);
    }

    Ok(details_by_id)
}

async fn fetch_surface_extension_map_postgres<T, F>(
    pool: &PgPool,
    table: &str,
    asset_ids: &[String],
    asset_id_of: F,
) -> Result<HashMap<String, Value>>
where
    String: for<'q> Encode<'q, Postgres> + Type<Postgres>,
    for<'row> T: Serialize + Send + Unpin + sqlx::FromRow<'row, PgRow>,
    F: Fn(&T) -> &str,
{
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut query_builder =
        QueryBuilder::<Postgres>::new(format!("SELECT * FROM {table} WHERE asset_id IN ("));
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");

    let rows: Vec<T> = query_builder.build_query_as().fetch_all(pool).await?;
    let mut details_by_id = HashMap::with_capacity(rows.len());

    for row in rows {
        let asset_id = asset_id_of(&row).to_string();
        details_by_id.insert(asset_id, serde_json::to_value(row)?);
    }

    Ok(details_by_id)
}

async fn fetch_surface_typed_details_for_sqlite(
    pool: &sqlx::SqlitePool,
    asset_type: &str,
    asset_ids: &[String],
) -> Result<HashMap<String, Value>> {
    match asset_type {
        "org" => {
            fetch_surface_extension_map_sqlite::<SurfaceOrgAssetRow, _>(
                pool,
                "surface_org_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "domain" => {
            fetch_surface_extension_map_sqlite::<SurfaceDomainAssetRow, _>(
                pool,
                "surface_domain_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "ip" => {
            fetch_surface_extension_map_sqlite::<SurfaceIpAssetRow, _>(
                pool,
                "surface_ip_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "host" => {
            fetch_surface_extension_map_sqlite::<SurfaceHostAssetRow, _>(
                pool,
                "surface_host_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "port" => {
            fetch_surface_extension_map_sqlite::<SurfacePortAssetRow, _>(
                pool,
                "surface_port_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "service" => {
            fetch_surface_extension_map_sqlite::<SurfaceServiceAssetRow, _>(
                pool,
                "surface_service_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "web" => {
            fetch_surface_extension_map_sqlite::<SurfaceWebAssetRow, _>(
                pool,
                "surface_web_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "certificate" => {
            fetch_surface_extension_map_sqlite::<SurfaceCertAssetRow, _>(
                pool,
                "surface_cert_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        _ => Ok(HashMap::new()),
    }
}

async fn fetch_surface_typed_details_for_mysql(
    pool: &MySqlPool,
    asset_type: &str,
    asset_ids: &[String],
) -> Result<HashMap<String, Value>> {
    match asset_type {
        "org" => {
            fetch_surface_extension_map_mysql::<SurfaceOrgAssetRow, _>(
                pool,
                "surface_org_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "domain" => {
            fetch_surface_extension_map_mysql::<SurfaceDomainAssetRow, _>(
                pool,
                "surface_domain_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "ip" => {
            fetch_surface_extension_map_mysql::<SurfaceIpAssetRow, _>(
                pool,
                "surface_ip_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "host" => {
            fetch_surface_extension_map_mysql::<SurfaceHostAssetRow, _>(
                pool,
                "surface_host_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "port" => {
            fetch_surface_extension_map_mysql::<SurfacePortAssetRow, _>(
                pool,
                "surface_port_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "service" => {
            fetch_surface_extension_map_mysql::<SurfaceServiceAssetRow, _>(
                pool,
                "surface_service_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "web" => {
            fetch_surface_extension_map_mysql::<SurfaceWebAssetRow, _>(
                pool,
                "surface_web_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "certificate" => {
            fetch_surface_extension_map_mysql::<SurfaceCertAssetRow, _>(
                pool,
                "surface_cert_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        _ => Ok(HashMap::new()),
    }
}

async fn fetch_surface_typed_details_for_postgres(
    pool: &PgPool,
    asset_type: &str,
    asset_ids: &[String],
) -> Result<HashMap<String, Value>> {
    match asset_type {
        "org" => {
            fetch_surface_extension_map_postgres::<SurfaceOrgAssetRow, _>(
                pool,
                "surface_org_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "domain" => {
            fetch_surface_extension_map_postgres::<SurfaceDomainAssetRow, _>(
                pool,
                "surface_domain_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "ip" => {
            fetch_surface_extension_map_postgres::<SurfaceIpAssetRow, _>(
                pool,
                "surface_ip_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "host" => {
            fetch_surface_extension_map_postgres::<SurfaceHostAssetRow, _>(
                pool,
                "surface_host_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "port" => {
            fetch_surface_extension_map_postgres::<SurfacePortAssetRow, _>(
                pool,
                "surface_port_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "service" => {
            fetch_surface_extension_map_postgres::<SurfaceServiceAssetRow, _>(
                pool,
                "surface_service_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "web" => {
            fetch_surface_extension_map_postgres::<SurfaceWebAssetRow, _>(
                pool,
                "surface_web_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        "certificate" => {
            fetch_surface_extension_map_postgres::<SurfaceCertAssetRow, _>(
                pool,
                "surface_cert_assets",
                asset_ids,
                |row| row.asset_id.as_str(),
            )
            .await
        }
        _ => Ok(HashMap::new()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAssetRelationDetail {
    pub relation: SurfaceRelationRow,
    pub direction: String,
    pub peer_asset: SurfaceAssetRow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAssetDetailResponse {
    pub asset: SurfaceAssetRow,
    pub typed_details: Option<Value>,
    pub relations: Vec<SurfaceAssetRelationDetail>,
    pub fingerprints: Vec<crate::database_service::surface::SurfaceFingerprintRow>,
    pub evidence: Vec<crate::database_service::surface::SurfaceEvidenceRow>,
    pub changes: Vec<crate::database_service::surface::SurfaceChangeLogRow>,
}

impl DatabaseService {
    pub async fn get_surface_asset_by_id(&self, asset_id: &str) -> Result<Option<SurfaceAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = $1 LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub(crate) async fn get_surface_typed_details(
        &self,
        asset: &SurfaceAssetRow,
    ) -> Result<Option<Value>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match asset.asset_type.as_str() {
            "org" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceOrgAssetRow>(
                    "SELECT * FROM surface_org_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceOrgAssetRow>(
                    "SELECT * FROM surface_org_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceOrgAssetRow>(
                    "SELECT * FROM surface_org_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "domain" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                    "SELECT * FROM surface_domain_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                    "SELECT * FROM surface_domain_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                    "SELECT * FROM surface_domain_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "ip" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "host" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "port" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "service" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                    "SELECT * FROM surface_service_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                    "SELECT * FROM surface_service_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                    "SELECT * FROM surface_service_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "web" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "certificate" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            _ => Ok(None),
        }
    }

    pub(crate) async fn list_surface_typed_details_map(
        &self,
        assets: &[SurfaceAssetRow],
    ) -> Result<HashMap<String, Value>> {
        if assets.is_empty() {
            return Ok(HashMap::new());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let mut asset_ids_by_type: HashMap<&str, Vec<String>> = HashMap::new();

        for asset in assets {
            asset_ids_by_type
                .entry(asset.asset_type.as_str())
                .or_default()
                .push(asset.id.clone());
        }

        let mut typed_details_by_id = HashMap::new();

        match runtime {
            DatabasePool::SQLite(pool) => {
                for (asset_type, asset_ids) in asset_ids_by_type {
                    typed_details_by_id.extend(
                        fetch_surface_typed_details_for_sqlite(pool, asset_type, &asset_ids)
                            .await?,
                    );
                }
            }
            DatabasePool::MySQL(pool) => {
                for (asset_type, asset_ids) in asset_ids_by_type {
                    typed_details_by_id.extend(
                        fetch_surface_typed_details_for_mysql(pool, asset_type, &asset_ids).await?,
                    );
                }
            }
            DatabasePool::PostgreSQL(pool) => {
                for (asset_type, asset_ids) in asset_ids_by_type {
                    typed_details_by_id.extend(
                        fetch_surface_typed_details_for_postgres(pool, asset_type, &asset_ids)
                            .await?,
                    );
                }
            }
        }

        Ok(typed_details_by_id)
    }

    pub async fn get_surface_asset_detail(
        &self,
        asset_id: &str,
    ) -> Result<Option<SurfaceAssetDetailResponse>> {
        let Some(asset) = self.get_surface_asset_by_id(asset_id).await? else {
            return Ok(None);
        };

        let typed_details = self.get_surface_typed_details(&asset).await?;
        let fingerprints = self
            .list_surface_fingerprints(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;
        let evidence = self
            .list_surface_evidence(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;
        let changes = self
            .list_surface_change_logs(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;

        let mut relations = Vec::new();
        for relation in self
            .list_surface_relations(&SurfaceRelationFilter {
                program_id: Some(asset.program_id.clone()),
                asset_id: Some(asset.id.clone()),
                relation_type: None,
                limit: Some(50),
            })
            .await?
        {
            let (peer_id, direction) = if relation.from_asset_id == asset.id {
                (relation.to_asset_id.clone(), "outgoing")
            } else {
                (relation.from_asset_id.clone(), "incoming")
            };

            if let Some(peer_asset) = self.get_surface_asset_by_id(&peer_id).await? {
                relations.push(SurfaceAssetRelationDetail {
                    relation,
                    direction: direction.to_string(),
                    peer_asset,
                });
            }
        }

        Ok(Some(SurfaceAssetDetailResponse {
            asset,
            typed_details,
            relations,
            fingerprints,
            evidence,
            changes,
        }))
    }
}
