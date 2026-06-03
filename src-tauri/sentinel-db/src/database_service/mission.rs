use crate::core::models::mission::{
    CreateMissionRequest, ListMissionsFilter, Mission, MissionAction, MissionActionResult,
    MissionArtifact, MissionDelivery, MissionEvent, MissionObservation, MissionRun,
    MissionRuntimeDetail, MissionStateSnapshot, MissionStatus, UpdateMissionFieldsRequest,
};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
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
                mission_spec_json TEXT,
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
            r#"CREATE TABLE IF NOT EXISTS mission_state_snapshots (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT REFERENCES mission_runs(id),
                snapshot_index INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                state_hash TEXT NOT NULL,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_events (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT REFERENCES mission_runs(id),
                event_type TEXT NOT NULL,
                title TEXT NOT NULL,
                payload_json TEXT,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_actions (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                action_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'recorded',
                input_json TEXT,
                result_json TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mission_action_results (
                id TEXT PRIMARY KEY,
                action_id TEXT NOT NULL REFERENCES mission_actions(id),
                mission_id TEXT NOT NULL REFERENCES missions(id),
                run_id TEXT NOT NULL REFERENCES mission_runs(id),
                status TEXT NOT NULL,
                result_json TEXT,
                error_message TEXT,
                created_at DATETIME NOT NULL
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
            "CREATE INDEX IF NOT EXISTS idx_mission_state_snapshots_mission ON mission_state_snapshots(mission_id, snapshot_index DESC)",
            "CREATE INDEX IF NOT EXISTS idx_mission_events_mission ON mission_events(mission_id, created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_mission_actions_mission ON mission_actions(mission_id, run_id)",
            "CREATE INDEX IF NOT EXISTS idx_mission_action_results_action ON mission_action_results(action_id)",
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
        self.add_sqlite_column_if_missing(pool, "missions", "mission_spec_json", "TEXT")
            .await?;
        Ok(())
    }

    async fn add_sqlite_column_if_missing(
        &self,
        pool: &sqlx::SqlitePool,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        let pragma = format!("PRAGMA table_info({table})");
        let rows = sqlx::query(&pragma).fetch_all(pool).await?;
        let exists = rows.iter().any(|row| {
            sqlx::Row::try_get::<String, _>(row, "name")
                .map(|name| name == column)
                .unwrap_or(false)
        });
        if !exists {
            let alter_sql = format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}");
            sqlx::query(&alter_sql).execute(pool).await?;
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
                trigger_json, mission_spec_json, step_plan_json, success_criteria_json,
                context_strategy_json, budget_json, failure_policy_json,
                missed_run_policy, next_run_at, last_run_at, last_error,
                run_count, created_at, updated_at
            ) VALUES (
                ?, ?, ?, 'draft', ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
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
        .bind(&req.mission_spec_json)
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
        let to: MissionStatus = new_status.parse().map_err(|e: String| anyhow::anyhow!(e))?;

        if !from.can_transition_to(&to) {
            bail!("illegal mission status transition: {} -> {}", from, to);
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
        maybe_set!(mission_spec_json, "mission_spec_json");
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

        self.get_mission(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mission not found: {id}"))?;

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
        sqlx::query("DELETE FROM mission_action_results WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_actions WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_events WHERE mission_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM mission_state_snapshots WHERE mission_id = ?")
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
        self.create_mission_run_with_snapshot(mission_id, trigger_kind, None, None)
            .await
    }

    pub async fn create_mission_run_with_snapshot(
        &self,
        mission_id: &str,
        trigger_kind: &str,
        profile_snapshot: Option<&str>,
        tool_config_snapshot: Option<&str>,
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
                ?, ?,
                NULL, NULL,
                NULL, NULL,
                ?, ?
            )"#,
        )
        .bind(&id)
        .bind(mission_id)
        .bind(run_index)
        .bind(trigger_kind)
        .bind(profile_snapshot)
        .bind(tool_config_snapshot)
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

    pub async fn update_mission_run_agent_execution_id(
        &self,
        run_id: &str,
        agent_execution_id: &str,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query("UPDATE mission_runs SET agent_execution_id = ?, updated_at = ? WHERE id = ?")
            .bind(agent_execution_id)
            .bind(now)
            .bind(run_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_mission_run_checkpoint(
        &self,
        run_id: &str,
        checkpoint_json: &str,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query("UPDATE mission_runs SET checkpoint_json = ?, updated_at = ? WHERE id = ?")
            .bind(checkpoint_json)
            .bind(now)
            .bind(run_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_mission_run_bot_execution_id(
        &self,
        run_id: &str,
        bot_execution_run_id: &str,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query(
            "UPDATE mission_runs SET bot_execution_run_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(bot_execution_run_id)
        .bind(now)
        .bind(run_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Stateful Mission Runtime
    // -----------------------------------------------------------------------

    pub async fn save_mission_state_snapshot(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        state_json: &str,
    ) -> Result<MissionStateSnapshot> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let state_hash = format!("{:x}", md5::compute(state_json.as_bytes()));
        let next_index: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(snapshot_index), 0) + 1 FROM mission_state_snapshots WHERE mission_id = ?",
        )
        .bind(mission_id)
        .fetch_one(pool)
        .await?;

        sqlx::query(
            r#"INSERT INTO mission_state_snapshots (
                id, mission_id, run_id, snapshot_index, state_json, state_hash, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(mission_id)
        .bind(run_id)
        .bind(next_index)
        .bind(state_json)
        .bind(&state_hash)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionStateSnapshot>(
            "SELECT * FROM mission_state_snapshots WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_latest_mission_state_snapshot(
        &self,
        mission_id: &str,
    ) -> Result<Option<MissionStateSnapshot>> {
        let pool = self.require_sqlite_pool()?;
        sqlx::query_as::<_, MissionStateSnapshot>(
            "SELECT * FROM mission_state_snapshots WHERE mission_id = ? ORDER BY snapshot_index DESC LIMIT 1",
        )
        .bind(mission_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_latest_mission_state_snapshot_for_run(
        &self,
        mission_id: &str,
        run_id: &str,
    ) -> Result<Option<MissionStateSnapshot>> {
        let pool = self.require_sqlite_pool()?;
        sqlx::query_as::<_, MissionStateSnapshot>(
            "SELECT * FROM mission_state_snapshots WHERE mission_id = ? AND run_id = ? ORDER BY snapshot_index DESC LIMIT 1",
        )
        .bind(mission_id)
        .bind(run_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_mission_runtime_detail(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        limit: i64,
    ) -> Result<MissionRuntimeDetail> {
        let latest_state = match run_id {
            Some(run_id) => {
                self.get_latest_mission_state_snapshot_for_run(mission_id, run_id)
                    .await?
            }
            None => self.get_latest_mission_state_snapshot(mission_id).await?,
        };
        let observations = self
            .list_recent_mission_observations(mission_id, run_id, limit)
            .await?;
        let events = self.list_mission_events(mission_id, run_id, limit).await?;
        let actions = self.list_mission_actions(mission_id, run_id, limit).await?;
        let action_results = self
            .list_mission_action_results_for_mission(mission_id, run_id, limit)
            .await?;

        Ok(MissionRuntimeDetail {
            latest_state,
            observations,
            events,
            actions,
            action_results,
        })
    }

    pub async fn save_mission_event(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        event_type: &str,
        title: &str,
        payload_json: Option<&str>,
    ) -> Result<MissionEvent> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO mission_events (
                id, mission_id, run_id, event_type, title, payload_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(mission_id)
        .bind(run_id)
        .bind(event_type)
        .bind(title)
        .bind(payload_json)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionEvent>("SELECT * FROM mission_events WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_mission_events(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MissionEvent>> {
        let pool = self.require_sqlite_pool()?;
        let rows = if let Some(run_id) = run_id {
            sqlx::query_as::<_, MissionEvent>(
                "SELECT * FROM mission_events WHERE mission_id = ? AND run_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(run_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, MissionEvent>(
                "SELECT * FROM mission_events WHERE mission_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };
        Ok(rows)
    }

    pub async fn save_mission_action(
        &self,
        mission_id: &str,
        run_id: &str,
        action_type: &str,
        status: &str,
        input_json: Option<&str>,
        result_json: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<MissionAction> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO mission_actions (
                id, mission_id, run_id, action_type, status, input_json,
                result_json, error_message, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(mission_id)
        .bind(run_id)
        .bind(action_type)
        .bind(status)
        .bind(input_json)
        .bind(result_json)
        .bind(error_message)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionAction>("SELECT * FROM mission_actions WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_mission_actions(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MissionAction>> {
        let pool = self.require_sqlite_pool()?;
        let rows = if let Some(run_id) = run_id {
            sqlx::query_as::<_, MissionAction>(
                "SELECT * FROM mission_actions WHERE mission_id = ? AND run_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(run_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, MissionAction>(
                "SELECT * FROM mission_actions WHERE mission_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };
        Ok(rows)
    }

    pub async fn save_mission_action_result(
        &self,
        action_id: &str,
        mission_id: &str,
        run_id: &str,
        status: &str,
        result_json: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<MissionActionResult> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO mission_action_results (
                id, action_id, mission_id, run_id, status, result_json, error_message, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(action_id)
        .bind(mission_id)
        .bind(run_id)
        .bind(status)
        .bind(result_json)
        .bind(error_message)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionActionResult>(
            "SELECT * FROM mission_action_results WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn list_mission_action_results_for_mission(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MissionActionResult>> {
        let pool = self.require_sqlite_pool()?;
        let rows = if let Some(run_id) = run_id {
            sqlx::query_as::<_, MissionActionResult>(
                "SELECT * FROM mission_action_results WHERE mission_id = ? AND run_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(run_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, MissionActionResult>(
                "SELECT * FROM mission_action_results WHERE mission_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };
        Ok(rows)
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
        sqlx::query(
            "UPDATE missions SET next_run_at = ?, last_run_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(next_run_at)
        .bind(now)
        .bind(now)
        .bind(mission_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_mission_last_error(
        &self,
        mission_id: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query("UPDATE missions SET last_error = ?, updated_at = ? WHERE id = ?")
            .bind(last_error)
            .bind(now)
            .bind(mission_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Artifacts
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn save_mission_artifact_record(
        &self,
        id: &str,
        mission_id: &str,
        run_id: &str,
        step_id: Option<&str>,
        artifact_type: &str,
        storage_kind: &str,
        uri: &str,
        size_bytes: i64,
        content_hash: &str,
        metadata_json: Option<&str>,
    ) -> Result<MissionArtifact> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO mission_artifacts (
                id, mission_id, run_id, step_id, artifact_type,
                storage_kind, uri, size_bytes, content_hash, metadata_json,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(mission_id)
        .bind(run_id)
        .bind(step_id)
        .bind(artifact_type)
        .bind(storage_kind)
        .bind(uri)
        .bind(size_bytes)
        .bind(content_hash)
        .bind(metadata_json)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionArtifact>("SELECT * FROM mission_artifacts WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_latest_artifact_hash(
        &self,
        mission_id: &str,
        artifact_type: &str,
    ) -> Result<Option<String>> {
        let pool = self.require_sqlite_pool()?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT content_hash FROM mission_artifacts WHERE mission_id = ? AND artifact_type = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(mission_id)
        .bind(artifact_type)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(h,)| h))
    }

    pub async fn list_mission_artifacts(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
    ) -> Result<Vec<MissionArtifact>> {
        let pool = self.require_sqlite_pool()?;
        if let Some(rid) = run_id {
            let rows = sqlx::query_as::<_, MissionArtifact>(
                "SELECT * FROM mission_artifacts WHERE mission_id = ? AND run_id = ? ORDER BY created_at ASC",
            )
            .bind(mission_id)
            .bind(rid)
            .fetch_all(pool)
            .await?;
            Ok(rows)
        } else {
            let rows = sqlx::query_as::<_, MissionArtifact>(
                "SELECT * FROM mission_artifacts WHERE mission_id = ? ORDER BY created_at ASC",
            )
            .bind(mission_id)
            .fetch_all(pool)
            .await?;
            Ok(rows)
        }
    }

    pub async fn delete_mission_artifact(&self, id: &str) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        sqlx::query("DELETE FROM mission_artifacts WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Observations
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn save_mission_observation_record(
        &self,
        id: &str,
        mission_id: &str,
        run_id: &str,
        step_id: Option<&str>,
        observation_type: &str,
        severity: &str,
        title: &str,
        summary: Option<&str>,
        data_json: Option<&str>,
        artifact_ids_json: Option<&str>,
    ) -> Result<MissionObservation> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO mission_observations (
                id, mission_id, run_id, step_id, observation_type,
                severity, title, summary, data_json, artifact_ids_json,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(mission_id)
        .bind(run_id)
        .bind(step_id)
        .bind(observation_type)
        .bind(severity)
        .bind(title)
        .bind(summary)
        .bind(data_json)
        .bind(artifact_ids_json)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionObservation>("SELECT * FROM mission_observations WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_mission_observations(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        observation_type: Option<&str>,
    ) -> Result<Vec<MissionObservation>> {
        let pool = self.require_sqlite_pool()?;

        let mut sql = String::from("SELECT * FROM mission_observations WHERE mission_id = ?");
        let mut binds: Vec<String> = vec![mission_id.to_string()];

        if let Some(rid) = run_id {
            sql.push_str(" AND run_id = ?");
            binds.push(rid.to_string());
        }
        if let Some(otype) = observation_type {
            sql.push_str(" AND observation_type = ?");
            binds.push(otype.to_string());
        }
        sql.push_str(" ORDER BY created_at ASC");

        let mut query = sqlx::query_as::<_, MissionObservation>(&sql);
        for b in &binds {
            query = query.bind(b);
        }
        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    pub async fn list_recent_mission_observations(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MissionObservation>> {
        let pool = self.require_sqlite_pool()?;
        let rows = if let Some(run_id) = run_id {
            sqlx::query_as::<_, MissionObservation>(
                "SELECT * FROM mission_observations WHERE mission_id = ? AND run_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(run_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, MissionObservation>(
                "SELECT * FROM mission_observations WHERE mission_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(mission_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };
        Ok(rows)
    }

    // -----------------------------------------------------------------------
    // Deliveries
    // -----------------------------------------------------------------------

    pub async fn save_mission_delivery(
        &self,
        id: &str,
        mission_id: &str,
        run_id: &str,
        target_json: &str,
        status: &str,
        payload_json: Option<&str>,
    ) -> Result<MissionDelivery> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO mission_deliveries (
                id, mission_id, run_id, target_json, status,
                message_id, payload_json, error_message,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, NULL, ?, NULL, ?, ?)"#,
        )
        .bind(id)
        .bind(mission_id)
        .bind(run_id)
        .bind(target_json)
        .bind(status)
        .bind(payload_json)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, MissionDelivery>("SELECT * FROM mission_deliveries WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_mission_delivery_status(
        &self,
        id: &str,
        status: &str,
        error_message: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<()> {
        let pool = self.require_sqlite_pool()?;
        let now = Utc::now();
        sqlx::query(
            "UPDATE mission_deliveries SET status = ?, error_message = ?, message_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(error_message)
        .bind(message_id)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_mission_deliveries(
        &self,
        mission_id: &str,
        run_id: Option<&str>,
    ) -> Result<Vec<MissionDelivery>> {
        let pool = self.require_sqlite_pool()?;
        if let Some(rid) = run_id {
            let rows = sqlx::query_as::<_, MissionDelivery>(
                "SELECT * FROM mission_deliveries WHERE mission_id = ? AND run_id = ? ORDER BY created_at DESC",
            )
            .bind(mission_id)
            .bind(rid)
            .fetch_all(pool)
            .await?;
            Ok(rows)
        } else {
            let rows = sqlx::query_as::<_, MissionDelivery>(
                "SELECT * FROM mission_deliveries WHERE mission_id = ? ORDER BY created_at DESC",
            )
            .bind(mission_id)
            .fetch_all(pool)
            .await?;
            Ok(rows)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_service::connection_manager::DatabasePool;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn in_memory_service() -> DatabaseService {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let mut service = DatabaseService::new();
        service.runtime_pool = Some(DatabasePool::SQLite(pool.clone()));
        service
            .create_mission_schema(&pool)
            .await
            .expect("mission schema");
        service
    }

    #[tokio::test]
    async fn mission_runtime_state_records_are_persisted() {
        let service = in_memory_service().await;
        let mission = service
            .create_mission(CreateMissionRequest {
                title: "Stateful mission".to_string(),
                objective: "Advance durable state".to_string(),
                owner_kind: "user".to_string(),
                owner_ref: "default".to_string(),
                source_json: None,
                delivery_policy_json: None,
                assistant_profile_id: None,
                trigger_json: Some(serde_json::json!({"kind":"manual"}).to_string()),
                mission_spec_json: Some(
                    serde_json::json!({
                        "kind": "generic_stateful_mission",
                        "state_schema": {"type": "object"},
                        "action_schema": {"type": "object"},
                        "completion_policy": {"kind": "manual_or_agent_completion"},
                        "report_policy": {"kind": "each_run_summary"}
                    })
                    .to_string(),
                ),
                step_plan_json: None,
                success_criteria_json: None,
                context_strategy_json: None,
                budget_json: None,
                failure_policy_json: None,
                missed_run_policy: "skip".to_string(),
                next_run_at: None,
            })
            .await
            .expect("create mission");
        let run = service
            .create_mission_run(&mission.id, "manual")
            .await
            .expect("create run");

        let snapshot = service
            .save_mission_state_snapshot(&mission.id, Some(&run.id), r#"{"day":1}"#)
            .await
            .expect("save state");
        assert_eq!(snapshot.snapshot_index, 1);

        let latest = service
            .get_latest_mission_state_snapshot(&mission.id)
            .await
            .expect("latest state")
            .expect("state exists");
        assert_eq!(latest.state_json, r#"{"day":1}"#);

        let action = service
            .save_mission_action(
                &mission.id,
                &run.id,
                "record_decision",
                "succeeded",
                Some(r#"{"input":true}"#),
                Some(r#"{"ok":true}"#),
                None,
            )
            .await
            .expect("save action");
        service
            .save_mission_action_result(
                &action.id,
                &mission.id,
                &run.id,
                "succeeded",
                Some(r#"{"ok":true}"#),
                None,
            )
            .await
            .expect("save action result");
        service
            .save_mission_event(
                &mission.id,
                Some(&run.id),
                "mission_tick_completed",
                "Tick completed",
                Some(r#"{"summary":"ok"}"#),
            )
            .await
            .expect("save event");

        let detail = service
            .get_mission_runtime_detail(&mission.id, None, 10)
            .await
            .expect("runtime detail");
        assert_eq!(
            detail.latest_state.expect("latest state").state_json,
            r#"{"day":1}"#
        );
        assert_eq!(detail.actions.len(), 1);
        assert_eq!(detail.action_results.len(), 1);
        assert_eq!(detail.events.len(), 1);

        let run_detail = service
            .get_mission_runtime_detail(&mission.id, Some(&run.id), 10)
            .await
            .expect("runtime detail for run");
        assert_eq!(
            run_detail.latest_state.expect("run state").state_json,
            r#"{"day":1}"#
        );
        assert_eq!(run_detail.actions.len(), 1);
        assert_eq!(run_detail.action_results.len(), 1);
        assert_eq!(run_detail.events.len(), 1);
    }

    #[tokio::test]
    async fn active_mission_can_be_deleted_manually() {
        let service = in_memory_service().await;
        let mission = service
            .create_mission(CreateMissionRequest {
                title: "Delete active mission".to_string(),
                objective: "Manual delete should remove active missions".to_string(),
                owner_kind: "user".to_string(),
                owner_ref: "default".to_string(),
                source_json: None,
                delivery_policy_json: None,
                assistant_profile_id: None,
                trigger_json: Some(serde_json::json!({"kind":"manual"}).to_string()),
                mission_spec_json: None,
                step_plan_json: None,
                success_criteria_json: None,
                context_strategy_json: None,
                budget_json: None,
                failure_policy_json: None,
                missed_run_policy: "skip".to_string(),
                next_run_at: None,
            })
            .await
            .expect("create mission");

        let active = service
            .update_mission_status(&mission.id, "active")
            .await
            .expect("activate mission");
        assert_eq!(active.status, "active");

        let run = service
            .create_mission_run(&mission.id, "manual")
            .await
            .expect("create run");
        service
            .save_mission_state_snapshot(&mission.id, Some(&run.id), r#"{"day":1}"#)
            .await
            .expect("save state");
        let action = service
            .save_mission_action(&mission.id, &run.id, "hold", "succeeded", None, None, None)
            .await
            .expect("save action");
        service
            .save_mission_action_result(&action.id, &mission.id, &run.id, "succeeded", None, None)
            .await
            .expect("save action result");
        service
            .save_mission_event(
                &mission.id,
                Some(&run.id),
                "deleted_test",
                "Delete test",
                None,
            )
            .await
            .expect("save event");

        service
            .delete_mission(&mission.id)
            .await
            .expect("delete active mission");

        assert!(service
            .get_mission(&mission.id)
            .await
            .expect("get mission")
            .is_none());
        assert!(service
            .list_mission_runs(&mission.id, 10, 0)
            .await
            .expect("list runs")
            .is_empty());
        let detail = service
            .get_mission_runtime_detail(&mission.id, None, 10)
            .await
            .expect("runtime detail");
        assert!(detail.latest_state.is_none());
        assert!(detail.actions.is_empty());
        assert!(detail.action_results.is_empty());
        assert!(detail.events.is_empty());
    }
}
