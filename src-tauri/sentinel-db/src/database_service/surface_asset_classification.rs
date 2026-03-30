use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Encode, QueryBuilder, Type};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceAssetClassificationRow {
    pub id: String,
    pub program_id: String,
    pub asset_id: String,
    pub primary_category: String,
    pub primary_product: String,
    pub primary_vendor: Option<String>,
    pub primary_family: Option<String>,
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
    pub confidence_score: Option<f64>,
    pub source: Option<String>,
    pub classified_at: String,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SurfaceClassificationFingerprintCandidate {
    asset_id: String,
    rule_id: Option<String>,
    rule_name: Option<String>,
    normalized_product: Option<String>,
    normalized_vendor: Option<String>,
    normalized_category: Option<String>,
    normalized_family: Option<String>,
    confidence_score: Option<f64>,
    is_primary: Option<bool>,
    source: Option<String>,
    observed_at: String,
}

#[derive(Debug, Clone)]
struct SurfaceClassificationResolvedCandidate {
    asset_id: String,
    rule_id: Option<String>,
    rule_name: Option<String>,
    primary_product: String,
    primary_vendor: Option<String>,
    primary_category: String,
    primary_family: Option<String>,
    confidence_score: Option<f64>,
    source: Option<String>,
    classified_at: String,
    is_primary: bool,
}

fn push_asset_ids<'args, DB>(query_builder: &mut QueryBuilder<'args, DB>, asset_ids: &[String])
where
    DB: sqlx::Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    query_builder.push(" AND asset_id IN (");
    {
        let mut separated = query_builder.separated(", ");
        for asset_id in asset_ids {
            separated.push_bind(asset_id.clone());
        }
    }
    query_builder.push(")");
}

fn resolve_candidate(
    row: SurfaceClassificationFingerprintCandidate,
) -> Option<SurfaceClassificationResolvedCandidate> {
    let primary_category = row
        .normalized_category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;
    let primary_product = row
        .normalized_product
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)?;

    Some(SurfaceClassificationResolvedCandidate {
        asset_id: row.asset_id,
        rule_id: row.rule_id,
        rule_name: row.rule_name,
        primary_product,
        primary_vendor: row
            .normalized_vendor
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        primary_category,
        primary_family: row
            .normalized_family
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        confidence_score: row.confidence_score,
        source: row.source,
        classified_at: row.observed_at,
        is_primary: row.is_primary.unwrap_or(false),
    })
}

