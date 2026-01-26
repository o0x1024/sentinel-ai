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
            use_tls: row.get("use_tls"),
            override_sni: row.get("override_sni"),
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
            WHERE id = $1"#
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        
        let tab = row.map(|row| RepeaterTab {
            id: row.get("id"),
            name: row.get("name"),
            target_host: row.get("target_host"),
            target_port: row.get("target_port"),
            use_tls: row.get("use_tls"),
            override_sni: row.get("override_sni"),
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#
        )
        .bind(&request.id)
        .bind(&request.name)
        .bind(&request.target_host)
        .bind(request.target_port)
        .bind(request.use_tls)
        .bind(request.override_sni)
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
        let mut query = String::from("UPDATE repeater_tabs SET updated_at = $1");
        let mut idx = 2;
        
        if request.name.is_some() {
            query.push_str(&format!(", name = ${}", idx));
            idx += 1;
        }
        if request.target_host.is_some() {
            query.push_str(&format!(", target_host = ${}", idx));
            idx += 1;
        }
        if request.target_port.is_some() {
            query.push_str(&format!(", target_port = ${}", idx));
            idx += 1;
        }
        if request.use_tls.is_some() {
            query.push_str(&format!(", use_tls = ${}", idx));
            idx += 1;
        }
        if request.override_sni.is_some() {
            query.push_str(&format!(", override_sni = ${}", idx));
            idx += 1;
        }
        if request.sni_host.is_some() {
            query.push_str(&format!(", sni_host = ${}", idx));
            idx += 1;
        }
        if request.raw_request.is_some() {
            query.push_str(&format!(", raw_request = ${}", idx));
            idx += 1;
        }
        if request.request_tab.is_some() {
            query.push_str(&format!(", request_tab = ${}", idx));
            idx += 1;
        }
        if request.response_tab.is_some() {
            query.push_str(&format!(", response_tab = ${}", idx));
            idx += 1;
        }
        if request.sort_order.is_some() {
            query.push_str(&format!(", sort_order = ${}", idx));
            idx += 1;
        }
        
        query.push_str(&format!(" WHERE id = ${}", idx));
        
        // Bind parameters
        let mut q = sqlx::query(&query).bind(now);
        
        if let Some(val) = &request.name { q = q.bind(val); }
        if let Some(val) = &request.target_host { q = q.bind(val); }
        if let Some(val) = request.target_port { q = q.bind(val); }
        if let Some(val) = request.use_tls { q = q.bind(val); }
        if let Some(val) = request.override_sni { q = q.bind(val); }
        if let Some(val) = &request.sni_host { q = q.bind(val); }
        if let Some(val) = &request.raw_request { q = q.bind(val); }
        if let Some(val) = &request.request_tab { q = q.bind(val); }
        if let Some(val) = &request.response_tab { q = q.bind(val); }
        if let Some(val) = request.sort_order { q = q.bind(val); }
        
        q = q.bind(&request.id);
        
        q.execute(pool).await?;
        
        Ok(())
    }
    
    /// Delete a repeater tab
    pub async fn delete_repeater_tab(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        
        sqlx::query("DELETE FROM repeater_tabs WHERE id = $1")
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
            sqlx::query("DELETE FROM repeater_tabs WHERE id = $1")
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
            sqlx::query("UPDATE repeater_tabs SET sort_order = $1, updated_at = $2 WHERE id = $3")
                .bind(sort_order)
                .bind(&now)
                .bind(&id)
                .execute(pool)
                .await?;
        }
        
        Ok(())
    }
}
