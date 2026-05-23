use std::collections::HashSet;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiInventoryEndpointRequestRow {
    pub id: String,
    pub program_id: String,
    pub base_url: String,
    pub endpoint_path: String,
    pub endpoint_source: Option<String>,
    pub method: String,
    pub request_url: String,
    pub success: bool,
    pub status_code: Option<i32>,
    pub status_text: Option<String>,
    pub duration_ms: i64,
    pub response_bytes: i64,
    pub response_content_type: Option<String>,
    pub body_preview: Option<String>,
    pub error_message: Option<String>,
    pub request_body: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DatabaseService {
    pub async fn create_api_inventory_endpoint_request(
        &self,
        row: &ApiInventoryEndpointRequestRow,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO api_inventory_endpoint_requests (
                        id, program_id, base_url, endpoint_path, endpoint_source, method,
                        request_url, success, status_code, status_text, duration_ms,
                        response_bytes, response_content_type, body_preview, error_message,
                        request_body, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&row.id)
                .bind(&row.program_id)
                .bind(&row.base_url)
                .bind(&row.endpoint_path)
                .bind(&row.endpoint_source)
                .bind(&row.method)
                .bind(&row.request_url)
                .bind(row.success)
                .bind(row.status_code)
                .bind(&row.status_text)
                .bind(row.duration_ms)
                .bind(row.response_bytes)
                .bind(&row.response_content_type)
                .bind(&row.body_preview)
                .bind(&row.error_message)
                .bind(&row.request_body)
                .bind(row.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO api_inventory_endpoint_requests (
                        id, program_id, base_url, endpoint_path, endpoint_source, method,
                        request_url, success, status_code, status_text, duration_ms,
                        response_bytes, response_content_type, body_preview, error_message,
                        request_body, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&row.id)
                .bind(&row.program_id)
                .bind(&row.base_url)
                .bind(&row.endpoint_path)
                .bind(&row.endpoint_source)
                .bind(&row.method)
                .bind(&row.request_url)
                .bind(row.success)
                .bind(row.status_code)
                .bind(&row.status_text)
                .bind(row.duration_ms)
                .bind(row.response_bytes)
                .bind(&row.response_content_type)
                .bind(&row.body_preview)
                .bind(&row.error_message)
                .bind(&row.request_body)
                .bind(row.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO api_inventory_endpoint_requests (
                        id, program_id, base_url, endpoint_path, endpoint_source, method,
                        request_url, success, status_code, status_text, duration_ms,
                        response_bytes, response_content_type, body_preview, error_message,
                        request_body, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)"#,
                )
                .bind(&row.id)
                .bind(&row.program_id)
                .bind(&row.base_url)
                .bind(&row.endpoint_path)
                .bind(&row.endpoint_source)
                .bind(&row.method)
                .bind(&row.request_url)
                .bind(row.success)
                .bind(row.status_code)
                .bind(&row.status_text)
                .bind(row.duration_ms)
                .bind(row.response_bytes)
                .bind(&row.response_content_type)
                .bind(&row.body_preview)
                .bind(&row.error_message)
                .bind(&row.request_body)
                .bind(row.created_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_latest_api_inventory_endpoint_requests(
        &self,
        program_id: &str,
        base_url: &str,
        method: &str,
        endpoint_paths: Option<&[String]>,
        limit: Option<i64>,
    ) -> Result<Vec<ApiInventoryEndpointRequestRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let safe_limit = limit.map(|value| value.max(0));
        let base_sql = "SELECT * FROM api_inventory_endpoint_requests WHERE program_id = ";
        macro_rules! push_filters {
            ($builder:expr) => {{
                $builder
                    .push_bind(program_id)
                    .push(" AND base_url = ")
                    .push_bind(base_url)
                    .push(" AND method = ")
                    .push_bind(method);
                if let Some(paths) = endpoint_paths {
                    if !paths.is_empty() {
                        $builder.push(" AND endpoint_path IN (");
                        let mut separated = $builder.separated(", ");
                        for path in paths {
                            separated.push_bind(path);
                        }
                        separated.push_unseparated(")");
                    }
                }
                $builder.push(" ORDER BY created_at DESC, id DESC");
                if let Some(limit) = safe_limit {
                    $builder.push(" LIMIT ").push_bind(limit);
                }
            }};
        }

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(base_sql);
                push_filters!(query_builder);
                query_builder
                    .build_query_as::<ApiInventoryEndpointRequestRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder = QueryBuilder::<MySql>::new(base_sql);
                push_filters!(query_builder);
                query_builder
                    .build_query_as::<ApiInventoryEndpointRequestRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder = QueryBuilder::<Postgres>::new(base_sql);
                push_filters!(query_builder);
                query_builder
                    .build_query_as::<ApiInventoryEndpointRequestRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        let mut seen = HashSet::new();
        let mut latest = Vec::new();
        for row in rows {
            if seen.insert(row.endpoint_path.clone()) {
                latest.push(row);
            }
        }

        Ok(latest)
    }
}
