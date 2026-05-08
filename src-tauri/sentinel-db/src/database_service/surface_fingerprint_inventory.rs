use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface::SurfaceAssetRow;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceFingerprintAssetFilter {
    pub program_id: Option<String>,
    pub category: String,
    pub product: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFingerprintAssetInventoryItem {
    pub asset: SurfaceAssetRow,
    pub typed_details: Option<Value>,
    pub matched_products: Vec<String>,
    pub matched_vendors: Vec<String>,
    pub matched_rule_names: Vec<String>,
    pub fingerprint_count: i64,
    pub last_observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceFingerprintAssetInventoryResponse {
    pub items: Vec<SurfaceFingerprintAssetInventoryItem>,
    pub total: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SurfaceFingerprintAssetMatchRow {
    asset_id: String,
    classified_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SurfaceFingerprintAssetMetadataRow {
    asset_id: String,
    primary_product: Option<String>,
    primary_vendor: Option<String>,
    rule_name: Option<String>,
}

fn push_match_filters<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    program_id: Option<&str>,
    category: &str,
    product: Option<&str>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder
        .push(" AND sac.primary_category = ")
        .push_bind(category.to_string());

    if let Some(program_id) = program_id {
        query_builder
            .push(" AND sac.program_id = ")
            .push_bind(program_id.to_string());
    }

    if let Some(product) = product.filter(|value| !value.trim().is_empty()) {
        query_builder
            .push(" AND sac.primary_product = ")
            .push_bind(product.to_string());
    }
}

fn push_asset_id_filter<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    asset_ids: &[String],
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder.push(" AND sac.asset_id IN (");
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");
}

fn push_surface_asset_id_clause<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    asset_ids: &[String],
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder.push("SELECT * FROM surface_assets WHERE id IN (");
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");
}

fn push_match_pagination<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    limit: Option<i64>,
    offset: Option<i64>,
    offset_without_limit_prefix: Option<&str>,
) where
    DB: Database,
    i64: for<'q> Encode<'q, DB> + Type<DB>,
{
    match (limit, offset) {
        (Some(limit), Some(offset)) => {
            query_builder
                .push(" LIMIT ")
                .push_bind(limit.max(1))
                .push(" OFFSET ")
                .push_bind(offset.max(0));
        }
        (Some(limit), None) => {
            query_builder.push(" LIMIT ").push_bind(limit.max(1));
        }
        (None, Some(offset)) => {
            let prefix = offset_without_limit_prefix.unwrap_or(" OFFSET ");
            query_builder.push(prefix).push_bind(offset.max(0));
        }
        (None, None) => {}
    }
}

fn normalize_unique_strings(values: impl IntoIterator<Item = Option<String>>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        if let Some(value) = value
            .as_deref()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
        {
            set.insert(value);
        }
    }
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_pagination_uses_single_limit_when_limit_and_offset_are_set() {
        let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM matches");

        push_match_pagination(
            &mut query_builder,
            Some(10),
            Some(20),
            Some(" LIMIT -1 OFFSET "),
        );

        let sql = query_builder.sql();
        assert!(sql.contains(" LIMIT ? OFFSET ?"));
        assert!(!sql.contains(" LIMIT ? LIMIT "));
    }

    #[test]
    fn match_pagination_uses_sqlite_offset_prefix_without_limit() {
        let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM matches");

        push_match_pagination(
            &mut query_builder,
            None,
            Some(20),
            Some(" LIMIT -1 OFFSET "),
        );

        assert!(query_builder.sql().contains(" LIMIT -1 OFFSET ?"));
    }
}

