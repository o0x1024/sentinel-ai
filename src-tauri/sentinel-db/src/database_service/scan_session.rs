use anyhow::Result;
use sqlx::Row;
use crate::database_service::service::DatabaseService;
use crate::core::models::scan_session::{
    ScanSession, ScanStage, ScanProgress, CreateScanSessionRequest, UpdateScanSessionRequest,
    ScanSessionStatus, ScanStageStatus, ScanStageProgress,
};
use uuid::Uuid;

impl DatabaseService {
    pub async fn create_scan_session_internal(&self, request: CreateScanSessionRequest) -> Result<ScanSession> {
        let pool = self.get_pool()?;
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

        Ok(session)
    }

    pub async fn get_scan_session_internal(&self, session_id: Uuid) -> Result<Option<ScanSession>> {
        let pool = self.get_pool()?;
        let query = r#"
            SELECT id, name, description, target, scan_type, status, config,
                   progress, current_stage, total_stages, completed_stages,
                   results_summary, error_message, created_at, started_at,
                   completed_at, created_by
            FROM scan_sessions WHERE id = $1
        "#;

        let row = sqlx::query(query)
            .bind(session_id.to_string())
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let session = ScanSession {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                name: row.get("name"),
                description: row.get("description"),
                target: row.get("target"),
                scan_type: row.get("scan_type"),
                status: serde_json::from_str(&row.get::<String, _>("status")).unwrap_or_default(),
                config: serde_json::from_str(&row.get::<String, _>("config")).unwrap_or_default(),
                progress: row.get("progress"),
                current_stage: row.get("current_stage"),
                total_stages: row.get("total_stages"),
                completed_stages: row.get("completed_stages"),
                results_summary: row
                    .get::<Option<String>, _>("results_summary")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                created_by: row.get("created_by"),
            };
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub async fn update_scan_session_internal(
        &self,
        session_id: Uuid,
        request: UpdateScanSessionRequest,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let _query = "UPDATE scan_sessions SET updated_at = CURRENT_TIMESTAMP".to_string();
        let _idx = 1;
        let mut query = String::from("UPDATE scan_sessions SET updated_at = CURRENT_TIMESTAMP");
        let mut params_count = 1; // session_id is $1
        
        if request.name.is_some() { params_count += 1; query.push_str(&format!(", name = ${}", params_count)); }
        if request.description.is_some() { params_count += 1; query.push_str(&format!(", description = ${}", params_count)); }
        if request.status.is_some() { params_count += 1; query.push_str(&format!(", status = ${}", params_count)); }
        if request.progress.is_some() { params_count += 1; query.push_str(&format!(", progress = ${}", params_count)); }
        if request.current_stage.is_some() { params_count += 1; query.push_str(&format!(", current_stage = ${}", params_count)); }
        if request.total_stages.is_some() { params_count += 1; query.push_str(&format!(", total_stages = ${}", params_count)); }
        if request.completed_stages.is_some() { params_count += 1; query.push_str(&format!(", completed_stages = ${}", params_count)); }
        if request.results_summary.is_some() { params_count += 1; query.push_str(&format!(", results_summary = ${}", params_count)); }
        if request.error_message.is_some() { params_count += 1; query.push_str(&format!(", error_message = ${}", params_count)); }
        
        query.push_str(" WHERE id = $1");
        
        let mut q = sqlx::query(&query).bind(session_id.to_string());
        
        if let Some(name) = request.name { q = q.bind(name); }
        if let Some(description) = request.description { q = q.bind(description); }
        if let Some(status) = request.status { q = q.bind(serde_json::to_string(&status)?); }
        if let Some(progress) = request.progress { q = q.bind(progress); }
        if let Some(current_stage) = request.current_stage { q = q.bind(current_stage); }
        if let Some(total_stages) = request.total_stages { q = q.bind(total_stages); }
        if let Some(completed_stages) = request.completed_stages { q = q.bind(completed_stages); }
        if let Some(results_summary) = request.results_summary { q = q.bind(serde_json::to_string(&results_summary)?); }
        if let Some(error_message) = request.error_message { q = q.bind(error_message); }
        
        q.execute(pool).await?;

        Ok(())
    }

    pub async fn list_scan_sessions_internal(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        status_filter: Option<ScanSessionStatus>,
    ) -> Result<Vec<ScanSession>> {
        let pool = self.get_pool()?;
        let mut query = String::from(
            r#"
            SELECT id, name, description, target, scan_type, status, config,
                   progress, current_stage, total_stages, completed_stages,
                   results_summary, error_message, created_at, started_at,
                   completed_at, created_by
            FROM scan_sessions
            WHERE 1=1
            "#
        );

        let mut param_idx = 1;
        
        if status_filter.is_some() {
            query.push_str(&format!(" AND status = ${}", param_idx));
            param_idx += 1;
        }

        query.push_str(" ORDER BY created_at DESC");

        if limit.is_some() {
            query.push_str(&format!(" LIMIT ${}", param_idx));
            param_idx += 1;
        }

        if offset.is_some() {
            query.push_str(&format!(" OFFSET ${}", param_idx));
        }

        let mut q = sqlx::query(&query);
        
        if let Some(status) = status_filter {
            q = q.bind(serde_json::to_string(&status)?);
        }
        if let Some(l) = limit {
            q = q.bind(l);
        }
        if let Some(o) = offset {
            q = q.bind(o);
        }

        let rows = q.fetch_all(pool).await?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = ScanSession {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                name: row.get("name"),
                description: row.get("description"),
                target: row.get("target"),
                scan_type: row.get("scan_type"),
                status: serde_json::from_str(&row.get::<String, _>("status")).unwrap_or_default(),
                config: serde_json::from_str(&row.get::<String, _>("config")).unwrap_or_default(),
                progress: row.get("progress"),
                current_stage: row.get("current_stage"),
                total_stages: row.get("total_stages"),
                completed_stages: row.get("completed_stages"),
                results_summary: row
                    .get::<Option<String>, _>("results_summary")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                created_by: row.get("created_by"),
            };
            sessions.push(session);
        }

        Ok(sessions)
    }

