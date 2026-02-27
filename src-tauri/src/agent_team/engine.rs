//! Agent Team 核心引擎 - 轮次编排与状态机流转

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use futures::stream::{FuturesUnordered, StreamExt};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration as StdDuration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info, warn};

use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use sentinel_llm::{ChatMessage, LlmConfig, StreamContent, StreamingLlmClient};
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::{
    buildin_tools::ShellTool,
    get_tool_server, DynamicTool,
};

use super::blackboard::BlackboardManager;
use super::models::*;
use super::repository_runtime as repo_rt;
use super::role_context::{build_role_context, RoleContextInput};
use super::scheduler::ToolGovernance;

/// 引擎配置（可运行时注入）
#[derive(Debug, Clone)]
pub struct TeamEngineConfig {
    /// 分歧度阈值，超过此值触发额外讨论轮次
    pub divergence_threshold: f64,
    /// 最大连续讨论轮次（防止死循环）
    pub max_challenge_rounds: i32,
}

impl Default for TeamEngineConfig {
    fn default() -> Self {
        Self {
            divergence_threshold: 0.4,
            max_challenge_rounds: 3,
        }
    }
}

const EXECUTION_MEMORY_MAX_CARDS: usize = 24;
const EXECUTION_MEMORY_PROMPT_CARDS: usize = 8;

/// Agent Team 引擎
pub struct AgentTeamEngine {
    app_handle: AppHandle,
    blackboard: Arc<BlackboardManager>,
    config: TeamEngineConfig,
    tool_governance: Arc<Mutex<ToolGovernance>>,
}

