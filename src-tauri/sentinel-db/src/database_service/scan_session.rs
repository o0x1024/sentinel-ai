use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::core::models::scan_session::{
    ScanSession, ScanStage, ScanProgress, CreateScanSessionRequest, UpdateScanSessionRequest,
    ScanSessionStatus, ScanStageStatus, ScanStageProgress,
};
use uuid::Uuid;

impl DatabaseService {
    pub async fn create_scan_session_internal(&self, request: CreateScanSessionRequest) -> Result<ScanSession> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let session = ScanSession::new(
            request.name,
            request.target,
            request.scan_type,
            request.config,
            request.created_by,
        );

        let query = r#"
            INSERT INTO scan_sessions (
                id, name, description, target, scan_type, status, config, 
                progress, current_stage, total_stages, completed_stages,
                created_at, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(query)
                    .bind(session.id.to_string())
                    .bind(&session.name)
                    .bind(&session.description)
                    .bind(&session.target)
                    .bind(&session.scan_type)
                    .bind(serde_json::to_string(&session.status)?)
                    .bind(serde_json::to_string(&session.config)?)
                    .bind(session.progress)
                    .bind(&session.current_stage)
                    .bind(session.total_stages)
                    .bind(session.completed_stages)
                    .bind(session.created_at)
                    .bind(&session.created_by)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                let query = r#"
                    INSERT INTO scan_sessions (
                        id, name, description, target, scan_type, status, config, 
                        progress, current_stage, total_stages, completed_stages,
                        created_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#;
                sqlx::query(query)
                    .bind(session.id.to_string())
                    .bind(&session.name)
                    .bind(&session.description)
                    .bind(&session.target)
                    .bind(&session.scan_type)
                    .bind(serde_json::to_string(&session.status)?)
                    .bind(serde_json::to_string(&session.config)?)
                    .bind(session.progress)
                    .bind(&session.current_stage)
                    .bind(session.total_stages)
                    .bind(session.completed_stages)
                    .bind(session.created_at)
                    .bind(&session.created_by)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                let query = r#"
                    INSERT INTO scan_sessions (
                        id, name, description, target, scan_type, status, config, 
                        progress, current_stage, total_stages, completed_stages,
                        created_at, created_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#;
                sqlx::query(query)
                    .bind(session.id.to_string())
                    .bind(&session.name)
                    .bind(&session.description)
                    .bind(&session.target)
                    .bind(&session.scan_type)
                    .bind(serde_json::to_string(&session.status)?)
                    .bind(serde_json::to_string(&session.config)?)
                    .bind(session.progress)
                    .bind(&session.current_stage)
                    .bind(session.total_stages)
                    .bind(session.completed_stages)
                    .bind(session.created_at)
                    .bind(&session.created_by)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(session)
    }

    pub async fn get_scan_session_internal(&self, session_id: Uuid) -> Result<Option<ScanSession>> {
        let rows = self
            .execute_query(&format!(
                r#"SELECT id, name, description, target, scan_type, status, config,
                          progress, current_stage, total_stages, completed_stages,
                          results_summary, error_message, created_at, started_at,
                          completed_at, created_by
                   FROM scan_sessions WHERE id = '{}'"#,
                session_id
            ))
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(parse_scan_session_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn update_scan_session_internal(
        &self,
        session_id: Uuid,
        request: UpdateScanSessionRequest,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let status_json = request
            .status
            .map(|s| serde_json::to_string(&s))
            .transpose()?;
        let results_summary_json = request
            .results_summary
            .map(|s| serde_json::to_string(&s))
            .transpose()?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"UPDATE scan_sessions SET
                           updated_at = CURRENT_TIMESTAMP,
                           name = COALESCE($2, name),
                           description = COALESCE($3, description),
                           status = COALESCE($4, status),
                           progress = COALESCE($5, progress),
                           current_stage = COALESCE($6, current_stage),
                           total_stages = COALESCE($7, total_stages),
                           completed_stages = COALESCE($8, completed_stages),
                           results_summary = COALESCE($9, results_summary),
                           error_message = COALESCE($10, error_message)
                       WHERE id = $1"#,
                )
                .bind(session_id.to_string())
                .bind(request.name)
                .bind(request.description)
                .bind(status_json)
                .bind(request.progress)
                .bind(request.current_stage)
                .bind(request.total_stages)
                .bind(request.completed_stages)
                .bind(results_summary_json)
                .bind(request.error_message)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                let name = request.name.clone();
                let description = request.description.clone();
                let status_json_for_sqlite = status_json.clone();
                let progress = request.progress;
                let current_stage = request.current_stage.clone();
                let total_stages = request.total_stages;
                let completed_stages = request.completed_stages;
                let results_summary_for_sqlite = results_summary_json.clone();
                let error_message = request.error_message.clone();
                let id = session_id.to_string();

                let update_with_updated_at = sqlx::query(
                    r#"UPDATE scan_sessions SET
                           updated_at = CURRENT_TIMESTAMP,
                           name = COALESCE(?, name),
                           description = COALESCE(?, description),
                           status = COALESCE(?, status),
                           progress = COALESCE(?, progress),
                           current_stage = COALESCE(?, current_stage),
                           total_stages = COALESCE(?, total_stages),
                           completed_stages = COALESCE(?, completed_stages),
                           results_summary = COALESCE(?, results_summary),
                           error_message = COALESCE(?, error_message)
                       WHERE id = ?"#,
                )
                .bind(name.clone())
                .bind(description.clone())
                .bind(status_json_for_sqlite.clone())
                .bind(progress)
                .bind(current_stage.clone())
                .bind(total_stages)
                .bind(completed_stages)
                .bind(results_summary_for_sqlite.clone())
                .bind(error_message.clone())
                .bind(id.clone());

                if let Err(e) = update_with_updated_at.execute(pool).await {
                    let err_text = e.to_string().to_lowercase();
                    if err_text.contains("no such column") && err_text.contains("updated_at") {
                        // Backward-compatible fallback for old SQLite schema without `updated_at`.
                        sqlx::query(
                            r#"UPDATE scan_sessions SET
                                   name = COALESCE(?, name),
                                   description = COALESCE(?, description),
                                   status = COALESCE(?, status),
                                   progress = COALESCE(?, progress),
                                   current_stage = COALESCE(?, current_stage),
                                   total_stages = COALESCE(?, total_stages),
                                   completed_stages = COALESCE(?, completed_stages),
                                   results_summary = COALESCE(?, results_summary),
                                   error_message = COALESCE(?, error_message)
                               WHERE id = ?"#,
                        )
                        .bind(name)
                        .bind(description)
                        .bind(status_json_for_sqlite)
                        .bind(progress)
                        .bind(current_stage)
                        .bind(total_stages)
                        .bind(completed_stages)
                        .bind(results_summary_for_sqlite)
                        .bind(error_message)
                        .bind(id)
                        .execute(pool)
                        .await?;
                    } else {
                        return Err(e.into());
                    }
                }
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"UPDATE scan_sessions SET
                           updated_at = CURRENT_TIMESTAMP,
                           name = COALESCE(?, name),
                           description = COALESCE(?, description),
                           status = COALESCE(?, status),
                           progress = COALESCE(?, progress),
                           current_stage = COALESCE(?, current_stage),
                           total_stages = COALESCE(?, total_stages),
                           completed_stages = COALESCE(?, completed_stages),
                           results_summary = COALESCE(?, results_summary),
                           error_message = COALESCE(?, error_message)
                       WHERE id = ?"#,
                )
                .bind(request.name)
                .bind(request.description)
                .bind(status_json)
                .bind(request.progress)
                .bind(request.current_stage)
                .bind(request.total_stages)
                .bind(request.completed_stages)
                .bind(results_summary_json)
                .bind(request.error_message)
                .bind(session_id.to_string())
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_scan_sessions_internal(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        status_filter: Option<ScanSessionStatus>,
    ) -> Result<Vec<ScanSession>> {
        let mut query = String::from(
            r#"SELECT id, name, description, target, scan_type, status, config,
                      progress, current_stage, total_stages, completed_stages,
                      results_summary, error_message, created_at, started_at,
                      completed_at, created_by
               FROM scan_sessions"#,
        );
        if let Some(status) = status_filter {
            let status_json = serde_json::to_string(&status)?;
            query.push_str(&format!(
                " WHERE status = '{}'",
                status_json.replace('\'', "''")
            ));
        }
        query.push_str(" ORDER BY created_at DESC");
        if let Some(l) = limit {
            query.push_str(&format!(" LIMIT {}", l.max(0)));
        }
        if let Some(o) = offset {
            query.push_str(&format!(" OFFSET {}", o.max(0)));
        }

        let rows = self.execute_query(&query).await?;
        rows.into_iter().map(|r| parse_scan_session_row(&r)).collect()
    }

    pub async fn delete_scan_session_internal(&self, session_id: Uuid) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = session_id.to_string();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM scan_stages WHERE session_id = $1")
                    .bind(&id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM scan_sessions WHERE id = $1")
                    .bind(&id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM scan_stages WHERE session_id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM scan_sessions WHERE id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM scan_stages WHERE session_id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM scan_sessions WHERE id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn create_scan_stage_internal(&self, stage: ScanStage) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let query = r#"
            INSERT INTO scan_stages (
                id, session_id, stage_name, stage_order, status, tool_name,
                config, started_at, completed_at, duration_ms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(query)
                    .bind(stage.id.to_string())
                    .bind(stage.session_id.to_string())
                    .bind(&stage.stage_name)
                    .bind(stage.stage_order)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(&stage.tool_name)
                    .bind(serde_json::to_string(&stage.config)?)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                let query = r#"
                    INSERT INTO scan_stages (
                        id, session_id, stage_name, stage_order, status, tool_name,
                        config, started_at, completed_at, duration_ms
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#;
                sqlx::query(query)
                    .bind(stage.id.to_string())
                    .bind(stage.session_id.to_string())
                    .bind(&stage.stage_name)
                    .bind(stage.stage_order)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(&stage.tool_name)
                    .bind(serde_json::to_string(&stage.config)?)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                let query = r#"
                    INSERT INTO scan_stages (
                        id, session_id, stage_name, stage_order, status, tool_name,
                        config, started_at, completed_at, duration_ms
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#;
                sqlx::query(query)
                    .bind(stage.id.to_string())
                    .bind(stage.session_id.to_string())
                    .bind(&stage.stage_name)
                    .bind(stage.stage_order)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(&stage.tool_name)
                    .bind(serde_json::to_string(&stage.config)?)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn update_scan_stage_internal(&self, stage: &ScanStage) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let query = r#"
            UPDATE scan_stages SET
                status = $1, results = $2, error_message = $3,
                started_at = $4, completed_at = $5, duration_ms = $6
            WHERE id = $7
        "#;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(query)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(
                        stage
                            .results
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?,
                    )
                    .bind(&stage.error_message)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .bind(stage.id.to_string())
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                let query = r#"
                    UPDATE scan_stages SET
                        status = ?, results = ?, error_message = ?,
                        started_at = ?, completed_at = ?, duration_ms = ?
                    WHERE id = ?
                "#;
                sqlx::query(query)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(
                        stage
                            .results
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?,
                    )
                    .bind(&stage.error_message)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .bind(stage.id.to_string())
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                let query = r#"
                    UPDATE scan_stages SET
                        status = ?, results = ?, error_message = ?,
                        started_at = ?, completed_at = ?, duration_ms = ?
                    WHERE id = ?
                "#;
                sqlx::query(query)
                    .bind(serde_json::to_string(&stage.status)?)
                    .bind(
                        stage
                            .results
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?,
                    )
                    .bind(&stage.error_message)
                    .bind(stage.started_at)
                    .bind(stage.completed_at)
                    .bind(stage.duration_ms)
                    .bind(stage.id.to_string())
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_scan_session_stages_internal(&self, session_id: Uuid) -> Result<Vec<ScanStage>> {
        let rows = self
            .execute_query(&format!(
                r#"SELECT id, session_id, stage_name, stage_order, status, tool_name,
                          config, results, error_message, started_at, completed_at, duration_ms
                   FROM scan_stages WHERE session_id = '{}' ORDER BY stage_order"#,
                session_id
            ))
            .await?;
        rows.into_iter().map(|r| parse_scan_stage_row(&r)).collect()
    }

    pub async fn get_scan_progress_internal(&self, session_id: Uuid) -> Result<Option<ScanProgress>> {
        let session = self.get_scan_session_internal(session_id).await?;
        if let Some(session) = session {
            let stages = self.get_scan_session_stages_internal(session_id).await?;

            let stage_progress: Vec<ScanStageProgress> = stages
                .iter()
                .map(|stage| ScanStageProgress {
                    stage_name: stage.stage_name.clone(),
                    status: stage.status.clone(),
                    progress: match stage.status {
                        ScanStageStatus::Completed => 100.0,
                        ScanStageStatus::Running => 50.0, // 估算值
                        _ => 0.0,
                    },
                    started_at: stage.started_at,
                    estimated_completion: None, // TODO: 实现时间估算
                })
                .collect();

            let progress = ScanProgress {
                session_id,
                overall_progress: session.progress,
                current_stage: session.current_stage,
                completed_stages: session.completed_stages,
                total_stages: session.total_stages,
                stages: stage_progress,
                estimated_time_remaining: None, // TODO: 实现时间估算
            };

            Ok(Some(progress))
        } else {
            Ok(None)
        }
    }
}

fn parse_scan_session_row(row: &Value) -> Result<ScanSession> {
    let obj = row
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("invalid scan session row"))?;
    let created_at = parse_datetime_required(obj.get("created_at"))?;
    let started_at = parse_datetime_optional(obj.get("started_at"));
    let completed_at = parse_datetime_optional(obj.get("completed_at"));
    let status = obj
        .get("status")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<ScanSessionStatus>(s).ok())
        .unwrap_or_default();
    let config = obj
        .get("config")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let results_summary = obj
        .get("results_summary")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok());

    Ok(ScanSession {
        id: Uuid::parse_str(obj.get("id").and_then(|v| v.as_str()).unwrap_or_default())?,
        name: obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        description: obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        target: obj
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        scan_type: obj
            .get("scan_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        status,
        config,
        progress: obj.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0),
        current_stage: obj
            .get("current_stage")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        total_stages: obj
            .get("total_stages")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
        completed_stages: obj
            .get("completed_stages")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
        results_summary,
        error_message: obj
            .get("error_message")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        created_at,
        started_at,
        completed_at,
        created_by: obj
            .get("created_by")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
    })
}

fn parse_scan_stage_row(row: &Value) -> Result<ScanStage> {
    let obj = row
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("invalid scan stage row"))?;
    let status = obj
        .get("status")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<ScanStageStatus>(s).ok())
        .unwrap_or_default();
    let config = obj
        .get("config")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let results = obj
        .get("results")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok());

    Ok(ScanStage {
        id: Uuid::parse_str(obj.get("id").and_then(|v| v.as_str()).unwrap_or_default())?,
        session_id: Uuid::parse_str(
            obj.get("session_id").and_then(|v| v.as_str()).unwrap_or_default(),
        )?,
        stage_name: obj
            .get("stage_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        stage_order: obj.get("stage_order").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        status,
        tool_name: obj
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        config,
        results,
        error_message: obj
            .get("error_message")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        started_at: parse_datetime_optional(obj.get("started_at")),
        completed_at: parse_datetime_optional(obj.get("completed_at")),
        duration_ms: obj.get("duration_ms").and_then(|v| v.as_i64()),
    })
}

fn parse_datetime_required(v: Option<&Value>) -> Result<DateTime<Utc>> {
    let s = v
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing datetime"))?;
    parse_datetime(s).ok_or_else(|| anyhow::anyhow!("invalid datetime: {}", s))
}

fn parse_datetime_optional(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(|x| x.as_str()).and_then(parse_datetime)
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
                .ok()
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
}
