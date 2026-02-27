//! Agent Team 核心引擎 - 轮次编排与状态机流转

use anyhow::{Context, Result};
use futures::future::{join_all, BoxFuture};
use serde_json::{json, Value};
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
    buildin_tools::{ShellTool, SkillsTool},
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

        let resume_from_step_id = session
            .state_machine
            .as_ref()
            .and_then(|sm| sm.get("orchestration_runtime"))
            .and_then(|rt| rt.get("resume_from_step_id"))
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        if let Some(step_id) = resume_from_step_id.as_deref() {
            if let Err(e) = self
                .clear_orchestration_resume_marker(session_id, &session, step_id)
                .await
            {
                warn!(
                    "Failed to clear orchestration resume marker for session {}: {}",
                    session_id, e
                );
            }
        }

        let plan = if let Some(plan) = session.orchestration_plan.as_ref() {
            plan
        } else {
            let err = anyhow::anyhow!(
                "Session {} missing orchestration_plan; workflow orchestration is required",
                session_id
            );
            self.emit_event(
                session_id,
                "agent_team:orchestration_required",
                json!({
                    "session_id": session_id,
                    "error": err.to_string()
                }),
            );
            return Err(err);
        };

        let suspended = match self
            .execute_orchestration_plan(
                session_id,
                &session,
                plan,
                resume_from_step_id.as_deref(),
            )
            .await
        {
            Ok(suspended) => suspended,
            Err(e) => {
                let err = anyhow::anyhow!(
                    "Failed to execute orchestration plan for session {}: {}",
                    session_id,
                    e
                );
                warn!("{:#}", err);
                self.emit_event(
                    session_id,
                    "agent_team:orchestration_failed",
                    json!({
                        "session_id": session_id,
                        "error": err.to_string()
                    }),
                );
                return Err(err);
            }
        };
        if suspended {
            // 等待 Human-in-the-Loop 恢复（resume 由 Tauri command 重入）
            return Ok(());
        }

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

    async fn execute_orchestration_plan(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        plan: &Value,
        resume_from_step_id: Option<&str>,
    ) -> Result<bool> {
        let steps = plan
            .get("steps")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("orchestration_plan.steps must be array"))?;
        if steps.is_empty() {
            return Err(anyhow::anyhow!("orchestration_plan.steps cannot be empty"));
        }

        let round_counter = Arc::new(AtomicI32::new(1));
        self.transition_state(session_id, TeamSessionState::Revising)
            .await?;
        self.emit_event(
            session_id,
            "agent_team:orchestration_started",
            json!({
                "session_id": session_id,
                "steps": steps.len(),
                "version": plan.get("version").and_then(|v| v.as_i64()).unwrap_or(1),
                "resume_from_step_id": resume_from_step_id
            }),
        );

        let resume_path = if let Some(step_id) = resume_from_step_id {
            if let Some(path) = Self::find_orchestration_step_path(steps, step_id) {
                self.emit_event(
                    session_id,
                    "agent_team:orchestration_resume_applied",
                    json!({
                        "session_id": session_id,
                        "resume_from_step_id": step_id,
                        "resume_path": path,
                    }),
                );
                Some(path)
            } else {
                warn!(
                    "resume_from_step_id '{}' not found in orchestration plan for session {}, fallback to full run",
                    step_id, session_id
                );
                self.emit_event(
                    session_id,
                    "agent_team:orchestration_resume_ignored",
                    json!({
                        "session_id": session_id,
                        "resume_from_step_id": step_id,
                        "reason": "step_not_found"
                    }),
                );
                None
            }
        } else {
            None
        };

        self.execute_orchestration_steps(
            session_id,
            session,
            steps,
            round_counter,
            Vec::new(),
            resume_path.as_deref(),
        )
        .await
    }

    fn execute_orchestration_steps<'a>(
        &'a self,
        session_id: &'a str,
        session: &'a AgentTeamSession,
        steps: &'a [Value],
        round_counter: Arc<AtomicI32>,
        path_prefix: Vec<usize>,
        resume_path: Option<&'a [usize]>,
    ) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move {
            let start_index = Self::resolve_resume_start_index(resume_path, &path_prefix).unwrap_or(0);
            let mut suspended = false;
            for idx in start_index..steps.len() {
                let step = &steps[idx];
                let mut current_path = path_prefix.clone();
                current_path.push(idx);
                let path = Self::format_orchestration_path(&current_path);
                let step_suspended = self
                    .execute_orchestration_step(
                        session_id,
                        session,
                        step,
                        round_counter.clone(),
                        path,
                        current_path,
                        resume_path,
                    )
                    .await?;
                if step_suspended {
                    suspended = true;
                    break;
                }
            }
            Ok(suspended)
        })
    }

    fn resolve_resume_start_index(
        resume_path: Option<&[usize]>,
        prefix: &[usize],
    ) -> Option<usize> {
        let path = resume_path?;
        if path.len() <= prefix.len() {
            return None;
        }
        if !path.starts_with(prefix) {
            return None;
        }
        Some(path[prefix.len()])
    }

    fn format_orchestration_path(indices: &[usize]) -> String {
        let mut path = String::new();
        for (depth, index) in indices.iter().enumerate() {
            if depth == 0 {
                path.push_str(&format!("steps[{}]", index));
            } else {
                path.push_str(&format!(".children[{}]", index));
            }
        }
        path
    }

    fn find_orchestration_step_path(steps: &[Value], target_step_id: &str) -> Option<Vec<usize>> {
        fn walk(
            steps: &[Value],
            target_step_id: &str,
            prefix: &mut Vec<usize>,
        ) -> Option<Vec<usize>> {
            for (idx, step) in steps.iter().enumerate() {
                prefix.push(idx);
                let matched = step
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|id| id == target_step_id)
                    .unwrap_or(false);
                if matched {
                    return Some(prefix.clone());
                }
                if let Some(children) = step.get("children").and_then(|v| v.as_array()) {
                    if let Some(found) = walk(children, target_step_id, prefix) {
                        return Some(found);
                    }
                }
                prefix.pop();
            }
            None
        }

        walk(steps, target_step_id, &mut Vec::new())
    }

    fn execute_orchestration_step<'a>(
        &'a self,
        session_id: &'a str,
        session: &'a AgentTeamSession,
        step: &'a Value,
        round_counter: Arc<AtomicI32>,
        path: String,
        path_indices: Vec<usize>,
        resume_path: Option<&'a [usize]>,
    ) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move {
            let step_obj = step
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("orchestration step {} must be object", path))?;
            let step_id = step_obj
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("orchestration step {} missing id", path))?;
            let step_type = step_obj
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("orchestration step {} missing type", path))?;

            match step_type {
                "agent" => {
                    self.run_orchestration_agent_step_with_retry(
                        session_id,
                        session,
                        step_id,
                        step_obj,
                        round_counter,
                    )
                    .await
                }
                "serial" => {
                    let children = step_obj
                        .get("children")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "orchestration step {} (id={}) missing children",
                                path,
                                step_id
                            )
                        })?;
                    self.execute_orchestration_steps(
                        session_id,
                        session,
                        children,
                        round_counter.clone(),
                        path_indices.clone(),
                        resume_path,
                    )
                    .await
                }
                "parallel" => {
                    let children = step_obj
                        .get("children")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "orchestration step {} (id={}) missing children",
                                path,
                                step_id
                            )
                        })?;
                    let child_start =
                        Self::resolve_resume_start_index(resume_path, &path_indices).unwrap_or(0);
                    let futures = children
                        .iter()
                        .enumerate()
                        .skip(child_start)
                        .map(|(idx, child)| {
                        let child_path = format!("{}.children[{}]", path, idx);
                        let mut child_indices = path_indices.clone();
                        child_indices.push(idx);
                        self.execute_orchestration_step(
                            session_id,
                            session,
                            child,
                            round_counter.clone(),
                            child_path,
                            child_indices,
                            resume_path,
                        )
                    });
                    let results = join_all(futures).await;
                    let mut suspended = false;
                    for result in results {
                        if result? {
                            suspended = true;
                        }
                    }
                    Ok(suspended)
                }
                other => Err(anyhow::anyhow!(
                    "orchestration step {} has unsupported type '{}'",
                    path,
                    other
                )),
            }
        })
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

    async fn run_orchestration_agent_step_with_retry(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        step_id: &str,
        step_obj: &serde_json::Map<String, Value>,
        round_counter: Arc<AtomicI32>,
    ) -> Result<bool> {
        let max_attempts = step_obj
            .get("retry")
            .and_then(|v| v.get("max_attempts"))
            .or_else(|| step_obj.get("retry_max_attempts"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .clamp(1, 5) as i32;
        let backoff_ms = step_obj
            .get("retry")
            .and_then(|v| v.get("backoff_ms"))
            .or_else(|| step_obj.get("retry_backoff_ms"))
            .and_then(|v| v.as_i64())
            .unwrap_or(800)
            .clamp(100, 10_000) as u64;

        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        let mut last_error: Option<anyhow::Error> = None;
        for attempt in 1..=max_attempts {
            let round_number = round_counter.fetch_add(1, Ordering::SeqCst);
            let started_at = Instant::now();
            match self
                .run_orchestration_agent_step(session_id, session, step_id, step_obj, round_number)
                .await
            {
                Ok(suspended) => {
                    let duration_ms = started_at.elapsed().as_millis() as i64;
                    self.persist_orchestration_checkpoint(
                        &pool,
                        session_id,
                        step_id,
                        "succeeded",
                        attempt,
                        round_number,
                        Some(duration_ms),
                        None,
                    )
                    .await;
                    return Ok(suspended);
                }
                Err(e) => {
                    let duration_ms = started_at.elapsed().as_millis() as i64;
                    let err_text = e.to_string();
                    self.persist_orchestration_checkpoint(
                        &pool,
                        session_id,
                        step_id,
                        "failed",
                        attempt,
                        round_number,
                        Some(duration_ms),
                        Some(err_text.as_str()),
                    )
                    .await;
                    if attempt < max_attempts {
                        warn!(
                            "Orchestration step {} failed on attempt {}/{} for session {}: {}",
                            step_id, attempt, max_attempts, session_id, err_text
                        );
                        self.emit_event(
                            session_id,
                            "agent_team:orchestration_step_retry",
                            json!({
                                "session_id": session_id,
                                "step_id": step_id,
                                "attempt": attempt,
                                "max_attempts": max_attempts,
                                "next_retry_in_ms": backoff_ms,
                                "error": err_text,
                            }),
                        );
                        tokio::time::sleep(StdDuration::from_millis(backoff_ms * attempt as u64))
                            .await;
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "orchestration step {} failed after retries for session {}",
                step_id,
                session_id
            )
        }))
    }

    async fn clear_orchestration_resume_marker(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        consumed_step_id: &str,
    ) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        let mut state_machine = session.state_machine.clone().unwrap_or_else(|| json!({}));
        if !state_machine.is_object() {
            state_machine = json!({});
        }
        let state_obj = state_machine
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("state_machine must be object"))?;
        let runtime_value = state_obj
            .entry("orchestration_runtime".to_string())
            .or_insert_with(|| json!({}));
        if !runtime_value.is_object() {
            *runtime_value = json!({});
        }
        let runtime_obj = runtime_value
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("orchestration_runtime must be object"))?;
        runtime_obj.remove("resume_from_step_id");
        runtime_obj.insert(
            "resume_consumed_step_id".to_string(),
            json!(consumed_step_id.to_string()),
        );
        runtime_obj.insert(
            "resume_consumed_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );

        repo_rt::update_agent_team_session(
            &pool,
            session_id,
            &UpdateAgentTeamSessionRequest {
                name: None,
                goal: None,
                state: None,
                max_rounds: None,
                orchestration_plan: None,
                plan_version: None,
                state_machine: Some(state_machine),
                error_message: None,
            },
        )
        .await?;

        Ok(())
    }

    async fn persist_orchestration_checkpoint(
        &self,
        pool: &DatabasePool,
        session_id: &str,
        step_id: &str,
        status: &str,
        attempt: i32,
        round_number: i32,
        duration_ms: Option<i64>,
        error: Option<&str>,
    ) {
        let session = match repo_rt::get_agent_team_session(pool, session_id).await {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(e) => {
                warn!(
                    "Failed to persist orchestration checkpoint for session {}: {}",
                    session_id, e
                );
                return;
            }
        };

        let mut state_machine = session.state_machine.unwrap_or_else(|| json!({}));
        if !state_machine.is_object() {
            state_machine = json!({});
        }
        let Some(state_obj) = state_machine.as_object_mut() else {
            return;
        };
        let runtime_value = state_obj
            .entry("orchestration_runtime".to_string())
            .or_insert_with(|| json!({}));
        if !runtime_value.is_object() {
            *runtime_value = json!({});
        }

        if let Some(runtime_obj) = runtime_value.as_object_mut() {
            runtime_obj.insert("last_step_id".to_string(), json!(step_id));
            runtime_obj.insert("last_step_status".to_string(), json!(status));
            runtime_obj.insert("last_attempt".to_string(), json!(attempt));
            runtime_obj.insert("last_round".to_string(), json!(round_number));
            if let Some(ms) = duration_ms {
                runtime_obj.insert("last_duration_ms".to_string(), json!(ms));
            }
            runtime_obj.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            if let Some(err) = error {
                runtime_obj.insert("last_error".to_string(), json!(shorten_text(err, 500)));
            } else {
                runtime_obj.remove("last_error");
            }
            self.update_orchestration_runtime_stats(
                runtime_obj,
                step_id,
                status,
                attempt,
                round_number,
                duration_ms,
                error,
            );
        }

        if let Err(e) = repo_rt::update_agent_team_session(
            pool,
            session_id,
            &UpdateAgentTeamSessionRequest {
                name: None,
                goal: None,
                state: None,
                max_rounds: None,
                orchestration_plan: None,
                plan_version: None,
                state_machine: Some(state_machine),
                error_message: None,
            },
        )
        .await
        {
            warn!(
                "Failed to update session state_machine checkpoint for session {}: {}",
                session_id, e
            );
        }
    }

    fn update_orchestration_runtime_stats(
        &self,
        runtime_obj: &mut serde_json::Map<String, Value>,
        step_id: &str,
        status: &str,
        attempt: i32,
        round_number: i32,
        duration_ms: Option<i64>,
        error: Option<&str>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();

        let mark_last_success = status == "succeeded";
        let mark_suggested_resume = status == "failed";
        {
            let step_stats_value = runtime_obj
                .entry("step_stats".to_string())
                .or_insert_with(|| json!({}));
            if !step_stats_value.is_object() {
                *step_stats_value = json!({});
            }
            let Some(step_stats_obj) = step_stats_value.as_object_mut() else {
                return;
            };
            let step_stat_value = step_stats_obj.entry(step_id.to_string()).or_insert_with(|| {
                json!({
                    "step_id": step_id,
                    "total_attempts": 0,
                    "success_count": 0,
                    "failure_count": 0,
                    "total_duration_ms": 0,
                    "avg_duration_ms": 0,
                })
            });
            if !step_stat_value.is_object() {
                *step_stat_value = json!({
                    "step_id": step_id,
                    "total_attempts": 0,
                    "success_count": 0,
                    "failure_count": 0,
                    "total_duration_ms": 0,
                    "avg_duration_ms": 0,
                });
            }
            let Some(step_stat_obj) = step_stat_value.as_object_mut() else {
                return;
            };

            let mut total_attempts = step_stat_obj
                .get("total_attempts")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let mut success_count = step_stat_obj
                .get("success_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let mut failure_count = step_stat_obj
                .get("failure_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let mut total_duration_ms = step_stat_obj
                .get("total_duration_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);

            total_attempts += 1;
            if mark_last_success {
                success_count += 1;
            } else if mark_suggested_resume {
                failure_count += 1;
            }
            if let Some(ms) = duration_ms {
                total_duration_ms += ms.max(0);
                step_stat_obj.insert("last_duration_ms".to_string(), json!(ms.max(0)));
            }

            let avg_duration_ms = if total_attempts > 0 {
                total_duration_ms / total_attempts
            } else {
                0
            };
            step_stat_obj.insert("step_id".to_string(), json!(step_id));
            step_stat_obj.insert("total_attempts".to_string(), json!(total_attempts));
            step_stat_obj.insert("success_count".to_string(), json!(success_count));
            step_stat_obj.insert("failure_count".to_string(), json!(failure_count));
            step_stat_obj.insert("total_duration_ms".to_string(), json!(total_duration_ms));
            step_stat_obj.insert("avg_duration_ms".to_string(), json!(avg_duration_ms));
            step_stat_obj.insert("last_status".to_string(), json!(status));
            step_stat_obj.insert("last_attempt".to_string(), json!(attempt));
            step_stat_obj.insert("last_round".to_string(), json!(round_number));
            step_stat_obj.insert("updated_at".to_string(), json!(now.clone()));
            if let Some(err) = error {
                step_stat_obj.insert("last_error".to_string(), json!(shorten_text(err, 500)));
            } else {
                step_stat_obj.remove("last_error");
            }
        }

        if mark_last_success {
            runtime_obj.insert("last_success_step_id".to_string(), json!(step_id));
        }
        if mark_suggested_resume {
            runtime_obj.insert("suggested_resume_step_id".to_string(), json!(step_id));
        }
        if mark_suggested_resume {
            if let Some(err) = error {
                let failure_mode = Self::classify_orchestration_failure_mode(err);
                self.update_orchestration_failure_mode_stats(
                    runtime_obj,
                    failure_mode,
                    step_id,
                    err,
                    now.as_str(),
                );
            } else {
                self.update_orchestration_failure_mode_stats(
                    runtime_obj,
                    "unknown",
                    step_id,
                    "",
                    now.as_str(),
                );
            }
        }

        {
            let summary_value = runtime_obj
                .entry("summary".to_string())
                .or_insert_with(|| json!({}));
            if !summary_value.is_object() {
                *summary_value = json!({});
            }
            let Some(summary_obj) = summary_value.as_object_mut() else {
                return;
            };
            let mut total_attempts_all = summary_obj
                .get("total_attempts")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let mut total_success = summary_obj
                .get("total_success")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let mut total_failed = summary_obj
                .get("total_failed")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            total_attempts_all += 1;
            if status == "succeeded" {
                total_success += 1;
            } else if status == "failed" {
                total_failed += 1;
            }

            summary_obj.insert("total_attempts".to_string(), json!(total_attempts_all));
            summary_obj.insert("total_success".to_string(), json!(total_success));
            summary_obj.insert("total_failed".to_string(), json!(total_failed));
            summary_obj.insert("updated_at".to_string(), json!(now.clone()));

            if let Some(ms) = duration_ms {
                let current_slowest = summary_obj
                    .get("slowest_duration_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if ms > current_slowest {
                    summary_obj.insert("slowest_duration_ms".to_string(), json!(ms));
                    summary_obj.insert("slowest_step_id".to_string(), json!(step_id));
                }
            }
        }
        self.refresh_orchestration_recovery_suggestions(runtime_obj);
    }

    fn update_orchestration_failure_mode_stats(
        &self,
        runtime_obj: &mut serde_json::Map<String, Value>,
        failure_mode: &str,
        step_id: &str,
        error: &str,
        now: &str,
    ) {
        let modes_value = runtime_obj
            .entry("failure_modes".to_string())
            .or_insert_with(|| json!({}));
        if !modes_value.is_object() {
            *modes_value = json!({});
        }
        let Some(modes_obj) = modes_value.as_object_mut() else {
            return;
        };
        let mode_value = modes_obj
            .entry(failure_mode.to_string())
            .or_insert_with(|| {
                json!({
                    "mode": failure_mode,
                    "count": 0,
                    "latest_step_id": "",
                    "latest_error": "",
                    "hint": Self::failure_mode_recovery_hint(failure_mode),
                    "updated_at": now,
                })
            });
        if !mode_value.is_object() {
            *mode_value = json!({
                "mode": failure_mode,
                "count": 0,
                "latest_step_id": "",
                "latest_error": "",
                "hint": Self::failure_mode_recovery_hint(failure_mode),
                "updated_at": now,
            });
        }
        let Some(mode_obj) = mode_value.as_object_mut() else {
            return;
        };

        let mut count = mode_obj
            .get("count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0);
        count += 1;
        mode_obj.insert("mode".to_string(), json!(failure_mode));
        mode_obj.insert("count".to_string(), json!(count));
        mode_obj.insert("latest_step_id".to_string(), json!(step_id));
        mode_obj.insert(
            "latest_error".to_string(),
            json!(shorten_text(error, 500)),
        );
        mode_obj.insert(
            "hint".to_string(),
            json!(Self::failure_mode_recovery_hint(failure_mode)),
        );
        mode_obj.insert("updated_at".to_string(), json!(now));
    }

    fn refresh_orchestration_recovery_suggestions(
        &self,
        runtime_obj: &mut serde_json::Map<String, Value>,
    ) {
        let mut suggestions: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let push_unique = |msg: String,
                           suggestions: &mut Vec<String>,
                           seen: &mut std::collections::HashSet<String>| {
            if msg.trim().is_empty() {
                return;
            }
            if seen.insert(msg.clone()) {
                suggestions.push(msg);
            }
        };

        if let Some(step_id) = runtime_obj
            .get("suggested_resume_step_id")
            .and_then(|v| v.as_str())
        {
            push_unique(
                format!("优先从失败节点 {} 恢复执行。", step_id),
                &mut suggestions,
                &mut seen,
            );
        }

        if let Some(modes_obj) = runtime_obj
            .get("failure_modes")
            .and_then(|v| v.as_object())
        {
            let mut modes: Vec<(String, i64)> = modes_obj
                .iter()
                .map(|(mode, value)| {
                    (
                        mode.clone(),
                        value.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
                    )
                })
                .collect();
            modes.sort_by(|a, b| b.1.cmp(&a.1));

            for (mode, _) in modes.into_iter().take(3) {
                push_unique(
                    Self::failure_mode_recovery_hint(&mode).to_string(),
                    &mut suggestions,
                    &mut seen,
                );
            }
        }

        if let Some(slowest_step_id) = runtime_obj
            .get("summary")
            .and_then(|v| v.get("slowest_step_id"))
            .and_then(|v| v.as_str())
        {
            let slowest_ms = runtime_obj
                .get("summary")
                .and_then(|v| v.get("slowest_duration_ms"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if slowest_ms >= 120_000 {
                push_unique(
                    format!(
                        "慢节点 {} 耗时较高（约{}秒），建议拆分任务或改并行。",
                        slowest_step_id,
                        slowest_ms / 1000
                    ),
                    &mut suggestions,
                    &mut seen,
                );
            }
        }

        if suggestions.is_empty() {
            suggestions.push("当前暂无明显失败风险，可按既定编排继续运行。".to_string());
        }
        runtime_obj.insert("recovery_suggestions".to_string(), json!(suggestions));
    }

    fn classify_orchestration_failure_mode(error: &str) -> &'static str {
        let lower = error.to_lowercase();
        if lower.contains("timeout")
            || lower.contains("timed out")
            || lower.contains("deadline exceeded")
        {
            "timeout"
        } else if lower.contains("rate limit")
            || lower.contains("429")
            || lower.contains("api key")
            || lower.contains("provider")
            || lower.contains("unauthorized")
        {
            "llm_provider"
        } else if lower.contains("permission")
            || lower.contains("denied")
            || lower.contains("approval")
        {
            "permission"
        } else if lower.contains("tool")
            || lower.contains("shell")
            || lower.contains("command")
        {
            "tool_execution"
        } else if lower.contains("missing")
            || lower.contains("invalid")
            || lower.contains("required")
            || lower.contains("json")
        {
            "input_validation"
        } else if lower.contains("member") && lower.contains("not found") {
            "member_mapping"
        } else {
            "unknown"
        }
    }

    fn failure_mode_recovery_hint(mode: &str) -> &'static str {
        match mode {
            "timeout" => "超时失败较多，建议提高 backoff、降低单步任务复杂度，必要时拆分步骤。",
            "llm_provider" => {
                "模型/供应商失败较多，建议检查 API key、限流配额并切换备用模型。"
            }
            "permission" => "权限失败较多，建议调整工具策略或角色权限后再重试。",
            "tool_execution" => "工具执行失败较多，建议先在单步验证工具输入，再恢复编排运行。",
            "input_validation" => "输入校验失败较多，建议检查 step 配置字段与 JSON 结构。",
            "member_mapping" => "成员映射失败，建议确认 step.member 与 Team 成员名称一致。",
            _ => "未知失败较多，建议从最近失败 step 恢复并开启更小步长重试。",
        }
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
        } else if lower.contains("challeng")
            || lower.contains("review")
            || lower.contains("audit")
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
                orchestration_plan: None,
                plan_version: None,
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
                if tool.name == SkillsTool::NAME {
                    continue;
                }
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

use std::collections::HashMap;
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
                            orchestration_plan: None,
                            plan_version: None,
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
