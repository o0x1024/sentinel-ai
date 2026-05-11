use crate::core::models::mission::{
    CreateMissionRequest, ListMissionsFilter, Mission, MissionArtifact, MissionDelivery, MissionLock,
    MissionObservation, MissionRun, MissionStatus, MissionStep, UpdateMissionFieldsRequest,
};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::{bail, Result};
use chrono::Utc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

impl DatabaseService {
    /// Create mission tables. Only SQLite DDL — no 3x match.
    pub async fn create_mission_schema(&self, pool: &sqlx::SqlitePool) -> Result<()> {
        let statements: &[&str] = &[
            r#"CREATE TABLE IF NOT EXISTS missions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                objective TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                owner_kind TEXT NOT NULL,
                owner_ref TEXT NOT NULL,
                source_json TEXT,
                delivery_policy_json TEXT,
                assistant_profile_id TEXT,
                trigger_json TEXT,
                step_plan_json TEXT,
                success_criteria_json TEXT,
                context_strategy_json TEXT,
                budget_json TEXT,
                failure_policy_json TEXT,
                missed_run_policy TEXT NOT NULL DEFAULT 'skip',
                next_run_at DATETIME,
                last_run_at DATETIME,
                last_error TEXT,
                run_count INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_runs (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_index INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'queued',
                trigger_kind TEXT NOT NULL,
                started_at DATETIME,
                completed_at DATETIME,
                agent_execution_id TEXT,
                bot_execution_run_id TEXT,
                assistant_profile_snapshot_json TEXT,
                tool_config_snapshot_json TEXT,
                checkpoint_json TEXT,
                context_injected_json TEXT,
                result_summary TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_steps (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                step_index INTEGER NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                input_json TEXT,
                output_json TEXT,
                error_message TEXT,
                started_at DATETIME,
                completed_at DATETIME
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_artifacts (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                step_id TEXT,
                artifact_type TEXT NOT NULL,
                storage_kind TEXT NOT NULL DEFAULT 'filesystem',
                uri TEXT NOT NULL,
                size_bytes INTEGER,
                content_hash TEXT,
                metadata_json TEXT,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_observations (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                step_id TEXT,
                observation_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'info',
                title TEXT NOT NULL,
                summary TEXT,
                data_json TEXT,
                artifact_ids_json TEXT,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_deliveries (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                target_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                message_id TEXT,
                payload_json TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_locks (
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL,
                lock_owner TEXT NOT NULL,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL,
                PRIMARY KEY (mission_id)
            )"#,
            // Indexes
            "CREATE INDEX IF NOT EXISTS idx_missions_owner ON missions(owner_kind, owner_ref, status)",
            "CREATE INDEX IF NOT EXISTS idx_missions_status_next_run ON missions(status, next_run_at)",
            "CREATE INDEX IF NOT EXISTS idx_mission_runs_mission_id ON mission_runs(mission_id, run_index DESC)",
            "CREATE INDEX IF NOT EXISTS idx_mission_runs_status ON mission_runs(status)",
            "CREATE INDEX IF NOT EXISTS idx_mission_steps_run_id ON mission_steps(run_id, step_index)",
            "CREATE INDEX IF NOT EXISTS idx_mission_artifacts_mission ON mission_artifacts(mission_id, run_id)",
            "CREATE INDEX IF NOT EXISTS idx_mission_artifacts_type ON mission_artifacts(mission_id, artifact_type)",
            "CREATE INDEX IF NOT EXISTS idx_mission_observations_mission ON mission_observations(mission_id, run_id)",
            "CREATE INDEX IF NOT EXISTS idx_mission_observations_type ON mission_observations(mission_id, observation_type)",
            "CREATE INDEX IF NOT EXISTS idx_mission_deliveries_mission ON mission_deliveries(mission_id, run_id)",
            "CREATE INDEX IF NOT EXISTS idx_mission_locks_expires ON mission_locks(expires_at)",
        ];

        for sql in statements {
            sqlx::query(sql).execute(pool).await?;
        }
        Ok(())
    }

    /// Ensure mission schema via runtime pool (called during app init).
    pub async fn ensure_mission_schema(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::SQLite(pool) => self.create_mission_schema(pool).await,
            _ => {
                tracing::warn!("Mission schema only supports SQLite; skipping for current DB type");
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Mission CRUD
    // -----------------------------------------------------------------------

    pub async fn create_mission(&self, req: CreateMissionRequest) -> Result<Mission> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO missions (
                id, title, objective, status, owner_kind, owner_ref,
                source_json, delivery_policy_json, assistant_profile_id,
                trigger_json, step_plan_json, success_criteria_json,
                context_strategy_json, budget_json, failure_policy_json,
                missed_run_policy, next_run_at, last_run_at, last_error,
                run_count, created_at, updated_at
            ) VALUES (
                ?, ?, ?, 'draft', ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, NULL, NULL,
                0, ?, ?
            )"#,
        )
        .bind(&id)
        .bind(&req.title)
        .bind(&req.objective)
        .bind(&req.owner_kind)
        .bind(&req.owner_ref)
        .bind(&req.source_json)
        .bind(&req.delivery_policy_json)
        .bind(&req.assistant_profile_id)
        .bind(&req.trigger_json)
        .bind(&req.step_plan_json)
        .bind(&req.success_criteria_json)
        .bind(&req.context_strategy_json)
        .bind(&req.budget_json)
        .bind(&req.failure_policy_json)
        .bind(&req.missed_run_policy)
        .bind(&req.next_run_at)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        self.get_mission(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to read back created mission"))
    }

    pub async fn get_mission(&self, id: &str) -> Result<Option<Mission>> {
        let pool = self.require_sqlite_pool()?;
        let row = sqlx::query_as::<_, Mission>("SELECT * FROM missions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn list_missions(&self, filter: &ListMissionsFilter) -> Result<Vec<Mission>> {
        let pool = self.require_sqlite_pool()?;

        let mut sql = String::from("SELECT * FROM missions WHERE 1=1");
        let mut binds: Vec<String> = Vec::new();

        if let Some(ref ok) = filter.owner_kind {
            sql.push_str(" AND owner_kind = ?");
            binds.push(ok.clone());
        }
        if let Some(ref or_) = filter.owner_ref {
            sql.push_str(" AND owner_ref = ?");
            binds.push(or_.clone());
        }
        if let Some(ref st) = filter.status {
            sql.push_str(" AND status = ?");
            binds.push(st.clone());
        }

        sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, Mission>(&sql);
        for b in &binds {
            query = query.bind(b);
        }
        query = query.bind(filter.limit).bind(filter.offset);

        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    pub async fn update_mission_status(&self, id: &str, new_status: &str) -> Result<Mission> {
        let pool = self.require_sqlite_pool()?;

        let current = self
            .get_mission(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found: {id}"))?;

        let from: MissionStatus = current
            .status
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let to: MissionStatus = new_status
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;

        if !from.can_transition_to(&to) {
            bail!(
                "illegal mission status transition: {} -> {}",
                from,
                to
            );
        }

        let now = Utc::now();
        sqlx::query("UPDATE missions SET status = ?, updated_at = ? WHERE id = ?")
            .bind(new_status)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;

        self.get_mission(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found after update"))
    }

    pub async fn update_mission_fields(&self, req: UpdateMissionFieldsRequest) -> Result<Mission> {
        let pool = self.require_sqlite_pool()?;

        let _existing = self
            .get_mission(&req.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found: {}", req.id))?;

        let now = Utc::now();
        let mut sets = vec!["updated_at = ?".to_string()];
        let mut binds: Vec<Option<String>> = vec![Some(now.to_rfc3339())];

        macro_rules! maybe_set {
            ($field:ident, $col:expr) => {
                if let Some(ref val) = req.$field {
                    sets.push(format!("{} = ?", $col));
                    binds.push(Some(val.clone()));
                }
            };
        }

        maybe_set!(title, "title");
        maybe_set!(objective, "objective");
        maybe_set!(trigger_json, "trigger_json");
        maybe_set!(delivery_policy_json, "delivery_policy_json");
        maybe_set!(assistant_profile_id, "assistant_profile_id");
        maybe_set!(step_plan_json, "step_plan_json");
        maybe_set!(success_criteria_json, "success_criteria_json");
        maybe_set!(context_strategy_json, "context_strategy_json");
        maybe_set!(budget_json, "budget_json");
        maybe_set!(failure_policy_json, "failure_policy_json");
        maybe_set!(missed_run_policy, "missed_run_policy");

        if let Some(ref next) = req.next_run_at {
            sets.push("next_run_at = ?".to_string());
            binds.push(Some(next.to_rfc3339()));
        }

        let sql = format!("UPDATE missions SET {} WHERE id = ?", sets.join(", "));
        let mut query = sqlx::query(&sql);
        for b in &binds {
            query = query.bind(b.as_deref());
        }
        query = query.bind(&req.id);
        query.execute(pool).await?;

        self.get_mission(&req.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found after update"))
    }

    pub async fn delete_mission(&self, id: &str) -> Result<()> {
        let pool = self.require_sqlite_pool()?;

        let mission = self
            .get_mission(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found: {id}"))?;

        let status: MissionStatus = mission
            .status
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        if !matches!(status, MissionStatus::Draft | MissionStatus::Archived) {
            bail!(
                "can only delete missions in draft or archived status, current: {}",
                status
            );
        }

        sqlx::query("DELETE FROM mission_locks WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_deliveries WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_observations WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_artifacts WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_steps WHERE run_id IN (SELECT id FROM mission_runs WHERE mission_id = ?)")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_runs WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM missions WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mission Runs
    // -----------------------------------------------------------------------

    pub async fn create_mission_run(
        &self,
        mission_id: &str,
        trigger_kind: &str,
    ) -> Result<MissionRun> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        let mission = self
            .get_mission(mission_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found: {mission_id}"))?;

        let run_index = mission.run_count + 1;

        sqlx::query(
            r#"INSERT INTO mission_runs (
                id, mission_id, run_index, status, trigger_kind,
                started_at, completed_at,
                agent_execution_id, bot_execution_run_id,
                assistant_profile_snapshot_json, tool_config_snapshot_json,
                checkpoint_json, context_injected_json,
                result_summary, error_message,
                created_at, updated_at
            ) VALUES (
                ?, ?, ?, 'queued', ?,
                NULL, NULL,
                NULL, NULL,
                NULL, NULL,
                NULL, NULL,
                NULL, NULL,
                ?, ?
            )"#,
        )
        .bind(&id)
        .bind(mission_id)
        .bind(run_index)
        .bind(trigger_kind)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query("UPDATE missions SET run_count = ?, updated_at = ? WHERE id = ?")
            .bind(run_index)
            .bind(now)
            .bind(mission_id)
            .execute(pool)
            .await?;

        self.get_mission_run(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to read back created mission run"))
    }

    pub async fn get_mission_run(&self, id: &str) -> Result<Option<MissionRun>> {
        let pool = self.require_sqlite_pool()?;
        let row = sqlx::query_as::<_, MissionRun>("SELECT * FROM mission_runs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn list_mission_runs(
        &self,
        mission_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MissionRun>> {
        let pool = self.require_sqlite_pool()?;
        let rows = sqlx::query_as::<_, MissionRun>(
            "SELECT * FROM mission_runs WHERE mission_id = ? ORDER BY run_index DESC LIMIT ? OFFSET ?",
        )
        .bind(mission_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_mission_run_status(
        &self,
        run_id: &str,
        status: &str,
        error_message: Option<&str>,
        result_summary: Option<&str>,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let completed_at = match status {
            "succeeded" | "partial" | "failed" | "cancelled" | "timed_out" => Some(now),
            _ => None,
        };
        let started_at = match status {
            "running" => Some(now),
            _ => None,
        };

        if let Some(sa) = started_at {
            sqlx::query(
                "UPDATE mission_runs SET status = ?, started_at = COALESCE(started_at, ?), updated_at = ? WHERE id = ?",
            )
            .bind(status)
            .bind(sa)
            .bind(now)
            .bind(run_id)
            .execute(pool)
            .await?;
        } else if let Some(ca) = completed_at {
            sqlx::query(
                "UPDATE mission_runs SET status = ?, completed_at = ?, error_message = COALESCE(?, error_message), result_summary = COALESCE(?, result_summary), updated_at = ? WHERE id = ?",
            )
            .bind(status)
            .bind(ca)
            .bind(error_message)
            .bind(result_summary)
            .bind(now)
            .bind(run_id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query("UPDATE mission_runs SET status = ?, updated_at = ? WHERE id = ?")
                .bind(status)
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mission Locks
    // -----------------------------------------------------------------------

    pub async fn acquire_mission_lock(
        &self,
        mission_id: &str,
        run_id: &str,
        lock_owner: &str,
        ttl_seconds: i64,
    ) -> Result<bool> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(ttl_seconds);

        // Clean expired locks first
        sqlx::query("DELETE FROM mission_locks WHERE expires_at < ?")
            .bind(now)
            .execute(pool)
            .await?;

        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO mission_locks (mission_id, run_id, lock_owner, expires_at, created_at)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(mission_id)
        .bind(run_id)
        .bind(lock_owner)
        .bind(expires_at)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn release_mission_lock(&self, mission_id: &str) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        sqlx::query("DELETE FROM mission_locks WHERE mission_id = ?")
            .bind(mission_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn release_expired_mission_locks(&self) -> Result<u64> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let result = sqlx::query("DELETE FROM mission_locks WHERE expires_at < ?")
            .bind(now)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    // -----------------------------------------------------------------------
    // Due missions query (for scheduler)
    // -----------------------------------------------------------------------

    pub async fn list_due_missions(&self) -> Result<Vec<Mission>> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let rows = sqlx::query_as::<_, Mission>(
            "SELECT * FROM missions WHERE status = 'active' AND next_run_at IS NOT NULL AND next_run_at <= ? ORDER BY next_run_at ASC",
        )
        .bind(now)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Find missions that were active but missed their run window (for startup recovery).
    pub async fn list_missed_missions(&self) -> Result<Vec<Mission>> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let rows = sqlx::query_as::<_, Mission>(
            "SELECT * FROM missions WHERE status = 'active' AND next_run_at IS NOT NULL AND next_run_at < ? ORDER BY next_run_at ASC",
        )
        .bind(now)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Find stale running runs (for crash recovery).
    pub async fn list_stale_running_runs(&self) -> Result<Vec<MissionRun>> {
        let pool = self.require_sqlite_pool()?;
        let rows = sqlx::query_as::<_, MissionRun>(
            r#"SELECT mr.* FROM mission_runs mr
               LEFT JOIN mission_locks ml ON mr.mission_id = ml.mission_id
               WHERE mr.status = 'running' AND ml.mission_id IS NULL
               ORDER BY mr.created_at ASC"#,
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_mission_next_run(
        &self,
        mission_id: &str,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query("UPDATE missions SET next_run_at = ?, last_run_at = ?, updated_at = ? WHERE id = ?")
            .bind(next_run_at)
            .bind(now)
            .bind(now)
            .bind(mission_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Helper: require SQLite pool
    // -----------------------------------------------------------------------

    fn require_sqlite_pool(&self) -> Result<&sqlx::SqlitePool> {
        match self.runtime_pool.as_ref() {
            Some(DatabasePool::SQLite(pool)) => Ok(pool),
            Some(_) => bail!("Mission operations require SQLite database"),
            None => bail!("数据库未初始化"),
        }
    }
}
