use crate::core::models::asset::*;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, FromRow)]
struct AssetDbRow {
    id: String,
    project_id: Option<String>,
    asset_type: String,
    name: String,
    value: String,
    description: Option<String>,
    confidence: f64,
    status: String,
    source: Option<String>,
    source_scan_id: Option<String>,
    metadata: String,
    tags: String,
    risk_level: String,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    created_by: String,
}

#[derive(Debug, Clone, FromRow)]
struct AssetRelationshipDbRow {
    id: String,
    source_asset_id: String,
    target_asset_id: String,
    relationship_type: String,
    description: String,
    confidence: f64,
    metadata: String,
    created_at: DateTime<Utc>,
    created_by: String,
}

fn asset_from_db_row(row: AssetDbRow) -> Asset {
    let metadata: HashMap<String, serde_json::Value> =
        serde_json::from_str(&row.metadata).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
    let asset_type = AssetType::from_str(&row.asset_type).unwrap_or(AssetType::Domain);
    let status = match row.status.as_str() {
        "active" => AssetStatus::Active,
        "inactive" => AssetStatus::Inactive,
        "verified" => AssetStatus::Verified,
        "unverified" => AssetStatus::Unverified,
        _ => AssetStatus::Active,
    };
    let risk_level = RiskLevel::from_str(&row.risk_level).unwrap_or(RiskLevel::Unknown);

    Asset {
        id: row.id,
        project_id: row.project_id,
        asset_type,
        name: row.name,
        value: row.value,
        description: row.description,
        confidence: row.confidence,
        status,
        source: row.source,
        source_scan_id: row.source_scan_id,
        metadata,
        tags,
        risk_level,
        last_seen: row.last_seen,
        first_seen: row.first_seen,
        created_at: row.created_at,
        updated_at: row.updated_at,
        created_by: row.created_by,
    }
}

fn relationship_from_db_row(row: AssetRelationshipDbRow) -> AssetRelationship {
    let metadata: HashMap<String, serde_json::Value> =
        serde_json::from_str(&row.metadata).unwrap_or_default();
    let relationship_type = match row.relationship_type.as_str() {
        "belongs_to" => RelationshipType::BelongsTo,
        "contains" => RelationshipType::Contains,
        "connects_to" => RelationshipType::ConnectsTo,
        "depends_on" => RelationshipType::DependsOn,
        "resolves_to" => RelationshipType::ResolvesTo,
        "hosts" => RelationshipType::Hosts,
        "uses" => RelationshipType::Uses,
        "exposes" => RelationshipType::Exposes,
        _ => RelationshipType::BelongsTo,
    };

    AssetRelationship {
        id: row.id,
        source_asset_id: row.source_asset_id,
        target_asset_id: row.target_asset_id,
        relationship_type,
        description: Some(row.description),
        confidence: row.confidence,
        metadata,
        created_at: row.created_at,
        created_by: row.created_by,
    }
}

fn asset_matches_filter(asset: &Asset, filter: &AssetFilter) -> bool {
    if let Some(asset_types) = &filter.asset_types {
        if !asset_types.is_empty() && !asset_types.iter().any(|t| t == &asset.asset_type) {
            return false;
        }
    }
    if let Some(statuses) = &filter.statuses {
        if !statuses.is_empty() && !statuses.iter().any(|s| s == &asset.status) {
            return false;
        }
    }
    if let Some(risk_levels) = &filter.risk_levels {
        if !risk_levels.is_empty() && !risk_levels.iter().any(|r| r == &asset.risk_level) {
            return false;
        }
    }
    if let Some(search) = &filter.search {
        let q = search.to_lowercase();
        let name_ok = asset.name.to_lowercase().contains(&q);
        let value_ok = asset.value.to_lowercase().contains(&q);
        let desc_ok = asset
            .description
            .as_ref()
            .map(|d| d.to_lowercase().contains(&q))
            .unwrap_or(false);
        if !(name_ok || value_ok || desc_ok) {
            return false;
        }
    }
    if let Some(created_after) = filter.created_after {
        if asset.created_at < created_after {
            return false;
        }
    }
    if let Some(created_before) = filter.created_before {
        if asset.created_at > created_before {
            return false;
        }
    }
    if let Some(last_seen_after) = filter.last_seen_after {
        if asset.last_seen < last_seen_after {
            return false;
        }
    }
    if let Some(last_seen_before) = filter.last_seen_before {
        if asset.last_seen > last_seen_before {
            return false;
        }
    }
    true
}

