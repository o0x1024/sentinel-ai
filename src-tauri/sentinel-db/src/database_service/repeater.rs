use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use crate::database_service::connection_manager::DatabasePool;
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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let tabs = match runtime {
            DatabasePool::PostgreSQL(pool) => {
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
                    ORDER BY sort_order ASC, updated_at DESC"#,
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter()
                    .map(|row| RepeaterTab {
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
                    })
                    .collect()
            }
            DatabasePool::SQLite(pool) => {
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
                    ORDER BY sort_order ASC, updated_at DESC"#,
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter()
                    .map(|row| RepeaterTab {
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
                    })
                    .collect()
            }
            DatabasePool::MySQL(pool) => {
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
                    ORDER BY sort_order ASC, updated_at DESC"#,
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter()
                    .map(|row| RepeaterTab {
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
                    })
                    .collect()
            }
        };

        Ok(tabs)
    }
    
    /// Get a single repeater tab by ID
    pub async fn get_repeater_tab(&self, id: &str) -> Result<Option<RepeaterTab>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let tab = match runtime {
            DatabasePool::PostgreSQL(pool) => {
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
                    WHERE id = $1"#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|row| RepeaterTab {
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
                })
            }
            DatabasePool::SQLite(pool) => {
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
                    WHERE id = ?"#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|row| RepeaterTab {
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
                })
            }
            DatabasePool::MySQL(pool) => {
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
                    WHERE id = ?"#,
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|row| RepeaterTab {
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
                })
            }
        };
        
        Ok(tab)
    }
    
    /// Create a new repeater tab
    pub async fn create_repeater_tab(&self, request: CreateRepeaterTabRequest) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = chrono::Utc::now().to_rfc3339();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO repeater_tabs (
                        id, name, target_host, target_port, use_tls, override_sni, sni_host,
                        raw_request, request_tab, response_tab, sort_order, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
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
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO repeater_tabs (
                        id, name, target_host, target_port, use_tls, override_sni, sni_host,
                        raw_request, request_tab, response_tab, sort_order, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO repeater_tabs (
                        id, name, target_host, target_port, use_tls, override_sni, sni_host,
                        raw_request, request_tab, response_tab, sort_order, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
            }
        }
        
        Ok(())
    }
    
    /// Update a repeater tab
    pub async fn update_repeater_tab(&self, request: UpdateRepeaterTabRequest) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = chrono::Utc::now().to_rfc3339();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"UPDATE repeater_tabs SET
                           updated_at = $1,
                           name = COALESCE($2, name),
                           target_host = COALESCE($3, target_host),
                           target_port = COALESCE($4, target_port),
                           use_tls = COALESCE($5, use_tls),
                           override_sni = COALESCE($6, override_sni),
                           sni_host = COALESCE($7, sni_host),
                           raw_request = COALESCE($8, raw_request),
                           request_tab = COALESCE($9, request_tab),
                           response_tab = COALESCE($10, response_tab),
                           sort_order = COALESCE($11, sort_order)
                       WHERE id = $12"#,
                )
                .bind(&now)
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
                .bind(&request.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"UPDATE repeater_tabs SET
                           updated_at = ?,
                           name = COALESCE(?, name),
                           target_host = COALESCE(?, target_host),
                           target_port = COALESCE(?, target_port),
                           use_tls = COALESCE(?, use_tls),
                           override_sni = COALESCE(?, override_sni),
                           sni_host = COALESCE(?, sni_host),
                           raw_request = COALESCE(?, raw_request),
                           request_tab = COALESCE(?, request_tab),
                           response_tab = COALESCE(?, response_tab),
                           sort_order = COALESCE(?, sort_order)
                       WHERE id = ?"#,
                )
                .bind(&now)
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
                .bind(&request.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"UPDATE repeater_tabs SET
                           updated_at = ?,
                           name = COALESCE(?, name),
                           target_host = COALESCE(?, target_host),
                           target_port = COALESCE(?, target_port),
                           use_tls = COALESCE(?, use_tls),
                           override_sni = COALESCE(?, override_sni),
                           sni_host = COALESCE(?, sni_host),
                           raw_request = COALESCE(?, raw_request),
                           request_tab = COALESCE(?, request_tab),
                           response_tab = COALESCE(?, response_tab),
                           sort_order = COALESCE(?, sort_order)
                       WHERE id = ?"#,
                )
                .bind(&now)
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
                .bind(&request.id)
                .execute(pool)
                .await?;
            }
        }
        
        Ok(())
    }
    
    /// Delete a repeater tab
    pub async fn delete_repeater_tab(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM repeater_tabs WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Delete all repeater tabs
    pub async fn delete_all_repeater_tabs(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM repeater_tabs")
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM repeater_tabs")
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM repeater_tabs")
                    .execute(pool)
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Delete multiple repeater tabs by IDs
    pub async fn delete_repeater_tabs(&self, ids: Vec<String>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                for id in ids {
                    sqlx::query("DELETE FROM repeater_tabs WHERE id = $1")
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                for id in ids {
                    sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                for id in ids {
                    sqlx::query("DELETE FROM repeater_tabs WHERE id = ?")
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Update sort orders for multiple tabs
    pub async fn update_repeater_tabs_sort_order(&self, tab_orders: Vec<(String, i32)>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = chrono::Utc::now().to_rfc3339();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                for (id, sort_order) in tab_orders {
                    sqlx::query("UPDATE repeater_tabs SET sort_order = $1, updated_at = $2 WHERE id = $3")
                        .bind(sort_order)
                        .bind(&now)
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                for (id, sort_order) in tab_orders {
                    sqlx::query("UPDATE repeater_tabs SET sort_order = ?, updated_at = ? WHERE id = ?")
                        .bind(sort_order)
                        .bind(&now)
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                for (id, sort_order) in tab_orders {
                    sqlx::query("UPDATE repeater_tabs SET sort_order = ?, updated_at = ? WHERE id = ?")
                        .bind(sort_order)
                        .bind(&now)
                        .bind(&id)
                        .execute(pool)
                        .await?;
                }
            }
        }
        
        Ok(())
    }
}
