use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use crate::database_service::service::DatabaseService;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RepeaterTab {
    pub id: String,
    pub name: String,
    pub target_host: String,
    pub target_port: i32,
    pub use_tls: bool,
    pub override_sni: bool,
    pub sni_host: Option<String>,
    pub raw_request: String,
    pub request_tab: String,
    pub response_tab: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepeaterTabRequest {
    pub id: String,
    pub name: String,
    pub target_host: String,
    pub target_port: i32,
    pub use_tls: bool,
    pub override_sni: bool,
    pub sni_host: Option<String>,
    pub raw_request: String,
    pub request_tab: String,
    pub response_tab: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRepeaterTabRequest {
    pub id: String,
    pub name: Option<String>,
    pub target_host: Option<String>,
    pub target_port: Option<i32>,
    pub use_tls: Option<bool>,
    pub override_sni: Option<bool>,
    pub sni_host: Option<String>,
    pub raw_request: Option<String>,
    pub request_tab: Option<String>,
    pub response_tab: Option<String>,
    pub sort_order: Option<i32>,
}

impl DatabaseService {
    /// Get all repeater tabs
    pub async fn get_repeater_tabs(&self) -> Result<Vec<RepeaterTab>> {
        let pool = self.get_pool()?;
        
        let rows = sqlx::query(
            r#"SELECT 
                id,
                name,
                target_host,
                target_port,
                use_tls,
                override_sni,
                sni_host,
                raw_request,
                request_tab,
                response_tab,
                sort_order,
                created_at,
                updated_at
            FROM repeater_tabs
            ORDER BY sort_order ASC, updated_at DESC"#
        )
        .fetch_all(pool)
        .await?;
        
        let tabs = rows.into_iter().map(|row| RepeaterTab {
            id: row.get("id"),
            name: row.get("name"),
            target_host: row.get("target_host"),
            target_port: row.get("target_port"),
            use_tls: row.get::<i32, _>("use_tls") != 0,
            override_sni: row.get::<i32, _>("override_sni") != 0,
            sni_host: row.get("sni_host"),
            raw_request: row.get("raw_request"),
            request_tab: row.get("request_tab"),
            response_tab: row.get("response_tab"),
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }).collect();
        
        Ok(tabs)
    }
    
    /// Get a single repeater tab by ID
    pub async fn get_repeater_tab(&self, id: &str) -> Result<Option<RepeaterTab>> {
        let pool = self.get_pool()?;
        
        let row = sqlx::query(
            r#"SELECT 
                id,
                name,
                target_host,
                target_port,
                use_tls,
                override_sni,
                sni_host,
                raw_request,
                request_tab,
                response_tab,
                sort_order,
                created_at,
                updated_at
            FROM repeater_tabs
            WHERE id = ?"#
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        
        let tab = row.map(|row| RepeaterTab {
            id: row.get("id"),
            name: row.get("name"),
            target_host: row.get("target_host"),
            target_port: row.get("target_port"),
            use_tls: row.get::<i32, _>("use_tls") != 0,
            override_sni: row.get::<i32, _>("override_sni") != 0,
            sni_host: row.get("sni_host"),
            raw_request: row.get("raw_request"),
            request_tab: row.get("request_tab"),
            response_tab: row.get("response_tab"),
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
        
        Ok(tab)
    }
    
    /// Create a new repeater tab
    pub async fn create_repeater_tab(&self, request: CreateRepeaterTabRequest) -> Result<()> {
        let pool = self.get_pool()?;
        let now = chrono::Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"INSERT INTO repeater_tabs (
                id, name, target_host, target_port, use_tls, override_sni, sni_host,
                raw_request, request_tab, response_tab, sort_order, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&request.id)
        .bind(&request.name)
        .bind(&request.target_host)
        .bind(request.target_port)
        .bind(if request.use_tls { 1 } else { 0 })
        .bind(if request.override_sni { 1 } else { 0 })
        .bind(&request.sni_host)
        .bind(&request.raw_request)
        .bind(&request.request_tab)
        .bind(&request.response_tab)
        .bind(request.sort_order)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Update a repeater tab
    pub async fn update_repeater_tab(&self, request: UpdateRepeaterTabRequest) -> Result<()> {
        let pool = self.get_pool()?;
        let now = chrono::Utc::now().to_rfc3339();
        
        // Build dynamic update query
        let mut query = String::from("UPDATE repeater_tabs SET updated_at = ?");
        let mut params: Vec<String> = vec![now.clone()];
        
        if let Some(name) = &request.name {
            query.push_str(", name = ?");
            params.push(name.clone());
        }
        if let Some(target_host) = &request.target_host {
            query.push_str(", target_host = ?");
            params.push(target_host.clone());
        }
        if let Some(target_port) = request.target_port {
            query.push_str(", target_port = ?");
            params.push(target_port.to_string());
        }
        if let Some(use_tls) = request.use_tls {
            query.push_str(", use_tls = ?");
            params.push(if use_tls { "1" } else { "0" }.to_string());
        }
        if let Some(override_sni) = request.override_sni {
            query.push_str(", override_sni = ?");
            params.push(if override_sni { "1" } else { "0" }.to_string());
        }
        if let Some(sni_host) = &request.sni_host {
            query.push_str(", sni_host = ?");
            params.push(sni_host.clone());
        }
        if let Some(raw_request) = &request.raw_request {
            query.push_str(", raw_request = ?");
            params.push(raw_request.clone());
        }
        if let Some(request_tab) = &request.request_tab {
            query.push_str(", request_tab = ?");
            params.push(request_tab.clone());
        }
        if let Some(response_tab) = &request.response_tab {
            query.push_str(", response_tab = ?");
            params.push(response_tab.clone());
        }
        if let Some(sort_order) = request.sort_order {
            query.push_str(", sort_order = ?");
            params.push(sort_order.to_string());
        }
        
        query.push_str(" WHERE id = ?");
        params.push(request.id.clone());
        
        // Execute the query
        let mut q = sqlx::query(&query);
        for param in params {
            q = q.bind(param);
        }
        q.execute(pool).await?;
        
        Ok(())
    }
    
    /// Delete a repeater tab
    pub async fn delete_repeater_tab(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    /// Delete all repeater tabs
    pub async fn delete_all_repeater_tabs(&self) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query("DELETE FROM repeater_tabs")
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    /// Delete multiple repeater tabs by IDs
    pub async fn delete_repeater_tabs(&self, ids: Vec<String>) -> Result<()> {
        let pool = self.get_pool()?;
        
        for id in ids {
            sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
                .bind(&id)
                .execute(pool)
                .await?;
        }
        
        Ok(())
    }
    
    /// Update sort orders for multiple tabs
    pub async fn update_repeater_tabs_sort_order(&self, tab_orders: Vec<(String, i32)>) -> Result<()> {
        let pool = self.get_pool()?;
        let now = chrono::Utc::now().to_rfc3339();
        
        for (id, sort_order) in tab_orders {
            sqlx::query("UPDATE repeater_tabs SET sort_order = ?, updated_at = ? WHERE id = ?")
                .bind(sort_order)
                .bind(&now)
                .bind(&id)
                .execute(pool)
                .await?;
        }
        
        Ok(())
    }
}