fn compare_candidates(
    left: &SurfaceClassificationResolvedCandidate,
    right: &SurfaceClassificationResolvedCandidate,
) -> std::cmp::Ordering {
    right
        .is_primary
        .cmp(&left.is_primary)
        .then_with(|| {
            right
                .confidence_score
                .unwrap_or(0.0)
                .partial_cmp(&left.confidence_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right.classified_at.cmp(&left.classified_at))
        .then_with(|| left.primary_category.cmp(&right.primary_category))
        .then_with(|| left.primary_product.cmp(&right.primary_product))
}

impl DatabaseService {
    pub async fn get_surface_asset_classification(
        &self,
        asset_id: &str,
    ) -> Result<Option<SurfaceAssetClassificationRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_asset_classifications WHERE asset_id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_asset_classifications WHERE asset_id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_asset_classifications WHERE asset_id = $1 LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn refresh_surface_asset_classifications(
        &self,
        program_id: &str,
        asset_ids: &[String],
    ) -> Result<()> {
        if asset_ids.is_empty() {
            return Ok(());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let candidate_sql = r#"
            SELECT
                asset_id,
                rule_id,
                rule_name,
                normalized_product,
                normalized_vendor,
                normalized_category,
                normalized_family,
                confidence_score,
                is_primary,
                source,
                observed_at
            FROM surface_fingerprints
            WHERE program_id = 
        "#;

        let rows: Vec<SurfaceClassificationFingerprintCandidate> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(candidate_sql);
                query_builder.push_bind(program_id.to_string());
                push_asset_ids(&mut query_builder, asset_ids);
                query_builder.push(
                    " AND COALESCE(TRIM(normalized_category), '') <> '' AND COALESCE(TRIM(normalized_product), '') <> ''",
                );
                query_builder
                    .build_query_as::<SurfaceClassificationFingerprintCandidate>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(candidate_sql);
                query_builder.push_bind(program_id.to_string());
                push_asset_ids(&mut query_builder, asset_ids);
                query_builder.push(
                    " AND COALESCE(TRIM(normalized_category), '') <> '' AND COALESCE(TRIM(normalized_product), '') <> ''",
                );
                query_builder
                    .build_query_as::<SurfaceClassificationFingerprintCandidate>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(candidate_sql);
                query_builder.push_bind(program_id.to_string());
                push_asset_ids(&mut query_builder, asset_ids);
                query_builder.push(
                    " AND COALESCE(TRIM(normalized_category), '') <> '' AND COALESCE(TRIM(normalized_product), '') <> ''",
                );
                query_builder
                    .build_query_as::<SurfaceClassificationFingerprintCandidate>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let mut best_by_asset: HashMap<String, SurfaceClassificationResolvedCandidate> =
            HashMap::new();
        for row in rows {
            let Some(candidate) = resolve_candidate(row) else {
                continue;
            };

            match best_by_asset.get(&candidate.asset_id) {
                Some(existing) if compare_candidates(existing, &candidate).is_lt() => {}
                _ => {
                    best_by_asset.insert(candidate.asset_id.clone(), candidate);
                }
            }
        }

        match runtime {
            DatabasePool::SQLite(pool) => {
                for asset_id in asset_ids {
                    sqlx::query("DELETE FROM surface_asset_classifications WHERE asset_id = ?")
                        .bind(asset_id)
                        .execute(pool)
                        .await?;

                    if let Some(candidate) = best_by_asset.get(asset_id) {
                        sqlx::query(
                            "INSERT INTO surface_asset_classifications (id, program_id, asset_id, primary_category, primary_product, primary_vendor, primary_family, rule_id, rule_name, confidence_score, source, classified_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        )
                        .bind(uuid::Uuid::new_v4().to_string())
                        .bind(program_id)
                        .bind(asset_id)
                        .bind(&candidate.primary_category)
                        .bind(&candidate.primary_product)
                        .bind(&candidate.primary_vendor)
                        .bind(&candidate.primary_family)
                        .bind(&candidate.rule_id)
                        .bind(&candidate.rule_name)
                        .bind(candidate.confidence_score)
                        .bind(&candidate.source)
                        .bind(&candidate.classified_at)
                        .bind(
                            serde_json::json!({
                                "source": candidate.source,
                                "rule_id": candidate.rule_id,
                                "rule_name": candidate.rule_name,
                                "primary_category": candidate.primary_category,
                                "primary_product": candidate.primary_product,
                                "primary_vendor": candidate.primary_vendor,
                                "primary_family": candidate.primary_family,
                                "confidence_score": candidate.confidence_score,
                                "classified_at": candidate.classified_at,
                            })
                            .to_string(),
                        )
                        .execute(pool)
                        .await?;
                    }
                }
            }
            DatabasePool::MySQL(pool) => {
                for asset_id in asset_ids {
                    sqlx::query("DELETE FROM surface_asset_classifications WHERE asset_id = ?")
                        .bind(asset_id)
                        .execute(pool)
                        .await?;

                    if let Some(candidate) = best_by_asset.get(asset_id) {
                        sqlx::query(
                            "INSERT INTO surface_asset_classifications (id, program_id, asset_id, primary_category, primary_product, primary_vendor, primary_family, rule_id, rule_name, confidence_score, source, classified_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        )
                        .bind(uuid::Uuid::new_v4().to_string())
                        .bind(program_id)
                        .bind(asset_id)
                        .bind(&candidate.primary_category)
                        .bind(&candidate.primary_product)
                        .bind(&candidate.primary_vendor)
                        .bind(&candidate.primary_family)
                        .bind(&candidate.rule_id)
                        .bind(&candidate.rule_name)
                        .bind(candidate.confidence_score)
                        .bind(&candidate.source)
                        .bind(&candidate.classified_at)
                        .bind(
                            serde_json::json!({
                                "source": candidate.source,
                                "rule_id": candidate.rule_id,
                                "rule_name": candidate.rule_name,
                                "primary_category": candidate.primary_category,
                                "primary_product": candidate.primary_product,
                                "primary_vendor": candidate.primary_vendor,
                                "primary_family": candidate.primary_family,
                                "confidence_score": candidate.confidence_score,
                                "classified_at": candidate.classified_at,
                            })
                            .to_string(),
                        )
                        .execute(pool)
                        .await?;
                    }
                }
            }
            DatabasePool::PostgreSQL(pool) => {
                for asset_id in asset_ids {
                    sqlx::query("DELETE FROM surface_asset_classifications WHERE asset_id = $1")
                        .bind(asset_id)
                        .execute(pool)
                        .await?;

                    if let Some(candidate) = best_by_asset.get(asset_id) {
                        sqlx::query(
                            "INSERT INTO surface_asset_classifications (id, program_id, asset_id, primary_category, primary_product, primary_vendor, primary_family, rule_id, rule_name, confidence_score, source, classified_at, metadata_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                        )
                        .bind(uuid::Uuid::new_v4().to_string())
                        .bind(program_id)
                        .bind(asset_id)
                        .bind(&candidate.primary_category)
                        .bind(&candidate.primary_product)
                        .bind(&candidate.primary_vendor)
                        .bind(&candidate.primary_family)
                        .bind(&candidate.rule_id)
                        .bind(&candidate.rule_name)
                        .bind(candidate.confidence_score)
                        .bind(&candidate.source)
                        .bind(&candidate.classified_at)
                        .bind(
                            serde_json::json!({
                                "source": candidate.source,
                                "rule_id": candidate.rule_id,
                                "rule_name": candidate.rule_name,
                                "primary_category": candidate.primary_category,
                                "primary_product": candidate.primary_product,
                                "primary_vendor": candidate.primary_vendor,
                                "primary_family": candidate.primary_family,
                                "confidence_score": candidate.confidence_score,
                                "classified_at": candidate.classified_at,
                            })
                            .to_string(),
                        )
                        .execute(pool)
                        .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
