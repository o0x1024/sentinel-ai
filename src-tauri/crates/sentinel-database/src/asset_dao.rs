use sentinel_models::asset::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePool, Row};
use serde_json;
use std::collections::HashMap;

pub struct AssetDao {
    pool: SqlitePool,
}

impl AssetDao {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建资产
    pub async fn create_asset(&self, request: CreateAssetRequest, created_by: String) -> Result<Asset> {
        let mut asset = Asset::new(
            request.asset_type.clone(),
            request.name.clone(),
            request.value.clone(),
            created_by.clone(),
        )
        .with_source(
            request.source.unwrap_or_default(),
            request.source_scan_id,
        )
        .with_metadata(request.metadata.unwrap_or_default())
        .with_tags(request.tags.unwrap_or_default())
        .with_risk_level(request.risk_level.unwrap_or(RiskLevel::Unknown))
        .with_confidence(request.confidence.unwrap_or(1.0));

        // 设置project_id
        asset.project_id = request.project_id;

        let metadata_json = serde_json::to_string(&asset.metadata).unwrap_or_default();
        let tags_json = serde_json::to_string(&asset.tags).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO assets (
                id, project_id, asset_type, name, value, description, confidence, status,
                source, source_scan_id, metadata, tags, risk_level,
                last_seen, first_seen, created_at, updated_at, created_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
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
        .bind(asset.last_seen.to_rfc3339())
        .bind(asset.first_seen.to_rfc3339())
        .bind(asset.created_at.to_rfc3339())
        .bind(asset.updated_at.to_rfc3339())
        .bind(&asset.created_by)
        .execute(&self.pool)
        .await?;

        Ok(asset)
    }

    /// 根据ID获取资产
    pub async fn get_asset_by_id(&self, id: &str) -> Result<Option<Asset>> {
        let row = sqlx::query(
            r#"
            SELECT id, project_id, asset_type, name, value, description, confidence, status,
                   source, source_scan_id, metadata, tags, risk_level,
                   first_seen, last_seen, created_at, updated_at, created_by
            FROM assets WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_asset(&row)?)),
            None => Ok(None),
        }
    }

    /// 根据类型和值查找资产
    pub async fn find_asset_by_type_and_value(
        &self,
        asset_type: &AssetType,
        value: &str,
    ) -> Result<Option<Asset>> {
        let row = sqlx::query(
            r#"
            SELECT id, project_id, asset_type, name, value, description, confidence, status,
                   source, source_scan_id, metadata, tags, risk_level,
                   first_seen, last_seen, created_at, updated_at, created_by
            FROM assets WHERE asset_type = ?1 AND value = ?2
            "#,
        )
        .bind(asset_type.as_str())
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_asset(&row)?)),
            None => Ok(None),
        }
    }

    /// 更新资产
    pub async fn update_asset(&self, id: &str, request: UpdateAssetRequest) -> Result<bool> {
        let  _updates: Vec<String> = Vec::new();
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE assets SET ");
        let mut has_updates = false;

        if let Some(project_id) = &request.project_id {
            if has_updates { query_builder.push(", "); }
            query_builder.push("project_id = ").push_bind(project_id);
            has_updates = true;
        }

        if let Some(name) = &request.name {
            if has_updates { query_builder.push(", "); }
            query_builder.push("name = ").push_bind(name);
            has_updates = true;
        }

        if let Some(value) = &request.value {
            if has_updates { query_builder.push(", "); }
            query_builder.push("value = ").push_bind(value);
            has_updates = true;
        }

        if let Some(description) = &request.description {
            if has_updates { query_builder.push(", "); }
            query_builder.push("description = ").push_bind(description);
            has_updates = true;
        }

        if let Some(confidence) = request.confidence {
            if has_updates { query_builder.push(", "); }
            query_builder.push("confidence = ").push_bind(confidence as f64);
            has_updates = true;
        }

        if let Some(status) = &request.status {
            if has_updates { query_builder.push(", "); }
            query_builder.push("status = ").push_bind(status.as_str());
            has_updates = true;
        }

        if let Some(metadata) = &request.metadata {
            let metadata_json = serde_json::to_string(metadata).unwrap_or_default();
            if has_updates { query_builder.push(", "); }
            query_builder.push("metadata = ").push_bind(metadata_json);
            has_updates = true;
        }

        if let Some(tags) = &request.tags {
            let tags_json = serde_json::to_string(tags).unwrap_or_default();
            if has_updates { query_builder.push(", "); }
            query_builder.push("tags = ").push_bind(tags_json);
            has_updates = true;
        }

        if let Some(risk_level) = &request.risk_level {
            if has_updates { query_builder.push(", "); }
            query_builder.push("risk_level = ").push_bind(risk_level.as_str());
            has_updates = true;
        }

        if !has_updates {
            return Ok(false);
        }

        query_builder.push(", updated_at = ").push_bind(Utc::now().to_rfc3339());
        query_builder.push(" WHERE id = ").push_bind(id);

        let result = query_builder.build().execute(&self.pool).await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除资产
    pub async fn delete_asset(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM assets WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// 查询资产列表
    pub async fn list_assets(
        &self,
        filter: Option<AssetFilter>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Asset>> {
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
                query_builder.push("(name LIKE ").push_bind(search_pattern.clone())
                    .push(" OR value LIKE ").push_bind(search_pattern.clone())
                    .push(" OR description LIKE ").push_bind(search_pattern)
                    .push(")");
            }

            if let Some(created_after) = filter.created_after {
                if !has_conditions {
                    query_builder.push(" WHERE ");
                    has_conditions = true;
                } else {
                    query_builder.push(" AND ");
                }
                query_builder.push("created_at >= ").push_bind(created_after.to_rfc3339());
            }

            if let Some(created_before) = filter.created_before {
                if !has_conditions {
                    query_builder.push(" WHERE ");
                    has_conditions = true;
                } else {
                    query_builder.push(" AND ");
                }
                query_builder.push("created_at <= ").push_bind(created_before.to_rfc3339());
            }

            if let Some(last_seen_after) = filter.last_seen_after {
                if !has_conditions {
                    query_builder.push(" WHERE ");
                    has_conditions = true;
                } else {
                    query_builder.push(" AND ");
                }
                query_builder.push("last_seen >= ").push_bind(last_seen_after.to_rfc3339());
            }

            if let Some(last_seen_before) = filter.last_seen_before {
                if !has_conditions {
                    query_builder.push(" WHERE ");
                } else {
                    query_builder.push(" AND ");
                }
                query_builder.push("last_seen <= ").push_bind(last_seen_before.to_rfc3339());
            }
        }

        query_builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ").push_bind(limit as i64);
            if let Some(offset) = offset {
                query_builder.push(" OFFSET ").push_bind(offset as i64);
            }
        }

        let rows = query_builder.build().fetch_all(&self.pool).await?;

        let mut assets = Vec::new();
        for row in rows {
            assets.push(self.row_to_asset(&row)?);
        }

        Ok(assets)
    }

    /// 获取资产统计信息
    pub async fn get_asset_stats(&self) -> Result<AssetStats> {
        let total_assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
            .fetch_one(&self.pool)
            .await?;

        // 按类型统计
        let mut by_type = HashMap::new();
        let type_rows = sqlx::query("SELECT asset_type, COUNT(*) FROM assets GROUP BY asset_type")
            .fetch_all(&self.pool)
            .await?;
        for row in type_rows {
            let asset_type: String = row.try_get(0)?;
            let count: i64 = row.try_get(1)?;
            by_type.insert(asset_type, count as f64);
        }

        // 按状态统计
        let mut by_status = HashMap::new();
        let status_rows = sqlx::query("SELECT status, COUNT(*) FROM assets GROUP BY status")
            .fetch_all(&self.pool)
            .await?;
        for row in status_rows {
            let status: String = row.try_get(0)?;
            let count: i64 = row.try_get(1)?;
            by_status.insert(status, count as f64);
        }

        // 按风险等级统计
        let mut by_risk_level = HashMap::new();
        let risk_rows = sqlx::query("SELECT risk_level, COUNT(*) FROM assets GROUP BY risk_level")
            .fetch_all(&self.pool)
            .await?;
        for row in risk_rows {
            let risk_level: String = row.try_get(0)?;
            let count: i64 = row.try_get(1)?;
            by_risk_level.insert(risk_level, count as f64);
        }

        // 按来源统计
        let mut by_source = HashMap::new();
        let source_rows = sqlx::query("SELECT source, COUNT(*) FROM assets WHERE source IS NOT NULL GROUP BY source")
            .fetch_all(&self.pool)
            .await?;
        for row in source_rows {
            let source: String = row.try_get(0)?;
            let count: i64 = row.try_get(1)?;
            by_source.insert(source, count as f64);
        }

        // 最近24小时新增
        let recent_additions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM assets WHERE created_at >= datetime('now', '-1 day')"
        )
        .fetch_one(&self.pool)
        .await?;

        // 超过30天未更新
        let stale_assets: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM assets WHERE updated_at <= datetime('now', '-30 days')"
        )
        .fetch_one(&self.pool)
        .await?;

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
    pub async fn create_relationship(
        &self,
        source_asset_id: String,
        target_asset_id: String,
        relationship_type: RelationshipType,
        created_by: String,
    ) -> Result<AssetRelationship> {
        let relationship = AssetRelationship::new(
            source_asset_id,
            target_asset_id,
            relationship_type,
            created_by,
        );

        let metadata_json = serde_json::to_string(&relationship.metadata).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO asset_relationships (
                id, source_asset_id, target_asset_id, relationship_type,
                description, confidence, metadata, created_at, created_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(&relationship.id)
        .bind(&relationship.source_asset_id)
        .bind(&relationship.target_asset_id)
        .bind(relationship.relationship_type.as_str())
        .bind(&relationship.description)
        .bind(relationship.confidence)
        .bind(&metadata_json)
        .bind(relationship.created_at.to_rfc3339())
        .bind(&relationship.created_by)
        .execute(&self.pool)
        .await?;

        Ok(relationship)
    }

    /// 获取资产的关系
    pub async fn get_asset_relationships(&self, asset_id: &str) -> Result<(Vec<AssetRelationship>, Vec<AssetRelationship>)> {
        // 获取传出关系
        let outgoing_rows = sqlx::query(
            r#"
            SELECT id, source_asset_id, target_asset_id, relationship_type,
                   description, confidence, metadata, created_at, created_by
            FROM asset_relationships WHERE source_asset_id = ?1
            "#,
        )
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut outgoing = Vec::new();
        for row in outgoing_rows {
            outgoing.push(self.row_to_relationship(&row)?);
        }

        // 获取传入关系
        let incoming_rows = sqlx::query(
            r#"
            SELECT id, source_asset_id, target_asset_id, relationship_type,
                   description, confidence, metadata, created_at, created_by
            FROM asset_relationships WHERE target_asset_id = ?1
            "#,
        )
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut incoming = Vec::new();
        for row in incoming_rows {
            incoming.push(self.row_to_relationship(&row)?);
        }

        Ok((incoming, outgoing))
    }

    /// 批量导入资产
    pub async fn import_assets(
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
            match self.handle_asset_import(&asset_request, &request.merge_strategy, &created_by).await {
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
        if let Some(existing) = self.find_asset_by_type_and_value(&request.asset_type, &request.value).await? {
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
                    self.update_asset(&existing.id, update_request).await?;
                    Ok(ImportAction::Updated)
                }
            }
        } else {
            self.create_asset(request.clone(), created_by.to_string()).await?;
            Ok(ImportAction::Created)
        }
    }

    /// 将数据库行转换为资产对象
    fn row_to_asset(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Asset> {
        use sqlx::Row as _;
        
        let metadata_str: String = row.try_get("metadata")?;
        let metadata: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&metadata_str).unwrap_or_default();

        let tags_str: String = row.try_get("tags")?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

        let asset_type_str: String = row.try_get("asset_type")?;
        let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Domain);

        let status_str: String = row.try_get("status")?;
        let status = match status_str.as_str() {
            "active" => AssetStatus::Active,
            "inactive" => AssetStatus::Inactive,
            "verified" => AssetStatus::Verified,
            "unverified" => AssetStatus::Unverified,
            _ => AssetStatus::Active,
        };

        let risk_level_str: String = row.try_get("risk_level")?;
        let risk_level = RiskLevel::from_str(&risk_level_str);

        let last_seen_str: String = row.try_get("last_seen")?;
        let last_seen = DateTime::parse_from_rfc3339(&last_seen_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse last_seen: {}", e))?
            .with_timezone(&Utc);

        let first_seen_str: String = row.try_get("first_seen")?;
        let first_seen = DateTime::parse_from_rfc3339(&first_seen_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse first_seen: {}", e))?
            .with_timezone(&Utc);

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse created_at: {}", e))?
            .with_timezone(&Utc);

        let updated_at_str: String = row.try_get("updated_at")?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse updated_at: {}", e))?
            .with_timezone(&Utc);

        Ok(Asset {
            id: row.try_get("id")?,
            project_id: row.try_get("project_id").ok(),
            asset_type,
            name: row.try_get("name")?,
            value: row.try_get("value")?,
            description: row.try_get("description").ok(),
            confidence: row.try_get("confidence")?,
            status,
            source: row.try_get("source").ok(),
            source_scan_id: row.try_get("source_scan_id").ok(),
            metadata,
            tags,
            risk_level,
            last_seen,
            first_seen,
            created_at,
            updated_at,
            created_by: row.try_get("created_by")?,
        })
    }

    /// 将数据库行转换为关系对象
    fn row_to_relationship(&self, row: &sqlx::sqlite::SqliteRow) -> Result<AssetRelationship> {
        use sqlx::Row as _;
        
        let metadata_str: String = row.try_get("metadata")?;
        let metadata: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&metadata_str).unwrap_or_default();

        let relationship_type_str: String = row.try_get("relationship_type")?;
        let relationship_type = match relationship_type_str.as_str() {
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

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse created_at: {}", e))?
            .with_timezone(&Utc);

        Ok(AssetRelationship {
            id: row.try_get("id")?,
            source_asset_id: row.try_get("source_asset_id")?,
            target_asset_id: row.try_get("target_asset_id")?,
            relationship_type,
            description: row.try_get("description")?,
            confidence: row.try_get("confidence")?,
            metadata,
            created_at,
            created_by: row.try_get("created_by")?,
        })
    }
}

#[derive(Debug)]
enum ImportAction {
    Created,
    Updated,
    Skipped,
}