impl DatabaseService {
    pub async fn list_surface_fingerprint_assets(
        &self,
        filter: &SurfaceFingerprintAssetFilter,
    ) -> Result<SurfaceFingerprintAssetInventoryResponse> {
        let category = filter.category.trim();
        if category.is_empty() {
            return Ok(SurfaceFingerprintAssetInventoryResponse::default());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let count_sql = r#"
            SELECT COUNT(*) AS count
            FROM (
                SELECT sac.asset_id
                FROM surface_asset_classifications sac
                INNER JOIN surface_assets sa ON sa.id = sac.asset_id
                WHERE COALESCE(TRIM(sac.primary_category), '') <> ''
        "#;
        let match_sql = r#"
            SELECT
                sac.asset_id,
                sac.classified_at
            FROM surface_asset_classifications sac
            INNER JOIN surface_assets sa ON sa.id = sac.asset_id
            WHERE COALESCE(TRIM(sac.primary_category), '') <> ''
        "#;
        let metadata_sql = r#"
            SELECT
                sac.asset_id,
                sac.primary_product,
                sac.primary_vendor,
                sac.rule_name
            FROM surface_asset_classifications sac
            WHERE COALESCE(TRIM(sac.primary_category), '') <> ''
        "#;

        let total = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(count_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" GROUP BY sac.asset_id) matched_assets");
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(count_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" GROUP BY sac.asset_id) matched_assets");
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(count_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" GROUP BY sac.asset_id) matched_assets");
                query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?
            }
        };

        if total <= 0 {
            return Ok(SurfaceFingerprintAssetInventoryResponse::default());
        }

        let match_rows: Vec<SurfaceFingerprintAssetMatchRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(match_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" ORDER BY sac.classified_at DESC, sac.asset_id DESC");
                push_match_pagination(
                    &mut query_builder,
                    filter.limit,
                    filter.offset,
                    Some(" LIMIT -1 OFFSET "),
                );
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMatchRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(match_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" ORDER BY sac.classified_at DESC, sac.asset_id DESC");
                push_match_pagination(
                    &mut query_builder,
                    filter.limit,
                    filter.offset,
                    Some(" LIMIT 18446744073709551615 OFFSET "),
                );
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMatchRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(match_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                query_builder.push(" ORDER BY sac.classified_at DESC, sac.asset_id DESC");
                push_match_pagination(&mut query_builder, filter.limit, filter.offset, None);
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMatchRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let asset_ids = match_rows
            .iter()
            .map(|row| row.asset_id.clone())
            .collect::<Vec<_>>();
        if asset_ids.is_empty() {
            return Ok(SurfaceFingerprintAssetInventoryResponse {
                items: vec![],
                total,
            });
        }

        let assets = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new("");
                push_surface_asset_id_clause(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new("");
                push_surface_asset_id_clause(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new("");
                push_surface_asset_id_clause(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let metadata_rows: Vec<SurfaceFingerprintAssetMetadataRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(metadata_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                push_asset_id_filter(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMetadataRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(metadata_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                push_asset_id_filter(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMetadataRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(metadata_sql);
                push_match_filters(
                    &mut query_builder,
                    filter.program_id.as_deref(),
                    category,
                    filter.product.as_deref(),
                );
                push_asset_id_filter(&mut query_builder, &asset_ids);
                query_builder
                    .build_query_as::<SurfaceFingerprintAssetMetadataRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let mut asset_by_id = assets
            .into_iter()
            .map(|asset| (asset.id.clone(), asset))
            .collect::<HashMap<_, _>>();
        let asset_rows = asset_by_id.values().cloned().collect::<Vec<_>>();
        let mut typed_details_by_id = self.list_surface_typed_details_map(&asset_rows).await?;

        let mut metadata_by_asset: HashMap<String, Vec<SurfaceFingerprintAssetMetadataRow>> =
            HashMap::new();
        for row in metadata_rows {
            metadata_by_asset
                .entry(row.asset_id.clone())
                .or_default()
                .push(row);
        }

        let mut items = Vec::with_capacity(match_rows.len());
        for row in match_rows {
            let Some(asset) = asset_by_id.remove(&row.asset_id) else {
                continue;
            };
            let metadata = metadata_by_asset.remove(&row.asset_id).unwrap_or_default();
            let matched_products =
                normalize_unique_strings(metadata.iter().map(|item| item.primary_product.clone()));
            let matched_vendors =
                normalize_unique_strings(metadata.iter().map(|item| item.primary_vendor.clone()));
            let matched_rule_names =
                normalize_unique_strings(metadata.iter().map(|item| item.rule_name.clone()));

            items.push(SurfaceFingerprintAssetInventoryItem {
                typed_details: typed_details_by_id.remove(&row.asset_id),
                asset,
                matched_products,
                matched_vendors,
                fingerprint_count: matched_rule_names.len() as i64,
                matched_rule_names,
                last_observed_at: row.classified_at,
            });
        }

        Ok(SurfaceFingerprintAssetInventoryResponse { items, total })
    }
}
