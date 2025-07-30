use crate::models::scan_session::*;
use crate::services::database::DatabaseService;
use anyhow::Result;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

pub struct ScanSessionService {
    db: Arc<DatabaseService>,
}

impl ScanSessionService {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self { db }
    }

    /// 创建新的扫描会话
    pub async fn create_session(&self, request: CreateScanSessionRequest) -> Result<ScanSession> {
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&session.id)
            .bind(&session.name)
            .bind(&session.description)
            .bind(&session.target)
            .bind(&session.scan_type)
            .bind(&session.status)
            .bind(&session.config)
            .bind(session.progress)
            .bind(&session.current_stage)
            .bind(session.total_stages)
            .bind(session.completed_stages)
            .bind(session.created_at)
            .bind(&session.created_by)
            .execute(self.db.get_pool()?)
            .await?;

        Ok(session)
    }

    /// 获取扫描会话
    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<ScanSession>> {
        let query = r#"
            SELECT id, name, description, target, scan_type, status, config,
                   progress, current_stage, total_stages, completed_stages,
                   results_summary, error_message, created_at, started_at,
                   completed_at, created_by
            FROM scan_sessions WHERE id = ?
        "#;

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(self.db.get_pool()?)
            .await?;

        if let Some(row) = row {
            let session = ScanSession {
                id: row.get("id"),
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

    /// 更新扫描会话
    pub async fn update_session(
        &self,
        session_id: Uuid,
        request: UpdateScanSessionRequest,
    ) -> Result<()> {
        let mut query_parts = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + Send + Sync>> = Vec::new();

        if let Some(name) = &request.name {
            query_parts.push("name = ?");
            params.push(Box::new(name.clone()));
        }

        if let Some(description) = &request.description {
            query_parts.push("description = ?");
            params.push(Box::new(description.clone()));
        }

        if let Some(status) = &request.status {
            query_parts.push("status = ?");
            params.push(Box::new(serde_json::to_string(status)?));
        }

        if let Some(progress) = request.progress {
            query_parts.push("progress = ?");
            params.push(Box::new(progress));
        }

        if let Some(current_stage) = &request.current_stage {
            query_parts.push("current_stage = ?");
            params.push(Box::new(current_stage.clone()));
        }

        if let Some(total_stages) = request.total_stages {
            query_parts.push("total_stages = ?");
            params.push(Box::new(total_stages));
        }

        if let Some(completed_stages) = request.completed_stages {
            query_parts.push("completed_stages = ?");
            params.push(Box::new(completed_stages));
        }

        if let Some(results_summary) = &request.results_summary {
            query_parts.push("results_summary = ?");
            params.push(Box::new(serde_json::to_string(results_summary)?));
        }

        if let Some(error_message) = &request.error_message {
            query_parts.push("error_message = ?");
            params.push(Box::new(error_message.clone()));
        }

        if query_parts.is_empty() {
            return Ok(());
        }

        let query = format!(
            "UPDATE scan_sessions SET {} WHERE id = ?",
            query_parts.join(", ")
        );

        let mut query_builder = sqlx::query(&query);
        for param in params {
            // Note: This is a simplified approach. In practice, you'd need to handle
            // the dynamic binding more carefully.
        }
        query_builder = query_builder.bind(session_id);

        query_builder.execute(self.db.get_pool()?).await?;

        Ok(())
    }

    /// 列出扫描会话
    pub async fn list_sessions(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        status_filter: Option<ScanSessionStatus>,
    ) -> Result<Vec<ScanSession>> {
        let mut query = r#"
            SELECT id, name, description, target, scan_type, status, config,
                   progress, current_stage, total_stages, completed_stages,
                   results_summary, error_message, created_at, started_at,
                   completed_at, created_by
            FROM scan_sessions
        "#
        .to_string();

        let mut conditions = Vec::new();
        if let Some(status) = &status_filter {
            conditions.push(format!("status = '{}'", serde_json::to_string(status)?));
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query).fetch_all(self.db.get_pool()?).await?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = ScanSession {
                id: row.get("id"),
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

    /// 删除扫描会话
    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        // 先删除相关的扫描阶段
        sqlx::query("DELETE FROM scan_stages WHERE session_id = ?")
            .bind(session_id)
            .execute(self.db.get_pool()?)
            .await?;

        // 删除扫描会话
        sqlx::query("DELETE FROM scan_sessions WHERE id = ?")
            .bind(session_id)
            .execute(self.db.get_pool()?)
            .await?;

        Ok(())
    }

    /// 创建扫描阶段
    pub async fn create_stage(&self, stage: ScanStage) -> Result<()> {
        let query = r#"
            INSERT INTO scan_stages (
                id, session_id, stage_name, stage_order, status, tool_name,
                config, started_at, completed_at, duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&stage.id)
            .bind(&stage.session_id)
            .bind(&stage.stage_name)
            .bind(stage.stage_order)
            .bind(serde_json::to_string(&stage.status)?)
            .bind(&stage.tool_name)
            .bind(serde_json::to_string(&stage.config)?)
            .bind(&stage.started_at)
            .bind(&stage.completed_at)
            .bind(stage.duration_ms)
            .execute(self.db.get_pool()?)
            .await?;

        Ok(())
    }

    /// 更新扫描阶段
    pub async fn update_stage(&self, stage: &ScanStage) -> Result<()> {
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
                    .map(|r| serde_json::to_string(r))
                    .transpose()?,
            )
            .bind(&stage.error_message)
            .bind(&stage.started_at)
            .bind(&stage.completed_at)
            .bind(stage.duration_ms)
            .bind(&stage.id)
            .execute(self.db.get_pool()?)
            .await?;

        Ok(())
    }

    /// 获取会话的所有阶段
    pub async fn get_session_stages(&self, session_id: Uuid) -> Result<Vec<ScanStage>> {
        let query = r#"
            SELECT id, session_id, stage_name, stage_order, status, tool_name,
                   config, results, error_message, started_at, completed_at, duration_ms
            FROM scan_stages WHERE session_id = ? ORDER BY stage_order
        "#;

        let rows = sqlx::query(query)
            .bind(session_id)
            .fetch_all(self.db.get_pool()?)
            .await?;

        let mut stages = Vec::new();
        for row in rows {
            let stage = ScanStage {
                id: row.get("id"),
                session_id: row.get("session_id"),
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

    /// 获取扫描进度
    pub async fn get_scan_progress(&self, session_id: Uuid) -> Result<Option<ScanProgress>> {
        let session = self.get_session(session_id).await?;
        if let Some(session) = session {
            let stages = self.get_session_stages(session_id).await?;

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
