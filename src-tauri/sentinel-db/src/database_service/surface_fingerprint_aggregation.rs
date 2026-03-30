use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFingerprintProductBucket {
    pub product: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFingerprintCategoryBucket {
    pub category: String,
    pub count: i64,
    pub top_products: Vec<SurfaceFingerprintProductBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFingerprintCategoryAggregation {
    pub total_classified: i64,
    pub categories: Vec<SurfaceFingerprintCategoryBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct SurfaceFingerprintAggregationRow {
    primary_category: Option<String>,
    primary_product: Option<String>,
    count: i64,
}

impl DatabaseService {
    fn push_surface_fingerprint_program_filter<'args, DB>(
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

    pub async fn get_surface_fingerprint_category_aggregation(
        &self,
        program_id: Option<&str>,
        category_limit: Option<i64>,
        product_limit: Option<i64>,
    ) -> Result<SurfaceFingerprintCategoryAggregation> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = r#"
            SELECT
                primary_category,
                primary_product,
                COUNT(*) AS count
            FROM surface_asset_classifications
            WHERE COALESCE(TRIM(primary_category), '') <> ''
        "#;

        let rows: Vec<SurfaceFingerprintAggregationRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(sql);
                Self::push_surface_fingerprint_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY primary_category, primary_product");
                query_builder
                    .build_query_as::<SurfaceFingerprintAggregationRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(sql);
                Self::push_surface_fingerprint_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY primary_category, primary_product");
                query_builder
                    .build_query_as::<SurfaceFingerprintAggregationRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(sql);
                Self::push_surface_fingerprint_program_filter(&mut query_builder, program_id);
                query_builder.push(" GROUP BY primary_category, primary_product");
                query_builder
                    .build_query_as::<SurfaceFingerprintAggregationRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let mut category_counts: HashMap<String, i64> = HashMap::new();
        let mut category_products: HashMap<String, HashMap<String, i64>> = HashMap::new();

        for row in rows {
            let Some(category) = row
                .primary_category
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
            else {
                continue;
            };

            *category_counts.entry(category.clone()).or_default() += row.count;

            if let Some(product) = row
                .primary_product
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
            {
                *category_products
                    .entry(category)
                    .or_default()
                    .entry(product)
                    .or_default() += row.count;
            }
        }

        let total_classified = category_counts.values().sum();
        let per_category_product_limit = product_limit.unwrap_or(10).max(1) as usize;

        let mut categories = category_counts
            .into_iter()
            .map(|(category, count)| {
                let mut top_products = category_products
                    .remove(&category)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(product, count)| SurfaceFingerprintProductBucket { product, count })
                    .collect::<Vec<_>>();
                top_products.sort_by(|left, right| {
                    right
                        .count
                        .cmp(&left.count)
                        .then_with(|| left.product.cmp(&right.product))
                });
                top_products.truncate(per_category_product_limit);

                SurfaceFingerprintCategoryBucket {
                    category,
                    count,
                    top_products,
                }
            })
            .collect::<Vec<_>>();

        categories.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.category.cmp(&right.category))
        });
        categories.truncate(category_limit.unwrap_or(12).max(1) as usize);

        Ok(SurfaceFingerprintCategoryAggregation {
            total_classified,
            categories,
        })
    }
}
