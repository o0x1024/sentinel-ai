use anyhow::Result;
use sentinel_plugins::PluginRecord;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn get_active_agent_plugins_internal(&self) -> Result<Vec<PluginRecord>> {
        let pool = self.get_pool()?;
        
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, p.version, p.author, p.main_category, p.category, p.description,
                   p.default_severity, p.tags, p.enabled, p.metadata, p.status
            FROM plugin_registry p
            WHERE p.main_category = 'agent' 
              AND p.enabled = TRUE 
              AND p.validation_status = 'Approved'
            ORDER BY p.updated_at DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut plugins = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = sqlx::Row::get(&row, "id");
            let name: String = sqlx::Row::get(&row, "name");
            let version: String = sqlx::Row::get(&row, "version");
            let author: Option<String> = sqlx::Row::get(&row, "author");
            let main_category: String = sqlx::Row::get(&row, "main_category");
            let category: String = sqlx::Row::get(&row, "category");
            let description: Option<String> = sqlx::Row::get(&row, "description");
            let default_severity_str: String = sqlx::Row::get(&row, "default_severity");
            let tags_json: Option<String> = sqlx::Row::get(&row, "tags");

            let severity = match default_severity_str.to_lowercase().as_str() {
                "critical" => sentinel_plugins::Severity::Critical,
                "high" => sentinel_plugins::Severity::High,
                "medium" => sentinel_plugins::Severity::Medium,
                "low" => sentinel_plugins::Severity::Low,
                "info" => sentinel_plugins::Severity::Info,
                _ => sentinel_plugins::Severity::Medium,
            };

            let tags = tags_json
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            let metadata = sentinel_plugins::PluginMetadata {
                id,
                name,
                version,
                author,
                main_category,
                category,
                default_severity: severity,
                tags,
                description,
            };

            #[allow(deprecated)]
            plugins.push(PluginRecord {
                metadata,
                path: None,
                status: sentinel_plugins::PluginStatus::Enabled,
                last_error: None,
                is_favorited: false,
            });
        }
        Ok(plugins)
    }

    pub async fn get_plugins_from_registry_internal(&self, user_id: Option<&str>) -> Result<Vec<PluginRecord>> {
        let pool = self.get_pool()?;
        let uid = user_id.unwrap_or("default");
        
        let rows = sqlx::query(
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
        .await?;

        let mut plugins = Vec::with_capacity(rows.len());
        for row in rows {
            let metadata_json: String = sqlx::Row::get(&row, "metadata");
            let is_favorited: bool = sqlx::Row::get::<i32, _>(&row, "is_favorited") == 1; // CASE returns int, usually 4 bytes in PG? Use i32 or i64.
                                                                                           // CASE 1/0 is int.
            
            // 尝试直接从 metadata JSON 解析（旧模式）
            if let Ok(mut record) = serde_json::from_str::<PluginRecord>(&metadata_json) {
                record.is_favorited = is_favorited;
                plugins.push(record);
                continue;
            }

            // 如果失败，从多列手动构造（新模式）
            let id: String = sqlx::Row::get(&row, "id");
            let name: String = sqlx::Row::get(&row, "name");
            let version: String = sqlx::Row::get(&row, "version");
            let author: Option<String> = sqlx::Row::get(&row, "author");
            let main_category: String = sqlx::Row::get(&row, "main_category");
            let category: String = sqlx::Row::get(&row, "category");
            let description: Option<String> = sqlx::Row::get(&row, "description");
            let default_severity_str: String = sqlx::Row::get(&row, "default_severity");
            let tags_json: Option<String> = sqlx::Row::get(&row, "tags");
            let enabled: bool = sqlx::Row::get(&row, "enabled");

            let severity = match default_severity_str.to_lowercase().as_str() {
                "critical" => sentinel_plugins::Severity::Critical,
                "high" => sentinel_plugins::Severity::High,
                "medium" => sentinel_plugins::Severity::Medium,
                "low" => sentinel_plugins::Severity::Low,
                "info" => sentinel_plugins::Severity::Info,
                _ => sentinel_plugins::Severity::Medium,
            };

            let tags = tags_json
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            let metadata = sentinel_plugins::PluginMetadata {
                id,
                name,
                version,
                author,
                main_category,
                category,
                default_severity: severity,
                tags,
                description,
            };

            let status = if enabled {
                sentinel_plugins::PluginStatus::Enabled
            } else {
                sentinel_plugins::PluginStatus::Disabled
            };

            #[allow(deprecated)]
            plugins.push(PluginRecord {
                metadata,
                path: None,
                status,
                last_error: None,
                is_favorited,
            });
        }
        Ok(plugins)
    }

    pub async fn update_plugin_status_internal(&self, plugin_id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE plugin_registry SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(status)
            .bind(plugin_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_plugin_internal(&self, metadata: &serde_json::Value, code: &str) -> Result<()> {
        let pool = self.get_pool()?;
        let plugin_id = metadata["id"].as_str().ok_or_else(|| anyhow::anyhow!("Plugin ID not found"))?;
        let metadata_json = serde_json::to_string(metadata)?;

        sqlx::query(
            "INSERT INTO plugin_registry (id, metadata, code, status, updated_at) VALUES ($1, $2, $3, 'active', CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET metadata = excluded.metadata, code = excluded.code, updated_at = excluded.updated_at"
        )
        .bind(plugin_id)
        .bind(metadata_json)
        .bind(code)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_plugin_from_registry_internal(&self, plugin_id: &str) -> Result<Option<PluginRecord>> {
        let pool = self.get_pool()?;
        
        let row = sqlx::query(
            r#"
            SELECT id, name, version, author, main_category, category, description,
                   default_severity, tags, enabled, metadata, status
            FROM plugin_registry 
            WHERE id = $1
            "#
        )
        .bind(plugin_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let id: String = sqlx::Row::get(&row, "id");
            let name: String = sqlx::Row::get(&row, "name");
            let version: String = sqlx::Row::get(&row, "version");
            let author: Option<String> = sqlx::Row::get(&row, "author");
            let main_category: String = sqlx::Row::get(&row, "main_category");
            let category: String = sqlx::Row::get(&row, "category");
            let description: Option<String> = sqlx::Row::get(&row, "description");
            let default_severity_str: String = sqlx::Row::get(&row, "default_severity");
            let tags_json: Option<String> = sqlx::Row::get(&row, "tags");
            let enabled: bool = sqlx::Row::get(&row, "enabled");
            let metadata_json: String = sqlx::Row::get(&row, "metadata");

            let mut record: Option<PluginRecord> = serde_json::from_str(&metadata_json).ok();

            if let Some(ref mut r) = record {
                return Ok(Some(r.clone()));
            }

            let severity = match default_severity_str.to_lowercase().as_str() {
                "critical" => sentinel_plugins::Severity::Critical,
                "high" => sentinel_plugins::Severity::High,
                "medium" => sentinel_plugins::Severity::Medium,
                "low" => sentinel_plugins::Severity::Low,
                "info" => sentinel_plugins::Severity::Info,
                _ => sentinel_plugins::Severity::Medium,
            };

            let tags = tags_json
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            let metadata = sentinel_plugins::PluginMetadata {
                id,
                name,
                version,
                author,
                main_category,
                category,
                default_severity: severity,
                tags,
                description,
            };

            let status = if enabled {
                sentinel_plugins::PluginStatus::Enabled
            } else {
                sentinel_plugins::PluginStatus::Disabled
            };

            #[allow(deprecated)]
            Ok(Some(PluginRecord {
                metadata,
                path: None,
                status,
                last_error: None,
                is_favorited: false,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_plugin_code_internal(&self, plugin_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let code: Option<String> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(NULLIF(plugin_code, ''), NULLIF(code, '')) as effective_code 
            FROM plugin_registry 
            WHERE id = $1
            "#
        )
        .bind(plugin_id)
        .fetch_optional(pool)
        .await?;
        Ok(code)
    }

    pub async fn delete_plugin_from_registry_internal(&self, plugin_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM plugin_registry WHERE id = $1")
            .bind(plugin_id)
            .execute(pool)
            .await?;
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
        let pool = self.get_pool()?;
        let offset = (page - 1) * page_size;

        let mut query_str = "SELECT * FROM plugin_registry WHERE 1=1".to_string();
        let mut param_idx = 1;
        
        // Actually, we can use if logic to add WHERE clauses and then bind later, but we need to track values.
        // It's easier to assume limited params and handle them.
        
        let mut has_status = false;
        let mut has_search = false;

        if status_filter.is_some() {
            query_str.push_str(&format!(" AND status = ${}", param_idx));
            param_idx += 1;
            has_status = true;
        }
        if search_text.is_some() {
            query_str.push_str(&format!(" AND (id LIKE ${0} OR metadata LIKE ${0} OR name LIKE ${0} OR description LIKE ${0})", param_idx));
            // Postgres supports using the same parameter index multiple times.
            param_idx += 1;
            has_search = true;
        }
        
        query_str.push_str(&format!(" ORDER BY updated_at DESC LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));
        
        let mut q = sqlx::query(&query_str);
        
        if has_status {
            q = q.bind(status_filter.unwrap());
        }
        if has_search {
            let s = format!("%{}%", search_text.unwrap());
            q = q.bind(s);
        }
        
        q = q.bind(page_size).bind(offset);

        let rows = q.fetch_all(pool).await?;

        let mut plugins = Vec::new();
        for row in rows {
            let metadata_json: String = sqlx::Row::get(&row, "metadata");
            
            // 尝试解析完整的 metadata (旧模式)
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                // 如果 metadata 是个非空对象，则优先使用它
                if metadata.is_object() && !metadata.as_object().unwrap().is_empty() {
                    plugins.push(metadata);
                    continue;
                }
            }
            
            // 如果 metadata 为空或 {}，手动构造 (新模式)
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
                "id": id,
                "name": name,
                "version": version,
                "author": author,
                "main_category": main_category,
                "category": category,
                "description": description,
                "default_severity": default_severity,
                "tags": tags,
                "enabled": enabled,
                "status": status,
            }));
        }

        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry")
            .fetch_one(pool)
            .await?;

        Ok(serde_json::json!({
            "items": plugins,
            "total": total_count,
            "page": page,
            "page_size": page_size
        }))
    }

    pub async fn toggle_plugin_favorite_internal(&self, plugin_id: &str, user_id: Option<&str>) -> Result<bool> {
        let pool = self.get_pool()?;
        let uid = user_id.unwrap_or("default_user");

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

    pub async fn get_favorited_plugins_internal(&self, user_id: Option<&str>) -> Result<Vec<String>> {
        let pool = self.get_pool()?;
        let uid = user_id.unwrap_or("default_user");

        let rows = sqlx::query("SELECT plugin_id FROM plugin_favorites WHERE user_id = $1")
            .bind(uid)
            .fetch_all(pool)
            .await?;

        Ok(rows.into_iter().map(|r| sqlx::Row::get(&r, "plugin_id")).collect())
    }

    pub async fn get_plugin_review_stats_internal(&self) -> Result<serde_json::Value> {
        let pool = self.get_pool()?;
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry").fetch_one(pool).await?;
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'active'").fetch_one(pool).await?;
        let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry WHERE status = 'pending'").fetch_one(pool).await?;

        Ok(serde_json::json!({
            "total": total,
            "active": active,
            "pending": pending
        }))
    }

    pub async fn update_plugin_enabled_internal(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE plugin_registry SET enabled = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(enabled)
            .bind(plugin_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_plugin_name_internal(&self, plugin_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM plugin_registry WHERE id = $1")
            .bind(plugin_id)
            .fetch_optional(pool)
            .await?;
        Ok(name)
    }

    pub async fn get_plugin_summary_internal(&self, plugin_id: &str) -> Result<Option<(String, bool)>> {
        let pool = self.get_pool()?;
        let row: Option<(String, bool)> = sqlx::query_as("SELECT main_category, enabled FROM plugin_registry WHERE id = $1")
            .bind(plugin_id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn get_plugin_tags_internal(&self, plugin_id: &str) -> Result<Vec<String>> {
        let pool = self.get_pool()?;
        let tags_json: Option<String> = sqlx::query_scalar("SELECT tags FROM plugin_registry WHERE id = $1")
            .bind(plugin_id)
            .fetch_optional(pool)
            .await?;
        
        let tags = tags_json
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Ok(tags)
    }
}
