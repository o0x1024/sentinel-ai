use anyhow::Result;
use sentinel_plugins::PluginRecord;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn get_plugins_from_registry_internal(&self) -> Result<Vec<PluginRecord>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT metadata FROM plugin_registry ORDER BY updated_at DESC")
            .fetch_all(pool)
            .await?;

        let mut plugins = Vec::with_capacity(rows.len());
        for row in rows {
            let metadata_json: String = sqlx::Row::get(&row, "metadata");
            if let Ok(metadata) = serde_json::from_str::<PluginRecord>(&metadata_json) {
                plugins.push(metadata);
            }
        }
        Ok(plugins)
    }

    pub async fn update_plugin_status_internal(&self, plugin_id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE plugin_registry SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
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
            "INSERT INTO plugin_registry (id, metadata, code, status, updated_at) VALUES (?, ?, ?, 'active', CURRENT_TIMESTAMP)
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
        let row = sqlx::query("SELECT metadata FROM plugin_registry WHERE id = ?")
            .bind(plugin_id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let metadata_json: String = sqlx::Row::get(&row, "metadata");
            Ok(Some(serde_json::from_str(&metadata_json)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_plugin_code_internal(&self, plugin_id: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let code: Option<String> = sqlx::query_scalar("SELECT code FROM plugin_registry WHERE id = ?")
            .bind(plugin_id)
            .fetch_optional(pool)
            .await?;
        Ok(code)
    }

    pub async fn delete_plugin_from_registry_internal(&self, plugin_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM plugin_registry WHERE id = ?")
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

        let mut query_str = "SELECT metadata FROM plugin_registry WHERE 1=1".to_string();
        if let Some(status) = status_filter {
            query_str.push_str(&format!(" AND status = '{}'", status));
        }
        if let Some(search) = search_text {
            query_str.push_str(&format!(" AND (id LIKE '%{}%' OR metadata LIKE '%{}%')", search, search));
        }
        query_str.push_str(&format!(" ORDER BY updated_at DESC LIMIT {} OFFSET {}", page_size, offset));

        let rows = sqlx::query(&query_str).fetch_all(pool).await?;

        let mut plugins = Vec::new();
        for row in rows {
            let metadata_json: String = sqlx::Row::get(&row, "metadata");
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                plugins.push(metadata);
            }
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

    pub async fn get_favorited_plugins_internal(&self, user_id: Option<&str>) -> Result<Vec<String>> {
        let pool = self.get_pool()?;
        let uid = user_id.unwrap_or("default_user");

        let rows = sqlx::query("SELECT plugin_id FROM plugin_favorites WHERE user_id = ?")
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
}
