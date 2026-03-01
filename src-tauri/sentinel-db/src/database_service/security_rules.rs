//! Database operations for CPG security rules (user-defined audit rules).

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};

// ============================================================================
// Data structures
// ============================================================================

/// A single security rule record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CpgSecurityRuleRecord {
    pub id: String,
    pub name: String,
    pub cwe: String,
    pub severity: String,
    pub description: String,
    /// JSON array of PatternSpec objects
    pub sources_json: String,
    /// JSON array of PatternSpec objects
    pub sinks_json: String,
    /// JSON array of PatternSpec objects
    pub sanitizers_json: String,
    /// Whether this is a built-in (seeded) rule
    pub is_builtin: bool,
    /// Whether the rule is enabled for scanning
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Filters for listing security rules
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpgSecurityRuleFilters {
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub is_builtin: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// DatabaseService impl
// ============================================================================

impl DatabaseService {
    /// Insert a new security rule
    pub async fn insert_cpg_security_rule(&self, rule: &CpgSecurityRuleRecord) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO cpg_security_rules (
                        id, name, cwe, severity, description,
                        sources_json, sinks_json, sanitizers_json,
                        is_builtin, enabled, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        cwe = excluded.cwe,
                        severity = excluded.severity,
                        description = excluded.description,
                        sources_json = excluded.sources_json,
                        sinks_json = excluded.sinks_json,
                        sanitizers_json = excluded.sanitizers_json,
                        is_builtin = excluded.is_builtin,
                        enabled = excluded.enabled,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&rule.id)
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.is_builtin)
                .bind(rule.enabled)
                .bind(rule.created_at)
                .bind(rule.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO cpg_security_rules (
                        id, name, cwe, severity, description,
                        sources_json, sinks_json, sanitizers_json,
                        is_builtin, enabled, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        cwe = excluded.cwe,
                        severity = excluded.severity,
                        description = excluded.description,
                        sources_json = excluded.sources_json,
                        sinks_json = excluded.sinks_json,
                        sanitizers_json = excluded.sanitizers_json,
                        is_builtin = excluded.is_builtin,
                        enabled = excluded.enabled,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&rule.id)
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.is_builtin)
                .bind(rule.enabled)
                .bind(rule.created_at)
                .bind(rule.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO cpg_security_rules (
                        id, name, cwe, severity, description,
                        sources_json, sinks_json, sanitizers_json,
                        is_builtin, enabled, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        name = VALUES(name),
                        cwe = VALUES(cwe),
                        severity = VALUES(severity),
                        description = VALUES(description),
                        sources_json = VALUES(sources_json),
                        sinks_json = VALUES(sinks_json),
                        sanitizers_json = VALUES(sanitizers_json),
                        is_builtin = VALUES(is_builtin),
                        enabled = VALUES(enabled),
                        updated_at = VALUES(updated_at)
                    "#,
                )
                .bind(&rule.id)
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.is_builtin)
                .bind(rule.enabled)
                .bind(rule.created_at)
                .bind(rule.updated_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Get a single rule by ID
    pub async fn get_cpg_security_rule(&self, id: &str) -> Result<Option<CpgSecurityRuleRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let record = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, CpgSecurityRuleRecord>(
                    "SELECT * FROM cpg_security_rules WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, CpgSecurityRuleRecord>(
                    "SELECT * FROM cpg_security_rules WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, CpgSecurityRuleRecord>(
                    "SELECT * FROM cpg_security_rules WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(record)
    }

    /// List rules with optional filters
    pub async fn list_cpg_security_rules(
        &self,
        filters: CpgSecurityRuleFilters,
    ) -> Result<Vec<CpgSecurityRuleRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = sqlx::QueryBuilder::<Postgres>::new(
                    "SELECT * FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                if let Some(ref s) = filters.search {
                    let like = format!("%{}%", s.to_lowercase());
                    qb.push(" AND (LOWER(name) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(cwe) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(description) LIKE ").push_bind(like);
                    qb.push(")");
                }
                qb.push(" ORDER BY is_builtin DESC, severity ASC, name ASC");
                if let Some(limit) = filters.limit {
                    qb.push(" LIMIT ").push_bind(limit);
                }
                if let Some(offset) = filters.offset {
                    qb.push(" OFFSET ").push_bind(offset);
                }
                Ok(qb
                    .build_query_as::<CpgSecurityRuleRecord>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::SQLite(pool) => {
                let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
                    "SELECT * FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                if let Some(ref s) = filters.search {
                    let like = format!("%{}%", s.to_lowercase());
                    qb.push(" AND (LOWER(name) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(cwe) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(description) LIKE ").push_bind(like);
                    qb.push(")");
                }
                qb.push(" ORDER BY is_builtin DESC, severity ASC, name ASC");
                if let Some(limit) = filters.limit {
                    qb.push(" LIMIT ").push_bind(limit);
                }
                if let Some(offset) = filters.offset {
                    qb.push(" OFFSET ").push_bind(offset);
                }
                Ok(qb
                    .build_query_as::<CpgSecurityRuleRecord>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = sqlx::QueryBuilder::<MySql>::new(
                    "SELECT * FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                if let Some(ref s) = filters.search {
                    let like = format!("%{}%", s.to_lowercase());
                    qb.push(" AND (LOWER(name) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(cwe) LIKE ").push_bind(like.clone());
                    qb.push(" OR LOWER(description) LIKE ").push_bind(like);
                    qb.push(")");
                }
                qb.push(" ORDER BY is_builtin DESC, severity ASC, name ASC");
                if let Some(limit) = filters.limit {
                    qb.push(" LIMIT ").push_bind(limit);
                }
                if let Some(offset) = filters.offset {
                    qb.push(" OFFSET ").push_bind(offset);
                }
                Ok(qb
                    .build_query_as::<CpgSecurityRuleRecord>()
                    .fetch_all(pool)
                    .await?)
            }
        }
    }

    /// Count rules with optional filters
    pub async fn count_cpg_security_rules(&self, filters: CpgSecurityRuleFilters) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let count: i64 = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = sqlx::QueryBuilder::<Postgres>::new(
                    "SELECT COUNT(*) FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                qb.build_query_scalar().fetch_one(pool).await?
            }
            DatabasePool::SQLite(pool) => {
                let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
                    "SELECT COUNT(*) FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                qb.build_query_scalar().fetch_one(pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = sqlx::QueryBuilder::<MySql>::new(
                    "SELECT COUNT(*) FROM cpg_security_rules WHERE 1=1",
                );
                if let Some(ref sev) = filters.severity {
                    qb.push(" AND severity = ").push_bind(sev);
                }
                if let Some(enabled) = filters.enabled {
                    qb.push(" AND enabled = ").push_bind(enabled);
                }
                if let Some(builtin) = filters.is_builtin {
                    qb.push(" AND is_builtin = ").push_bind(builtin);
                }
                qb.build_query_scalar().fetch_one(pool).await?
            }
        };

        Ok(count)
    }

    /// Update an existing rule
    pub async fn update_cpg_security_rule(&self, rule: &CpgSecurityRuleRecord) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE cpg_security_rules SET
                        name = $1, cwe = $2, severity = $3, description = $4,
                        sources_json = $5, sinks_json = $6, sanitizers_json = $7,
                        enabled = $8, updated_at = $9
                    WHERE id = $10
                    "#,
                )
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.enabled)
                .bind(rule.updated_at)
                .bind(&rule.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    UPDATE cpg_security_rules SET
                        name = ?, cwe = ?, severity = ?, description = ?,
                        sources_json = ?, sinks_json = ?, sanitizers_json = ?,
                        enabled = ?, updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.enabled)
                .bind(rule.updated_at)
                .bind(&rule.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE cpg_security_rules SET
                        name = ?, cwe = ?, severity = ?, description = ?,
                        sources_json = ?, sinks_json = ?, sanitizers_json = ?,
                        enabled = ?, updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(&rule.name)
                .bind(&rule.cwe)
                .bind(&rule.severity)
                .bind(&rule.description)
                .bind(&rule.sources_json)
                .bind(&rule.sinks_json)
                .bind(&rule.sanitizers_json)
                .bind(rule.enabled)
                .bind(rule.updated_at)
                .bind(&rule.id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Toggle rule enabled status
    pub async fn toggle_cpg_security_rule(&self, id: &str, enabled: bool) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let now = Utc::now();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE cpg_security_rules SET enabled = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(enabled)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE cpg_security_rules SET enabled = ?, updated_at = ? WHERE id = ?",
                )
                .bind(enabled)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE cpg_security_rules SET enabled = ?, updated_at = ? WHERE id = ?",
                )
                .bind(enabled)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Delete a rule by ID
    pub async fn delete_cpg_security_rule(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM cpg_security_rules WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM cpg_security_rules WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM cpg_security_rules WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Get all enabled rules (for engine consumption)
    pub async fn get_enabled_cpg_security_rules(&self) -> Result<Vec<CpgSecurityRuleRecord>> {
        self.list_cpg_security_rules(CpgSecurityRuleFilters {
            enabled: Some(true),
            ..Default::default()
        })
        .await
    }

    /// Check if rules table has any data
    pub async fn has_cpg_security_rules(&self) -> Result<bool> {
        let count = self
            .count_cpg_security_rules(CpgSecurityRuleFilters::default())
            .await?;
        Ok(count > 0)
    }
}