impl AgentTeamEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            blackboard: BlackboardManager::new(),
            config: TeamEngineConfig::default(),
            tool_governance: Arc::new(Mutex::new(ToolGovernance::new())),
        }
    }

    pub fn with_config(mut self, config: TeamEngineConfig) -> Self {
        self.config = config;
        self
    }

    /// 启动 Team 运行
    pub async fn start_run(&self, session_id: &str) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        // Keep in-memory shell/terminal runtime aligned with latest Agent settings.
        if let Err(e) =
            crate::commands::tool_commands::agent_config::init_agent_config(db.as_ref()).await
        {
            warn!("Failed to sync agent config before team run: {}", e);
        }

        // 获取会话信息
        let session = repo_rt::get_agent_team_session(&pool, session_id)
            .await?
            .context("Session not found")?;

        if session.members.is_empty() {
            return Err(anyhow::anyhow!("Session has no members"));
        }
        if session.schema_version < 2 {
            return Err(anyhow::anyhow!(
                "Team session schema_version={} is no longer supported. Please run template/session upgrade to v2 first.",
                session.schema_version
            ));
        }

        info!(
            "Starting Agent Team run: session_id={}, goal={}",
            session_id,
            session.goal.as_deref().unwrap_or("(no goal)")
        );

        // 状态: PENDING -> INITIALIZING
        self.transition_state(session_id, TeamSessionState::Initializing)
            .await?;
        self.emit_event(
            session_id,
            "agent_team:start",
            json!({"session_id": session_id}),
        );

        // 从数据库重建白板（支持断点恢复）
        let blackboard_entries = repo_rt::get_blackboard_entries(&pool, session_id).await?;
        self.blackboard
            .rebuild_from_entries(session_id, blackboard_entries)
            .await;

        // 初始化角色工具策略（写入引擎内治理器）
        {
            let mut tool_governance = self
                .tool_governance
                .lock()
                .map_err(|_| anyhow::anyhow!("tool governance lock poisoned"))?;
            *tool_governance = ToolGovernance::new();
            if let Some(state_machine) = &session.state_machine {
                if let Some(policy) = state_machine.get("tool_policy") {
                    tool_governance.load_session_policy(policy);
                }
            }
        }

        let runtime_spec_v2 = session
            .runtime_spec_v2
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime_spec_v2 missing for v2 session"))?;
        self.execute_task_graph_v2(session_id, &session, runtime_spec_v2)
            .await?;

        // 生成标准文档产物
        self.generate_artifacts(session_id, &session).await?;

        // 状态: -> COMPLETED
        self.transition_state(session_id, TeamSessionState::Completed)
            .await?;

        // 清理白板内存缓存
        self.blackboard.cleanup(session_id).await;

        self.emit_event(
            session_id,
            "agent_team:complete",
            json!({"session_id": session_id}),
        );

        info!("Agent Team run completed: session_id={}", session_id);
        Ok(())
    }

    async fn execute_task_graph_v2(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        runtime_spec_v2: &Value,
    ) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        repo_rt::ensure_session_tasks_from_runtime_spec(&pool, session_id, runtime_spec_v2).await?;

        let node_map = Self::build_task_node_map(runtime_spec_v2);
        let agent_limits = self.build_agent_concurrency_limits(session, runtime_spec_v2);
        let global_max_parallel = runtime_spec_v2
            .get("max_parallel_tasks")
            .or_else(|| runtime_spec_v2.get("global_max_parallel_tasks"))
            .and_then(|v| v.as_i64())
            .unwrap_or(4)
            .clamp(1, 32) as usize;
        let round_counter = Arc::new(AtomicI32::new((session.current_round + 1).max(1)));

        self.transition_state(session_id, TeamSessionState::Revising)
            .await?;
        self.emit_event(
            session_id,
            "agent_team:task_graph_started",
            json!({
                "session_id": session_id,
                "global_max_parallel_tasks": global_max_parallel,
                "node_count": node_map.len(),
            }),
        );

        let mut in_flight: FuturesUnordered<BoxFuture<'_, (String, String, Result<()>)>> =
            FuturesUnordered::new();
        let mut in_flight_by_member: HashMap<String, usize> = HashMap::new();
        let mut in_flight_task_ids: HashSet<String> = HashSet::new();

        loop {
            let mut tasks = repo_rt::list_tasks(&pool, session_id).await?;
            if tasks.is_empty() {
                return Err(anyhow::anyhow!(
                    "No task rows found for session {} runtime_spec_v2",
                    session_id
                ));
            }

            let status_map: HashMap<String, String> = tasks
                .iter()
                .map(|t| (t.task_id.clone(), t.status.to_lowercase()))
                .collect();

            for task in tasks.iter().filter(|t| Self::is_task_pending(&t.status)) {
                let mut blocked_reason: Option<String> = None;
                for dep in &task.depends_on {
                    let dep_status = status_map
                        .get(dep)
                        .map(|s| s.as_str())
                        .unwrap_or("missing_dependency");
                    if Self::is_task_failed(dep_status) {
                        blocked_reason = Some(format!(
                            "Blocked by dependency '{}' status={}",
                            dep, dep_status
                        ));
                        break;
                    }
                }

                if let Some(reason) = blocked_reason {
                    repo_rt::update_task(
                        &pool,
                        session_id,
                        &UpdateTaskRequest {
                            task_id: task.task_id.clone(),
                            status: Some("blocked".to_string()),
                            assignee_agent_id: None,
                            last_error: Some(reason.clone()),
                        },
                    )
                    .await?;
                    let payload = json!({
                        "session_id": session_id,
                        "task_id": task.task_id,
                        "task_record_id": task.id,
                        "reason": reason,
                    });
                    let _ = repo_rt::append_task_event(
                        &pool,
                        session_id,
                        Some(&task.id),
                        "task_blocked",
                        &payload,
                    )
                    .await;
                    self.emit_event(session_id, "agent_team:task_blocked", payload);
                }
            }

            tasks = repo_rt::list_tasks(&pool, session_id).await?;
            let status_map: HashMap<String, String> = tasks
                .iter()
                .map(|t| (t.task_id.clone(), t.status.to_lowercase()))
                .collect();

            let all_terminal = tasks.iter().all(|t| Self::is_task_terminal(&t.status));
            if all_terminal {
                let failed: Vec<String> = tasks
                    .iter()
                    .filter(|t| Self::is_task_failed(&t.status))
                    .map(|t| t.task_id.clone())
                    .collect();
                if !failed.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Task graph completed with failures: {}",
                        failed.join(", ")
                    ));
                }
                break;
            }

            let ready_tasks: Vec<TeamTask> = tasks
                .iter()
                .filter(|t| {
                    Self::is_task_pending(&t.status)
                        && !in_flight_task_ids.contains(&t.task_id)
                        && t.depends_on.iter().all(|dep| {
                            status_map
                                .get(dep)
                                .map(|s| Self::is_task_completed(s))
                                .unwrap_or(false)
                        })
                })
                .cloned()
                .collect();

            for task in ready_tasks {
                if in_flight.len() >= global_max_parallel {
                    break;
                }
                let node = node_map.get(&task.task_id);
                let Some(member) = self.select_task_assignee(
                    session,
                    &task,
                    node,
                    &in_flight_by_member,
                    &agent_limits,
                ) else {
                    continue;
                };
                let used = *in_flight_by_member.get(&member.id).unwrap_or(&0);
                let limit = *agent_limits.get(&member.id).unwrap_or(&1);
                if used >= limit {
                    continue;
                }
                let member_id = member.id.clone();
                let member_name = member.name.clone();

                repo_rt::update_task(
                    &pool,
                    session_id,
                    &UpdateTaskRequest {
                        task_id: task.task_id.clone(),
                        status: Some("running".to_string()),
                        assignee_agent_id: Some(member_id.clone()),
                        last_error: None,
                    },
                )
                .await?;

                let dispatch_payload = json!({
                    "session_id": session_id,
                    "task_id": task.task_id,
                    "task_record_id": task.id,
                    "assignee_agent_id": member_id,
                    "assignee_agent_name": member_name,
                    "attempt": task.attempt + 1,
                    "max_attempts": task.max_attempts,
                });
                let _ = repo_rt::append_task_event(
                    &pool,
                    session_id,
                    Some(&task.id),
                    "task_dispatch",
                    &dispatch_payload,
                )
                .await;
                self.emit_event(session_id, "agent_team:task_dispatched", dispatch_payload);

                in_flight_by_member
                    .entry(member_id.clone())
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
                in_flight_task_ids.insert(task.task_id.clone());
                in_flight.push(self.execute_single_task_v2(
                    &pool,
                    session_id,
                    session,
                    task,
                    member.clone(),
                    node.cloned(),
                    round_counter.clone(),
                ));
            }

            if in_flight.is_empty() {
                return Err(anyhow::anyhow!(
                    "Task scheduler stalled for session {}. Check dependency graph / assignee strategy.",
                    session_id
                ));
            }

            if let Some((task_id, member_id, result)) = in_flight.next().await {
                in_flight_task_ids.remove(&task_id);
                if let Some(used) = in_flight_by_member.get_mut(&member_id) {
                    if *used > 0 {
                        *used -= 1;
                    }
                }
                result?;
            }
        }

        self.emit_event(
            session_id,
            "agent_team:task_graph_completed",
            json!({"session_id": session_id}),
        );
        Ok(())
    }

    fn execute_single_task_v2<'a>(
        &'a self,
        pool: &'a DatabasePool,
        session_id: &'a str,
        session: &'a AgentTeamSession,
        task: TeamTask,
        member: AgentTeamMember,
        node: Option<Value>,
        round_counter: Arc<AtomicI32>,
    ) -> BoxFuture<'a, (String, String, Result<()>)> {
        Box::pin(async move {
            let task_id = task.task_id.clone();
            let member_id = member.id.clone();
            let attempt = task.attempt + 1;
            let phase = node
                .as_ref()
                .and_then(|n| n.get("phase"))
                .and_then(|v| v.as_str())
                .unwrap_or("task_execution");
            let step_label = node
                .as_ref()
                .and_then(|n| n.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or(task.title.as_str());
            let step_instruction = node
                .as_ref()
                .and_then(|n| n.get("instruction"))
                .and_then(|v| v.as_str())
                .unwrap_or(task.instruction.as_str());
            let backoff_ms = node
                .as_ref()
                .and_then(|n| n.get("retry"))
                .and_then(|r| r.get("backoff_ms"))
                .and_then(|v| v.as_i64())
                .unwrap_or(800)
                .clamp(100, 30_000) as u64;
            let round_number = round_counter.fetch_add(1, Ordering::SeqCst);
            let started = Instant::now();
            let step_obj = json!({
                "id": task.task_id,
                "type": "agent",
                "name": step_label,
                "phase": phase,
                "member": member_id.clone(),
                "instruction": step_instruction,
            });
            let step_map = step_obj.as_object().cloned().unwrap_or_default();

            let result = match self
                .run_orchestration_agent_step(
                    session_id,
                    session,
                    &task_id,
                    &step_map,
                    round_number,
                )
                .await
            {
                Ok(_) => {
                    let duration_ms = started.elapsed().as_millis() as i64;
                    match repo_rt::update_task(
                        pool,
                        session_id,
                        &UpdateTaskRequest {
                            task_id: task_id.clone(),
                            status: Some("completed".to_string()),
                            assignee_agent_id: Some(member_id.clone()),
                            last_error: None,
                        },
                    )
                    .await
                    {
                        Ok(_) => {
                            let _ = repo_rt::append_task_attempt(
                                pool,
                                session_id,
                                &task.id,
                                attempt,
                                "succeeded",
                                None,
                                Some(duration_ms),
                            )
                            .await;
                            let payload = json!({
                                "session_id": session_id,
                                "task_id": task_id,
                                "task_record_id": task.id,
                                "assignee_agent_id": member_id,
                                "attempt": attempt,
                                "duration_ms": duration_ms,
                            });
                            let _ = repo_rt::append_task_event(
                                pool,
                                session_id,
                                Some(&task.id),
                                "task_complete",
                                &payload,
                            )
                            .await;
                            self.emit_event(session_id, "agent_team:task_completed", payload);
                            Ok(())
                        }
                        Err(err) => Err(err),
                    }
                }
                Err(e) => {
                    let duration_ms = started.elapsed().as_millis() as i64;
                    let err_text = e.to_string();
                    let _ = repo_rt::append_task_attempt(
                        pool,
                        session_id,
                        &task.id,
                        attempt,
                        "failed",
                        Some(err_text.as_str()),
                        Some(duration_ms),
                    )
                    .await;

                    if attempt < task.max_attempts {
                        match repo_rt::update_task(
                            pool,
                            session_id,
                            &UpdateTaskRequest {
                                task_id: task_id.clone(),
                                status: Some("pending".to_string()),
                                assignee_agent_id: Some(member_id.clone()),
                                last_error: Some(err_text.clone()),
                            },
                        )
                        .await
                        {
                            Ok(_) => {
                                let payload = json!({
                                    "session_id": session_id,
                                    "task_id": task_id,
                                    "task_record_id": task.id,
                                    "attempt": attempt,
                                    "max_attempts": task.max_attempts,
                                    "retry_in_ms": backoff_ms,
                                    "error": err_text,
                                });
                                let _ = repo_rt::append_task_event(
                                    pool,
                                    session_id,
                                    Some(&task.id),
                                    "task_retry",
                                    &payload,
                                )
                                .await;
                                self.emit_event(session_id, "agent_team:task_retry", payload);
                                tokio::time::sleep(StdDuration::from_millis(
                                    backoff_ms * attempt as u64,
                                ))
                                .await;
                                Ok(())
                            }
                            Err(err) => Err(err),
                        }
                    } else {
                        match repo_rt::update_task(
                            pool,
                            session_id,
                            &UpdateTaskRequest {
                                task_id: task_id.clone(),
                                status: Some("failed".to_string()),
                                assignee_agent_id: Some(member_id.clone()),
                                last_error: Some(err_text.clone()),
                            },
                        )
                        .await
                        {
                            Ok(_) => {
                                let failed_payload = json!({
                                    "session_id": session_id,
                                    "task_id": task_id,
                                    "task_record_id": task.id,
                                    "assignee_agent_id": member_id,
                                    "attempt": attempt,
                                    "error": err_text,
                                });
                                let _ = repo_rt::append_task_event(
                                    pool,
                                    session_id,
                                    Some(&task.id),
                                    "task_failed",
                                    &failed_payload,
                                )
                                .await;
                                let _ = repo_rt::create_mailbox_message(
                                    pool,
                                    session_id,
                                    Some(member_id.as_str()),
                                    None,
                                    Some(task.id.as_str()),
                                    "task_failed",
                                    &failed_payload,
                                )
                                .await;
                                self.emit_event(session_id, "agent_team:task_failed", failed_payload);
                                Ok(())
                            }
                            Err(err) => Err(err),
                        }
                    }
                }
            };

            (task_id, member_id, result)
        })
    }

    fn build_task_node_map(runtime_spec_v2: &Value) -> HashMap<String, Value> {
        runtime_spec_v2
            .get("task_graph")
            .and_then(|v| v.get("nodes"))
            .and_then(|v| v.as_array())
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|node| {
                        let task_id = node.get("id").and_then(|v| v.as_str())?;
                        Some((task_id.to_string(), node.clone()))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default()
    }

    fn build_agent_concurrency_limits(
        &self,
        session: &AgentTeamSession,
        runtime_spec_v2: &Value,
    ) -> HashMap<String, usize> {
        let mut limits: HashMap<String, usize> = session
            .members
            .iter()
            .map(|m| (m.id.clone(), 1usize))
            .collect();

        if let Some(agents) = runtime_spec_v2.get("agents").and_then(|v| v.as_array()) {
            for agent in agents {
                let max_parallel = agent
                    .get("max_parallel_tasks")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1)
                    .clamp(1, 16) as usize;

                let mut selectors: Vec<String> = Vec::new();
                if let Some(id) = agent.get("id").and_then(|v| v.as_str()) {
                    selectors.push(id.to_string());
                }
                if let Some(name) = agent.get("name").and_then(|v| v.as_str()) {
                    selectors.push(name.to_string());
                }

                for selector in selectors {
                    if let Some(member) = self.resolve_member(session, &selector) {
                        limits.insert(member.id.clone(), max_parallel);
                        break;
                    }
                }
            }
        }
        limits
    }

    fn select_task_assignee<'a>(
        &self,
        session: &'a AgentTeamSession,
        task: &TeamTask,
        node: Option<&Value>,
        in_flight_by_member: &HashMap<String, usize>,
        agent_limits: &HashMap<String, usize>,
    ) -> Option<&'a AgentTeamMember> {
        let mut selectors: Vec<String> = Vec::new();
        if let Some(selector) = task.assignee_agent_id.as_ref() {
            selectors.push(selector.to_string());
        }
        if let Some(selector) = node.and_then(Self::extract_task_assignee_selector) {
            selectors.push(selector);
        }

        for selector in selectors {
            if let Some(member) = self.resolve_member(session, &selector) {
                let used = *in_flight_by_member.get(&member.id).unwrap_or(&0);
                let limit = *agent_limits.get(&member.id).unwrap_or(&1);
                if used < limit {
                    return Some(member);
                }
            }
        }

        session
            .members
            .iter()
            .filter(|member| {
                let used = *in_flight_by_member.get(&member.id).unwrap_or(&0);
                let limit = *agent_limits.get(&member.id).unwrap_or(&1);
                used < limit
            })
            .min_by_key(|member| in_flight_by_member.get(&member.id).copied().unwrap_or(0))
    }

    fn extract_task_assignee_selector(node: &Value) -> Option<String> {
        let strategy = node.get("assignee_strategy")?;
        if let Some(s) = strategy.as_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
                return Some(trimmed.to_string());
            }
        }
        let strategy_obj = strategy.as_object()?;
        for key in ["agent_id", "agent_name"] {
            if let Some(v) = strategy_obj.get(key).and_then(|v| v.as_str()) {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        if let Some(fixed) = strategy_obj.get("fixed_agent").and_then(|v| v.as_object()) {
            for key in ["agent_id", "agent_name"] {
                if let Some(v) = fixed.get(key).and_then(|v| v.as_str()) {
                    let trimmed = v.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
        None
    }

    fn is_task_pending(status: &str) -> bool {
        status.eq_ignore_ascii_case("pending")
    }

    fn is_task_completed(status: &str) -> bool {
        status.eq_ignore_ascii_case("completed")
    }

    fn is_task_failed(status: &str) -> bool {
        status.eq_ignore_ascii_case("failed")
            || status.eq_ignore_ascii_case("blocked")
            || status.eq_ignore_ascii_case("cancelled")
    }

    fn is_task_terminal(status: &str) -> bool {
        Self::is_task_completed(status) || Self::is_task_failed(status)
    }

    async fn run_orchestration_agent_step(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        step_id: &str,
        step_obj: &serde_json::Map<String, Value>,
        round_number: i32,
    ) -> Result<bool> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        let phase = step_obj
            .get("phase")
            .and_then(|v| v.as_str())
            .unwrap_or("orchestrating");
        self.transition_state(session_id, Self::phase_to_state(phase))
            .await?;

        let member_selector = step_obj
            .get("member")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("orchestration step {} missing member", step_id))?;
        let member = self
            .resolve_member(session, member_selector)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "orchestration step {} member '{}' not found in session members",
                    step_id,
                    member_selector
                )
            })?;

        let step_label = step_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(step_id);
        let step_instruction = step_obj
            .get("instruction")
            .or_else(|| step_obj.get("prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let round = repo_rt::create_round(&pool, session_id, round_number, phase).await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({
                "round": round_number,
                "phase": phase,
                "step_id": step_id,
                "step_name": step_label,
                "member_id": member.id,
                "member_name": member.name,
            }),
        );
        if !phase.to_lowercase().starts_with("task") {
            self.emit_event(
                session_id,
                "agent_team:role_thinking",
                json!({
                    "member_id": member.id,
                    "member_name": member.name,
                    "phase": phase,
                    "step_id": step_id,
                }),
            );
        }

        let goal = session.goal.as_deref().unwrap_or("请讨论并制定方案");
        let llm_config = self.get_llm_config_for_member(Some(member)).await?;
        let step_prompt = self.build_orchestration_agent_prompt(
            goal,
            step_id,
            step_label,
            phase,
            round_number,
            member,
            step_instruction,
        );
        let shared_memory_prompt = self
            .build_shared_execution_memory_prompt(&pool, session_id, EXECUTION_MEMORY_PROMPT_CARDS)
            .await
            .unwrap_or_default();

        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member,
            session_goal: goal,
            round_task: &step_prompt,
            round_number,
            llm_config: &llm_config,
            blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: step_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            shared_execution_memory: Some(shared_memory_prompt.as_str()),
        })
        .await?;

        let (content, tool_calls) = self
            .invoke_llm_streaming(
                session_id,
                Some(member),
                phase,
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;
        let msg = repo_rt::create_message(
            &pool,
            session_id,
            Some(&round.id),
            Some(&member.id),
            Some(&member.name),
            "assistant",
            &content,
            None,
        )
        .await?;
        if let Some(tc) = &tool_calls {
            repo_rt::update_message_tool_calls(&pool, &msg.id, tc).await?;
        }
        self.summarize_and_persist_member_execution(
            &pool,
            session_id,
            goal,
            round_number,
            phase,
            member,
            &llm_config,
            &content,
            tool_calls.as_ref(),
        )
        .await;

        let (entry_type_str, entry_type_enum) = Self::phase_to_blackboard_entry(phase);
        let entry_title = format!("{} · {}", member.name, step_label);
        let entry_content = shorten_text(&content, 500);
        self.blackboard
            .append_entry(
                session_id,
                entry_type_enum,
                &entry_title,
                &entry_content,
                Some(&member.name),
                &round.id,
            )
            .await;
        repo_rt::upsert_blackboard_entry(
            &pool,
            &UpdateBlackboardRequest {
                session_id: session_id.to_string(),
                entry_type: entry_type_str.to_string(),
                title: entry_title,
                content: entry_content,
                contributed_by: Some(member.name.clone()),
                round_id: Some(round.id.clone()),
            },
        )
        .await?;

        repo_rt::complete_round(&pool, &round.id, None).await?;
        self.emit_event(
            session_id,
            "agent_team:round_completed",
            json!({
                "round": round_number,
                "phase": phase,
                "step_id": step_id,
                "step_name": step_label,
                "member_id": member.id,
                "member_name": member.name,
            }),
        );

        Ok(false)
    }


    fn resolve_member<'a>(
        &self,
        session: &'a AgentTeamSession,
        selector: &str,
    ) -> Option<&'a AgentTeamMember> {
        let normalized = selector.trim().to_lowercase();
        session.members.iter().find(|m| {
            m.id.eq_ignore_ascii_case(selector) || m.name.trim().to_lowercase() == normalized
        })
    }

    fn phase_to_state(phase: &str) -> TeamSessionState {
        let lower = phase.trim().to_lowercase();
        if lower.contains("propos") || lower.contains("draft") {
            TeamSessionState::Proposing
        } else if lower.contains("challeng") || lower.contains("review") || lower.contains("audit")
        {
            TeamSessionState::Challenging
        } else if lower.contains("decid") || lower.contains("merge") {
            TeamSessionState::Deciding
        } else {
            TeamSessionState::Revising
        }
    }

    fn phase_to_blackboard_entry(phase: &str) -> (&'static str, BlackboardEntryType) {
        let lower = phase.trim().to_lowercase();
        if lower.contains("challeng") || lower.contains("review") || lower.contains("audit") {
            ("dispute", BlackboardEntryType::Dispute)
        } else if lower.contains("decid") || lower.contains("merge") {
            ("consensus", BlackboardEntryType::Consensus)
        } else {
            ("action_item", BlackboardEntryType::ActionItem)
        }
    }

    fn build_orchestration_agent_prompt(
        &self,
        goal: &str,
        step_id: &str,
        step_label: &str,
        phase: &str,
        round_number: i32,
        member: &AgentTeamMember,
        step_instruction: &str,
    ) -> String {
        format!(
            "你正在执行 Team 工作流节点，请严格围绕当前节点任务输出。\n\
             - 会话目标: {}\n\
             - 节点ID: {}\n\
             - 节点名称: {}\n\
             - 执行阶段: {}\n\
             - 节点序号: {}\n\
             - 你的角色: {}\n\
             - 角色职责: {}\n\
             - 节点额外要求: {}\n\n\
             输出要求:\n\
             1) 本节点可执行结论\n\
             2) 关键依据（含风险/假设/边界）\n\
             3) 对后续节点的交接信息（避免重复劳动）",
            goal,
            step_id,
            step_label,
            phase,
            round_number,
            member.name,
            member.responsibility.as_deref().unwrap_or(""),
            if step_instruction.trim().is_empty() {
                "无"
            } else {
                step_instruction
            },
        )
    }

    /// 生成标准文档产物（PRD / Architecture）
    async fn generate_artifacts(&self, session_id: &str, session: &AgentTeamSession) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::ArtifactGeneration)
            .await?;

        let blackboard_summary = self.blackboard.get_context_summary(session_id).await;
        let messages = repo_rt::get_messages(&pool, session_id).await?;
        let goal = session.goal.as_deref().unwrap_or("");

        // 构建上下文摘要
        let discussion_summary = messages
            .iter()
            .filter(|m| m.role == "assistant")
            .map(|m| {
                format!(
                    "[{}]: {}",
                    m.member_name.as_deref().unwrap_or("系统"),
                    shorten_text(&m.content, 300)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let artifact_prompt = format!(
            "请基于以下团队讨论结果，生成一份标准的 **架构设计文档（Architecture.md）**\n\n\
            **会话目标**: {}\n\n\
            **团队讨论摘要**:\n{}\n\n\
            **白板状态**:\n{}\n\n\
            请严格按照以下 Markdown 格式输出：\n\
            # 架构设计文档\n\
            ## 1. 概述\n\
            ## 2. 系统架构\n\
            ## 3. 核心模块\n\
            ## 4. 数据模型\n\
            ## 5. API 设计\n\
            ## 6. 安全考量\n\
            ## 7. 测试策略\n\
            ## 8. 风险与对策\n",
            goal, discussion_summary, blackboard_summary
        );

        // 使用架构师角色（第2个成员）生成架构文档
        let architect = session.members.get(1).unwrap_or(&session.members[0]);
        let llm_config = self.get_llm_config_for_member(Some(architect)).await?;
        let shared_memory_prompt = self
            .build_shared_execution_memory_prompt(&pool, session_id, EXECUTION_MEMORY_PROMPT_CARDS)
            .await
            .unwrap_or_default();
        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member: architect,
            session_goal: goal,
            round_task: &artifact_prompt,
            round_number: 99,
            llm_config: &llm_config,
            blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: artifact_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            shared_execution_memory: Some(shared_memory_prompt.as_str()),
        })
        .await?;

        let (artifact_content, _artifact_tool_calls) = self
            .invoke_llm_streaming(
                session_id,
                Some(architect),
                "artifact_generation",
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;
        self.summarize_and_persist_member_execution(
            &pool,
            session_id,
            goal,
            99,
            "artifact_generation",
            architect,
            &llm_config,
            &artifact_content,
            None,
        )
        .await;

        repo_rt::create_artifact(
            &pool,
            session_id,
            "architecture",
            "架构设计文档",
            &artifact_content,
            Some(&architect.name),
            None,
            None,
        )
        .await?;

        self.emit_event(
            session_id,
            "agent_team:artifact_generated",
            json!({
                "session_id": session_id,
                "artifact_type": "architecture",
                "title": "架构设计文档"
            }),
        );

        info!("Artifact generation completed for session: {}", session_id);
        Ok(())
    }

    // ==================== 辅助方法 ====================

    /// 状态机转换
    async fn transition_state(&self, session_id: &str, new_state: TeamSessionState) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        let state_str = new_state.to_string();
        repo_rt::update_session_state(&pool, session_id, &state_str).await?;

        self.emit_event(
            session_id,
            "agent_team:state_changed",
            json!({"session_id": session_id, "state": &state_str}),
        );

        info!("Session {} state -> {}", session_id, state_str);
        Ok(())
    }

    /// 发送 Tauri 事件
    fn emit_event(&self, _session_id: &str, event: &str, payload: serde_json::Value) {
        let _ = self.app_handle.emit(event, &payload);
    }

    async fn summarize_and_persist_member_execution(
        &self,
        pool: &DatabasePool,
        session_id: &str,
        session_goal: &str,
        round_number: i32,
        phase: &str,
        member: &AgentTeamMember,
        llm_config: &LlmConfig,
        assistant_output: &str,
        tool_calls: Option<&Value>,
    ) {
        let card = match self
            .summarize_execution_with_llm(
                session_id,
                session_goal,
                round_number,
                phase,
                member,
                llm_config,
                assistant_output,
                tool_calls,
            )
            .await
        {
            Ok(card) => card,
            Err(e) => {
                warn!(
                    "Failed to summarize execution for member '{}' ({}), fallback to deterministic card: {}",
                    member.name, phase, e
                );
                self.build_fallback_execution_memory_card(
                    session_id,
                    session_goal,
                    round_number,
                    phase,
                    member,
                    assistant_output,
                    tool_calls,
                )
            }
        };

        if let Err(e) = self
            .append_execution_memory_card(pool, session_id, card)
            .await
        {
            warn!(
                "Failed to persist execution memory card for member '{}' in session '{}': {}",
                member.name, session_id, e
            );
        }
    }

    async fn summarize_execution_with_llm(
        &self,
        session_id: &str,
        session_goal: &str,
        round_number: i32,
        phase: &str,
        member: &AgentTeamMember,
        llm_config: &LlmConfig,
        assistant_output: &str,
        tool_calls: Option<&Value>,
    ) -> Result<Value> {
        let tool_calls_value = tool_calls.cloned().unwrap_or_else(|| json!([]));
        let summarizer_system_prompt =
            "You summarize one role execution into a strict JSON object for multi-agent handoff. \
Return JSON only, no markdown, no extra text. \
Be faithful to evidence from provided tool calls and assistant output. \
Never invent facts. Use concise, generic task-agnostic language.";
        let summarizer_user_prompt = format!(
            "Create ONE JSON object with this exact shape:\n\
{{\n\
  \"task_scope\": \"string\",\n\
  \"actions\": [{{\"intent\":\"string\",\"status\":\"success|failed|skipped\",\"inputs\":\"string\",\"outputs\":\"string\",\"evidence_refs\":[\"string\"],\"cost\":{{\"time_ms\":number,\"tokens\":number}}}}],\n\
  \"findings\": [{{\"type\":\"fact|risk|opportunity|anomaly\",\"statement\":\"string\",\"confidence\":number,\"evidence_refs\":[\"string\"]}}],\n\
  \"decisions\": [{{\"decision\":\"string\",\"rationale\":\"string\",\"impact\":\"string\"}}],\n\
  \"state_updates\": {{\"entities\":[\"string\"],\"artifacts\":[\"string\"]}},\n\
  \"blockers\": [{{\"issue\":\"string\",\"needed\":\"string\"}}],\n\
  \"handoff\": {{\"next_best_actions\":[\"string\"],\"avoid_rework\":[\"string\"]}}\n\
}}\n\
\n\
Rules:\n\
- Keep arrays short and high-signal.\n\
- If evidence is missing, reduce confidence and mark as risk/anomaly.\n\
- evidence_refs should reference tool call ids when available.\n\
\n\
Input:\n\
- session_goal: {}\n\
- round_number: {}\n\
- phase: {}\n\
- member_name: {}\n\
- member_role: {}\n\
- assistant_output:\n{}\n\
\n\
- tool_calls_json:\n{}",
            session_goal,
            round_number,
            phase,
            member.name,
            member.responsibility.as_deref().unwrap_or(""),
            shorten_text(assistant_output, 3000),
            shorten_text(&tool_calls_value.to_string(), 6000),
        );

        let response = self
            .invoke_llm(
                llm_config,
                summarizer_system_prompt,
                &[ChatMessage {
                    role: "user".to_string(),
                    content: summarizer_user_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
            )
            .await?;

        let parsed = Self::parse_json_value_from_text(&response)
            .ok_or_else(|| anyhow::anyhow!("LLM summary is not valid JSON"))?;
        let normalized = self.normalize_execution_memory_card(
            session_id,
            session_goal,
            round_number,
            phase,
            member,
            parsed,
        );
        Ok(normalized)
    }

    fn normalize_execution_memory_card(
        &self,
        session_id: &str,
        session_goal: &str,
        round_number: i32,
        phase: &str,
        member: &AgentTeamMember,
        parsed: Value,
    ) -> Value {
        let mut obj = parsed.as_object().cloned().unwrap_or_default();

        obj.insert(
            "card_id".to_string(),
            json!(uuid::Uuid::new_v4().to_string()),
        );
        obj.insert("session_id".to_string(), json!(session_id));
        obj.insert("round".to_string(), json!(round_number));
        obj.insert("phase".to_string(), json!(phase));
        obj.insert("member_id".to_string(), json!(member.id.clone()));
        obj.insert("member_name".to_string(), json!(member.name.clone()));
        obj.insert(
            "created_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );

        if !obj
            .get("task_scope")
            .map(|v| v.is_string())
            .unwrap_or(false)
        {
            obj.insert(
                "task_scope".to_string(),
                json!(format!(
                    "Round {} {} execution for goal: {}",
                    round_number,
                    phase,
                    shorten_text(session_goal, 240)
                )),
            );
        }
        if !obj.get("actions").map(|v| v.is_array()).unwrap_or(false) {
            obj.insert("actions".to_string(), json!([]));
        }
        if !obj.get("findings").map(|v| v.is_array()).unwrap_or(false) {
            obj.insert("findings".to_string(), json!([]));
        }
        if !obj.get("decisions").map(|v| v.is_array()).unwrap_or(false) {
            obj.insert("decisions".to_string(), json!([]));
        }
        if !obj.get("blockers").map(|v| v.is_array()).unwrap_or(false) {
            obj.insert("blockers".to_string(), json!([]));
        }
        if !obj
            .get("state_updates")
            .map(|v| v.is_object())
            .unwrap_or(false)
        {
            obj.insert(
                "state_updates".to_string(),
                json!({
                    "entities": [],
                    "artifacts": [],
                }),
            );
        }
        if !obj.get("handoff").map(|v| v.is_object()).unwrap_or(false) {
            obj.insert(
                "handoff".to_string(),
                json!({
                    "next_best_actions": [],
                    "avoid_rework": [],
                }),
            );
        }

        Value::Object(obj)
    }

    fn build_fallback_execution_memory_card(
        &self,
        session_id: &str,
        session_goal: &str,
        round_number: i32,
        phase: &str,
        member: &AgentTeamMember,
        assistant_output: &str,
        tool_calls: Option<&Value>,
    ) -> Value {
        let calls = tool_calls
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let action_items: Vec<Value> = calls
            .iter()
            .take(6)
            .map(|call| {
                let name = call
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool");
                let id = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let success = call
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                json!({
                    "intent": format!("Invoke tool '{}'", name),
                    "status": if success { "success" } else { "failed" },
                    "inputs": shorten_text(&call.get("arguments").cloned().unwrap_or(Value::Null).to_string(), 240),
                    "outputs": shorten_text(&call.get("result").cloned().unwrap_or(Value::Null).to_string(), 240),
                    "evidence_refs": [id],
                    "cost": { "time_ms": 0, "tokens": 0 }
                })
            })
            .collect();

        json!({
            "card_id": uuid::Uuid::new_v4().to_string(),
            "session_id": session_id,
            "round": round_number,
            "phase": phase,
            "member_id": member.id,
            "member_name": member.name,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "task_scope": format!("Round {} {} execution for goal: {}", round_number, phase, shorten_text(session_goal, 240)),
            "actions": action_items,
            "findings": [{
                "type": "fact",
                "statement": shorten_text(assistant_output, 240),
                "confidence": 0.5,
                "evidence_refs": []
            }],
            "decisions": [],
            "state_updates": { "entities": [], "artifacts": [] },
            "blockers": [],
            "handoff": {
                "next_best_actions": [],
                "avoid_rework": calls.iter().filter_map(|c| {
                    let name = c.get("name").and_then(|v| v.as_str())?;
                    Some(format!("avoid duplicate tool call: {}", name))
                }).collect::<Vec<String>>()
            }
        })
    }

    async fn append_execution_memory_card(
        &self,
        pool: &DatabasePool,
        session_id: &str,
        card: Value,
    ) -> Result<()> {
        let session = repo_rt::get_agent_team_session(pool, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        let mut state_machine = session.state_machine.unwrap_or_else(|| json!({}));
        if !state_machine.is_object() {
            state_machine = json!({});
        }

        let state_obj = state_machine
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("state_machine must be object"))?;
        let memory_value = state_obj
            .entry("execution_memory".to_string())
            .or_insert_with(|| {
                json!({
                    "version": 1,
                    "cards": [],
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })
            });

        if !memory_value.is_object() {
            *memory_value = json!({
                "version": 1,
                "cards": [],
                "updated_at": chrono::Utc::now().to_rfc3339(),
            });
        }

        let memory_obj = memory_value
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("execution_memory must be object"))?;
        let cards_value = memory_obj
            .entry("cards".to_string())
            .or_insert_with(|| json!([]));
        if !cards_value.is_array() {
            *cards_value = json!([]);
        }
        let cards = cards_value
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("execution_memory.cards must be array"))?;
        cards.push(card);
        if cards.len() > EXECUTION_MEMORY_MAX_CARDS {
            let drop_count = cards.len() - EXECUTION_MEMORY_MAX_CARDS;
            cards.drain(0..drop_count);
        }
        memory_obj.insert(
            "updated_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );

        repo_rt::update_agent_team_session(
            pool,
            session_id,
            &UpdateAgentTeamSessionRequest {
                name: None,
                goal: None,
                state: None,
                max_rounds: None,
                schema_version: None,
                runtime_spec_v2: None,
                state_machine: Some(state_machine),
                error_message: None,
            },
        )
        .await?;

        Ok(())
    }

    async fn build_shared_execution_memory_prompt(
        &self,
        pool: &DatabasePool,
        session_id: &str,
        max_cards: usize,
    ) -> Result<String> {
        let session = repo_rt::get_agent_team_session(pool, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;
        let cards = session
            .state_machine
            .as_ref()
            .and_then(|s| s.get("execution_memory"))
            .and_then(|m| m.get("cards"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if cards.is_empty() {
            return Ok(String::new());
        }

        let keep = max_cards.max(1);
        let start = cards.len().saturating_sub(keep);
        let selected = &cards[start..];

        let mut lines: Vec<String> = Vec::new();
        for (idx, card) in selected.iter().enumerate() {
            let round = card
                .get("round")
                .and_then(|v| v.as_i64())
                .unwrap_or_default();
            let phase = card
                .get("phase")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let member = card
                .get("member_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let scope = card
                .get("task_scope")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            lines.push(format!(
                "{}. [R{}|{}|{}] {}",
                idx + 1,
                round,
                phase,
                member,
                shorten_text(scope, 200)
            ));

            if let Some(actions) = card.get("actions").and_then(|v| v.as_array()) {
                let action_summaries: Vec<String> = actions
                    .iter()
                    .take(2)
                    .filter_map(|a| {
                        let intent = a.get("intent").and_then(|v| v.as_str())?;
                        let status = a
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        Some(format!("{}({})", shorten_text(intent, 60), status))
                    })
                    .collect();
                if !action_summaries.is_empty() {
                    lines.push(format!("   actions: {}", action_summaries.join("; ")));
                }
            }

            if let Some(findings) = card.get("findings").and_then(|v| v.as_array()) {
                let finding_summaries: Vec<String> = findings
                    .iter()
                    .take(2)
                    .filter_map(|f| f.get("statement").and_then(|v| v.as_str()))
                    .map(|s| shorten_text(s, 120))
                    .collect();
                if !finding_summaries.is_empty() {
                    lines.push(format!("   findings: {}", finding_summaries.join(" | ")));
                }
            }

            if let Some(avoid) = card
                .get("handoff")
                .and_then(|v| v.get("avoid_rework"))
                .and_then(|v| v.as_array())
            {
                let avoid_items: Vec<String> = avoid
                    .iter()
                    .take(2)
                    .filter_map(|v| v.as_str())
                    .map(|s| shorten_text(s, 100))
                    .collect();
                if !avoid_items.is_empty() {
                    lines.push(format!("   do_not_repeat: {}", avoid_items.join(" | ")));
                }
            }
        }

        lines.push(
            "Use this memory to avoid duplicate actions and build on verified progress."
                .to_string(),
        );

        Ok(lines.join("\n"))
    }

    fn parse_json_value_from_text(text: &str) -> Option<Value> {
        if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
            return Some(v);
        }

        let fenced = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if let Ok(v) = serde_json::from_str::<Value>(fenced) {
            return Some(v);
        }

        let start = text.find('{')?;
        let end = text.rfind('}')?;
        if end <= start {
            return None;
        }
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    }

    /// 获取 LLM 配置（支持角色模型覆盖，未配置回退到全局默认）
    async fn get_llm_config_for_member(
        &self,
        member: Option<&AgentTeamMember>,
    ) -> Result<LlmConfig> {
        let ai_manager = self
            .app_handle
            .try_state::<Arc<crate::services::ai::AiServiceManager>>()
            .context("AiServiceManager not available")?;

        let role_override = member.and_then(extract_member_model_override);

        let (provider, model) = match role_override {
            Some((p, m)) => (p, m),
            None => ai_manager
                .get_default_llm_model()
                .await?
                .unwrap_or_else(|| ("openai".to_string(), "gpt-4o".to_string())),
        };

        let provider_cfg = ai_manager
            .get_provider_config(&provider)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Provider config not found for {}", provider))?;

        let api_key = provider_cfg.api_key.clone();
        if api_key.as_deref().unwrap_or("").trim().is_empty() {
            return Err(anyhow::anyhow!(
                "LLM API key is empty for provider '{}'. Please configure it in AI Settings.",
                provider
            ));
        }

        Ok(LlmConfig {
            provider: provider_cfg
                .rig_provider
                .clone()
                .unwrap_or(provider_cfg.provider.clone()),
            model,
            api_key,
            base_url: provider_cfg.api_base.clone(),
            rig_provider: provider_cfg.rig_provider.clone(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            timeout_secs: 120,
            max_turns: provider_cfg.max_turns,
            ..LlmConfig::default()
        })
    }

    /// 调用 LLM（使用 sentinel_llm LlmClient）
    async fn invoke_llm(
        &self,
        config: &LlmConfig,
        system_prompt: &str,
        history: &[ChatMessage],
    ) -> Result<String> {
        use sentinel_llm::LlmClient;

        // Setup API key env vars
        config.setup_env_vars();

        let client = LlmClient::new(config.clone());

        // 提取最后一条 user 消息作为本次提示词，其余作为历史
        let (user_prompt, hist) = if let Some(last) = history.last() {
            if last.role == "user" {
                (last.content.as_str(), &history[..history.len() - 1])
            } else {
                ("请继续分析。", history.as_ref())
            }
        } else {
            ("请开始分析。", history.as_ref())
        };

        client
            .chat(Some(system_prompt), user_prompt, hist, None)
            .await
    }

    /// 调用 LLM（流式，并通过事件增量推送到前端）
    async fn invoke_llm_streaming(
        &self,
        session_id: &str,
        member: Option<&AgentTeamMember>,
        phase: &str,
        config: &LlmConfig,
        system_prompt: &str,
        history: &[ChatMessage],
    ) -> Result<(String, Option<serde_json::Value>)> {
        let client = StreamingLlmClient::new(config.clone());
        let stream_id = uuid::Uuid::new_v4().to_string();
        let member_id = member.map(|m| m.id.clone());
        let member_name = member.map(|m| m.name.clone());

        self.emit_event(
            session_id,
            "agent_team:message_stream_start",
            json!({
                "session_id": session_id,
                "stream_id": &stream_id,
                "member_id": member_id.clone(),
                "member_name": member_name.clone(),
                "phase": phase
            }),
        );

        let (user_prompt, hist) = if let Some(last) = history.last() {
            if last.role == "user" {
                (last.content.as_str(), &history[..history.len() - 1])
            } else {
                ("请继续分析。", history)
            }
        } else {
            ("请开始分析。", history)
        };

        let mut emitted_any = false;
        let mut tool_calls_by_id: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut collected_tool_calls: Vec<serde_json::Value> = Vec::new();
        let execution_id = format!(
            "team:{}:{}:{}",
            session_id,
            member_id.clone().unwrap_or_else(|| "unknown".to_string()),
            stream_id
        );
        let dynamic_tools = self
            .resolve_dynamic_tools_for_member(member, &execution_id)
            .await;
        info!(
            "Team member '{}' available dynamic tools: {}",
            member_name.as_deref().unwrap_or("unknown"),
            dynamic_tools.len()
        );

        let content = client
            .stream_chat_with_dynamic_tools(
                Some(system_prompt),
                user_prompt,
                hist,
                None,
                dynamic_tools,
                |chunk| {
                    match chunk {
                        StreamContent::Text(piece) | StreamContent::Reasoning(piece) => {
                            if !piece.is_empty() {
                                emitted_any = true;
                                self.emit_event(
                                    session_id,
                                    "agent_team:message_stream_delta",
                                    json!({
                                        "session_id": session_id,
                                        "stream_id": &stream_id,
                                        "member_id": member_id.clone(),
                                        "member_name": member_name.clone(),
                                        "phase": phase,
                                        "delta": piece
                                    }),
                                );
                            }
                        }
                        StreamContent::ToolCallComplete {
                            id,
                            name,
                            arguments,
                        } => {
                            if !tool_calls_by_id.contains_key(&id) {
                                collected_tool_calls.push(json!({
                                    "id": id.clone(),
                                    "name": name.clone(),
                                    "arguments": arguments,
                                }));
                                let idx = collected_tool_calls.len() - 1;
                                tool_calls_by_id.insert(id.clone(), idx);
                            }
                            self.emit_event(
                                session_id,
                                "agent_team:tool_call",
                                json!({
                                    "session_id": session_id,
                                    "stream_id": &stream_id,
                                    "member_id": member_id.clone(),
                                    "member_name": member_name.clone(),
                                    "phase": phase,
                                    "tool_call_id": id.clone(),
                                    "name": name.clone(),
                                    "arguments": arguments.clone(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }),
                            );
                            if let Some(mid) = member_id.as_deref() {
                                if let Ok(mut gov) = self.tool_governance.lock() {
                                    gov.record_call(mid, &name);
                                }
                            }
                        }
                        StreamContent::ToolResult { id, result } => {
                            if let Some(idx) = tool_calls_by_id.get(&id).copied() {
                                if let Some(existing) = collected_tool_calls.get_mut(idx) {
                                    if let Some(obj) = existing.as_object_mut() {
                                        obj.insert("result".to_string(), json!(result.clone()));
                                        obj.insert("success".to_string(), json!(true));
                                    }
                                }
                            }
                            self.emit_event(
                                session_id,
                                "agent_team:tool_result",
                                json!({
                                    "session_id": session_id,
                                    "stream_id": &stream_id,
                                    "member_id": member_id.clone(),
                                    "member_name": member_name.clone(),
                                    "phase": phase,
                                    "tool_call_id": id,
                                    "result": result,
                                    "success": true,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }),
                            );
                        }
                        _ => {}
                    }
                    true
                },
            )
            .await;

        match content {
            Ok(text) => {
                self.emit_event(
                    session_id,
                    "agent_team:message_stream_done",
                    json!({
                        "session_id": session_id,
                        "stream_id": &stream_id,
                        "member_id": member_id.clone(),
                        "member_name": member_name.clone(),
                        "phase": phase,
                        "content": text,
                        "had_delta": emitted_any
                    }),
                );
                let tool_calls = if collected_tool_calls.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Array(collected_tool_calls))
                };
                Ok((text, tool_calls))
            }
            Err(e) => {
                self.emit_event(
                    session_id,
                    "agent_team:message_stream_done",
                    json!({
                        "session_id": session_id,
                        "stream_id": &stream_id,
                        "member_id": member_id.clone(),
                        "member_name": member_name.clone(),
                        "phase": phase,
                        "error": e.to_string(),
                        "had_delta": emitted_any
                    }),
                );
                Err(e)
            }
        }
    }

    /// 公开的 get_llm_config（供命令层调用）
    pub async fn get_llm_config_pub(&self) -> Result<LlmConfig> {
        self.get_llm_config_for_member(None).await
    }

    /// 公开的 invoke_llm（供命令层调用）
    pub async fn invoke_llm_pub(
        &self,
        config: &LlmConfig,
        system_prompt: &str,
        history: &[ChatMessage],
    ) -> Result<String> {
        self.invoke_llm(config, system_prompt, history).await
    }

    /// 基于角色策略筛选可用动态工具
    async fn resolve_dynamic_tools_for_member(
        &self,
        member: Option<&AgentTeamMember>,
        execution_id: &str,
    ) -> Vec<DynamicTool> {
        let Some(member) = member else {
            return vec![];
        };

        let server = get_tool_server();
        server.init_builtin_tools().await;

        let tools = server.list_tools().await;
        let mut selected_names: Vec<String> = Vec::new();

        if let Ok(gov) = self.tool_governance.lock() {
            for tool in tools {
                match gov.check_permission(&member.id, &tool.name) {
                    super::scheduler::ToolPermissionResult::Allowed => {
                        selected_names.push(tool.name);
                    }
                    super::scheduler::ToolPermissionResult::NeedsApproval => {
                        // 当前版本无逐次审批流，默认禁用高危工具
                    }
                    super::scheduler::ToolPermissionResult::Denied(_) => {}
                }
            }
        }

        let mut dynamic_tools = server.get_dynamic_tools(&selected_names).await;

        // Mirror AI assistant behavior: inject execution_id into shell calls so
        // runtime mode/config is applied consistently for team sessions.
        if selected_names.iter().any(|name| name == ShellTool::NAME) {
            if let Some(shell_info) = server.get_tool(ShellTool::NAME).await {
                let execution_id_for_shell = execution_id.to_string();
                let shell_input_schema = shell_info.input_schema.clone();
                let shell_description = shell_info.description.clone();
                let shell_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
                    let execution_id_for_shell = execution_id_for_shell.clone();
                    Box::pin(async move {
                        use rig::tool::Tool;
                        use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellTool};

                        let mut patched_args = args;
                        if let Some(obj) = patched_args.as_object_mut() {
                            obj.insert(
                                "execution_id".to_string(),
                                serde_json::Value::String(execution_id_for_shell.clone()),
                            );
                        }

                        let tool_args: ShellArgs = serde_json::from_value(patched_args)
                            .map_err(|e| format!("Invalid arguments: {}", e))?;

                        let tool = ShellTool::new();
                        let result = tool
                            .call(tool_args)
                            .await
                            .map_err(|e| format!("Shell execution failed: {}", e))?;

                        serde_json::to_value(result)
                            .map_err(|e| format!("Failed to serialize shell result: {}", e))
                    })
                });

                let shell_def = DynamicToolDef {
                    name: ShellTool::NAME.to_string(),
                    description: shell_description,
                    input_schema: shell_input_schema,
                    output_schema: None,
                    source: ToolSource::Builtin,
                    category: "system".to_string(),
                    executor: shell_executor,
                };

                dynamic_tools = dynamic_tools
                    .into_iter()
                    .map(|tool| {
                        if tool.name() == ShellTool::NAME {
                            DynamicTool::new(shell_def.clone())
                        } else {
                            tool
                        }
                    })
                    .collect();
            }
        }

        dynamic_tools
    }
} // end impl AgentTeamEngine

// ==================== 全局引擎管理 ====================

use std::sync::OnceLock;
use tokio::sync::RwLock;

static RUNNING_SESSIONS: OnceLock<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>> =
    OnceLock::new();

fn get_running_sessions() -> &'static RwLock<HashMap<String, tokio::task::JoinHandle<()>>> {
    RUNNING_SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 启动 Team 运行（异步，后台执行）
pub async fn start_agent_team_run_async(app_handle: AppHandle, session_id: String) -> Result<()> {
    let sessions = get_running_sessions();
    let is_running = {
        let mut lock = sessions.write().await;
        if let Some(handle) = lock.get(&session_id) {
            if handle.is_finished() {
                lock.remove(&session_id);
            }
        }
        lock.contains_key(&session_id)
    };

    if is_running {
        return Err(anyhow::anyhow!("Session {} is already running", session_id));
    }

    let session_id_clone = session_id.clone();
    let handle = tokio::spawn(async move {
        let app_for_recovery = app_handle.clone();
        let engine = AgentTeamEngine::new(app_handle);
        if let Err(e) = engine.start_run(&session_id_clone).await {
            error!(
                "Agent Team run failed for session {}: {:#}",
                session_id_clone, e
            );
            if let Some(db) = app_for_recovery.try_state::<Arc<DatabaseService>>() {
                if let Ok(pool) = db.get_runtime_pool() {
                    let _ = repo_rt::update_session_state(&pool, &session_id_clone, "FAILED").await;
                    let _ = repo_rt::update_agent_team_session(
                        &pool,
                        &session_id_clone,
                        &UpdateAgentTeamSessionRequest {
                            name: None,
                            goal: None,
                            state: Some("FAILED".to_string()),
                            max_rounds: None,
                            schema_version: None,
                            runtime_spec_v2: None,
                            state_machine: None,
                            error_message: Some(e.to_string()),
                        },
                    )
                    .await;
                }
            }
        }
        // Always cleanup running handle record after run exits.
        let mut lock = get_running_sessions().write().await;
        lock.remove(&session_id_clone);
    });

    let mut lock = sessions.write().await;
    lock.insert(session_id, handle);

    Ok(())
}

/// 停止 Team 运行（中断后台任务）
pub async fn stop_agent_team_run_async(session_id: &str) -> Result<bool> {
    let sessions = get_running_sessions();
    let handle_opt = {
        let mut lock = sessions.write().await;
        lock.remove(session_id)
    };

    if let Some(handle) = handle_opt {
        handle.abort();
        return Ok(true);
    }
    Ok(false)
}

// ==================== 工具函数 ====================

/// 截断文本
fn shorten_text(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}...", chars[..max_chars].iter().collect::<String>())
    }
}

fn extract_member_model_override(member: &AgentTeamMember) -> Option<(String, String)> {
    // Preferred: output_schema = { model_provider, model_name }
    if let Some(schema) = &member.output_schema {
        let provider = schema
            .get("model_provider")
            .or_else(|| schema.get("provider"))
            .or_else(|| schema.get("llm_provider"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let model = schema
            .get("model_name")
            .or_else(|| schema.get("model"))
            .or_else(|| schema.get("llm_model_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let (Some(p), Some(m)) = (provider, model) {
            return Some((p.to_lowercase(), m));
        }

        // Compatible: output_schema = { llm_model: "provider/model" }
        if let Some(full) = schema.get("llm_model").and_then(|v| v.as_str()) {
            if let Some((p, m)) = full.split_once('/') {
                let p = p.trim();
                let m = m.trim();
                if !p.is_empty() && !m.is_empty() {
                    return Some((p.to_lowercase(), m.to_string()));
                }
            }
        }
    }
    None
}
