use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::sqlx_compat::{MySqlPool, PgPool};
use crate::database_service::surface::{SurfaceAssetFilter, SurfaceAssetRow};
use crate::database_service::surface_asset_query::push_surface_asset_filters;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInventoryItem {
    pub asset: SurfaceAssetRow,
    pub typed_details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInventoryCursor {
    pub last_seen_at: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInventoryResponse {
    pub items: Vec<SurfaceInventoryItem>,
    pub total: Option<i64>,
    pub has_next: bool,
    pub next_cursor: Option<SurfaceInventoryCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceFacetBucket {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceInventoryFacetsResponse {
    pub service_names: Vec<SurfaceFacetBucket>,
    pub transport_protocols: Vec<SurfaceFacetBucket>,
    pub http_status_codes: Vec<SurfaceFacetBucket>,
}

#[derive(Debug, Clone, FromRow)]
struct FacetBucketRow {
    value: String,
    count: i64,
}

impl DatabaseService {
    pub async fn list_surface_inventory(
        &self,
        filter: &SurfaceAssetFilter,
        cursor: Option<&SurfaceInventoryCursor>,
    ) -> Result<SurfaceInventoryResponse> {
        let page_limit = filter.limit.unwrap_or(100).clamp(1, 1_000);
        let query_filter = SurfaceAssetFilter {
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
            is_favorite: filter.is_favorite,
            column_filters: filter.column_filters.clone(),
            limit: Some(page_limit + 1),
            offset: None,
        };

        let mut assets = self
            .list_surface_assets_after(&query_filter, cursor)
            .await?;
        let has_next = assets.len() > page_limit as usize;
        if has_next {
            assets.truncate(page_limit as usize);
        }
        let next_cursor = if has_next {
            assets.last().map(|asset| SurfaceInventoryCursor {
                last_seen_at: asset.last_seen_at.clone(),
                id: asset.id.clone(),
            })
        } else {
            None
        };
        let mut typed_details_by_id = self.list_surface_typed_details_map(&assets).await?;
        let mut rows = Vec::with_capacity(assets.len());

        for asset in assets {
            rows.push(SurfaceInventoryItem {
                typed_details: typed_details_by_id.remove(&asset.id),
                asset,
            });
        }

        Ok(SurfaceInventoryResponse {
            items: rows,
            total: None,
            has_next,
            next_cursor,
        })
    }

    pub async fn get_surface_inventory_facets(
        &self,
        filter: &SurfaceAssetFilter,
    ) -> Result<SurfaceInventoryFacetsResponse> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let mut base_filter = filter.clone();
        if base_filter.asset_type.as_deref() == Some("service") {
            base_filter.asset_type = Some("service".to_string());
        }
        if base_filter.asset_type.as_deref() == Some("web") {
            base_filter.asset_type = Some("web".to_string());
        }
        base_filter.service_name = None;
        base_filter.transport_protocol = None;
        base_filter.http_status_code = None;
        base_filter.limit = None;
        base_filter.offset = None;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let service_names = fetch_service_name_facets_sqlite(pool, &base_filter).await?;
                let transport_protocols =
                    fetch_service_transport_facets_sqlite(pool, &base_filter).await?;
                let http_status_codes =
                    fetch_web_http_status_facets_sqlite(pool, &base_filter).await?;
                Ok(SurfaceInventoryFacetsResponse {
                    service_names,
                    transport_protocols,
                    http_status_codes,
                })
            }
            DatabasePool::MySQL(pool) => {
                let service_names = fetch_service_name_facets_mysql(pool, &base_filter).await?;
                let transport_protocols =
                    fetch_service_transport_facets_mysql(pool, &base_filter).await?;
                let http_status_codes =
                    fetch_web_http_status_facets_mysql(pool, &base_filter).await?;
                Ok(SurfaceInventoryFacetsResponse {
                    service_names,
                    transport_protocols,
                    http_status_codes,
                })
            }
            DatabasePool::PostgreSQL(pool) => {
                let service_names = fetch_service_name_facets_postgres(pool, &base_filter).await?;
                let transport_protocols =
                    fetch_service_transport_facets_postgres(pool, &base_filter).await?;
                let http_status_codes =
                    fetch_web_http_status_facets_postgres(pool, &base_filter).await?;
                Ok(SurfaceInventoryFacetsResponse {
                    service_names,
                    transport_protocols,
                    http_status_codes,
                })
            }
        }
    }
}

async fn fetch_service_name_facets_sqlite(
    pool: &sqlx::SqlitePool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_service_transport_facets_sqlite(
    pool: &sqlx::SqlitePool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.transport_protocol, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.transport_protocol, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.transport_protocol, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_web_http_status_facets_sqlite(
    pool: &sqlx::SqlitePool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT CAST(surface_web_assets.http_status_code AS TEXT) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_web_assets ON surface_web_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND surface_web_assets.http_status_code IS NOT NULL GROUP BY surface_web_assets.http_status_code ORDER BY surface_web_assets.http_status_code ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_service_name_facets_mysql(
    pool: &MySqlPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<MySql>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_service_transport_facets_mysql(
    pool: &MySqlPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<MySql>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.transport_protocol, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.transport_protocol, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.transport_protocol, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_web_http_status_facets_mysql(
    pool: &MySqlPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<MySql>::new(
        "SELECT CAST(surface_web_assets.http_status_code AS CHAR) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_web_assets ON surface_web_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND surface_web_assets.http_status_code IS NOT NULL GROUP BY surface_web_assets.http_status_code ORDER BY surface_web_assets.http_status_code ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_service_name_facets_postgres(
    pool: &PgPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.application_service_name, surface_service_assets.protocol_name, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_service_transport_facets_postgres(
    pool: &PgPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT LOWER(COALESCE(surface_service_assets.transport_protocol, '')) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_service_assets ON surface_service_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND TRIM(COALESCE(surface_service_assets.transport_protocol, '')) != '' GROUP BY LOWER(COALESCE(surface_service_assets.transport_protocol, '')) ORDER BY count DESC, value ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}

async fn fetch_web_http_status_facets_postgres(
    pool: &PgPool,
    filter: &SurfaceAssetFilter,
) -> Result<Vec<SurfaceFacetBucket>> {
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT CAST(surface_web_assets.http_status_code AS TEXT) AS value, COUNT(*) AS count FROM surface_assets JOIN surface_web_assets ON surface_web_assets.asset_id = surface_assets.id WHERE 1=1",
    );
    push_surface_asset_filters(&mut query_builder, filter);
    query_builder.push(
        " AND surface_web_assets.http_status_code IS NOT NULL GROUP BY surface_web_assets.http_status_code ORDER BY surface_web_assets.http_status_code ASC",
    );

    let rows: Vec<FacetBucketRow> = query_builder.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| SurfaceFacetBucket {
            value: row.value,
            count: row.count,
        })
        .collect())
}