impl DatabaseService {
    /// 创建资产
    pub async fn create_asset_internal(
        &self,
        request: CreateAssetRequest,
        created_by: String,
    ) -> Result<Asset> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let mut asset = Asset::new(
            request.asset_type.clone(),
            request.name.clone(),
            request.value.clone(),
            created_by.clone(),
        )
        .with_source(request.source.unwrap_or_default(), request.source_scan_id)
        .with_metadata(request.metadata.unwrap_or_default())
        .with_tags(request.tags.unwrap_or_default())
        .with_risk_level(request.risk_level.unwrap_or(RiskLevel::Unknown))
        .with_confidence(request.confidence.unwrap_or(1.0));

        // 设置project_id
        asset.project_id = request.project_id;

        let metadata_json = serde_json::to_string(&asset.metadata).unwrap_or_default();
        let tags_json = serde_json::to_string(&asset.tags).unwrap_or_default();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO assets (
                        id, project_id, asset_type, name, value, description, confidence, status,
                        source, source_scan_id, metadata, tags, risk_level,
                        last_seen, first_seen, created_at, updated_at, created_by
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                    "#,
                )
                .bind(&asset.id)
                .bind(&asset.project_id)
                .bind(asset.asset_type.as_str())
                .bind(&asset.name)
                .bind(&asset.value)
                .bind(&asset.description)
                .bind(asset.confidence)
                .bind(asset.status.as_str())
                .bind(&asset.source)
                .bind(&asset.source_scan_id)
                .bind(&metadata_json)
                .bind(&tags_json)
                .bind(asset.risk_level.as_str())
                .bind(asset.last_seen)
                .bind(asset.first_seen)
                .bind(asset.created_at)
                .bind(asset.updated_at)
                .bind(&asset.created_by)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO assets (
                        id, project_id, asset_type, name, value, description, confidence, status,
                        source, source_scan_id, metadata, tags, risk_level,
                        last_seen, first_seen, created_at, updated_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&asset.id)
                .bind(&asset.project_id)
                .bind(asset.asset_type.as_str())
                .bind(&asset.name)
                .bind(&asset.value)
                .bind(&asset.description)
                .bind(asset.confidence)
                .bind(asset.status.as_str())
                .bind(&asset.source)
                .bind(&asset.source_scan_id)
                .bind(&metadata_json)
                .bind(&tags_json)
                .bind(asset.risk_level.as_str())
                .bind(asset.last_seen)
                .bind(asset.first_seen)
                .bind(asset.created_at)
                .bind(asset.updated_at)
                .bind(&asset.created_by)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO assets (
                        id, project_id, asset_type, name, value, description, confidence, status,
                        source, source_scan_id, metadata, tags, risk_level,
                        last_seen, first_seen, created_at, updated_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&asset.id)
                .bind(&asset.project_id)
                .bind(asset.asset_type.as_str())
                .bind(&asset.name)
                .bind(&asset.value)
                .bind(&asset.description)
                .bind(asset.confidence)
                .bind(asset.status.as_str())
                .bind(&asset.source)
                .bind(&asset.source_scan_id)
                .bind(&metadata_json)
                .bind(&tags_json)
                .bind(asset.risk_level.as_str())
                .bind(asset.last_seen)
                .bind(asset.first_seen)
                .bind(asset.created_at)
                .bind(asset.updated_at)
                .bind(&asset.created_by)
                .execute(pool)
                .await?;
            }
        }

        Ok(asset)
    }

    /// 根据ID获取资产
    pub async fn get_asset_by_id_internal(&self, id: &str) -> Result<Option<Asset>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row: Option<AssetDbRow> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE id = $1
                    "#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE id = ?
                    "#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE id = ?
                    "#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row.map(asset_from_db_row))
    }

    /// 根据类型和值查找资产
    pub async fn find_asset_by_type_and_value_internal(
        &self,
        asset_type: &AssetType,
        value: &str,
    ) -> Result<Option<Asset>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row: Option<AssetDbRow> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE asset_type = $1 AND value = $2
                    "#,
                )
                .bind(asset_type.as_str())
                .bind(value)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE asset_type = ? AND value = ?
                    "#,
                )
                .bind(asset_type.as_str())
                .bind(value)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets WHERE asset_type = ? AND value = ?
                    "#,
                )
                .bind(asset_type.as_str())
                .bind(value)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row.map(asset_from_db_row))
    }

    /// 更新资产
    pub async fn update_asset_internal(
        &self,
        id: &str,
        request: UpdateAssetRequest,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let project_id = request.project_id.clone();
        let name = request.name.clone();
        let value = request.value.clone();
        let description = request.description.clone();
        let confidence = request.confidence;
        let status = request.status.as_ref().map(|s| s.as_str().to_string());
        let metadata_json = request
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());
        let tags_json = request
            .tags
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());
        let risk_level = request.risk_level.as_ref().map(|r| r.as_str().to_string());

        let has_updates = project_id.is_some()
            || name.is_some()
            || value.is_some()
            || description.is_some()
            || confidence.is_some()
            || status.is_some()
            || metadata_json.is_some()
            || tags_json.is_some()
            || risk_level.is_some();
        if !has_updates {
            return Ok(false);
        }

        let now = Utc::now();
        let rows_affected = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                r#"UPDATE assets SET
                           project_id = COALESCE($1, project_id),
                           name = COALESCE($2, name),
                           value = COALESCE($3, value),
                           description = COALESCE($4, description),
                           confidence = COALESCE($5, confidence),
                           status = COALESCE($6, status),
                           metadata = COALESCE($7, metadata),
                           tags = COALESCE($8, tags),
                           risk_level = COALESCE($9, risk_level),
                           updated_at = $10
                       WHERE id = $11"#,
            )
            .bind(project_id)
            .bind(name)
            .bind(value)
            .bind(description)
            .bind(confidence)
            .bind(status)
            .bind(metadata_json)
            .bind(tags_json)
            .bind(risk_level)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::SQLite(pool) => sqlx::query(
                r#"UPDATE assets SET
                           project_id = COALESCE(?, project_id),
                           name = COALESCE(?, name),
                           value = COALESCE(?, value),
                           description = COALESCE(?, description),
                           confidence = COALESCE(?, confidence),
                           status = COALESCE(?, status),
                           metadata = COALESCE(?, metadata),
                           tags = COALESCE(?, tags),
                           risk_level = COALESCE(?, risk_level),
                           updated_at = ?
                       WHERE id = ?"#,
            )
            .bind(project_id)
            .bind(name)
            .bind(value)
            .bind(description)
            .bind(confidence)
            .bind(status)
            .bind(metadata_json)
            .bind(tags_json)
            .bind(risk_level)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query(
                r#"UPDATE assets SET
                           project_id = COALESCE(?, project_id),
                           name = COALESCE(?, name),
                           value = COALESCE(?, value),
                           description = COALESCE(?, description),
                           confidence = COALESCE(?, confidence),
                           status = COALESCE(?, status),
                           metadata = COALESCE(?, metadata),
                           tags = COALESCE(?, tags),
                           risk_level = COALESCE(?, risk_level),
                           updated_at = ?
                       WHERE id = ?"#,
            )
            .bind(project_id)
            .bind(name)
            .bind(value)
            .bind(description)
            .bind(confidence)
            .bind(status)
            .bind(metadata_json)
            .bind(tags_json)
            .bind(risk_level)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    /// 删除资产
    pub async fn delete_asset_internal(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows_affected = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query("DELETE FROM assets WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::SQLite(pool) => sqlx::query("DELETE FROM assets WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query("DELETE FROM assets WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
        };
        Ok(rows_affected > 0)
    }

    /// 查询资产列表
    pub async fn list_assets_internal(
        &self,
        filter: Option<AssetFilter>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Asset>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = sqlx::QueryBuilder::new(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets
                    "#,
                );

                let mut has_conditions = false;
                if let Some(filter) = filter {
                    if let Some(asset_types) = filter.asset_types {
                        if !asset_types.is_empty() {
                            if !has_conditions {
                                query_builder.push(" WHERE ");
                                has_conditions = true;
                            } else {
                                query_builder.push(" AND ");
                            }
                            query_builder.push("asset_type IN (");
                            let mut separated = query_builder.separated(", ");
                            for asset_type in asset_types {
                                separated.push_bind(asset_type.as_str());
                            }
                            query_builder.push(")");
                        }
                    }
                    if let Some(statuses) = filter.statuses {
                        if !statuses.is_empty() {
                            if !has_conditions {
                                query_builder.push(" WHERE ");
                                has_conditions = true;
                            } else {
                                query_builder.push(" AND ");
                            }
                            query_builder.push("status IN (");
                            let mut separated = query_builder.separated(", ");
                            for status in statuses {
                                separated.push_bind(status.as_str());
                            }
                            query_builder.push(")");
                        }
                    }
                    if let Some(risk_levels) = filter.risk_levels {
                        if !risk_levels.is_empty() {
                            if !has_conditions {
                                query_builder.push(" WHERE ");
                                has_conditions = true;
                            } else {
                                query_builder.push(" AND ");
                            }
                            query_builder.push("risk_level IN (");
                            let mut separated = query_builder.separated(", ");
                            for risk_level in risk_levels {
                                separated.push_bind(risk_level.as_str());
                            }
                            query_builder.push(")");
                        }
                    }
                    if let Some(search) = filter.search {
                        let search_pattern = format!("%{}%", search);
                        if !has_conditions {
                            query_builder.push(" WHERE ");
                            has_conditions = true;
                        } else {
                            query_builder.push(" AND ");
                        }
                        query_builder
                            .push("(name LIKE ")
                            .push_bind(search_pattern.clone())
                            .push(" OR value LIKE ")
                            .push_bind(search_pattern.clone())
                            .push(" OR description LIKE ")
                            .push_bind(search_pattern)
                            .push(")");
                    }
                    if let Some(created_after) = filter.created_after {
                        if !has_conditions {
                            query_builder.push(" WHERE ");
                            has_conditions = true;
                        } else {
                            query_builder.push(" AND ");
                        }
                        query_builder
                            .push("created_at >= ")
                            .push_bind(created_after);
                    }
                    if let Some(created_before) = filter.created_before {
                        if !has_conditions {
                            query_builder.push(" WHERE ");
                            has_conditions = true;
                        } else {
                            query_builder.push(" AND ");
                        }
                        query_builder
                            .push("created_at <= ")
                            .push_bind(created_before);
                    }
                    if let Some(last_seen_after) = filter.last_seen_after {
                        if !has_conditions {
                            query_builder.push(" WHERE ");
                            has_conditions = true;
                        } else {
                            query_builder.push(" AND ");
                        }
                        query_builder
                            .push("last_seen >= ")
                            .push_bind(last_seen_after);
                    }
                    if let Some(last_seen_before) = filter.last_seen_before {
                        if !has_conditions {
                            query_builder.push(" WHERE ");
                        } else {
                            query_builder.push(" AND ");
                        }
                        query_builder
                            .push("last_seen <= ")
                            .push_bind(last_seen_before);
                    }
                }

                query_builder.push(" ORDER BY created_at DESC");
                if let Some(limit) = limit {
                    query_builder.push(" LIMIT ").push_bind(limit as i64);
                    if let Some(offset) = offset {
                        query_builder.push(" OFFSET ").push_bind(offset as i64);
                    }
                }

                let rows: Vec<AssetDbRow> = query_builder.build_query_as().fetch_all(pool).await?;
                Ok(rows.into_iter().map(asset_from_db_row).collect())
            }
            DatabasePool::SQLite(pool) => {
                let rows: Vec<AssetDbRow> = sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets
                    ORDER BY created_at DESC
                    "#,
                )
                .fetch_all(pool)
                .await?;
                let mut assets: Vec<Asset> = rows.into_iter().map(asset_from_db_row).collect();
                if let Some(filter) = &filter {
                    assets.retain(|a| asset_matches_filter(a, filter));
                }
                if let Some(off) = offset {
                    assets = assets.into_iter().skip(off as usize).collect();
                }
                if let Some(lim) = limit {
                    assets.truncate(lim as usize);
                }
                Ok(assets)
            }
            DatabasePool::MySQL(pool) => {
                let rows: Vec<AssetDbRow> = sqlx::query_as(
                    r#"
                    SELECT id, project_id, asset_type, name, value, description, confidence, status,
                           source, source_scan_id, metadata, tags, risk_level,
                           first_seen, last_seen, created_at, updated_at, created_by
                    FROM assets
                    ORDER BY created_at DESC
                    "#,
                )
                .fetch_all(pool)
                .await?;
                let mut assets: Vec<Asset> = rows.into_iter().map(asset_from_db_row).collect();
                if let Some(filter) = &filter {
                    assets.retain(|a| asset_matches_filter(a, filter));
                }
                if let Some(off) = offset {
                    assets = assets.into_iter().skip(off as usize).collect();
                }
                if let Some(lim) = limit {
                    assets.truncate(lim as usize);
                }
                Ok(assets)
            }
        }
    }

    /// 获取资产统计信息
    pub async fn get_asset_stats_internal(&self) -> Result<AssetStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let total_assets: i64 = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM assets")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM assets")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM assets")
                    .fetch_one(pool)
                    .await?
            }
        };

        // 按类型统计
        let mut by_type = HashMap::new();
        let type_rows: Vec<(String, i64)> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT asset_type, COUNT(*) FROM assets GROUP BY asset_type")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT asset_type, COUNT(*) FROM assets GROUP BY asset_type")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT asset_type, COUNT(*) FROM assets GROUP BY asset_type")
                    .fetch_all(pool)
                    .await?
            }
        };
        for (asset_type, count) in type_rows {
            by_type.insert(asset_type, count as f64);
        }

        // 按状态统计
        let mut by_status = HashMap::new();
        let status_rows: Vec<(String, i64)> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM assets GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM assets GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM assets GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
        };
        for (status, count) in status_rows {
            by_status.insert(status, count as f64);
        }

        // 按风险等级统计
        let mut by_risk_level = HashMap::new();
        let risk_rows: Vec<(String, i64)> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT risk_level, COUNT(*) FROM assets GROUP BY risk_level")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT risk_level, COUNT(*) FROM assets GROUP BY risk_level")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT risk_level, COUNT(*) FROM assets GROUP BY risk_level")
                    .fetch_all(pool)
                    .await?
            }
        };
        for (risk_level, count) in risk_rows {
            by_risk_level.insert(risk_level, count as f64);
        }

        // 按来源统计
        let mut by_source = HashMap::new();
        let source_rows: Vec<(String, i64)> =
            match runtime {
                DatabasePool::PostgreSQL(pool) => sqlx::query_as(
                    "SELECT source, COUNT(*) FROM assets WHERE source IS NOT NULL GROUP BY source",
                )
                .fetch_all(pool)
                .await?,
                DatabasePool::SQLite(pool) => sqlx::query_as(
                    "SELECT source, COUNT(*) FROM assets WHERE source IS NOT NULL GROUP BY source",
                )
                .fetch_all(pool)
                .await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(
                    "SELECT source, COUNT(*) FROM assets WHERE source IS NOT NULL GROUP BY source",
                )
                .fetch_all(pool)
                .await?,
            };
        for (source, count) in source_rows {
            by_source.insert(source, count as f64);
        }

        // 最近24小时新增
        let (recent_additions, stale_assets): (i64, i64) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let recent_additions: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE CAST(created_at AS TIMESTAMP WITH TIME ZONE) >= NOW() - INTERVAL '1 day'"
                )
                .fetch_one(pool)
                .await?;
                let stale_assets: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE CAST(updated_at AS TIMESTAMP WITH TIME ZONE) <= NOW() - INTERVAL '30 days'"
                )
                .fetch_one(pool)
                .await?;
                (recent_additions, stale_assets)
            }
            DatabasePool::SQLite(pool) => {
                let recent_additions: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE datetime(created_at) >= datetime('now', '-1 day')"
                )
                .fetch_one(pool)
                .await?;
                let stale_assets: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE datetime(updated_at) <= datetime('now', '-30 days')"
                )
                .fetch_one(pool)
                .await?;
                (recent_additions, stale_assets)
            }
            DatabasePool::MySQL(pool) => {
                let recent_additions: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE created_at >= DATE_SUB(NOW(), INTERVAL 1 DAY)"
                )
                .fetch_one(pool)
                .await?;
                let stale_assets: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM assets WHERE updated_at <= DATE_SUB(NOW(), INTERVAL 30 DAY)"
                )
                .fetch_one(pool)
                .await?;
                (recent_additions, stale_assets)
            }
        };

        Ok(AssetStats {
            total_assets: total_assets as f64,
            by_type,
            by_status,
            by_risk_level,
            by_source,
            recent_additions: recent_additions as f64,
            stale_assets: stale_assets as f64,
        })
    }

    /// 创建资产关系
    pub async fn create_relationship_internal(
        &self,
        source_asset_id: String,
        target_asset_id: String,
        relationship_type: RelationshipType,
        created_by: String,
    ) -> Result<AssetRelationship> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let relationship = AssetRelationship::new(
            source_asset_id,
            target_asset_id,
            relationship_type,
            created_by,
        );

        let metadata_json = serde_json::to_string(&relationship.metadata).unwrap_or_default();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO asset_relationships (
                        id, source_asset_id, target_asset_id, relationship_type,
                        description, confidence, metadata, created_at, created_by
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(&relationship.id)
                .bind(&relationship.source_asset_id)
                .bind(&relationship.target_asset_id)
                .bind(relationship.relationship_type.as_str())
                .bind(&relationship.description)
                .bind(relationship.confidence)
                .bind(&metadata_json)
                .bind(relationship.created_at)
                .bind(&relationship.created_by)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO asset_relationships (
                        id, source_asset_id, target_asset_id, relationship_type,
                        description, confidence, metadata, created_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&relationship.id)
                .bind(&relationship.source_asset_id)
                .bind(&relationship.target_asset_id)
                .bind(relationship.relationship_type.as_str())
                .bind(&relationship.description)
                .bind(relationship.confidence)
                .bind(&metadata_json)
                .bind(relationship.created_at)
                .bind(&relationship.created_by)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO asset_relationships (
                        id, source_asset_id, target_asset_id, relationship_type,
                        description, confidence, metadata, created_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&relationship.id)
                .bind(&relationship.source_asset_id)
                .bind(&relationship.target_asset_id)
                .bind(relationship.relationship_type.as_str())
                .bind(&relationship.description)
                .bind(relationship.confidence)
                .bind(&metadata_json)
                .bind(relationship.created_at)
                .bind(&relationship.created_by)
                .execute(pool)
                .await?;
            }
        }

        Ok(relationship)
    }

    /// 获取资产的关系
    pub async fn get_asset_relationships_internal(
        &self,
        asset_id: &str,
    ) -> Result<(Vec<AssetRelationship>, Vec<AssetRelationship>)> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let outgoing_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE source_asset_id = $1
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut outgoing = Vec::new();
                for row in outgoing_rows {
                    outgoing.push(relationship_from_db_row(row));
                }

                let incoming_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE target_asset_id = $1
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut incoming = Vec::new();
                for row in incoming_rows {
                    incoming.push(relationship_from_db_row(row));
                }
                Ok((incoming, outgoing))
            }
            DatabasePool::SQLite(pool) => {
                let outgoing_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE source_asset_id = ?
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut outgoing = Vec::new();
                for row in outgoing_rows {
                    outgoing.push(relationship_from_db_row(row));
                }

                let incoming_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE target_asset_id = ?
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut incoming = Vec::new();
                for row in incoming_rows {
                    incoming.push(relationship_from_db_row(row));
                }
                Ok((incoming, outgoing))
            }
            DatabasePool::MySQL(pool) => {
                let outgoing_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE source_asset_id = ?
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut outgoing = Vec::new();
                for row in outgoing_rows {
                    outgoing.push(relationship_from_db_row(row));
                }

                let incoming_rows = sqlx::query_as::<_, AssetRelationshipDbRow>(
                    r#"
                    SELECT id, source_asset_id, target_asset_id, relationship_type,
                           description, confidence, metadata, created_at, created_by
                    FROM asset_relationships WHERE target_asset_id = ?
                    "#,
                )
                .bind(asset_id)
                .fetch_all(pool)
                .await?;
                let mut incoming = Vec::new();
                for row in incoming_rows {
                    incoming.push(relationship_from_db_row(row));
                }
                Ok((incoming, outgoing))
            }
        }
    }

    /// 批量导入资产
    pub async fn import_assets_internal(
        &self,
        request: ImportAssetsRequest,
        created_by: String,
    ) -> Result<ImportResult> {
        let mut result = ImportResult {
            total: request.assets.len(),
            created: 0,
            updated: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        for asset_request in request.assets {
            match self
                .handle_asset_import(&asset_request, &request.merge_strategy, &created_by)
                .await
            {
                Ok(action) => match action {
                    ImportAction::Created => result.created += 1,
                    ImportAction::Updated => result.updated += 1,
                    ImportAction::Skipped => result.skipped += 1,
                },
                Err(e) => {
                    result.errors.push(format!(
                        "Failed to import asset {}: {}",
                        asset_request.name, e
                    ));
                }
            }
        }

        Ok(result)
    }

    /// 处理单个资产导入
    async fn handle_asset_import(
        &self,
        request: &CreateAssetRequest,
        strategy: &MergeStrategy,
        created_by: &str,
    ) -> Result<ImportAction> {
        // 检查是否已存在
        if let Some(existing) = self
            .find_asset_by_type_and_value_internal(&request.asset_type, &request.value)
            .await?
        {
            match strategy {
                MergeStrategy::Skip => Ok(ImportAction::Skipped),
                MergeStrategy::Update | MergeStrategy::Replace => {
                    let update_request = UpdateAssetRequest {
                        project_id: request.project_id.clone(),
                        name: None,
                        value: Some(request.value.clone()),
                        description: None,
                        confidence: request.confidence,
                        status: None,
                        metadata: request.metadata.clone(),
                        tags: request.tags.clone(),
                        risk_level: request.risk_level.clone(),
                    };
                    self.update_asset_internal(&existing.id, update_request)
                        .await?;
                    Ok(ImportAction::Updated)
                }
            }
        } else {
            self.create_asset_internal(request.clone(), created_by.to_string())
                .await?;
            Ok(ImportAction::Created)
        }
    }
}

#[derive(Debug)]
enum ImportAction {
    Created,
    Updated,
    Skipped,
}