    pub async fn delete_scan_session_internal(&self, session_id: Uuid) -> Result<()> {
        let pool = self.get_pool()?;
        // 先删除相关的扫描阶段
        sqlx::query("DELETE FROM scan_stages WHERE session_id = $1")
            .bind(session_id.to_string())
            .execute(pool)
            .await?;

        // 删除扫描会话
        sqlx::query("DELETE FROM scan_sessions WHERE id = $1")
            .bind(session_id.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn create_scan_stage_internal(&self, stage: ScanStage) -> Result<()> {
        let pool = self.get_pool()?;
        let query = r#"
            INSERT INTO scan_stages (
                id, session_id, stage_name, stage_order, status, tool_name,
                config, started_at, completed_at, duration_ms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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

        Ok(())
    }

    pub async fn update_scan_stage_internal(&self, stage: &ScanStage) -> Result<()> {
        let pool = self.get_pool()?;
        let query = r#"
            UPDATE scan_stages SET
                status = $1, results = $2, error_message = $3,
                started_at = $4, completed_at = $5, duration_ms = $6
            WHERE id = $7
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

        Ok(())
    }

    pub async fn get_scan_session_stages_internal(&self, session_id: Uuid) -> Result<Vec<ScanStage>> {
        let pool = self.get_pool()?;
        let query = r#"
            SELECT id, session_id, stage_name, stage_order, status, tool_name,
                   config, results, error_message, started_at, completed_at, duration_ms
            FROM scan_stages WHERE session_id = $1 ORDER BY stage_order
        "#;

        let rows = sqlx::query(query)
            .bind(session_id.to_string())
            .fetch_all(pool)
            .await?;

        let mut stages = Vec::new();
        for row in rows {
            let stage = ScanStage {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                session_id: Uuid::parse_str(&row.get::<String, _>("session_id"))?,
                stage_name: row.get("stage_name"),
                stage_order: row.get("stage_order"),
                status: serde_json::from_str(&row.get::<String, _>("status")).unwrap_or_default(),
                tool_name: row.get("tool_name"),
                config: serde_json::from_str(&row.get::<String, _>("config")).unwrap_or_default(),
                results: row
                    .get::<Option<String>, _>("results")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                error_message: row.get("error_message"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                duration_ms: row.get("duration_ms"),
            };
            stages.push(stage);
        }

        Ok(stages)
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

