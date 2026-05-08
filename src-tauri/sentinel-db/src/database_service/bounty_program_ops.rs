//! Bug bounty program and scope database operations.

use anyhow::Result;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_program, row_to_program_scope,
    timestamp_string_to_datetime, BountyProgramCompatRow, BountyProgramRow, BountyProgramStats,
    ProgramScopeRow,
};
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

// ============================================================================
// Database Operations
// ============================================================================

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Program CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty program
    pub async fn create_bounty_program(&self, program: &BountyProgramRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_programs (
                    id, name, organization, platform, platform_handle, url,
                    program_type, status, description, rewards_json,
                    response_sla_days, resolution_sla_days, rules_json, tags_json,
                    metadata_json, priority_score, total_submissions, accepted_submissions,
                    total_earnings, created_at, updated_at, last_activity_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
                        .bind(&program.id)
                        .bind(&program.name)
                        .bind(&program.organization)
                        .bind(&program.platform)
                        .bind(&program.platform_handle)
                        .bind(&program.url)
                        .bind(&program.program_type)
                        .bind(&program.status)
                        .bind(&program.description)
                        .bind(&program.rewards_json)
                        .bind(program.response_sla_days)
                        .bind(program.resolution_sla_days)
                        .bind(&program.rules_json)
                        .bind(&program.tags_json)
                        .bind(&program.metadata_json)
                        .bind(program.priority_score)
                        .bind(program.total_submissions)
                        .bind(program.accepted_submissions)
                        .bind(program.total_earnings)
                        .bind(&program.created_at)
                        .bind(&program.updated_at)
                        .bind(&program.last_activity_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
                        .bind(&program.id)
                        .bind(&program.name)
                        .bind(&program.organization)
                        .bind(&program.platform)
                        .bind(&program.platform_handle)
                        .bind(&program.url)
                        .bind(&program.program_type)
                        .bind(&program.status)
                        .bind(&program.description)
                        .bind(&program.rewards_json)
                        .bind(program.response_sla_days)
                        .bind(program.resolution_sla_days)
                        .bind(&program.rules_json)
                        .bind(&program.tags_json)
                        .bind(&program.metadata_json)
                        .bind(program.priority_score)
                        .bind(program.total_submissions)
                        .bind(program.accepted_submissions)
                        .bind(program.total_earnings)
                        .bind(&program.created_at)
                        .bind(&program.updated_at)
                        .bind(&program.last_activity_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty program: {}", program.id);
            return Ok(());
        }

        sqlx::query(
            r#"INSERT INTO bounty_programs (
                id, name, organization, platform, platform_handle, url,
                program_type, status, description, rewards_json,
                response_sla_days, resolution_sla_days, rules_json, tags_json,
                metadata_json, priority_score, total_submissions, accepted_submissions,
                total_earnings, created_at, updated_at, last_activity_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)"#
        )
        .bind(&program.id)
        .bind(&program.name)
        .bind(&program.organization)
        .bind(&program.platform)
        .bind(&program.platform_handle)
        .bind(&program.url)
        .bind(&program.program_type)
        .bind(&program.status)
        .bind(&program.description)
        .bind(&program.rewards_json)
        .bind(program.response_sla_days)
        .bind(program.resolution_sla_days)
        .bind(&program.rules_json)
        .bind(&program.tags_json)
        .bind(&program.metadata_json)
        .bind(program.priority_score)
        .bind(program.total_submissions)
        .bind(program.accepted_submissions)
        .bind(program.total_earnings)
        .bind(timestamp_string_to_datetime(&program.created_at))
        .bind(timestamp_string_to_datetime(&program.updated_at))
        .bind(optional_timestamp_string_to_datetime(&program.last_activity_at))
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty program: {}", program.id);
        Ok(())
    }

    /// Get a bounty program by ID
    pub async fn get_bounty_program(&self, id: &str) -> Result<Option<BountyProgramRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, name, organization, platform, platform_handle, url, program_type, status,
                    description, rewards_json, response_sla_days, resolution_sla_days, rules_json,
                    tags_json, metadata_json, priority_score, total_submissions, accepted_submissions,
                    total_earnings, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at,
                    CAST(last_activity_at AS TEXT) AS last_activity_at
                FROM bounty_programs
                WHERE id = ?
            "#;
            let row = match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query_as(query).bind(id).fetch_optional(pool).await?
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query_as(query).bind(id).fetch_optional(pool).await?
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            return Ok(row);
        }

        let row = sqlx::query("SELECT * FROM bounty_programs WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_program))
    }

    /// Update a bounty program
    pub async fn update_bounty_program(&self, program: &BountyProgramRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_programs SET
                    name = ?, organization = ?, platform = ?, platform_handle = ?,
                    url = ?, program_type = ?, status = ?, description = ?,
                    rewards_json = ?, response_sla_days = ?, resolution_sla_days = ?,
                    rules_json = ?, tags_json = ?, metadata_json = ?, priority_score = ?,
                    total_submissions = ?, accepted_submissions = ?, total_earnings = ?,
                    updated_at = ?, last_activity_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
                        .bind(&program.name)
                        .bind(&program.organization)
                        .bind(&program.platform)
                        .bind(&program.platform_handle)
                        .bind(&program.url)
                        .bind(&program.program_type)
                        .bind(&program.status)
                        .bind(&program.description)
                        .bind(&program.rewards_json)
                        .bind(program.response_sla_days)
                        .bind(program.resolution_sla_days)
                        .bind(&program.rules_json)
                        .bind(&program.tags_json)
                        .bind(&program.metadata_json)
                        .bind(program.priority_score)
                        .bind(program.total_submissions)
                        .bind(program.accepted_submissions)
                        .bind(program.total_earnings)
                        .bind(&program.updated_at)
                        .bind(&program.last_activity_at)
                        .bind(&program.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
                        .bind(&program.name)
                        .bind(&program.organization)
                        .bind(&program.platform)
                        .bind(&program.platform_handle)
                        .bind(&program.url)
                        .bind(&program.program_type)
                        .bind(&program.status)
                        .bind(&program.description)
                        .bind(&program.rewards_json)
                        .bind(program.response_sla_days)
                        .bind(program.resolution_sla_days)
                        .bind(&program.rules_json)
                        .bind(&program.tags_json)
                        .bind(&program.metadata_json)
                        .bind(program.priority_score)
                        .bind(program.total_submissions)
                        .bind(program.accepted_submissions)
                        .bind(program.total_earnings)
                        .bind(&program.updated_at)
                        .bind(&program.last_activity_at)
                        .bind(&program.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_programs SET
                name = $1, organization = $2, platform = $3, platform_handle = $4,
                url = $5, program_type = $6, status = $7, description = $8,
                rewards_json = $9, response_sla_days = $10, resolution_sla_days = $11,
                rules_json = $12, tags_json = $13, metadata_json = $14, priority_score = $15,
                total_submissions = $16, accepted_submissions = $17, total_earnings = $18,
                updated_at = $19, last_activity_at = $20
            WHERE id = $21"#,
        )
        .bind(&program.name)
        .bind(&program.organization)
        .bind(&program.platform)
        .bind(&program.platform_handle)
        .bind(&program.url)
        .bind(&program.program_type)
        .bind(&program.status)
        .bind(&program.description)
        .bind(&program.rewards_json)
        .bind(program.response_sla_days)
        .bind(program.resolution_sla_days)
        .bind(&program.rules_json)
        .bind(&program.tags_json)
        .bind(&program.metadata_json)
        .bind(program.priority_score)
        .bind(program.total_submissions)
        .bind(program.accepted_submissions)
        .bind(program.total_earnings)
        .bind(timestamp_string_to_datetime(&program.updated_at))
        .bind(optional_timestamp_string_to_datetime(
            &program.last_activity_at,
        ))
        .bind(&program.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty program
    pub async fn delete_bounty_program(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_programs WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_programs WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_programs WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List bounty programs with optional filtering
    pub async fn list_bounty_programs(
        &self,
        platforms: Option<&[String]>,
        statuses: Option<&[String]>,
        search: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountyProgramRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, name, organization, platform, platform_handle, url, program_type, status,
                    description, rewards_json, response_sla_days, resolution_sla_days, rules_json,
                    tags_json, metadata_json, priority_score, total_submissions, accepted_submissions,
                    total_earnings, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at,
                    CAST(last_activity_at AS TEXT) AS last_activity_at
                FROM bounty_programs
                ORDER BY priority_score DESC, updated_at DESC
            "#;

            let rows: Vec<BountyProgramCompatRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };

            let mut programs: Vec<BountyProgramRow> = rows.into_iter().map(Into::into).collect();

            if let Some(platforms) = platforms {
                if !platforms.is_empty() {
                    programs.retain(|p| platforms.contains(&p.platform));
                }
            }

            if let Some(statuses) = statuses {
                if !statuses.is_empty() {
                    programs.retain(|p| statuses.contains(&p.status));
                }
            }

            if let Some(search) = search {
                if !search.is_empty() {
                    let needle = search.to_lowercase();
                    programs.retain(|p| {
                        p.name.to_lowercase().contains(&needle)
                            || p.organization.to_lowercase().contains(&needle)
                    });
                }
            }

            if let Some(off) = offset {
                programs = programs.into_iter().skip(off as usize).collect();
            }
            if let Some(lim) = limit {
                programs.truncate(lim as usize);
            }

            return Ok(programs);
        }

        let mut query = String::from("SELECT * FROM bounty_programs WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(platforms) = platforms {
            if !platforms.is_empty() {
                let mut placeholders = Vec::new();
                for _ in platforms {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND platform IN ({})", placeholders.join(",")));
                params.extend(platforms.iter().cloned());
            }
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let mut placeholders = Vec::new();
                for _ in statuses {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
                params.extend(statuses.iter().cloned());
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                let p1 = params.len() + 1;
                let p2 = params.len() + 2;
                query.push_str(&format!(
                    " AND (name LIKE ${} OR organization LIKE ${})",
                    p1, p2
                ));
                let search_pattern = format!("%{}%", search);
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        query.push_str(" ORDER BY priority_score DESC, updated_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Build dynamic query
        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;
        Ok(rows.into_iter().map(row_to_bounty_program).collect())
    }

    /// Get bounty program statistics
    pub async fn get_bounty_program_stats(&self) -> Result<BountyProgramStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let total: (i64,) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs")
                    .fetch_one(pool)
                    .await?
            }
        };

        let active: (i64,) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs WHERE status = 'active'")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs WHERE status = 'active'")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_programs WHERE status = 'active'")
                    .fetch_one(pool)
                    .await?
            }
        };

        let totals: (i64, i64, f64) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as(
                    "SELECT COALESCE(SUM(total_submissions), 0), COALESCE(SUM(accepted_submissions), 0), COALESCE(SUM(total_earnings), 0.0) FROM bounty_programs"
                )
                .fetch_one(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as(
                    "SELECT COALESCE(SUM(total_submissions), 0), COALESCE(SUM(accepted_submissions), 0), COALESCE(SUM(total_earnings), 0.0) FROM bounty_programs"
                )
                .fetch_one(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as(
                    "SELECT COALESCE(SUM(total_submissions), 0), COALESCE(SUM(accepted_submissions), 0), COALESCE(SUM(total_earnings), 0.0) FROM bounty_programs"
                )
                .fetch_one(pool)
                .await?
            }
        };

        Ok(BountyProgramStats {
            total_programs: total.0 as i32,
            active_programs: active.0 as i32,
            total_submissions: totals.0 as i32,
            total_accepted: totals.1 as i32,
            total_earnings: totals.2,
        })
    }

    // ------------------------------------------------------------------------
    // Scope CRUD
    // ------------------------------------------------------------------------

    /// Create a new program scope
    pub async fn create_program_scope(&self, scope: &ProgramScopeRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_scopes (
                    id, program_id, scope_type, target_type, target, description,
                    allowed_tests_json, instructions_json, requires_auth, test_accounts_json,
                    asset_count, finding_count, priority, metadata_json, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
                        .bind(&scope.id)
                        .bind(&scope.program_id)
                        .bind(&scope.scope_type)
                        .bind(&scope.target_type)
                        .bind(&scope.target)
                        .bind(&scope.description)
                        .bind(&scope.allowed_tests_json)
                        .bind(&scope.instructions_json)
                        .bind(scope.requires_auth)
                        .bind(&scope.test_accounts_json)
                        .bind(scope.asset_count)
                        .bind(scope.finding_count)
                        .bind(scope.priority)
                        .bind(&scope.metadata_json)
                        .bind(&scope.created_at)
                        .bind(&scope.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
                        .bind(&scope.id)
                        .bind(&scope.program_id)
                        .bind(&scope.scope_type)
                        .bind(&scope.target_type)
                        .bind(&scope.target)
                        .bind(&scope.description)
                        .bind(&scope.allowed_tests_json)
                        .bind(&scope.instructions_json)
                        .bind(scope.requires_auth)
                        .bind(&scope.test_accounts_json)
                        .bind(scope.asset_count)
                        .bind(scope.finding_count)
                        .bind(scope.priority)
                        .bind(&scope.metadata_json)
                        .bind(&scope.created_at)
                        .bind(&scope.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created program scope: {}", scope.id);
            return Ok(());
        }

        sqlx::query(
            r#"INSERT INTO bounty_scopes (
                id, program_id, scope_type, target_type, target, description,
                allowed_tests_json, instructions_json, requires_auth, test_accounts_json,
                asset_count, finding_count, priority, metadata_json, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
        )
        .bind(&scope.id)
        .bind(&scope.program_id)
        .bind(&scope.scope_type)
        .bind(&scope.target_type)
        .bind(&scope.target)
        .bind(&scope.description)
        .bind(&scope.allowed_tests_json)
        .bind(&scope.instructions_json)
        .bind(scope.requires_auth)
        .bind(&scope.test_accounts_json)
        .bind(scope.asset_count)
        .bind(scope.finding_count)
        .bind(scope.priority)
        .bind(&scope.metadata_json)
        .bind(timestamp_string_to_datetime(&scope.created_at))
        .bind(timestamp_string_to_datetime(&scope.updated_at))
        .execute(self.get_pool()?)
        .await?;

        info!("Created program scope: {}", scope.id);
        Ok(())
    }

    /// Get a program scope by ID
    pub async fn get_program_scope(&self, id: &str) -> Result<Option<ProgramScopeRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, scope_type, target_type, target, description, allowed_tests_json,
                    instructions_json, requires_auth, test_accounts_json, asset_count, finding_count,
                    priority, metadata_json, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at
                FROM bounty_scopes
                WHERE id = ?
            "#;
            let row = match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query_as(query).bind(id).fetch_optional(pool).await?
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query_as(query).bind(id).fetch_optional(pool).await?
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            return Ok(row);
        }

        let row = sqlx::query("SELECT * FROM bounty_scopes WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_program_scope))
    }

    /// Update a program scope
    pub async fn update_program_scope(&self, scope: &ProgramScopeRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_scopes SET
                    scope_type = ?, target_type = ?, target = ?, description = ?,
                    allowed_tests_json = ?, instructions_json = ?, requires_auth = ?,
                    test_accounts_json = ?, asset_count = ?, finding_count = ?,
                    priority = ?, metadata_json = ?, updated_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
                        .bind(&scope.scope_type)
                        .bind(&scope.target_type)
                        .bind(&scope.target)
                        .bind(&scope.description)
                        .bind(&scope.allowed_tests_json)
                        .bind(&scope.instructions_json)
                        .bind(scope.requires_auth)
                        .bind(&scope.test_accounts_json)
                        .bind(scope.asset_count)
                        .bind(scope.finding_count)
                        .bind(scope.priority)
                        .bind(&scope.metadata_json)
                        .bind(&scope.updated_at)
                        .bind(&scope.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
                        .bind(&scope.scope_type)
                        .bind(&scope.target_type)
                        .bind(&scope.target)
                        .bind(&scope.description)
                        .bind(&scope.allowed_tests_json)
                        .bind(&scope.instructions_json)
                        .bind(scope.requires_auth)
                        .bind(&scope.test_accounts_json)
                        .bind(scope.asset_count)
                        .bind(scope.finding_count)
                        .bind(scope.priority)
                        .bind(&scope.metadata_json)
                        .bind(&scope.updated_at)
                        .bind(&scope.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_scopes SET
                scope_type = $1, target_type = $2, target = $3, description = $4,
                allowed_tests_json = $5, instructions_json = $6, requires_auth = $7,
                test_accounts_json = $8, asset_count = $9, finding_count = $10,
                priority = $11, metadata_json = $12, updated_at = $13
            WHERE id = $14"#,
        )
        .bind(&scope.scope_type)
        .bind(&scope.target_type)
        .bind(&scope.target)
        .bind(&scope.description)
        .bind(&scope.allowed_tests_json)
        .bind(&scope.instructions_json)
        .bind(scope.requires_auth)
        .bind(&scope.test_accounts_json)
        .bind(scope.asset_count)
        .bind(scope.finding_count)
        .bind(scope.priority)
        .bind(&scope.metadata_json)
        .bind(timestamp_string_to_datetime(&scope.updated_at))
        .bind(&scope.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a program scope
    pub async fn delete_program_scope(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_scopes WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_scopes WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_scopes WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List scopes for a program
    pub async fn list_program_scopes(
        &self,
        program_id: Option<&str>,
        scope_type: Option<&str>,
    ) -> Result<Vec<ProgramScopeRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, scope_type, target_type, target, description, allowed_tests_json,
                    instructions_json, requires_auth, test_accounts_json, asset_count, finding_count,
                    priority, metadata_json, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at
                FROM bounty_scopes
                ORDER BY priority DESC, created_at DESC
            "#;
            let mut scopes: Vec<ProgramScopeRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            if let Some(pid) = program_id {
                scopes.retain(|s| s.program_id == pid);
            }
            if let Some(st) = scope_type {
                scopes.retain(|s| s.scope_type == st);
            }
            return Ok(scopes);
        }

        let mut query = String::from("SELECT * FROM bounty_scopes WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }

        if let Some(st) = scope_type {
            params.push(st.to_string());
            query.push_str(&format!(" AND scope_type = ${}", params.len()));
        }

        query.push_str(" ORDER BY priority DESC, created_at DESC");

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;

        Ok(rows.into_iter().map(row_to_program_scope).collect())
    }
}
