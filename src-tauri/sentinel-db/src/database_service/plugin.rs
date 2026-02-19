use anyhow::Result;
use sentinel_plugins::PluginRecord;
use sqlx::FromRow;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

#[derive(Debug, Clone, FromRow)]
struct PluginRegistryRow {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    main_category: String,
    category: String,
    description: Option<String>,
    default_severity: String,
    tags: Option<String>,
    enabled: bool,
    metadata: String,
}

#[derive(Debug, Clone, FromRow)]
struct PluginRegistryFavoriteRow {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    main_category: String,
    category: String,
    description: Option<String>,
    default_severity: String,
    tags: Option<String>,
    enabled: bool,
    metadata: String,
    is_favorited: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct TrafficPluginScanRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub default_severity: String,
    pub tags: Option<String>,
    pub plugin_code: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct TrafficPluginReloadRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub default_severity: String,
    pub tags: Option<String>,
    pub plugin_code: String,
    pub enabled: bool,
    pub main_category: String,
}

fn row_to_plugin_record(row: PluginRegistryRow, is_favorited: bool) -> PluginRecord {
    if let Ok(mut record) = serde_json::from_str::<PluginRecord>(&row.metadata) {
        record.is_favorited = is_favorited;
        return record;
    }

    let severity = match row.default_severity.to_lowercase().as_str() {
        "critical" => sentinel_plugins::Severity::Critical,
        "high" => sentinel_plugins::Severity::High,
        "medium" => sentinel_plugins::Severity::Medium,
        "low" => sentinel_plugins::Severity::Low,
        "info" => sentinel_plugins::Severity::Info,
        _ => sentinel_plugins::Severity::Medium,
    };

    let tags = row
        .tags
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let metadata = sentinel_plugins::PluginMetadata {
        id: row.id,
        name: row.name,
        version: row.version,
        author: row.author,
        main_category: row.main_category,
        category: row.category,
        default_severity: severity,
        tags,
        description: row.description,
    };

    let status = if row.enabled {
        sentinel_plugins::PluginStatus::Enabled
    } else {
        sentinel_plugins::PluginStatus::Disabled
    };

    #[allow(deprecated)]
    PluginRecord {
        metadata,
        path: None,
        status,
        last_error: None,
        is_favorited,
    }
}

impl DatabaseService {
    pub async fn list_enabled_traffic_plugins_for_scan(
        &self,
    ) -> Result<Vec<TrafficPluginScanRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let query = r#"
            SELECT id, name, version, author, category, description,
                   default_severity, tags, plugin_code
            FROM plugin_registry
            WHERE enabled = true AND main_category = 'traffic'
            "#;

        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
        };

        Ok(rows)
    }

    pub async fn get_traffic_plugin_for_reload(
        &self,
        plugin_id: &str,
    ) -> Result<Option<TrafficPluginReloadRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, TrafficPluginReloadRow>(
                    r#"
                    SELECT id, name, version, author, category, description,
                           default_severity, tags, plugin_code, enabled, main_category
                    FROM plugin_registry
                    WHERE id = $1
                    "#,
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, TrafficPluginReloadRow>(
                    r#"
                    SELECT id, name, version, author, category, description,
                           default_severity, tags, plugin_code, enabled, main_category
                    FROM plugin_registry
                    WHERE id = ?
                    "#,
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, TrafficPluginReloadRow>(
                    r#"
                    SELECT id, name, version, author, category, description,
                           default_severity, tags, plugin_code, enabled, main_category
                    FROM plugin_registry
                    WHERE id = ?
                    "#,
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn get_active_agent_plugins_internal(&self) -> Result<Vec<PluginRecord>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let query = r#"
            SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                   p.default_severity, p.tags, p.enabled, p.metadata, p.status
            FROM plugin_registry p
            WHERE p.main_category = 'agent' 
              AND p.enabled = TRUE 
              AND p.validation_status = 'Approved'
            ORDER BY p.updated_at DESC
            "#;

        let rows: Vec<PluginRegistryRow> = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
        };

        Ok(rows
            .into_iter()
            .map(|row| row_to_plugin_record(row, false))
            .collect())
    }

    pub async fn get_plugins_from_registry_internal(&self, user_id: Option<&str>) -> Result<Vec<PluginRecord>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let uid = user_id.unwrap_or("default");
        let rows: Vec<PluginRegistryFavoriteRow> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                           p.default_severity, p.tags, p.enabled, p.metadata, p.status,
                           CASE WHEN f.plugin_id IS NOT NULL THEN 1 ELSE 0 END as is_favorited
                    FROM plugin_registry p
                    LEFT JOIN plugin_favorites f ON p.id = f.plugin_id AND f.user_id = $1
                    WHERE p.validation_status = 'Approved'
                    ORDER BY p.updated_at DESC
                    "#
                )
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                           p.default_severity, p.tags, p.enabled, p.metadata, p.status,
                           CASE WHEN f.plugin_id IS NOT NULL THEN 1 ELSE 0 END as is_favorited
                    FROM plugin_registry p
                    LEFT JOIN plugin_favorites f ON p.id = f.plugin_id AND f.user_id = ?
                    WHERE p.validation_status = 'Approved'
                    ORDER BY p.updated_at DESC
                    "#
                )
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                           p.default_severity, p.tags, p.enabled, p.metadata, p.status,
                           CASE WHEN f.plugin_id IS NOT NULL THEN 1 ELSE 0 END as is_favorited
                    FROM plugin_registry p
                    LEFT JOIN plugin_favorites f ON p.id = f.plugin_id AND f.user_id = ?
                    WHERE p.validation_status = 'Approved'
                    ORDER BY p.updated_at DESC
                    "#
                )
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| {
                row_to_plugin_record(
                    PluginRegistryRow {
                        id: row.id,
                        name: row.name,
                        version: row.version,
                        author: row.author,
                        main_category: row.main_category,
                        category: row.category,
                        description: row.description,
                        default_severity: row.default_severity,
                        tags: row.tags,
                        enabled: row.enabled,
                        metadata: row.metadata,
                    },
                    row.is_favorited == 1,
                )
            })
            .collect())
    }

    pub async fn update_plugin_status_internal(&self, plugin_id: &str, status: &str) -> Result<()> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE plugin_registry SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(status)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE plugin_registry SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE plugin_registry SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_plugin_internal(&self, metadata: &serde_json::Value, code: &str) -> Result<()> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let plugin_id = metadata["id"].as_str().ok_or_else(|| anyhow::anyhow!("Plugin ID not found"))?;
        let metadata_json = serde_json::to_string(metadata)?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO plugin_registry (id, metadata, code, status, updated_at) VALUES ($1, $2, $3, 'active', CURRENT_TIMESTAMP)
                     ON CONFLICT(id) DO UPDATE SET metadata = excluded.metadata, code = excluded.code, updated_at = excluded.updated_at"
                )
                .bind(plugin_id)
                .bind(&metadata_json)
                .bind(code)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO plugin_registry (id, metadata, code, status, updated_at) VALUES (?, ?, ?, 'active', CURRENT_TIMESTAMP)
                     ON CONFLICT(id) DO UPDATE SET metadata = excluded.metadata, code = excluded.code, updated_at = excluded.updated_at"
                )
                .bind(plugin_id)
                .bind(&metadata_json)
                .bind(code)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO plugin_registry (id, metadata, code, status, updated_at) VALUES (?, ?, ?, 'active', CURRENT_TIMESTAMP)
                     ON DUPLICATE KEY UPDATE metadata = VALUES(metadata), code = VALUES(code), updated_at = CURRENT_TIMESTAMP"
                )
                .bind(plugin_id)
                .bind(&metadata_json)
                .bind(code)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_plugin_from_registry_internal(&self, plugin_id: &str) -> Result<Option<PluginRecord>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row: Option<PluginRegistryRow> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, name, version, author, main_category, category, description,
                           default_severity, tags, enabled, metadata, status
                    FROM plugin_registry 
                    WHERE id = $1
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, name, version, author, main_category, category, description,
                           default_severity, tags, enabled, metadata, status
                    FROM plugin_registry 
                    WHERE id = ?
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as(
                    r#"
                    SELECT id, name, version, author, main_category, category, description,
                           default_severity, tags, enabled, metadata, status
                    FROM plugin_registry 
                    WHERE id = ?
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
        };

        if let Some(row) = row {
            Ok(Some(row_to_plugin_record(row, false)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_plugin_code_internal(&self, plugin_id: &str) -> Result<Option<String>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let code: Option<String> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(NULLIF(plugin_code, ''), NULLIF(code, '')) as effective_code 
                    FROM plugin_registry 
                    WHERE id = $1
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(NULLIF(plugin_code, ''), NULLIF(code, '')) as effective_code 
                    FROM plugin_registry 
                    WHERE id = ?
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(NULLIF(plugin_code, ''), NULLIF(code, '')) as effective_code 
                    FROM plugin_registry 
                    WHERE id = ?
                    "#
                )
                .bind(plugin_id)
                .fetch_optional(pool)
                .await?
            }
        };
        Ok(code)
    }

    pub async fn delete_plugin_from_registry_internal(&self, plugin_id: &str) -> Result<()> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM plugin_registry WHERE id = $1")
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_plugins_paginated_internal(
        &self,
        page: i64,
        page_size: i64,
        status_filter: Option<&str>,
        search_text: Option<&str>,
        _user_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let offset = (page - 1) * page_size;
        let plugins = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let mut query_str = "SELECT * FROM plugin_registry WHERE 1=1".to_string();
                let mut param_idx = 1;
                let mut has_status = false;
                let mut has_search = false;
                if status_filter.is_some() {
                    query_str.push_str(&format!(" AND status = ${}", param_idx));
                    param_idx += 1;
                    has_status = true;
                }
                if search_text.is_some() {
                    query_str.push_str(&format!(" AND (id LIKE ${0} OR metadata LIKE ${0} OR name LIKE ${0} OR description LIKE ${0})", param_idx));
                    param_idx += 1;
                    has_search = true;
                }
                query_str.push_str(&format!(" ORDER BY updated_at DESC LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));
                let mut q = sqlx::query(&query_str);
                if has_status {
                    q = q.bind(status_filter.unwrap());
                }
                if has_search {
                    q = q.bind(format!("%{}%", search_text.unwrap()));
                }
                q = q.bind(page_size).bind(offset);
                let rows = q.fetch_all(pool).await?;
                let mut plugins = Vec::new();
                for row in rows {
                    let metadata_json: String = sqlx::Row::get(&row, "metadata");
                    if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                        if metadata.is_object() && !metadata.as_object().unwrap().is_empty() {
                            plugins.push(metadata);
                            continue;
                        }
                    }
                    let id: String = sqlx::Row::get(&row, "id");
                    let name: String = sqlx::Row::get(&row, "name");
                    let version: String = sqlx::Row::get(&row, "version");
                    let author: Option<String> = sqlx::Row::get(&row, "author");
                    let main_category: String = sqlx::Row::get(&row, "main_category");
                    let category: String = sqlx::Row::get(&row, "category");
                    let description: Option<String> = sqlx::Row::get(&row, "description");
                    let default_severity: String = sqlx::Row::get(&row, "default_severity");
                    let tags_json: Option<String> = sqlx::Row::get(&row, "tags");
                    let enabled: bool = sqlx::Row::get(&row, "enabled");
                    let status: String = sqlx::Row::get(&row, "status");
                    let tags: serde_json::Value = tags_json
                        .and_then(|t| serde_json::from_str(&t).ok())
                        .unwrap_or_else(|| serde_json::json!([]));
                    plugins.push(serde_json::json!({
                        "id": id, "name": name, "version": version, "author": author,
                        "main_category": main_category, "category": category, "description": description,
                        "default_severity": default_severity, "tags": tags, "enabled": enabled, "status": status,
                    }));
                }
                plugins
            }
            DatabasePool::SQLite(pool) => {
                let mut query_str = "SELECT * FROM plugin_registry WHERE 1=1".to_string();
                let mut has_status = false;
                let mut has_search = false;
                if status_filter.is_some() {
                    query_str.push_str(" AND status = ?");
                    has_status = true;
                }
                if search_text.is_some() {
                    query_str.push_str(" AND (id LIKE ? OR metadata LIKE ? OR name LIKE ? OR description LIKE ?)");
                    has_search = true;
                }
                query_str.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
                let mut q = sqlx::query(&query_str);
                if has_status {
                    q = q.bind(status_filter.unwrap());
                }
                if has_search {
                    let s = format!("%{}%", search_text.unwrap());
                    q = q.bind(s.clone()).bind(s.clone()).bind(s.clone()).bind(s);
                }
                q = q.bind(page_size).bind(offset);
                let rows = q.fetch_all(pool).await?;
                let mut plugins = Vec::new();
                for row in rows {
                    let metadata_json: String = sqlx::Row::get(&row, "metadata");
                    if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                        if metadata.is_object() && !metadata.as_object().unwrap().is_empty() {
                            plugins.push(metadata);
                            continue;
                        }
                    }
                    let id: String = sqlx::Row::get(&row, "id");
                    let name: String = sqlx::Row::get(&row, "name");
                    let version: String = sqlx::Row::get(&row, "version");
                    let author: Option<String> = sqlx::Row::get(&row, "author");
                    let main_category: String = sqlx::Row::get(&row, "main_category");
                    let category: String = sqlx::Row::get(&row, "category");
                    let description: Option<String> = sqlx::Row::get(&row, "description");
                    let default_severity: String = sqlx::Row::get(&row, "default_severity");
                    let tags_json: Option<String> = sqlx::Row::get(&row, "tags");
                    let enabled: bool = sqlx::Row::get(&row, "enabled");
                    let status: String = sqlx::Row::get(&row, "status");
                    let tags: serde_json::Value = tags_json
                        .and_then(|t| serde_json::from_str(&t).ok())
                        .unwrap_or_else(|| serde_json::json!([]));
                    plugins.push(serde_json::json!({
                        "id": id, "name": name, "version": version, "author": author,
                        "main_category": main_category, "category": category, "description": description,
                        "default_severity": default_severity, "tags": tags, "enabled": enabled, "status": status,
                    }));
                }
                plugins
            }
            DatabasePool::MySQL(pool) => {
                let mut query_str = "SELECT * FROM plugin_registry WHERE 1=1".to_string();
                let mut has_status = false;
                let mut has_search = false;
                if status_filter.is_some() {
                    query_str.push_str(" AND status = ?");
                    has_status = true;
                }
                if search_text.is_some() {
                    query_str.push_str(" AND (id LIKE ? OR metadata LIKE ? OR name LIKE ? OR description LIKE ?)");
                    has_search = true;
                }
                query_str.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
                let mut q = sqlx::query(&query_str);
                if has_status {
                    q = q.bind(status_filter.unwrap());
                }
                if has_search {
                    let s = format!("%{}%", search_text.unwrap());
                    q = q.bind(s.clone()).bind(s.clone()).bind(s.clone()).bind(s);
                }
                q = q.bind(page_size).bind(offset);
                let rows = q.fetch_all(pool).await?;
                let mut plugins = Vec::new();
                for row in rows {
                    let metadata_json: String = sqlx::Row::get(&row, "metadata");
                    if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                        if metadata.is_object() && !metadata.as_object().unwrap().is_empty() {
                            plugins.push(metadata);
                            continue;
                        }
                    }
                    let id: String = sqlx::Row::get(&row, "id");
                    let name: String = sqlx::Row::get(&row, "name");
                    let version: String = sqlx::Row::get(&row, "version");
                    let author: Option<String> = sqlx::Row::get(&row, "author");
                    let main_category: String = sqlx::Row::get(&row, "main_category");
                    let category: String = sqlx::Row::get(&row, "category");
                    let description: Option<String> = sqlx::Row::get(&row, "description");
                    let default_severity: String = sqlx::Row::get(&row, "default_severity");
                    let tags_json: Option<String> = sqlx::Row::get(&row, "tags");
                    let enabled: bool = sqlx::Row::get(&row, "enabled");
                    let status: String = sqlx::Row::get(&row, "status");
                    let tags: serde_json::Value = tags_json
                        .and_then(|t| serde_json::from_str(&t).ok())
                        .unwrap_or_else(|| serde_json::json!([]));
                    plugins.push(serde_json::json!({
                        "id": id, "name": name, "version": version, "author": author,
                        "main_category": main_category, "category": category, "description": description,
                        "default_severity": default_severity, "tags": tags, "enabled": enabled, "status": status,
                    }));
                }
                plugins
            }
        };

        let total_count: i64 = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
            DatabasePool::SQLite(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
            DatabasePool::MySQL(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
        };

        Ok(serde_json::json!({
            "items": plugins,
            "total": total_count,
            "page": page,
            "page_size": page_size
        }))
    }

    pub async fn toggle_plugin_favorite_internal(&self, plugin_id: &str, user_id: Option<&str>) -> Result<bool> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let uid = user_id.unwrap_or("default_user");
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let exists = sqlx::query("SELECT 1 FROM plugin_favorites WHERE plugin_id = $1 AND user_id = $2")
                    .bind(plugin_id)
                    .bind(uid)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() {
                    sqlx::query("DELETE FROM plugin_favorites WHERE plugin_id = $1 AND user_id = $2")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(false)
                } else {
                    sqlx::query("INSERT INTO plugin_favorites (plugin_id, user_id) VALUES ($1, $2)")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(true)
                }
            }
            DatabasePool::SQLite(pool) => {
                let exists = sqlx::query("SELECT 1 FROM plugin_favorites WHERE plugin_id = ? AND user_id = ?")
                    .bind(plugin_id)
                    .bind(uid)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() {
                    sqlx::query("DELETE FROM plugin_favorites WHERE plugin_id = ? AND user_id = ?")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(false)
                } else {
                    sqlx::query("INSERT INTO plugin_favorites (plugin_id, user_id) VALUES (?, ?)")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(true)
                }
            }
            DatabasePool::MySQL(pool) => {
                let exists = sqlx::query("SELECT 1 FROM plugin_favorites WHERE plugin_id = ? AND user_id = ?")
                    .bind(plugin_id)
                    .bind(uid)
                    .fetch_optional(pool)
                    .await?;
                if exists.is_some() {
                    sqlx::query("DELETE FROM plugin_favorites WHERE plugin_id = ? AND user_id = ?")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(false)
                } else {
                    sqlx::query("INSERT INTO plugin_favorites (plugin_id, user_id) VALUES (?, ?)")
                        .bind(plugin_id)
                        .bind(uid)
                        .execute(pool)
                        .await?;
                    Ok(true)
                }
            }
        }
    }

    pub async fn get_favorited_plugins_internal(&self, user_id: Option<&str>) -> Result<Vec<String>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let uid = user_id.unwrap_or("default_user");

        let plugin_ids = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query("SELECT plugin_id FROM plugin_favorites WHERE user_id = $1")
                    .bind(uid)
                    .fetch_all(pool)
                    .await?;
                rows.into_iter().map(|r| sqlx::Row::get(&r, "plugin_id")).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query("SELECT plugin_id FROM plugin_favorites WHERE user_id = ?")
                    .bind(uid)
                    .fetch_all(pool)
                    .await?;
                rows.into_iter().map(|r| sqlx::Row::get(&r, "plugin_id")).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query("SELECT plugin_id FROM plugin_favorites WHERE user_id = ?")
                    .bind(uid)
                    .fetch_all(pool)
                    .await?;
                rows.into_iter().map(|r| sqlx::Row::get(&r, "plugin_id")).collect()
            }
        };

        Ok(plugin_ids)
    }

    pub async fn get_plugin_review_stats_internal(&self) -> Result<serde_json::Value> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let (total, active, pending): (i64, i64, i64) = match runtime {
            DatabasePool::PostgreSQL(pool) => (
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'active'").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'pending'").fetch_one(pool).await?,
            ),
            DatabasePool::SQLite(pool) => (
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'active'").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'pending'").fetch_one(pool).await?,
            ),
            DatabasePool::MySQL(pool) => (
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'active'").fetch_one(pool).await?,
                sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'pending'").fetch_one(pool).await?,
            ),
        };

        Ok(serde_json::json!({
            "total": total,
            "active": active,
            "pending": pending
        }))
    }

    pub async fn update_plugin_enabled_internal(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE plugin_registry SET enabled = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(enabled)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE plugin_registry SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(enabled)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE plugin_registry SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(enabled)
                    .bind(plugin_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_plugin_name_internal(&self, plugin_id: &str) -> Result<Option<String>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let name: Option<String> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar("SELECT name FROM plugin_registry WHERE id = $1")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar("SELECT name FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar("SELECT name FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(name)
    }

    pub async fn get_plugin_summary_internal(&self, plugin_id: &str) -> Result<Option<(String, bool)>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row: Option<(String, bool)> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT main_category, enabled FROM plugin_registry WHERE id = $1")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT main_category, enabled FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT main_category, enabled FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row)
    }

    pub async fn get_plugin_tags_internal(&self, plugin_id: &str) -> Result<Vec<String>> {
        let runtime = self.runtime_pool.as_ref().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let tags_json: Option<String> = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = $1")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = ?")
                    .bind(plugin_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        
        let tags = tags_json
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Ok(tags)
    }
}
